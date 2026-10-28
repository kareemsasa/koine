//! One application-local discovery worker; two waiting jobs; no daemon.
use crate::{
    config::{Configuration, Root},
    journal::Journal,
    *,
};
use koine_sqlite::Store;
use std::{
    collections::{BTreeSet, VecDeque},
    sync::{Condvar, Mutex},
    thread::{self, JoinHandle},
};
pub const WORKERS: usize = 1;
pub const WAITING_JOBS: usize = 2;
type Observer = Arc<dyn Fn(&Ticket, usize) + Send + Sync>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Pending,
    Running,
    Complete,
    Partial,
    Failed,
    Inconclusive,
}
#[derive(Clone, Debug)]
pub struct Progress {
    pub state: State,
    pub finished: bool,
    pub observations: usize,
    pub inspected_records: usize,
    pub completed_roots: usize,
    pub attempts: Vec<String>,
    pub outcomes: Vec<String>,
    pub error: Option<String>,
}
#[derive(Clone)]
pub struct Ticket {
    pub id: String,
    cancel: Arc<AtomicBool>,
    progress: Arc<(Mutex<Progress>, Condvar)>,
}
impl std::fmt::Debug for Ticket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ticket")
            .field("id", &self.id)
            .field("progress", &self.progress())
            .finish()
    }
}
impl Ticket {
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }
    pub fn progress(&self) -> Progress {
        self.progress.0.lock().unwrap().clone()
    }
    pub fn wait(&self) -> Progress {
        let (lock, cv) = &*self.progress;
        let mut p = lock.lock().unwrap();
        while !p.finished {
            p = cv.wait(p).unwrap();
        }
        p.clone()
    }
    fn update(&self, f: impl FnOnce(&mut Progress)) {
        let mut p = self.progress.0.lock().unwrap();
        f(&mut p);
        self.progress.1.notify_all();
    }
}
struct Attempt {
    root: Root,
    plan: Plan,
}
struct Job {
    attempts: Vec<Attempt>,
    ticket: Ticket,
}
struct Queue {
    waiting: VecDeque<Job>,
    running: Option<Ticket>,
    occupied: Vec<(String, Vec<(String, PathBuf)>)>,
    stopping: bool,
    blocked: bool,
}
struct Shared {
    queue: Mutex<Queue>,
    wake: Condvar,
}
pub struct Scheduler {
    shared: Arc<Shared>,
    worker: Option<JoinHandle<()>>,
    store: Arc<Store>,
    settings: Arc<Mutex<Configuration>>,
    journal: Arc<Journal>,
}
fn attempt_id(request: &str, root: &Root, multiple: bool) -> String {
    if !multiple {
        return request.into();
    }
    let mut bytes =
        canonical::hash(format!("koine.steam.attempt.v1:{request}:{}", root.scope).as_bytes());
    bytes[6] = (bytes[6] & 15) | 64;
    bytes[8] = (bytes[8] & 63) | 128;
    let s = hex(&bytes[..16]);
    format!(
        "{}-{}-{}-{}-{}",
        &s[..8],
        &s[8..12],
        &s[12..16],
        &s[16..20],
        &s[20..]
    )
}
impl Scheduler {
    pub fn new(
        store: Arc<Store>,
        settings: Arc<Mutex<Configuration>>,
        database: &Path,
        recovery_at: i64,
    ) -> Result<Self> {
        Self::start(store, settings, database, recovery_at, Arc::new(|_, _| {}))
    }
    fn start(
        store: Arc<Store>,
        settings: Arc<Mutex<Configuration>>,
        database: &Path,
        recovery_at: i64,
        observer: Observer,
    ) -> Result<Self> {
        let journal = Arc::new(Journal::open(database)?);
        journal.recover(&store)?;
        integration::recover_pending(&store, recovery_at)?;
        let shared = Arc::new(Shared {
            queue: Mutex::new(Queue {
                waiting: VecDeque::new(),
                running: None,
                occupied: Vec::new(),
                stopping: false,
                blocked: false,
            }),
            wake: Condvar::new(),
        });
        let (work_shared, work_store, work_settings, work_journal) = (
            shared.clone(),
            store.clone(),
            settings.clone(),
            journal.clone(),
        );
        let worker = thread::Builder::new()
            .name("koine-steam-discovery".into())
            .spawn(move || {
                work(
                    work_shared,
                    work_store,
                    work_settings,
                    work_journal,
                    observer,
                )
            })
            .map_err(|_| "discovery worker unavailable")?;
        Ok(Self {
            shared,
            worker: Some(worker),
            store,
            settings,
            journal,
        })
    }
    /// Nonblocking capacity/overlap admission. Registration uses the existing writer
    /// transaction before the request is acknowledged or any source file is read.
    pub fn refresh(&self, request: String, at: i64, root_ids: &[String]) -> Result<Ticket> {
        config::uuid(&request)?;
        let (owner, mut roots) = {
            let settings = self.settings.lock().map_err(|_| "settings poisoned")?;
            let roots: Vec<_> = if root_ids.is_empty() {
                settings.roots.clone()
            } else {
                let mut unique = BTreeSet::new();
                let mut roots = Vec::new();
                for id in root_ids {
                    if !unique.insert(id) {
                        return Err("duplicate root selector".into());
                    }
                    roots.push(
                        settings
                            .roots
                            .iter()
                            .find(|r| &r.id == id)
                            .ok_or("unknown configured root")?
                            .clone(),
                    );
                }
                roots
            };
            (settings.owner.clone(), roots)
        };
        if roots.len() > config::ROOT_LIMIT {
            return Err("configured root limit exceeded".into());
        }
        if roots.is_empty() {
            return Err("no configured roots".into());
        }
        roots.sort_by(|a, b| a.id.cmp(&b.id));
        let mut q = self.shared.queue.lock().map_err(|_| "scheduler poisoned")?;
        if q.stopping {
            return Err("scheduler stopping".into());
        }
        if q.blocked {
            return Err("persistence blocked; retry recovery required".into());
        }
        if q.occupied.len() >= WORKERS + WAITING_JOBS {
            return Err("discovery backpressure: queue full; request not accepted".into());
        }
        if q.occupied.iter().any(|(id, scopes)| {
            id == &request
                || roots.iter().any(|r| {
                    scopes
                        .iter()
                        .any(|(scope, path)| scope == &r.scope || path == &r.path)
                })
        }) {
            return Err(
                "equivalent discovery already pending or running; request not accepted".into(),
            );
        }
        let mut attempts = Vec::new();
        for root in &roots {
            let plan = controlled::plan(
                &owner,
                root,
                attempt_id(&request, root, roots.len() > 1),
                at,
            )?;
            attempts.push(Attempt {
                root: root.clone(),
                plan,
            });
        }
        let signature = Value::List(vec![
            owner.into(),
            at.into(),
            Value::List(
                roots
                    .iter()
                    .map(|r| {
                        Value::List(vec![
                            r.id.clone().into(),
                            r.scope.clone().into(),
                            fixture_fs::bytes(&r.path).into(),
                            r.layout.clone().into(),
                            i64::from(r.synthetic).into(),
                            i64::from(r.authorized).into(),
                        ])
                    })
                    .collect(),
            ),
        ]);
        self.journal.bind_request(&request, &signature)?;
        integration::register_plans(
            &self.store,
            attempts.iter().map(|a| a.plan.clone()).collect(),
        )?;
        let ticket = Ticket {
            id: request.clone(),
            cancel: Arc::new(AtomicBool::new(false)),
            progress: Arc::new((
                Mutex::new(Progress {
                    state: State::Pending,
                    finished: false,
                    observations: 0,
                    inspected_records: 0,
                    completed_roots: 0,
                    attempts: attempts.iter().map(|a| a.plan.delivery.clone()).collect(),
                    outcomes: Vec::new(),
                    error: None,
                }),
                Condvar::new(),
            )),
        };
        q.occupied.push((
            request,
            roots.into_iter().map(|r| (r.scope, r.path)).collect(),
        ));
        q.waiting.push_back(Job {
            attempts,
            ticket: ticket.clone(),
        });
        self.shared.wake.notify_one();
        Ok(ticket)
    }
    /// Retry durable reports against immutable identities; never re-scan sources.
    pub fn recover(&self, at: i64) -> Result<()> {
        let mut q = self.shared.queue.lock().map_err(|_| "scheduler poisoned")?;
        if !q.occupied.is_empty() {
            return Err("wait for active requests before recovery".into());
        }
        self.journal.recover(&self.store)?;
        integration::recover_pending(&self.store, at)?;
        q.blocked = false;
        Ok(())
    }
    pub fn shutdown(&mut self) -> Result<()> {
        {
            let mut q = self.shared.queue.lock().map_err(|_| "scheduler poisoned")?;
            q.stopping = true;
            if let Some(t) = &q.running {
                t.cancel();
            }
            for job in &q.waiting {
                job.ticket.cancel();
            }
            self.shared.wake.notify_all();
        }
        if let Some(worker) = self.worker.take() {
            worker
                .join()
                .map_err(|_| "discovery worker panicked; durable attempts require recovery")?;
        }
        Ok(())
    }
}
impl Drop for Scheduler {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
fn work(
    shared: Arc<Shared>,
    store: Arc<Store>,
    settings: Arc<Mutex<Configuration>>,
    journal: Arc<Journal>,
    observer: Observer,
) {
    loop {
        let (job, blocked) = {
            let mut q = shared.queue.lock().unwrap();
            while q.waiting.is_empty() && !q.stopping {
                q = shared.wake.wait(q).unwrap();
            }
            let Some(job) = q.waiting.pop_front() else {
                break;
            };
            q.running = Some(job.ticket.clone());
            (job, q.blocked)
        };
        job.ticket.update(|p| p.state = State::Running);
        let result = if blocked {
            Err("persistence blocked; recover journal before another refresh".into())
        } else {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                execute(&store, &settings, &journal, &job, &*observer)
            }))
            .unwrap_or_else(|_| {
                Err(
                    "discovery worker operation panicked; recover durable attempts before retry"
                        .into(),
                )
            })
        };
        if let Err(error) = result {
            shared.queue.lock().unwrap().blocked = true;
            job.ticket.update(|p| {
                p.state = if p.observations > 0 {
                    State::Partial
                } else {
                    State::Failed
                };
                p.error = Some(error);
            });
        }
        {
            let mut q = shared.queue.lock().unwrap();
            q.running = None;
            q.occupied.retain(|(id, _)| id != &job.ticket.id);
        }
        job.ticket.update(|p| p.finished = true);
    }
}
fn execute(
    store: &Store,
    settings: &Mutex<Configuration>,
    journal: &Journal,
    job: &Job,
    observer: &dyn Fn(&Ticket, usize),
) -> Result<()> {
    let mut seen = BTreeSet::new();
    for attempt in &job.attempts {
        let s = store.snapshot()?;
        if let Ok(c) = s.find(
            "collection_closures",
            "collection_id",
            &attempt.plan.delivery.clone().into(),
        ) {
            let outcome = c.text("outcome")?.to_string();
            let count = s
                .rows("evidence_records")
                .filter(|e| e.get("collection_id") == c.get("collection_id"))
                .count();
            job.ticket.update(|p| {
                p.completed_roots += 1;
                p.observations += count;
                p.outcomes.push(outcome);
            });
            continue;
        }
        let report = controlled::scan(
            settings,
            &attempt.root,
            attempt.plan.clone(),
            job.ticket.cancel.clone(),
            &mut seen,
            &mut |count| {
                job.ticket
                    .update(|p| p.inspected_records = p.observations + count);
                observer(&job.ticket, count);
            },
        )?;
        // Acknowledgement follows durable journal finalization. SQLite replay is
        // idempotent even if the previous commit succeeded but its reply was lost.
        journal.save(&report)?;
        integration::apply(store, &report)?;
        journal.remove(&report.plan.delivery)?;
        job.ticket.update(|p| {
            p.completed_roots += 1;
            p.observations += report
                .roots
                .iter()
                .map(|r| r.observations.len())
                .sum::<usize>();
            p.outcomes.push(report.outcome().into());
        });
    }
    job.ticket.update(|p| {
        p.inspected_records = p.inspected_records.max(p.observations);
        p.state = if p.outcomes.iter().all(|o| o == "complete") {
            State::Complete
        } else if p.observations > 0 {
            State::Partial
        } else if p.outcomes.iter().any(|o| o == "failed") {
            State::Failed
        } else {
            State::Inconclusive
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Lab;
    use std::{sync::mpsc, time::Duration};
    type Observer = Arc<dyn Fn(&Ticket, usize) + Send + Sync>;
    fn gated() -> (Observer, mpsc::Receiver<()>, mpsc::Sender<()>) {
        let (reached_tx, reached_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::channel();
        let release_rx = Mutex::new(release_rx);
        let observer = Arc::new(move |_: &Ticket, count| {
            if count == 1 {
                reached_tx.send(()).unwrap();
                release_rx
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(10))
                    .unwrap();
            }
        });
        (observer, reached_rx, release_tx)
    }
    #[test]
    fn one_worker_two_waiting_jobs_overlap_rejection_and_backpressure() {
        let lab = Lab::new();
        let roots: Vec<_> = (0..4).map(|i| lab.add(&format!("root{i}"))).collect();
        let (observer, reached, release) = gated();
        let scheduler = Scheduler::start(
            lab.store.clone(),
            lab.settings.clone(),
            &lab.db,
            10,
            observer,
        )
        .unwrap();
        let a = scheduler.refresh(id(), 10, &roots[..1]).unwrap();
        reached.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(!a.progress().finished);
        assert_eq!(a.progress().inspected_records, 1);
        let count = lab.store.snapshot().unwrap().rows("collections").count();
        assert!(
            scheduler
                .refresh(id(), 10, &roots[..1])
                .unwrap_err()
                .contains("equivalent")
        );
        assert_eq!(
            lab.store.snapshot().unwrap().rows("collections").count(),
            count
        );
        let b = scheduler.refresh(id(), 10, &roots[1..2]).unwrap();
        let c = scheduler.refresh(id(), 10, &roots[2..3]).unwrap();
        assert!(
            scheduler
                .refresh(id(), 10, &roots[3..4])
                .unwrap_err()
                .contains("backpressure")
        );
        assert_eq!(b.progress().state, State::Pending);
        b.cancel();
        c.cancel();
        release.send(()).unwrap();
        assert_eq!(a.wait().state, State::Complete);
        assert_eq!(b.wait().state, State::Inconclusive);
        assert_eq!(c.wait().state, State::Inconclusive);
        assert_eq!(
            lab.store
                .snapshot()
                .unwrap()
                .rows("evidence_records")
                .count(),
            2
        );
    }
    #[test]
    fn concurrent_equivalent_refreshes_have_one_accepted_request() {
        let lab = Lab::new();
        let r = lab.add("root");
        let (observer, reached, release) = gated();
        let scheduler = Arc::new(
            Scheduler::start(
                lab.store.clone(),
                lab.settings.clone(),
                &lab.db,
                10,
                observer,
            )
            .unwrap(),
        );
        let a = scheduler
            .refresh(id(), 10, std::slice::from_ref(&r))
            .unwrap();
        reached.recv_timeout(Duration::from_secs(5)).unwrap();
        let threads: Vec<_> = (0..4)
            .map(|_| {
                let scheduler = scheduler.clone();
                let r = r.clone();
                thread::spawn(move || scheduler.refresh(id(), 10, &[r]).is_err())
            })
            .collect();
        for t in threads {
            assert!(t.join().unwrap());
        }
        release.send(()).unwrap();
        assert_eq!(a.wait().state, State::Complete);
        assert_eq!(lab.store.snapshot().unwrap().rows("collections").count(), 1);
    }
    #[test]
    fn cancellation_after_observation_preserves_partial_result_and_reports_it() {
        let lab = Lab::new();
        let r = lab.add("root");
        let observer = Arc::new(|t: &Ticket, _| t.cancel());
        let scheduler = Scheduler::start(
            lab.store.clone(),
            lab.settings.clone(),
            &lab.db,
            10,
            observer,
        )
        .unwrap();
        let t = scheduler.refresh(id(), 10, &[r]).unwrap();
        let p = t.wait();
        assert_eq!(p.state, State::Partial);
        assert_eq!(p.observations, 1);
        assert!(p.error.is_none());
        assert_eq!(
            lab.store
                .snapshot()
                .unwrap()
                .rows("evidence_records")
                .count(),
            1
        );
    }
    #[test]
    fn shutdown_cancels_active_and_queued_work_then_joins() {
        let lab = Lab::new();
        let a = lab.add("a");
        let b = lab.add("b");
        let (observer, reached, release) = gated();
        let mut scheduler = Scheduler::start(
            lab.store.clone(),
            lab.settings.clone(),
            &lab.db,
            10,
            observer,
        )
        .unwrap();
        let first = scheduler.refresh(id(), 10, &[a]).unwrap();
        reached.recv_timeout(Duration::from_secs(5)).unwrap();
        let queued = scheduler.refresh(id(), 10, &[b]).unwrap();
        // Release only after shutdown has marked cancellation, without a timing race.
        let shared = scheduler.shared.clone();
        let releaser = thread::spawn(move || {
            loop {
                if shared.queue.lock().unwrap().stopping {
                    release.send(()).unwrap();
                    break;
                }
                thread::yield_now();
            }
        });
        scheduler.shutdown().unwrap();
        releaser.join().unwrap();
        assert_eq!(first.wait().state, State::Partial);
        assert_eq!(queued.wait().state, State::Inconclusive);
        assert!(scheduler.refresh(id(), 10, &[]).is_err());
    }
    #[test]
    fn panicked_collection_is_visible_and_recovers_as_interrupted() {
        let lab = Lab::new();
        let r = lab.add("root");
        let observer = Arc::new(|_: &Ticket, _| panic!("synthetic worker panic"));
        let scheduler = Scheduler::start(
            lab.store.clone(),
            lab.settings.clone(),
            &lab.db,
            10,
            observer,
        )
        .unwrap();
        let t = scheduler.refresh(id(), 10, &[r]).unwrap();
        assert_eq!(t.wait().state, State::Failed);
        assert!(t.progress().error.unwrap().contains("panicked"));
        scheduler.recover(11).unwrap();
        let s = lab.store.snapshot().unwrap();
        assert!(
            s.rows("collection_closures")
                .all(|c| c.is("outcome", "inconclusive"))
        );
        assert_eq!(s.rows("evidence_records").count(), 0);
    }
    #[test]
    fn pending_registration_reopens_with_original_scope_and_no_source_read() {
        let lab = Lab::new();
        let r = lab.add("root");
        let root = lab.root(&r);
        let plan = controlled::plan(&lab.settings.lock().unwrap().owner, &root, id(), 10).unwrap();
        integration::register(&lab.store, &plan).unwrap();
        let pending = lab.store.snapshot().unwrap();
        assert_eq!(pending.rows("collection_closures").count(), 0);
        std::fs::remove_dir_all(&root.path).unwrap();
        let db = lab.db.clone();
        drop(lab.store);
        let store = Arc::new(Store::open(&db).unwrap());
        let settings = Arc::new(Mutex::new(Configuration::open(&store, &db).unwrap()));
        let _scheduler = Scheduler::new(store.clone(), settings, &db, 12).unwrap();
        let s = store.snapshot().unwrap();
        assert_eq!(
            s.rows("requested_units").cloned().collect::<Vec<_>>(),
            pending.rows("requested_units").cloned().collect::<Vec<_>>()
        );
        assert!(
            s.rows("coverage_parts")
                .all(|p| p.is("inspection", "uninspected"))
        );
        assert!(
            s.rows("collection_closures")
                .all(|c| c.is("outcome", "inconclusive"))
        );
    }
}

#[cfg(all(test, feature = "fault-injection"))]
mod writer_recovery_tests {
    use super::*;
    use crate::test_support::Lab;
    #[test]
    fn writer_full_blocks_further_refresh_then_replays_journal_without_rescan() {
        let lab = Lab::fault();
        let r = lab.add("root");
        let root = lab.root(&r);
        std::fs::write(root.path.join("steamapps/appmanifest_41.acf"),format!("\"AppState\" {{\"appid\" \"41\" \"installdir\" \"Synthetic\" \"name\" \"{}\" \"StateFlags\" \"4\"}}","S".repeat(500_000))).unwrap();
        let limit = Arc::new(Mutex::new(None));
        let saved = limit.clone();
        let store = lab.store.clone();
        let observer = Arc::new(move |_: &Ticket, count| {
            if count == 1 {
                *saved.lock().unwrap() = Some(store.fault_limit_pages(1).unwrap());
            }
        });
        let scheduler = Scheduler::start(
            lab.store.clone(),
            lab.settings.clone(),
            &lab.db,
            10,
            observer,
        )
        .unwrap();
        let request = id();
        let t = scheduler
            .refresh(request.clone(), 10, std::slice::from_ref(&r))
            .unwrap();
        assert_eq!(t.wait().state, State::Failed);
        assert!(t.progress().error.is_some());
        assert!(
            scheduler
                .refresh(id(), 11, std::slice::from_ref(&r))
                .unwrap_err()
                .contains("persistence blocked")
        );
        assert_eq!(
            lab.store
                .snapshot()
                .unwrap()
                .rows("evidence_records")
                .count(),
            0
        );
        lab.store
            .fault_limit_pages(limit.lock().unwrap().unwrap() + 4096)
            .unwrap();
        std::fs::remove_dir_all(root.path).unwrap();
        scheduler.recover(12).unwrap();
        assert_eq!(
            lab.store
                .snapshot()
                .unwrap()
                .rows("evidence_records")
                .count(),
            2
        );
        assert_eq!(
            scheduler
                .refresh(request, 10, &[r])
                .unwrap()
                .wait()
                .observations,
            2
        );
    }
}

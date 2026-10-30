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
    #[cfg(test)]
    stage: Mutex<Option<tests::StageHook>>,
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
        canonical::hash(format!("koine.steam.attempt.v1:{request}:{}", root.scope()).as_bytes());
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
            #[cfg(test)]
            stage: Mutex::new(None),
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
                settings.roots().to_vec()
            } else {
                let mut unique = BTreeSet::new();
                let mut roots = Vec::new();
                for id in root_ids {
                    if !unique.insert(id) {
                        return Err("duplicate root selector".into());
                    }
                    roots.push(
                        settings
                            .roots()
                            .iter()
                            .find(|r| r.id() == id)
                            .ok_or("unknown configured root")?
                            .clone(),
                    );
                }
                roots
            };
            (settings.owner().to_string(), roots)
        };
        if roots.len() > config::ROOT_LIMIT {
            return Err("configured root limit exceeded".into());
        }
        if roots.is_empty() {
            return Err("no configured roots".into());
        }
        roots.sort_by(|a, b| a.id().cmp(b.id()));
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
                        .any(|(scope, path)| scope == r.scope() || path == r.path())
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
                            r.id().into(),
                            r.scope().into(),
                            fixture_fs::bytes(r.path()).into(),
                            r.layout().into(),
                            i64::from(r.synthetic()).into(),
                            i64::from(r.authorized()).into(),
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
            roots
                .into_iter()
                .map(|r| (r.scope().to_string(), r.path().to_path_buf()))
                .collect(),
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
                execute(&shared, &store, &settings, &journal, &job, &*observer)
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
    #[cfg_attr(not(test), allow(unused_variables))] shared: &Shared,
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
        #[cfg(test)]
        tests::reached(shared, tests::Stage::Scanned, settings);
        // The journal write is the acceptance boundary. Holding the settings lock
        // from the final configuration check through that write orders every
        // removal or reconfiguration either before acceptance, discarding what was
        // read, or after it, leaving the accepted report valid.
        let report = {
            let configured = settings.lock().map_err(|_| "settings lock poisoned")?;
            let report = controlled::acceptable(&configured, &attempt.root, &attempt.plan, report);
            // Acknowledgement follows durable journal finalization. SQLite replay is
            // idempotent even if the previous commit succeeded but its reply was lost.
            journal.save(&report)?;
            #[cfg(test)]
            tests::reached(shared, tests::Stage::Accepted, settings);
            report
        };
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
    use std::{
        sync::{atomic::AtomicUsize, mpsc},
        time::Duration,
    };
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
    /// Durable outcome, evidence count, diagnostic codes and whether any coverage
    /// part of the collection was inspected.
    fn closed(lab: &Lab, delivery: &str) -> (String, usize, Vec<String>, bool) {
        let s = lab.store.snapshot().unwrap();
        let delivery = Value::from(delivery.to_string());
        let mine = |r: &&Record| r.get("collection_id") == &delivery;
        (
            s.find("collection_closures", "collection_id", &delivery)
                .unwrap()
                .text("outcome")
                .unwrap()
                .to_string(),
            s.rows("evidence_records").filter(mine).count(),
            s.rows("diagnostics")
                .filter(mine)
                .map(|d| d.text("code").unwrap().to_string())
                .collect(),
            s.rows("coverage_parts")
                .filter(mine)
                .any(|p| p.is("inspection", "inspected")),
        )
    }
    const CHANGED: &str = "configuration-changed-after-admission";
    /// Like `gated`, but holds only the first scan so later scans run freely.
    fn gated_once() -> (Observer, mpsc::Receiver<()>, mpsc::Sender<()>) {
        let (gate, reached, release) = gated();
        let used = AtomicBool::new(false);
        let observer = Arc::new(move |t: &Ticket, count| {
            if count == 1 && !used.swap(true, Ordering::SeqCst) {
                gate(t, count);
            }
        });
        (observer, reached, release)
    }
    #[test]
    fn configuration_change_after_admission_closes_only_that_attempt_without_blocking() {
        let lab = Lab::new();
        let hold = lab.add("hold");
        let r = lab.add("root");
        let path = lab.root(&r).path().to_path_buf();
        let (observer, reached, release) = gated_once();
        let scheduler = Scheduler::start(
            lab.store.clone(),
            lab.settings.clone(),
            &lab.db,
            10,
            observer,
        )
        .unwrap();
        let first = scheduler.refresh(id(), 10, &[hold]).unwrap();
        reached.recv_timeout(Duration::from_secs(5)).unwrap();
        let admitted = scheduler
            .refresh(id(), 10, std::slice::from_ref(&r))
            .unwrap();
        // Reconfiguring rotates the scope generation of the admitted root.
        lab.settings
            .lock()
            .unwrap()
            .set(&r, &path, "flatpak", true)
            .unwrap();
        release.send(()).unwrap();
        assert_eq!(first.wait().state, State::Complete);
        let p = admitted.wait();
        // The scheduler is not blocked: another request is admitted.
        let next = scheduler.refresh(id(), 11, &[r]).unwrap();
        assert_eq!(p.state, State::Inconclusive);
        assert!(p.error.is_none(), "{:?}", p.error);
        assert_eq!(p.observations, 0);
        assert_eq!(p.inspected_records, 0);
        assert_eq!(p.completed_roots, 1);
        assert_eq!(p.outcomes, ["inconclusive"]);
        // The root holds two readable manifests; a source read would have produced
        // observations and inspected coverage.
        let (outcome, evidence, codes, inspected) = closed(&lab, &admitted.id);
        assert_eq!(outcome, "inconclusive");
        assert_eq!(evidence, 0);
        assert!(codes.iter().any(|c| c == CHANGED), "{codes:?}");
        assert!(!inspected);
        let next = next.wait();
        assert_eq!(next.state, State::Complete);
        assert_eq!(next.observations, 2);
        assert!(next.error.is_none());
    }
    #[test]
    fn multi_root_request_still_scans_roots_whose_configuration_is_unchanged() {
        let lab = Lab::new();
        let hold = lab.add("hold");
        let kept = lab.add("kept");
        let removed = lab.add("removed");
        let (observer, reached, release) = gated_once();
        let scheduler = Scheduler::start(
            lab.store.clone(),
            lab.settings.clone(),
            &lab.db,
            10,
            observer,
        )
        .unwrap();
        let first = scheduler.refresh(id(), 10, &[hold]).unwrap();
        reached.recv_timeout(Duration::from_secs(5)).unwrap();
        let batch = scheduler
            .refresh(id(), 10, &[kept.clone(), removed.clone()])
            .unwrap();
        lab.settings.lock().unwrap().remove(&removed).unwrap();
        release.send(()).unwrap();
        assert_eq!(first.wait().state, State::Complete);
        let p = batch.wait();
        assert!(p.error.is_none(), "{:?}", p.error);
        assert_eq!(p.state, State::Partial);
        assert_eq!(p.observations, 2);
        assert_eq!(p.completed_roots, 2);
        // Attempts follow root-id order.
        let (kept_at, removed_at) = if kept < removed { (0, 1) } else { (1, 0) };
        let (outcome, evidence, _, inspected) = closed(&lab, &p.attempts[kept_at]);
        assert_eq!(
            (outcome.as_str(), evidence, inspected),
            ("complete", 2, true)
        );
        let (outcome, evidence, codes, inspected) = closed(&lab, &p.attempts[removed_at]);
        assert_eq!(outcome, "inconclusive");
        assert_eq!(evidence, 0);
        assert!(codes.iter().any(|c| c == CHANGED), "{codes:?}");
        assert!(!inspected);
        assert_eq!(
            scheduler.refresh(id(), 11, &[kept]).unwrap().wait().state,
            State::Complete
        );
    }
    #[test]
    fn revoked_real_root_authorization_after_admission_reads_no_source() {
        let lab = Lab::new();
        let hold = lab.add("hold");
        let r = lab.add_authorized_real("real");
        let path = lab.root(&r).path().to_path_buf();
        assert!(lab.root(&r).authorized());
        let (observer, reached, release) = gated_once();
        let scheduler = Scheduler::start(
            lab.store.clone(),
            lab.settings.clone(),
            &lab.db,
            10,
            observer,
        )
        .unwrap();
        let first = scheduler.refresh(id(), 10, &[hold]).unwrap();
        reached.recv_timeout(Duration::from_secs(5)).unwrap();
        let admitted = scheduler
            .refresh(id(), 10, std::slice::from_ref(&r))
            .unwrap();
        // Removing and configuring the same path again clears the approval.
        {
            let mut settings = lab.settings.lock().unwrap();
            settings.remove(&r).unwrap();
            settings.set(&r, &path, "native", false).unwrap();
        }
        assert!(!lab.root(&r).authorized());
        release.send(()).unwrap();
        assert_eq!(first.wait().state, State::Complete);
        let p = admitted.wait();
        assert_eq!(p.state, State::Inconclusive);
        assert!(p.error.is_none(), "{:?}", p.error);
        assert_eq!((p.observations, p.inspected_records), (0, 0));
        let (outcome, evidence, codes, inspected) = closed(&lab, &admitted.id);
        assert_eq!(outcome, "inconclusive");
        assert_eq!(evidence, 0);
        assert!(codes.iter().any(|c| c == CHANGED), "{codes:?}");
        assert!(!inspected);
        // A fresh request is admitted and still refused source access until the
        // exact path is authorized again.
        let next = scheduler
            .refresh(id(), 11, std::slice::from_ref(&r))
            .unwrap();
        let p = next.wait();
        assert_eq!(p.state, State::Inconclusive);
        assert!(p.error.is_none());
        let (_, evidence, codes, inspected) = closed(&lab, &next.id);
        assert_eq!(evidence, 0);
        assert!(
            codes
                .iter()
                .any(|c| c == "explicit-root-authorization-required"),
            "{codes:?}"
        );
        assert!(!inspected);
    }
    #[test]
    fn real_root_authorized_only_after_admission_is_not_read_by_that_request() {
        let lab = Lab::new();
        let hold = lab.add("hold");
        let r = lab.add_authorized_real("real");
        let path = lab.root(&r).path().to_path_buf();
        // Configure the same path again so the root is real-mode and unauthorized.
        {
            let mut settings = lab.settings.lock().unwrap();
            settings.remove(&r).unwrap();
            settings.set(&r, &path, "native", false).unwrap();
        }
        assert!(!lab.root(&r).authorized());
        let (observer, reached, release) = gated_once();
        let scheduler = Scheduler::start(
            lab.store.clone(),
            lab.settings.clone(),
            &lab.db,
            10,
            observer,
        )
        .unwrap();
        let first = scheduler.refresh(id(), 10, &[hold]).unwrap();
        reached.recv_timeout(Duration::from_secs(5)).unwrap();
        let admitted = scheduler
            .refresh(id(), 10, std::slice::from_ref(&r))
            .unwrap();
        lab.settings.lock().unwrap().authorize(&r, &path).unwrap();
        assert!(lab.root(&r).authorized());
        release.send(()).unwrap();
        assert_eq!(first.wait().state, State::Complete);
        let p = admitted.wait();
        assert_eq!(p.state, State::Inconclusive);
        assert!(p.error.is_none(), "{:?}", p.error);
        assert_eq!((p.observations, p.inspected_records), (0, 0));
        let (outcome, evidence, codes, inspected) = closed(&lab, &admitted.id);
        assert_eq!(outcome, "inconclusive");
        assert_eq!(evidence, 0);
        assert!(codes.iter().any(|c| c == CHANGED), "{codes:?}");
        assert!(!inspected);
        // A fresh request is admitted and uses the authorization now recorded.
        let next = scheduler.refresh(id(), 11, &[r]).unwrap();
        let p = next.wait();
        assert!(p.error.is_none(), "{:?}", p.error);
        assert_eq!(p.state, State::Complete);
        assert_eq!(p.observations, 2);
        let (outcome, evidence, _, inspected) = closed(&lab, &next.id);
        assert_eq!(
            (outcome.as_str(), evidence, inspected),
            ("complete", 2, true)
        );
    }
    #[test]
    fn pending_registration_reopens_with_original_scope_and_no_source_read() {
        let lab = Lab::new();
        let r = lab.add("root");
        let root = lab.root(&r);
        let plan = controlled::plan(lab.settings.lock().unwrap().owner(), &root, id(), 10).unwrap();
        integration::register(&lab.store, &plan).unwrap();
        let pending = lab.store.snapshot().unwrap();
        assert_eq!(pending.rows("collection_closures").count(), 0);
        std::fs::remove_dir_all(root.path()).unwrap();
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
    /// Test-only pause points: after an attempt's scan returns, before the final
    /// configuration check; and after its journal write, with the settings lock held.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(super) enum Stage {
        Scanned,
        Accepted,
    }
    pub(super) type StageHook = Arc<dyn Fn(Stage, &Mutex<Configuration>) + Send + Sync>;
    pub(super) fn reached(shared: &Shared, stage: Stage, settings: &Mutex<Configuration>) {
        let hook = shared.stage.lock().unwrap().clone();
        if let Some(hook) = hook {
            hook(stage, settings);
        }
    }
    const DURING: &str = "configuration-changed-during-discovery";
    /// Counts every observer call, so a test can show no manifest was read after a
    /// revocation.
    fn counted(inner: Observer) -> (Observer, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        let seen = calls.clone();
        let observer = Arc::new(move |t: &Ticket, count| {
            seen.fetch_add(1, Ordering::SeqCst);
            inner(t, count);
        });
        (observer, calls)
    }
    /// A third readable manifest, so a revoked attempt has manifests left to read.
    fn third_manifest(path: &Path) {
        std::fs::write(
            path.join("steamapps/appmanifest_43.acf"),
            "\"AppState\" {\"appid\" \"43\" \"installdir\" \"Synthetic 43\" \"name\" \"Synthetic 43\" \"StateFlags\" \"4\"}",
        )
        .unwrap();
    }
    /// Whether an unapplied report for this collection remains in the journal.
    fn journaled(lab: &Lab, delivery: &str) -> bool {
        let mut dir = std::fs::canonicalize(&lab.db).unwrap().into_os_string();
        dir.push(".steam-journal");
        PathBuf::from(dir).join(format!("{delivery}.koin")).exists()
    }
    /// A single-root attempt that lost its configuration after reading `read`
    /// manifests: nothing it read is committed and it closes inconclusive.
    fn assert_discarded(lab: &Lab, ticket: &Ticket, calls: &AtomicUsize, read: usize) {
        let p = ticket.wait();
        assert!(p.error.is_none(), "{:?}", p.error);
        assert_eq!(p.state, State::Inconclusive);
        assert_eq!(p.observations, 0);
        assert_eq!(p.outcomes, ["inconclusive"]);
        assert_eq!(p.inspected_records, read);
        assert_eq!(calls.load(Ordering::SeqCst), read);
        let (outcome, evidence, codes, inspected) = closed(lab, &ticket.id);
        assert_eq!(outcome, "inconclusive");
        assert_eq!(evidence, 0);
        // Only the configuration code, once per durable diagnostic row.
        assert!(
            !codes.is_empty() && codes.iter().all(|c| c == DURING),
            "{codes:?}"
        );
        assert!(!inspected);
        assert!(!journaled(lab, &ticket.id));
        let s = lab.store.snapshot().unwrap();
        assert_eq!(s.rows("evidence_records").count(), 0);
        assert_eq!(s.rows("source_entries").count(), 0);
    }
    #[test]
    fn authorized_real_root_removed_mid_scan_commits_none_of_its_observations() {
        let lab = Lab::new();
        let r = lab.add_authorized_real("real");
        let path = lab.root(&r).path().to_path_buf();
        third_manifest(&path);
        let (gate, reached, release) = gated();
        let (observer, calls) = counted(gate);
        let scheduler = Scheduler::start(
            lab.store.clone(),
            lab.settings.clone(),
            &lab.db,
            10,
            observer,
        )
        .unwrap();
        let t = scheduler
            .refresh(id(), 10, std::slice::from_ref(&r))
            .unwrap();
        reached.recv_timeout(Duration::from_secs(5)).unwrap();
        lab.settings.lock().unwrap().remove(&r).unwrap();
        release.send(()).unwrap();
        assert_discarded(&lab, &t, &calls, 1);
        // The scheduler is not blocked: the root configured and authorized again is
        // admitted and read in full by a fresh request.
        {
            let mut settings = lab.settings.lock().unwrap();
            settings.set(&r, &path, "native", false).unwrap();
            settings.authorize(&r, &path).unwrap();
        }
        let next = scheduler.refresh(id(), 11, &[r]).unwrap();
        release.send(()).unwrap();
        let p = next.wait();
        assert!(p.error.is_none(), "{:?}", p.error);
        assert_eq!((p.state, p.observations), (State::Complete, 3));
    }
    #[test]
    fn real_root_reconfigured_mid_scan_commits_none_of_its_observations() {
        let lab = Lab::new();
        let r = lab.add_authorized_real("real");
        let before = lab.root(&r);
        let path = before.path().to_path_buf();
        third_manifest(&path);
        let (gate, reached, release) = gated_once();
        let (observer, calls) = counted(gate);
        let scheduler = Scheduler::start(
            lab.store.clone(),
            lab.settings.clone(),
            &lab.db,
            10,
            observer,
        )
        .unwrap();
        let t = scheduler
            .refresh(id(), 10, std::slice::from_ref(&r))
            .unwrap();
        reached.recv_timeout(Duration::from_secs(5)).unwrap();
        // A changed field rotates the scope generation and clears the approval.
        lab.settings
            .lock()
            .unwrap()
            .set(&r, &path, "flatpak", false)
            .unwrap();
        let after = lab.root(&r);
        assert_ne!(after.scope(), before.scope());
        assert!(!after.authorized());
        release.send(()).unwrap();
        assert_discarded(&lab, &t, &calls, 1);
        lab.settings.lock().unwrap().authorize(&r, &path).unwrap();
        let p = scheduler.refresh(id(), 11, &[r]).unwrap().wait();
        assert!(p.error.is_none(), "{:?}", p.error);
        assert_eq!((p.state, p.observations), (State::Complete, 3));
    }
    #[test]
    fn synthetic_root_removed_mid_scan_commits_none_of_its_observations() {
        let lab = Lab::new();
        let r = lab.add("synthetic");
        third_manifest(lab.root(&r).path());
        let (gate, reached, release) = gated_once();
        let (observer, calls) = counted(gate);
        let scheduler = Scheduler::start(
            lab.store.clone(),
            lab.settings.clone(),
            &lab.db,
            10,
            observer,
        )
        .unwrap();
        let t = scheduler
            .refresh(id(), 10, std::slice::from_ref(&r))
            .unwrap();
        reached.recv_timeout(Duration::from_secs(5)).unwrap();
        lab.settings.lock().unwrap().remove(&r).unwrap();
        release.send(()).unwrap();
        assert_discarded(&lab, &t, &calls, 1);
        let other = lab.add("other");
        let p = scheduler.refresh(id(), 11, &[other]).unwrap().wait();
        assert!(p.error.is_none(), "{:?}", p.error);
        assert_eq!((p.state, p.observations), (State::Complete, 2));
    }
    #[test]
    fn unrelated_root_changed_mid_scan_leaves_the_scanned_attempt_intact() {
        let lab = Lab::new();
        let r = lab.add_authorized_real("scanned");
        third_manifest(lab.root(&r).path());
        let unrelated = lab.add("unrelated");
        let removed = lab.add("removed");
        let unrelated_path = lab.root(&unrelated).path().to_path_buf();
        let (gate, reached, release) = gated();
        let (observer, calls) = counted(gate);
        let scheduler = Scheduler::start(
            lab.store.clone(),
            lab.settings.clone(),
            &lab.db,
            10,
            observer,
        )
        .unwrap();
        let t = scheduler
            .refresh(id(), 10, std::slice::from_ref(&r))
            .unwrap();
        reached.recv_timeout(Duration::from_secs(5)).unwrap();
        {
            let mut settings = lab.settings.lock().unwrap();
            settings
                .set(&unrelated, &unrelated_path, "flatpak", true)
                .unwrap();
            settings.remove(&removed).unwrap();
        }
        release.send(()).unwrap();
        let p = t.wait();
        assert!(p.error.is_none(), "{:?}", p.error);
        assert_eq!((p.state, p.observations), (State::Complete, 3));
        assert_eq!(calls.load(Ordering::SeqCst), 3);
        let (outcome, evidence, codes, inspected) = closed(&lab, &t.id);
        assert_eq!(
            (outcome.as_str(), evidence, inspected),
            ("complete", 3, true)
        );
        assert!(codes.is_empty(), "{codes:?}");
        assert!(!journaled(&lab, &t.id));
    }
    #[test]
    fn multi_root_request_completes_other_roots_after_one_is_revoked_mid_scan() {
        let lab = Lab::new();
        let a = lab.add("a");
        let b = lab.add_authorized_real("b");
        for r in [&a, &b] {
            third_manifest(lab.root(r).path());
        }
        // Attempts follow root-id order; revoke the one read first while it is read.
        let (first, second) = if a < b { (&a, &b) } else { (&b, &a) };
        let (gate, reached, release) = gated_once();
        let (observer, calls) = counted(gate);
        let scheduler = Scheduler::start(
            lab.store.clone(),
            lab.settings.clone(),
            &lab.db,
            10,
            observer,
        )
        .unwrap();
        let t = scheduler
            .refresh(id(), 10, &[a.clone(), b.clone()])
            .unwrap();
        reached.recv_timeout(Duration::from_secs(5)).unwrap();
        lab.settings.lock().unwrap().remove(first).unwrap();
        release.send(()).unwrap();
        let p = t.wait();
        assert!(p.error.is_none(), "{:?}", p.error);
        assert_eq!(p.state, State::Partial);
        assert_eq!(p.observations, 3);
        assert_eq!(p.completed_roots, 2);
        assert_eq!(p.outcomes, ["inconclusive", "complete"]);
        // One manifest of the revoked root, then all three of the other.
        assert_eq!(calls.load(Ordering::SeqCst), 4);
        let (outcome, evidence, codes, inspected) = closed(&lab, &p.attempts[0]);
        assert_eq!((outcome.as_str(), evidence), ("inconclusive", 0));
        // Only the configuration code, once per durable diagnostic row.
        assert!(
            !codes.is_empty() && codes.iter().all(|c| c == DURING),
            "{codes:?}"
        );
        assert!(!inspected);
        let (outcome, evidence, _, inspected) = closed(&lab, &p.attempts[1]);
        assert_eq!(
            (outcome.as_str(), evidence, inspected),
            ("complete", 3, true)
        );
        assert!(p.attempts.iter().all(|d| !journaled(&lab, d)));
        let p = scheduler
            .refresh(id(), 11, std::slice::from_ref(second))
            .unwrap()
            .wait();
        assert_eq!((p.state, p.observations), (State::Complete, 3));
    }
    #[test]
    fn revocation_completed_before_the_final_check_discards_a_fully_read_report() {
        let lab = Lab::new();
        let r = lab.add_authorized_real("real");
        let (observer, calls) = counted(Arc::new(|_: &Ticket, _| {}));
        let scheduler = Scheduler::start(
            lab.store.clone(),
            lab.settings.clone(),
            &lab.db,
            10,
            observer,
        )
        .unwrap();
        // Revoke after every read and in-scan check has passed, before acceptance.
        let rid = r.clone();
        *scheduler.shared.stage.lock().unwrap() = Some(Arc::new(move |stage, settings| {
            if stage == Stage::Scanned {
                settings.lock().unwrap().remove(&rid).unwrap();
            }
        }));
        let t = scheduler.refresh(id(), 10, &[r]).unwrap();
        assert_discarded(&lab, &t, &calls, 2);
    }
    #[test]
    fn revocation_attempted_at_acceptance_waits_and_leaves_the_report_accepted() {
        let lab = Lab::new();
        let r = lab.add_authorized_real("real");
        let scheduler =
            Scheduler::new(lab.store.clone(), lab.settings.clone(), &lab.db, 10).unwrap();
        let held = Arc::new(AtomicBool::new(false));
        let revoker = Arc::new(Mutex::new(None));
        let (rid, shared_settings, h, v) = (
            r.clone(),
            lab.settings.clone(),
            held.clone(),
            revoker.clone(),
        );
        *scheduler.shared.stage.lock().unwrap() = Some(Arc::new(move |stage, settings| {
            if stage == Stage::Accepted {
                // The journal write has happened and the final check's lock is still
                // held, so a concurrent removal can only complete after acceptance.
                h.store(
                    matches!(
                        settings.try_lock(),
                        Err(std::sync::TryLockError::WouldBlock)
                    ),
                    Ordering::SeqCst,
                );
                let (rid, settings) = (rid.clone(), shared_settings.clone());
                *v.lock().unwrap() = Some(thread::spawn(move || {
                    settings.lock().unwrap().remove(&rid).unwrap()
                }));
            }
        }));
        let t = scheduler
            .refresh(id(), 10, std::slice::from_ref(&r))
            .unwrap();
        let p = t.wait();
        revoker.lock().unwrap().take().unwrap().join().unwrap();
        assert!(held.load(Ordering::SeqCst));
        assert!(lab.settings.lock().unwrap().roots().is_empty());
        assert!(p.error.is_none(), "{:?}", p.error);
        assert_eq!((p.state, p.observations), (State::Complete, 2));
        let (outcome, evidence, codes, inspected) = closed(&lab, &t.id);
        assert_eq!(
            (outcome.as_str(), evidence, inspected),
            ("complete", 2, true)
        );
        assert!(codes.is_empty(), "{codes:?}");
        assert!(!journaled(&lab, &t.id));
    }
    #[test]
    fn report_journaled_before_revocation_is_replayed_by_recovery() {
        let lab = Lab::new();
        let r = lab.add_authorized_real("real");
        let report = lab.report(&r);
        assert_eq!(report.outcome(), "complete");
        {
            let journal = Journal::open(&lab.db).unwrap();
            journal.save(&report).unwrap();
        }
        lab.settings.lock().unwrap().remove(&r).unwrap();
        let delivery = report.plan.delivery.clone();
        assert!(journaled(&lab, &delivery));
        let _scheduler =
            Scheduler::new(lab.store.clone(), lab.settings.clone(), &lab.db, 12).unwrap();
        let (outcome, evidence, codes, inspected) = closed(&lab, &delivery);
        assert_eq!(
            (outcome.as_str(), evidence, inspected),
            ("complete", 2, true)
        );
        assert!(codes.is_empty(), "{codes:?}");
        assert!(!journaled(&lab, &delivery));
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
        std::fs::write(root.path().join("steamapps/appmanifest_41.acf"),format!("\"AppState\" {{\"appid\" \"41\" \"installdir\" \"Synthetic\" \"name\" \"{}\" \"StateFlags\" \"4\"}}","S".repeat(500_000))).unwrap();
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
        std::fs::remove_dir_all(root.path()).unwrap();
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

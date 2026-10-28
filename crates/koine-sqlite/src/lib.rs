//! One bounded writer, one bounded reader, immutable SQLite evidence.
//! Normal operations never accept SQL and never execute a launcher.
use koine_core::{
    canonical::{self, CATALOG},
    reconcile, *,
};
use rusqlite::{
    Connection, OpenFlags, params_from_iter,
    types::{Value as SqlValue, ValueRef},
};
use std::{
    cell::Cell,
    fs::{File, OpenOptions},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
#[cfg(feature = "fault-injection")]
pub mod faults;
pub mod jobs;
pub use jobs::{Cancellation, Inspection, SubmitError, WriteOutcome, WriteState, WriteTicket};
mod backup;
mod catalog;
use backup::backup;
use catalog::{check_identity, insert, integrity, load, prepare, read_snapshot};

pub const BOOTSTRAP: &str = include_str!("bootstrap_v1.sql");
pub const APPLICATION_ID: i64 = 0x4b4f494e;
const VERSION: i64 = 1;
type Reply<T> = mpsc::Sender<Result<T>>;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checkpoint {
    pub busy: i64,
    pub wal_pages: i64,
    pub checkpointed_pages: i64,
}
struct Reservation {
    target: String,
    launcher: String,
    capable: bool,
}
type Planner = Box<dyn FnOnce(&Snapshot) -> Result<Vec<Record>> + Send>;
enum Write {
    Checkpoint(Reply<Checkpoint>),
    Apply(Vec<Record>, bool, Reply<Option<Reservation>>),
    Job(
        Vec<Record>,
        Cancellation,
        jobs::Fault,
        mpsc::Sender<WriteOutcome>,
    ),
    Plan(Planner, Reply<()>),
    #[cfg(feature = "fault-injection")]
    LimitPages(i64, Reply<i64>),
    Stop,
}
enum Read {
    Snapshot(Reply<Snapshot>),
    Backup(PathBuf, Arc<AtomicBool>, jobs::Fault, Reply<()>),
    Stop,
}
pub struct Store {
    writer: SyncSender<Write>,
    reader: SyncSender<Read>,
    workers: Vec<JoinHandle<()>>,
    _ownership: File,
    #[cfg(feature = "fault-injection")]
    fault_lab: bool,
}
thread_local! {static BUDGET:Cell<Duration>=const {Cell::new(Duration::from_secs(5))};}
fn deadline() {
    BUDGET.set(Duration::from_secs(5));
}
fn waiting() -> bool {
    !BUDGET.get().is_zero()
}
fn lock_pause() -> bool {
    if jobs::checkpoint(jobs::Point::BeforeBegin).is_err() {
        return false;
    }
    let remaining = BUDGET.get();
    if remaining.is_zero() {
        return false;
    }
    let start = Instant::now();
    thread::sleep(remaining.min(Duration::from_millis(5)));
    BUDGET.set(remaining.saturating_sub(start.elapsed()));
    waiting()
}
fn sql<T>(r: rusqlite::Result<T>) -> Result<T> {
    r.map_err(|e| {
        jobs::sql_error(&e);
        e.to_string()
    })
}
fn io<T>(r: std::io::Result<T>) -> Result<T> {
    r.map_err(|e| e.to_string())
}
fn runtime() -> Result<()> {
    let n = rusqlite::version_number();
    if n < 3_051_003 {
        return Err(format!(
            "SQLite {n}: require verified WAL-reset-fixed runtime >=3.51.3"
        ));
    }
    Ok(())
}
fn open_rw(path: &Path) -> Result<Connection> {
    sql(Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_PRIVATE_CACHE,
    ))
}
fn configure(c: &mut Connection, reader: bool) -> Result<()> {
    deadline();
    sql(c.busy_handler(Some(|_| lock_pause())))?;
    sql(c.execute_batch("PRAGMA foreign_keys=ON; PRAGMA read_uncommitted=OFF; PRAGMA recursive_triggers=ON; PRAGMA synchronous=FULL; PRAGMA wal_autocheckpoint=1000;"))?;
    let mode: String = sql(c.query_row("PRAGMA journal_mode=WAL", [], |r| r.get(0)))?;
    if mode != "wal" {
        return Err("WAL unsupported on this database".into());
    }
    if reader {
        sql(c.execute_batch("PRAGMA query_only=ON"))?
    }
    let fk: i64 = sql(c.query_row("PRAGMA foreign_keys", [], |r| r.get(0)))?;
    let sync: i64 = sql(c.query_row("PRAGMA synchronous", [], |r| r.get(0)))?;
    let dirty: i64 = sql(c.query_row("PRAGMA read_uncommitted", [], |r| r.get(0)))?;
    let recursive: i64 = sql(c.query_row("PRAGMA recursive_triggers", [], |r| r.get(0)))?;
    let checkpoint: i64 = sql(c.query_row("PRAGMA wal_autocheckpoint", [], |r| r.get(0)))?;
    let query_only: i64 = sql(c.query_row("PRAGMA query_only", [], |r| r.get(0)))?;
    if fk != 1
        || sync != 2
        || dirty != 0
        || recursive != 1
        || checkpoint != 1000
        || query_only != i64::from(reader)
    {
        return Err("effective connection policy mismatch".into());
    }
    Ok(())
}
fn begin(c: &Connection) -> Result<()> {
    loop {
        jobs::checkpoint(jobs::Point::BeforeBegin)?;
        match c.execute_batch("BEGIN IMMEDIATE") {
            Ok(()) => return Ok(()),
            Err(e)
                if e.sqlite_error_code() == Some(rusqlite::ErrorCode::DatabaseBusy)
                    && e.sqlite_error()
                        .is_none_or(|e| e.extended_code != rusqlite::ffi::SQLITE_BUSY_SNAPSHOT)
                    && waiting() =>
            {
                lock_pause();
            }
            Err(e) => {
                jobs::sql_error(&e);
                return Err(e.to_string());
            }
        }
    }
}
fn apply(c: &Connection, rows: Vec<Record>, fresh_attempt: bool) -> Result<Option<Reservation>> {
    deadline();
    jobs::checkpoint(jobs::Point::BeforeBegin)?;
    if c.is_autocommit() {
        begin(c)?;
    }
    jobs::phase(jobs::Phase::Active);
    let result = (|| {
        let existing = load(c)?;
        if fresh_attempt
            && rows
                .iter()
                .any(|r| existing.by_id("launch_attempts", r.get("id")).is_ok())
        {
            return Err("attempt already reserved; no replay".into());
        }
        let reservation = if fresh_attempt {
            let intent = rows.first().ok_or("missing launch intent")?;
            if rows.len() != 1 || intent.table != "launch_attempts" {
                return Err("invalid intent reservation".into());
            }
            let target = existing.by_id("launch_targets", intent.get("target_id"))?;
            let assessment = Assessment {
                at: intent.int("created_at")?,
                observer_epoch: None,
                reconciliation_rule: rule_id(&existing, "reconciliation")?,
                freshness_rule: rule_id(&existing, "freshness")?,
            };
            let library = reconcile::reconstruct(&existing, &assessment);
            let capable = library.claims.iter().any(|c| {
                c.dimension == "handoff_capability"
                    && c.knowledge == Knowledge::Observed("supported".into())
                    && existing
                        .by_id("claim_subjects", &c.subject.clone().into())
                        .is_ok_and(|subject| subject.get("target_id") == target.get("id"))
            });
            Some(Reservation {
                target: target.text("id")?.into(),
                launcher: target.text("launcher_context_id")?.into(),
                capable,
            })
        } else {
            None
        };
        let rows = prepare(&existing, rows)?;
        for (index, r) in rows.iter().enumerate() {
            let m = CATALOG.iter().find(|m| m.table == r.table).ok_or("table")?;
            if !existing
                .rows(&r.table)
                .any(|old| m.keys.iter().all(|k| old.get(k) == r.get(k)))
            {
                insert(c, r)?
            }
            jobs::checkpoint(jobs::Point::AfterInsert(index + 1))?;
        }
        let proposed = load(c)?;
        reconcile::validate(&proposed)?;
        integrity(c)?;
        loop {
            jobs::checkpoint(jobs::Point::BeforeCommit)?;
            jobs::phase(jobs::Phase::CommitAttempted);
            match c.execute_batch("COMMIT") {
                Ok(()) => {
                    jobs::phase(jobs::Phase::Committed);
                    jobs::checkpoint(jobs::Point::AfterCommit)?;
                    return Ok(reservation);
                }
                Err(e)
                    if e.sqlite_error_code() == Some(rusqlite::ErrorCode::DatabaseBusy)
                        && e.sqlite_error().is_none_or(|e| {
                            e.extended_code != rusqlite::ffi::SQLITE_BUSY_SNAPSHOT
                        })
                        && !c.is_autocommit()
                        && waiting() =>
                {
                    lock_pause();
                }
                Err(e) => {
                    jobs::sql_error(&e);
                    return Err(format!(
                        "commit outcome requires inspection; no body replay: {e}"
                    ));
                }
            }
        }
    })();
    if result.is_err() && !c.is_autocommit() {
        let rollback = jobs::rollback_checkpoint()
            .and_then(|_| c.execute_batch("ROLLBACK").map_err(|e| e.to_string()));
        match rollback {
            Ok(()) => jobs::phase(jobs::Phase::Active),
            Err(e) => {
                jobs::phase(jobs::Phase::CommitAttempted);
                return Err(format!(
                    "transaction outcome uncertain; preserve database and inspect identities: {e}"
                ));
            }
        }
    }
    result
}
impl Store {
    pub fn create(path: impl AsRef<Path>) -> Result<Self> {
        Self::start(path.as_ref(), true)
    }
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::start(path.as_ref(), false)
    }
    fn start(path: &Path, create: bool) -> Result<Self> {
        runtime()?;
        if create {
            io(OpenOptions::new()
                .write(true)
                .mode(0o600)
                .create_new(true)
                .open(path))?;
        }
        let ownership = io(OpenOptions::new().read(true).write(true).open(path))?;
        ownership
            .try_lock()
            .map_err(|_| "another Koiné process owns this database")?;
        let path = io(std::fs::canonicalize(path))?;
        let mut writer = open_rw(&path)?;
        if create {
            configure(&mut writer, false)?;
            sql(writer.execute_batch("BEGIN IMMEDIATE"))?;
            sql(writer.execute_batch(BOOTSTRAP))?;
            insert(
                &writer,
                &Record::new(
                    "database_identity",
                    [
                        ("id", id().into()),
                        ("family", "koine.offline.foundation".into()),
                        ("initial_version", 1i64.into()),
                    ],
                ),
            )?;
            insert(
                &writer,
                &Record::new(
                    "migration_history",
                    [
                        ("id", id().into()),
                        ("migration_id", "bootstrap.v1".into()),
                        ("source_version", 0i64.into()),
                        ("target_version", 1i64.into()),
                        (
                            "artifact_digest",
                            canonical::hash(BOOTSTRAP.as_bytes()).into(),
                        ),
                        ("build", env!("CARGO_PKG_VERSION").into()),
                        (
                            "applied_at",
                            i64::try_from(
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .map_err(|e| e.to_string())?
                                    .as_micros(),
                            )
                            .map_err(|_| "clock out of range")?
                            .into(),
                        ),
                    ],
                ),
            )?;
            integrity(&writer)?;
            reconcile::validate(&load(&writer)?)?;
            sql(writer.execute_batch(&format!(
                "PRAGMA application_id={APPLICATION_ID}; PRAGMA user_version={VERSION}; COMMIT;"
            )))?;
        }
        if create {
            writer.close().map_err(|(_, e)| e.to_string())?;
            writer = open_rw(&path)?;
        }
        check_identity(&writer)?;
        configure(&mut writer, false)?;
        integrity(&writer)?;
        reconcile::validate(&load(&writer)?)?;
        let mut reader = open_rw(&path)?;
        configure(&mut reader, true)?;
        let (wtx, wrx) = mpsc::sync_channel(32);
        let (rtx, rrx) = mpsc::sync_channel(32);
        let w = thread::Builder::new()
            .name("koine-writer".into())
            .spawn(move || {
                while let Ok(msg) = wrx.recv() {
                    match msg {
                        Write::Apply(rows, fresh, reply) => {
                            jobs::reset(Cancellation::default(), None);
                            let _ = reply.send(apply(&writer, rows, fresh));
                        }
                        Write::Job(rows, cancel, fault, reply) => {
                            jobs::reset(cancel, fault);
                            let result = apply(&writer, rows, false);
                            let _ = reply.send(jobs::receipt(&result));
                        }
                        Write::Plan(plan, reply) => {
                            jobs::reset(Cancellation::default(), None);
                            deadline();
                            let mut result = (|| {
                                begin(&writer)?;
                                let snapshot = load(&writer)?;
                                let rows = plan(&snapshot)?;
                                apply(&writer, rows, false).map(|_| ())
                            })();
                            if !writer.is_autocommit() && let Err(error) = writer.execute_batch("ROLLBACK") {
                                result = Err(format!("planner transaction outcome uncertain; inspect durable identities: {error}"));
                            }
                            let _ = reply.send(result);
                        }
                        #[cfg(feature = "fault-injection")]
                        Write::LimitPages(pages, reply) => {
                            let _ = reply.send(sql(writer.query_row(
                                &format!("PRAGMA max_page_count={pages}"),
                                [],
                                |r| r.get(0),
                            )));
                        }
                        Write::Checkpoint(reply) => {
                            jobs::reset(Cancellation::default(), None);
                            deadline();
                            let _ = reply.send(sql(writer.query_row(
                                "PRAGMA wal_checkpoint(PASSIVE)",
                                [],
                                |r| {
                                    Ok(Checkpoint {
                                        busy: r.get(0)?,
                                        wal_pages: r.get(1)?,
                                        checkpointed_pages: r.get(2)?,
                                    })
                                },
                            )));
                        }
                        Write::Stop => break,
                    }
                    // Never reuse a connection with an unresolved transaction.
                    // Dropped queued replies report unavailable/uncertain outcomes.
                    if !writer.is_autocommit() { break; }
                }
            })
            .map_err(|e| e.to_string())?;
        let r = thread::Builder::new()
            .name("koine-reader".into())
            .spawn(move || {
                while let Ok(msg) = rrx.recv() {
                    match msg {
                        Read::Snapshot(reply) => {
                            let _ = reply.send(read_snapshot(&reader));
                        }
                        Read::Backup(path, cancel, fault, reply) => {
                            jobs::reset(Cancellation::default(), fault);
                            let mut stopping = false;
                            let _ =
                                reply.send(backup(&reader, &path, &cancel, &rrx, &mut stopping));
                            if stopping {
                                break;
                            }
                        }
                        Read::Stop => break,
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            writer: wtx,
            reader: rtx,
            workers: vec![w, r],
            _ownership: ownership,
            #[cfg(feature = "fault-injection")]
            fault_lab: false,
        })
    }
    /// Plan against the writer snapshot inside the same IMMEDIATE transaction.
    /// The planner must be pure: it must not block or call back into this Store.
    pub fn mutate(
        &self,
        plan: impl FnOnce(&Snapshot) -> Result<Vec<Record>> + Send + 'static,
    ) -> Result<()> {
        let (tx, rx) = mpsc::channel();
        self.writer
            .send(Write::Plan(Box::new(plan), tx))
            .map_err(|_| "writer unavailable")?;
        rx.recv()
            .map_err(|_| "mutation outcome unknown; inspect identities")?
    }
    /// Nonblocking queue admission. Full/disconnected means the job was not accepted.
    pub fn try_submit(
        &self,
        rows: Vec<Record>,
        cancel: Cancellation,
    ) -> std::result::Result<WriteTicket, SubmitError> {
        let (tx, rx) = mpsc::channel();
        self.writer
            .try_send(Write::Job(rows, cancel, None, tx))
            .map_err(|e| match e {
                mpsc::TrySendError::Full(_) => SubmitError::Full,
                mpsc::TrySendError::Disconnected(_) => SubmitError::Disconnected,
            })?;
        Ok(WriteTicket(rx))
    }
    /// Inspect every supplied immutable operation record before any retry.
    /// Partial or conflicting content requires investigation, not body replay.
    pub fn inspect_operation(&self, rows: &[Record]) -> Result<Inspection> {
        jobs::inspect(&self.snapshot()?, rows)
    }
    pub fn request_snapshot(&self) -> Result<Receiver<Result<Snapshot>>> {
        let (tx, rx) = mpsc::channel();
        self.reader
            .send(Read::Snapshot(tx))
            .map_err(|_| "reader unavailable")?;
        Ok(rx)
    }
    #[cfg(feature = "fault-injection")]
    pub fn create_fault_lab(path: impl AsRef<Path>) -> Result<Self> {
        let mut store = Self::create(path)?;
        store.fault_lab = true;
        Ok(store)
    }
    #[cfg(feature = "fault-injection")]
    pub fn fault_submit(
        &self,
        rows: Vec<Record>,
        cancel: Cancellation,
        fault: faults::FaultPlan,
    ) -> std::result::Result<WriteTicket, SubmitError> {
        if !self.fault_lab {
            return Err(SubmitError::FaultLabRequired);
        }
        let (tx, rx) = mpsc::channel();
        self.writer
            .try_send(Write::Job(rows, cancel, Some(fault), tx))
            .map_err(|e| match e {
                mpsc::TrySendError::Full(_) => SubmitError::Full,
                mpsc::TrySendError::Disconnected(_) => SubmitError::Disconnected,
            })?;
        Ok(WriteTicket(rx))
    }
    #[cfg(feature = "fault-injection")]
    pub fn fault_limit_pages(&self, pages: i64) -> Result<i64> {
        if !self.fault_lab {
            return Err("fault lab required".into());
        }
        let (tx, rx) = mpsc::channel();
        self.writer
            .send(Write::LimitPages(pages, tx))
            .map_err(|_| "writer unavailable")?;
        rx.recv().map_err(|_| "writer unavailable")?
    }
    #[cfg(feature = "fault-injection")]
    pub fn fault_backup(
        &self,
        path: impl AsRef<Path>,
        cancel: Arc<AtomicBool>,
        fault: faults::FaultPlan,
    ) -> Result<()> {
        if !self.fault_lab {
            return Err("fault lab required".into());
        }
        let (tx, rx) = mpsc::channel();
        self.reader
            .send(Read::Backup(path.as_ref().into(), cancel, Some(fault), tx))
            .map_err(|_| "reader unavailable")?;
        rx.recv().map_err(|_| "backup outcome unknown")?
    }
    pub fn checkpoint(&self) -> Result<Checkpoint> {
        let (tx, rx) = mpsc::channel();
        self.writer
            .send(Write::Checkpoint(tx))
            .map_err(|_| "writer unavailable")?;
        rx.recv().map_err(|_| "checkpoint outcome unavailable")?
    }
    pub fn write(&self, records: Vec<Record>) -> Result<()> {
        let (tx, rx) = mpsc::channel();
        self.writer
            .send(Write::Apply(records, false, tx))
            .map_err(|_| "writer unavailable")?;
        rx.recv()
            .map_err(|_| "writer outcome unavailable; inspect immutable IDs before retry")?
            .map(|_| ())
    }
    fn reserve_intent(&self, intent: Record) -> Result<Reservation> {
        let (tx, rx) = mpsc::channel();
        self.writer
            .send(Write::Apply(vec![intent], true, tx))
            .map_err(|_| "writer unavailable")?;
        rx.recv()
            .map_err(|_| "intent outcome uncertain; do not submit")?
            .and_then(|r| r.ok_or("missing intent reservation".into()))
    }
    pub fn snapshot(&self) -> Result<Snapshot> {
        let (tx, rx) = mpsc::channel();
        self.reader
            .send(Read::Snapshot(tx))
            .map_err(|_| "reader unavailable")?;
        rx.recv().map_err(|_| "reader unavailable")?
    }
    pub fn reconstruct(&self, assessment: &Assessment) -> Result<Library> {
        Ok(reconcile::reconstruct(&self.snapshot()?, assessment))
    }
    pub fn backup(&self, path: impl AsRef<Path>, cancel: Arc<AtomicBool>) -> Result<()> {
        let (tx, rx) = mpsc::channel();
        self.reader
            .send(Read::Backup(path.as_ref().into(), cancel, None, tx))
            .map_err(|_| "reader unavailable")?;
        rx.recv()
            .map_err(|_| "backup outcome unavailable; validate artifact")?
    }
    /// Intent commits before calling even a fake transport; no storage retry resubmits.
    pub fn handoff(
        &self,
        intent: Record,
        mut result: Record,
        transport: &mut impl Transport,
    ) -> Result<()> {
        if intent.table != "launch_attempts"
            || result.table != "handoff_records"
            || result.get("attempt_id") != intent.get("id")
        {
            return Err("handoff result must belong to reserved attempt".into());
        }
        let snapshot = self.snapshot()?;
        reconcile::supported_rule(&snapshot, result.get("transport_rule_id"), "transport")?;
        result.set("exit_status", Value::Null);
        result.set("termination_signal", Value::Null);
        result.set("reason", Value::Null);
        let reservation = self.reserve_intent(intent)?;
        if !reservation.capable {
            result.set("issuance", "not_issued");
            result.set("disposition", "not_requested");
            result.set("boundary", "preflight");
            result.set("reason", "supported handoff unavailable");
            return self.write(vec![result]);
        }
        let observed = transport.submit(&reservation.target, &reservation.launcher);
        match observed {
            Ok(r) => {
                result.set("issuance", r.issuance);
                result.set("disposition", r.disposition);
                result.set("boundary", r.boundary);
                if let Some(status) = r.status {
                    result.set("exit_status", status)
                }
                if let Some(reason) = r.reason {
                    result.set("reason", reason)
                }
            }
            Err(reason) => {
                result.set("issuance", "unknown");
                result.set("disposition", "inconclusive");
                result.set("reason", reason)
            }
        }
        self.write(vec![result]).map_err(|e| {
            format!("handoff already attempted; durable result uncertain; never replay: {e}")
        })
    }
}
fn rule_id(s: &Snapshot, kind: &str) -> Result<String> {
    s.rows("rule_versions")
        .find(|r| r.is("kind", kind) && reconcile::supported_rule(s, r.get("id"), kind).is_ok())
        .ok_or("policy missing")?
        .text("id")
        .map(String::from)
}
impl Drop for Store {
    fn drop(&mut self) {
        let _ = self.writer.send(Write::Stop);
        let _ = self.reader.send(Read::Stop);
        for h in self.workers.drain(..) {
            let _ = h.join();
        }
    }
}

pub fn runtime_version() -> &'static str {
    rusqlite::version()
}

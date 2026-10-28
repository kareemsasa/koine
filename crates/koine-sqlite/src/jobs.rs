//! Cancellable bounded writer jobs and truthful durable-outcome inspection.
use super::*;
use std::{cell::RefCell, fmt};

#[derive(Clone, Default, Debug)]
pub struct Cancellation(Arc<AtomicBool>);
impl Cancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst)
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteState {
    Committed,
    NotStarted,
    RolledBack,
    Cancelled,
    CommitUncertain,
}
#[derive(Clone, Debug)]
pub struct WriteOutcome {
    pub state: WriteState,
    pub sqlite_code: Option<i32>,
    pub error: Option<String>,
}
impl WriteOutcome {
    pub fn into_result(self) -> Result<()> {
        if self.state == WriteState::Committed {
            Ok(())
        } else {
            Err(format!(
                "{:?} (SQLite {:?}): {}",
                self.state,
                self.sqlite_code,
                self.error.unwrap_or_default()
            ))
        }
    }
}
#[derive(Debug, PartialEq, Eq)]
pub enum SubmitError {
    Full,
    Disconnected,
    FaultLabRequired,
}
impl fmt::Display for SubmitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}: job was not accepted")
    }
}
#[derive(Debug)]
pub struct WriteTicket(pub(super) Receiver<WriteOutcome>);
impl WriteTicket {
    pub fn wait(self) -> WriteOutcome {
        self.0.recv().unwrap_or(WriteOutcome {
            state: WriteState::CommitUncertain,
            sqlite_code: None,
            error: Some(
                "writer response unavailable; inspect durable identities before retry".into(),
            ),
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Inspection {
    Absent,
    Committed,
    Partial,
    Conflicting,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Point {
    BeforeBegin,
    AfterInsert(usize),
    BeforeCommit,
    BeforeRollback,
    AfterCommit,
    BackupStep(usize),
    BackupValidate,
    BackupFinish,
    BackupRead,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Phase {
    #[default]
    Queued,
    Active,
    CommitAttempted,
    Committed,
}
#[cfg(feature = "fault-injection")]
pub(crate) type Fault = Option<crate::faults::FaultPlan>;
#[cfg(not(feature = "fault-injection"))]
pub(crate) type Fault = Option<()>;
#[derive(Default)]
struct Active {
    cancel: Cancellation,
    phase: Phase,
    code: Option<i32>,
    #[cfg(feature = "fault-injection")]
    fault: Option<crate::faults::FaultPlan>,
}
thread_local! {static ACTIVE:RefCell<Active>=RefCell::new(Active::default());}
pub(crate) fn reset(cancel: Cancellation, fault: Fault) {
    let _ = &fault;
    ACTIVE.with_borrow_mut(|a| {
        *a = Active {
            cancel,
            #[cfg(feature = "fault-injection")]
            fault,
            ..Active::default()
        }
    })
}
pub(crate) fn phase(phase: Phase) {
    ACTIVE.with_borrow_mut(|a| a.phase = phase)
}
pub(crate) fn sql_error(error: &rusqlite::Error) {
    ACTIVE.with_borrow_mut(|a| a.code = error.sqlite_error().map(|e| e.extended_code))
}
#[cfg(feature = "fault-injection")]
pub(crate) fn injected_code(code: i32) {
    ACTIVE.with_borrow_mut(|a| a.code = Some(code))
}
pub(crate) fn checkpoint(point: Point) -> Result<()> {
    #[cfg(feature = "fault-injection")]
    {
        let plan = ACTIVE.with_borrow(|a| a.fault.clone());
        if let Some(plan) = plan {
            plan.hit(point)?
        }
    }
    let _ = point;
    if ACTIVE.with_borrow(|a| a.cancel.is_cancelled() && a.phase != Phase::Committed) {
        return Err("operation cancelled before commit".into());
    }
    Ok(())
}
#[cfg(feature = "fault-injection")]
pub(crate) fn corrupt_candidate(path: &Path) -> Result<()> {
    if let Some(plan) = ACTIVE.with_borrow(|a| a.fault.clone()) {
        plan.corrupt_candidate(path)?;
    }
    Ok(())
}
pub(crate) fn rollback_checkpoint() -> Result<()> {
    let _ = Point::BeforeRollback;
    #[cfg(feature = "fault-injection")]
    if let Some(plan) = ACTIVE.with_borrow(|a| a.fault.clone()) {
        plan.hit(Point::BeforeRollback)?;
    }
    Ok(())
}
pub(crate) fn receipt(result: &Result<Option<Reservation>>) -> WriteOutcome {
    ACTIVE.with_borrow(|a| {
        let state = if result.is_ok() {
            WriteState::Committed
        } else if a.phase == Phase::CommitAttempted || a.phase == Phase::Committed {
            WriteState::CommitUncertain
        } else if a.cancel.is_cancelled() {
            WriteState::Cancelled
        } else if a.phase == Phase::Active {
            WriteState::RolledBack
        } else {
            WriteState::NotStarted
        };
        WriteOutcome {
            state,
            sqlite_code: if result.is_ok() { None } else { a.code },
            error: result.as_ref().err().cloned(),
        }
    })
}
pub(crate) fn inspect(snapshot: &Snapshot, records: &[Record]) -> Result<Inspection> {
    reconcile::validate(snapshot)?;
    if records.is_empty() {
        return Err("operation inspection requires identities".into());
    }
    let mut found = 0;
    for expected in records {
        let mut expected = catalog::normalized(expected.clone())?;
        let m = CATALOG
            .iter()
            .find(|m| m.table == expected.table)
            .ok_or("unknown record")?;
        if let Some(actual) = snapshot
            .rows(&expected.table)
            .find(|r| m.keys.iter().all(|k| r.get(k) == expected.get(k)))
        {
            found += 1;
            for field in ["request_digest", "result_digest", "content_digest"] {
                if expected.get(field) == &Value::Null && m.columns.contains(&field) {
                    expected.set(field, actual.get(field).clone())
                }
            }
            if &expected != actual {
                return Ok(Inspection::Conflicting);
            }
        }
    }
    Ok(if found == 0 {
        Inspection::Absent
    } else if found == records.len() {
        Inspection::Committed
    } else {
        Inspection::Partial
    })
}

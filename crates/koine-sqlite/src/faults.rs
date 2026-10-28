//! Opt-in deterministic fault laboratory. Injection is refused for ordinary or
//! reopened stores; only newly created disposable fault-lab stores accept it.
use super::{
    jobs::{self, Point},
    *,
};
use std::sync::{Condvar, Mutex};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaultPoint {
    BeforeBegin,
    AfterInsert(usize),
    BeforeCommit,
    BeforeRollback,
    AfterCommit,
    BackupStep(usize),
    BackupValidate,
    BackupFinish,
}
impl From<FaultPoint> for Point {
    fn from(p: FaultPoint) -> Point {
        match p {
            FaultPoint::BeforeBegin => Point::BeforeBegin,
            FaultPoint::AfterInsert(n) => Point::AfterInsert(n),
            FaultPoint::BeforeCommit => Point::BeforeCommit,
            FaultPoint::BeforeRollback => Point::BeforeRollback,
            FaultPoint::AfterCommit => Point::AfterCommit,
            FaultPoint::BackupStep(n) => Point::BackupStep(n),
            FaultPoint::BackupValidate => Point::BackupValidate,
            FaultPoint::BackupFinish => Point::BackupFinish,
        }
    }
}
#[derive(Clone, Default)]
pub struct Gate(Arc<(Mutex<(bool, bool)>, Condvar)>);
impl Gate {
    pub fn wait_reached(&self) -> bool {
        let (lock, cv) = &*self.0;
        let state = lock.lock().unwrap();
        cv.wait_timeout_while(state, Duration::from_secs(5), |s| !s.0)
            .unwrap()
            .0
            .0
    }
    pub fn release(&self) {
        let (lock, cv) = &*self.0;
        lock.lock().unwrap().1 = true;
        cv.notify_all()
    }
    fn pause(&self) -> Result<()> {
        let (lock, cv) = &*self.0;
        let mut state = lock.lock().map_err(|_| "fault gate poisoned")?;
        state.0 = true;
        cv.notify_all();
        let (result, timeout) = cv
            .wait_timeout_while(state, Duration::from_secs(5), |s| !s.1)
            .map_err(|_| "fault gate poisoned")?;
        if timeout.timed_out() && !result.1 {
            return Err("fault gate timed out".into());
        }
        Ok(())
    }
}
#[derive(Clone)]
pub enum Effect {
    Pause(Gate),
    SimulatedSqlite(i32),
    LoseCommitAck,
    Cancel(Cancellation),
    RejectValidation,
    /// Damage only the newly created private backup candidate, never the source.
    CorruptBackup,
}
#[derive(Clone)]
pub struct FaultPlan {
    pub at: FaultPoint,
    pub effect: Effect,
    events: Arc<Mutex<Vec<String>>>,
}
impl FaultPlan {
    pub fn new(at: FaultPoint, effect: Effect) -> Self {
        Self {
            at,
            effect,
            events: Arc::default(),
        }
    }
    pub fn events(&self) -> Vec<String> {
        self.events.lock().unwrap().clone()
    }
    pub(crate) fn corrupt_candidate(&self, path: &Path) -> Result<()> {
        if self.at == FaultPoint::BackupValidate && matches!(self.effect, Effect::CorruptBackup) {
            use std::io::Write;
            let mut file = io(OpenOptions::new().write(true).open(path))?;
            file.write_all(b"INVALID-COPY!!!!")
                .map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    pub(crate) fn hit(&self, point: Point) -> Result<()> {
        self.events
            .lock()
            .map_err(|_| "fault trace poisoned")?
            .push(format!("{point:?}"));
        if point != Point::from(self.at) {
            return Ok(());
        }
        match &self.effect {
            Effect::Pause(gate) => gate.pause(),
            Effect::SimulatedSqlite(code) => {
                jobs::injected_code(*code);
                Err(format!("SIMULATED SQLite result {code}"))
            }
            Effect::LoseCommitAck => {
                Err("SIMULATED lost commit acknowledgement; inspect durable identities".into())
            }
            Effect::Cancel(cancel) => {
                cancel.cancel();
                Ok(())
            }
            Effect::CorruptBackup => Ok(()),
            Effect::RejectValidation => Err("SIMULATED backup validation rejection".into()),
        }
    }
}

//! Incremental copy-only backup with checked completion and reader fairness.
use super::*;

/// Raw backup wrapper checks finish's result (rusqlite's Drop cannot report it).
struct BackupHandle<'a> {
    ptr: *mut rusqlite::ffi::sqlite3_backup,
    _lifetime: std::marker::PhantomData<(&'a Connection, &'a mut Connection)>,
}
impl<'a> BackupHandle<'a> {
    fn new(from: &'a Connection, to: &'a mut Connection) -> Result<Self> {
        // SAFETY: Both connections outlive the handle; destination remains exclusively
        // borrowed until finish. Names are static, NUL-terminated SQLite schema names.
        let ptr = unsafe {
            rusqlite::ffi::sqlite3_backup_init(
                to.handle(),
                c"main".as_ptr(),
                from.handle(),
                c"main".as_ptr(),
            )
        };
        if ptr.is_null() {
            return Err("backup initialization failed".into());
        }
        Ok(Self {
            ptr,
            _lifetime: std::marker::PhantomData,
        })
    }
    fn step(&self) -> i32 {
        // SAFETY: handle is live until consuming finish; no concurrent access.
        unsafe { rusqlite::ffi::sqlite3_backup_step(self.ptr, 32) }
    }
    fn finish(mut self) -> Result<()> {
        // SAFETY: consume the uniquely owned live handle exactly once.
        let code = unsafe { rusqlite::ffi::sqlite3_backup_finish(self.ptr) };
        self.ptr = std::ptr::null_mut();
        if code != rusqlite::ffi::SQLITE_OK {
            return Err(format!("backup finish failed: {code}"));
        }
        Ok(())
    }
}
impl Drop for BackupHandle<'_> {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            // SAFETY: abandoned handle is uniquely owned and still live.
            unsafe {
                rusqlite::ffi::sqlite3_backup_finish(self.ptr);
            }
        }
    }
}
pub(super) fn backup(
    source: &Connection,
    path: &Path,
    cancel: &AtomicBool,
    requests: &Receiver<Read>,
    stopping: &mut bool,
) -> Result<()> {
    if path.exists() {
        return Err("backup destination already exists".into());
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let partial = parent.join(format!(".koine-{}.partial", id()));
    io(OpenOptions::new()
        .write(true)
        .mode(0o600)
        .create_new(true)
        .open(&partial))?;
    let mut dest = open_rw(&partial)?;
    deadline();
    let handle = BackupHandle::new(source, &mut dest)?;
    let mut steps = 0;
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err("backup cancelled; incomplete artifact is not usable".into());
        }
        steps += 1;
        let code = handle.step();
        jobs::checkpoint(jobs::Point::BackupStep(steps))?;
        match code {
            rusqlite::ffi::SQLITE_DONE => break,
            rusqlite::ffi::SQLITE_OK => {}
            rusqlite::ffi::SQLITE_BUSY | rusqlite::ffi::SQLITE_LOCKED if waiting() => {
                lock_pause();
            }
            code => return Err(format!("backup incomplete: {code}")),
        }
        // Serve an ordinary read between bounded steps. Another backup is rejected
        // explicitly; no accepted request silently disappears.
        if let Ok(message) = requests.try_recv() {
            match message {
                Read::Snapshot(reply) => {
                    let saved = BUDGET.get();
                    let _ = reply.send(read_snapshot(source));
                    jobs::checkpoint(jobs::Point::BackupRead)?;
                    BUDGET.set(saved);
                }
                Read::Backup(_, _, _, reply) => {
                    let _ = reply.send(Err("backup already active".into()));
                }
                Read::Stop => {
                    *stopping = true;
                    return Err("shutdown cancelled backup".into());
                }
            }
        }
        thread::yield_now();
    }
    handle.finish()?;
    jobs::checkpoint(jobs::Point::BackupFinish)?;
    dest.close().map_err(|(_, e)| e.to_string())?;
    #[cfg(feature = "fault-injection")]
    jobs::corrupt_candidate(&partial)?;
    jobs::checkpoint(jobs::Point::BackupValidate)?;
    if cancel.load(Ordering::Relaxed) {
        return Err("backup cancelled before finalization".into());
    }
    let check = open_rw(&partial)?;
    check_identity(&check)?;
    integrity(&check)?;
    reconcile::validate(&load(&check)?)?;
    check.close().map_err(|(_, e)| e.to_string())?;
    io(File::open(&partial))?
        .sync_all()
        .map_err(|e| e.to_string())?;
    // Hard-link publication is atomic and fails if the destination appeared meanwhile.
    io(std::fs::hard_link(&partial, path))?;
    io(File::open(parent))?
        .sync_all()
        .map_err(|e| e.to_string())?;
    // Keep the private staging name too: no deletion/rotation path is introduced.
    Ok(())
}

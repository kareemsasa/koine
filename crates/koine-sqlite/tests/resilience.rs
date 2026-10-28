#![cfg(feature = "fault-injection")]
use koine_core::*;
use koine_sqlite::{faults::*, *};
use rusqlite::{Connection, params};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
fn context(key: &str) -> Record {
    Record::new(
        "contexts",
        [
            ("id", id().into()),
            ("kind", "subject".into()),
            ("namespace", "fault-fixture".into()),
            ("resolution", "identified".into()),
            ("external_key", key.into()),
        ],
    )
}
fn lab() -> (Store, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::create_fault_lab(dir.path().join("lab.sqlite")).unwrap();
    (store, dir)
}
#[test]
fn saturated_queue_rejects_explicitly_and_cancelled_accepted_job_reports_outcome() {
    let (store, _dir) = lab();
    let gate = Gate::default();
    let first = store
        .fault_submit(
            vec![context("active")],
            Cancellation::default(),
            FaultPlan::new(FaultPoint::AfterInsert(1), Effect::Pause(gate.clone())),
        )
        .unwrap();
    assert!(gate.wait_reached());
    assert_eq!(store.snapshot().unwrap().rows("contexts").count(), 0);
    let cancel = Cancellation::default();
    let mut queued = Vec::new();
    for n in 0..32 {
        queued.push(
            store
                .try_submit(
                    vec![context(&format!("queued-{n}"))],
                    if n == 0 {
                        cancel.clone()
                    } else {
                        Cancellation::default()
                    },
                )
                .unwrap(),
        );
    }
    assert_eq!(
        store
            .try_submit(vec![context("rejected")], Cancellation::default())
            .unwrap_err(),
        SubmitError::Full
    );
    cancel.cancel();
    gate.release();
    assert_eq!(first.wait().state, WriteState::Committed);
    assert_eq!(queued.remove(0).wait().state, WriteState::Cancelled);
    for job in queued {
        assert_eq!(job.wait().state, WriteState::Committed)
    }
    assert_eq!(store.snapshot().unwrap().rows("contexts").count(), 32);
}
#[test]
fn cancellation_before_start_and_before_commit_never_commits() {
    let (store, _) = lab();
    let cancel = Cancellation::default();
    cancel.cancel();
    assert_eq!(
        store
            .try_submit(vec![context("cancelled")], cancel)
            .unwrap()
            .wait()
            .state,
        WriteState::Cancelled
    );
    let cancel = Cancellation::default();
    let outcome = store
        .fault_submit(
            vec![context("rollback")],
            cancel.clone(),
            FaultPlan::new(FaultPoint::BeforeCommit, Effect::Cancel(cancel)),
        )
        .unwrap()
        .wait();
    assert_eq!(outcome.state, WriteState::Cancelled);
    assert_eq!(store.snapshot().unwrap().rows("contexts").count(), 0);
    store.write(vec![context("usable")]).unwrap();
}
#[test]
fn cancellation_after_known_commit_is_too_late_not_rollback() {
    let (store, _) = lab();
    let rows = vec![context("committed")];
    let cancel = Cancellation::default();
    let outcome = store
        .fault_submit(
            rows.clone(),
            cancel.clone(),
            FaultPlan::new(FaultPoint::AfterCommit, Effect::Cancel(cancel)),
        )
        .unwrap()
        .wait();
    assert_eq!(outcome.state, WriteState::Committed);
    assert_eq!(
        store.inspect_operation(&rows).unwrap(),
        Inspection::Committed
    );
}
#[test]
fn simulated_full_io_and_snapshot_errors_roll_back_without_replay() {
    for code in [
        rusqlite::ffi::SQLITE_FULL,
        rusqlite::ffi::SQLITE_IOERR_WRITE,
        rusqlite::ffi::SQLITE_BUSY_SNAPSHOT,
    ] {
        let (store, _) = lab();
        let rows = vec![context("first"), context("second")];
        let outcome = store
            .fault_submit(
                rows.clone(),
                Cancellation::default(),
                FaultPlan::new(FaultPoint::AfterInsert(1), Effect::SimulatedSqlite(code)),
            )
            .unwrap()
            .wait();
        assert_eq!(outcome.state, WriteState::RolledBack);
        assert_eq!(outcome.sqlite_code, Some(code));
        assert!(outcome.error.unwrap().contains("SIMULATED"));
        assert_eq!(store.inspect_operation(&rows).unwrap(), Inspection::Absent);
        store.write(rows).unwrap();
        assert_eq!(store.snapshot().unwrap().rows("contexts").count(), 2);
    }
}
#[test]
fn actual_sqlite_full_is_observed_and_worker_recovers() {
    let (store, _) = lab();
    let current = store.fault_limit_pages(1).unwrap();
    assert!(current > 1);
    let rows = vec![context(&"x".repeat(2_000_000))];
    let outcome = store
        .try_submit(rows.clone(), Cancellation::default())
        .unwrap()
        .wait();
    assert_eq!(outcome.sqlite_code, Some(rusqlite::ffi::SQLITE_FULL));
    assert_eq!(outcome.state, WriteState::RolledBack);
    assert_eq!(store.inspect_operation(&rows).unwrap(), Inspection::Absent);
    store.fault_limit_pages(current + 4096).unwrap();
    store.write(vec![context("after-full")]).unwrap();
}
#[test]
fn actual_busy_snapshot_requires_abandoning_read_transaction() {
    let (store, dir) = lab();
    let path = dir.path().join("lab.sqlite");
    let c = Connection::open(&path).unwrap();
    c.execute_batch("BEGIN; SELECT count(*) FROM contexts;")
        .unwrap();
    store.write(vec![context("other-writer")]).unwrap();
    let error=c.execute("INSERT INTO contexts(id,kind,namespace,resolution,external_key) VALUES(?1,'subject','fault-fixture','identified','stale-snapshot')",[id()]).unwrap_err();
    assert_eq!(
        error.sqlite_error().unwrap().extended_code,
        rusqlite::ffi::SQLITE_BUSY_SNAPSHOT
    );
    assert!(!c.is_autocommit());
    c.execute_batch("ROLLBACK").unwrap();
    assert!(c.is_autocommit());
    assert_eq!(store.snapshot().unwrap().rows("contexts").count(), 1);
}
#[test]
fn actual_busy_cancellation_and_deadline_do_not_replay() {
    let (store, dir) = lab();
    let c = Connection::open(dir.path().join("lab.sqlite")).unwrap();
    c.execute_batch("BEGIN IMMEDIATE").unwrap();
    let cancel = Cancellation::default();
    let ticket = store
        .try_submit(vec![context("blocked")], cancel.clone())
        .unwrap();
    cancel.cancel();
    assert_eq!(ticket.wait().state, WriteState::Cancelled);
    let outcome = store
        .try_submit(vec![context("expiry")], Cancellation::default())
        .unwrap()
        .wait();
    assert_eq!(outcome.state, WriteState::NotStarted);
    assert_eq!(outcome.sqlite_code, Some(rusqlite::ffi::SQLITE_BUSY));
    c.execute_batch("ROLLBACK").unwrap();
    assert!(store.snapshot().unwrap().rows("contexts").next().is_none());
}
#[test]
fn lost_commit_ack_is_resolved_by_exact_durable_identity_before_retry() {
    let (store, dir) = lab();
    let rows = vec![context("ack-lost")];
    let outcome = store
        .fault_submit(
            rows.clone(),
            Cancellation::default(),
            FaultPlan::new(FaultPoint::AfterCommit, Effect::LoseCommitAck),
        )
        .unwrap()
        .wait();
    assert_eq!(outcome.state, WriteState::CommitUncertain);
    assert_eq!(
        store.inspect_operation(&rows).unwrap(),
        Inspection::Committed
    );
    let mut changed = rows.clone();
    changed[0].set("external_key", "different-body");
    assert_eq!(
        store.inspect_operation(&changed).unwrap(),
        Inspection::Conflicting
    );
    let path = dir.path().join("lab.sqlite");
    drop(store);
    let reopened = Store::open(path).unwrap();
    assert_eq!(
        reopened.inspect_operation(&rows).unwrap(),
        Inspection::Committed
    );
    assert_eq!(reopened.snapshot().unwrap().rows("contexts").count(), 1);
}
#[test]
fn operation_inspection_distinguishes_absent_partial_and_conflicting() {
    let (store, _) = lab();
    let rows = vec![context("one"), context("two")];
    assert_eq!(store.inspect_operation(&rows).unwrap(), Inspection::Absent);
    store.write(vec![rows[0].clone()]).unwrap();
    assert_eq!(store.inspect_operation(&rows).unwrap(), Inspection::Partial);
    store.write(vec![rows[1].clone()]).unwrap();
    assert_eq!(
        store.inspect_operation(&rows).unwrap(),
        Inspection::Committed
    );
}
#[test]
fn interrupted_backup_services_reads_and_cancel_never_finalizes_partial_copy() {
    let (store, dir) = lab();
    let c = Connection::open(dir.path().join("lab.sqlite")).unwrap();
    let pages: i64 = c.query_row("PRAGMA page_count", [], |r| r.get(0)).unwrap();
    assert!(pages > 32);
    drop(c);
    let gate = Gate::default();
    let fault = FaultPlan::new(FaultPoint::BackupStep(1), Effect::Pause(gate.clone()));
    let cancel = Arc::new(AtomicBool::new(false));
    let dest = dir.path().join("never-final.sqlite");
    std::thread::scope(|scope| {
        let handle = scope.spawn(|| store.fault_backup(&dest, cancel.clone(), fault.clone()));
        assert!(gate.wait_reached());
        let read = store.request_snapshot().unwrap();
        cancel.store(true, Ordering::SeqCst);
        gate.release();
        read.recv_timeout(std::time::Duration::from_secs(3))
            .unwrap()
            .unwrap();
        assert!(handle.join().unwrap().is_err());
    });
    assert!(!dest.exists());
    assert!(fault.events().iter().any(|e| e == "BackupRead"));
    store
        .backup(&dest, Arc::new(AtomicBool::new(false)))
        .unwrap();
}
#[test]
fn failed_backup_finish_and_validation_preserve_source_and_last_good_copy() {
    for point in [FaultPoint::BackupFinish, FaultPoint::BackupValidate] {
        let (store, dir) = lab();
        store.write(vec![context("preserve")]).unwrap();
        let good = dir.path().join("good.sqlite");
        store
            .backup(&good, Arc::new(AtomicBool::new(false)))
            .unwrap();
        let before = std::fs::read(&good).unwrap();
        let snapshot = store.snapshot().unwrap();
        let failed = dir.path().join("failed.sqlite");
        let result = store.fault_backup(
            &failed,
            Arc::new(AtomicBool::new(false)),
            FaultPlan::new(point, Effect::RejectValidation),
        );
        assert!(result.unwrap_err().contains("SIMULATED"));
        assert!(!failed.exists());
        assert_eq!(std::fs::read(&good).unwrap(), before);
        assert_eq!(store.snapshot().unwrap(), snapshot);
    }
}
#[test]
fn fault_injection_refuses_regular_and_reopened_databases() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ordinary.sqlite");
    let store = Store::create(&path).unwrap();
    assert_eq!(
        store
            .fault_submit(
                vec![],
                Cancellation::default(),
                FaultPlan::new(FaultPoint::BeforeCommit, Effect::LoseCommitAck)
            )
            .unwrap_err(),
        SubmitError::FaultLabRequired
    );
    drop(store);
    let reopened = Store::open(&path).unwrap();
    assert!(reopened.fault_limit_pages(1).is_err());
}
#[test]
fn actual_deferred_seal_failure_keeps_transaction_active_until_rollback() {
    let (store, dir) = lab();
    let c = Connection::open(dir.path().join("lab.sqlite")).unwrap();
    c.execute_batch("PRAGMA foreign_keys=ON; BEGIN IMMEDIATE")
        .unwrap();
    let rid = id();
    c.execute("INSERT INTO rule_versions(id,kind,name,version,artifact_digest) VALUES(?1,'reconciliation','test-only','1',?2)",params![rid,vec![0u8;32]]).unwrap();
    c.execute("INSERT INTO reconciliation_decisions(id,kind,rule_id,assessed_at,reason) VALUES(?1,'dispute',?2,1,'synthetic')",params![id(),rid]).unwrap();
    let error = c.execute_batch("COMMIT").unwrap_err();
    assert_eq!(
        error.sqlite_error().unwrap().code,
        rusqlite::ErrorCode::ConstraintViolation
    );
    assert_eq!(
        error.sqlite_error().unwrap().extended_code,
        rusqlite::ffi::SQLITE_CONSTRAINT_FOREIGNKEY
    );
    assert!(!c.is_autocommit());
    c.execute_batch("ROLLBACK").unwrap();
    assert!(
        store
            .snapshot()
            .unwrap()
            .rows("reconciliation_decisions")
            .next()
            .is_none()
    );
}

#[test]
fn corrupted_backup_candidate_fails_real_validation_and_next_copy_recovers() {
    let (store, dir) = lab();
    store.write(vec![context("intact-source")]).unwrap();
    let before = store.snapshot().unwrap();
    let dest = dir.path().join("bad.sqlite");
    let error = store
        .fault_backup(
            &dest,
            Arc::new(AtomicBool::new(false)),
            FaultPlan::new(FaultPoint::BackupValidate, Effect::CorruptBackup),
        )
        .unwrap_err();
    assert!(!error.contains("SIMULATED"));
    assert!(!dest.exists());
    assert_eq!(before, store.snapshot().unwrap());
    let partial = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|e| e == "partial"))
        .unwrap();
    assert!(Store::open(&partial).is_err());
    store
        .backup(&dest, Arc::new(AtomicBool::new(false)))
        .unwrap();
    assert_eq!(before, Store::open(&dest).unwrap().snapshot().unwrap());
}

#[test]
fn pinned_reader_allows_wal_growth_past_threshold_then_checkpoint_recovers() {
    let (store, dir) = lab();
    store.checkpoint().unwrap();
    let c = Connection::open(dir.path().join("lab.sqlite")).unwrap();
    c.execute_batch("BEGIN; SELECT count(*) FROM contexts")
        .unwrap();
    store
        .write(
            (0..8)
                .map(|n| context(&format!("{n}{}", "x".repeat(700_000))))
                .collect(),
        )
        .unwrap();
    let progress = store.checkpoint().unwrap();
    assert!(progress.wal_pages > 1000);
    assert!(progress.checkpointed_pages < progress.wal_pages);
    assert!(
        std::fs::metadata(dir.path().join("lab.sqlite-wal"))
            .unwrap()
            .len()
            > 4_000_000
    );
    assert_eq!(store.snapshot().unwrap().rows("contexts").count(), 8);
    c.execute_batch("ROLLBACK").unwrap();
    let done = store.checkpoint().unwrap();
    assert_eq!(done.wal_pages, done.checkpointed_pages);
}

#[test]
fn injected_rollback_failure_quarantines_writer_until_reopen_and_inspection() {
    let (store, dir) = lab();
    let mut invalid = context("invalid");
    invalid.set("kind", "not-a-kind");
    let outcome = store
        .fault_submit(
            vec![invalid],
            Cancellation::default(),
            FaultPlan::new(
                FaultPoint::BeforeRollback,
                Effect::SimulatedSqlite(rusqlite::ffi::SQLITE_IOERR),
            ),
        )
        .unwrap()
        .wait();
    assert_eq!(outcome.state, WriteState::CommitUncertain);
    // Either rejected on submission or accepted during quarantine and disconnected;
    // no subsequent body may run inside the unresolved transaction.
    match store.try_submit(vec![context("must-not-run")], Cancellation::default()) {
        Err(SubmitError::Disconnected) => {}
        Ok(ticket) => assert_eq!(ticket.wait().state, WriteState::CommitUncertain),
        other => panic!("unexpected result {other:?}"),
    }
    let path = dir.path().join("lab.sqlite");
    drop(store);
    let reopened = Store::open(path).unwrap();
    assert_eq!(reopened.snapshot().unwrap().rows("contexts").count(), 0);
    reopened.write(vec![context("new-worker")]).unwrap();
}

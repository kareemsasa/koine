#![cfg(feature = "fault-injection")]
use koine_core::*;
use koine_sqlite::Store;
use koine_steam::{config::Configuration, integration, scheduler::Scheduler};
use std::{
    fs,
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};
struct Probe(Child);
impl Drop for Probe {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn kill_boundary(phase: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("synthetic");
    fs::create_dir_all(root.join("steamapps")).unwrap();
    fs::write(
        root.join(".koine-synthetic-root"),
        format!("koine-fixture-v1\n{}\n", id()),
    )
    .unwrap();
    fs::write(
        root.join("steamapps/libraryfolders.vdf"),
        format!(
            "\"libraryfolders\" {{\"0\" {{\"path\" \"{}\"}}}}",
            root.display()
        ),
    )
    .unwrap();
    fs::write(root.join("steamapps/appmanifest_42.acf"),"\"AppState\" {\"appid\" \"42\" \"installdir\" \"Synthetic\" \"name\" \"Synthetic\" \"StateFlags\" \"4\"}").unwrap();
    let db = dir.path().join("crash.sqlite");
    let mut child = Probe(
        Command::new(env!("CARGO_BIN_EXE_koine-steam-crash-probe"))
            .arg(&db)
            .arg(&root)
            .arg(phase)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let out = child.0.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut line = String::new();
        BufReader::new(out).read_line(&mut line).unwrap();
        tx.send(line).unwrap();
    });
    let line = rx.recv_timeout(Duration::from_secs(10)).unwrap();
    assert!(line.contains(phase));
    child.0.kill().unwrap();
    child.0.wait().unwrap();
    reader.join().unwrap();
    fs::remove_dir_all(root).unwrap();
    (dir, db)
}
#[test]
fn sigkill_after_registration_preserves_original_pending_scope_and_recovers_inconclusive() {
    let (_dir, db) = kill_boundary("registered");
    let store = Arc::new(Store::open(&db).unwrap());
    let before = store.snapshot().unwrap();
    assert_eq!(before.rows("collection_closures").count(), 0);
    let settings = Arc::new(Mutex::new(Configuration::open(&store, &db).unwrap()));
    let scheduler = Scheduler::new(store.clone(), settings, &db, 20).unwrap();
    let s = store.snapshot().unwrap();
    assert_eq!(
        before.rows("requested_units").cloned().collect::<Vec<_>>(),
        s.rows("requested_units").cloned().collect::<Vec<_>>()
    );
    assert!(
        s.rows("collection_closures")
            .all(|c| c.is("outcome", "inconclusive"))
    );
    assert_eq!(s.rows("evidence_records").count(), 0);
    drop(scheduler);
    drop(store);
    Store::open(db).unwrap();
}
#[test]
fn sigkill_after_durable_journal_recovers_observations_without_source_reread() {
    let (_dir, db) = kill_boundary("journaled");
    let store = Arc::new(Store::open(&db).unwrap());
    assert_eq!(
        store.snapshot().unwrap().rows("evidence_records").count(),
        0
    );
    let settings = Arc::new(Mutex::new(Configuration::open(&store, &db).unwrap()));
    let scheduler = Scheduler::new(store.clone(), settings, &db, 20).unwrap();
    let s = store.snapshot().unwrap();
    assert_eq!(s.rows("evidence_records").count(), 1);
    assert!(
        s.rows("evidence_records")
            .all(|e| e.int("collected_at").unwrap() == 10)
    );
    assert!(
        s.rows("collection_closures")
            .all(|c| c.is("outcome", "complete"))
    );
    let assessment = integration::assessment(&store, 20).unwrap();
    let view = store.reconstruct(&assessment).unwrap();
    drop(scheduler);
    drop(store);
    let reopened = Store::open(db).unwrap();
    assert_eq!(view, reopened.reconstruct(&assessment).unwrap());
}

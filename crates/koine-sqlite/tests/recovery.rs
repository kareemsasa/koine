mod common;
use common::*;
use koine_core::*;
use koine_sqlite::Store;
use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
};
#[test]
fn killed_process_before_and_after_commit_preserves_boundary_meaning() {
    for (mode, exists, disposition) in [
        ("uncommitted-intent", false, Handoff::Inconclusive),
        ("committed-intent", true, Handoff::Inconclusive),
        ("committed-failure", true, Handoff::Failed),
    ] {
        let mut f = Fixture::new();
        let (_, _, _, target) = f.observe(
            "fixture:steam",
            "1549690",
            None,
            None,
            None,
            Some("supported"),
            1,
        );
        let aid = id();
        let rule = f.rules["transport"].clone();
        let assessment = f.assessment(25, true);
        let path = f.path.clone();
        let target = target.unwrap();
        drop(f.store);
        let mut child = Command::new(env!("CARGO_BIN_EXE_koine-crash-probe"))
            .arg(mode)
            .arg(&path)
            .arg(&aid)
            .arg(&target)
            .arg(&rule)
            .current_dir(f.dir.path())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        assert_eq!(line.trim(), "READY");
        child.kill().unwrap();
        assert!(!child.wait().unwrap().success());
        let store = Store::open(&path).unwrap();
        let snapshot = store.snapshot().unwrap();
        assert_eq!(
            snapshot.rows("launch_attempts").any(|r| r.is("id", &aid)),
            exists
        );
        let view = store.reconstruct(&assessment).unwrap();
        if exists {
            let attempt = view.attempts.iter().find(|a| a.attempt == aid).unwrap();
            assert_eq!(attempt.handoff, disposition);
            assert_eq!(attempt.process, Process::Unobserved);
            if mode == "committed-failure" {
                assert_eq!(attempt.exit_status, Some(127))
            } else {
                assert_eq!(attempt.issuance, "unknown")
            }
        }
    }
}
#[test]
fn interrupted_result_transaction_keeps_original_scope_and_previous_evidence() {
    let mut f = Fixture::new();
    let (_, i, _, _) = f.observe("fixture:steam", "app", Some("present"), None, None, None, 1);
    let pending = f.report(
        "fixture:steam",
        &[("root-A", "complete")],
        "complete",
        2,
        Some(2),
    );
    let before = f.store.snapshot().unwrap();
    let c = rusqlite::Connection::open(&f.path).unwrap();
    c.execute_batch("PRAGMA foreign_keys=ON; BEGIN IMMEDIATE")
        .unwrap();
    let result = &pending.rows[0];
    let fields: Vec<_> = result.fields.keys().collect();
    let values: Vec<_> = result
        .fields
        .values()
        .map(|v| match v {
            Value::Text(t) => rusqlite::types::Value::Text(t.clone()),
            Value::Integer(n) => rusqlite::types::Value::Integer(*n),
            _ => rusqlite::types::Value::Null,
        })
        .collect();
    c.execute(
        &format!(
            "INSERT INTO coverage_results({}) VALUES({})",
            fields
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(","),
            vec!["?"; fields.len()].join(",")
        ),
        rusqlite::params_from_iter(values),
    )
    .unwrap();
    drop(c);
    assert_eq!(before, f.store.snapshot().unwrap());
    assert_eq!(
        f.conclusion(i.as_ref().unwrap(), "presence", 3).knowledge,
        Knowledge::Observed("present".into())
    );
    assert_eq!(
        f.view(3)
            .collections
            .iter()
            .find(|c| c.collection == pending.id)
            .unwrap()
            .outcome,
        "inconclusive"
    );
}

#[test]
fn process_kill_during_and_after_collection_result_keeps_atomic_boundary() {
    for committed in [false, true] {
        let mut f = Fixture::new();
        let (_, _, _, _) = f.observe("fixture:steam", "app", Some("present"), None, None, None, 1);
        let pending = f.report(
            "fixture:steam",
            &[("root-A", "complete")],
            "complete",
            2,
            Some(2),
        );
        let before = f.store.snapshot().unwrap();
        let path = f.path.clone();
        drop(f.store);
        let mut child = Command::new(env!("CARGO_BIN_EXE_koine-crash-probe"))
            .arg(if committed {
                "committed-result"
            } else {
                "uncommitted-result"
            })
            .arg(&path)
            .arg(&pending.id)
            .arg("unused")
            .arg("unused")
            .current_dir(f.dir.path())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        assert_eq!(line.trim(), "READY");
        child.kill().unwrap();
        assert!(!child.wait().unwrap().success());
        let store = Store::open(&path).unwrap();
        let after = store.snapshot().unwrap();
        if !committed {
            assert_eq!(before, after);
        }
        assert_eq!(
            after
                .rows("collection_closures")
                .any(|c| c.is("collection_id", &pending.id)),
            committed
        );
        let assessment = Assessment {
            at: 30,
            observer_epoch: None,
            reconciliation_rule: f.rules["reconciliation"].clone(),
            freshness_rule: f.rules["freshness"].clone(),
        };
        let view = store.reconstruct(&assessment).unwrap();
        assert!(
            view.claims.iter().any(|c| c.dimension == "presence"
                && c.knowledge == Knowledge::Observed("present".into()))
        );
        assert_eq!(
            view.collections
                .iter()
                .find(|c| c.collection == pending.id)
                .unwrap()
                .outcome,
            if committed {
                "complete"
            } else {
                "inconclusive"
            }
        );
        for old in before.rows("claims") {
            assert_eq!(old, after.by_id("claims", old.get("id")).unwrap());
        }
    }
}

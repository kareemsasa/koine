mod common;
use common::*;
use koine_core::{canonical, reconcile, *};
use koine_sqlite::{BOOTSTRAP, Store};
use rusqlite::{Connection, params, params_from_iter, types::Value as SqlValue};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
fn connection(f: &Fixture) -> Connection {
    let c = Connection::open(&f.path).unwrap();
    c.execute_batch("PRAGMA foreign_keys=ON; PRAGMA recursive_triggers=ON;")
        .unwrap();
    c
}
fn raw_insert(c: &Connection, r: &Record) -> rusqlite::Result<usize> {
    let cols: Vec<_> = r.fields.keys().collect();
    let values: Vec<_> = r
        .fields
        .values()
        .map(|v| match v {
            Value::Null => SqlValue::Null,
            Value::Integer(n) => SqlValue::Integer(*n),
            Value::Text(t) => SqlValue::Text(t.clone()),
            Value::Blob(b) => SqlValue::Blob(b.clone()),
            Value::List(_) => panic!("encode first"),
        })
        .collect();
    c.execute(
        &format!(
            "INSERT INTO {} ({}) VALUES ({})",
            r.table,
            cols.iter()
                .map(|c| c.as_str())
                .collect::<Vec<_>>()
                .join(","),
            vec!["?"; cols.len()].join(",")
        ),
        params_from_iter(values),
    )
}
#[test]
fn bootstrap_is_strict_and_has_full_relational_catalog() {
    let f = Fixture::new();
    let c = connection(&f);
    let strict:i64=c.query_row("SELECT count(*) FROM pragma_table_list WHERE schema='main' AND name NOT LIKE 'sqlite_%' AND strict=1",[],|r|r.get(0)).unwrap();
    assert_eq!(strict, 35);
    let check: String = c
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .unwrap();
    assert_eq!(check, "ok");
    assert!(
        !c.prepare("PRAGMA foreign_key_check")
            .unwrap()
            .exists([])
            .unwrap()
    );
    assert_eq!(
        c.query_row("PRAGMA journal_mode", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "wal"
    );
}
#[test]
fn immutable_updates_deletes_and_replacement_inserts_rejected() {
    let mut f = Fixture::new();
    f.observe("fixture:steam", "app", Some("present"), None, None, None, 1);
    let c = connection(&f);
    for command in [
        "UPDATE claims SET basis_code='tampered'",
        "DELETE FROM claims",
        "INSERT OR REPLACE INTO contexts SELECT * FROM contexts LIMIT 1",
    ] {
        let error = c.execute_batch(command).unwrap_err().to_string();
        assert!(
            error.contains("immutable") || error.contains("duplicate"),
            "{error}"
        )
    }
    let r = row(
        "launch_attempts",
        [
            ("id", id().into()),
            ("target_id", id().into()),
            ("created_at", 1i64.into()),
            ("intent_reason", "synthetic".into()),
        ],
    );
    assert!(
        raw_insert(&c, &r)
            .unwrap_err()
            .to_string()
            .contains("FOREIGN KEY")
    );
}
#[test]
fn closed_collections_reject_late_children_and_request_shrinking() {
    let mut f = Fixture::new();
    f.observe("fixture:steam", "app", None, None, None, None, 1);
    let c = connection(&f);
    let s = f.store.snapshot().unwrap();
    let collection = s.rows("collections").next().unwrap().text("id").unwrap();
    let r = row(
        "diagnostics",
        [
            ("id", id().into()),
            ("collection_id", collection.into()),
            ("item_key", id().into()),
            ("target_kind", "collection".into()),
            ("severity", "warning".into()),
            ("code", "late".into()),
            ("message", "synthetic".into()),
            ("recorded_at", 2i64.into()),
        ],
    );
    assert!(
        raw_insert(&c, &r)
            .unwrap_err()
            .to_string()
            .contains("closed collection")
    );
    assert!(
        c.execute(
            "DELETE FROM requested_units WHERE collection_id=?1",
            [collection]
        )
        .is_err()
    );
}
#[test]
fn unsealed_decisions_cannot_commit_and_sealed_children_cannot_change() {
    let mut f = Fixture::new();
    f.observe("fixture:steam", "app", Some("present"), None, None, None, 1);
    f.observe("fixture:steam", "app", Some("present"), None, None, None, 2);
    let c = connection(&f);
    let did = id();
    c.execute_batch("BEGIN IMMEDIATE").unwrap();
    c.execute("INSERT INTO reconciliation_decisions(id,kind,rule_id,assessed_at,reason) VALUES(?1,'dispute',?2,2,'synthetic')",params![did,f.rules["reconciliation"]]).unwrap();
    assert!(
        c.execute_batch("COMMIT")
            .unwrap_err()
            .to_string()
            .contains("FOREIGN KEY")
    );
    c.execute_batch("ROLLBACK").unwrap();
    let snap = f.store.snapshot().unwrap();
    let sealed = snap.rows("decision_seals").next().unwrap();
    let claim = snap.rows("claims").next().unwrap();
    let r = row(
        "decision_claims",
        [
            ("decision_id", sealed.get("decision_id").clone()),
            ("claim_id", claim.get("id").clone()),
            ("role", "ordering_support".into()),
        ],
    );
    assert!(
        raw_insert(&c, &r)
            .unwrap_err()
            .to_string()
            .contains("sealed decision")
    );
}
#[test]
fn bad_seals_incomplete_decisions_and_partial_results_rollback() {
    let mut f = Fixture::new();
    let (e, _, _, _) = f.observe("fixture:steam", "app", None, None, None, None, 1);
    let count = f.store.snapshot().unwrap().records.len();
    let d = id();
    let rows = vec![
        row(
            "reconciliation_decisions",
            [
                ("id", d.clone().into()),
                ("kind", "dispute".into()),
                ("rule_id", f.rules["reconciliation"].clone().into()),
                ("assessed_at", 2i64.into()),
                ("reason", "missing conflict evidence".into()),
            ],
        ),
        row(
            "decision_seals",
            [
                ("decision_id", d.into()),
                ("protocol", "koine.decision.v1".into()),
                ("content_digest", vec![0; 32].into()),
            ],
        ),
    ];
    assert!(f.store.write(rows).is_err());
    assert_eq!(f.store.snapshot().unwrap().records.len(), count);
    let mut b = f.report(
        "fixture:steam",
        &[("root-A", "complete")],
        "complete",
        2,
        Some(2),
    );
    f.claim(&mut b, &e, "entry_title", Some("new"), "root-A", false);
    assert!(
        f.store
            .write(b.rows)
            .unwrap_err()
            .contains("atomic closure")
    );
}
#[test]
fn cycle_guard_rejects_reverse_edge_before_commit() {
    let mut f = Fixture::new();
    f.observe("fixture:steam", "app", Some("present"), None, None, None, 1);
    f.observe("fixture:steam", "app", Some("present"), None, None, None, 2);
    let s = f.store.snapshot().unwrap();
    let edge = s.rows("supersessions").next().unwrap();
    let c = connection(&f);
    let d = id();
    c.execute_batch("BEGIN IMMEDIATE").unwrap();
    c.execute("INSERT INTO reconciliation_decisions(id,kind,rule_id,ordering_rule_id,assessed_at,reason) VALUES(?1,'supersession',?2,?3,3,'synthetic')",params![d,f.rules["reconciliation"],f.rules["ordering"]]).unwrap();
    let reverse = row(
        "supersessions",
        [
            ("older_claim_id", edge.get("replacement_claim_id").clone()),
            ("replacement_claim_id", edge.get("older_claim_id").clone()),
            ("dimension", edge.get("dimension").clone()),
            ("decision_id", d.into()),
        ],
    );
    assert!(
        raw_insert(&c, &reverse)
            .unwrap_err()
            .to_string()
            .contains("cycle")
    );
    c.execute_batch("ROLLBACK").unwrap();
}
#[test]
fn coverage_scope_null_shapes_and_parent_guards() {
    let f = Fixture::new();
    let b = f.report(
        "fixture:steam",
        &[("root-A", "complete")],
        "complete",
        1,
        Some(1),
    );
    let c = connection(&f);
    let result = b
        .rows
        .iter()
        .find(|r| r.table == "coverage_results")
        .unwrap();
    raw_insert(&c, result).unwrap();
    let part = b.rows.iter().find(|r| r.table == "coverage_parts").unwrap();
    let mut invalid = part.clone();
    invalid.set("parent_part_id", id());
    invalid.set("selector_kind", "bounded");
    invalid.set("selector", vec![0]);
    invalid.set("selector_rule_id", f.rules["interpretation"].clone());
    assert!(
        raw_insert(&c, &invalid)
            .unwrap_err()
            .to_string()
            .contains("coverage parent")
    );
    let mut bad = part.clone();
    bad.set("inspection", "uninspected");
    assert!(raw_insert(&c, &bad).is_err());
    raw_insert(&c, part).unwrap();
    let mut duplicate = part.clone();
    duplicate.set("id", id());
    duplicate.set("part_key", "second-root");
    assert!(raw_insert(&c, &duplicate).is_err());
}
#[test]
fn negative_without_required_basis_and_wrong_account_cannot_commit() {
    let mut f = Fixture::new();
    let (e, i, _, _) = f.observe("fixture:steam", "app", Some("present"), None, None, None, 1);
    let mut b = f.report(
        "fixture:steam",
        &[("root-A", "complete")],
        "complete",
        2,
        Some(2),
    );
    let cid = f.claim(
        &mut b,
        i.as_ref().unwrap(),
        "presence",
        Some("absent"),
        "root-A",
        true,
    );
    b.rows
        .iter_mut()
        .find(|r| r.is("id", &cid))
        .unwrap()
        .set("negative_justification_id", Value::Null);
    b.rows.push(row(
        "collection_closures",
        [
            ("collection_id", b.id.clone().into()),
            ("outcome", "complete".into()),
            ("finished_at", 2i64.into()),
            ("result_protocol", "koine.result.v1".into()),
        ],
    ));
    assert!(f.store.write(b.rows).is_err());
    let mut b = f.report(
        "fixture:steam",
        &[("root-A", "complete")],
        "complete",
        3,
        Some(3),
    );
    let a = f.entity(&mut b, &e, "entitlements", "B");
    f.claim_account(
        &mut b,
        &a,
        "access",
        Some("accessible"),
        "root-A",
        false,
        "account-A",
    );
    b.rows.push(row(
        "collection_closures",
        [
            ("collection_id", b.id.clone().into()),
            ("outcome", "complete".into()),
            ("finished_at", 3i64.into()),
            ("result_protocol", "koine.result.v1".into()),
        ],
    ));
    assert!(f.store.write(b.rows).is_err());
}
#[test]
fn bounded_absence_cannot_cross_installation_roots() {
    let mut f = Fixture::new();
    let mut b = f.report(
        "fixture:steam",
        &[("root-A", "complete"), ("root-B", "complete")],
        "complete",
        1,
        Some(1),
    );
    let e = f.entry(&mut b, "app", "Meridian");
    let i = f.entity(&mut b, &e, "installations", "root-B");
    f.claim(&mut b, &i, "presence", Some("present"), "root-B", false);
    f.finish(b);
    let mut b = f.report(
        "fixture:steam",
        &[("root-A", "complete")],
        "complete",
        2,
        Some(2),
    );
    f.claim(&mut b, &i, "presence", Some("absent"), "root-A", true);
    b.rows.push(row(
        "collection_closures",
        [
            ("collection_id", b.id.clone().into()),
            ("outcome", "complete".into()),
            ("finished_at", 2i64.into()),
            ("result_protocol", "koine.result.v1".into()),
        ],
    ));
    assert!(
        f.store
            .write(b.rows)
            .unwrap_err()
            .contains("outside inspected")
    );
}
#[test]
fn canonical_tampering_withholds_conclusions_without_resealing() {
    let mut f = Fixture::new();
    let (_, i, _, _) = f.observe("fixture:steam", "app", Some("present"), None, None, None, 1);
    let mut snapshot = f.store.snapshot().unwrap();
    snapshot
        .records
        .iter_mut()
        .find(|r| r.table == "claims")
        .unwrap()
        .set("basis_code", "tampered");
    let view = reconcile::reconstruct(&snapshot, &f.assessment(3, true));
    assert!(!view.diagnostics.is_empty());
    assert!(
        view.claims
            .iter()
            .all(|c| c.knowledge == Knowledge::Unknown)
    );
    assert!(
        snapshot
            .rows("installations")
            .any(|r| r.is("id", i.as_ref().unwrap()))
    );
}
#[test]
fn reconstruction_is_insertion_order_independent_and_policy_explicit() {
    let mut f = Fixture::new();
    f.observe(
        "fixture:steam",
        "app",
        Some("present"),
        Some("complete"),
        None,
        None,
        1,
    );
    let mut s = f.store.snapshot().unwrap();
    let a = f.assessment(3, true);
    let before = reconcile::reconstruct(&s, &a);
    s.records.reverse();
    assert_eq!(before, reconcile::reconstruct(&s, &a));
    let mut unsupported = a;
    unsupported.freshness_rule = id();
    assert!(
        !reconcile::reconstruct(&s, &unsupported)
            .diagnostics
            .is_empty()
    );
}
#[test]
fn incomplete_registration_remains_inconclusive_after_restart() {
    let f = Fixture::new();
    let b = f.report(
        "fixture:steam",
        &[("root-A", "complete")],
        "complete",
        1,
        Some(1),
    );
    let a = f.assessment(3, true);
    let path = f.path.clone();
    drop(f.store);
    let store = Store::open(path).unwrap();
    let v = store.reconstruct(&a).unwrap();
    assert_eq!(
        v.collections
            .iter()
            .find(|c| c.collection == b.id)
            .unwrap()
            .outcome,
        "inconclusive"
    );
    assert!(v.claims.is_empty());
}
#[test]
fn ownership_refuses_second_owner_and_symlink_alias() {
    let f = Fixture::new();
    assert!(Store::open(&f.path).err().unwrap().contains("owns"));
    #[cfg(unix)]
    {
        let alias = f.dir.path().join("alias.sqlite");
        std::os::unix::fs::symlink(&f.path, &alias).unwrap();
        assert!(Store::open(alias).is_err());
    }
}
#[test]
fn unsupported_foreign_missing_and_altered_catalog_files_refused() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing.sqlite");
    assert!(Store::open(&missing).is_err());
    assert!(!missing.exists());
    let foreign = dir.path().join("foreign.sqlite");
    let c = Connection::open(&foreign).unwrap();
    c.execute_batch(
        "CREATE TABLE external_data(value TEXT); INSERT INTO external_data VALUES('preserve');",
    )
    .unwrap();
    drop(c);
    let original = std::fs::read(&foreign).unwrap();
    assert!(Store::open(&foreign).is_err());
    assert_eq!(original, std::fs::read(&foreign).unwrap());
    let f = Fixture::new();
    let path = f.path.clone();
    drop(f.store);
    let c = Connection::open(&path).unwrap();
    c.execute_batch("DROP TRIGGER claims_immutable_delete")
        .unwrap();
    drop(c);
    assert!(Store::open(path).err().unwrap().contains("catalog"));
}
#[test]
fn no_implicit_upgrade_or_existing_database_recreation() {
    let f = Fixture::new();
    assert!(Store::create(&f.path).is_err());
    let path = f.path.clone();
    drop(f.store);
    let c = Connection::open(&path).unwrap();
    c.pragma_update(None, "user_version", 999).unwrap();
    drop(c);
    assert!(Store::open(&path).is_err());
    let c = Connection::open(path).unwrap();
    assert_eq!(
        c.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        999
    );
}
#[test]
fn backup_requires_completion_validation_and_does_not_prune() {
    let mut f = Fixture::new();
    f.observe("fixture:steam", "app", Some("present"), None, None, None, 1);
    let before = f.store.snapshot().unwrap();
    let dest = f.dir.path().join("archive.sqlite");
    f.store
        .backup(&dest, Arc::new(AtomicBool::new(false)))
        .unwrap();
    assert_eq!(before, f.store.snapshot().unwrap());
    let archive = Store::open(&dest).unwrap();
    assert_eq!(before, archive.snapshot().unwrap());
    assert!(
        f.store
            .backup(&dest, Arc::new(AtomicBool::new(false)))
            .is_err()
    );
    let failed = f.dir.path().join("cancelled.sqlite");
    let flag = Arc::new(AtomicBool::new(true));
    assert!(
        f.store
            .backup(&failed, flag.clone())
            .unwrap_err()
            .contains("cancelled")
    );
    assert!(!failed.exists());
    flag.store(false, Ordering::Relaxed);
}
#[test]
fn serialized_writes_are_idempotent_under_concurrent_delivery() {
    let mut f = Fixture::new();
    let (_, _, _, t) = f.observe(
        "fixture:steam",
        "app",
        None,
        None,
        None,
        Some("supported"),
        1,
    );
    let r = f.intent(t.as_ref().unwrap());
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|_| scope.spawn(|| f.store.write(vec![r.clone()])))
            .collect();
        for h in handles {
            h.join().unwrap().unwrap();
        }
    });
    assert_eq!(
        f.store.snapshot().unwrap().rows("launch_attempts").count(),
        1
    );
}
#[test]
fn provisional_scopes_are_isolated_and_missing_contexts_rejected() {
    let f = Fixture::new();
    let mut rows = Vec::new();
    for _ in 0..2 {
        rows.push(row(
            "identity_scopes",
            [
                ("id", id().into()),
                ("namespace", "fixture:account-scoped".into()),
                ("scope_kind", "account".into()),
                ("resolution", "provisional".into()),
                ("provisional_token", id().into()),
            ],
        ));
    }
    f.store.write(rows).unwrap();
    assert_eq!(
        f.store
            .snapshot()
            .unwrap()
            .rows("identity_scopes")
            .filter(|r| r.is("resolution", "provisional"))
            .count(),
        2
    );
    let mut invalid = row(
        "identity_scopes",
        [
            ("id", id().into()),
            ("namespace", "fixture:account-scoped".into()),
            ("scope_kind", "account".into()),
            ("resolution", "resolved".into()),
        ],
    );
    assert!(f.store.write(vec![invalid.clone()]).is_err());
    invalid.set("subject_context_id", f.contexts["manager-S"].clone());
    assert!(f.store.write(vec![invalid]).is_err());
}
#[test]
fn invalid_uuid_digest_types_and_negative_null_bypasses_rejected() {
    let c = Connection::open_in_memory().unwrap();
    c.execute_batch(BOOTSTRAP).unwrap();
    for id in [
        "00000000-0000-4000-8000-00000000000G",
        "00000000000040008000000000000000----",
        "00000000-0000-4000-8000-000000000000X",
    ] {
        assert!(c.execute("INSERT INTO contexts(id,kind,namespace,resolution,external_key) VALUES(?1,'subject','fixture','identified','A')",[id]).is_err());
    }
    assert!(c.execute("INSERT INTO contexts(id,kind,namespace,resolution,external_key) VALUES(?1,'subject','fixture','provisional','A')",[id()]).is_err());
    assert_ne!(
        canonical::frame("koine.authority-scope.v1", &Value::List(vec![Value::Null])),
        Vec::<u8>::new()
    );
}

#[test]
fn profile_upgrade_keeps_key_identity_and_original_observations() {
    let mut f = Fixture::new();
    let (e, _, _, _) = f.observe("fixture:steam", "app", None, None, None, None, 1);
    let old = f.profiles["fixture:steam"].clone();
    let pid = id();
    f.store
        .write(vec![row(
            "identity_profiles",
            [
                ("id", pid.clone().into()),
                ("namespace", "fixture:steam".into()),
                ("version", "2".into()),
                ("scope_kind", "global".into()),
                ("key_role", "catalog".into()),
                ("catalog_namespace", "fixture:steam".into()),
                ("rule_id", f.rules["identity"].clone().into()),
            ],
        )])
        .unwrap();
    f.profiles.insert("fixture:steam".into(), pid.clone());
    let (next, _, _, _) = f.observe("fixture:steam", "app", None, None, None, None, 2);
    assert_eq!(e, next);
    let s = f.store.snapshot().unwrap();
    assert_eq!(s.rows("entry_keys").count(), 1);
    let observations: Vec<_> = s.rows("entry_key_observations").collect();
    assert_eq!(observations.len(), 2);
    assert!(observations.iter().any(|o| o.is("profile_id", &old)));
    assert!(observations.iter().any(|o| o.is("profile_id", &pid)));
}
#[test]
fn equivalence_conflict_applies_to_entire_connected_component() {
    let mut f = Fixture::new();
    let (a, _, _, _) = f.observe("fixture:steam", "a", None, None, None, None, 1);
    let (b, _, _, _) = f.observe("fixture:steam", "b", None, None, None, None, 1);
    let (c, _, _, _) = f.observe("fixture:steam", "c", None, None, None, None, 1);
    let mut report = f.report(
        "koine:manual",
        &[("root-A", "complete")],
        "complete",
        2,
        None,
    );
    for (left, right, value) in [
        (&a, &b, "equivalent"),
        (&b, &c, "equivalent"),
        (&a, &c, "different"),
    ] {
        let pair = id();
        let subject = id();
        let evidence = f.evidence(&mut report);
        let (left, right) = if left < right {
            (left, right)
        } else {
            (right, left)
        };
        report.rows.push(row(
            "entry_equivalences",
            [
                ("id", pair.clone().into()),
                ("left_entry_id", left.clone().into()),
                ("right_entry_id", right.clone().into()),
            ],
        ));
        report.rows.push(row(
            "claim_subjects",
            [
                ("id", subject.clone().into()),
                ("dimension", "entry_equivalence".into()),
                (
                    "authority_scope_id",
                    f.authorities["koine:manual/account-A"].clone().into(),
                ),
                ("equivalence_id", pair.into()),
            ],
        ));
        report.rows.push(row(
            "claims",
            [
                ("id", id().into()),
                ("subject_id", subject.into()),
                ("dimension", "entry_equivalence".into()),
                (
                    "authority_scope_id",
                    f.authorities["koine:manual/account-A"].clone().into(),
                ),
                ("evidence_id", evidence.into()),
                ("collection_id", report.id.clone().into()),
                ("knowledge", "observed".into()),
                ("value_text", value.into()),
                ("basis_code", "synthetic explicit identity proof".into()),
            ],
        ));
    }
    f.finish(report);
    let view = f.view(3);
    assert_eq!(view.identity_components.len(), 1);
    assert_eq!(view.identity_components[0].handles.len(), 3);
    assert!(view.identity_components[0].disputed);
    assert_eq!(
        f.store.snapshot().unwrap().rows("source_entries").count(),
        3
    );
}
#[test]
fn diagnostic_gap_cannot_hide_behind_a_complete_unit() {
    let mut f = Fixture::new();
    let (_, i, _, _) = f.observe("fixture:steam", "app", Some("present"), None, None, None, 1);
    let mut b = f.report(
        "fixture:steam",
        &[("root-A", "complete")],
        "complete",
        2,
        Some(2),
    );
    f.claim(
        &mut b,
        i.as_ref().unwrap(),
        "presence",
        Some("present"),
        "root-A",
        false,
    );
    let unit = b
        .rows
        .iter()
        .find(|r| r.table == "coverage_parts" && r.is("dimension", "presence"))
        .unwrap()
        .get("unit_id")
        .clone();
    b.rows.push(row(
        "diagnostics",
        [
            ("id", id().into()),
            ("collection_id", b.id.clone().into()),
            ("item_key", id().into()),
            ("target_kind", "unit".into()),
            ("unit_id", unit),
            ("severity", "warning".into()),
            ("code", "parse-gap".into()),
            ("message", "synthetic unsupported record".into()),
            ("recorded_at", 2i64.into()),
        ],
    ));
    b.rows.push(row(
        "collection_closures",
        [
            ("collection_id", b.id.clone().into()),
            ("outcome", "complete".into()),
            ("finished_at", 2i64.into()),
            ("result_protocol", "koine.result.v1".into()),
        ],
    ));
    assert!(
        f.store
            .write(b.rows)
            .unwrap_err()
            .contains("diagnostic gap")
    );
    assert_eq!(
        f.conclusion(i.as_ref().unwrap(), "presence", 3).knowledge,
        Knowledge::Observed("present".into())
    );
}
#[test]
fn passive_checkpoint_respects_reader_snapshot_and_later_completes() {
    let mut f = Fixture::new();
    f.observe("fixture:steam", "app", None, None, None, None, 1);
    f.store.checkpoint().unwrap();
    let reader = connection(&f);
    reader
        .execute_batch("BEGIN; SELECT count(*) FROM claims;")
        .unwrap();
    f.observe("fixture:steam", "app", None, None, None, None, 2);
    let partial = f.store.checkpoint().unwrap();
    assert!(partial.wal_pages > partial.checkpointed_pages);
    reader.execute_batch("ROLLBACK").unwrap();
    let complete = f.store.checkpoint().unwrap();
    assert_eq!(complete.wal_pages, complete.checkpointed_pages);
    assert_eq!(complete.busy, 0);
}
#[test]
fn writer_waits_for_lock_without_blocking_reader_or_replaying_body() {
    let mut f = Fixture::new();
    let (_, _, _, target) = f.observe(
        "fixture:steam",
        "app",
        None,
        None,
        None,
        Some("supported"),
        1,
    );
    let intent = f.intent(target.as_ref().unwrap());
    let external = connection(&f);
    external.execute_batch("BEGIN IMMEDIATE").unwrap();
    std::thread::scope(|scope| {
        let write = scope.spawn(|| f.store.write(vec![intent.clone()]));
        assert_eq!(
            f.store.snapshot().unwrap().rows("launch_attempts").count(),
            0
        );
        external.execute_batch("ROLLBACK").unwrap();
        write.join().unwrap().unwrap();
    });
    assert_eq!(
        f.store.snapshot().unwrap().rows("launch_attempts").count(),
        1
    );
}

#[test]
fn manual_evidence_cannot_launder_external_access_authority() {
    let mut f = Fixture::new();
    let (_, _, access, _) = f.observe(
        "fixture:steam",
        "app",
        None,
        None,
        Some("accessible"),
        None,
        1,
    );
    let snapshot = f.store.snapshot().unwrap();
    let existing = snapshot
        .rows("claim_subjects")
        .find(|s| s.is("entitlement_id", access.as_ref().unwrap()))
        .unwrap();
    let mut report = f.report(
        "koine:manual",
        &[("root-A", "complete")],
        "complete",
        2,
        None,
    );
    let evidence = f.evidence(&mut report);
    report.rows.push(row(
        "claims",
        [
            ("id", id().into()),
            ("subject_id", existing.get("id").clone()),
            ("dimension", "access".into()),
            (
                "authority_scope_id",
                existing.get("authority_scope_id").clone(),
            ),
            ("evidence_id", evidence.into()),
            ("collection_id", report.id.clone().into()),
            ("knowledge", "observed".into()),
            ("value_text", "accessible".into()),
            ("basis_code", "manual user assertion".into()),
        ],
    ));
    report.rows.push(row(
        "collection_closures",
        [
            ("collection_id", report.id.clone().into()),
            ("outcome", "complete".into()),
            ("finished_at", 2i64.into()),
            ("result_protocol", "koine.result.v1".into()),
        ],
    ));
    assert!(
        f.store
            .write(report.rows)
            .unwrap_err()
            .contains("cannot overwrite external authority")
    );
}
#[test]
fn concurrent_duplicate_handoff_reserves_intent_once() {
    use std::sync::atomic::AtomicUsize;
    struct Counter(Arc<AtomicUsize>);
    impl Transport for Counter {
        fn submit(&mut self, _target: &str, _launcher: &str) -> Result<TransportResult> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(TransportResult {
                issuance: "issued".into(),
                disposition: "accepted".into(),
                status: None,
                boundary: "fixture transport".into(),
                reason: None,
            })
        }
    }
    let mut f = Fixture::new();
    let (_, _, _, target) = f.observe(
        "fixture:steam",
        "app",
        None,
        None,
        None,
        Some("supported"),
        1,
    );
    let intent = f.intent(target.as_ref().unwrap());
    let calls = Arc::new(AtomicUsize::new(0));
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..2)
            .map(|_| {
                let intent = intent.clone();
                let result = f.handoff_record(intent.text("id").unwrap());
                let calls = calls.clone();
                let store = &f.store;
                scope.spawn(move || store.handoff(intent, result, &mut Counter(calls)))
            })
            .collect();
        let successful = handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .filter(|result| result.is_ok())
            .count();
        assert_eq!(successful, 1);
    });
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        f.store.snapshot().unwrap().rows("launch_attempts").count(),
        1
    );
}

#[test]
fn exhausted_contention_budget_rolls_back_and_keeps_writer_usable() {
    let mut f = Fixture::new();
    let (_, _, _, target) = f.observe(
        "fixture:steam",
        "app",
        None,
        None,
        None,
        Some("supported"),
        1,
    );
    let intent = f.intent(target.as_ref().unwrap());
    let external = connection(&f);
    external.execute_batch("BEGIN IMMEDIATE").unwrap();
    let start = std::time::Instant::now();
    assert!(f.store.write(vec![intent.clone()]).is_err());
    assert!(start.elapsed() >= std::time::Duration::from_secs(4));
    external.execute_batch("ROLLBACK").unwrap();
    assert_eq!(
        f.store.snapshot().unwrap().rows("launch_attempts").count(),
        0
    );
    f.store.write(vec![intent]).unwrap();
    assert_eq!(
        f.store.snapshot().unwrap().rows("launch_attempts").count(),
        1
    );
}

#[test]
fn non_utf8_paths_round_trip_as_bytes_and_nul_is_rejected() {
    let mut f = Fixture::new();
    let (_, i, _, _) = f.observe("fixture:steam", "app", Some("present"), None, None, None, 1);
    let installation = i.unwrap();
    let mut b = f.report(
        "fixture:steam",
        &[("root-A", "complete")],
        "complete",
        2,
        Some(2),
    );
    let cid = f.claim(&mut b, &installation, "location", None, "root-A", false);
    let claim = b.rows.iter_mut().find(|r| r.is("id", &cid)).unwrap();
    claim.set("knowledge", "observed");
    claim.set("reason", Value::Null);
    claim.set("value_bytes", b"/synthetic/\xff\x80".to_vec());
    f.finish(b);
    let s = f.store.snapshot().unwrap();
    assert_eq!(
        s.by_id("claims", &cid.clone().into())
            .unwrap()
            .get("value_bytes"),
        &Value::Blob(b"/synthetic/\xff\x80".to_vec())
    );
    let mut b = f.report(
        "fixture:steam",
        &[("root-A", "complete")],
        "complete",
        3,
        Some(3),
    );
    let cid = f.claim(&mut b, &installation, "location", None, "root-A", false);
    let claim = b.rows.iter_mut().find(|r| r.is("id", &cid)).unwrap();
    claim.set("knowledge", "observed");
    claim.set("reason", Value::Null);
    claim.set("value_bytes", b"/invalid/\0path".to_vec());
    b.rows.push(row(
        "collection_closures",
        [
            ("collection_id", b.id.clone().into()),
            ("outcome", "complete".into()),
            ("finished_at", 3i64.into()),
            ("result_protocol", "koine.result.v1".into()),
        ],
    ));
    assert!(f.store.write(b.rows).unwrap_err().contains("NUL"));
}

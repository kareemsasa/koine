use koine_core::*;
use koine_sqlite::Store;
use koine_steam::{integration, *};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicBool},
};
fn fixtures() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(".koine-synthetic-root"),
        format!("koine-fixture-v1\n{}\n", id()),
    )
    .unwrap();
    fs::create_dir(dir.path().join("steamapps")).unwrap();
    index(dir.path(), &[dir.path().to_path_buf()]);
    dir
}
fn index(root: &Path, paths: &[PathBuf]) {
    let mut text = String::from("\"libraryfolders\"\n{\n");
    for (i, path) in paths.iter().enumerate() {
        text += &format!(
            "\"{i}\" {{ \"path\" \"{}\" \"apps\" {{ \"999999\" \"0\" }} }}\n",
            path.to_str()
                .unwrap()
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
        );
    }
    text += "}\n";
    fs::write(root.join("steamapps/libraryfolders.vdf"), text).unwrap();
}
fn write_manifest(root: &Path, app: u32, flags: &str) {
    fs::create_dir_all(root.join("steamapps")).unwrap();
    fs::write(root.join(format!("steamapps/appmanifest_{app}.acf")),format!("\"AppState\" {{ \"appid\" \"{app}\" \"name\" \"Fixture {app}\" \"installdir\" \"Fixture {app}\" \"StateFlags\" \"{flags}\" }}")).unwrap();
}
fn run(store: &Store, dir: &Path, delivery: String, at: i64) -> Report {
    let fixture = FixtureFs::open(dir).unwrap();
    integration::discover(
        store,
        &fixture,
        delivery,
        at,
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap()
}
fn store() -> (Store, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    (Store::create(dir.path().join("db.sqlite")).unwrap(), dir)
}
#[test]
fn manifest_fields_and_conservative_completion() {
    let complete = parse_manifest(
        "appmanifest_900001.acf",
        include_bytes!("fixtures/complete.acf"),
        b"steamapps/appmanifest_900001.acf".to_vec(),
    )
    .unwrap();
    assert_eq!(complete.completion.as_deref(), Some("complete"));
    let incomplete = parse_manifest(
        "appmanifest_900002.acf",
        include_bytes!("fixtures/incomplete.acf"),
        vec![],
    )
    .unwrap();
    assert_eq!(incomplete.completion.as_deref(), Some("incomplete"));
    for flags in [
        None,
        Some(0),
        Some(2),
        Some(6),
        Some(4 | 1048576),
        Some(1 << 31),
    ] {
        assert_eq!(completion(flags), None)
    }
    assert_ne!(completion(Some(64)), Some("complete"));
}
#[test]
fn strict_keyvalues_rejects_ambiguous_and_unsupported_inputs() {
    for bytes in [
        b"\"AppState\" {\"appid\" \"1\" \"AppID\" \"2\"}".as_slice(),
        b"#include \"outside\"",
        b"\"root\" {",
        b"\"root\" \"bad\\q\"",
        b"\xff",
    ] {
        assert!(keyvalues::parse(bytes).is_err())
    }
    let parsed = keyvalues::parse(b"// note\n \"r\" { \"name\" \"Quoted \\\" text\" }");
    assert!(parsed.is_ok());
}
#[test]
fn malformed_ids_and_unsafe_install_dirs_are_rejected() {
    for (id, dir) in [
        ("0", "ok"),
        ("4294967296", "ok"),
        ("42", "../escape"),
        ("42", "/absolute"),
    ] {
        let bytes = format!("\"AppState\" {{\"appid\" \"{id}\" \"installdir\" \"{dir}\"}}");
        assert!(parse_manifest(&format!("appmanifest_{id}.acf"), bytes.as_bytes(), vec![]).is_err())
    }
    assert!(
        parse_manifest(
            "appmanifest_7.acf",
            b"\"AppState\" {\"appid\" \"8\" \"installdir\" \"ok\"}",
            vec![]
        )
        .is_err()
    );
}
#[test]
fn multiple_libraries_persist_distinct_instances_and_restart_identically() {
    let dir = fixtures();
    let extra = dir.path().join("library B");
    write_manifest(dir.path(), 42, "4");
    write_manifest(&extra, 42, "1048576");
    index(dir.path(), &[dir.path().into(), extra]);
    let (store, db) = store();
    let report = run(&store, dir.path(), id(), 10);
    assert_eq!(report.roots.len(), 2);
    let snapshot = store.snapshot().unwrap();
    assert_eq!(snapshot.rows("source_entries").count(), 1);
    assert_eq!(snapshot.rows("installations").count(), 2);
    assert_eq!(snapshot.rows("entitlements").count(), 0);
    assert_eq!(snapshot.rows("launch_targets").count(), 0);
    let assessment = integration::assessment(&store, 20).unwrap();
    let before = store.reconstruct(&assessment).unwrap();
    assert!(
        before
            .claims
            .iter()
            .any(|c| c.knowledge == Knowledge::Observed("complete".into()))
    );
    assert!(
        before
            .claims
            .iter()
            .any(|c| c.knowledge == Knowledge::Observed("incomplete".into()))
    );
    assert!(
        before
            .claims
            .iter()
            .all(|c| c.freshness == Freshness::Unknown)
    );
    drop(store);
    let reopened = Store::open(db.path().join("db.sqlite")).unwrap();
    assert_eq!(snapshot, reopened.snapshot().unwrap());
    assert_eq!(before, reopened.reconstruct(&assessment).unwrap());
}
#[test]
fn duplicate_delivery_reuses_all_ids_and_changed_payload_is_rejected() {
    let dir = fixtures();
    write_manifest(dir.path(), 42, "4");
    let (store, _) = store();
    let report = run(&store, dir.path(), id(), 10);
    let before = store.snapshot().unwrap();
    integration::register(&store, &report.plan).unwrap();
    integration::apply(&store, &report).unwrap();
    assert_eq!(before, store.snapshot().unwrap());
    let mut changed = report;
    changed
        .roots
        .iter_mut()
        .find(|r| !r.observations.is_empty())
        .unwrap()
        .observations[0]
        .name = Some("conflicting delivery".into());
    assert!(integration::apply(&store, &changed).is_err());
    assert_eq!(before, store.snapshot().unwrap());
}
#[test]
fn partial_corrupt_and_unknown_records_preserve_valid_observations() {
    let dir = fixtures();
    write_manifest(dir.path(), 42, "4");
    write_manifest(dir.path(), 43, "2147483648");
    fs::write(dir.path().join("steamapps/appmanifest_44.acf"), b"corrupt").unwrap();
    let (store, _) = store();
    run(&store, dir.path(), id(), 10);
    let view = store
        .reconstruct(&integration::assessment(&store, 20).unwrap())
        .unwrap();
    assert_eq!(store.snapshot().unwrap().rows("installations").count(), 2);
    assert!(view.collections.iter().any(|c| c.outcome == "partial"));
    assert!(
        view.claims
            .iter()
            .any(|c| c.dimension == "completion" && c.knowledge == Knowledge::Unknown)
    );
    assert!(
        view.claims
            .iter()
            .all(|c| !matches!(c.knowledge, Knowledge::ConfirmedNegative(_)))
    );
}
#[test]
fn missing_root_is_inconclusive_and_failed_refresh_does_not_remove_last_known() {
    let dir = fixtures();
    write_manifest(dir.path(), 42, "4");
    let (store, _) = store();
    run(&store, dir.path(), id(), 10);
    let old = store.snapshot().unwrap();
    index(
        dir.path(),
        &[dir.path().into(), dir.path().join("missing volume")],
    );
    fs::remove_file(dir.path().join("steamapps/appmanifest_42.acf")).unwrap();
    let report = run(&store, dir.path(), id(), 20);
    assert!(report.roots.iter().any(|r| !r.inspected));
    let s = store.snapshot().unwrap();
    assert_eq!(s.rows("installations").count(), 1);
    for claim in old.rows("claims") {
        assert!(s.by_id("claims", claim.get("id")).is_ok())
    }
    assert!(s.rows("negative_justifications").next().is_none());
}
#[test]
fn no_marker_no_follow_no_outside_paths() {
    let dir = tempfile::tempdir().unwrap();
    assert!(FixtureFs::open(dir.path()).is_err());
    let dir = fixtures();
    let outside = tempfile::tempdir().unwrap();
    index(dir.path(), &[dir.path().into(), outside.path().into()]);
    let fixture = FixtureFs::open(dir.path()).unwrap();
    let plan = fixture.plan(id(), 10).unwrap();
    assert!(plan.roots.iter().any(|r| r.relative.is_none()));
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(outside.path(), dir.path().join("linked")).unwrap();
        index(dir.path(), &[dir.path().into(), dir.path().join("linked")]);
        let plan = fixture.plan(id(), 11).unwrap();
        let report = fixture
            .scan(plan, Arc::new(AtomicBool::new(false)))
            .unwrap();
        assert!(report.roots.iter().any(|r| !r.inspected));
    }
}
#[test]
fn changed_and_delayed_observations_remain_disputed_without_source_order() {
    let dir = fixtures();
    write_manifest(dir.path(), 42, "4");
    let (store, _) = store();
    run(&store, dir.path(), id(), 20);
    write_manifest(dir.path(), 42, "1048576");
    run(&store, dir.path(), id(), 10);
    let view = store
        .reconstruct(&integration::assessment(&store, 30).unwrap())
        .unwrap();
    assert!(
        view.claims
            .iter()
            .any(|c| c.dimension == "completion" && c.knowledge == Knowledge::Disputed)
    );
    assert!(
        store
            .snapshot()
            .unwrap()
            .rows("supersessions")
            .next()
            .is_none()
    );
    assert!(
        store
            .snapshot()
            .unwrap()
            .rows("reconciliation_decisions")
            .any(|d| d.is("kind", "dispute"))
    );
}
#[test]
fn unreadable_fixture_record_is_diagnosed_without_erasing_prior_claims() {
    use std::os::unix::fs::PermissionsExt;
    let dir = fixtures();
    write_manifest(dir.path(), 42, "4");
    let (store, _) = store();
    run(&store, dir.path(), id(), 10);
    let file = dir.path().join("steamapps/appmanifest_42.acf");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o0)).unwrap();
    let report = run(&store, dir.path(), id(), 20);
    assert!(
        report
            .roots
            .iter()
            .any(|r| r.diagnostics.iter().any(|d| d.code == "unreadable-path"))
    );
    assert_eq!(store.snapshot().unwrap().rows("installations").count(), 1);
    fs::set_permissions(file, fs::Permissions::from_mode(0o600)).unwrap();
}

#[test]
fn legacy_library_entries_are_gaps_while_default_root_remains_usable() {
    let dir = fixtures();
    write_manifest(dir.path(), 42, "4");
    fs::write(
        dir.path().join("steamapps/libraryfolders.vdf"),
        b"\"libraryfolders\" { \"1\" \"legacy-path\" }",
    )
    .unwrap();
    let (store, _) = store();
    let report = run(&store, dir.path(), id(), 10);
    assert!(!report.index_issues.is_empty());
    assert!(
        report
            .roots
            .iter()
            .any(|r| r.root.relative.is_none() && !r.inspected)
    );
    assert_eq!(store.snapshot().unwrap().rows("installations").count(), 1);
    assert!(
        store
            .reconstruct(&integration::assessment(&store, 20).unwrap())
            .unwrap()
            .collections
            .iter()
            .all(|c| c.outcome == "partial")
    );
}

#[test]
fn changed_index_does_not_silently_expand_durable_requested_scope() {
    let dir = fixtures();
    write_manifest(dir.path(), 42, "4");
    let fixture = FixtureFs::open(dir.path()).unwrap();
    let (store, _) = store();
    let plan = fixture.plan(id(), 10).unwrap();
    integration::register(&store, &plan).unwrap();
    let extra = dir.path().join("later");
    write_manifest(&extra, 43, "4");
    index(dir.path(), &[dir.path().into(), extra]);
    let report = fixture
        .scan(plan, Arc::new(AtomicBool::new(false)))
        .unwrap();
    assert_eq!(report.roots.len(), 1);
    assert!(
        report
            .index_issues
            .iter()
            .any(|s| s == "library-index-changed")
    );
    integration::apply(&store, &report).unwrap();
    assert_eq!(store.snapshot().unwrap().rows("installations").count(), 1);
}

#[test]
fn cancelled_scan_closes_as_inconclusive_and_keeps_requested_units() {
    let dir = fixtures();
    write_manifest(dir.path(), 42, "4");
    let fixture = FixtureFs::open(dir.path()).unwrap();
    let (store, _) = store();
    let plan = fixture.plan(id(), 10).unwrap();
    integration::register(&store, &plan).unwrap();
    let before = store.snapshot().unwrap();
    let report = fixture.scan(plan, Arc::new(AtomicBool::new(true))).unwrap();
    integration::apply(&store, &report).unwrap();
    let after = store.snapshot().unwrap();
    for u in before.rows("requested_units") {
        assert_eq!(u, after.by_id("requested_units", u.get("id")).unwrap());
    }
    assert_eq!(after.rows("installations").count(), 0);
    assert!(
        store
            .reconstruct(&integration::assessment(&store, 20).unwrap())
            .unwrap()
            .collections
            .iter()
            .all(|c| c.outcome == "inconclusive")
    );
}

#[test]
fn decimal_aliases_share_scoped_key_and_conflicting_names_remain_disputed() {
    let dir = fixtures();
    write_manifest(dir.path(), 42, "4");
    fs::write(dir.path().join("steamapps/appmanifest_00042.acf"), b"\"AppState\" { \"appid\" \"00042\" \"name\" \"Alternative title\" \"installdir\" \"Fixture 42\" \"StateFlags\" \"4\" }").unwrap();
    let (store, _) = store();
    run(&store, dir.path(), id(), 10);
    let s = store.snapshot().unwrap();
    assert_eq!(s.rows("source_entries").count(), 1);
    assert_eq!(s.rows("entry_keys").count(), 1);
    assert_eq!(s.rows("installations").count(), 1);
    assert!(
        s.rows("entry_key_observations")
            .any(|o| o.is("reported_key", "00042"))
    );
    assert!(
        store
            .reconstruct(&integration::assessment(&store, 20).unwrap())
            .unwrap()
            .claims
            .iter()
            .any(|c| c.dimension == "entry_title" && c.knowledge == Knowledge::Disputed)
    );
}

#[test]
fn uninstalled_or_ambiguous_manifest_never_revokes_known_presence() {
    let dir = fixtures();
    let (store, _) = store();
    write_manifest(dir.path(), 42, "4");
    run(&store, dir.path(), id(), 10);
    for (index, flag) in ["0", "1", "2048", "invalid", "6"].iter().enumerate() {
        write_manifest(dir.path(), 42, flag);
        run(&store, dir.path(), id(), 20 + index as i64);
    }
    let view = store
        .reconstruct(&integration::assessment(&store, 30).unwrap())
        .unwrap();
    assert!(
        view.claims
            .iter()
            .any(|c| c.dimension == "presence"
                && c.knowledge == Knowledge::Observed("present".into()))
    );
    assert!(
        view.claims
            .iter()
            .all(|c| !matches!(c.knowledge, Knowledge::ConfirmedNegative(_)))
    );
    assert_eq!(store.snapshot().unwrap().rows("supersessions").count(), 0);
}

#[test]
fn report_validation_and_registered_time_refuse_changed_interpretation() {
    let dir = fixtures();
    write_manifest(dir.path(), 42, "1048576");
    let fixture = FixtureFs::open(dir.path()).unwrap();
    let (store, _) = store();
    let plan = fixture.plan(id(), 10).unwrap();
    integration::register(&store, &plan).unwrap();
    let before = store.snapshot().unwrap();
    let report = fixture
        .scan(plan, Arc::new(AtomicBool::new(false)))
        .unwrap();
    let mut tampered = report.clone();
    tampered.roots[0].observations[0].completion = Some("complete".into());
    assert!(integration::apply(&store, &tampered).is_err());
    assert_eq!(before, store.snapshot().unwrap());
    integration::apply(&store, &report).unwrap();
    let closed = store.snapshot().unwrap();
    let mut replay = report;
    replay.plan.at += 1;
    assert!(integration::register(&store, &replay.plan).is_err());
    assert!(integration::apply(&store, &replay).is_err());
    assert_eq!(closed, store.snapshot().unwrap());
}

#[test]
fn malformed_locator_is_attributed_and_raw_account_metadata_is_not_persisted() {
    let dir = fixtures();
    let bytes = b"\"AppState\" { \"appid\" \"42\" \"installdir\" \"Synthetic\" \"StateFlags\" \"4\" \"LastOwner\" \"SYNTHETIC-ACCOUNT-DO-NOT-STORE\" \"LastUpdated\" \"999999999\" }";
    fs::write(dir.path().join("steamapps/appmanifest_42.acf"), bytes).unwrap();
    fs::write(dir.path().join("steamapps/appmanifest_43.acf"), b"bad").unwrap();
    let (store, _) = store();
    run(&store, dir.path(), id(), 10);
    let s = store.snapshot().unwrap();
    let hex = b"steamapps/appmanifest_43.acf"
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    assert!(
        s.rows("diagnostics")
            .any(|d| d.text("message").unwrap().contains(&hex))
    );
    assert!(!format!("{s:?}").contains("SYNTHETIC-ACCOUNT-DO-NOT-STORE"));
    assert!(
        s.rows("evidence_records")
            .all(|e| e.get("source_effective_at") == &Value::Null
                && e.int("collected_at").unwrap() == 10)
    );
}

#[test]
fn oversized_records_and_special_files_are_inconclusive_not_absent() {
    use std::os::unix::ffi::OsStrExt;
    let dir = fixtures();
    fs::write(
        dir.path().join("steamapps/appmanifest_42.acf"),
        vec![b'x'; 1_048_577],
    )
    .unwrap();
    let fifo = dir.path().join("steamapps/appmanifest_43.acf");
    let path = std::ffi::CString::new(fifo.as_os_str().as_bytes()).unwrap();
    // SAFETY: NUL-terminated synthetic path in this test's disposable directory.
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
    let (store, _) = store();
    let report = run(&store, dir.path(), id(), 10);
    assert!(
        report.roots[0]
            .diagnostics
            .iter()
            .any(|d| d.code == "oversized-file")
    );
    assert!(
        report.roots[0]
            .diagnostics
            .iter()
            .any(|d| d.code == "special-file-refused")
    );
    assert_eq!(store.snapshot().unwrap().rows("installations").count(), 0);
    assert_eq!(
        store
            .reconstruct(&integration::assessment(&store, 20).unwrap())
            .unwrap()
            .collections[0]
            .outcome,
        "inconclusive"
    );
}

#[test]
fn optional_malformed_title_preserves_installation_and_supported_rule_is_exact() {
    let dir = fixtures();
    fs::write(dir.path().join("steamapps/appmanifest_42.acf"), b"\"AppState\" { \"appid\" \"42\" \"name\" {} \"installdir\" \"Synthetic\" \"StateFlags\" \"4\" }").unwrap();
    let (store, _) = store();
    run(&store, dir.path(), id(), 10);
    let mut s = store.snapshot().unwrap();
    assert_eq!(s.rows("installations").count(), 1);
    let rule = s
        .rows("rule_versions")
        .find(|r| r.is("name", koine_core::rules::STEAM_INTERPRETATION))
        .unwrap()
        .get("id")
        .clone();
    koine_core::validation::supported_rule(&s, &rule, "interpretation").unwrap();
    s.records
        .iter_mut()
        .find(|r| r.table == "rule_versions" && r.get("id") == &rule)
        .unwrap()
        .set("artifact_digest", vec![0u8; 32]);
    assert!(koine_core::validation::supported_rule(&s, &rule, "interpretation").is_err());
}

#[test]
fn concurrent_duplicate_application_uses_serialized_snapshot_and_stable_ids() {
    let dir = fixtures();
    write_manifest(dir.path(), 42, "4");
    let fixture = FixtureFs::open(dir.path()).unwrap();
    let (store, _) = store();
    let plan = fixture.plan(id(), 10).unwrap();
    integration::register(&store, &plan).unwrap();
    let report = fixture
        .scan(plan, Arc::new(AtomicBool::new(false)))
        .unwrap();
    std::thread::scope(|scope| {
        let first = scope.spawn(|| integration::apply(&store, &report));
        let second = scope.spawn(|| integration::apply(&store, &report));
        first.join().unwrap().unwrap();
        second.join().unwrap().unwrap();
    });
    let s = store.snapshot().unwrap();
    assert_eq!(s.rows("evidence_records").count(), 1);
    assert_eq!(s.rows("collection_closures").count(), 1);
    assert_eq!(s.rows("claims").count(), 4);
}

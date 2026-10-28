use koine_core::*;
use koine_sqlite::Store;
use koine_steam::{
    config::{Configuration, normalize},
    controlled, integration,
    scheduler::{Scheduler, State},
    *,
};
use std::{
    collections::BTreeSet,
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
struct Lab {
    _dir: tempfile::TempDir,
    db: PathBuf,
    store: Arc<Store>,
    settings: Arc<Mutex<Configuration>>,
}
impl Lab {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("db.sqlite");
        let store = Arc::new(Store::create(&db).unwrap());
        let settings = Arc::new(Mutex::new(Configuration::open(&store, &db).unwrap()));
        Self {
            _dir: dir,
            db,
            store,
            settings,
        }
    }
    fn add(&self, name: &str, layout: &str) -> String {
        let path = self._dir.path().join(name);
        fixture(&path);
        let rid = id();
        self.settings
            .lock()
            .unwrap()
            .set(&rid, &path, layout, true)
            .unwrap();
        rid
    }
    fn root(&self, rid: &str) -> koine_steam::config::Root {
        self.settings
            .lock()
            .unwrap()
            .roots()
            .iter()
            .find(|r| r.id() == rid)
            .unwrap()
            .clone()
    }
    fn report(
        &self,
        rid: &str,
        cancel: Arc<AtomicBool>,
        progress: &mut dyn FnMut(usize),
    ) -> Report {
        let root = self.root(rid);
        let owner = self.settings.lock().unwrap().owner().to_string();
        let plan = controlled::plan(&owner, &root, id(), 10).unwrap();
        integration::register(&self.store, &plan).unwrap();
        controlled::scan(
            &self.settings,
            &root,
            plan,
            cancel,
            &mut BTreeSet::new(),
            progress,
        )
        .unwrap()
    }
    fn run(&self, rid: &str) -> Report {
        let r = self.report(rid, Arc::new(AtomicBool::new(false)), &mut |_| {});
        integration::apply(&self.store, &r).unwrap();
        r
    }
}
fn fixture(path: &Path) {
    fs::create_dir_all(path.join("steamapps")).unwrap();
    fs::write(
        path.join(".koine-synthetic-root"),
        format!("koine-fixture-v1\n{}\n", id()),
    )
    .unwrap();
    index(path, &[path.to_path_buf()]);
}
fn index(path: &Path, refs: &[PathBuf]) {
    let text = refs
        .iter()
        .enumerate()
        .map(|(i, p)| format!("\"{i}\" {{\"path\" \"{}\"}}", p.display()))
        .collect::<String>();
    fs::write(
        path.join("steamapps/libraryfolders.vdf"),
        format!("\"libraryfolders\" {{{text}}}"),
    )
    .unwrap();
}
fn manifest(path: &Path, app: u32, flags: &str) {
    fs::write(path.join(format!("steamapps/appmanifest_{app}.acf")),format!("\"AppState\" {{\"appid\" \"{app}\" \"name\" \"Synthetic {app}\" \"installdir\" \"Game {app}\" \"StateFlags\" \"{flags}\" \"LastOwner\" \"DO_NOT_RETAIN\"}}")).unwrap();
}
#[test]
fn configuration_is_metadata_only_normalized_private_and_reopens() {
    let lab = Lab::new();
    let rid = id();
    let missing = lab._dir.path().join("not-created");
    lab.settings
        .lock()
        .unwrap()
        .set(&rid, &missing.join("./"), "native", false)
        .unwrap();
    assert!(!missing.exists());
    let settings = Configuration::open(&lab.store, &lab.db).unwrap();
    assert_eq!(settings.roots()[0].path(), missing);
    assert!(!settings.roots()[0].authorized());
    let sidecar = PathBuf::from(format!("{}.steam-roots.koin", lab.db.display()));
    assert_eq!(
        fs::metadata(sidecar).unwrap().permissions().mode() & 0o777,
        0o600
    );
}
#[test]
fn traversal_broad_roots_and_relative_paths_refused_without_canonicalization() {
    for p in ["relative", "/", "/tmp/a/../b"] {
        assert!(normalize(Path::new(p)).is_err());
    }
    assert_eq!(
        normalize(Path::new("/tmp//a/./")).unwrap(),
        Path::new("/tmp/a")
    );
}
#[test]
fn duplicate_spelling_and_config_limit_are_explicit() {
    let lab = Lab::new();
    let r = lab.add("root", "native");
    let root = lab.root(&r);
    assert!(
        lab.settings
            .lock()
            .unwrap()
            .set(&id(), &root.path().join("./"), "native", true)
            .is_err()
    );
    for i in 1..16 {
        lab.settings
            .lock()
            .unwrap()
            .set(
                &id(),
                &lab._dir.path().join(format!("missing{i}")),
                "native",
                true,
            )
            .unwrap();
    }
    assert!(
        lab.settings
            .lock()
            .unwrap()
            .set(&id(), &lab._dir.path().join("overflow"), "native", true)
            .is_err()
    );
}
#[test]
fn real_mode_requires_exact_authorization_and_configuration_change_revokes_it() {
    let lab = Lab::new();
    let r = lab.add("root", "native");
    let path = lab.root(&r).path().to_path_buf();
    manifest(&path, 42, "4");
    lab.settings
        .lock()
        .unwrap()
        .set(&r, &path, "native", false)
        .unwrap();
    let report = lab.run(&r);
    assert_eq!(
        report.roots[0].diagnostics[0].code,
        "explicit-root-authorization-required"
    );
    assert!(!report.roots[0].inspected);
    assert_eq!(
        lab.store
            .snapshot()
            .unwrap()
            .rows("evidence_records")
            .count(),
        0
    );
    // Only application settings are exercised here; no real-mode scan is authorized.
    let scope = lab.root(&r).scope().to_string();
    assert!(
        lab.settings
            .lock()
            .unwrap()
            .authorize(&r, &path.join("other"))
            .is_err()
    );
    lab.settings.lock().unwrap().authorize(&r, &path).unwrap();
    assert!(Configuration::open(&lab.store, &lab.db).unwrap().roots()[0].authorized());
    lab.settings
        .lock()
        .unwrap()
        .set(&r, &path, "flatpak", false)
        .unwrap();
    assert!(!lab.root(&r).authorized());
    assert_ne!(scope, lab.root(&r).scope());
}
#[test]
fn native_and_flatpak_explicit_library_layouts_share_versioned_manifest_semantics() {
    for (name, layout) in [
        ("native/Steam", "native"),
        (
            "packaged/.var/app/com.valvesoftware.Steam/.local/share/Steam",
            "flatpak",
        ),
    ] {
        let lab = Lab::new();
        let r = lab.add(name, layout);
        let root = lab.root(&r);
        manifest(root.path(), 42, "4");
        let report = lab.run(&r);
        assert_eq!(report.outcome(), "complete");
        let s = lab.store.snapshot().unwrap();
        assert_eq!(s.rows("installations").count(), 1);
        let e = s.rows("evidence_records").next().unwrap();
        assert!(e.is("origin_namespace", "koine:steam-synthetic"));
        assert!(
            s.by_id("rule_versions", e.get("interpretation_rule_id"))
                .unwrap()
                .is("name", koine_core::rules::STEAM_CONTROLLED)
        );
        assert!(!format!("{s:?}").contains("DO_NOT_RETAIN"));
        assert_eq!(s.rows("entitlements").count(), 0);
        assert_eq!(s.rows("negative_justifications").count(), 0);
        assert_eq!(s.rows("launch_targets").count(), 0);
    }
}
#[test]
fn multiple_configured_roots_reconcile_distinct_installations_and_reopen() {
    let lab = Lab::new();
    let a = lab.add("one", "native");
    let b = lab.add("two", "flatpak");
    manifest(lab.root(&a).path(), 42, "4");
    manifest(lab.root(&b).path(), 42, "1048576");
    let scheduler = Scheduler::new(lab.store.clone(), lab.settings.clone(), &lab.db, 10).unwrap();
    let t = scheduler.refresh(id(), 10, &[]).unwrap();
    assert_eq!(t.wait().state, State::Complete);
    assert_eq!(t.progress().observations, 2);
    drop(scheduler);
    let s = lab.store.snapshot().unwrap();
    assert_eq!(s.rows("source_entries").count(), 1);
    assert_eq!(s.rows("installations").count(), 2);
    assert_eq!(s.rows("entitlements").count(), 0);
    let assessment = integration::assessment(&lab.store, 10).unwrap();
    let expected = lab.store.reconstruct(&assessment).unwrap();
    let path = lab.db.clone();
    drop(lab.store);
    let reopened = Store::open(&path).unwrap();
    assert_eq!(expected, reopened.reconstruct(&assessment).unwrap());
}
#[test]
fn unapproved_external_and_internal_references_are_never_expanded() {
    let lab = Lab::new();
    let r = lab.add("one", "native");
    let root = lab.root(&r);
    let internal = root.path().join("library B");
    fixture(&internal);
    let outside = lab._dir.path().join("outside");
    fixture(&outside);
    manifest(&internal, 99, "4");
    manifest(&outside, 100, "4");
    manifest(root.path(), 42, "4");
    index(root.path(), &[root.path().to_path_buf(), internal, outside]);
    let report = lab.run(&r);
    assert_eq!(report.roots.len(), 1);
    assert_eq!(report.roots[0].observations.len(), 1);
    assert_eq!(report.outcome(), "partial");
    assert!(
        report
            .index_issues
            .iter()
            .all(|c| c == "referenced-library-not-inspected-needs-separate-scope")
    );
    assert_eq!(
        lab.store.snapshot().unwrap().rows("installations").count(),
        1
    );
}
#[test]
fn root_and_steamapps_symlinks_are_refused() {
    for kind in ["root", "steamapps"] {
        let lab = Lab::new();
        let r = lab.add("one", "native");
        let root = lab.root(&r);
        manifest(root.path(), 42, "4");
        let outside = lab._dir.path().join("renamed");
        if kind == "root" {
            fs::rename(root.path(), &outside).unwrap();
            symlink(&outside, root.path()).unwrap();
        } else {
            fs::rename(root.path().join("steamapps"), &outside).unwrap();
            symlink(&outside, root.path().join("steamapps")).unwrap();
        }
        let report = lab.run(&r);
        assert!(report.roots[0].observations.is_empty());
        assert!(!report.roots[0].inspected);
        assert_eq!(
            lab.store.snapshot().unwrap().rows("installations").count(),
            0
        );
    }
}
#[test]
fn manifest_and_index_symlinks_and_game_directory_links_cannot_expand_scope() {
    let lab = Lab::new();
    let r = lab.add("one", "native");
    let root = lab.root(&r);
    manifest(root.path(), 42, "4");
    let outside = lab._dir.path().join("outside");
    fixture(&outside);
    manifest(&outside, 99, "4");
    symlink(
        outside.join("steamapps/appmanifest_99.acf"),
        root.path().join("steamapps/appmanifest_99.acf"),
    )
    .unwrap();
    fs::remove_file(root.path().join("steamapps/libraryfolders.vdf")).unwrap();
    symlink(
        outside.join("steamapps/libraryfolders.vdf"),
        root.path().join("steamapps/libraryfolders.vdf"),
    )
    .unwrap();
    symlink(&outside, root.path().join("steamapps/common")).unwrap();
    let report = lab.run(&r);
    assert_eq!(report.roots[0].observations.len(), 1);
    assert_eq!(report.outcome(), "partial");
    assert!(report.index_issues.iter().any(|c| c == "symlink-refused"));
}
#[test]
fn unavailable_mount_and_access_failures_preserve_known_claims() {
    for action in ["missing", "unreadable"] {
        let lab = Lab::new();
        let r = lab.add("root", "native");
        let root = lab.root(&r);
        manifest(root.path(), 42, "4");
        lab.run(&r);
        let before = lab
            .store
            .snapshot()
            .unwrap()
            .rows("claims")
            .cloned()
            .collect::<Vec<_>>();
        if action == "missing" {
            fs::rename(root.path(), lab._dir.path().join("detached")).unwrap();
        } else {
            fs::set_permissions(
                root.path().join("steamapps"),
                fs::Permissions::from_mode(0o000),
            )
            .unwrap();
        }
        let report = lab.run(&r);
        assert_eq!(
            report.outcome(),
            if action == "missing" {
                "inconclusive"
            } else {
                "failed"
            }
        );
        assert_eq!(
            lab.store
                .snapshot()
                .unwrap()
                .rows("claims")
                .cloned()
                .collect::<Vec<_>>(),
            before
        );
        assert_eq!(
            lab.store
                .snapshot()
                .unwrap()
                .rows("negative_justifications")
                .count(),
            0
        );
        if action == "unreadable" {
            fs::set_permissions(
                root.path().join("steamapps"),
                fs::Permissions::from_mode(0o755),
            )
            .unwrap();
        }
        let assessment = integration::assessment(&lab.store, 10).unwrap();
        let snapshot = lab.store.snapshot().unwrap();
        let expected = lab.store.reconstruct(&assessment).unwrap();
        let db = lab.db.clone();
        drop(lab.store);
        let reopened = Store::open(db).unwrap();
        assert_eq!(snapshot, reopened.snapshot().unwrap());
        assert_eq!(expected, reopened.reconstruct(&assessment).unwrap());
    }
}
#[test]
fn replaced_root_requires_reconfiguration_instead_of_reusing_installation_identity() {
    let lab = Lab::new();
    let r = lab.add("root", "native");
    let root = lab.root(&r);
    manifest(root.path(), 42, "4");
    lab.run(&r);
    fs::rename(root.path(), lab._dir.path().join("old-root")).unwrap();
    fixture(root.path());
    manifest(root.path(), 99, "4");
    let report = lab.run(&r);
    assert_eq!(
        report.roots[0].diagnostics[0].code,
        "root-identity-changed-reconfigure-required"
    );
    assert_eq!(
        lab.store.snapshot().unwrap().rows("source_entries").count(),
        1
    );
}
#[test]
fn unsupported_packages_and_old_or_malformed_indexes_have_explicit_limits() {
    let lab = Lab::new();
    let r = lab.add("snap", "snap");
    manifest(lab.root(&r).path(), 42, "4");
    let report = lab.run(&r);
    assert_eq!(
        report.roots[0].diagnostics[0].code,
        "unsupported-package-layout"
    );
    for index_bytes in [
        b"\"libraryfolders\" {\"1\" \"/external\"}".as_slice(),
        b"#include \"/external\"",
        b"\"libraryfolders\" {",
    ] {
        let lab = Lab::new();
        let r = lab.add("root", "native");
        manifest(lab.root(&r).path(), 42, "4");
        fs::write(
            lab.root(&r).path().join("steamapps/libraryfolders.vdf"),
            index_bytes,
        )
        .unwrap();
        assert_eq!(lab.run(&r).outcome(), "partial");
    }
}
#[test]
fn malformed_unreadable_incomplete_and_unknown_manifests_keep_independent_observations() {
    let lab = Lab::new();
    let r = lab.add("root", "native");
    let root = lab.root(&r);
    manifest(root.path(), 41, "4");
    manifest(root.path(), 42, "1048576");
    manifest(root.path(), 43, "2147483648");
    manifest(root.path(), 44, "4");
    fs::set_permissions(
        root.path().join("steamapps/appmanifest_44.acf"),
        fs::Permissions::from_mode(0o000),
    )
    .unwrap();
    fs::write(root.path().join("steamapps/appmanifest_45.acf"), "broken").unwrap();
    let report = lab.run(&r);
    assert_eq!(report.roots[0].observations.len(), 3);
    assert_eq!(
        report.roots[0].observations[1].completion.as_deref(),
        Some("incomplete")
    );
    assert!(report.roots[0].observations[2].completion.is_none());
    assert_eq!(report.outcome(), "partial");
    assert_eq!(
        lab.store
            .snapshot()
            .unwrap()
            .rows("claims")
            .filter(|c| c.is("knowledge", "confirmed_negative"))
            .count(),
        0
    );
}
#[test]
fn cancellation_before_scan_and_after_one_valid_observation_is_durable() {
    for after in [false, true] {
        let lab = Lab::new();
        let r = lab.add("root", "native");
        let root = lab.root(&r);
        manifest(root.path(), 41, "4");
        manifest(root.path(), 42, "4");
        let cancel = Arc::new(AtomicBool::new(!after));
        let c = cancel.clone();
        let report = lab.report(&r, cancel, &mut |_| {
            c.store(true, Ordering::SeqCst);
        });
        integration::apply(&lab.store, &report).unwrap();
        assert_eq!(report.roots[0].observations.len(), usize::from(after));
        assert_eq!(
            report.outcome(),
            if after { "partial" } else { "inconclusive" }
        );
        let before = lab.store.snapshot().unwrap();
        let path = lab.db.clone();
        drop(lab.store);
        let reopened = Store::open(path).unwrap();
        assert_eq!(before, reopened.snapshot().unwrap());
        assert_eq!(before.rows("negative_justifications").count(), 0);
    }
}
#[test]
fn duplicate_request_and_result_never_rescan_or_refresh_evidence_time() {
    let lab = Lab::new();
    let r = lab.add("root", "native");
    let root = lab.root(&r);
    manifest(root.path(), 42, "4");
    let scheduler = Scheduler::new(lab.store.clone(), lab.settings.clone(), &lab.db, 10).unwrap();
    let request = id();
    assert_eq!(
        scheduler
            .refresh(request.clone(), 10, std::slice::from_ref(&r))
            .unwrap()
            .wait()
            .observations,
        1
    );
    let expected = lab.store.snapshot().unwrap();
    fs::write(
        root.path().join("steamapps/appmanifest_42.acf"),
        "now invalid",
    )
    .unwrap();
    assert_eq!(
        scheduler
            .refresh(request, 10, &[r])
            .unwrap()
            .wait()
            .observations,
        1
    );
    assert_eq!(expected, lab.store.snapshot().unwrap());
}
#[test]
fn byte_and_directory_entry_bounds_are_coverage_gaps() {
    let lab = Lab::new();
    let r = lab.add("root", "native");
    let root = lab.root(&r);
    for i in 0..8193 {
        fs::write(root.path().join(format!("steamapps/ignored{i}")), b"").unwrap();
    }
    let report = lab.run(&r);
    assert!(
        report.roots[0]
            .diagnostics
            .iter()
            .any(|d| d.code == "directory-entry-limit")
    );
    assert!(!report.roots[0].inspected);
    let lab = Lab::new();
    let r = lab.add("root", "native");
    let root = lab.root(&r);
    fs::write(
        root.path().join("steamapps/appmanifest_42.acf"),
        vec![b'x'; 1_048_577],
    )
    .unwrap();
    let report = lab.run(&r);
    assert!(
        report.roots[0]
            .diagnostics
            .iter()
            .any(|d| d.code == "oversized-file")
    );
}
#[test]
fn delivery_order_does_not_choose_between_unordered_manifest_revisions() {
    let lab = Lab::new();
    let r = lab.add("root", "native");
    let root = lab.root(&r);
    manifest(root.path(), 42, "4");
    let a = lab.report(&r, Arc::new(AtomicBool::new(false)), &mut |_| {});
    manifest(root.path(), 42, "1048576");
    let b = lab.report(&r, Arc::new(AtomicBool::new(false)), &mut |_| {});
    integration::apply(&lab.store, &b).unwrap();
    integration::apply(&lab.store, &a).unwrap();
    let assessment = integration::assessment(&lab.store, 10).unwrap();
    let view = lab.store.reconstruct(&assessment).unwrap();
    assert!(
        view.claims
            .iter()
            .any(|c| c.dimension == "completion" && matches!(c.knowledge, Knowledge::Disputed))
    );
    let mut shuffled = lab.store.snapshot().unwrap();
    shuffled.records.reverse();
    assert_eq!(
        view,
        koine_core::reconcile::reconstruct(&shuffled, &assessment)
    );
}

#[test]
fn scan_byte_budget_stops_with_valid_positives_and_no_negative_authority() {
    let lab = Lab::new();
    let r = lab.add("root", "native");
    let root = lab.root(&r);
    let padding = "x".repeat(900_000);
    for app in 100..120 {
        fs::write(root.path().join(format!("steamapps/appmanifest_{app}.acf")),format!("\"AppState\" {{\"appid\" \"{app}\" \"name\" \"Synthetic\" \"installdir\" \"Synthetic\" \"StateFlags\" \"4\" \"ignored\" \"{padding}\"}}")).unwrap();
    }
    let report = lab.run(&r);
    assert_eq!(report.outcome(), "partial");
    assert!(report.roots[0].observations.len() < 20);
    assert!(!report.roots[0].observations.is_empty());
    assert!(
        report.roots[0]
            .diagnostics
            .iter()
            .any(|d| d.code == "scan-byte-budget-exhausted")
    );
    assert_eq!(
        lab.store
            .snapshot()
            .unwrap()
            .rows("negative_justifications")
            .count(),
        0
    );
}
#[test]
fn root_replacement_during_scan_retains_positives_and_marks_coverage_partial() {
    let lab = Lab::new();
    let r = lab.add("root", "native");
    let root = lab.root(&r);
    manifest(root.path(), 41, "4");
    manifest(root.path(), 42, "4");
    let report = lab.report(&r, Arc::new(AtomicBool::new(false)), &mut |count| {
        if count == 1 {
            fs::rename(root.path(), lab._dir.path().join("detached")).unwrap();
            fixture(root.path());
            manifest(root.path(), 99, "4");
        }
    });
    assert_eq!(report.outcome(), "partial");
    assert_eq!(report.roots[0].observations.len(), 2);
    assert!(
        report.roots[0]
            .diagnostics
            .iter()
            .any(|d| d.code == "root-changed-or-unavailable")
    );
    integration::apply(&lab.store, &report).unwrap();
    assert_eq!(
        lab.store.snapshot().unwrap().rows("source_entries").count(),
        2
    );
}
#[test]
fn explicit_remove_and_reconfigure_rotates_scope_without_erasing_historical_locators() {
    let lab = Lab::new();
    let r = lab.add("first", "native");
    let root = lab.root(&r);
    manifest(root.path(), 42, "4");
    lab.run(&r);
    lab.settings.lock().unwrap().remove(&r).unwrap();
    let path = lab._dir.path().join("second");
    fixture(&path);
    manifest(&path, 42, "4");
    lab.settings
        .lock()
        .unwrap()
        .set(&r, &path, "native", true)
        .unwrap();
    lab.run(&r);
    let s = lab.store.snapshot().unwrap();
    assert_eq!(s.rows("installations").count(), 2);
    assert!(s.rows("claims").any(|c| {
        c.is("dimension", "location")
            && c.get("value_bytes")
                .blob()
                .is_ok_and(|b| b.starts_with(root.path().as_os_str().as_encoded_bytes()))
    }));
    let settings = Configuration::open(&lab.store, &lab.db).unwrap();
    assert_ne!(settings.roots()[0].scope(), root.scope());
}
#[test]
fn request_uuid_is_bound_to_exact_root_set_and_time_across_restarts() {
    let lab = Lab::new();
    let a = lab.add("a", "native");
    let b = lab.add("b", "native");
    manifest(lab.root(&a).path(), 42, "4");
    manifest(lab.root(&b).path(), 43, "4");
    let request = id();
    let scheduler = Scheduler::new(lab.store.clone(), lab.settings.clone(), &lab.db, 10).unwrap();
    assert_eq!(
        scheduler
            .refresh(request.clone(), 10, &[])
            .unwrap()
            .wait()
            .observations,
        2
    );
    assert!(
        scheduler
            .refresh(request.clone(), 11, &[])
            .unwrap_err()
            .contains("already bound")
    );
    assert!(
        scheduler
            .refresh(request.clone(), 10, &[a])
            .unwrap_err()
            .contains("already bound")
    );
    drop(scheduler);
    let scheduler = Scheduler::new(lab.store.clone(), lab.settings.clone(), &lab.db, 20).unwrap();
    assert_eq!(
        scheduler
            .refresh(request, 10, &[])
            .unwrap()
            .wait()
            .observations,
        2
    );
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
fn a_second_scheduler_cannot_recover_or_close_an_active_scheduler_attempt() {
    let lab = Lab::new();
    let _first = Scheduler::new(lab.store.clone(), lab.settings.clone(), &lab.db, 10).unwrap();
    assert!(
        Scheduler::new(lab.store.clone(), lab.settings.clone(), &lab.db, 10)
            .err()
            .unwrap()
            .contains("another discovery scheduler")
    );
}

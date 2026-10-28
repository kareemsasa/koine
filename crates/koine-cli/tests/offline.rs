use std::{fs, process::Command};
fn cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_koine-cli"))
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn explicit_fixture_cli_persists_and_reconstructs_identical_offline_conclusions() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = dir.path().join("fixture");
    fs::create_dir(&fixture).unwrap();
    fs::create_dir(fixture.join("steamapps")).unwrap();
    fs::write(
        fixture.join(".koine-synthetic-root"),
        format!("koine-fixture-v1\n{}\n", koine_core::id()),
    )
    .unwrap();
    fs::write(
        fixture.join("steamapps/libraryfolders.vdf"),
        format!(
            "\"libraryfolders\" {{\"0\" {{\"path\" \"{}\"}}}}",
            fixture.display()
        ),
    )
    .unwrap();
    fs::write(fixture.join("steamapps/appmanifest_42.acf"), "\"AppState\" {\"appid\" \"42\" \"name\" \"Synthetic\" \"installdir\" \"Synthetic\" \"StateFlags\" \"4\"}").unwrap();
    let files = [
        fixture.join(".koine-synthetic-root"),
        fixture.join("steamapps/libraryfolders.vdf"),
        fixture.join("steamapps/appmanifest_42.acf"),
    ];
    let before: Vec<_> = files.iter().map(|f| fs::read(f).unwrap()).collect();
    let db = dir.path().join("library.sqlite");
    let db = db.to_str().unwrap();
    assert!(cli(&["init", db]).status.success());
    let delivery = koine_core::id();
    let args = [
        "steam-fixture",
        db,
        fixture.to_str().unwrap(),
        &delivery,
        "10",
    ];
    let first = cli(&args);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let first = String::from_utf8(first.stdout).unwrap();
    assert!(first.contains("1 observations; 0 diagnostics"));
    assert!(first.contains("Unknown"));
    assert!(first.contains("steamapps") || first.contains("locator_bytes"));
    let reopened = cli(&["inspect", db, "10"]);
    assert!(reopened.status.success());
    assert_eq!(
        &first[first.find("Koiné offline").unwrap()..],
        String::from_utf8(reopened.stdout).unwrap()
    );
    assert_eq!(first, String::from_utf8(cli(&args).stdout).unwrap());
    assert_eq!(
        before,
        files
            .iter()
            .map(|f| fs::read(f).unwrap())
            .collect::<Vec<_>>()
    );
}
#[test]
fn cli_refuses_unmarked_roots_and_never_creates_database_for_discovery() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("not-created.sqlite");
    let output = cli(&[
        "steam-fixture",
        db.to_str().unwrap(),
        dir.path().to_str().unwrap(),
        &koine_core::id(),
        "1",
    ]);
    assert!(!output.status.success());
    assert!(!db.exists());
}

#[test]
fn configured_cli_keeps_configuration_refresh_and_authorization_separate() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("library.sqlite");
    let db = db.to_str().unwrap();
    assert!(cli(&["init", db]).status.success());
    let fixture = dir.path().join("fixture");
    fs::create_dir_all(fixture.join("steamapps")).unwrap();
    fs::write(
        fixture.join(".koine-synthetic-root"),
        format!("koine-fixture-v1\n{}\n", koine_core::id()),
    )
    .unwrap();
    fs::write(
        fixture.join("steamapps/libraryfolders.vdf"),
        format!(
            "\"libraryfolders\" {{\"0\" {{\"path\" \"{}\"}}}}",
            fixture.display()
        ),
    )
    .unwrap();
    fs::write(fixture.join("steamapps/appmanifest_42.acf"),"\"AppState\" {\"appid\" \"42\" \"name\" \"PRIVATE_SYNTHETIC_TITLE\" \"installdir\" \"Synthetic\" \"StateFlags\" \"4\"}").unwrap();
    let rid = koine_core::id();
    let path = fixture.to_str().unwrap();
    assert!(
        cli(&["steam-root-set", db, &rid, "native", "synthetic", path])
            .status
            .success()
    );
    let output = cli(&["steam-status", db]);
    assert!(
        !String::from_utf8(output.stdout)
            .unwrap()
            .contains("collection=")
    );
    let roots = cli(&["steam-roots", db]);
    assert!(roots.status.success());
    assert!(String::from_utf8(roots.stdout).unwrap().contains(path));
    let request = koine_core::id();
    let output = cli(&["steam-refresh", db, &request, "10"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = String::from_utf8(output.stdout).unwrap();
    assert!(output.contains("durable_observations=1"));
    assert!(!output.contains("PRIVATE_SYNTHETIC_TITLE"));
    assert!(!output.contains(path));
    let output = cli(&["steam-evidence", db]);
    assert!(output.status.success());
    let output = String::from_utf8(output.stdout).unwrap();
    assert!(output.contains("koine.steam.synthetic"));
    assert!(output.contains("value=complete"));
    assert!(!output.contains("PRIVATE_SYNTHETIC_TITLE"));
    assert!(!output.contains(path));
    // Real mode only exercises the denied gate on this synthetic directory.
    assert!(
        cli(&["steam-root-set", db, &rid, "native", "real", path])
            .status
            .success()
    );
    let output = cli(&["steam-refresh", db, &koine_core::id(), "11"]);
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("Inconclusive")
    );
    let output = cli(&["steam-status", db]);
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("explicit-root-authorization-required")
    );
    assert!(cli(&["steam-recover", db]).status.success());
}
#[test]
fn interactive_session_accepts_progress_and_cancel_without_another_database_owner() {
    use std::{io::Write, process::Stdio};
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("library.sqlite");
    let db = db.to_str().unwrap();
    assert!(cli(&["init", db]).status.success());
    let missing = dir.path().join("synthetic-unavailable");
    let rid = koine_core::id();
    assert!(
        cli(&[
            "steam-root-set",
            db,
            &rid,
            "native",
            "synthetic",
            missing.to_str().unwrap()
        ])
        .status
        .success()
    );
    let mut child = Command::new(env!("CARGO_BIN_EXE_koine-cli"))
        .args(["steam-session", db])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let request = koine_core::id();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(
            format!(
                "refresh {request} 10\nstatus {request}\ncancel {request}\nwait {request}\nquit\n"
            )
            .as_bytes(),
        )
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let output = String::from_utf8(output.stdout).unwrap();
    assert!(output.contains("request="));
    assert!(output.contains("state=Inconclusive"));
    assert!(output.contains("finished=true"));
    assert!(cli(&["steam-status", db]).status.success());
}

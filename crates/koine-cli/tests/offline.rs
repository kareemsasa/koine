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

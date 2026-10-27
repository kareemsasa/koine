//! Explicit offline database tools; no discovery paths or launcher commands.
mod demo;
use koine_core::{reconcile, *};
use koine_sqlite::Store;
fn main() {
    if let Err(error) = run() {
        eprintln!("Koiné: {error}");
        std::process::exit(1)
    }
}
fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err(
            "usage: koine-cli init|inspect|demo DATABASE (new path required for init/demo)".into(),
        );
    }
    let store = match args[1].as_str() {
        "init" => Store::create(&args[2])?,
        "demo" => {
            let store = Store::create(&args[2])?;
            demo::populate(&store)?;
            store
        }
        "inspect" => Store::open(&args[2])?,
        _ => return Err("unknown offline command".into()),
    };
    let snapshot = store.snapshot()?;
    reconcile::validate(&snapshot)?;
    println!("Koiné offline foundation; SQLite {}", rusqlite_version());
    println!(
        "{} immutable records, {} entries, {} collections, {} attempts",
        snapshot.records.len(),
        snapshot.rows("source_entries").count(),
        snapshot.rows("collections").count(),
        snapshot.rows("launch_attempts").count()
    );
    let rule = |kind: &str| {
        snapshot
            .rows("rule_versions")
            .find(|r| r.is("kind", kind))
            .and_then(|r| r.text("id").ok())
            .map(String::from)
    };
    if let (Some(reconciliation_rule), Some(freshness_rule)) =
        (rule("reconciliation"), rule("freshness"))
    {
        let view = store.reconstruct(&Assessment {
            at: 100,
            observer_epoch: None,
            reconciliation_rule,
            freshness_rule,
        })?;
        for attempt in view.attempts {
            println!(
                "handoff={:?}; issuance={}; status={:?}; process={:?}",
                attempt.handoff, attempt.issuance, attempt.exit_status, attempt.process
            )
        }
    }
    Ok(())
}
fn rusqlite_version() -> &'static str {
    koine_sqlite::runtime_version()
}

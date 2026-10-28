//! Explicit offline tools. Steam discovery accepts marked synthetic roots only.
mod demo;
use koine_core::{reconcile, *};
use koine_sqlite::Store;
use std::sync::{Arc, atomic::AtomicBool};
const USAGE: &str = "usage: koine-cli init|demo DATABASE (new path); inspect DATABASE [ASSESSMENT_MICROS]; steam-fixture DATABASE ABSOLUTE_FIXTURE_ROOT DELIVERY_UUID COLLECTED_AT_MICROS (existing database)";
fn main() {
    if let Err(error) = run() {
        eprintln!("Koiné: {error}");
        std::process::exit(1)
    }
}
fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 3 {
        return Err(USAGE.into());
    }
    let mut at = 100;
    let store = match args[1].as_str() {
        "init" if args.len() == 3 => Store::create(&args[2])?,
        "demo" if args.len() == 3 => {
            let store = Store::create(&args[2])?;
            demo::populate(&store)?;
            store
        }
        "inspect" if (3..=4).contains(&args.len()) => {
            if let Some(value) = args.get(3) {
                at = value.parse().map_err(|_| "invalid assessment time")?;
            }
            Store::open(&args[2])?
        }
        "steam-fixture" if args.len() == 6 => {
            at = args[5].parse().map_err(|_| "invalid collection time")?;
            let fixture = koine_steam::FixtureFs::open(std::path::Path::new(&args[3]))?;
            let store = Store::open(&args[2])?;
            let report = koine_steam::integration::discover(
                &store,
                &fixture,
                args[4].clone(),
                at,
                Arc::new(AtomicBool::new(false)),
            )?;
            println!(
                "Steam fixture interpretation v1: {} requested library roots; {} observations; {} diagnostics; no absence or entitlement authority",
                report.roots.len(),
                report
                    .roots
                    .iter()
                    .map(|r| r.observations.len())
                    .sum::<usize>(),
                report.index_issues.len()
                    + report
                        .roots
                        .iter()
                        .map(|r| r.diagnostics.len())
                        .sum::<usize>()
            );
            store
        }
        _ => return Err(USAGE.into()),
    };
    let snapshot = store.snapshot()?;
    reconcile::validate(&snapshot)?;
    println!(
        "Koiné offline foundation; SQLite {}",
        koine_sqlite::runtime_version()
    );
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
            at,
            observer_epoch: None,
            reconciliation_rule,
            freshness_rule,
        })?;
        for collection in view.collections {
            println!(
                "collection={}; outcome={}; diagnostics={:?}",
                collection.collection, collection.outcome, collection.limitations
            );
        }
        for claim in view.claims {
            println!(
                "subject={}; dimension={}; knowledge={:?}; freshness={:?}; supporting_claims={:?}; last_known_claims={:?}",
                claim.subject,
                claim.dimension,
                claim.knowledge,
                claim.freshness,
                claim.supporting_claims,
                claim.last_known_claims
            );
        }
        for attempt in view.attempts {
            println!(
                "handoff={:?}; issuance={}; status={:?}; process={:?}",
                attempt.handoff, attempt.issuance, attempt.exit_status, attempt.process
            )
        }
        for evidence in snapshot.rows("evidence_records") {
            println!(
                "evidence={}; rule={}; collected_at={}; source_effective_at={:?}; source_revision={:?}; locator_bytes={:?}",
                evidence.text("id")?,
                evidence.text("interpretation_rule_id")?,
                evidence.int("collected_at")?,
                evidence.get("source_effective_at"),
                evidence.get("source_revision"),
                evidence.get("locator")
            );
        }
    }
    Ok(())
}

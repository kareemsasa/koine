//! Opt-in process-kill probe. Newly created disposable DBs and marked fixtures only.
use crate::{config::Configuration, journal::Journal, *};
use std::{
    collections::BTreeSet,
    io::{self, Write},
    sync::Mutex,
};
/// Pause at an identified disposable discovery boundary for an external test kill.
pub fn crash_probe(database: &Path, fixture: &Path, phase: &str) -> Result<()> {
    if !["registered", "journaled"].contains(&phase) {
        return Err("unsupported crash boundary".into());
    }
    let _marked = FixtureFs::open(fixture)?;
    let store = koine_sqlite::Store::create_fault_lab(database)?;
    let settings = Mutex::new(Configuration::open(&store, database)?);
    let rid = id();
    settings
        .lock()
        .unwrap()
        .set(&rid, fixture, "native", true)?;
    let (owner, root) = {
        let c = settings.lock().unwrap();
        (c.owner().to_string(), c.roots()[0].clone())
    };
    let plan = controlled::plan(&owner, &root, id(), 10)?;
    integration::register(&store, &plan)?;
    let journal = Journal::open(database)?;
    if phase == "journaled" {
        let report = controlled::scan(
            &settings,
            &root,
            plan,
            Arc::new(AtomicBool::new(false)),
            &mut BTreeSet::new(),
            &mut |_| {},
        )?;
        journal.save(&report)?;
    }
    println!("disposable-discovery-boundary={phase}");
    io::stdout().flush().map_err(|_| "probe stdout failed")?;
    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .map_err(|_| "probe stdin failed")?;
    Err("crash probe continued without external kill".into())
}

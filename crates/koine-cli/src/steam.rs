//! Explicit offline configuration and application-local discovery commands.
use koine_core::*;
use koine_sqlite::Store;
use koine_steam::{
    config::Configuration,
    scheduler::{Scheduler, Ticket},
};
use std::{
    collections::BTreeMap,
    io::{self, BufRead, Write},
    path::Path,
    sync::{Arc, Mutex},
};
fn time() -> Result<i64> {
    i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| "clock before epoch")?
            .as_micros(),
    )
    .map_err(|_| "clock overflow".into())
}
fn show(t: &Ticket) {
    let p = t.progress();
    println!(
        "request={}; state={:?}; finished={}; durable_observations={}; inspected_records={}; completed_roots={}; attempts={:?}; outcomes={:?}; error={:?}",
        t.id,
        p.state,
        p.finished,
        p.observations,
        p.inspected_records,
        p.completed_roots,
        p.attempts,
        p.outcomes,
        p.error
    );
}
pub fn run(args: &[String]) -> Result<bool> {
    let command = args[1].as_str();
    if !matches!(
        command,
        "steam-roots"
            | "steam-root-set"
            | "steam-root-remove"
            | "steam-authorize"
            | "steam-refresh"
            | "steam-session"
            | "steam-status"
            | "steam-evidence"
            | "steam-recover"
    ) {
        return Ok(false);
    }
    let store = Arc::new(Store::open(&args[2])?);
    if matches!(command, "steam-status" | "steam-evidence") {
        if args.len() != 3 {
            return Err("usage: steam-status|steam-evidence DATABASE".into());
        }
        let s = store.snapshot()?;
        for c in s.rows("collections").filter(|c| {
            c.text("reporter")
                .is_ok_and(|r| r.starts_with("koine.steam."))
        }) {
            let closure = s
                .find("collection_closures", "collection_id", c.get("id"))
                .ok();
            println!(
                "collection={}; reporter={}; version={}; collected_at={}; outcome={}",
                c.text("id")?,
                c.text("reporter")?,
                c.text("reporter_version")?,
                c.int("started_at")?,
                closure.map_or("pending (or interrupted; run steam-recover)", |c| c
                    .text("outcome")
                    .unwrap_or("invalid"))
            );
        }
        for d in s.rows("diagnostics") {
            println!(
                "diagnostic={}; collection={:?}; dimension={:?}",
                d.text("code")?,
                d.get("collection_id"),
                d.get("affected_dimension")
            );
        }
        if command == "steam-evidence" {
            for i in s.rows("installations") {
                println!(
                    "installation={}; source_entry={}; root_scope={}",
                    i.text("id")?,
                    i.text("entry_id")?,
                    i.text("instance_key")?
                );
                for subject in s
                    .rows("claim_subjects")
                    .filter(|sub| sub.get("installation_id") == i.get("id"))
                {
                    for claim in s
                        .rows("claims")
                        .filter(|c| c.get("subject_id") == subject.get("id"))
                    {
                        let e = s.by_id("evidence_records", claim.get("evidence_id"))?;
                        let rule = s.by_id("rule_versions", e.get("interpretation_rule_id"))?;
                        println!(
                            "  dimension={}; knowledge={}; value={}; collected_at={}; interpreter={}; evidence={}",
                            claim.text("dimension")?,
                            claim.text("knowledge")?,
                            claim
                                .text("value_text")
                                .unwrap_or("[root-relative location withheld]"),
                            e.int("collected_at")?,
                            rule.text("name")?,
                            e.text("id")?
                        );
                    }
                }
            }
            println!(
                "Presence is a manifest observation; completeness is separate; entitlement and launchability remain unknown."
            );
        }
        return Ok(true);
    }
    let settings = Arc::new(Mutex::new(Configuration::open(
        &store,
        Path::new(&args[2]),
    )?));
    match command {
        "steam-roots" if args.len()==3=>{
            for r in &settings.lock().map_err(|_|"settings poisoned")?.roots {println!("root={}; scope={}; layout={}; synthetic={}; authorized={}; physical_binding={}; path={}",r.id,r.scope,r.layout,r.synthetic,r.authorized,r.binding.is_some(),r.path.display());}
        }
        "steam-root-set" if args.len()==7=>{
            let synthetic=match args[5].as_str(){"synthetic"=>true,"real"=>false,_=>return Err("root mode must be synthetic or real".into())};
            settings.lock().map_err(|_|"settings poisoned")?.set(&args[3],Path::new(&args[6]),&args[4],synthetic)?;
            println!("Configured root {}; discovery has not been requested. Real scope requires steam-authorize before any Steam source reads.",args[3]);
        }
        "steam-root-remove" if args.len()==4=>{settings.lock().map_err(|_|"settings poisoned")?.remove(&args[3])?;println!("Configuration removed; historical evidence retained.");}
        "steam-authorize" if args.len()==6 && args[5]=="--read-manifests-and-index-only"=>{
            settings.lock().map_err(|_|"settings poisoned")?.authorize(&args[3],Path::new(&args[4]))?;
            println!("Authorized exact configured root {} for steamapps/libraryfolders.vdf and steamapps/appmanifest_*.acf, directory names and metadata only. References may name other roots; none will be followed. Read-only, no symlinks or descendant mount crossings. Refresh remains a separate operation.",args[3]);
        }
        "steam-refresh" if args.len()>=5=>{
            let at=args[4].parse().map_err(|_|"invalid collection time")?;
            let scheduler=Scheduler::new(store.clone(),settings,Path::new(&args[2]),time()?)?;
            let ticket=scheduler.refresh(args[3].clone(),at,&args[5..])?;
            show(&ticket);let result=ticket.wait();show(&ticket);
            if let Some(error)=result.error {return Err(error);}
        }
        "steam-recover" if args.len()==3=>{let _scheduler=Scheduler::new(store.clone(),settings,Path::new(&args[2]),time()?)?;println!("Durable results replayed; interrupted requests are inconclusive. No source filesystem was inspected.");}
        "steam-session" if args.len()==3=>session(Scheduler::new(store.clone(),settings,Path::new(&args[2]),time()?)?)?,
        _=>return Err("usage: steam-roots DATABASE; steam-root-set DATABASE ROOT_UUID native|flatpak synthetic|real ABSOLUTE_ROOT; steam-root-remove DATABASE ROOT_UUID; steam-authorize DATABASE ROOT_UUID EXACT_PATH --read-manifests-and-index-only; steam-refresh DATABASE REQUEST_UUID COLLECTED_AT_MICROS [ROOT_UUID ...]; steam-session|steam-status|steam-evidence|steam-recover DATABASE".into()),
    }
    Ok(true)
}
fn session(mut scheduler: Scheduler) -> Result<()> {
    println!(
        "Local Steam session: refresh REQUEST_UUID TIME [ROOT_UUID ...]; status REQUEST_UUID; cancel REQUEST_UUID; wait REQUEST_UUID; recover; quit. EOF cancels and joins accepted work."
    );
    io::stdout().flush().map_err(|_| "stdout unavailable")?;
    let mut tickets: BTreeMap<String, Ticket> = BTreeMap::new();
    for line in io::stdin().lock().lines() {
        let line = line.map_err(|_| "stdin unavailable")?;
        let words: Vec<_> = line.split_whitespace().collect();
        let result: Result<()> = match words.as_slice() {
            ["quit"] => break,
            ["recover"] => scheduler.recover(time()?),
            ["refresh", request, at, roots @ ..] => {
                // Completed progress is durable in SQLite; cap the session's ticket cache.
                if tickets.len() >= 64 {
                    tickets.retain(|_, t| !t.progress().finished);
                }
                match at
                    .parse::<i64>()
                    .map_err(|_| "invalid collection time".into())
                    .and_then(|at| {
                        scheduler.refresh(
                            (*request).into(),
                            at,
                            &roots.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
                        )
                    }) {
                    Ok(t) => {
                        show(&t);
                        tickets.insert((*request).into(), t);
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            }
            [action, request] if ["status", "cancel", "wait"].contains(action) => {
                if let Some(t) = tickets.get(*request) {
                    if *action == "cancel" {
                        t.cancel();
                    }
                    if *action == "wait" {
                        t.wait();
                    }
                    show(t);
                    Ok(())
                } else {
                    Err("unknown session request; durable outcomes: steam-status after exit".into())
                }
            }
            _ => Err("unknown session command".into()),
        };
        if let Err(e) = result {
            println!("request error: {e}");
        }
        io::stdout().flush().map_err(|_| "stdout unavailable")?;
    }
    scheduler.shutdown()?;
    for ticket in tickets.values() {
        show(ticket);
    }
    Ok(())
}

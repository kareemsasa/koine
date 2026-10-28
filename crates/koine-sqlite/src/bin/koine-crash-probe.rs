//! Fault-injection child for disposable synthetic databases only. No launcher.
use koine_core::{canonical::CATALOG, *};
use koine_sqlite::Store;
use rusqlite::{Connection, params};
use std::{
    io::{self, Write},
    path::Path,
};
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(2)
    }
}
fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 6 {
        return Err(
            "usage: koine-crash-probe MODE DISPOSABLE_DATABASE ATTEMPT TARGET TRANSPORT_RULE"
                .into(),
        );
    }
    let mode = &args[1];
    let path = Path::new(&args[2]);
    let aid = &args[3];
    let target = &args[4];
    let rule = &args[5];
    let intent = Record::new(
        "launch_attempts",
        [
            ("id", aid.clone().into()),
            ("target_id", target.clone().into()),
            ("created_at", 20i64.into()),
            ("intent_reason", "synthetic fault probe".into()),
        ],
    );
    let mut held_store = None;
    let mut held_connection = None;
    if mode == "uncommitted-result" {
        let c = Connection::open(path).map_err(|e| e.to_string())?;
        c.execute_batch("PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL; BEGIN IMMEDIATE")
            .map_err(|e| e.to_string())?;
        c.execute("INSERT INTO coverage_results(id,collection_id,unit_id,dimension,authority_scope_id,outcome) SELECT ?1,collection_id,id,dimension,authority_scope_id,'complete' FROM requested_units WHERE collection_id=?2 LIMIT 1",params![id(),aid]).map_err(|e|e.to_string())?;
        held_connection = Some(c);
    } else if mode == "uncommitted-intent" {
        let c = Connection::open(path).map_err(|e| e.to_string())?;
        c.execute_batch("PRAGMA foreign_keys=ON; BEGIN IMMEDIATE")
            .map_err(|e| e.to_string())?;
        c.execute("INSERT INTO launch_attempts(id,target_id,created_at,intent_reason) VALUES(?1,?2,20,'synthetic fault probe')",params![aid,target]).map_err(|e|e.to_string())?;
        held_connection = Some(c);
    } else {
        let store = Store::open(path)?;
        if mode == "committed-result" {
            let s = store.snapshot()?;
            let mut rows = Vec::new();
            for unit in s
                .rows("requested_units")
                .filter(|u| u.is("collection_id", aid))
            {
                let cid = id();
                let common = [
                    ("collection_id", aid.clone().into()),
                    ("unit_id", unit.get("id").clone()),
                    ("dimension", unit.get("dimension").clone()),
                    ("authority_scope_id", unit.get("authority_scope_id").clone()),
                    ("outcome", "complete".into()),
                ];
                let mut coverage = Record::new("coverage_results", common.clone());
                coverage.set("id", cid.clone());
                let mut part = Record::new("coverage_parts", common);
                part.set("id", id());
                part.set("coverage_id", cid);
                part.set("part_key", "whole");
                part.set("selector_kind", "whole_unit");
                part.set("inspection", "inspected");
                rows.extend([coverage, part]);
            }
            rows.push(Record::new(
                "collection_closures",
                [
                    ("collection_id", aid.clone().into()),
                    ("outcome", "complete".into()),
                    ("finished_at", 20i64.into()),
                    ("reason", "synthetic crash boundary".into()),
                    ("result_protocol", "koine.result.v1".into()),
                ],
            ));
            store.write(rows)?;
        } else {
            store.write(vec![intent])?;
        }
        if mode == "committed-failure" {
            store.write(vec![Record::new(
                "handoff_records",
                [
                    ("id", id().into()),
                    ("attempt_id", aid.clone().into()),
                    ("item_key", "synthetic result".into()),
                    ("recorded_at", 21i64.into()),
                    ("transport_rule_id", rule.clone().into()),
                    ("boundary", "fixture transport".into()),
                    ("issuance", "issued".into()),
                    ("disposition", "failed".into()),
                    ("exit_status", 127i64.into()),
                    ("reason", "synthetic status-127 failure".into()),
                ],
            )])?;
        } else if mode != "committed-intent" && mode != "committed-result" {
            return Err("unsupported fault mode".into());
        }
        held_store = Some(store);
    }
    println!("READY");
    io::stdout().flush().map_err(|e| e.to_string())?;
    // Parent kills this process here, without Rust destructors or SQLite close.
    loop {
        std::hint::black_box((&held_store, &held_connection, CATALOG.len()));
        std::thread::park();
    }
}

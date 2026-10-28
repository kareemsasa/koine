//! Private application recovery files. These are local encodings, not import APIs.
use crate::{
    config::{boolean, list, optional},
    *,
};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt},
};
const LIMIT: u64 = 64 * 1024 * 1024;
fn text(v: &Value) -> Result<String> {
    v.text().map(String::from)
}
fn strings(v: &Value) -> Result<Vec<String>> {
    list(v)?.iter().map(text).collect()
}
fn string_list(v: &[String]) -> Value {
    Value::List(v.iter().cloned().map(Value::Text).collect())
}
fn root(v: &LibraryRoot) -> Value {
    Value::List(vec![
        v.key.clone().into(),
        v.relative
            .as_ref()
            .map_or(Value::Null, |p| fixture_fs::bytes(p).into()),
        v.issue.clone().map_or(Value::Null, Value::Text),
    ])
}
fn parse_root(v: &Value) -> Result<LibraryRoot> {
    let f = list(v)?;
    if f.len() != 3 {
        return Err("invalid journal root".into());
    }
    Ok(LibraryRoot {
        key: text(&f[0])?,
        relative: if f[1] == Value::Null {
            None
        } else {
            Some(fixture_fs::from_bytes(f[1].blob()?.into()))
        },
        issue: optional(&f[2])?,
    })
}
fn wire(r: &Report) -> Value {
    Value::List(vec![
        "koine.steam.journal.v1".into(),
        r.plan.profile.reporter().into(),
        r.plan.delivery.clone().into(),
        r.plan.fixture.clone().into(),
        r.plan.at.into(),
        r.plan
            .index_revision
            .clone()
            .map_or(Value::Null, Value::Blob),
        string_list(&r.plan.index_issues),
        Value::List(r.plan.roots.iter().map(root).collect()),
        string_list(&r.index_issues),
        r.plan.boundary.clone().map_or(Value::Null, Value::Blob),
        r.plan
            .physical_binding
            .clone()
            .map_or(Value::Null, Value::Blob),
        Value::List(
            r.roots
                .iter()
                .map(|r| {
                    Value::List(vec![
                        root(&r.root),
                        i64::from(r.inspected).into(),
                        Value::List(
                            r.observations
                                .iter()
                                .map(|o| {
                                    Value::List(vec![
                                        i64::from(o.app_id).into(),
                                        o.reported_id.clone().into(),
                                        o.name.clone().map_or(Value::Null, Value::Text),
                                        o.install_dir.clone().into(),
                                        o.completion.clone().map_or(Value::Null, Value::Text),
                                        i64::from(o.presence).into(),
                                        o.flags.map_or(Value::Null, |f| i64::from(f).into()),
                                        o.locator.clone().into(),
                                        o.revision.clone().into(),
                                    ])
                                })
                                .collect(),
                        ),
                        Value::List(
                            r.diagnostics
                                .iter()
                                .map(|d| {
                                    Value::List(vec![
                                        d.code.clone().into(),
                                        d.dimension.clone().map_or(Value::Null, Value::Text),
                                        d.locator.clone().into(),
                                    ])
                                })
                                .collect(),
                        ),
                    ])
                })
                .collect(),
        ),
    ])
}
fn parse(v: &Value) -> Result<Report> {
    let f = list(v)?;
    if f.len() != 12 || f[0].text()? != "koine.steam.journal.v1" {
        return Err("unsupported journal".into());
    }
    let profile = match f[1].text()? {
        "koine.steam.synthetic" => Profile::SyntheticV2,
        "koine.steam.configured" => Profile::RealV2,
        _ => return Err("unsupported journal reporter".into()),
    };
    let plan = Plan {
        profile,
        physical_binding: if f[10] == Value::Null {
            None
        } else {
            Some(f[10].blob()?.into())
        },
        boundary: if f[9] == Value::Null {
            None
        } else {
            Some(f[9].blob()?.into())
        },
        delivery: text(&f[2])?,
        fixture: text(&f[3])?,
        at: f[4].integer()?,
        index_revision: if f[5] == Value::Null {
            None
        } else {
            Some(f[5].blob()?.into())
        },
        index_issues: strings(&f[6])?,
        roots: list(&f[7])?.iter().map(parse_root).collect::<Result<_>>()?,
    };
    let mut roots = Vec::new();
    for r in list(&f[11])? {
        let r = list(r)?;
        if r.len() != 4 {
            return Err("invalid journal report".into());
        }
        let mut observations = Vec::new();
        for o in list(&r[2])? {
            let o = list(o)?;
            if o.len() != 9 {
                return Err("invalid journal observation".into());
            }
            observations.push(Observation {
                app_id: u32::try_from(o[0].integer()?).map_err(|_| "invalid journal appid")?,
                reported_id: text(&o[1])?,
                name: optional(&o[2])?,
                install_dir: text(&o[3])?,
                completion: optional(&o[4])?,
                presence: boolean(&o[5])?,
                flags: if o[6] == Value::Null {
                    None
                } else {
                    Some(u32::try_from(o[6].integer()?).map_err(|_| "invalid journal flags")?)
                },
                locator: o[7].blob()?.into(),
                revision: o[8].blob()?.into(),
            });
        }
        let mut diagnostics = Vec::new();
        for d in list(&r[3])? {
            let d = list(d)?;
            if d.len() != 3 {
                return Err("invalid journal diagnostic".into());
            }
            diagnostics.push(Diagnostic {
                code: text(&d[0])?,
                dimension: optional(&d[1])?,
                locator: d[2].blob()?.into(),
            });
        }
        roots.push(RootReport {
            root: parse_root(&r[0])?,
            inspected: boolean(&r[1])?,
            observations,
            diagnostics,
        });
    }
    let report = Report {
        plan,
        roots,
        index_issues: strings(&f[8])?,
    };
    report.validate()?;
    Ok(report)
}
pub(crate) struct Journal(PathBuf, File);
impl Journal {
    pub fn open(database: &Path) -> Result<Self> {
        let mut path = fs::canonicalize(database)
            .map_err(|_| "database path unavailable")?
            .into_os_string();
        path.push(".steam-journal");
        let path = PathBuf::from(path);
        match fs::DirBuilder::new().mode(0o700).create(&path) {
            Ok(()) => File::open(path.parent().ok_or("journal parent")?)
                .and_then(|f| f.sync_all())
                .map_err(|_| "journal parent sync failed")?,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
            Err(_) => return Err("journal creation failed".into()),
        }
        let m = fs::symlink_metadata(&path).map_err(|_| "journal metadata failed")?;
        if !m.is_dir() || m.permissions().mode() & 0o077 != 0 {
            return Err("journal must be a private directory (no symlink)".into());
        }
        let lease = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(path.join("scheduler.lock"))
            .map_err(|_| "scheduler lock unavailable")?;
        if !lease
            .metadata()
            .map_err(|_| "scheduler lock metadata failed")?
            .is_file()
        {
            return Err("scheduler lock must be regular".into());
        }
        lease
            .try_lock()
            .map_err(|_| "another discovery scheduler owns this database")?;
        Ok(Self(path, lease))
    }
    /// Reserve a refresh UUID for one exact scope/time/configuration snapshot.
    /// The ledger contains only a digest; original selectors remain in SQLite.
    pub fn bind_request(&self, request: &str, signature: &Value) -> Result<()> {
        config::uuid(request)?;
        let directory = self.0.join("requests");
        match fs::DirBuilder::new().mode(0o700).create(&directory) {
            Ok(()) => self.sync()?,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
            Err(_) => return Err("request ledger unavailable".into()),
        }
        let m = fs::symlink_metadata(&directory).map_err(|_| "request ledger metadata failed")?;
        if !m.is_dir() || m.permissions().mode() & 0o077 != 0 {
            return Err("request ledger must be private (no symlink)".into());
        }
        let path = directory.join(format!("{request}.koin"));
        let data = canonical::encode(&Value::List(vec![
            "koine.steam.request-binding.v1".into(),
            request.into(),
            canonical::hash(&canonical::encode(signature)).into(),
        ]));
        let matches = || -> Result<()> {
            let file = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
                .open(&path)
                .map_err(|_| "request ledger read failed")?;
            if !file
                .metadata()
                .map_err(|_| "request ledger metadata failed")?
                .is_file()
            {
                return Err("request ledger must be regular".into());
            }
            let mut existing = Vec::new();
            file.take(1024)
                .read_to_end(&mut existing)
                .map_err(|_| "request ledger read failed")?;
            if existing != data {
                return Err(
                    "request UUID already bound to different scope, configuration or time".into(),
                );
            }
            Ok(())
        };
        if fs::symlink_metadata(&path).is_ok() {
            return matches();
        }
        let temp = self.0.join(format!("{}.partial", id()));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp)
            .map_err(|_| "request staging failed")?;
        file.write_all(&data)
            .and_then(|()| file.sync_all())
            .map_err(|_| "request sync failed")?;
        match fs::hard_link(&temp, &path) {
            Ok(()) => (),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => matches()?,
            Err(_) => return Err("request ledger finalization failed".into()),
        }
        File::open(&directory)
            .and_then(|f| f.sync_all())
            .map_err(|_| "request ledger directory sync failed")?;
        fs::remove_file(temp).map_err(|_| "request staging cleanup failed")?;
        self.sync()
    }
    fn path(&self, id: &str) -> Result<PathBuf> {
        config::uuid(id)?;
        Ok(self.0.join(format!("{id}.koin")))
    }
    pub fn save(&self, report: &Report) -> Result<()> {
        let data = canonical::encode(&Value::List(vec![
            wire(report),
            report.fingerprint().into(),
        ]));
        if data.len() as u64 > LIMIT {
            return Err("journal report size limit".into());
        }
        let temp = self.0.join(format!("{}.partial", id()));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp)
            .map_err(|_| "journal staging failed")?;
        file.write_all(&data)
            .and_then(|()| file.sync_all())
            .map_err(|_| "journal sync failed; result not acknowledged")?;
        // No accepted attempt is re-scanned; recovery reuses this exact report.
        fs::rename(temp, self.path(&report.plan.delivery)?)
            .map_err(|_| "journal finalization failed")?;
        self.sync()
    }
    fn sync(&self) -> Result<()> {
        let _lease = &self.1;
        File::open(&self.0)
            .and_then(|f| f.sync_all())
            .map_err(|_| "journal directory sync failed".into())
    }
    pub fn remove(&self, delivery: &str) -> Result<()> {
        fs::remove_file(self.path(delivery)?).map_err(|_| "journal cleanup failed")?;
        self.sync()
    }
    pub fn recover(&self, store: &koine_sqlite::Store) -> Result<usize> {
        let mut files = Vec::new();
        let mut total = 0;
        for entry in fs::read_dir(&self.0).map_err(|_| "journal listing failed")? {
            let entry = entry.map_err(|_| "journal listing failed")?;
            total += 1;
            if total > 128 {
                return Err(
                    "journal directory entry bound exceeded; preserve files for inspection".into(),
                );
            }
            if entry.file_name().as_encoded_bytes().ends_with(b".koin") {
                files.push(entry.path());
            }
            if files.len() > 48 {
                return Err("journal recovery bound exceeded".into());
            }
        }
        files.sort();
        let mut count = 0;
        for path in files {
            let file = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
                .open(&path)
                .map_err(|_| "journal read failed")?;
            if !file
                .metadata()
                .map_err(|_| "journal metadata failed")?
                .is_file()
            {
                return Err("journal must be regular".into());
            }
            let mut bytes = Vec::new();
            file.take(LIMIT + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| "journal read failed")?;
            if bytes.len() as u64 > LIMIT {
                return Err("journal size limit".into());
            }
            let envelope = canonical::decode(&bytes)?;
            let pair = list(&envelope)?;
            if pair.len() != 2 {
                return Err("journal envelope mismatch".into());
            }
            let report = parse(&pair[0])?;
            if pair[1].blob()? != report.fingerprint() {
                return Err("journal fingerprint mismatch".into());
            }
            if path != self.path(&report.plan.delivery)? {
                return Err("journal delivery mismatch".into());
            }
            integration::apply(store, &report)?;
            self.remove(&report.plan.delivery)?;
            count += 1;
        }
        Ok(count)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn journal_rejects_bad_envelopes_without_panicking() {
        assert!(parse(&Value::Null).is_err());
        assert!(parse(&Value::List(vec![])).is_err());
    }
    #[test]
    fn journal_roundtrip_replays_after_restart_without_reading_source() {
        let lab = crate::test_support::Lab::new();
        let root = lab.add("root");
        let report = lab.report(&root);
        assert_eq!(parse(&wire(&report)).unwrap(), report);
        let journal = Journal::open(&lab.db).unwrap();
        journal.save(&report).unwrap();
        std::fs::remove_dir_all(&lab.root(&root).path).unwrap();
        let db = lab.db.clone();
        drop(lab.store);
        let store = koine_sqlite::Store::open(&db).unwrap();
        assert_eq!(journal.recover(&store).unwrap(), 1);
        assert_eq!(journal.recover(&store).unwrap(), 0);
        let s = store.snapshot().unwrap();
        assert_eq!(s.rows("evidence_records").count(), 2);
        assert!(
            s.rows("evidence_records")
                .all(|e| e.int("collected_at").unwrap() == 10)
        );
        assert!(
            s.rows("collection_closures")
                .all(|c| c.is("outcome", "complete"))
        );
    }
    #[test]
    fn corrupt_journal_blocks_recovery_and_preserves_pending_evidence() {
        let lab = crate::test_support::Lab::new();
        let root = lab.add("root");
        let report = lab.report(&root);
        let journal = Journal::open(&lab.db).unwrap();
        journal.save(&report).unwrap();
        let path = journal.path(&report.plan.delivery).unwrap();
        let mut bytes = std::fs::read(&path).unwrap();
        let n = bytes.len() - 1;
        bytes[n] ^= 1;
        std::fs::write(&path, bytes).unwrap();
        assert!(
            journal
                .recover(&lab.store)
                .unwrap_err()
                .contains("fingerprint")
        );
        assert!(path.exists());
        assert_eq!(
            lab.store
                .snapshot()
                .unwrap()
                .rows("collection_closures")
                .count(),
            0
        );
    }
    #[test]
    fn journal_cleanup_after_known_commit_does_not_duplicate_delivery() {
        let lab = crate::test_support::Lab::new();
        let root = lab.add("root");
        let report = lab.report(&root);
        let journal = Journal::open(&lab.db).unwrap();
        journal.save(&report).unwrap();
        integration::apply(&lab.store, &report).unwrap();
        let before = lab.store.snapshot().unwrap();
        assert_eq!(journal.recover(&lab.store).unwrap(), 1);
        assert_eq!(before, lab.store.snapshot().unwrap());
    }
    #[cfg(feature = "fault-injection")]
    #[test]
    fn actual_sqlite_full_during_discovery_recovers_from_journal_after_reopen() {
        use koine_sqlite::{Cancellation, Inspection, WriteState};
        let lab = crate::test_support::Lab::fault();
        let root = lab.add("root");
        let path = lab.root(&root).path.join("steamapps/appmanifest_41.acf");
        std::fs::write(path,format!("\"AppState\" {{\"appid\" \"41\" \"installdir\" \"Synthetic\" \"name\" \"{}\" \"StateFlags\" \"4\"}}","S".repeat(500_000))).unwrap();
        let report = lab.report(&root);
        let journal = Journal::open(&lab.db).unwrap();
        journal.save(&report).unwrap();
        let rows = integration::application(&lab.store.snapshot().unwrap(), &report).unwrap();
        let current = lab.store.fault_limit_pages(1).unwrap();
        let outcome = lab
            .store
            .try_submit(rows.clone(), Cancellation::default())
            .unwrap()
            .wait();
        assert_eq!(outcome.state, WriteState::RolledBack);
        assert_eq!(outcome.sqlite_code, Some(13)); // SQLite FULL; no VFS or hardware failure is claimed
        assert_eq!(
            lab.store.inspect_operation(&rows).unwrap(),
            Inspection::Absent
        );
        lab.store.fault_limit_pages(current + 4096).unwrap();
        let db = lab.db.clone();
        drop(lab.store);
        let store = koine_sqlite::Store::open(db).unwrap();
        assert_eq!(journal.recover(&store).unwrap(), 1);
        assert_eq!(
            store.snapshot().unwrap().rows("evidence_records").count(),
            2
        );
    }
    #[cfg(feature = "fault-injection")]
    #[test]
    fn lost_discovery_commit_ack_replays_same_immutable_identity() {
        use koine_sqlite::{
            Cancellation, WriteState,
            faults::{Effect, FaultPlan, FaultPoint},
        };
        let lab = crate::test_support::Lab::fault();
        let root = lab.add("root");
        let report = lab.report(&root);
        let journal = Journal::open(&lab.db).unwrap();
        journal.save(&report).unwrap();
        let rows = integration::application(&lab.store.snapshot().unwrap(), &report).unwrap();
        let outcome = lab
            .store
            .fault_submit(
                rows,
                Cancellation::default(),
                FaultPlan::new(FaultPoint::AfterCommit, Effect::LoseCommitAck),
            )
            .unwrap()
            .wait();
        assert_eq!(outcome.state, WriteState::CommitUncertain);
        let before = lab.store.snapshot().unwrap();
        assert_eq!(journal.recover(&lab.store).unwrap(), 1);
        assert_eq!(before, lab.store.snapshot().unwrap());
    }
}

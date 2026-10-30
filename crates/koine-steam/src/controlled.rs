//! Explicit configured-library v2. No default-path discovery or automatic expansion.
use crate::{
    config::{Configuration, Root},
    *,
};
use std::{collections::BTreeSet, sync::Mutex};
pub fn plan(owner: &str, root: &Root, delivery: String, at: i64) -> Result<Plan> {
    config::uuid(&delivery)?;
    Ok(Plan {
        profile: if root.synthetic() {
            Profile::SyntheticV2
        } else {
            Profile::RealV2
        },
        boundary: Some(fixture_fs::bytes(root.path())),
        physical_binding: None,
        delivery,
        fixture: owner.into(),
        at,
        roots: vec![LibraryRoot {
            key: root.scope().into(),
            relative: Some(PathBuf::new()),
            issue: None,
        }],
        index_revision: None,
        index_issues: Vec::new(),
    })
}
/// Diagnostic for an attempt whose root changed between admission and its start.
pub(crate) const CHANGED_AFTER_ADMISSION: &str = "configuration-changed-after-admission";
/// Diagnostic for an attempt whose root changed after it started and before its
/// report was accepted into the journal.
pub(crate) const CHANGED_DURING_DISCOVERY: &str = "configuration-changed-during-discovery";
/// Whether `root` is still configured exactly as admitted: the same id, scope
/// generation, path, layout, mode and authorization. Removal, reconfiguration and
/// authorization changes all fail this check; recording a physical binding does not.
pub(crate) fn still_configured(configured: &Configuration, root: &Root) -> bool {
    configured.roots().iter().any(|r| {
        r.id() == root.id()
            && r.scope() == root.scope()
            && r.path() == root.path()
            && r.layout() == root.layout()
            && r.synthetic() == root.synthetic()
            && r.authorized() == root.authorized()
    })
}
fn configured(settings: &Mutex<Configuration>, root: &Root) -> Result<bool> {
    let configured = settings.lock().map_err(|_| "settings lock poisoned")?;
    Ok(still_configured(&configured, root))
}
/// The report accepted for an attempt, decided with the settings lock held until
/// its journal write. An attempt whose root is no longer configured as admitted
/// discards everything it read and closes with the admitted plan only. Reports
/// that are already configuration refusals are kept as they are.
pub(crate) fn acceptable(
    configured: &Configuration,
    root: &Root,
    admitted: &Plan,
    report: Report,
) -> Report {
    if still_configured(configured, root)
        || report
            .index_issues
            .iter()
            .any(|c| c == CHANGED_AFTER_ADMISSION || c == CHANGED_DURING_DISCOVERY)
    {
        report
    } else {
        gap(admitted.clone(), CHANGED_DURING_DISCOVERY.into())
    }
}
fn gap(plan: Plan, code: String) -> Report {
    Report {
        roots: plan
            .roots
            .iter()
            .map(|root| RootReport {
                root: root.clone(),
                observations: vec![],
                diagnostics: vec![Diagnostic {
                    code: code.clone(),
                    dimension: None,
                    locator: vec![],
                }],
                inspected: false,
            })
            .collect(),
        index_issues: vec![code],
        plan,
    }
}
pub fn scan(
    settings: &Mutex<Configuration>,
    root: &Root,
    mut plan: Plan,
    cancel: Arc<AtomicBool>,
    seen: &mut BTreeSet<String>,
    progress: &mut dyn FnMut(usize),
) -> Result<Report> {
    if !plan.profile.controlled()
        || plan.roots.len() != 1
        || plan.roots[0].key != root.scope()
        || plan.boundary.as_ref() != Some(&fixture_fs::bytes(root.path()))
    {
        return Err("configured request mismatch".into());
    }
    {
        let configured = settings.lock().map_err(|_| "settings lock poisoned")?;
        if configured.owner() != plan.fixture
            || plan.profile
                != if root.synthetic() {
                    Profile::SyntheticV2
                } else {
                    Profile::RealV2
                }
        {
            return Err("request is not the authorized configuration snapshot".into());
        }
        // The root was removed, reconfigured or had its authorization changed after
        // admission. Close only this attempt, before any source access, instead of
        // failing the whole request.
        if !still_configured(&configured, root) {
            return Ok(gap(plan, CHANGED_AFTER_ADMISSION.into()));
        }
    }
    // A later change discards what this attempt has read. Opening the root follows
    // the check above; every later source access repeats it, and a failed check
    // closes the attempt with the plan as admitted.
    let admitted = plan.clone();
    let revoked =
        || -> Result<Report> { Ok(gap(admitted.clone(), CHANGED_DURING_DISCOVERY.into())) };
    if cancel.load(Ordering::Relaxed) {
        return Ok(gap(plan, "cancelled".into()));
    }
    if !root.synthetic() && !root.authorized() {
        return Ok(gap(plan, "explicit-root-authorization-required".into()));
    }
    if !["native", "flatpak"].contains(&root.layout()) {
        return Ok(gap(plan, "unsupported-package-layout".into()));
    }
    let opened = if root.synthetic() {
        FixtureFs::open(root.path()).map(|mut fs| {
            fs.confined = true;
            fs.id = plan.fixture.clone();
            fs
        })
    } else {
        FixtureFs::boundary(root.path(), plan.fixture.clone())
    };
    let fs = match opened {
        Ok(fs) => fs,
        Err(e) => return Ok(gap(plan, e)),
    };
    let identity = fs.identity()?;
    #[cfg(test)]
    tests::before_check(Check::Bind);
    {
        let mut configured = settings.lock().map_err(|_| "settings lock poisoned")?;
        if !still_configured(&configured, root) {
            return revoked();
        }
        if let Err(e) = configured.bind(root.scope(), &identity) {
            return Ok(gap(plan, e));
        }
    }
    // A batch has one descriptor-backed scan per physical root; aliases stay visible gaps.
    if !seen.insert(identity.clone()) {
        return Ok(gap(plan, "duplicate-physical-scope".into()));
    }
    plan.physical_binding = Some(canonical::hash(identity.as_bytes()));
    let index = Path::new("steamapps/libraryfolders.vdf");
    #[cfg(test)]
    tests::before_check(Check::Index);
    if !configured(settings, root)? {
        return revoked();
    }
    let bytes = fs.read(index);
    plan.index_revision = bytes.as_ref().ok().map(|b| canonical::hash(b));
    match bytes.and_then(|b| keyvalues::parse(&b)).and_then(|kv| {
        if kv.object()?.len() != 1 {
            return Err("unsupported-library-index".into());
        }
        kv.get("libraryfolders")?.object().cloned()
    }) {
        Err(code) => plan.index_issues.push(code),
        Ok(fields) => {
            for (key, value) in fields {
                if key.is_empty() || !key.bytes().all(|b| b.is_ascii_digit()) {
                    continue;
                }
                match value
                    .get("path")
                    .and_then(|v| v.text())
                    .and_then(|v| config::normalize(Path::new(v)))
                {
                    Ok(path) if path == root.path() => (),
                    Ok(_) => plan
                        .index_issues
                        .push("referenced-library-not-inspected-needs-separate-scope".into()),
                    Err(_) => plan
                        .index_issues
                        .push("unsupported-or-unsafe-library-reference".into()),
                }
            }
        }
    }
    let Some(mut report) = fs.scan_while(plan, cancel, progress, &mut || {
        #[cfg(test)]
        tests::before_check(Check::Scan);
        configured(settings, root)
    })?
    else {
        return revoked();
    };
    #[cfg(test)]
    tests::before_check(Check::ClosingIndex);
    if !configured(settings, root)? {
        return revoked();
    }
    if fs.read(index).ok().map(|b| canonical::hash(&b)) != report.plan.index_revision {
        report.index_issues.push("library-index-changed".into());
    }
    #[cfg(test)]
    tests::before_check(Check::Reopen);
    if !configured(settings, root)? {
        return revoked();
    }
    // Reopen only the same authorized path without following links; a replaced/missing
    // mount cannot turn a descriptor to an old detached directory into full coverage.
    if FixtureFs::boundary(root.path(), report.plan.fixture.clone())
        .and_then(|f| f.identity())
        .ok()
        .as_ref()
        != Some(&identity)
    {
        for r in &mut report.roots {
            r.diagnostics.push(Diagnostic {
                code: "root-changed-or-unavailable".into(),
                dimension: None,
                locator: vec![],
            });
        }
    }
    Ok(report)
}
/// Test-only names for the configuration checks made after the attempt starts.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Check {
    Bind,
    Index,
    Scan,
    ClosingIndex,
    Reopen,
}
pub(crate) fn definitive_failure(code: &str) -> bool {
    matches!(
        code,
        "unreadable-path" | "inaccessible-path" | "read-failed" | "enumeration-failed"
    )
}
impl Report {
    pub fn outcome(&self) -> &'static str {
        let usable = self.roots.iter().any(|r| !r.observations.is_empty());
        let complete = self.index_issues.is_empty()
            && self.plan.index_revision.is_some()
            && self
                .roots
                .iter()
                .all(|r| r.inspected && r.diagnostics.is_empty());
        if complete {
            "complete"
        } else if usable {
            "partial"
        } else if self.plan.profile.controlled()
            && self
                .roots
                .iter()
                .flat_map(|r| &r.diagnostics)
                .any(|d| definitive_failure(&d.code))
        {
            "failed"
        } else {
            "inconclusive"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Lab;
    use std::cell::RefCell;
    type Hook = Box<dyn FnMut(Check)>;
    thread_local! {
        static HOOK: RefCell<Option<Hook>> = RefCell::new(None);
    }
    /// Runs this thread's hook, if any, immediately before a configuration check.
    pub(super) fn before_check(at: Check) {
        HOOK.with(|h| {
            if let Some(hook) = h.borrow_mut().as_mut() {
                hook(at)
            }
        });
    }
    #[derive(Clone, Copy, Debug)]
    enum Change {
        Remove,
        Reconfigure,
        Authorize,
    }
    /// Changes the root's configuration immediately before the check `at`, then
    /// asserts the attempt is discarded there: no later check (so no later source
    /// access) runs, only `reads` manifests were read, and nothing is committed.
    fn revoke_at(at: Check, change: Change, real: bool, reads: usize) {
        let lab = Lab::new();
        let rid = if real {
            lab.add_authorized_real("root")
        } else {
            lab.add("root")
        };
        let root = lab.root(&rid);
        std::fs::write(
            root.path().join("steamapps/appmanifest_43.acf"),
            "\"AppState\" {\"appid\" \"43\" \"installdir\" \"Synthetic 43\" \"name\" \"Synthetic 43\" \"StateFlags\" \"4\"}",
        )
        .unwrap();
        let plan = plan(lab.settings.lock().unwrap().owner(), &root, id(), 10).unwrap();
        integration::register(&lab.store, &plan).unwrap();
        let seen = std::rc::Rc::new(RefCell::new(Vec::new()));
        let (settings, record, target) = (lab.settings.clone(), seen.clone(), root.clone());
        HOOK.with(|h| {
            *h.borrow_mut() = Some(Box::new(move |check| {
                record.borrow_mut().push(check);
                if check != at {
                    return;
                }
                let mut c = settings.lock().unwrap();
                match change {
                    Change::Remove => c.remove(target.id()).unwrap(),
                    Change::Reconfigure => c
                        .set(target.id(), target.path(), "flatpak", target.synthetic())
                        .unwrap(),
                    // An in-place change: same scope generation, different approval.
                    Change::Authorize => c.authorize(target.id(), target.path()).unwrap(),
                }
            }))
        });
        let mut progress = 0;
        let report = scan(
            &lab.settings,
            &root,
            plan.clone(),
            Arc::new(AtomicBool::new(false)),
            &mut BTreeSet::new(),
            &mut |_| progress += 1,
        );
        HOOK.with(|h| *h.borrow_mut() = None);
        let report = report.unwrap();
        let context = format!("{at:?} {change:?} real={real}");
        assert_eq!(
            seen.borrow().last(),
            Some(&at),
            "{context}: {:?}",
            seen.borrow()
        );
        assert_eq!(
            seen.borrow().iter().filter(|c| **c == at).count(),
            1,
            "{context}"
        );
        assert_eq!(progress, reads, "{context}");
        assert_eq!(report.plan, plan, "{context}");
        assert_eq!(report.index_issues, [CHANGED_DURING_DISCOVERY], "{context}");
        assert_eq!(report.outcome(), "inconclusive", "{context}");
        assert!(
            report
                .roots
                .iter()
                .all(|r| r.observations.is_empty() && !r.inspected),
            "{context}"
        );
        if matches!(at, Check::Bind) {
            // A root that lost its configuration before binding is never bound.
            assert!(
                lab.settings
                    .lock()
                    .unwrap()
                    .roots()
                    .iter()
                    .all(|r| r.binding().is_none()),
                "{context}"
            );
        }
        integration::apply(&lab.store, &report).unwrap();
        let s = lab.store.snapshot().unwrap();
        assert_eq!(s.rows("evidence_records").count(), 0, "{context}");
        assert_eq!(s.rows("source_entries").count(), 0, "{context}");
    }
    #[test]
    fn a_change_before_any_check_after_the_start_discards_the_attempt_there() {
        for (at, reads) in [
            (Check::Bind, 0),
            (Check::Index, 0),
            (Check::ClosingIndex, 3),
            (Check::Reopen, 3),
        ] {
            for change in [Change::Remove, Change::Reconfigure] {
                for real in [false, true] {
                    revoke_at(at, change, real, reads);
                }
            }
            // Authorizing a synthetic root keeps its scope but changes its approval.
            revoke_at(at, Change::Authorize, false, reads);
        }
    }
}

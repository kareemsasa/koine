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
    let Some(mut report) =
        fs.scan_while(plan, cancel, progress, &mut || configured(settings, root))?
    else {
        return revoked();
    };
    if !configured(settings, root)? {
        return revoked();
    }
    if fs.read(index).ok().map(|b| canonical::hash(&b)) != report.plan.index_revision {
        report.index_issues.push("library-index-changed".into());
    }
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

//! Versioned read-only Steam discovery within explicit configured or marked fixture roots.
pub mod config;
pub mod controlled;
#[cfg(feature = "fault-injection")]
pub mod faults;
mod fixture_fs;
pub mod integration;
mod journal;
pub mod keyvalues;
pub mod scheduler;
pub use fixture_fs::FixtureFs;
use koine_core::{canonical, *};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: String,
    pub dimension: Option<String>,
    pub locator: Vec<u8>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryRoot {
    pub key: String,
    pub relative: Option<PathBuf>,
    pub issue: Option<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    FixtureV1,
    SyntheticV2,
    RealV2,
}
impl Profile {
    pub fn controlled(self) -> bool {
        self != Self::FixtureV1
    }
    pub fn reporter(self) -> &'static str {
        match self {
            Self::FixtureV1 => "koine.steam.fixture",
            Self::SyntheticV2 => "koine.steam.synthetic",
            Self::RealV2 => "koine.steam.configured",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub profile: Profile,
    pub boundary: Option<Vec<u8>>,
    pub physical_binding: Option<Vec<u8>>,
    pub delivery: String,
    pub fixture: String,
    pub at: i64,
    pub roots: Vec<LibraryRoot>,
    pub index_revision: Option<Vec<u8>>,
    pub index_issues: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observation {
    pub app_id: u32,
    pub reported_id: String,
    pub name: Option<String>,
    pub install_dir: String,
    pub completion: Option<String>,
    pub presence: bool,
    pub flags: Option<u32>,
    pub locator: Vec<u8>,
    pub revision: Vec<u8>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RootReport {
    pub root: LibraryRoot,
    pub observations: Vec<Observation>,
    pub diagnostics: Vec<Diagnostic>,
    pub inspected: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    pub plan: Plan,
    pub roots: Vec<RootReport>,
    pub index_issues: Vec<String>,
}
fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
pub fn app_id(text: &str) -> Result<u32> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err("invalid-appid".into());
    }
    let id: u32 = text.parse().map_err(|_| "invalid-appid")?;
    if id == 0 {
        return Err("invalid-appid".into());
    }
    Ok(id)
}
pub fn completion(flags: Option<u32>) -> Option<&'static str> {
    let flags = flags?;
    if flags == 4 {
        return Some("complete");
    }
    let known = 1
        | 2
        | 4
        | 8
        | 16
        | 32
        | 64
        | 128
        | 256
        | 512
        | 1024
        | 2048
        | 4096
        | 65536
        | 131072
        | 262144
        | 524288
        | 1048576
        | 2097152
        | 4194304
        | 8388608;
    let incomplete =
        32 | 128 | 256 | 512 | 1024 | 262144 | 524288 | 1048576 | 2097152 | 4194304 | 8388608;
    if flags & !known == 0 && flags & 4 == 0 && flags & incomplete != 0 && flags & (1 | 2048) == 0 {
        Some("incomplete")
    } else {
        None
    }
}
pub fn parse_manifest(name: &str, bytes: &[u8], locator: Vec<u8>) -> Result<Observation> {
    let file_id = name
        .strip_prefix("appmanifest_")
        .and_then(|s| s.strip_suffix(".acf"))
        .ok_or("unsupported-manifest-name")?;
    let file_id = app_id(file_id)?;
    let kv = keyvalues::parse(bytes)?;
    let top = kv.object()?;
    if top.len() != 1 {
        return Err("unsupported-manifest-root".into());
    }
    let state = kv.get("appstate")?;
    let reported = state.get("appid")?.text()?;
    let id = app_id(reported)?;
    if id != file_id {
        return Err("appid-mismatch".into());
    }
    let dir = state.get("installdir")?.text()?;
    if dir.is_empty()
        || dir == "."
        || dir == ".."
        || dir.len() > 255
        || dir.contains(['/', '\\', '\0'])
    {
        return Err("unsafe-install-dir".into());
    }
    let name = state
        .object()?
        .get("name")
        .and_then(|v| v.text().ok())
        .filter(|s| !s.is_empty())
        .map(String::from);
    let flags = state
        .object()?
        .get("stateflags")
        .and_then(|v| v.text().ok())
        .and_then(|s| s.parse::<u32>().ok());
    Ok(Observation {
        app_id: id,
        reported_id: reported.into(),
        name,
        install_dir: dir.into(),
        completion: completion(flags).map(String::from),
        presence: !flags.is_some_and(|f| f == 0 || f & 1 != 0 || f & 2048 != 0),
        flags,
        locator,
        revision: canonical::hash(bytes),
    })
}
impl FixtureFs {
    pub fn plan(&self, delivery: String, at: i64) -> Result<Plan> {
        let mut roots = BTreeMap::new();
        let key = |rel: &Path| format!("{}:{}", self.id, hex(&fixture_fs::bytes(rel)));
        roots.insert(
            key(Path::new("")),
            LibraryRoot {
                key: key(Path::new("")),
                relative: Some(PathBuf::new()),
                issue: None,
            },
        );
        let mut issues = Vec::new();
        let index = self.read(Path::new("steamapps/libraryfolders.vdf"));
        let revision = index.as_ref().ok().map(|b| canonical::hash(b));
        match index
            .and_then(|b| keyvalues::parse(&b))
            .and_then(|v| v.get("libraryfolders").and_then(|v| v.object()).cloned())
        {
            Ok(fields) => {
                for (name, value) in fields {
                    if name.is_empty() || !name.bytes().all(|b| b.is_ascii_digit()) {
                        continue;
                    }
                    let parsed = value
                        .get("path")
                        .and_then(|v| v.text())
                        .and_then(|p| self.library_path(p));
                    match parsed {
                        Ok(relative) => {
                            let k = key(&relative);
                            roots.insert(
                                k.clone(),
                                LibraryRoot {
                                    key: k,
                                    relative: Some(relative),
                                    issue: None,
                                },
                            );
                        }
                        Err(reason) => {
                            issues.push(reason.clone());
                            let k = format!("{}:rejected-{name}", self.id);
                            roots.insert(
                                k.clone(),
                                LibraryRoot {
                                    key: k,
                                    relative: None,
                                    issue: Some(reason),
                                },
                            );
                        }
                    }
                }
            }
            Err(reason) => issues.push(reason),
        }
        Ok(Plan {
            profile: Profile::FixtureV1,
            boundary: None,
            physical_binding: None,
            delivery,
            fixture: self.id.clone(),
            at,
            roots: roots.into_values().collect(),
            index_revision: revision,
            index_issues: issues,
        })
    }
    pub fn scan(&self, plan: Plan, cancel: Arc<AtomicBool>) -> Result<Report> {
        self.scan_progress(plan, cancel, &mut |_| {})
    }
    pub fn scan_progress(
        &self,
        plan: Plan,
        cancel: Arc<AtomicBool>,
        progress: &mut dyn FnMut(usize),
    ) -> Result<Report> {
        if plan.fixture != self.id {
            return Err("fixture identity changed".into());
        }
        let mut reports = Vec::new();
        let mut remaining_bytes = 16 * 1024 * 1024usize;
        for root in &plan.roots {
            let mut report = RootReport {
                root: root.clone(),
                observations: Vec::new(),
                diagnostics: Vec::new(),
                inspected: false,
            };
            let mut gap = |code: &str, dimension: Option<&str>, locator: Vec<u8>| {
                report.diagnostics.push(Diagnostic {
                    code: code.into(),
                    dimension: dimension.map(String::from),
                    locator,
                })
            };
            if cancel.load(Ordering::Relaxed) {
                gap("cancelled", None, Vec::new());
                reports.push(report);
                continue;
            }
            let Some(relative) = &root.relative else {
                gap(
                    root.issue.as_deref().unwrap_or("unavailable-root"),
                    None,
                    Vec::new(),
                );
                reports.push(report);
                continue;
            };
            let steamapps = relative.join("steamapps");
            let listing = match self.list(&steamapps) {
                Ok(list) => list,
                Err(reason) => {
                    gap(&reason, None, fixture_fs::bytes(&steamapps));
                    reports.push(report);
                    continue;
                }
            };
            report.inspected = true;
            for file in &listing {
                if remaining_bytes == 0 {
                    report.diagnostics.push(Diagnostic {
                        code: "scan-byte-budget-exhausted".into(),
                        dimension: None,
                        locator: fixture_fs::bytes(&steamapps),
                    });
                    break;
                }
                if cancel.load(Ordering::Relaxed) {
                    report.diagnostics.push(Diagnostic {
                        code: "cancelled".into(),
                        dimension: None,
                        locator: fixture_fs::bytes(&steamapps),
                    });
                    break;
                }
                let Some(name) = file.to_str() else {
                    if file.as_encoded_bytes().starts_with(b"appmanifest_")
                        && file.as_encoded_bytes().ends_with(b".acf")
                    {
                        report.diagnostics.push(Diagnostic {
                            code: "unsupported-manifest-name".into(),
                            dimension: None,
                            locator: fixture_fs::bytes(&steamapps.join(file)),
                        })
                    }
                    continue;
                };
                if !name.starts_with("appmanifest_") || !name.ends_with(".acf") {
                    continue;
                }
                let locator = fixture_fs::bytes(&steamapps.join(file));
                match self.read(&steamapps.join(file)).and_then(|b| {
                    if b.len() > remaining_bytes {
                        remaining_bytes = 0;
                        return Err("scan-byte-budget-exhausted".into());
                    }
                    remaining_bytes -= b.len();
                    parse_manifest(name, &b, locator.clone())
                }) {
                    Ok(observation) => {
                        if observation.completion.is_none() {
                            report.diagnostics.push(Diagnostic {
                                code: "completion-unknown".into(),
                                dimension: Some("completion".into()),
                                locator: locator.clone(),
                            })
                        }
                        if !observation.presence {
                            report.diagnostics.push(Diagnostic {
                                code: "presence-unknown".into(),
                                dimension: Some("presence".into()),
                                locator: locator.clone(),
                            })
                        }
                        if observation.name.is_none() {
                            report.diagnostics.push(Diagnostic {
                                code: "title-unknown".into(),
                                dimension: Some("entry_title".into()),
                                locator: locator.clone(),
                            })
                        }
                        report.observations.push(observation);
                        progress(report.observations.len())
                    }
                    Err(reason) => report.diagnostics.push(Diagnostic {
                        code: reason,
                        dimension: None,
                        locator,
                    }),
                }
            }
            if self.list(&steamapps).ok().as_ref() != Some(&listing) {
                report.diagnostics.push(Diagnostic {
                    code: "directory-changed".into(),
                    dimension: None,
                    locator: fixture_fs::bytes(&steamapps),
                })
            }
            reports.push(report)
        }
        let mut index_issues = plan.index_issues.clone();
        if !plan.profile.controlled()
            && self
                .read(Path::new("steamapps/libraryfolders.vdf"))
                .ok()
                .map(|b| canonical::hash(&b))
                != plan.index_revision
        {
            index_issues.push("library-index-changed".into())
        }
        Ok(Report {
            plan,
            roots: reports,
            index_issues,
        })
    }
}
impl Report {
    fn validate(&self) -> Result<()> {
        config::uuid(&self.plan.delivery)?;
        if self.plan.profile.controlled()
            && (self.plan.roots.len() != 1
                || self.plan.roots[0].relative.as_deref() != Some(Path::new(""))
                || self.plan.roots[0].issue.is_some())
        {
            return Err("invalid configured report scope".into());
        }
        if self.plan.profile.controlled() {
            let boundary = fixture_fs::from_bytes(
                self.plan
                    .boundary
                    .clone()
                    .ok_or("configured boundary missing")?,
            );
            if config::normalize(&boundary)? != boundary {
                return Err("invalid configured boundary".into());
            }
        }
        if self.plan.roots.is_empty()
            || self
                .plan
                .index_revision
                .as_ref()
                .is_some_and(|r| r.len() != 32)
        {
            return Err("invalid report request".into());
        }
        for r in &self.plan.roots {
            if r.relative
                .as_ref()
                .is_some_and(|p| !fixture_fs::safe_relative(p))
            {
                return Err("unsafe report root".into());
            }
        }

        for root in &self.roots {
            let mut seen = std::collections::BTreeSet::new();
            for o in &root.observations {
                if self.plan.profile.controlled()
                    && self
                        .plan
                        .physical_binding
                        .as_ref()
                        .is_none_or(|b| b.len() != 32)
                {
                    return Err("configured observation lacks physical binding".into());
                }
                let locator = fixture_fs::from_bytes(o.locator.clone());
                let parent = root
                    .root
                    .relative
                    .as_ref()
                    .ok_or("observation in rejected root")?
                    .join("steamapps");
                let filename = locator
                    .file_name()
                    .and_then(|n| n.to_str())
                    .ok_or("invalid observation locator")?;
                let file_id = filename
                    .strip_prefix("appmanifest_")
                    .and_then(|n| n.strip_suffix(".acf"))
                    .ok_or("invalid observation locator")?;
                if !root.inspected
                    || locator.parent() != Some(parent.as_path())
                    || !seen.insert(o.locator.clone())
                    || app_id(file_id)? != o.app_id
                    || app_id(&o.reported_id)? != o.app_id
                    || o.revision.len() != 32
                    || o.completion.as_deref() != completion(o.flags)
                    || o.presence != !o.flags.is_some_and(|f| f == 0 || f & (1 | 2048) != 0)
                    || o.install_dir.is_empty()
                    || o.install_dir.len() > 255
                    || matches!(o.install_dir.as_str(), "." | "..")
                    || o.install_dir.contains(['/', '\\', '\0'])
                {
                    return Err("inconsistent versioned Steam observation".into());
                }
            }
        }
        Ok(())
    }
    pub fn fingerprint(&self) -> Vec<u8> {
        let roots = self
            .roots
            .iter()
            .map(|r| {
                Value::List(vec![
                    r.root.key.clone().into(),
                    Value::Integer(i64::from(r.inspected)),
                    Value::List(
                        r.observations
                            .iter()
                            .map(|o| {
                                Value::List(vec![
                                    i64::from(o.app_id).into(),
                                    o.reported_id.clone().into(),
                                    o.flags.map_or(Value::Null, |f| i64::from(f).into()),
                                    o.name.clone().map_or(Value::Null, Value::Text),
                                    o.install_dir.clone().into(),
                                    o.completion.clone().map_or(Value::Null, Value::Text),
                                    Value::Integer(i64::from(o.presence)),
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
            .collect();
        canonical::hash(&canonical::frame(
            if self.plan.profile.controlled() {
                self.plan.profile.reporter()
            } else {
                "koine.steam.report.v1"
            },
            &Value::List(vec![
                self.plan.delivery.clone().into(),
                if self.plan.profile.controlled() {
                    Value::List(vec![
                        self.plan.fixture.clone().into(),
                        self.plan.boundary.clone().map_or(Value::Null, Value::Blob),
                        self.plan
                            .physical_binding
                            .clone()
                            .map_or(Value::Null, Value::Blob),
                    ])
                } else {
                    self.plan.fixture.clone().into()
                },
                self.plan
                    .index_revision
                    .clone()
                    .map_or(Value::Null, Value::Blob),
                Value::List(
                    self.plan
                        .roots
                        .iter()
                        .map(|r| {
                            Value::List(vec![
                                r.key.clone().into(),
                                r.relative
                                    .as_ref()
                                    .map_or(Value::Null, |p| fixture_fs::bytes(p).into()),
                                r.issue.clone().map_or(Value::Null, Value::Text),
                            ])
                        })
                        .collect(),
                ),
                Value::List(
                    self.plan
                        .index_issues
                        .iter()
                        .cloned()
                        .map(Value::Text)
                        .collect(),
                ),
                self.plan.at.into(),
                Value::List(roots),
                Value::List(self.index_issues.iter().cloned().map(Value::Text).collect()),
            ]),
        ))
    }
}

#[cfg(test)]
mod test_support;

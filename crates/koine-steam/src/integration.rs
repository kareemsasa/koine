//! Pure record planning executed by the existing serialized SQLite writer.
use super::*;
use koine_core::rules;
use koine_sqlite::Store;
const DIMS: [&str; 4] = ["entry_title", "presence", "completion", "location"];
fn row(table: &str, fields: impl IntoIterator<Item = (&'static str, Value)>) -> Record {
    Record::new(table, fields)
}
fn lookup<'a>(s: &'a Snapshot, table: &str, fields: &[(&str, Value)]) -> Option<&'a Record> {
    s.rows(table)
        .find(|r| fields.iter().all(|(k, v)| r.get(k) == v))
}
fn ensure(
    s: &Snapshot,
    out: &mut Vec<Record>,
    table: &str,
    keys: &[&str],
    mut fields: Vec<(&'static str, Value)>,
) -> Result<String> {
    let matched = |r: &&Record| {
        r.table == table
            && keys.iter().all(|k| {
                fields
                    .iter()
                    .find(|(f, _)| f == k)
                    .is_some_and(|(_, v)| r.get(k) == v)
            })
    };
    if let Some(r) = s.records.iter().chain(out.iter()).find(matched) {
        if fields
            .iter()
            .any(|(k, v)| *k != "created_evidence_id" && r.get(k) != v)
        {
            return Err(format!("inconsistent immutable {table}"));
        }
        return r.text("id").map(String::from);
    }
    let rid = id();
    fields.push(("id", rid.clone().into()));
    out.push(row(table, fields));
    Ok(rid)
}
fn rule(
    s: &Snapshot,
    out: &mut Vec<Record>,
    name: &'static str,
    kind: &'static str,
) -> Result<String> {
    ensure(
        s,
        out,
        "rule_versions",
        &["kind", "name", "version"],
        vec![
            ("kind", kind.into()),
            ("name", name.into()),
            ("version", rules::version(name).into()),
            (
                "artifact_digest",
                rules::artifact(name, kind)
                    .ok_or("unsupported rule")?
                    .into(),
            ),
        ],
    )
}
fn context(
    s: &Snapshot,
    out: &mut Vec<Record>,
    kind: &'static str,
    ns: &'static str,
    key: &str,
) -> Result<String> {
    ensure(
        s,
        out,
        "contexts",
        &["kind", "namespace", "external_key"],
        vec![
            ("kind", kind.into()),
            ("namespace", ns.into()),
            ("resolution", "identified".into()),
            ("external_key", key.into()),
        ],
    )
}
fn index_key(plan: &Plan) -> String {
    if plan.profile.controlled() {
        return format!("{}|index", plan.roots[0].key);
    }
    format!(
        "{}|index|{}",
        plan.fixture,
        plan.index_revision
            .as_ref()
            .map_or_else(|| "unavailable".into(), |b| hex(b))
    )
}
fn selector(relative: &Path, item: &str) -> Value {
    canonical::encode(&Value::List(vec![
        "path".into(),
        fixture_fs::bytes(relative).into(),
        item.into(),
    ]))
    .into()
}
fn metadata(s: &Snapshot, out: &mut Vec<Record>, plan: &Plan) -> Result<(String, String, String)> {
    let interpretation = rule(
        s,
        out,
        if plan.profile.controlled() {
            rules::STEAM_CONTROLLED
        } else {
            rules::STEAM_INTERPRETATION
        },
        "interpretation",
    )?;
    let identity = rule(s, out, rules::STEAM_IDENTITY, "identity")?;
    rule(s, out, rules::RECONCILIATION, "reconciliation")?;
    rule(s, out, rules::FRESHNESS, "freshness")?;
    let manager = context(
        s,
        out,
        "manager",
        if plan.profile.controlled() {
            plan.profile.reporter()
        } else {
            "steam:fixture"
        },
        &plan.fixture,
    )?;
    let machine = context(
        s,
        out,
        "machine",
        if plan.profile.controlled() {
            plan.profile.reporter()
        } else {
            "koine:fixture"
        },
        &plan.fixture,
    )?;
    ensure(
        s,
        out,
        "identity_scopes",
        &["namespace", "scope_kind", "resolution"],
        vec![
            ("namespace", "steam:app".into()),
            ("scope_kind", "global".into()),
            ("resolution", "resolved".into()),
        ],
    )?;
    ensure(
        s,
        out,
        "identity_profiles",
        &["namespace", "version"],
        vec![
            ("namespace", "steam:app".into()),
            ("version", "1".into()),
            ("scope_kind", "global".into()),
            ("key_role", "catalog".into()),
            ("catalog_namespace", "steam".into()),
            ("rule_id", identity.into()),
        ],
    )?;
    Ok((interpretation, manager, machine))
}
fn registration(s: &Snapshot, plan: &Plan) -> Result<Vec<Record>> {
    let mut out = Vec::new();
    let (interp, manager, machine) = metadata(s, &mut out, plan)?;
    let existing = s.by_id("collections", &plan.delivery.clone().into()).ok();
    out.push(row(
        "collections",
        [
            ("id", plan.delivery.clone().into()),
            ("kind", "discovery".into()),
            ("reporter", plan.profile.reporter().into()),
            (
                "reporter_version",
                if plan.profile.controlled() { "2" } else { "1" }.into(),
            ),
            ("reporter_context_id", manager.clone().into()),
            ("interpretation_rule_id", interp.clone().into()),
            ("started_at", plan.at.into()),
            ("request_protocol", "koine.request.v1".into()),
        ],
    ));
    let roots = plan
        .roots
        .iter()
        .cloned()
        .chain(std::iter::once(LibraryRoot {
            key: index_key(plan),
            relative: Some(PathBuf::from("steamapps/libraryfolders.vdf")),
            issue: None,
        }));
    for root in roots {
        let library = context(
            s,
            &mut out,
            "library",
            if plan.profile.controlled() {
                "steam:configured-library"
            } else {
                "steam:fixture-library"
            },
            &root.key,
        )?;
        let root_context = context(
            s,
            &mut out,
            "source_instance",
            if plan.profile.controlled() {
                "steam:configured-root"
            } else {
                "steam:fixture-root"
            },
            &root.key,
        )?;
        let mut authority = row(
            "authority_scopes",
            [
                (
                    "authority_namespace",
                    if plan.profile == Profile::SyntheticV2 {
                        "koine:steam-synthetic"
                    } else {
                        "steam:local"
                    }
                    .into(),
                ),
                ("manager_context_id", manager.clone().into()),
                ("machine_context_id", machine.clone().into()),
                ("library_context_id", library.into()),
            ],
        );
        authority.set("scope_key", canonical::authority_key(&authority));
        let aid = ensure(
            s,
            &mut out,
            "authority_scopes",
            &["authority_namespace", "scope_key"],
            authority
                .fields
                .into_iter()
                .map(|(k, v)| match k.as_str() {
                    "authority_namespace" => ("authority_namespace", v),
                    "manager_context_id" => ("manager_context_id", v),
                    "machine_context_id" => ("machine_context_id", v),
                    "library_context_id" => ("library_context_id", v),
                    "scope_key" => ("scope_key", v),
                    _ => unreachable!(),
                })
                .collect(),
        )?;
        let dimensions: Vec<_> = if root.key == index_key(plan) {
            vec!["steam_index"]
        } else {
            DIMS.to_vec()
        };
        for dim in dimensions {
            let unit_key = format!("{}|{dim}", root.key);
            let uid = lookup(
                s,
                "requested_units",
                &[
                    ("collection_id", plan.delivery.clone().into()),
                    ("unit_key", unit_key.clone().into()),
                ],
            )
            .map(|r| r.text("id").map(String::from))
            .transpose()?
            .unwrap_or_else(id);
            let mut unit = row(
                "requested_units",
                [
                    ("id", uid.into()),
                    ("collection_id", plan.delivery.clone().into()),
                    ("unit_key", unit_key.into()),
                    ("authority_scope_id", aid.clone().into()),
                    ("dimension", dim.into()),
                    ("root_context_id", root_context.clone().into()),
                ],
            );
            if let Some(relative) = &root.relative {
                unit.set(
                    "selector",
                    selector(
                        &{
                            let base = if let Some(boundary) = &plan.boundary {
                                fixture_fs::from_bytes(boundary.clone()).join(relative)
                            } else {
                                relative.clone()
                            };
                            if dim == "steam_index" {
                                base
                            } else {
                                base.join("steamapps")
                            }
                        },
                        if dim == "steam_index" {
                            "item"
                        } else {
                            "subtree"
                        },
                    ),
                );
                unit.set("selector_rule_id", interp.clone())
            }
            out.push(unit);
        }
    }
    if let Some(existing) = existing {
        if !existing.is("reporter", plan.profile.reporter()) {
            return Err("delivery belongs to another reporter".into());
        }
        let expected_count = plan.roots.len() * DIMS.len() + 1;
        if s.rows("requested_units")
            .filter(|u| u.get("collection_id") == existing.get("id"))
            .count()
            != expected_count
        {
            return Err("registered scope changed".into());
        }
    }
    Ok(out)
}
pub fn register(store: &Store, plan: &Plan) -> Result<()> {
    let plan = plan.clone();
    store.mutate(move |snapshot| registration(snapshot, &plan))
}
/// Atomically register all scopes in a bounded scheduler request.
pub fn register_plans(store: &Store, plans: Vec<Plan>) -> Result<()> {
    store.mutate(move |s| {
        let mut combined = s.clone();
        let mut out = Vec::new();
        for plan in plans {
            let rows = registration(&combined, &plan)?;
            for row in &rows {
                let m = canonical::CATALOG
                    .iter()
                    .find(|m| m.table == row.table)
                    .ok_or("registration table")?;
                if !combined
                    .rows(&row.table)
                    .any(|r| m.keys.iter().all(|k| r.get(k) == row.get(k)))
                {
                    combined.records.push(row.clone());
                }
            }
            out.extend(rows);
        }
        Ok(out)
    })
}
struct Coverage<'a> {
    delivery: &'a str,
    key: &'a str,
    dim: &'a str,
    inspected: bool,
    gaps: &'a [(String, Vec<u8>)],
    usable: bool,
}
fn coverage(
    s: &Snapshot,
    out: &mut Vec<Record>,
    input: Coverage<'_>,
) -> Result<(String, String, Value)> {
    let Coverage {
        delivery,
        key,
        dim,
        inspected,
        gaps,
        usable,
    } = input;
    let unit = lookup(
        s,
        "requested_units",
        &[
            ("collection_id", delivery.into()),
            ("unit_key", format!("{key}|{dim}").into()),
        ],
    )
    .ok_or("unregistered coverage unit")?;
    let hard_failure = !usable
        && gaps
            .iter()
            .any(|g| crate::controlled::definitive_failure(&g.0))
        && s.by_id("collections", &Value::from(delivery))?
            .is("reporter_version", "2");
    let status = if hard_failure {
        "failed"
    } else if !inspected {
        "inconclusive"
    } else if gaps.is_empty() {
        "complete"
    } else if usable {
        "partial"
    } else {
        "inconclusive"
    };
    let rid = id();
    let pid = id();
    let common = [
        ("collection_id", delivery.into()),
        ("unit_id", unit.get("id").clone()),
        ("dimension", dim.into()),
        ("authority_scope_id", unit.get("authority_scope_id").clone()),
        ("outcome", status.into()),
    ];
    let mut result = row("coverage_results", common.clone());
    result.set("id", rid.clone());
    let mut part = row("coverage_parts", common);
    part.set("id", pid.clone());
    part.set("coverage_id", rid);
    part.set("part_key", "whole-root");
    part.set("selector_kind", "whole_unit");
    part.set(
        "inspection",
        if inspected {
            "inspected"
        } else {
            "uninspected"
        },
    );
    if status != "complete" {
        let reason = gaps.first().map_or("uninspected-root", |g| g.0.as_str());
        result.set("reason", reason);
        part.set("reason", reason)
    }
    out.push(result);
    out.push(part);
    for (index, (code, locator)) in gaps.iter().enumerate() {
        out.push(row(
            "diagnostics",
            [
                ("id", id().into()),
                ("collection_id", delivery.into()),
                ("item_key", format!("{key}/{dim}/diagnostic/{index}").into()),
                ("target_kind", "unit".into()),
                ("unit_id", unit.get("id").clone()),
                ("severity", "warning".into()),
                ("code", code.clone().into()),
                (
                    "message",
                    format!(
                        "Steam interpretation: {code}; relative locator bytes (hex): {}",
                        hex(locator)
                    )
                    .into(),
                ),
                ("affected_dimension", dim.into()),
                (
                    "recorded_at",
                    s.by_id("collections", &Value::from(delivery))?
                        .get("started_at")
                        .clone(),
                ),
            ],
        ));
    }
    Ok((pid, status.into(), unit.get("authority_scope_id").clone()))
}
pub fn application(s: &Snapshot, report: &Report) -> Result<Vec<Record>> {
    report.validate()?;
    let c = s.by_id("collections", &report.plan.delivery.clone().into())?;
    if c.int("started_at")? != report.plan.at {
        return Err("delivery time changed".into());
    }
    let fingerprint = format!(
        "steam.report.{}:{}",
        if report.plan.profile.controlled() {
            "v2"
        } else {
            "v1"
        },
        hex(&report.fingerprint())
    );
    if let Ok(closure) = s.find("collection_closures", "collection_id", c.get("id")) {
        return if closure.is("reason", &fingerprint) {
            Ok(vec![])
        } else {
            Err("conflicting report under closed delivery identity".into())
        };
    }
    // Rebuild expected registration and compare against the durable request without changing it.
    for expected in registration(s, &report.plan)? {
        let manifest = canonical::CATALOG
            .iter()
            .find(|m| m.table == expected.table)
            .ok_or("record")?;
        let actual = s
            .rows(&expected.table)
            .find(|r| manifest.keys.iter().all(|k| r.get(k) == expected.get(k)))
            .ok_or("missing registration")?;
        if expected
            .fields
            .iter()
            .any(|(k, v)| k != "request_digest" && actual.get(k) != v)
        {
            return Err("report differs from registered scope".into());
        }
    }
    if report.roots.iter().map(|r| &r.root).collect::<Vec<_>>()
        != report.plan.roots.iter().collect::<Vec<_>>()
    {
        return Err("report shrank or changed requested roots".into());
    }
    let mut out = Vec::new();
    let mut statuses = Vec::new();
    let interp = c.text("interpretation_rule_id")?.to_string();
    let profile = s
        .rows("identity_profiles")
        .find(|p| p.is("namespace", "steam:app") && p.is("version", "1"))
        .ok_or("Steam profile missing")?;
    let scope = s
        .rows("identity_scopes")
        .find(|p| p.is("namespace", "steam:app") && p.is("scope_kind", "global"))
        .ok_or("Steam scope missing")?;
    for root in &report.roots {
        let mut parts = BTreeMap::new();
        let mut authority = Value::Null;
        for dim in DIMS {
            let gaps: Vec<_> = root
                .diagnostics
                .iter()
                .filter(|d| d.dimension.as_deref().is_none_or(|scope| scope == dim))
                .map(|d| (d.code.clone(), d.locator.clone()))
                .collect();
            let (pid, status, aid) = coverage(
                s,
                &mut out,
                Coverage {
                    delivery: &report.plan.delivery,
                    key: &root.root.key,
                    dim,
                    inspected: root.inspected,
                    gaps: &gaps,
                    usable: root.observations.iter().any(|o| match dim {
                        "entry_title" => o.name.is_some(),
                        "presence" => o.presence,
                        "completion" => o.completion.is_some(),
                        "location" => true,
                        _ => false,
                    }),
                },
            )?;
            parts.insert(dim, pid);
            statuses.push(status);
            authority = aid;
        }
        for observation in &root.observations {
            let item = format!("{}/{}", root.root.key, hex(&observation.locator));
            let eid = id();
            out.push(row(
                "evidence_records",
                [
                    ("id", eid.clone().into()),
                    ("collection_id", report.plan.delivery.clone().into()),
                    ("origin_kind", "external".into()),
                    (
                        "origin_namespace",
                        if report.plan.profile == Profile::SyntheticV2 {
                            "koine:steam-synthetic"
                        } else {
                            "steam:local"
                        }
                        .into(),
                    ),
                    ("locator_kind", "path".into()),
                    ("locator", observation.locator.clone().into()),
                    ("collected_at", report.plan.at.into()),
                    (
                        "source_revision",
                        format!("sha256:{}", hex(&observation.revision)).into(),
                    ),
                    ("interpretation_rule_id", interp.clone().into()),
                    ("item_key", item.clone().into()),
                ],
            ));
            let all = Snapshot {
                records: s.records.iter().chain(out.iter()).cloned().collect(),
            };
            let key = lookup(
                &all,
                "entry_keys",
                &[
                    ("namespace", "steam:app".into()),
                    ("scope_id", scope.get("id").clone()),
                    ("external_key", observation.app_id.to_string().into()),
                ],
            );
            let (entry, kid) = if let Some(key) = key {
                (
                    key.text("entry_id")?.to_string(),
                    key.text("id")?.to_string(),
                )
            } else {
                let entry = id();
                let kid = id();
                out.push(row(
                    "source_entries",
                    [
                        ("id", entry.clone().into()),
                        ("catalog_namespace", "steam".into()),
                        ("created_evidence_id", eid.clone().into()),
                    ],
                ));
                out.push(row(
                    "entry_keys",
                    [
                        ("id", kid.clone().into()),
                        ("entry_id", entry.clone().into()),
                        ("namespace", "steam:app".into()),
                        ("scope_id", scope.get("id").clone()),
                        ("scope_kind", "global".into()),
                        ("external_key", observation.app_id.to_string().into()),
                        ("created_evidence_id", eid.clone().into()),
                    ],
                ));
                (entry, kid)
            };
            out.push(row(
                "entry_key_observations",
                [
                    ("id", id().into()),
                    ("key_id", kid.into()),
                    ("reported_key", observation.reported_id.clone().into()),
                    ("profile_id", profile.get("id").clone()),
                    ("namespace", "steam:app".into()),
                    ("scope_kind", "global".into()),
                    ("evidence_id", eid.clone().into()),
                    ("collection_id", report.plan.delivery.clone().into()),
                    ("coverage_part_id", parts["presence"].clone().into()),
                    ("item_key", format!("{item}/key").into()),
                    (
                        "basis",
                        "versioned Steam decimal AppID; no entitlement assertion".into(),
                    ),
                ],
            ));
            let manager = c.get("reporter_context_id").clone();
            let machine = s
                .rows("authority_scopes")
                .find(|a| a.get("id") == &authority)
                .ok_or("authority")?
                .get("machine_context_id")
                .clone();
            let install = ensure(
                s,
                &mut out,
                "installations",
                &[
                    "entry_id",
                    "manager_context_id",
                    "machine_context_id",
                    "instance_key",
                ],
                vec![
                    ("entry_id", entry.clone().into()),
                    ("manager_context_id", manager),
                    ("machine_context_id", machine),
                    ("instance_key", root.root.key.clone().into()),
                    ("created_evidence_id", eid.clone().into()),
                ],
            )?;
            for dim in DIMS {
                let entity = if dim == "entry_title" {
                    &entry
                } else {
                    &install
                };
                let field = if dim == "entry_title" {
                    "entry_id"
                } else {
                    "installation_id"
                };
                let sid = ensure(
                    s,
                    &mut out,
                    "claim_subjects",
                    &[field, "dimension", "authority_scope_id"],
                    vec![
                        (field, entity.clone().into()),
                        ("dimension", dim.into()),
                        ("authority_scope_id", authority.clone()),
                    ],
                )?;
                let value = match dim {
                    "entry_title" => observation.name.clone().map(Value::Text),
                    "presence" => observation.presence.then(|| "present".into()),
                    "completion" => observation.completion.clone().map(Value::Text),
                    "location" => {
                        let relative = fixture_fs::from_bytes(observation.locator.clone());
                        let manifest = if let Some(boundary) = &report.plan.boundary {
                            fixture_fs::from_bytes(boundary.clone()).join(relative)
                        } else {
                            relative
                        };
                        Some(
                            fixture_fs::bytes(
                                &manifest
                                    .parent()
                                    .ok_or("manifest locator")?
                                    .join("common")
                                    .join(&observation.install_dir),
                            )
                            .into(),
                        )
                    }
                    _ => unreachable!(),
                };
                let mut claim = row(
                    "claims",
                    [
                        ("id", id().into()),
                        ("subject_id", sid.into()),
                        ("dimension", dim.into()),
                        ("authority_scope_id", authority.clone()),
                        ("evidence_id", eid.clone().into()),
                        ("collection_id", report.plan.delivery.clone().into()),
                        (
                            "knowledge",
                            if value.is_some() {
                                "observed"
                            } else {
                                "unknown"
                            }
                            .into(),
                        ),
                        (
                            "basis_code",
                            format!(
                                "{}:appid={};StateFlags={}{}",
                                if report.plan.profile.controlled() {
                                    "steam.controlled.v2"
                                } else {
                                    "steam.manifest.v1"
                                },
                                observation.reported_id,
                                observation
                                    .flags
                                    .map_or_else(|| "unknown".into(), |f| f.to_string()),
                                report.plan.physical_binding.as_ref().map_or_else(
                                    String::new,
                                    |b| format!(";root-binding-sha256={}", hex(b))
                                )
                            )
                            .into(),
                        ),
                        ("coverage_part_id", parts[dim].clone().into()),
                    ],
                );
                if let Some(value) = value {
                    claim.set(
                        if dim == "location" {
                            "value_bytes"
                        } else {
                            "value_text"
                        },
                        value,
                    )
                } else {
                    claim.set("reason", format!("Steam {dim} interpretation unavailable"))
                }
                out.push(claim);
            }
        }
    }
    let index_gaps: Vec<_> = report
        .index_issues
        .iter()
        .map(|code| (code.clone(), b"steamapps/libraryfolders.vdf".to_vec()))
        .collect();
    let (_, index_status, _) = coverage(
        s,
        &mut out,
        Coverage {
            delivery: &report.plan.delivery,
            key: &index_key(&report.plan),
            dim: "steam_index",
            inspected: report.plan.index_revision.is_some(),
            gaps: &index_gaps,
            usable: report.plan.index_issues.is_empty(),
        },
    )?;
    statuses.push(index_status);
    let usable = report.roots.iter().any(|r| !r.observations.is_empty());
    let outcome = if report.plan.profile.controlled() {
        report.outcome()
    } else if statuses.iter().all(|s| s == "complete") {
        "complete"
    } else if usable {
        "partial"
    } else {
        "inconclusive"
    };
    out.push(row(
        "collection_closures",
        [
            ("collection_id", report.plan.delivery.clone().into()),
            ("outcome", outcome.into()),
            ("finished_at", report.plan.at.into()),
            ("reason", fingerprint.into()),
            ("result_protocol", "koine.result.v1".into()),
        ],
    ));
    Ok(out)
}
pub fn apply(store: &Store, report: &Report) -> Result<()> {
    let report = report.clone();
    store.mutate(move |snapshot| application(snapshot, &report))
}
pub fn discover(
    store: &Store,
    fixture: &FixtureFs,
    delivery: String,
    at: i64,
    cancel: Arc<AtomicBool>,
) -> Result<Report> {
    let plan = fixture.plan(delivery, at)?;
    register(store, &plan)?;
    let report = fixture.scan(plan, cancel)?;
    apply(store, &report)?;
    Ok(report)
}
pub fn assessment(store: &Store, at: i64) -> Result<Assessment> {
    let s = store.snapshot()?;
    let rule = |name: &str| {
        s.rows("rule_versions")
            .find(|r| r.is("name", name))
            .ok_or("policy missing")?
            .text("id")
            .map(String::from)
    };
    Ok(Assessment {
        at,
        observer_epoch: None,
        reconciliation_rule: rule(rules::RECONCILIATION)?,
        freshness_rule: rule(rules::FRESHNESS)?,
    })
}

/// Close only unfinished configured collections after journal replay, without source I/O.
/// Observations already committed remain immutable. An interruption grants no negatives.
pub fn recover_pending(store: &Store, at: i64) -> Result<()> {
    store.mutate(move |s| {
        let mut out = Vec::new();
        for c in s.rows("collections").filter(|c| {
            c.is("reporter", "koine.steam.configured") || c.is("reporter", "koine.steam.synthetic")
        }) {
            let delivery = c.text("id")?;
            if s.find("collection_closures", "collection_id", c.get("id"))
                .is_ok()
            {
                continue;
            }
            for u in s
                .rows("requested_units")
                .filter(|u| u.get("collection_id") == c.get("id"))
            {
                let dim = u.text("dimension")?;
                let key = u
                    .text("unit_key")?
                    .strip_suffix(&format!("|{dim}"))
                    .ok_or("unsupported pending unit")?;
                coverage(
                    s,
                    &mut out,
                    Coverage {
                        delivery,
                        key,
                        dim,
                        inspected: false,
                        gaps: &[("interrupted-before-durable-result".into(), vec![])],
                        usable: false,
                    },
                )?;
            }
            out.push(row(
                "collection_closures",
                [
                    ("collection_id", delivery.into()),
                    ("outcome", "inconclusive".into()),
                    ("finished_at", at.max(c.int("started_at")?).into()),
                    ("reason", "steam.scheduler.interrupted.v1".into()),
                    ("result_protocol", "koine.result.v1".into()),
                ],
            ));
        }
        Ok(out)
    })
}

//! Validate immutable evidence and reconstruct conclusions with explicit inputs.
use crate::{
    canonical::{self, CATALOG},
    *,
};
use std::collections::{BTreeMap, BTreeSet};

fn require(ok: bool, message: &str) -> Result<()> {
    if ok { Ok(()) } else { Err(message.into()) }
}
pub(crate) fn referenced<'a>(
    s: &'a Snapshot,
    r: &Record,
    field: &str,
    table: &str,
) -> Result<&'a Record> {
    s.by_id(table, r.get(field))
}
/// Only this deliberately synthetic semantics is implemented. No real source is
/// silently interpreted by a fixture profile.
pub fn synthetic_artifact(kind: &str) -> Vec<u8> {
    canonical::hash(format!("koine.synthetic.v1:{kind}").as_bytes())
}
pub fn supported_rule(s: &Snapshot, id: &Value, kind: &str) -> Result<()> {
    let r = s.by_id("rule_versions", id)?;
    require(
        r.is("kind", kind)
            && r.is("version", rules::version(r.text("name")?))
            && (r.is("name", "koine.synthetic.v1")
                && r.get("artifact_digest") == &Value::Blob(synthetic_artifact(kind))
                || rules::artifact(r.text("name")?, kind)
                    .is_some_and(|digest| r.get("artifact_digest") == &Value::Blob(digest))),
        "unsupported rule artifact",
    )
}
fn no_cycle(s: &Snapshot, table: &str, from: &str, to: &str) -> Result<()> {
    let mut graph: BTreeMap<Value, Vec<Value>> = BTreeMap::new();
    for r in s.rows(table) {
        if r.get(to) != &Value::Null {
            graph
                .entry(r.get(from).clone())
                .or_default()
                .push(r.get(to).clone())
        }
    }
    let mut colors = BTreeMap::new();
    for start in graph.keys() {
        let mut stack = vec![(start.clone(), false)];
        while let Some((node, exit)) = stack.pop() {
            if exit {
                colors.insert(node, 2u8);
                continue;
            }
            match colors.get(&node) {
                Some(1) => return Err("cyclic evidence graph".into()),
                Some(2) => continue,
                _ => {}
            }
            colors.insert(node.clone(), 1);
            stack.push((node.clone(), true));
            if let Some(next) = graph.get(&node) {
                for child in next.iter().rev() {
                    stack.push((child.clone(), false))
                }
            }
        }
    }
    Ok(())
}
fn diagnostic_affects(s: &Snapshot, d: &Record, part: &Record) -> bool {
    if d.is("severity", "info") {
        return false;
    }
    if d.is("target_kind", "collection") {
        return true;
    }
    if d.is("target_kind", "unit") {
        return d.get("unit_id") == part.get("unit_id");
    }
    let affected = if d.is("target_kind", "coverage_part") {
        Some(d.get("coverage_part_id"))
    } else if d.is("target_kind", "claim") {
        s.by_id("claims", d.get("claim_id"))
            .ok()
            .map(|r| r.get("coverage_part_id"))
    } else if d.is("target_kind", "key_observation") {
        s.by_id("entry_key_observations", d.get("key_observation_id"))
            .ok()
            .map(|r| r.get("coverage_part_id"))
    } else {
        None
    };
    affected
        .and_then(|id| s.by_id("coverage_parts", id).ok())
        .is_none_or(|p| p.get("unit_id") == part.get("unit_id"))
}
pub(crate) fn source_order(
    s: &Snapshot,
    older: &Record,
    newer: &Record,
    decision: &Record,
) -> Result<()> {
    supported_rule(s, decision.get("ordering_rule_id"), "ordering")?;
    let a = referenced(s, older, "evidence_id", "evidence_records")?;
    let b = referenced(s, newer, "evidence_id", "evidence_records")?;
    if a.is("origin_kind", "user_assertion") && b.is("origin_kind", "user_assertion") {
        return require(
            decision.text("reason")? == format!("user-action:{}", older.text("id")?),
            "manual order needs explicit causal user action",
        );
    }
    let ac = referenced(s, a, "collection_id", "collections")?;
    let bc = referenced(s, b, "collection_id", "collections")?;
    require(
        a.get("origin_namespace") == b.get("origin_namespace")
            && ac.get("reporter_context_id") == bc.get("reporter_context_id")
            && ac.get("reporter") == bc.get("reporter"),
        "incomparable source chronology",
    )?;
    require(
        referenced(s, a, "interpretation_rule_id", "rule_versions")?
            .is("name", "koine.synthetic.v1")
            && referenced(s, b, "interpretation_rule_id", "rule_versions")?
                .is("name", "koine.synthetic.v1"),
        "no source chronology proof for this interpretation",
    )?;
    let revision = |r: &Record| -> Result<u64> {
        r.text("source_revision")?
            .strip_prefix("sequence:")
            .ok_or("missing fixture sequence")?
            .parse()
            .map_err(|_| "invalid fixture sequence".into())
    };
    require(
        revision(a)? < revision(b)?,
        "ordering not established by source sequence",
    )
}
fn validate_decision(s: &Snapshot, d: &Record) -> Result<()> {
    supported_rule(s, d.get("rule_id"), "reconciliation")?;
    let seal = s.find("decision_seals", "decision_id", d.get("id"))?;
    require(
        seal.is("protocol", "koine.decision.v1")
            && seal.get("content_digest")
                == &Value::Blob(canonical::digest(s, "koine.decision.v1", d.get("id"))?),
        "invalid decision seal",
    )?;
    validate_decision_content(s, d)
}
/// Validate the complete unsealed proof before generating a decision seal.
pub fn validate_decision_content(s: &Snapshot, d: &Record) -> Result<()> {
    supported_rule(s, d.get("rule_id"), "reconciliation")?;
    require(
        ["supersession", "supersession_rejected", "dispute"]
            .iter()
            .any(|kind| d.is("kind", kind)),
        "unsupported decision kind",
    )?;
    no_cycle(s, "supersessions", "older_claim_id", "replacement_claim_id")?;
    let links: Vec<_> = s
        .rows("decision_claims")
        .filter(|r| r.get("decision_id") == d.get("id"))
        .collect();
    for l in &links {
        let c = referenced(s, l, "claim_id", "claims")?;
        validate_claim(s, c)?;
    }
    let edges: Vec<_> = s
        .rows("supersessions")
        .filter(|r| r.get("decision_id") == d.get("id"))
        .collect();
    if !d.is("kind", "supersession") {
        require(edges.is_empty(), "non-supersession has edges")?;
        if d.is("kind", "dispute") {
            require(
                links.iter().filter(|r| r.is("role", "conflicting")).count() >= 2,
                "incomplete dispute",
            )?
        }
        return Ok(());
    }
    require(!edges.is_empty(), "empty supersession")?;
    for edge in edges {
        let older = referenced(s, edge, "older_claim_id", "claims")?;
        let newer = referenced(s, edge, "replacement_claim_id", "claims")?;
        require(
            ["subject_id", "dimension", "authority_scope_id"]
                .iter()
                .all(|k| older.get(k) == newer.get(k)),
            "supersession subject or authority mismatch",
        )?;
        require(
            !newer.is("knowledge", "unknown"),
            "unknown does not revoke known evidence",
        )?;
        for (role, c) in [("older", older), ("replacement", newer)] {
            require(
                links
                    .iter()
                    .any(|l| l.is("role", role) && l.get("claim_id") == c.get("id")),
                "missing decision endpoint role",
            )?
        }
        source_order(s, older, newer, d)?;
        if newer.is("knowledge", "confirmed_negative") {
            require(
                links.iter().any(|l| {
                    l.is("role", "negative_support") && l.get("claim_id") == newer.get("id")
                }),
                "missing negative support role",
            )?
        }
    }
    Ok(())
}
fn validate_claim(s: &Snapshot, c: &Record) -> Result<()> {
    require(
        ["observed", "confirmed_negative", "unknown"]
            .iter()
            .any(|k| c.is("knowledge", k)),
        "unsupported claim knowledge",
    )?;
    if c.is("knowledge", "unknown") {
        require(
            c.get("reason") != &Value::Null
                && [
                    "value_text",
                    "value_bytes",
                    "access_kind",
                    "negative_justification_id",
                ]
                .iter()
                .all(|k| c.get(k) == &Value::Null),
            "unknown claim has invented payload",
        )?
    } else if c.is("dimension", "location") {
        require(
            c.get("value_text") == &Value::Null
                && matches!(c.get("value_bytes"),Value::Blob(b) if !b.is_empty()),
            "location requires exact bytes",
        )?
    } else {
        require(
            c.get("value_text") != &Value::Null && c.get("value_bytes") == &Value::Null,
            "claim payload missing or ambiguous",
        )?
    }
    if c.is("knowledge", "confirmed_negative") {
        require(
            (c.is("dimension", "access") && c.is("value_text", "inaccessible"))
                || (c.is("dimension", "presence") && c.is("value_text", "absent")),
            "negative value not permitted for dimension",
        )?
    }
    if c.is("knowledge", "observed") {
        let permitted: &[&str] = match c.text("dimension")? {
            "entry_title" | "entry_edition" | "game_label" => &[],
            "manual_visibility" => &["visible", "hidden"],
            "access" => &["accessible"],
            "presence" => &["present"],
            "completion" => &["complete", "incomplete"],
            "location" => &[],
            "handoff_capability" => &["supported", "unavailable"],
            "game_link" | "target_installation" => &["linked", "unlinked"],
            "entry_equivalence" | "context_equivalence" => &["equivalent", "different"],
            _ => return Err("unsupported claim dimension".into()),
        };
        if !permitted.is_empty() {
            require(
                permitted.iter().any(|v| c.is("value_text", v)),
                "invalid typed claim value",
            )?
        }
    }
    let subject = referenced(s, c, "subject_id", "claim_subjects")?;
    let fields = [
        "game_id",
        "entry_id",
        "entitlement_id",
        "installation_id",
        "target_id",
        "game_link_id",
        "equivalence_id",
        "context_equivalence_id",
        "target_installation_link_id",
    ];
    require(
        fields
            .iter()
            .filter(|field| subject.get(field) != &Value::Null)
            .count()
            == 1,
        "claim subject must identify exactly one entity",
    )?;
    let evidence = referenced(s, c, "evidence_id", "evidence_records")?;
    supported_rule(s, evidence.get("interpretation_rule_id"), "interpretation")?;
    if [rules::STEAM_INTERPRETATION, rules::STEAM_CONTROLLED]
        .iter()
        .any(|name| {
            referenced(s, evidence, "interpretation_rule_id", "rule_versions")
                .is_ok_and(|r| r.is("name", name))
        })
    {
        require(
            ["entry_title", "presence", "completion", "location"]
                .iter()
                .any(|d| c.is("dimension", d))
                && !c.is("knowledge", "confirmed_negative"),
            "Steam installation observations cannot assert entitlement, absence or handoff",
        )?;
    }
    require(
        subject.get("dimension") == c.get("dimension")
            && subject.get("authority_scope_id") == c.get("authority_scope_id")
            && evidence.get("collection_id") == c.get("collection_id"),
        "claim scope mismatch",
    )?;
    let authority = referenced(s, c, "authority_scope_id", "authority_scopes")?;
    if evidence.is("origin_kind", "user_assertion") {
        require(
            authority.is("authority_namespace", "koine:manual")
                && evidence.is("origin_namespace", "koine:manual"),
            "manual assertions cannot overwrite external authority",
        )?
    }
    for (field, table, roles) in [
        (
            "entitlement_id",
            "entitlements",
            &["subject_context_id"][..],
        ),
        (
            "installation_id",
            "installations",
            &["manager_context_id", "machine_context_id"][..],
        ),
    ] {
        if subject.get(field) != &Value::Null {
            let entity = referenced(s, subject, field, table)?;
            for role in roles {
                require(
                    entity.get(role) == authority.get(role),
                    "claim authority context does not match its entity",
                )?
            }
        }
    }
    if subject.get("target_id") != &Value::Null {
        let target = referenced(s, subject, "target_id", "launch_targets")?;
        require(
            target.get("launcher_context_id") == authority.get("manager_context_id"),
            "capability authority does not match launcher",
        )?
    }
    if evidence.is("origin_kind", "external") {
        let part = referenced(s, c, "coverage_part_id", "coverage_parts")?;
        require(
            ["collection_id", "dimension", "authority_scope_id"]
                .iter()
                .all(|k| part.get(k) == c.get(k)),
            "external claim lacks scoped coverage",
        )?;
        require(
            part.is("inspection", "inspected") || part.is("inspection", "mixed"),
            "uninspected evidence cannot support external claim",
        )?;
        require(
            evidence.get("origin_namespace") == authority.get("authority_namespace"),
            "source authority mismatch",
        )?;
        if subject.get("installation_id") != &Value::Null {
            let installation = referenced(s, subject, "installation_id", "installations")?;
            let unit = referenced(s, part, "unit_id", "requested_units")?;
            let root = referenced(s, unit, "root_context_id", "contexts")?;
            require(
                installation.get("instance_key") == root.get("external_key"),
                "synthetic claim outside inspected installation root",
            )?;
        }
    }
    if c.is("dimension", "entry_equivalence") && c.is("value_text", "equivalent") {
        let pair = referenced(s, subject, "equivalence_id", "entry_equivalences")?;
        let a = referenced(s, pair, "left_entry_id", "source_entries")?;
        let b = referenced(s, pair, "right_entry_id", "source_entries")?;
        require(
            a.get("catalog_namespace") != &Value::Null
                && a.get("catalog_namespace") == b.get("catalog_namespace"),
            "unproved catalog identity equivalence",
        )?;
    }
    if c.is("dimension", "context_equivalence") && c.is("value_text", "equivalent") {
        let pair = referenced(s, subject, "context_equivalence_id", "context_equivalences")?;
        let a = referenced(s, pair, "left_context_id", "contexts")?;
        let b = referenced(s, pair, "right_context_id", "contexts")?;
        require(
            a.get("kind") == b.get("kind") && a.get("namespace") == b.get("namespace"),
            "incompatible context identity",
        )?;
        require(
            !(a.is("resolution", "identified") && b.is("resolution", "identified"))
                || a.get("external_key") == b.get("external_key"),
            "distinct identified contexts cannot merge",
        )?;
    }
    if c.is("dimension", "location")
        && let Value::Blob(b) = c.get("value_bytes")
    {
        require(!b.contains(&0), "NUL in Linux path")?
    }
    if c.is("dimension", "manual_visibility") {
        require(
            evidence.is("origin_kind", "user_assertion")
                && authority.is("authority_namespace", "koine:manual"),
            "manual visibility requires user authority",
        )?;
        let entry = referenced(s, subject, "entry_id", "source_entries")?;
        let creation = referenced(s, entry, "created_evidence_id", "evidence_records")?;
        require(
            creation.is("origin_kind", "user_assertion")
                && creation.is("origin_namespace", "koine:manual"),
            "external entry cannot be hidden as a manual handle",
        )?;
        let library = authority.get("library_context_id");
        require(library != &Value::Null, "manual library missing")?;
        require(
            s.rows("entry_keys").any(|key| {
                key.get("entry_id") == subject.get("entry_id")
                    && s.by_id("identity_scopes", key.get("scope_id"))
                        .is_ok_and(|scope| {
                            scope.is("scope_kind", "library")
                                && scope.get("library_context_id") == library
                                && s.rows("entry_key_observations").any(|o| {
                                    o.get("key_id") == key.get("id")
                                        && s.by_id("identity_profiles", o.get("profile_id"))
                                            .is_ok_and(|p| p.is("key_role", "manual"))
                                        && s.by_id("evidence_records", o.get("evidence_id"))
                                            .is_ok_and(|e| e.is("origin_kind", "user_assertion"))
                                })
                        })
            }),
            "manual entry lacks library key",
        )?;
    }
    if c.is("knowledge", "confirmed_negative") {
        let n = referenced(s, c, "negative_justification_id", "negative_justifications")?;
        supported_rule(s, n.get("capability_rule_id"), "interpretation")?;
        require(
            [
                "subject_id",
                "dimension",
                "authority_scope_id",
                "collection_id",
            ]
            .iter()
            .all(|k| n.get(k) == c.get(k)),
            "negative proof scope mismatch",
        )?;
        if n.is("kind", "exhaustive_omission") {
            let part = referenced(s, n, "coverage_part_id", "coverage_parts")?;
            require(
                part.is("outcome", "complete") && part.is("inspection", "inspected"),
                "omission needs complete bounded inspection",
            )?;
            // Fixture capability is explicit, not inferred from a complete flag.
            require(
                n.is("basis", "synthetic exhaustive enumeration") && c.is("dimension", "presence"),
                "no exhaustive authority for this dimension",
            )?;
            require(
                !s.rows("diagnostics").any(|d| {
                    d.get("collection_id") == c.get("collection_id")
                        && diagnostic_affects(s, d, part)
                }),
                "negative proof has relevant diagnostic gap",
            )?;
        } else {
            require(
                n.is("kind", "explicit") && n.get("evidence_id") == c.get("evidence_id"),
                "explicit negative needs its own evidence",
            )?
        }
    }
    Ok(())
}
fn validate_coverage(s: &Snapshot, closure: &Record) -> Result<()> {
    let cid = closure.get("collection_id");
    let mut outcomes = Vec::new();
    for unit in s
        .rows("requested_units")
        .filter(|r| r.get("collection_id") == cid)
    {
        let result = s.find("coverage_results", "unit_id", unit.get("id"))?;
        let roots: Vec<_> = s
            .rows("coverage_parts")
            .filter(|r| {
                r.get("coverage_id") == result.get("id") && r.get("parent_part_id") == &Value::Null
            })
            .collect();
        require(
            roots.len() == 1 && roots[0].get("outcome") == result.get("outcome"),
            "missing or inconsistent coverage root",
        )?;
        if result.is("outcome", "complete") {
            require(
                !s.rows("diagnostics")
                    .any(|d| d.get("collection_id") == cid && diagnostic_affects(s, d, roots[0])),
                "complete coverage hides diagnostic gap",
            )?
        }
        outcomes.push(result.text("outcome")?);
    }
    require(!outcomes.is_empty(), "collection request has no units")?;
    let usable = s
        .rows("claims")
        .any(|c| c.get("collection_id") == cid && !c.is("knowledge", "unknown"))
        || s.rows("entry_key_observations")
            .any(|r| r.get("collection_id") == cid);
    if closure.is("outcome", "complete") {
        require(
            outcomes.iter().all(|x| *x == "complete"),
            "aggregate completeness hides gap",
        )?
    } else if usable {
        require(
            closure.is("outcome", "partial"),
            "usable partial work cannot be failed/inconclusive",
        )?
    } else if closure.is("outcome", "failed") {
        require(outcomes.contains(&"failed"), "failure needs evidence")?
    }
    for part in s
        .rows("coverage_parts")
        .filter(|r| r.get("collection_id") == cid)
    {
        let children: Vec<_> = s
            .rows("coverage_parts")
            .filter(|r| r.get("parent_part_id") == part.get("id"))
            .collect();
        if part.is("inspection", "mixed") {
            require(!children.is_empty(), "mixed part needs children")?
        }
        if !children.is_empty() {
            // General source-specific partitions are deliberately unsupported.
            // Supported synthetic key-set partitions are exact, disjoint and exhaustive.
            let decode_keys = |r: &Record| -> Result<BTreeSet<Value>> {
                let Value::List(v) = canonical::decode(r.get("selector").blob()?)? else {
                    return Err("selector must be list".into());
                };
                require(
                    v.len() == 4 && v[0] == Value::from("keys"),
                    "unsupported selector partition",
                )?;
                let Value::List(keys) = &v[3] else {
                    return Err("key selector list missing".into());
                };
                let set: BTreeSet<_> = keys.iter().cloned().collect();
                require(set.len() == keys.len(), "duplicate selector member")?;
                Ok(set)
            };
            let mut union = BTreeSet::new();
            for child in &children {
                supported_rule(s, child.get("selector_rule_id"), "interpretation")?;
                for key in decode_keys(child)? {
                    require(union.insert(key), "overlapping coverage children")?
                }
            }
            if part.get("parent_part_id") != &Value::Null {
                require(
                    union == decode_keys(part)?,
                    "nonexhaustive coverage partition",
                )?
            } else {
                let unit = referenced(s, part, "unit_id", "requested_units")?;
                require(
                    unit.get("selector") != &Value::Null,
                    "root partition needs declared synthetic key extent",
                )?;
                let mut extent = part.clone();
                extent.set("selector", unit.get("selector").clone());
                require(
                    union == decode_keys(&extent)?,
                    "root partition not exhaustive",
                )?;
            }
            if part.is("outcome", "complete") {
                require(
                    children.iter().all(|p| p.is("outcome", "complete")),
                    "complete parent hides child gap",
                )?
            }
        }
    }
    Ok(())
}
/// Validation for committing supported synthetic evidence. SQL independently
/// enforces types, references and immutable boundaries.
pub fn validate(s: &Snapshot) -> Result<()> {
    for r in &s.records {
        let m = CATALOG
            .iter()
            .find(|m| m.table == r.table)
            .ok_or("unknown record family")?;
        require(
            r.fields.keys().all(|k| m.columns.contains(&k.as_str())),
            "unknown record field",
        )?;
        for (k, v) in &r.fields {
            if (k == "id"
                || (k.ends_with("_id") && k != "migration_id")
                || k == "provisional_token"
                || k == "observer_epoch")
                && v != &Value::Null
            {
                let text = v.text()?;
                let parsed = uuid::Uuid::parse_str(text).map_err(|_| "invalid UUID")?;
                require(
                    parsed.to_string() == text && parsed.get_variant() == uuid::Variant::RFC4122,
                    "invalid canonical UUID",
                )?;
            }
        }
    }
    for a in s.rows("authority_scopes") {
        require(
            a.get("scope_key") == &Value::Blob(canonical::authority_key(a)),
            "invalid authority key",
        )?
    }
    for (table, columns) in [
        (
            "identity_scopes",
            vec![
                ("subject_context_id", "subject"),
                ("library_context_id", "library"),
            ],
        ),
        ("entitlements", vec![("subject_context_id", "subject")]),
        (
            "installations",
            vec![
                ("manager_context_id", "manager"),
                ("machine_context_id", "machine"),
            ],
        ),
        ("launch_targets", vec![("launcher_context_id", "manager")]),
        (
            "authority_scopes",
            vec![
                ("subject_context_id", "subject"),
                ("manager_context_id", "manager"),
                ("machine_context_id", "machine"),
                ("library_context_id", "library"),
            ],
        ),
    ] {
        for r in s.rows(table) {
            for (field, kind) in &columns {
                if r.get(field) != &Value::Null {
                    require(
                        referenced(s, r, field, "contexts")?.is("kind", kind),
                        "context role mismatch",
                    )?
                }
            }
        }
    }
    for p in s.rows("identity_profiles") {
        supported_rule(s, p.get("rule_id"), "identity")?
    }
    for o in s.rows("entry_key_observations") {
        let key = referenced(s, o, "key_id", "entry_keys")?;
        let profile = referenced(s, o, "profile_id", "identity_profiles")?;
        let e = referenced(s, o, "evidence_id", "evidence_records")?;
        require(
            if referenced(s, profile, "rule_id", "rule_versions")?.is("name", rules::STEAM_IDENTITY)
            {
                profile.is("namespace", "steam:app")
                    && profile.is("scope_kind", "global")
                    && rules::steam_key(o.text("reported_key")?)? == key.text("external_key")?
            } else {
                o.get("reported_key") == key.get("external_key")
            },
            "reported key violates its supported identity profile",
        )?;
        if profile.is("key_role", "manual") {
            require(
                e.is("origin_kind", "user_assertion"),
                "manual key origin mismatch",
            )?
        }
        if e.is("origin_kind", "external") {
            let p = referenced(s, o, "coverage_part_id", "coverage_parts")?;
            require(
                p.get("collection_id") == o.get("collection_id"),
                "key observation coverage mismatch",
            )?
        }
    }
    for link in s.rows("target_installation_links") {
        let a = referenced(s, link, "target_id", "launch_targets")?;
        let b = referenced(s, link, "installation_id", "installations")?;
        require(
            a.get("entry_id") == b.get("entry_id"),
            "unproved cross-entry target association",
        )?;
    }
    for c in s.rows("collections") {
        supported_rule(s, c.get("interpretation_rule_id"), "interpretation")?;
        if c.is("kind", "manual") {
            require(
                referenced(s, c, "reporter_context_id", "contexts")?.is("kind", "library"),
                "manual reporter must identify its library",
            )?
        }
        require(
            c.is("request_protocol", "koine.request.v1")
                && c.get("request_digest")
                    == &Value::Blob(canonical::digest(s, "koine.request.v1", c.get("id"))?),
            "invalid request digest",
        )?;
    }
    no_cycle(s, "supersessions", "older_claim_id", "replacement_claim_id")?;
    no_cycle(s, "coverage_parts", "id", "parent_part_id")?;
    for c in s.rows("claims") {
        validate_claim(s, c)?
    }
    for d in s.rows("reconciliation_decisions") {
        validate_decision(s, d)?
    }
    for c in s.rows("collection_closures") {
        validate_coverage(s, c)?;
        require(
            c.is("result_protocol", "koine.result.v1")
                && c.get("result_digest")
                    == &Value::Blob(canonical::digest(
                        s,
                        "koine.result.v1",
                        c.get("collection_id"),
                    )?),
            "invalid result digest",
        )?;
    }
    for e in s.rows("evidence_records") {
        let collection = referenced(s, e, "collection_id", "collections")?;
        require(
            (e.is("origin_kind", "external") && collection.is("kind", "discovery"))
                || (e.is("origin_kind", "user_assertion") && collection.is("kind", "manual")),
            "evidence origin does not match collection kind",
        )?;
        if e.is("locator_kind", "path") {
            require(
                !e.get("locator").blob()?.contains(&0),
                "NUL in evidence path",
            )?
        }
    }
    for p in s.rows("process_observations") {
        if p.is("state", "observed") || p.is("state", "exited") {
            require(
                p.is("identity_basis", "synthetic identity beyond PID"),
                "unsupported process association basis",
            )?
        }
        supported_rule(s, p.get("method_rule_id"), "interpretation")?;
        if p.get("prior_observed_id") != &Value::Null {
            let old = referenced(s, p, "prior_observed_id", "process_observations")?;
            require(
                old.is("state", "observed")
                    && old.get("attempt_id") == p.get("attempt_id")
                    && old.get("observer_epoch") == p.get("observer_epoch")
                    && old.int("observed_at")? <= p.int("observed_at")?,
                "process continuity not established",
            )?;
            if p.is("state", "exited") {
                require(
                    old.get("identity_token") == p.get("identity_token"),
                    "process identity changed",
                )?
            }
        }
    }
    for h in s.rows("handoff_records") {
        supported_rule(s, h.get("transport_rule_id"), "transport")?
    }
    Ok(())
}

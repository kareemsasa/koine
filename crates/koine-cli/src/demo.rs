//! A deterministic semantic scenario with generated local IDs, never a live transport.
use koine_core::{canonical, reconcile, *};
use koine_sqlite::Store;
use std::collections::BTreeMap;
fn row(table: &str, fields: impl IntoIterator<Item = (&'static str, Value)>) -> Record {
    Record::new(table, fields)
}
struct Failure;
impl Transport for Failure {
    fn submit(&mut self, _target: &str, _launcher: &str) -> Result<TransportResult> {
        Ok(TransportResult {
            issuance: "issued".into(),
            disposition: "failed".into(),
            status: Some(127),
            boundary: "synthetic transport".into(),
            reason: Some("synthetic definite handoff failure".into()),
        })
    }
}
pub fn populate(store: &Store) -> Result<()> {
    let mut rules = BTreeMap::new();
    let mut seed = Vec::new();
    for kind in [
        "identity",
        "interpretation",
        "reconciliation",
        "ordering",
        "freshness",
        "transport",
    ] {
        let rid = id();
        rules.insert(kind, rid.clone());
        seed.push(row(
            "rule_versions",
            [
                ("id", rid.into()),
                ("kind", kind.into()),
                ("name", "koine.synthetic.v1".into()),
                ("version", "1".into()),
                (
                    "artifact_digest",
                    reconcile::synthetic_artifact(kind).into(),
                ),
            ],
        ));
    }
    let manager = id();
    let root = id();
    for (cid, kind, key) in [
        (&manager, "manager", "fake-launcher"),
        (&root, "source_instance", "root-A"),
    ] {
        seed.push(row(
            "contexts",
            [
                ("id", cid.clone().into()),
                ("kind", kind.into()),
                ("namespace", "fixture".into()),
                ("resolution", "identified".into()),
                ("external_key", key.into()),
            ],
        ));
    }
    let authority = id();
    let mut a = row(
        "authority_scopes",
        [
            ("id", authority.clone().into()),
            ("authority_namespace", "fixture:steam".into()),
            ("manager_context_id", manager.clone().into()),
        ],
    );
    a.set("scope_key", canonical::authority_key(&a));
    seed.push(a);
    let scope = id();
    seed.push(row(
        "identity_scopes",
        [
            ("id", scope.clone().into()),
            ("namespace", "fixture:steam".into()),
            ("scope_kind", "global".into()),
            ("resolution", "resolved".into()),
        ],
    ));
    let profile = id();
    seed.push(row(
        "identity_profiles",
        [
            ("id", profile.clone().into()),
            ("namespace", "fixture:steam".into()),
            ("version", "1".into()),
            ("scope_kind", "global".into()),
            ("key_role", "catalog".into()),
            ("catalog_namespace", "fixture:steam".into()),
            ("rule_id", rules["identity"].clone().into()),
        ],
    ));
    store.write(seed)?;
    let collection = id();
    let mut registration = vec![row(
        "collections",
        [
            ("id", collection.clone().into()),
            ("kind", "discovery".into()),
            ("reporter", "offline synthetic adapter".into()),
            ("reporter_version", "1".into()),
            ("reporter_context_id", manager.clone().into()),
            (
                "interpretation_rule_id",
                rules["interpretation"].clone().into(),
            ),
            ("started_at", 1i64.into()),
            ("request_protocol", "koine.request.v1".into()),
        ],
    )];
    let mut result = Vec::new();
    let mut parts = BTreeMap::new();
    for dimension in ["entry_title", "handoff_capability"] {
        let unit = id();
        let coverage = id();
        let part = id();
        parts.insert(dimension, part.clone());
        registration.push(row(
            "requested_units",
            [
                ("id", unit.clone().into()),
                ("collection_id", collection.clone().into()),
                ("unit_key", dimension.into()),
                ("authority_scope_id", authority.clone().into()),
                ("dimension", dimension.into()),
                ("root_context_id", root.clone().into()),
            ],
        ));
        result.push(row(
            "coverage_results",
            [
                ("id", coverage.clone().into()),
                ("unit_id", unit.clone().into()),
                ("collection_id", collection.clone().into()),
                ("dimension", dimension.into()),
                ("authority_scope_id", authority.clone().into()),
                ("outcome", "complete".into()),
            ],
        ));
        result.push(row(
            "coverage_parts",
            [
                ("id", part.into()),
                ("coverage_id", coverage.into()),
                ("collection_id", collection.clone().into()),
                ("unit_id", unit.into()),
                ("dimension", dimension.into()),
                ("authority_scope_id", authority.clone().into()),
                ("part_key", "root".into()),
                ("selector_kind", "whole_unit".into()),
                ("inspection", "inspected".into()),
                ("outcome", "complete".into()),
            ],
        ));
    }
    store.write(registration)?;
    let evidence = id();
    result.push(row(
        "evidence_records",
        [
            ("id", evidence.clone().into()),
            ("collection_id", collection.clone().into()),
            ("origin_kind", "external".into()),
            ("origin_namespace", "fixture:steam".into()),
            ("locator_kind", "opaque".into()),
            ("locator", b"synthetic fixture".to_vec().into()),
            ("collected_at", 1i64.into()),
            ("source_effective_at", 1i64.into()),
            ("source_revision", "sequence:1".into()),
            (
                "interpretation_rule_id",
                rules["interpretation"].clone().into(),
            ),
            ("item_key", "fixture".into()),
        ],
    ));
    let entry = id();
    let key = id();
    let target = id();
    result.push(row(
        "source_entries",
        [
            ("id", entry.clone().into()),
            ("catalog_namespace", "fixture:steam".into()),
            ("created_evidence_id", evidence.clone().into()),
        ],
    ));
    result.push(row(
        "entry_keys",
        [
            ("id", key.clone().into()),
            ("entry_id", entry.clone().into()),
            ("namespace", "fixture:steam".into()),
            ("scope_id", scope.into()),
            ("scope_kind", "global".into()),
            ("external_key", "1549690".into()),
            ("created_evidence_id", evidence.clone().into()),
        ],
    ));
    result.push(row(
        "entry_key_observations",
        [
            ("id", id().into()),
            ("key_id", key.into()),
            ("reported_key", "1549690".into()),
            ("profile_id", profile.into()),
            ("namespace", "fixture:steam".into()),
            ("scope_kind", "global".into()),
            ("evidence_id", evidence.clone().into()),
            ("collection_id", collection.clone().into()),
            ("coverage_part_id", parts["entry_title"].clone().into()),
            ("item_key", "fixture-key".into()),
            ("basis", "synthetic exact key".into()),
        ],
    ));
    result.push(row(
        "launch_targets",
        [
            ("id", target.clone().into()),
            ("entry_id", entry.clone().into()),
            ("launcher_context_id", manager.into()),
            ("target_key", "fixture-only".into()),
            ("created_evidence_id", evidence.clone().into()),
        ],
    ));
    for (dimension, field, entity, value) in [
        ("entry_title", "entry_id", entry, "SPRAWL (synthetic)"),
        (
            "handoff_capability",
            "target_id",
            target.clone(),
            "supported",
        ),
    ] {
        let subject = id();
        result.push(row(
            "claim_subjects",
            [
                ("id", subject.clone().into()),
                ("dimension", dimension.into()),
                ("authority_scope_id", authority.clone().into()),
                (field, entity.into()),
            ],
        ));
        result.push(row(
            "claims",
            [
                ("id", id().into()),
                ("subject_id", subject.into()),
                ("dimension", dimension.into()),
                ("authority_scope_id", authority.clone().into()),
                ("evidence_id", evidence.clone().into()),
                ("collection_id", collection.clone().into()),
                ("knowledge", "observed".into()),
                ("value_text", value.into()),
                ("basis_code", "synthetic explicit fact".into()),
                ("coverage_part_id", parts[dimension].clone().into()),
            ],
        ));
    }
    result.push(row(
        "collection_closures",
        [
            ("collection_id", collection.into()),
            ("outcome", "complete".into()),
            ("finished_at", 2i64.into()),
            ("result_protocol", "koine.result.v1".into()),
        ],
    ));
    store.write(result)?;
    let attempt = id();
    let intent = row(
        "launch_attempts",
        [
            ("id", attempt.clone().into()),
            ("target_id", target.into()),
            ("created_at", 3i64.into()),
            ("intent_reason", "offline demo only".into()),
        ],
    );
    let handoff = row(
        "handoff_records",
        [
            ("id", id().into()),
            ("attempt_id", attempt.clone().into()),
            ("item_key", "result".into()),
            ("recorded_at", 4i64.into()),
            ("transport_rule_id", rules["transport"].clone().into()),
        ],
    );
    store.handoff(intent, handoff, &mut Failure)?;
    store.write(vec![row(
        "process_observations",
        [
            ("id", id().into()),
            ("attempt_id", attempt.into()),
            ("item_key", "observer".into()),
            ("observed_at", 5i64.into()),
            ("method_rule_id", rules["interpretation"].clone().into()),
            ("observer_epoch", id().into()),
            ("state", "inconclusive".into()),
            ("reason", "no synthetic game process established".into()),
        ],
    )])
}

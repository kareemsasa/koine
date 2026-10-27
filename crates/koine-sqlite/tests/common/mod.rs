#![allow(dead_code)]
use koine_core::{canonical, reconcile, *};
use koine_sqlite::Store;
use std::collections::BTreeMap;
use tempfile::TempDir;
pub fn row(t: &str, fields: impl IntoIterator<Item = (&'static str, Value)>) -> Record {
    Record::new(t, fields)
}
pub struct Fixture {
    pub store: Store,
    pub dir: TempDir,
    pub path: std::path::PathBuf,
    pub rules: BTreeMap<String, String>,
    pub contexts: BTreeMap<String, String>,
    pub authorities: BTreeMap<String, String>,
    pub scopes: BTreeMap<String, String>,
    pub profiles: BTreeMap<String, String>,
    pub entries: BTreeMap<String, String>,
    pub keys: BTreeMap<String, String>,
    pub subjects: BTreeMap<String, String>,
    pub instances: BTreeMap<String, String>,
    pub epoch: String,
}
pub struct Batch {
    pub id: String,
    pub ns: String,
    pub rows: Vec<Record>,
    pub parts: BTreeMap<String, String>,
    pub outcome: String,
    pub time: i64,
    pub revision: Option<u64>,
}
impl Fixture {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.sqlite");
        let store = Store::create(&path).unwrap();
        let mut f = Self {
            dir,
            path,
            store,
            rules: BTreeMap::new(),
            contexts: BTreeMap::new(),
            authorities: BTreeMap::new(),
            scopes: BTreeMap::new(),
            profiles: BTreeMap::new(),
            entries: BTreeMap::new(),
            keys: BTreeMap::new(),
            subjects: BTreeMap::new(),
            instances: BTreeMap::new(),
            epoch: id(),
        };
        let mut rows = Vec::new();
        for kind in [
            "identity",
            "interpretation",
            "reconciliation",
            "ordering",
            "freshness",
            "transport",
        ] {
            let rid = id();
            f.rules.insert(kind.into(), rid.clone());
            rows.push(row(
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
        for (name, kind) in [
            ("account-A", "subject"),
            ("account-B", "subject"),
            ("manager-S", "manager"),
            ("manager-H", "manager"),
            ("machine", "machine"),
            ("library", "library"),
            ("root-A", "source_instance"),
            ("root-B", "source_instance"),
        ] {
            let cid = id();
            f.contexts.insert(name.into(), cid.clone());
            rows.push(row(
                "contexts",
                [
                    ("id", cid.into()),
                    ("kind", kind.into()),
                    ("namespace", "fixture".into()),
                    ("resolution", "identified".into()),
                    ("external_key", name.into()),
                ],
            ));
        }
        for ns in ["fixture:steam", "fixture:epic", "koine:manual"] {
            let sid = id();
            f.scopes.insert(ns.into(), sid.clone());
            let mut scope = row(
                "identity_scopes",
                [
                    ("id", sid.into()),
                    ("namespace", ns.into()),
                    (
                        "scope_kind",
                        if ns == "koine:manual" {
                            "library"
                        } else {
                            "global"
                        }
                        .into(),
                    ),
                    ("resolution", "resolved".into()),
                ],
            );
            if ns == "koine:manual" {
                scope.set("library_context_id", f.contexts["library"].clone())
            }
            rows.push(scope);
            let pid = id();
            f.profiles.insert(ns.into(), pid.clone());
            let mut profile = row(
                "identity_profiles",
                [
                    ("id", pid.into()),
                    ("namespace", ns.into()),
                    ("version", "1".into()),
                    (
                        "scope_kind",
                        if ns == "koine:manual" {
                            "library"
                        } else {
                            "global"
                        }
                        .into(),
                    ),
                    (
                        "key_role",
                        if ns == "koine:manual" {
                            "manual"
                        } else {
                            "catalog"
                        }
                        .into(),
                    ),
                    ("rule_id", f.rules["identity"].clone().into()),
                ],
            );
            if ns != "koine:manual" {
                profile.set("catalog_namespace", ns)
            }
            rows.push(profile);
            for account in ["account-A", "account-B"] {
                let aid = id();
                f.authorities.insert(format!("{ns}/{account}"), aid.clone());
                let mut a = row(
                    "authority_scopes",
                    [
                        ("id", aid.into()),
                        ("authority_namespace", ns.into()),
                        ("subject_context_id", f.contexts[account].clone().into()),
                        (
                            "manager_context_id",
                            f.contexts[if ns == "fixture:epic" {
                                "manager-H"
                            } else {
                                "manager-S"
                            }]
                            .clone()
                            .into(),
                        ),
                        ("machine_context_id", f.contexts["machine"].clone().into()),
                    ],
                );
                if ns == "koine:manual" {
                    a.set("library_context_id", f.contexts["library"].clone())
                }
                a.set("scope_key", canonical::authority_key(&a));
                rows.push(a);
            }
        }
        f.store.write(rows).unwrap();
        f
    }
    pub fn assessment(&self, at: i64, cold: bool) -> Assessment {
        Assessment {
            at,
            observer_epoch: if cold { None } else { Some(self.epoch.clone()) },
            reconciliation_rule: self.rules["reconciliation"].clone(),
            freshness_rule: self.rules["freshness"].clone(),
        }
    }
    pub fn view(&self, at: i64) -> Library {
        self.store.reconstruct(&self.assessment(at, false)).unwrap()
    }
    pub fn report(
        &self,
        ns: &str,
        roots: &[(&str, &str)],
        outcome: &str,
        time: i64,
        revision: Option<u64>,
    ) -> Batch {
        self.report_account(ns, roots, outcome, time, revision, "account-A")
    }
    pub fn report_account(
        &self,
        ns: &str,
        roots: &[(&str, &str)],
        outcome: &str,
        time: i64,
        revision: Option<u64>,
        account: &str,
    ) -> Batch {
        let cid = id();
        let c = row(
            "collections",
            [
                ("id", cid.clone().into()),
                (
                    "kind",
                    if ns == "koine:manual" {
                        "manual"
                    } else {
                        "discovery"
                    }
                    .into(),
                ),
                ("reporter", "synthetic adapter".into()),
                ("reporter_version", "1".into()),
                (
                    "reporter_context_id",
                    self.contexts[if ns == "koine:manual" {
                        "library"
                    } else if ns == "fixture:epic" {
                        "manager-H"
                    } else {
                        "manager-S"
                    }]
                    .clone()
                    .into(),
                ),
                (
                    "interpretation_rule_id",
                    self.rules["interpretation"].clone().into(),
                ),
                ("started_at", time.into()),
                ("request_protocol", "koine.request.v1".into()),
            ],
        );
        let mut registration = vec![c];
        let mut rows = Vec::new();
        let mut parts = BTreeMap::new();
        for (root, status) in roots {
            for dim in [
                "entry_title",
                "entry_edition",
                "access",
                "presence",
                "completion",
                "location",
                "handoff_capability",
                "manual_visibility",
                "game_link",
                "game_label",
            ] {
                let uid = id();
                let rid = id();
                let pid = id();
                let aid = self.authorities[&format!("{ns}/{account}")].clone();
                registration.push(row(
                    "requested_units",
                    [
                        ("id", uid.clone().into()),
                        ("collection_id", cid.clone().into()),
                        ("unit_key", format!("{root}/{dim}").into()),
                        ("authority_scope_id", aid.clone().into()),
                        ("dimension", dim.into()),
                        ("root_context_id", self.contexts[*root].clone().into()),
                    ],
                ));
                let mut result = row(
                    "coverage_results",
                    [
                        ("id", rid.clone().into()),
                        ("unit_id", uid.clone().into()),
                        ("collection_id", cid.clone().into()),
                        ("dimension", dim.into()),
                        ("authority_scope_id", aid.clone().into()),
                        ("outcome", (*status).into()),
                    ],
                );
                if *status != "complete" {
                    result.set("reason", "synthetic gap")
                }
                rows.push(result);
                let mut part = row(
                    "coverage_parts",
                    [
                        ("id", pid.clone().into()),
                        ("coverage_id", rid.into()),
                        ("collection_id", cid.clone().into()),
                        ("unit_id", uid.into()),
                        ("dimension", dim.into()),
                        ("authority_scope_id", aid.into()),
                        ("part_key", "root".into()),
                        ("selector_kind", "whole_unit".into()),
                        (
                            "inspection",
                            if ["complete", "partial"].contains(status) {
                                "inspected"
                            } else {
                                "uninspected"
                            }
                            .into(),
                        ),
                        ("outcome", (*status).into()),
                    ],
                );
                if *status != "complete" {
                    part.set("reason", "synthetic gap")
                }
                rows.push(part);
                parts.insert(format!("{root}/{dim}"), pid);
            }
        }
        self.store.write(registration).unwrap();
        Batch {
            id: cid,
            ns: ns.into(),
            rows,
            parts,
            outcome: outcome.into(),
            time,
            revision,
        }
    }
    pub fn evidence(&self, b: &mut Batch) -> String {
        let eid = id();
        let mut e = row(
            "evidence_records",
            [
                ("id", eid.clone().into()),
                ("collection_id", b.id.clone().into()),
                (
                    "origin_kind",
                    if b.ns == "koine:manual" {
                        "user_assertion"
                    } else {
                        "external"
                    }
                    .into(),
                ),
                ("origin_namespace", b.ns.clone().into()),
                ("locator_kind", "opaque".into()),
                ("locator", b"synthetic fixture".to_vec().into()),
                ("collected_at", b.time.into()),
                ("source_effective_at", b.time.into()),
                (
                    "interpretation_rule_id",
                    self.rules["interpretation"].clone().into(),
                ),
                ("item_key", eid.clone().into()),
            ],
        );
        if let Some(rev) = b.revision {
            e.set("source_revision", format!("sequence:{rev}"))
        }
        b.rows.push(e);
        eid
    }
    pub fn entry(&mut self, b: &mut Batch, key: &str, title: &str) -> String {
        let compound = format!("{}/{key}", b.ns);
        let eid = self.evidence(b);
        let entry = if let Some(id) = self.entries.get(&compound) {
            id.clone()
        } else {
            let entry = id();
            let kid = id();
            self.entries.insert(compound.clone(), entry.clone());
            self.keys.insert(compound.clone(), kid.clone());
            b.rows.push(row(
                "source_entries",
                [
                    ("id", entry.clone().into()),
                    ("catalog_namespace", b.ns.clone().into()),
                    ("created_evidence_id", eid.clone().into()),
                ],
            ));
            b.rows.push(row(
                "entry_keys",
                [
                    ("id", kid.into()),
                    ("entry_id", entry.clone().into()),
                    ("namespace", b.ns.clone().into()),
                    ("scope_id", self.scopes[&b.ns].clone().into()),
                    (
                        "scope_kind",
                        if b.ns == "koine:manual" {
                            "library"
                        } else {
                            "global"
                        }
                        .into(),
                    ),
                    ("external_key", key.into()),
                    ("created_evidence_id", eid.clone().into()),
                ],
            ));
            entry
        };
        b.rows.push(row(
            "entry_key_observations",
            [
                ("id", id().into()),
                ("key_id", self.keys[&compound].clone().into()),
                ("reported_key", key.into()),
                ("profile_id", self.profiles[&b.ns].clone().into()),
                ("namespace", b.ns.clone().into()),
                (
                    "scope_kind",
                    if b.ns == "koine:manual" {
                        "library"
                    } else {
                        "global"
                    }
                    .into(),
                ),
                ("evidence_id", eid.into()),
                ("collection_id", b.id.clone().into()),
                (
                    "coverage_part_id",
                    b.parts["root-A/entry_title"].clone().into(),
                ),
                ("item_key", id().into()),
                ("basis", "explicit synthetic key".into()),
            ],
        ));
        self.claim(b, &entry, "entry_title", Some(title), "root-A", false);
        entry
    }
    pub fn entity(&mut self, b: &mut Batch, entry: &str, kind: &str, key: &str) -> String {
        let compound = format!("{entry}/{kind}/{key}");
        if let Some(id) = self.instances.get(&compound) {
            return id.clone();
        }
        let entity = id();
        self.instances.insert(compound, entity.clone());
        let eid = self.evidence(b);
        let mut r = row(
            kind,
            [
                ("id", entity.clone().into()),
                ("entry_id", entry.into()),
                ("created_evidence_id", eid.into()),
            ],
        );
        match kind {
            "entitlements" => {
                r.set(
                    "subject_context_id",
                    self.contexts[if key == "B" { "account-B" } else { "account-A" }].clone(),
                );
                r.set(
                    "authority_scope_id",
                    self.authorities[&format!(
                        "{}/{}",
                        b.ns,
                        if key == "B" { "account-B" } else { "account-A" }
                    )]
                        .clone(),
                );
                r.set("entitlement_key", "synthetic summary");
            }
            "installations" => {
                r.set(
                    "manager_context_id",
                    self.contexts[if b.ns == "fixture:epic" {
                        "manager-H"
                    } else {
                        "manager-S"
                    }]
                    .clone(),
                );
                r.set("machine_context_id", self.contexts["machine"].clone());
                r.set("instance_key", key);
            }
            "launch_targets" => {
                r.set(
                    "launcher_context_id",
                    self.contexts[if b.ns == "fixture:epic" {
                        "manager-H"
                    } else {
                        "manager-S"
                    }]
                    .clone(),
                );
                r.set("target_key", key);
            }
            _ => panic!("unknown fixture entity"),
        };
        b.rows.push(r);
        entity
    }
    pub fn claim(
        &mut self,
        b: &mut Batch,
        entity: &str,
        dim: &str,
        value: Option<&str>,
        root: &str,
        negative: bool,
    ) -> String {
        self.claim_account(b, entity, dim, value, root, negative, "account-A")
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "Fixture inputs keep independent evidence dimensions explicit"
    )]
    pub fn claim_account(
        &mut self,
        b: &mut Batch,
        entity: &str,
        dim: &str,
        value: Option<&str>,
        root: &str,
        negative: bool,
        account: &str,
    ) -> String {
        let aid = self.authorities[&format!("{}/{account}", b.ns)].clone();
        let key = format!("{entity}/{dim}/{aid}");
        let subject = if let Some(s) = self.subjects.get(&key) {
            s.clone()
        } else {
            let sid = id();
            self.subjects.insert(key, sid.clone());
            let field = match dim {
                "entry_title" | "entry_edition" | "manual_visibility" => "entry_id",
                "access" => "entitlement_id",
                "presence" | "completion" | "location" => "installation_id",
                "handoff_capability" => "target_id",
                "game_link" => "game_link_id",
                "game_label" => "game_id",
                _ => panic!("unsupported fixture dimension"),
            };
            b.rows.push(row(
                "claim_subjects",
                [
                    ("id", sid.clone().into()),
                    ("dimension", dim.into()),
                    ("authority_scope_id", aid.clone().into()),
                    (field, entity.into()),
                ],
            ));
            sid
        };
        let eid = self.evidence(b);
        let cid = id();
        let mut c = row(
            "claims",
            [
                ("id", cid.clone().into()),
                ("subject_id", subject.clone().into()),
                ("dimension", dim.into()),
                ("authority_scope_id", aid.clone().into()),
                ("evidence_id", eid.clone().into()),
                ("collection_id", b.id.clone().into()),
                (
                    "knowledge",
                    if negative {
                        "confirmed_negative"
                    } else if value.is_none() {
                        "unknown"
                    } else {
                        "observed"
                    }
                    .into(),
                ),
                ("basis_code", "synthetic explicit assertion".into()),
            ],
        );
        if b.ns != "koine:manual" {
            c.set(
                "coverage_part_id",
                b.parts[&format!("{root}/{dim}")].clone(),
            )
        }
        if let Some(v) = value {
            c.set("value_text", v)
        } else {
            c.set("reason", "unsupported fixture marker")
        }
        if negative {
            let nid = id();
            b.rows.push(row(
                "negative_justifications",
                [
                    ("id", nid.clone().into()),
                    ("subject_id", subject.into()),
                    ("dimension", dim.into()),
                    ("authority_scope_id", aid.into()),
                    ("evidence_id", eid.into()),
                    ("collection_id", b.id.clone().into()),
                    ("kind", "explicit".into()),
                    (
                        "capability_rule_id",
                        self.rules["interpretation"].clone().into(),
                    ),
                    ("basis", "synthetic explicit negative".into()),
                ],
            ));
            c.set("negative_justification_id", nid)
        }
        b.rows.push(c);
        cid
    }
    pub fn finish(&self, mut b: Batch) -> Vec<Record> {
        b.rows.push(row(
            "collection_closures",
            [
                ("collection_id", b.id.clone().into()),
                ("outcome", b.outcome.into()),
                ("finished_at", b.time.into()),
                ("reason", "synthetic result".into()),
                ("result_protocol", "koine.result.v1".into()),
            ],
        ));
        self.store.write(b.rows.clone()).unwrap();
        b.rows
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "Fixture inputs keep independent evidence dimensions explicit"
    )]
    pub fn observe(
        &mut self,
        ns: &str,
        key: &str,
        installation: Option<&str>,
        completion: Option<&str>,
        access: Option<&str>,
        capability: Option<&str>,
        revision: u64,
    ) -> (String, Option<String>, Option<String>, Option<String>) {
        let mut b = self.report(
            ns,
            &[("root-A", "complete")],
            "complete",
            revision as i64,
            Some(revision),
        );
        let e = self.entry(&mut b, key, "Meridian");
        let installation = installation.map(|v| {
            let i = self.entity(&mut b, &e, "installations", "root-A");
            self.claim(&mut b, &i, "presence", Some(v), "root-A", false);
            if let Some(c) = completion {
                self.claim(&mut b, &i, "completion", Some(c), "root-A", false);
            }
            i
        });
        let access = access.map(|v| {
            let a = self.entity(&mut b, &e, "entitlements", "A");
            self.claim(&mut b, &a, "access", Some(v), "root-A", false);
            a
        });
        let target = capability.map(|v| {
            let t = self.entity(&mut b, &e, "launch_targets", "default");
            self.claim(&mut b, &t, "handoff_capability", Some(v), "root-A", false);
            t
        });
        self.finish(b);
        (e, installation, access, target)
    }
    pub fn intent(&self, target: &str) -> Record {
        row(
            "launch_attempts",
            [
                ("id", id().into()),
                ("target_id", target.into()),
                ("created_at", 20i64.into()),
                ("intent_reason", "synthetic user request".into()),
            ],
        )
    }
    pub fn handoff_record(&self, attempt: &str) -> Record {
        row(
            "handoff_records",
            [
                ("id", id().into()),
                ("attempt_id", attempt.into()),
                ("item_key", id().into()),
                ("recorded_at", 21i64.into()),
                ("transport_rule_id", self.rules["transport"].clone().into()),
                ("boundary", "fixture transport".into()),
                ("issuance", "unknown".into()),
                ("disposition", "inconclusive".into()),
                ("reason", "synthetic boundary".into()),
            ],
        )
    }
    pub fn process(&self, attempt: &str, state: &str, prior: Option<&str>, time: i64) -> Record {
        let mut p = row(
            "process_observations",
            [
                ("id", id().into()),
                ("attempt_id", attempt.into()),
                ("item_key", id().into()),
                ("observed_at", time.into()),
                (
                    "method_rule_id",
                    self.rules["interpretation"].clone().into(),
                ),
                ("observer_epoch", self.epoch.clone().into()),
                ("state", state.into()),
            ],
        );
        if state == "inconclusive" {
            p.set("reason", "synthetic observer lost continuity")
        } else {
            p.set("identity_token", b"process-identity-A".to_vec());
            p.set("identity_basis", "synthetic identity beyond PID");
            p.set("pid", 42i64)
        }
        if let Some(prior) = prior {
            p.set("prior_observed_id", prior);
            p.set("prior_state", "observed")
        }
        p
    }
    pub fn conclusion(&self, entity: &str, dim: &str, at: i64) -> Conclusion {
        let snap = self.store.snapshot().unwrap();
        self.view(at)
            .claims
            .into_iter()
            .find(|c| {
                c.dimension == dim
                    && snap
                        .by_id("claim_subjects", &c.subject.clone().into())
                        .unwrap()
                        .fields
                        .values()
                        .any(|v| v == &Value::from(entity))
            })
            .expect("conclusion")
    }
}
pub struct FakeTransport {
    pub calls: Vec<(String, String)>,
    pub disposition: &'static str,
    pub status: Option<i64>,
}
impl FakeTransport {
    pub fn new(disposition: &'static str) -> Self {
        Self {
            calls: Vec::new(),
            disposition,
            status: None,
        }
    }
}
impl Transport for FakeTransport {
    fn submit(&mut self, target: &str, launcher: &str) -> Result<TransportResult> {
        self.calls.push((target.into(), launcher.into()));
        Ok(TransportResult {
            issuance: "issued".into(),
            disposition: self.disposition.into(),
            status: self.status,
            boundary: "fixture transport".into(),
            reason: Some("synthetic transport result".into()),
        })
    }
}

//! Deterministic reconstruction and planning, independent of storage and GTK.
use crate::validation::{referenced, source_order};
pub use crate::validation::{
    supported_rule, synthetic_artifact, validate, validate_decision_content,
};
use crate::*;
use std::collections::{BTreeMap, BTreeSet};

pub fn reconstruct(s: &Snapshot, inputs: &Assessment) -> Library {
    let mut out = Library::default();
    let policy_error = if supported_rule(
        s,
        &inputs.reconciliation_rule.clone().into(),
        "reconciliation",
    )
    .is_err()
        || supported_rule(s, &inputs.freshness_rule.clone().into(), "freshness").is_err()
    {
        Some("unsupported assessment policy".to_string())
    } else {
        None
    };
    let invalid = policy_error.or_else(|| validate(s).err());
    if let Some(ref error) = invalid {
        out.diagnostics.push(error.clone())
    }
    for subject in s.rows("claim_subjects") {
        let mut claims: Vec<_> = s
            .rows("claims")
            .filter(|c| c.get("subject_id") == subject.get("id"))
            .collect();
        claims.sort_by_key(|c| c.get("id").clone());
        let last_known_claims = claims
            .iter()
            .filter(|c| !c.is("knowledge", "unknown"))
            .filter_map(|c| c.text("id").ok().map(String::from))
            .collect();
        let surviving: Vec<_> = claims
            .iter()
            .filter(|c| {
                !s.rows("supersessions")
                    .any(|edge| edge.get("older_claim_id") == c.get("id"))
            })
            .copied()
            .collect();
        let known: Vec<_> = surviving
            .iter()
            .filter(|c| !c.is("knowledge", "unknown"))
            .copied()
            .collect();
        let values: BTreeSet<_> = known
            .iter()
            .map(|c| {
                (
                    c.get("knowledge").clone(),
                    if c.get("value_bytes") != &Value::Null {
                        c.get("value_bytes").clone()
                    } else {
                        c.get("value_text").clone()
                    },
                )
            })
            .collect();
        let knowledge = if invalid.is_some() || values.is_empty() {
            Knowledge::Unknown
        } else if values.len() > 1 {
            Knowledge::Disputed
        } else {
            let (k, v) = values.first().expect("nonempty values");
            if k == &Value::from("confirmed_negative") {
                Knowledge::ConfirmedNegative(v.clone())
            } else {
                Knowledge::Observed(v.clone())
            }
        };
        let mut freshness = Freshness::Current;
        if known.is_empty() {
            freshness = Freshness::Unknown
        }
        for c in &known {
            if let Ok(e) = s.by_id("evidence_records", c.get("evidence_id")) {
                if let Value::Integer(until) = e.get("valid_until") {
                    if inputs.at > *until {
                        freshness = Freshness::Stale
                    }
                } else if e.get("source_effective_at") == &Value::Null
                    && freshness != Freshness::Stale
                {
                    freshness = Freshness::Unknown
                }
            }
        }
        out.claims.push(Conclusion {
            subject: subject.text("id").unwrap_or_default().into(),
            dimension: subject.text("dimension").unwrap_or_default().into(),
            authority: subject
                .text("authority_scope_id")
                .unwrap_or_default()
                .into(),
            knowledge,
            freshness,
            supporting_claims: surviving
                .iter()
                .filter_map(|c| c.text("id").ok().map(String::from))
                .collect(),
            last_known_claims,
            diagnostics: invalid.iter().cloned().collect(),
        });
    }
    for attempt in s.rows("launch_attempts") {
        let records: Vec<_> = s
            .rows("handoff_records")
            .filter(|h| h.get("attempt_id") == attempt.get("id"))
            .collect();
        let terminal: Vec<_> = records
            .iter()
            .filter(|h| !h.is("disposition", "pending"))
            .copied()
            .collect();
        let dispositions: BTreeSet<_> = terminal
            .iter()
            .map(|h| {
                (
                    h.get("disposition").clone(),
                    h.get("boundary").clone(),
                    h.get("issuance").clone(),
                    h.get("exit_status").clone(),
                )
            })
            .collect();
        let mut handoff = Handoff::Inconclusive;
        let mut issuance = "unknown".to_string();
        let mut exit_status = None;
        if invalid.is_none() && dispositions.len() == 1 {
            let h = terminal[0];
            handoff = match h.text("disposition").unwrap_or("") {
                "accepted" => Handoff::Accepted,
                "failed" => Handoff::Failed,
                "not_requested" => Handoff::NotRequested,
                _ => Handoff::Inconclusive,
            };
            issuance = h.text("issuance").unwrap_or("unknown").into();
            exit_status = h.int("exit_status").ok();
        } else if invalid.is_none() && terminal.is_empty() && !records.is_empty() {
            handoff = if inputs.observer_epoch.is_some() {
                Handoff::Pending
            } else {
                Handoff::Inconclusive
            };
            issuance = "issued".into()
        }
        // Inconclusive may be resolved by a later definitive record at the same boundary.
        if terminal.len() > 1 && invalid.is_none() {
            let definitive: Vec<_> = terminal
                .iter()
                .filter(|h| h.is("disposition", "accepted") || h.is("disposition", "failed"))
                .copied()
                .collect();
            if definitive.len() == 1
                && terminal.iter().all(|h| {
                    h.get("boundary") == definitive[0].get("boundary")
                        && (h.is("disposition", "inconclusive")
                            || h.get("id") == definitive[0].get("id"))
                })
            {
                let h = definitive[0];
                handoff = if h.is("disposition", "failed") {
                    Handoff::Failed
                } else {
                    Handoff::Accepted
                };
                issuance = h.text("issuance").unwrap_or("unknown").into();
                exit_status = h.int("exit_status").ok();
            }
        }
        let observations: Vec<_> = s
            .rows("process_observations")
            .filter(|p| p.get("attempt_id") == attempt.get("id"))
            .collect();
        let mut process = if observations.is_empty() {
            Process::Unobserved
        } else {
            Process::Inconclusive
        };
        if invalid.is_none()
            && let Some(epoch) = &inputs.observer_epoch
        {
            let current: Vec<_> = observations
                .iter()
                .filter(|p| p.is("observer_epoch", epoch))
                .copied()
                .collect();
            let last = current
                .iter()
                .max_by_key(|p| p.int("observed_at").unwrap_or(i64::MIN));
            if let Some(p) = last {
                let ties: Vec<_> = current
                    .iter()
                    .filter(|q| q.get("observed_at") == p.get("observed_at"))
                    .collect();
                if ties.iter().all(|q| {
                    q.get("state") == p.get("state")
                        && q.get("identity_token") == p.get("identity_token")
                }) {
                    process = match p.text("state").unwrap_or("") {
                        "observed" => Process::Observed,
                        "exited" => Process::Exited,
                        _ => Process::Inconclusive,
                    }
                }
            }
        }
        out.attempts.push(AttemptConclusion {
            attempt: attempt.text("id").unwrap_or_default().into(),
            target: attempt.text("target_id").unwrap_or_default().into(),
            handoff,
            process,
            exit_status,
            issuance,
        });
    }
    out.claims.sort_by(|a, b| a.subject.cmp(&b.subject));
    out.attempts.sort_by(|a, b| a.attempt.cmp(&b.attempt));
    for collection in s.rows("collections") {
        let closure = s
            .find("collection_closures", "collection_id", collection.get("id"))
            .ok();
        let mut limitations: Vec<_> = s
            .rows("diagnostics")
            .filter(|d| d.get("collection_id") == collection.get("id"))
            .filter_map(|d| d.text("code").ok().map(String::from))
            .collect();
        if closure.is_none() {
            limitations.push("collection remains incomplete after recovery".into())
        }
        limitations.extend(
            s.rows("coverage_results")
                .filter(|r| {
                    r.get("collection_id") == collection.get("id") && !r.is("outcome", "complete")
                })
                .filter_map(|r| r.text("reason").ok().map(String::from)),
        );
        limitations.sort();
        out.collections.push(CollectionConclusion {
            collection: collection.text("id").unwrap_or_default().into(),
            outcome: closure
                .and_then(|r| r.text("outcome").ok())
                .unwrap_or("inconclusive")
                .into(),
            limitations,
        });
    }
    out.collections
        .sort_by(|a, b| a.collection.cmp(&b.collection));
    let mut remaining: BTreeSet<_> = s
        .rows("source_entries")
        .filter_map(|r| r.text("id").ok().map(String::from))
        .collect();
    let mut equivalent = Vec::new();
    let mut different = Vec::new();
    for c in out
        .claims
        .iter()
        .filter(|c| c.dimension == "entry_equivalence")
    {
        if let Ok(subject) = s.by_id("claim_subjects", &c.subject.clone().into())
            && let Ok(pair) = s.by_id("entry_equivalences", subject.get("equivalence_id"))
        {
            let edge = (
                pair.text("left_entry_id").unwrap_or_default().to_string(),
                pair.text("right_entry_id").unwrap_or_default().to_string(),
            );
            if c.knowledge == Knowledge::Observed("equivalent".into()) {
                equivalent.push(edge)
            } else if c.knowledge == Knowledge::Observed("different".into())
                || c.knowledge == Knowledge::Disputed
            {
                different.push(edge)
            }
        }
    }
    while let Some(start) = remaining.pop_first() {
        let mut members = BTreeSet::from([start]);
        loop {
            let before = members.len();
            for (a, b) in &equivalent {
                if members.contains(a) || members.contains(b) {
                    members.insert(a.clone());
                    members.insert(b.clone());
                }
            }
            if before == members.len() {
                break;
            }
        }
        for member in &members {
            remaining.remove(member);
        }
        let disputed = different
            .iter()
            .any(|(a, b)| members.contains(a) && members.contains(b));
        out.identity_components.push(IdentityComponent {
            handles: members.into_iter().collect(),
            disputed,
        });
    }
    // Conflicting governing organizational links stay disputed rather than choosing a game.
    let mut grouped: BTreeMap<Value, Vec<usize>> = BTreeMap::new();
    for (i, c) in out.claims.iter().enumerate().filter(|(_, c)| {
        c.dimension == "game_link" && c.knowledge == Knowledge::Observed(Value::from("linked"))
    }) {
        if let Ok(subject) = s.by_id("claim_subjects", &c.subject.clone().into())
            && let Ok(link) = s.by_id("game_links", subject.get("game_link_id"))
        {
            let entry = link.text("entry_id").unwrap_or_default();
            let component = out
                .identity_components
                .iter()
                .find(|c| c.handles.iter().any(|h| h == entry));
            let key = component
                .filter(|c| !c.disputed)
                .and_then(|c| c.handles.first())
                .map_or_else(|| link.get("entry_id").clone(), |h| h.clone().into());
            grouped.entry(key).or_default().push(i)
        }
    }
    for indices in grouped.values().filter(|v| v.len() > 1) {
        let games: BTreeSet<_> = indices
            .iter()
            .filter_map(|i| {
                s.by_id("claim_subjects", &out.claims[*i].subject.clone().into())
                    .ok()
            })
            .filter_map(|subject| s.by_id("game_links", subject.get("game_link_id")).ok())
            .map(|link| link.get("game_id").clone())
            .collect();
        if games.len() < 2 {
            continue;
        }
        for i in indices {
            out.claims[*i].knowledge = Knowledge::Disputed
        }
    }
    out
}

pub fn active_entries(s: &Snapshot, library: &Library, library_context: &str) -> Vec<String> {
    s.rows("source_entries")
        .filter(|entry| {
            !library.claims.iter().any(|c| {
                c.dimension == "manual_visibility"
                    && s.by_id("authority_scopes", &c.authority.clone().into())
                        .is_ok_and(|a| a.is("library_context_id", library_context))
                    && c.knowledge == Knowledge::Observed(Value::from("hidden"))
                    && s.by_id("claim_subjects", &c.subject.clone().into())
                        .is_ok_and(|sub| sub.get("entry_id") == entry.get("id"))
            })
        })
        .filter_map(|e| e.text("id").ok().map(String::from))
        .collect()
}

/// Plan justified supersessions or disputes for newly supplied known claims.
/// Source sequence is meaningful only under the explicit synthetic ordering rule.
/// Manual causal actions must supply their explicit decision separately.
pub fn plan(s: &Snapshot, new_ids: &BTreeSet<Value>) -> Result<Vec<Record>> {
    let mut rows = Vec::new();
    let mut seen = BTreeSet::new();
    let rule = |kind: &str| -> Result<Value> {
        Ok(s.rows("rule_versions")
            .find(|r| r.is("kind", kind))
            .ok_or("missing reconciliation rule")?
            .get("id")
            .clone())
    };
    for a in s
        .rows("claims")
        .filter(|c| new_ids.contains(c.get("id")) && !c.is("knowledge", "unknown"))
    {
        for b in s.rows("claims").filter(|c| {
            c.get("subject_id") == a.get("subject_id")
                && c.get("id") != a.get("id")
                && !c.is("knowledge", "unknown")
        }) {
            let pair = if a.get("id") < b.get("id") {
                (a.get("id").clone(), b.get("id").clone())
            } else {
                (b.get("id").clone(), a.get("id").clone())
            };
            if !seen.insert(pair) {
                continue;
            }
            if s.rows("reconciliation_decisions").any(|d| {
                s.rows("decision_claims").any(|l| {
                    l.get("decision_id") == d.get("id") && l.get("claim_id") == a.get("id")
                }) && s.rows("decision_claims").any(|l| {
                    l.get("decision_id") == d.get("id") && l.get("claim_id") == b.get("id")
                })
            }) {
                continue;
            }
            let did = id();
            let mut decision = Record::new(
                "reconciliation_decisions",
                [
                    ("id", did.clone().into()),
                    ("kind", "supersession".into()),
                    ("rule_id", rule("reconciliation")?),
                    ("ordering_rule_id", rule("ordering")?),
                    (
                        "assessed_at",
                        referenced(s, a, "evidence_id", "evidence_records")?
                            .get("collected_at")
                            .clone(),
                    ),
                    ("reason", "synthetic source sequence".into()),
                    ("collection_id", a.get("collection_id").clone()),
                ],
            );
            let ordered = if source_order(s, b, a, &decision).is_ok() {
                Some((b, a))
            } else if source_order(s, a, b, &decision).is_ok() {
                Some((a, b))
            } else {
                None
            };
            if let Some((old, new)) = ordered {
                rows.push(decision);
                for (role, c) in [("older", old), ("replacement", new)] {
                    rows.push(Record::new(
                        "decision_claims",
                        [
                            ("decision_id", did.clone().into()),
                            ("claim_id", c.get("id").clone()),
                            ("role", role.into()),
                        ],
                    ));
                }
                if new.is("knowledge", "confirmed_negative") {
                    rows.push(Record::new(
                        "decision_claims",
                        [
                            ("decision_id", did.clone().into()),
                            ("claim_id", new.get("id").clone()),
                            ("role", "negative_support".into()),
                        ],
                    ));
                }
                rows.push(Record::new(
                    "supersessions",
                    [
                        ("older_claim_id", old.get("id").clone()),
                        ("replacement_claim_id", new.get("id").clone()),
                        ("dimension", old.get("dimension").clone()),
                        ("decision_id", did.clone().into()),
                    ],
                ));
            } else {
                if a.get("knowledge") == b.get("knowledge")
                    && a.get("value_text") == b.get("value_text")
                    && a.get("value_bytes") == b.get("value_bytes")
                {
                    continue;
                }
                decision.set("kind", "dispute");
                decision.set("ordering_rule_id", Value::Null);
                decision.set("reason", "unordered incompatible evidence");
                rows.push(decision);
                for c in [a, b] {
                    rows.push(Record::new(
                        "decision_claims",
                        [
                            ("decision_id", did.clone().into()),
                            ("claim_id", c.get("id").clone()),
                            ("role", "conflicting".into()),
                        ],
                    ));
                }
            }
            rows.push(Record::new(
                "decision_seals",
                [
                    ("decision_id", did.into()),
                    ("protocol", "koine.decision.v1".into()),
                ],
            ));
        }
    }
    Ok(rows)
}

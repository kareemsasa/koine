//! Traceable offline translations of docs/acceptance-cases.md. No real launcher.
mod common;
use common::*;
use koine_core::{reconcile, *};
use koine_sqlite::Store;

#[test]
fn multiple_store_entitlements() {
    let mut f = Fixture::new();
    let (steam, _, _, target) = f.observe(
        "fixture:steam",
        "1549690",
        Some("present"),
        Some("complete"),
        Some("accessible"),
        Some("supported"),
        1,
    );
    let (epic, _, _, _) = f.observe(
        "fixture:epic",
        "epic-key",
        None,
        None,
        Some("accessible"),
        None,
        1,
    );
    let mut b = f.report(
        "koine:manual",
        &[("root-A", "complete")],
        "complete",
        3,
        None,
    );
    let evidence = f.evidence(&mut b);
    let game = id();
    b.rows.push(row(
        "games",
        [
            ("id", game.clone().into()),
            ("created_evidence_id", evidence.into()),
        ],
    ));
    let mut link_ids = Vec::new();
    for e in [&steam, &epic] {
        let link = id();
        b.rows.push(row(
            "game_links",
            [
                ("id", link.clone().into()),
                ("game_id", game.clone().into()),
                ("entry_id", e.clone().into()),
            ],
        ));
        f.claim(&mut b, &link, "game_link", Some("linked"), "root-A", false);
        link_ids.push(link)
    }
    f.finish(b);
    let snap = f.store.snapshot().unwrap();
    assert_eq!(snap.rows("games").count(), 1);
    assert_eq!(snap.rows("entitlements").count(), 2);
    assert_eq!(snap.rows("installations").count(), 1);
    assert_eq!(snap.rows("launch_targets").count(), 1);
    let intent = f.intent(target.as_ref().unwrap());
    let result = f.handoff_record(intent.text("id").unwrap());
    let mut fake = FakeTransport::new("accepted");
    f.store.handoff(intent, result, &mut fake).unwrap();
    assert_eq!(fake.calls[0].1, f.contexts["manager-S"]);
    // Explicit unlink retains source facts; do not manufacture a different identity.
    let mut b = f.report(
        "koine:manual",
        &[("root-A", "complete")],
        "complete",
        4,
        None,
    );
    let new = f.claim(
        &mut b,
        &link_ids[0],
        "game_link",
        Some("unlinked"),
        "root-A",
        false,
    );
    manual_replace(&f, &mut b, &link_ids[0], "game_link", &new);
    f.finish(b);
    assert_eq!(
        f.store.snapshot().unwrap().rows("source_entries").count(),
        2
    );
    assert_eq!(f.store.snapshot().unwrap().rows("entitlements").count(), 2);
}
#[test]
fn editions_and_title_collision() {
    let mut f = Fixture::new();
    let mut b = f.report(
        "fixture:steam",
        &[("root-A", "complete")],
        "complete",
        1,
        Some(1),
    );
    let a = f.entry(&mut b, "base", "Same Title");
    let z = f.entry(&mut b, "remaster", "Same Title");
    f.claim(&mut b, &a, "entry_edition", Some("base"), "root-A", false);
    f.claim(
        &mut b,
        &z,
        "entry_edition",
        Some("remaster"),
        "root-A",
        false,
    );
    f.finish(b);
    let s = f.store.snapshot().unwrap();
    assert_ne!(a, z);
    assert_eq!(s.rows("source_entries").count(), 2);
    assert_eq!(s.rows("entry_equivalences").count(), 0);
    assert_eq!(
        f.conclusion(&a, "entry_edition", 2).knowledge,
        Knowledge::Observed("base".into())
    );
}
#[test]
fn heroic_managed_installation() {
    let mut f = Fixture::new();
    let (_, i, _, t) = f.observe(
        "fixture:epic",
        "epic-key",
        Some("present"),
        Some("complete"),
        None,
        Some("supported"),
        1,
    );
    assert!(i.is_some());
    let intent = f.intent(t.as_ref().unwrap());
    let result = f.handoff_record(intent.text("id").unwrap());
    let mut fake = FakeTransport::new("accepted");
    f.store.handoff(intent, result, &mut fake).unwrap();
    assert_eq!(fake.calls[0].1, f.contexts["manager-H"]);
    assert_eq!(f.store.snapshot().unwrap().rows("entitlements").count(), 0);
    assert_eq!(f.view(2).attempts[0].process, Process::Unobserved);
}
#[test]
fn heroic_handoff_unavailable() {
    let mut f = Fixture::new();
    let (_, i, _, t) = f.observe(
        "fixture:epic",
        "epic-key",
        Some("present"),
        Some("complete"),
        None,
        Some("unavailable"),
        1,
    );
    let intent = f.intent(t.as_ref().unwrap());
    let result = f.handoff_record(intent.text("id").unwrap());
    let mut fake = FakeTransport::new("accepted");
    f.store.handoff(intent, result, &mut fake).unwrap();
    assert!(fake.calls.is_empty());
    assert!(i.is_some());
    assert_eq!(f.view(2).attempts[0].handoff, Handoff::NotRequested);
}
#[test]
fn steam_installation_incomplete() {
    let mut f = Fixture::new();
    let (_, i, _, _) = f.observe(
        "fixture:steam",
        "app",
        Some("present"),
        Some("incomplete"),
        None,
        None,
        1,
    );
    assert_eq!(
        f.conclusion(i.as_ref().unwrap(), "completion", 2).knowledge,
        Knowledge::Observed("incomplete".into())
    );
    assert_eq!(f.store.snapshot().unwrap().rows("entitlements").count(), 0);
}
#[test]
fn steam_completion_unknown() {
    let mut f = Fixture::new();
    let mut b = f.report(
        "fixture:steam",
        &[("root-A", "partial")],
        "partial",
        1,
        Some(1),
    );
    let e = f.entry(&mut b, "app", "Meridian");
    let i = f.entity(&mut b, &e, "installations", "root-A");
    f.claim(&mut b, &i, "presence", Some("present"), "root-A", false);
    let c = f.claim(&mut b, &i, "completion", None, "root-A", false);
    b.rows.push(row(
        "diagnostics",
        [
            ("id", id().into()),
            ("collection_id", b.id.clone().into()),
            ("item_key", id().into()),
            ("target_kind", "claim".into()),
            ("claim_id", c.into()),
            ("severity", "warning".into()),
            ("code", "unsupported-marker".into()),
            ("message", "synthetic completion marker unknown".into()),
            ("recorded_at", 1i64.into()),
        ],
    ));
    f.finish(b);
    assert_eq!(
        f.conclusion(&i, "completion", 2).knowledge,
        Knowledge::Unknown
    );
    assert_eq!(
        f.conclusion(&i, "presence", 2).knowledge,
        Knowledge::Observed("present".into())
    );
}
#[test]
fn incomplete_source_scan() {
    let (mut f, i_a, i_b) = two_roots();
    let mut b = f.report(
        "fixture:steam",
        &[("root-A", "complete"), ("root-B", "inconclusive")],
        "partial",
        2,
        Some(2),
    );
    f.claim(&mut b, &i_a, "presence", Some("present"), "root-A", false);
    f.finish(b);
    assert_eq!(
        f.conclusion(&i_b, "presence", 3).knowledge,
        Knowledge::Observed("present".into())
    );
    let s = f.store.snapshot().unwrap();
    let c = &f.conclusion(&i_b, "presence", 3).supporting_claims[0];
    let e = s
        .by_id(
            "evidence_records",
            s.by_id("claims", &c.clone().into())
                .unwrap()
                .get("evidence_id"),
        )
        .unwrap();
    assert_eq!(e.int("collected_at").unwrap(), 1);
}
#[test]
fn failed_and_inconclusive_empty_results() {
    let mut f = Fixture::new();
    let (_, i, a, _) = f.observe(
        "fixture:steam",
        "app",
        Some("present"),
        None,
        Some("accessible"),
        None,
        1,
    );
    for outcome in ["failed", "inconclusive", "inconclusive"] {
        let b = f.report("fixture:steam", &[("root-A", outcome)], outcome, 2, None);
        f.finish(b);
    }
    assert_eq!(
        f.conclusion(i.as_ref().unwrap(), "presence", 3).knowledge,
        Knowledge::Observed("present".into())
    );
    assert_eq!(
        f.conclusion(a.as_ref().unwrap(), "access", 3).knowledge,
        Knowledge::Observed("accessible".into())
    );
}
#[test]
fn complete_scan_without_negative_authority() {
    let mut f = Fixture::new();
    let (_, _, a, _) = f.observe(
        "fixture:steam",
        "app",
        None,
        None,
        Some("accessible"),
        None,
        1,
    );
    let b = f.report(
        "fixture:steam",
        &[("root-A", "complete")],
        "complete",
        2,
        Some(2),
    );
    f.finish(b);
    assert_eq!(
        f.conclusion(a.as_ref().unwrap(), "access", 3).knowledge,
        Knowledge::Observed("accessible".into())
    );
    assert_eq!(
        f.store
            .snapshot()
            .unwrap()
            .rows("negative_justifications")
            .count(),
        0
    );
}
#[test]
fn supported_bounded_absence() {
    let (mut f, i_a, i_b) = two_roots();
    let mut b = f.report(
        "fixture:steam",
        &[("root-A", "complete")],
        "complete",
        2,
        Some(2),
    );
    let c = f.claim(&mut b, &i_a, "presence", Some("absent"), "root-A", true);
    let nid = b
        .rows
        .iter()
        .find(|r| r.table == "claims" && r.is("id", &c))
        .unwrap()
        .get("negative_justification_id")
        .clone();
    let proof = b
        .rows
        .iter_mut()
        .find(|r| r.table == "negative_justifications" && r.get("id") == &nid)
        .unwrap();
    proof.set("kind", "exhaustive_omission");
    proof.set("coverage_part_id", b.parts["root-A/presence"].clone());
    proof.set("required_outcome", "complete");
    proof.set("basis", "synthetic exhaustive enumeration");
    f.finish(b);
    assert_eq!(
        f.conclusion(&i_a, "presence", 3).knowledge,
        Knowledge::ConfirmedNegative("absent".into())
    );
    assert_eq!(
        f.conclusion(&i_b, "presence", 3).knowledge,
        Knowledge::Observed("present".into())
    );
    assert!(
        !f.store
            .snapshot()
            .unwrap()
            .rows("supersessions")
            .collect::<Vec<_>>()
            .is_empty()
    );
    let mut bad = f.report(
        "fixture:steam",
        &[("root-A", "partial")],
        "partial",
        3,
        Some(3),
    );
    let claim = f.claim(&mut bad, &i_b, "presence", Some("absent"), "root-A", true);
    let nid = bad
        .rows
        .iter()
        .find(|r| r.is("id", &claim))
        .unwrap()
        .get("negative_justification_id")
        .clone();
    let p = bad
        .rows
        .iter_mut()
        .find(|r| r.table == "negative_justifications" && r.get("id") == &nid)
        .unwrap();
    p.set("kind", "exhaustive_omission");
    p.set("coverage_part_id", bad.parts["root-A/presence"].clone());
    p.set("required_outcome", "complete");
    p.set("basis", "synthetic exhaustive enumeration");
    assert!(f.store.write(bad.rows).is_err());
}
#[test]
fn stale_ownership_evidence() {
    let mut f = Fixture::new();
    let mut b = f.report(
        "fixture:steam",
        &[("root-A", "complete")],
        "complete",
        1,
        Some(1),
    );
    let e = f.entry(&mut b, "app", "Meridian");
    let a = f.entity(&mut b, &e, "entitlements", "A");
    let c = f.claim(&mut b, &a, "access", Some("accessible"), "root-A", false);
    let eid = b
        .rows
        .iter()
        .find(|r| r.is("id", &c))
        .unwrap()
        .get("evidence_id")
        .clone();
    b.rows
        .iter_mut()
        .find(|r| r.table == "evidence_records" && r.get("id") == &eid)
        .unwrap()
        .set("valid_until", 2i64);
    f.finish(b);
    let b = f.report("fixture:steam", &[("root-A", "failed")], "failed", 3, None);
    f.finish(b);
    let mut cached = f.report(
        "fixture:steam",
        &[("root-A", "complete")],
        "complete",
        4,
        Some(1),
    );
    let copy = f.claim(
        &mut cached,
        &a,
        "access",
        Some("accessible"),
        "root-A",
        false,
    );
    let evidence = cached
        .rows
        .iter()
        .find(|r| r.is("id", &copy))
        .unwrap()
        .get("evidence_id")
        .clone();
    let source = cached
        .rows
        .iter_mut()
        .find(|r| r.table == "evidence_records" && r.get("id") == &evidence)
        .unwrap();
    source.set("source_effective_at", 1i64);
    source.set("valid_until", 2i64);
    f.finish(cached);
    assert_eq!(f.conclusion(&a, "access", 4).freshness, Freshness::Stale);
    assert_eq!(
        f.conclusion(&a, "access", 4).knowledge,
        Knowledge::Observed("accessible".into())
    );
}
#[test]
fn independent_source_and_account_authority() {
    let mut f = Fixture::new();
    let (steam, _, a, _) = f.observe(
        "fixture:steam",
        "same",
        None,
        None,
        Some("accessible"),
        None,
        1,
    );
    let mut b = f.report_account(
        "fixture:steam",
        &[("root-A", "complete")],
        "complete",
        2,
        Some(2),
        "account-B",
    );
    let a_b = f.entity(&mut b, &steam, "entitlements", "B");
    f.claim_account(
        &mut b,
        &a_b,
        "access",
        Some("inaccessible"),
        "root-A",
        true,
        "account-B",
    );
    f.finish(b);
    let (epic, _, _, _) = f.observe("fixture:epic", "same", Some("present"), None, None, None, 1);
    assert_eq!(
        f.conclusion(a.as_ref().unwrap(), "access", 3).knowledge,
        Knowledge::Observed("accessible".into())
    );
    assert_eq!(
        f.conclusion(&a_b, "access", 3).knowledge,
        Knowledge::ConfirmedNegative("inaccessible".into())
    );
    assert_ne!(steam, epic);
}
#[test]
fn duplicate_delayed_and_conflicting_results() {
    let mut f = Fixture::new();
    let (e, i, _, _) = f.observe("fixture:steam", "app", Some("present"), None, None, None, 2);
    let i = i.unwrap();
    let mut b = f.report(
        "fixture:steam",
        &[("root-A", "complete")],
        "complete",
        1,
        Some(1),
    );
    f.claim(&mut b, &i, "presence", Some("absent"), "root-A", true);
    let rows = f.finish(b);
    assert_eq!(
        f.conclusion(&i, "presence", 4).knowledge,
        Knowledge::Observed("present".into())
    );
    let n = f.store.snapshot().unwrap().records.len();
    f.store.write(rows).unwrap();
    assert_eq!(f.store.snapshot().unwrap().records.len(), n);
    let mut b = f.report(
        "fixture:steam",
        &[("root-A", "complete")],
        "complete",
        3,
        None,
    );
    f.claim(&mut b, &i, "presence", Some("absent"), "root-A", true);
    f.finish(b);
    assert_eq!(
        f.conclusion(&i, "presence", 4).knowledge,
        Knowledge::Disputed
    );
    assert!(
        f.store
            .snapshot()
            .unwrap()
            .rows("source_entries")
            .any(|r| r.is("id", &e))
    );
}
#[test]
fn cancelled_and_changing_scan() {
    let mut f = Fixture::new();
    let mut b = f.report(
        "fixture:steam",
        &[("root-A", "partial")],
        "partial",
        1,
        Some(1),
    );
    let e = f.entry(&mut b, "app", "Meridian");
    let i = f.entity(&mut b, &e, "installations", "root-A");
    f.claim(&mut b, &i, "presence", Some("present"), "root-A", false);
    f.finish(b);
    let b = f.report(
        "fixture:steam",
        &[("root-A", "inconclusive")],
        "inconclusive",
        2,
        None,
    );
    f.finish(b);
    assert_eq!(
        f.conclusion(&i, "presence", 3).knowledge,
        Knowledge::Observed("present".into())
    );
}
#[test]
fn manual_title_without_access() {
    let mut f = Fixture::new();
    let mut b = f.report(
        "koine:manual",
        &[("root-A", "complete")],
        "complete",
        1,
        None,
    );
    let e = f.entry(&mut b, "manual", "Meridian");
    f.claim(
        &mut b,
        &e,
        "manual_visibility",
        Some("visible"),
        "root-A",
        false,
    );
    f.finish(b);
    assert_eq!(f.store.snapshot().unwrap().rows("entitlements").count(), 0);
    let mut b = f.report(
        "koine:manual",
        &[("root-A", "complete")],
        "complete",
        2,
        None,
    );
    let a = f.entity(&mut b, &e, "entitlements", "A");
    f.claim(&mut b, &a, "access", Some("accessible"), "root-A", false);
    let hidden = f.claim(
        &mut b,
        &e,
        "manual_visibility",
        Some("hidden"),
        "root-A",
        false,
    );
    manual_replace(&f, &mut b, &e, "manual_visibility", &hidden);
    f.finish(b);
    let (steam, _, _, _) = f.observe(
        "fixture:steam",
        "app",
        None,
        None,
        Some("accessible"),
        None,
        1,
    );
    let active = reconcile::active_entries(
        &f.store.snapshot().unwrap(),
        &f.view(3),
        &f.contexts["library"],
    );
    assert!(!active.contains(&e));
    assert!(active.contains(&steam));
    let mut b = f.report(
        "koine:manual",
        &[("root-A", "complete")],
        "complete",
        4,
        None,
    );
    let visible = f.claim(
        &mut b,
        &e,
        "manual_visibility",
        Some("visible"),
        "root-A",
        false,
    );
    manual_replace(&f, &mut b, &e, "manual_visibility", &visible);
    f.finish(b);
    assert!(
        reconcile::active_entries(
            &f.store.snapshot().unwrap(),
            &f.view(5),
            &f.contexts["library"]
        )
        .contains(&e)
    );
    assert_eq!(
        f.conclusion(&a, "access", 5).knowledge,
        Knowledge::Observed("accessible".into())
    );
}
#[test]
fn multiple_installations_and_missing_mount() {
    let (f, a, b) = two_roots();
    let report = f.report(
        "fixture:steam",
        &[("root-B", "inconclusive")],
        "inconclusive",
        2,
        None,
    );
    f.finish(report);
    assert_ne!(a, b);
    for i in [&a, &b] {
        assert_eq!(
            f.conclusion(i, "presence", 3).knowledge,
            Knowledge::Observed("present".into())
        );
    }
    assert_eq!(f.store.snapshot().unwrap().rows("installations").count(), 2);
}
#[test]
fn persistent_library_recovery() {
    let mut f = Fixture::new();
    let mut b = f.report(
        "fixture:steam",
        &[("root-A", "complete")],
        "complete",
        1,
        Some(1),
    );
    let steam = f.entry(&mut b, "app", "Meridian");
    let access = f.entity(&mut b, &steam, "entitlements", "A");
    let c = f.claim(
        &mut b,
        &access,
        "access",
        Some("accessible"),
        "root-A",
        false,
    );
    let eid = b
        .rows
        .iter()
        .find(|r| r.is("id", &c))
        .unwrap()
        .get("evidence_id")
        .clone();
    b.rows
        .iter_mut()
        .find(|r| r.table == "evidence_records" && r.get("id") == &eid)
        .unwrap()
        .set("valid_until", 2i64);
    let installation = f.entity(&mut b, &steam, "installations", "root-A");
    f.claim(
        &mut b,
        &installation,
        "presence",
        Some("present"),
        "root-A",
        false,
    );
    f.claim(&mut b, &installation, "completion", None, "root-A", false);
    let target = f.entity(&mut b, &steam, "launch_targets", "default");
    f.claim(
        &mut b,
        &target,
        "handoff_capability",
        Some("supported"),
        "root-A",
        false,
    );
    f.finish(b);
    let (epic, _, _, _) = f.observe(
        "fixture:epic",
        "epic",
        Some("present"),
        None,
        Some("accessible"),
        Some("supported"),
        1,
    );
    let mut b = f.report(
        "fixture:steam",
        &[("root-A", "complete")],
        "complete",
        3,
        Some(3),
    );
    let negative = f.claim(
        &mut b,
        &installation,
        "presence",
        Some("absent"),
        "root-A",
        true,
    );
    let nid = b
        .rows
        .iter()
        .find(|r| r.is("id", &negative))
        .unwrap()
        .get("negative_justification_id")
        .clone();
    let proof = b
        .rows
        .iter_mut()
        .find(|r| r.table == "negative_justifications" && r.get("id") == &nid)
        .unwrap();
    proof.set("kind", "exhaustive_omission");
    proof.set("coverage_part_id", b.parts["root-A/presence"].clone());
    proof.set("required_outcome", "complete");
    proof.set("basis", "synthetic exhaustive enumeration");
    f.finish(b);
    let mut b = f.report(
        "koine:manual",
        &[("root-A", "complete")],
        "complete",
        4,
        None,
    );
    let evidence = f.evidence(&mut b);
    let game = id();
    b.rows.push(row(
        "games",
        [
            ("id", game.clone().into()),
            ("created_evidence_id", evidence.into()),
        ],
    ));
    for e in [steam, epic] {
        let link = id();
        b.rows.push(row(
            "game_links",
            [
                ("id", link.clone().into()),
                ("game_id", game.clone().into()),
                ("entry_id", e.into()),
            ],
        ));
        f.claim(&mut b, &link, "game_link", Some("linked"), "root-A", false);
    }
    f.finish(b);
    let before = f.store.reconstruct(&f.assessment(5, true)).unwrap();
    assert!(
        before
            .claims
            .iter()
            .any(|c| c.freshness == Freshness::Stale)
    );
    assert!(
        before
            .claims
            .iter()
            .any(|c| c.knowledge == Knowledge::Unknown)
    );
    assert!(
        before
            .claims
            .iter()
            .any(|c| c.knowledge == Knowledge::ConfirmedNegative("absent".into()))
    );
    let snapshot = f.store.snapshot().unwrap();
    let path = f.path.clone();
    let assessment = f.assessment(5, true);
    drop(f.store);
    let store = Store::open(path).unwrap();
    assert_eq!(snapshot, store.snapshot().unwrap());
    assert_eq!(before, store.reconstruct(&assessment).unwrap());
}
#[test]
fn missing_artwork_valid_target() {
    let mut f = Fixture::new();
    let (_, _, _, t) = f.observe(
        "fixture:steam",
        "app",
        None,
        None,
        None,
        Some("supported"),
        1,
    );
    let b = f.report("fixture:steam", &[("root-A", "failed")], "failed", 2, None);
    f.finish(b);
    let intent = f.intent(t.as_ref().unwrap());
    let result = f.handoff_record(intent.text("id").unwrap());
    let mut fake = FakeTransport::new("accepted");
    f.store.handoff(intent, result, &mut fake).unwrap();
    assert_eq!(fake.calls.len(), 1);
    assert_eq!(f.view(3).attempts[0].handoff, Handoff::Accepted);
}
#[test]
fn sprawl_exit_127() {
    let mut f = Fixture::new();
    let (_, _, _, t) = f.observe(
        "fixture:steam",
        "1549690",
        None,
        None,
        None,
        Some("supported"),
        1,
    );
    let intent = f.intent(t.as_ref().unwrap());
    let aid = intent.text("id").unwrap().to_string();
    let result = f.handoff_record(&aid);
    let mut fake = FakeTransport::new("failed");
    fake.status = Some(127);
    f.store.handoff(intent, result, &mut fake).unwrap();
    f.store
        .write(vec![f.process(&aid, "inconclusive", None, 22)])
        .unwrap();
    let a = &f.view(23).attempts[0];
    assert_eq!(a.handoff, Handoff::Failed);
    assert_eq!(a.exit_status, Some(127));
    assert_eq!(a.process, Process::Inconclusive);
    let assessment = f.assessment(23, true);
    let path = f.path.clone();
    drop(f.store);
    let store = Store::open(path).unwrap();
    assert_eq!(
        store.reconstruct(&assessment).unwrap().attempts[0].exit_status,
        Some(127)
    );
}
#[test]
fn accepted_without_process_confirmation() {
    let mut f = Fixture::new();
    let (_, _, _, t) = f.observe(
        "fixture:steam",
        "app",
        None,
        None,
        None,
        Some("supported"),
        1,
    );
    let intent = f.intent(t.as_ref().unwrap());
    let aid = intent.text("id").unwrap().to_string();
    let result = f.handoff_record(&aid);
    let mut fake = FakeTransport::new("accepted");
    f.store.handoff(intent, result, &mut fake).unwrap();
    f.store
        .write(vec![f.process(&aid, "inconclusive", None, 22)])
        .unwrap();
    let a = &f.view(23).attempts[0];
    assert_eq!(a.handoff, Handoff::Accepted);
    assert_eq!(a.process, Process::Inconclusive);
}
#[test]
fn timeout_is_not_definite_failure() {
    let mut f = Fixture::new();
    let (_, _, _, t) = f.observe(
        "fixture:steam",
        "app",
        None,
        None,
        None,
        Some("supported"),
        1,
    );
    let intent = f.intent(t.as_ref().unwrap());
    let aid = intent.text("id").unwrap().to_string();
    let result = f.handoff_record(&aid);
    let mut fake = FakeTransport::new("inconclusive");
    f.store.handoff(intent.clone(), result, &mut fake).unwrap();
    assert_eq!(f.view(23).attempts[0].handoff, Handoff::Inconclusive);
    let repeat = f.handoff_record(&aid);
    assert!(f.store.handoff(intent, repeat, &mut fake).is_err());
    assert_eq!(fake.calls.len(), 1);
    let mut definitive = f.handoff_record(&aid);
    definitive.set("disposition", "accepted");
    definitive.set("issuance", "issued");
    f.store.write(vec![definitive]).unwrap();
    assert_eq!(f.view(24).attempts[0].handoff, Handoff::Accepted);
}
#[test]
fn observed_process_and_confirmed_exit() {
    let (f, aid) = observed_attempt();
    let p = f.process(&aid, "observed", None, 22);
    let pid = p.text("id").unwrap().to_string();
    f.store.write(vec![p]).unwrap();
    assert_eq!(f.view(23).attempts[0].process, Process::Observed);
    f.store
        .write(vec![f.process(&aid, "exited", Some(&pid), 24)])
        .unwrap();
    assert_eq!(f.view(25).attempts[0].process, Process::Exited);
}
#[test]
fn observer_loss_and_pid_reuse() {
    let (f, aid) = observed_attempt();
    let p = f.process(&aid, "observed", None, 22);
    let pid = p.text("id").unwrap().to_string();
    f.store.write(vec![p]).unwrap();
    f.store
        .write(vec![f.process(&aid, "inconclusive", Some(&pid), 23)])
        .unwrap();
    assert_eq!(f.view(24).attempts[0].process, Process::Inconclusive);
    assert_eq!(
        f.store
            .reconstruct(&f.assessment(24, true))
            .unwrap()
            .attempts[0]
            .process,
        Process::Inconclusive
    );
    let mut wrong = f.process(&aid, "exited", Some(&pid), 25);
    wrong.set("identity_token", b"unrelated reused PID".to_vec());
    assert!(f.store.write(vec![wrong]).is_err());
}
#[test]
fn retry_and_unrelated_running_game() {
    let (f, a) = observed_attempt();
    f.store
        .write(vec![f.process(&a, "observed", None, 22)])
        .unwrap();
    let target = f
        .store
        .snapshot()
        .unwrap()
        .rows("launch_targets")
        .next()
        .unwrap()
        .text("id")
        .unwrap()
        .to_string();
    let intent = f.intent(&target);
    let b = intent.text("id").unwrap().to_string();
    let result = f.handoff_record(&b);
    let mut fake = FakeTransport::new("failed");
    fake.status = Some(127);
    f.store.handoff(intent, result, &mut fake).unwrap();
    let view = f.view(23);
    assert_eq!(
        view.attempts
            .iter()
            .find(|r| r.attempt == a)
            .unwrap()
            .process,
        Process::Observed
    );
    assert_eq!(
        view.attempts
            .iter()
            .find(|r| r.attempt == b)
            .unwrap()
            .handoff,
        Handoff::Failed
    );
    let snap = f.store.snapshot().unwrap();
    let record = snap
        .rows("handoff_records")
        .find(|r| r.is("attempt_id", &b))
        .unwrap()
        .clone();
    f.store.write(vec![record]).unwrap();
    assert_eq!(fake.calls.len(), 1);
}

fn two_roots() -> (Fixture, String, String) {
    let mut f = Fixture::new();
    let mut b = f.report(
        "fixture:steam",
        &[("root-A", "complete"), ("root-B", "complete")],
        "complete",
        1,
        Some(1),
    );
    let e = f.entry(&mut b, "app", "Meridian");
    let a = f.entity(&mut b, &e, "installations", "root-A");
    let z = f.entity(&mut b, &e, "installations", "root-B");
    f.claim(&mut b, &a, "presence", Some("present"), "root-A", false);
    f.claim(&mut b, &z, "presence", Some("present"), "root-B", false);
    f.finish(b);
    (f, a, z)
}
fn observed_attempt() -> (Fixture, String) {
    let mut f = Fixture::new();
    let (_, _, _, t) = f.observe(
        "fixture:steam",
        "app",
        None,
        None,
        None,
        Some("supported"),
        1,
    );
    let intent = f.intent(t.as_ref().unwrap());
    let aid = intent.text("id").unwrap().to_string();
    let result = f.handoff_record(&aid);
    let mut fake = FakeTransport::new("accepted");
    f.store.handoff(intent, result, &mut fake).unwrap();
    (f, aid)
}
fn manual_replace(f: &Fixture, b: &mut Batch, entity: &str, dim: &str, new: &str) {
    let snapshot = f.store.snapshot().unwrap();
    let subject = f
        .subjects
        .iter()
        .find(|(key, _)| key.starts_with(&format!("{entity}/{dim}/")))
        .unwrap()
        .1;
    let old = f
        .view(50)
        .claims
        .iter()
        .find(|c| &c.subject == subject)
        .unwrap()
        .supporting_claims[0]
        .clone();
    let did = id();
    b.rows.push(row(
        "reconciliation_decisions",
        [
            ("id", did.clone().into()),
            ("kind", "supersession".into()),
            ("rule_id", f.rules["reconciliation"].clone().into()),
            ("ordering_rule_id", f.rules["ordering"].clone().into()),
            ("assessed_at", b.time.into()),
            ("reason", format!("user-action:{old}").into()),
            ("collection_id", b.id.clone().into()),
        ],
    ));
    for (role, claim) in [("older", old.as_str()), ("replacement", new)] {
        b.rows.push(row(
            "decision_claims",
            [
                ("decision_id", did.clone().into()),
                ("claim_id", claim.into()),
                ("role", role.into()),
            ],
        ));
    }
    b.rows.push(row(
        "supersessions",
        [
            ("older_claim_id", old.into()),
            ("replacement_claim_id", new.into()),
            ("dimension", dim.into()),
            ("decision_id", did.clone().into()),
        ],
    ));
    b.rows.push(row(
        "decision_seals",
        [
            ("decision_id", did.into()),
            ("protocol", "koine.decision.v1".into()),
        ],
    ));
    drop(snapshot);
}

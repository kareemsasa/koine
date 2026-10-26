# Domain model

This is a logical contract under the [foundation authority](README.md). It defines
meaning and identity constraints, not tables, classes, or a persistence engine.

## Five distinct concepts

| Concept | Meaning and identity | Relationships and constraints |
| --- | --- | --- |
| Game identity | Koiné's organizational identity for a particular game/edition. Its stable local identity is independent of title spelling and external IDs. | Groups source entries without absorbing their authority. Renaming a display title does not create a different game. |
| Source entry | A title reported by a storefront, launcher, or manual source. Identified within its source namespace by that source's stable title key. | Retains original identifiers, title, and edition evidence. Several entries can link to one game. An unresolved entry can remain separate pending matching. |
| Entitlement | Evidence that a particular subject has access to a source entry, with the claimed access kind and any known validity limits. | Distinct from installation. Scope includes the source entry and subject/account context; multiple access claims may coexist. Unknown access kind must not become a purchase claim. |
| Installation | An observed local instance managed by an application, with its own evidence of location and completion state. | Refers to a source entry and manager context. Multiple installations of an entry may exist. Presence does not establish account ownership or launch success. |
| Launch target | The authoritative application and source-specific target used for a handoff. | Refers to a source entry, launcher context, and an installation when known and applicable. It is not a stored reconstruction of runner, prefix, environment, or game launch options. |

Conceptually: a game groups source entries; each entry can have zero or more
entitlement claims, installations, and launch targets. A launch attempt selects
one target, not the game's entire group. An entitlement need not have an
installation, and an installation need not have known entitlement evidence.
A target may exist without a confirmed installation; that fact alone does not
promise launchability or authorize Koiné to install anything.

For example, an Epic storefront entry can have an Epic access claim and a
Heroic-managed installation and target. Epic is the store; Heroic is the execution
authority; the discovery adapter is the reporter. Those names describe different
roles even when one integration provides evidence about all of them.

## Identity and matching invariants

- **D1 — Scoped external identity.** A bare title or bare external ID is not a
  global key. Include the namespace; observations also include their account,
  launcher instance, and machine context where relevant. Context identifiers may
  be opaque local references and must not require collecting credentials.
- **D2 — Stable local references.** Keep organizational identity and source keys
  stable across refreshes and display-name changes. A duplicate report of the same
  scoped entry updates evidence rather than creating a new title. Multiple
  reporters of the same source entry retain distinct observation provenance.
- **D3 — Grouping preserves records.** Linking entries to one game must not merge
  entitlements, installations, or launch targets. Removing an incorrect link must
  leave the source records and their evidence intact. A title match alone cannot
  establish equivalence; editions, remasters, DLC, and bundles may be different.
- **D4 — No inferred access.** An installed game, manual title, launch target, or
  successful handoff is not evidence of purchase or current account access.
  Explicit manual access assertions are user assertions, not storefront proof.
- **D5 — Independent dimensions.** Access, local presence, installation
  completeness, handoff capability, and process observation are independent.
  A single `owned`, `installed`, or `running` flag cannot replace these concepts.

Represent an unresolved or disputed match as such, with its evidence and reason.
Keeping separate entries is sufficient until matching is supported. This contract
does not require an automatic matching service or a matching UI in the first slice.

An installation key must distinguish instances within a manager and machine
context. A path is an observed locator, not by itself a universal identity: mounts
can disappear and directories can move. If continuity cannot be established,
retain uncertainty rather than silently combining observations. A launch target
must distinguish the launcher instance and source target even when two stores
use the same display title. The encoding of these keys remains open.

## Evidence belongs to claims

Every external claim, including a negative claim and a match assertion, must be
traceable to sufficient evidence:

| Information | Required meaning |
| --- | --- |
| Subject and scope | Which entry/claim, source namespace, account context if relevant, and local/manager context the observation concerns. |
| Reporter and origin | Adapter identity and interpretation/version information when available; originating source or user assertion; a safe locator for the relevant document, record, or observation. |
| Collection identity and time | Scan or observation reference and collection time; source-provided effective time if available, separately from collection time. Unknown source time stays unknown. |
| Claim and basis | What was actually reported, how it was interpreted, and why a negative or completeness assertion is justified. A file's presence supports only the facts its supported interpretation establishes. |
| Coverage and quality | Relevant scan coverage, limitations, freshness assessment and reason, and conflicting evidence or diagnostics affecting this claim. |

Re-observing an old cached record does not necessarily renew the freshness of its
underlying claim. Keep source age separate from when Koiné read it. Sufficient
provenance does not require retaining entire files, credentials, or unlimited raw
history. Physical storage layout and retention mechanisms are unselected, but must
preserve the basis for current conclusions and their supersession.

Persistent library records must preserve the five concepts, their stable
references and relationships, and the evidence needed to recover those conclusions
across application restarts. A restart must not convert unknown claims to false
or historical process evidence into a current running claim. This does not require
an event store, a particular database, or unlimited history.

## Knowledge and freshness

These dimensions are not interchangeable:

| Dimension | Values and interpretation |
| --- | --- |
| Knowledge | Observed positive; explicitly unknown with a reason; confirmed negative within a stated scope; disputed when incompatible relevant claims remain unresolved. |
| Freshness | Current as of a justified observation, stale with a reason, or freshness unknown. A current observation still describes a point in time, not an eternal guarantee. |
| Collection outcome | Complete, partial, failed, or inconclusive collection, as defined by the discovery contract. This describes the scan, not ownership. |

**D6 — Unknown is not false.** No evidence of an entitlement means access unknown.
No installation observed during an unreadable scan means installation unknown,
possibly with a last-known observation; it does not mean uninstalled.

**D7 — Stale is not revoked.** Retain the last-known claim and its age. Refresh
failure must not manufacture a new negative or refresh the old claim's timestamp.
Freshness thresholds are not selected; sources may provide validity limits and
implementations must expose the basis of any stale assessment.

**D8 — Conflicts retain provenance.** Reconcile claims only when scope, authority,
and observation ordering justify supersession. Different accounts are not a
conflict. Different fields from independent reporters do not gain authority over
one another. Incompatible claims with no justified ordering remain disputed; do
not pick whichever request finished last.

An installation can be observed present and incomplete, present and complete, or
present with completeness unknown. Confirmed absence is scoped to the context
actually inspected. `Owned but uninstalled` requires access evidence and justified
installation absence for the relevant context; a stale presence observation or
incomplete scan must be surfaced instead of collapsed into that label.

## Manual records and optional metadata

Manual entries use stable local source keys and explicit user provenance. A title
alone asserts neither access nor installation. Manual observations cannot overwrite
Steam or Heroic facts. A manually supplied target for a launcher-managed title
must still delegate to that launcher. Arbitrary imported command templates are
not automatically trusted or executed; manual command execution mechanics remain
outside this contract.

Artwork, screenshots, save locations, achievements, and documents are optional
claims with their own evidence when external. **D9 — Metadata independence:**
missing or failed optional metadata cannot invalidate an otherwise valid target,
erase ownership/installation evidence, or turn a handoff into failure.

# Discovery and reconciliation contract

This technology-neutral contract elaborates the [domain model](domain-model.md).
Adapters report observations; reconciliation maintains Koiné's evidence. Neither
component edits external launcher configuration or takes over its authority.

## Input and output

A discovery request identifies the adapter, a collection reference, the requested
source/context, and the requested evidence dimensions. For example, installation
discovery for two configured Steam library roots is different from account access
discovery. A request does not grant authentication, filesystem permissions, or
additional product scope. Unavailable capabilities must be reported explicitly.

A result contains the following logical information:

| Part | Required meaning |
| --- | --- |
| Identity | Collection reference, adapter identity/interpretation version, source namespace, and relevant context references. |
| Timing | Start and finish/termination times; per-observation collection/source times as available. |
| Coverage | Requested, actually inspected, and uninspected scope; separate evidence dimensions such as installation versus entitlement. Include enumeration boundaries, filters, and reasons for gaps. |
| Outcome | `complete`, `partial`, `failed`, or `inconclusive`, plus reasons. A result can describe several explicitly bounded coverage units with different outcomes. |
| Observations | Scoped source entries and claims about entitlements, installations, targets, or metadata, including provenance and unknown/negative assertions when justified. |
| Diagnostics | Missing permission, unavailable source, unsupported format, invalid record, cancellation, change during reading, or other relevant limitation; no secrets in diagnostics. |

An adapter can provide only the dimensions it supports. An empty observation list
has no negative meaning without coverage and an appropriate evidence basis.

## Outcomes and completeness

| Outcome | Meaning | Reconciliation consequence |
| --- | --- | --- |
| `complete` | All requested units and evidence dimensions within the declared scope were successfully inspected and interpreted, with adequate consistency for the reported claim. An unsupported requested dimension prevents this outcome for the aggregate. | May support bounded absence only under R3 below; does not establish global ownership completeness. |
| `partial` | Some useful requested work completed; other relevant units or records could not be inspected/interpreted. | Apply independently valid observations. Retain last-known claims in affected gaps; missing results do not imply absence. |
| `failed` | The requested discovery operation failed without usable observations. | Retain prior claims and record failure; do not replace the library with an empty result. |
| `inconclusive` | The operation cannot establish the requested conclusion or its coverage, without asserting a definite negative. For example, a missing root may be an unmounted volume or an unavailable locator. | Record unknowns/limitations. Retain prior claims where no valid replacement evidence exists. |

If a failed attempt produced independently usable observations, its aggregate
outcome is partial. An unsupported capability that establishes nothing is
inconclusive, with the unsupported reason. Cancellation follows the same rules:
partial if useful work survives, otherwise inconclusive, with cancellation noted.

*Update 2026-10-30:* revocation is not cancellation. If the configuration or
approval under which a source is read is withdrawn or changed before the
collection's result is durably accepted, the collection must discard everything
it read and close inconclusive, with the revocation noted and its original
requested units; no observation or coverage from it is applied. Acceptance and
revocation must be ordered deterministically: a revocation ordered after
acceptance leaves the accepted result valid, including when it is replayed after
an interruption. A source operation already in progress at revocation may
complete, but its result is discarded.

Completeness is relative to the original request. Do not shrink the reported
request to the subset that happened to succeed and call the whole scan complete.
A partial aggregate can contain a complete sub-scope, but only if the result
explicitly demonstrates that boundary. A corrupt record prevents complete
coverage for the claims depending on interpreting that record; other independently
readable records may still supply positive observations.

## Reconciliation invariants

- **R1 — Claim isolation.** Apply valid observations only to their scoped subjects
  and dimensions. Steam installation discovery cannot revoke access claims, a
  Heroic result cannot remove Steam installations, and a metadata outage cannot
  invalidate a launch target. Account or machine changes create different scopes.
- **R2 — Non-destructive gaps.** Failed, partial, cancelled, unsupported, and
  inconclusive discovery do not silently remove prior claims in unproven scope.
  Keep last-known evidence and report the collection gap. Age may make evidence
  stale; the failure itself is not evidence of absence.
- **R3 — Negative evidence needs authority and coverage.** An explicit negative
  must identify its subject, dimension, scope, time, and basis. Omission may support
  a negative only where the adapter's externally supported source semantics
  establish exhaustive enumeration for that dimension, that bounded scope was
  completely inspected, and no permission/filter/parse/consistency gap undermines
  it. `complete` alone does not supply those semantics. Without them, retain
  unknown/last-known state. Local manifests never prove absence of account access.
- **R4 — Supersession is narrow.** A justified newer negative can supersede an
  older positive within its authority and scope. Preserve the older evidence's
  basis and the reason for supersession; do not delete the source entry or unrelated
  relationships. Manual removal from Koiné is not external revocation or uninstall.
- **R5 — Stable refresh.** Reapplying the same result must not duplicate logical
  entries, claims, or installations or renew their observation time. New genuine
  observations may update evidence while preserving identity. Duplicate keys with
  incompatible claims require diagnostics/dispute, not arbitrary selection.
- **R6 — Ordering is evidence based.** A delayed older result cannot replace a
  newer relevant observation merely because it arrived later. Distinguish source
  effective time from collection time. If order or snapshot consistency cannot be
  justified, preserve the conflict/uncertainty; do not require a background sync
  architecture or assume wall clocks establish causality across sources.
- **R7 — Visible conclusions require their evidence.** Claims and coverage used to
  justify them must be presented/reconciled consistently. Do not expose an empty
  authoritative library while still collecting a scan. Incremental positives are
  possible, but absence deductions wait for adequate coverage evidence. This is a
  behavioral requirement, not a database transaction prescription.
- **R8 — Read-only collection.** Reading a source never authorizes repairing its
  records, changing launch options, installing a title, or refreshing account
  tokens through an unapproved authentication flow.

A reconciliation pass validates scope and claim provenance, applies justified
observations, evaluates any supported bounded negatives, and preserves unresolved
last-known claims with their limitations. A failed refresh can change collection
health without changing entitlement truth. No physical deletion policy or raw
observation retention duration is selected here.

## Initial adapter boundaries

**Manual:** report only explicit user-supplied entries and claims, with local
identity and user provenance. Omitting a field means unknown. Explicit deletion
of a manual record is a Koiné organizational operation, not a scan-based negative
about Steam or Heroic.

**Steam local discovery:** treat library-folder and app-manifest reading as an
installation evidence channel. Preserve manifest-reported incomplete installation
separately from complete installation; unsupported or ambiguous completion markers
mean completeness unknown. Do not assume every manifest is fully installed. Exact
file formats, flags, root discovery, and supported client/package variants need
adapter-specific evidence and fixtures before implementation. Do not hard-code a
workstation path or interpret an unreadable/unmounted library as empty.

**Steam ownership:** local installation observations do not enumerate account
ownership. The Web API is a possible later channel with key, account, visibility,
and filtering boundaries documented in [external evidence](integration-evidence.md).
This contract does not select or authorize authentication. Preserve access unknown
where only installation evidence exists.

**Heroic (next integration):** distinguish storefront identity from the Heroic
manager instance. Read-only installation evidence must not be turned into a
Legendary/gogdl or Wine command assembled by Koiné. Discovery and handoff capability
are separate: an installation may be known while a supported Heroic handoff is
unverified or unavailable. Formats, contexts, and transport compatibility still
need adapter-specific evidence; they are not resolved by a synthetic case.

The [controlled Steam implementation report](controlled-steam.md) records the
current versioned subset, explicit root authorization, filesystem boundaries,
application-local scheduling and synthetic verification. Those results do not
certify real Steam installations or grant account/absence authority.

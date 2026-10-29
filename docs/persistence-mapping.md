# Approved logical-to-relational mapping

**Status: Approved logical behavior; physical design separately approved.** Recorded
2026-10-26 from the owner's approval of the clarified mapping draft. The approval
covers that mapping, not independent inspection of this record or every detail
in it.
[Architecture A–D](architecture-options.md) and the existing
[domain](domain-model.md), [discovery](discovery-contract.md), and
[launch](launch-contract.md) contracts remain authoritative.

## Approval and provenance

The owner first conditionally approved the relational direction, requested four
clarifications, and then explicitly approved the clarified mapping. The original
mapping and clarifications originated as engineering proposals; owner review
established the behavioral obligations below. The approval does not turn the
24 synthetic acceptance specifications into executed tests.

| Review requirement | Owner review status | Approved obligation |
| --- | --- | --- |
| Scoped source identities | Satisfied | Use versioned identity profiles, explicit global scopes, and isolated provisional identities. Missing contexts cannot merge distinct records. |
| Supersession validity | Satisfied | Prove compatible subject, dimension, authority scope, justified ordering and negative basis; reject cycles and preserve unordered conflicts. |
| Crash-safe launch recovery | Satisfied | Distinguish intent, confirmed issuance, and acceptance. Ambiguous recovery cannot cause automatic retries or invented issuance. |
| Deterministic reconstruction | Satisfied | Use explicit policy versions and assessment time, validate evidence, retain disputes, and never decide facts through arrival or insertion order. |

The approved direction separates stable identities for the five concepts from
immutable claim-level evidence, collection outcomes and bounded coverage,
reconciliation decisions, and derived current state. Handoff and process evidence
remain separate and attempt-scoped. Reconciliation is transactional; external
operations remain outside database transactions. Authoritative `owned`,
`installed`, or `running` flags cannot replace the evidence.

## Identity scope and the Heroic boundary

A source identity consists of a namespace, an explicit identity scope, and its
source-specific key, interpreted by a versioned profile. Global namespaces have
an explicit global scope. Context-dependent namespaces require the applicable
account, instance, or library context. An unresolved context receives an isolated
provisional identity; no shared "unknown account" sentinel can merge records.
Reuse requires continuity evidence, not title similarity.

The mapping's `steam:app` profile concerns validated application identifiers;
manual entries are library-scoped. Neither statement establishes global identity
rules for every storefront or launcher identifier.

**The proposed `heroic:reported-entry/<store>` profile remains provisional.** Heroic
is an installation manager and execution authority, not automatically the
storefront identity authority. A Heroic-reported entry and an independently
reported Epic or GOG entry may prove to be the same source entry. The physical
model must support that resolution while preserving original observations and
references. It must not merge them before evidence establishes equivalence.

Source-entry equivalence is different from grouping Steam and Epic entries under
one game. Grouping never merges source-specific entitlements, installations, or
targets. Resolving a provisional identity must preserve provenance and cannot
silently delete a colliding existing identity.

## Supersession contract

Every supersession decision must establish:

1. The same resolved subject, or a supported identity-resolution relationship.
2. The same evidence dimension.
3. Compatible authority scope, including relevant subject/account, manager,
   machine, and bounded source context. Reporter identity alone is insufficient.
4. Supported interpretations and referenced evidence for both claims.
5. A justified ordering rule. Arrival, row order, UUID order, and collection
   timestamp alone are insufficient.
6. An explicit negative basis or exhaustive, complete bounded coverage when the
   replacement is negative. Scan completeness alone supplies no source authority.
7. No self-reference or directed supersession cycle.

Store the claims, applicable rule/version, scope, ordering basis, justification,
and decision time. Decision time records reconciliation, not external truth.
Preserve unordered contradictory claims as disputed; different accounts or
unrelated scopes remain independent. Reject a proposed cycle before commit. If
stored decisions are invalid or cyclic on reconstruction, diagnose the affected
scope and withhold a definitive conclusion rather than choose an arbitrary winner.

## Launch recovery contract

| Persisted evidence | Reconstructed meaning |
| --- | --- |
| Intent only after a crash | Issuance unknown; handoff inconclusive. Intent proves neither submission nor acceptance. |
| Known preflight refusal | No handoff issued; preserve the refusal reason. |
| Confirmed submission without disposition | Issuance established only at its recorded boundary; disposition inconclusive. |
| Definite handoff-command failure | Handoff failed with its actual diagnostic/status. No process or session is invented. |
| Transport acceptance | Acceptance at that boundary; startup still requires independent process evidence. |

Persist intent before the external operation and actual evidence afterward, in
separate transactions. Recover uncertainty across either crash window. Do not
replay an ambiguous attempt automatically; an explicit retry has a new identity.
For `sprawl-exit-127`, a durable definite status-127 failure remains failed without
an unsupported running claim. If only intent survived, do not infer the lost exit
status. Independent observations from other attempts remain separate.

## Reconstruction contract

Inputs are a consistent evidence snapshot, supported identity/interpretation
versions, explicitly selected reconciliation and freshness policy versions,
assessment time, and recovery context. Identical inputs produce identical
conclusions; presentation sorting cannot determine truth.

Reconstruction validates references and interpretations, resolves only supported
identities, validates supersession, evaluates surviving comparable claims,
assesses coverage separately, assesses freshness, and derives independent library
and launch conclusions. Expose supporting references, scope, supersession basis,
freshness basis, limitations, and diagnostics.

Unknown does not erase a last-known observation. Failed or incomplete collection
does not establish absence. Stale access is not revocation. Superseded claims stay
historical. Present/incomplete installation stays distinct from fully installed
or absent. Cold restart makes current process observation inconclusive until
continuity is re-established; historical positive evidence remains available.
Restart alone neither renews source freshness nor makes every claim stale.

Changing assessment time may change freshness. Unsupported policy or decoder
versions require explicit uncertainty; silently reinterpreting old evidence under
new adapter semantics is forbidden. Any reinterpretation needs traceable evidence
and a recorded decision.

## Physical-design approval and implementation boundary

The owner reviewed the revised
[physical SQLite design baseline](sqlite-schema-proposal.md) and approved its
catalog, canonical digest protocols and specified guard/seal behavior at the design
level. This is a separate approval from the logical mapping recorded above; it
establishes neither verification against the Git repository nor executable
correctness.

The implementation must verify canonical serialization using reproducible vectors
and enforce complete proofs, valid references, closure and coverage. Selected
canonical and database checks now pass in the separately authorized
[offline foundation](executable-foundation.md); that report limits the executable
claim and identifies untested conformance work. The separately approved
[SQLite operational policy](sqlite-operational-policy.md) selects connection
ownership, serialized writes, WAL/FULL, contention/checkpoint values, crash
recovery and backup mechanics at design level. Its backup completion/reader
availability and transaction-uncertainty obligations have selected offline coverage;
the full stress/fault matrix remains unverified.
The separately approved [migration policy](sqlite-migration-proposal.md) resolves
migration strategy at design level, with metadata approved in principle and
independent review required for each actual transition. Initial bootstrap metadata,
application ID and artifact digest are implemented in the separately authorized
offline milestone; released transitions, general backup administration,
guarded-table rebuild
mechanisms and distribution runtime verification remain open. The separately approved
[initial retention and archival policy](evidence-retention-proposal.md) retains all
committed structured evidence live and uses validated full copies without pruning
or relocation. Archival administration, pruning/cold relocation, optional-cache
cleanup, permanent erasure, routine backup scheduling and restoration remain open;
distribution compatibility remains unverified.

The design approval alone authorized documentation. The later executable-foundation
milestone was separately authorized for offline Rust code, disposable databases,
synthetic tests and local commits. All 24 unchanged Markdown specifications now
have passing executable translations; this does not authorize released upgrades,
live launcher execution or publication (publication was separately authorized on
2026-10-25).

# Physical SQLite schema design baseline

**Status: Approved design; selected offline implementation checks recorded separately.**
Originally prepared 2026-10-26. The owner independently reviewed the uploaded
revision at reported baseline `e728114` and approved its physical catalog,
canonical digest protocols, and specified guard/seal behavior. This is a separate
approval from [A–D](architecture-options.md) and the
[logical mapping](persistence-mapping.md).
This document specifies implementation obligations; it is not executable SQL or
an SQL migration. The later owner-authorized [offline foundation](executable-foundation.md)
now supplies bootstrap DDL, disposable databases and selected executable checks.
The original approval authorized design recording only; the later milestone
supplies implementation authority. Full conformance remains incomplete.

## Physical review disposition

The earlier review of `d951846` required revisions. The owner subsequently inspected
the uploaded revision's relational catalog, coverage model, canonical protocols,
decision sealing, and recovery rules, and approved the design baseline. The owner
did not inspect local Git state or execute SQL. The engineering origin of the
specification is unchanged by this approval; the review history is retained below.

| Choice | Owner disposition | Remaining boundary |
| --- | --- | --- |
| Canonical lowercase UUIDs as TEXT | Accepted | Selected offline format/round-trip checks pass; see the executable report for scope. |
| UTC integer microseconds | Accepted | Source time remains distinct from collection time; no ordering authority follows from a timestamp. |
| Byte-preserving Linux paths as BLOB | Accepted | Path interpretation remains source/context-specific. |
| STRICT and SQLite minimum 3.37.0 | Accepted as schema baseline | Distribution compatibility still needs verification; no packaging choice follows. |
| SHA-256 content digests and canonical protocols | Approved at design level | Independent framing and request/result/decision vectors now pass; exhaustive conformance remains incomplete. |
| Deferred decision sealing and specified guards | Approved at design level | Bootstrap DDL and selected guard/transactional checks now pass; exhaustive conformance remains incomplete. |
| Immutable evidence and derived current state | Approved design | No evidence-retention or pruning policy is implied. |
| Complete physical catalog | Approved at design level | Approval establishes implementation obligations, not executable correctness or implementation authorization. |

| Required correction | Independent owner review result |
| --- | --- |
| Manual-entry removal and restoration | Satisfied |
| Bounded coverage and attributable diagnostics | Satisfied |
| Repeatable source-key evidence | Satisfied |
| Canonical digest framing and seal verification | Satisfied |

### Attached validation requirements and authorization boundary

The future implementation must verify canonical serialization using reproducible
test vectors. Guard/conformance tests must demonstrate that incomplete decisions,
invalid references, post-closure mutations, and inconsistent coverage cannot become
authoritative. Actual DDL and transactional validation are required before claiming
executable correctness. These are attached obligations, not claims of exhaustive validation
or implementation permission from that approval. The later milestone separately
authorized offline code/databases/tests. All 24 specifications remain unchanged
and now have passing executable translations; selected guard checks also pass.

The separately approved [SQLite operational policy](sqlite-operational-policy.md)
selects connection ownership, journal/durability configuration, busy handling,
checkpointing, crash recovery and backup mechanics at design level. Its conformance
obligations have selected offline coverage; the full matrix remains incomplete. The separate
[migration policy](sqlite-migration-proposal.md) now has design approval, with
metadata approved in principle and independent review required for each actual
transition. Its detailed open decisions remain implementation prerequisites.
The separately approved [initial retention policy](evidence-retention-proposal.md)
keeps all committed structured evidence live and uses validated full archival
copies without pruning or relocation. Repair/restore tooling, archival
administration, pruning/cold relocation, optional-cache cleanup, permanent erasure,
backup scheduling/destinations and packaging/runtime supply remain independent
open decisions. Distribution compatibility remains unverified.
No authorization to create migrations, Rust code, databases, or run live launcher
integrations or publish follows from
the design approval alone. The later offline milestone separately authorizes local
implementation and synthetic tests; live integrations/publication remain excluded.

## Design choices, retained alternatives, and operational decisions

| Design area | Baseline or open decision | Retained alternative / practical consequence |
| --- | --- | --- |
| Encoding enforcement | Use UUID/text, timestamp/integer and path/BLOB choices with the specified checks and canonical framing below. | UUID BLOBs and older-baseline non-STRICT tables remain historical alternatives. Enforcement and framing have design approval; executable validation remains required. |
| SQLite distribution support | Use the accepted STRICT/3.37.0 schema baseline. | Verify compatibility in the eventual distribution model rather than infer availability from the development workstation. |
| Claim layout | A closed claim-subject registry and one finite typed-payload claim table, described below. | Separate tables per claim family reduce nullable payload fields but multiply common provenance and supersession structures. No unrestricted attribute/plugin schema is proposed. |
| Source identity resolution | Preserve source-entry handles and reporter-local keys; derive equivalence only from supported identity claims. | Destructive canonicalization would require rewriting references and loses the original distinction. |
| Current state | Derive from evidence and validated decisions; no authoritative current-state cache initially. | Materialized projections need explicit invalidation and consistency rules. |
| Mutation protection | Immutable facts and decisions, with rejecting update/delete guards; revisions add claims. | Relying solely on application convention gives weaker database protection. Each migration needs independently reviewed guard/preservation mechanisms under the separate migration policy; initial retention approval grants no pruning exception. |
| Connections and durability | Separately approved in the [operational policy](sqlite-operational-policy.md): dedicated reader/writer ownership, serialized IMMEDIATE writes, WAL/FULL, bounded contention, controlled checkpoints, crash recovery and validated backups. | Operational design approval does not establish executable conformance or select packaging/runtime supply. |
| Migration and retention | Migration policy separately approved at design level; metadata approved in principle. Initial retention and validated full archival copies separately approved without pruning or relocation. Actual transitions, repair/restore tooling, archival administration and pruning/cold relocation remain open. | No migration script or automatic purge is included in this design; each actual transition requires independent review. |

Catalog approval is separate from operational-policy, migration-policy and initial
retention-policy approval, and the remaining distribution, archival administration,
pruning/cold relocation and backup-administration decisions.
The [migration/schema-upgrade policy](sqlite-migration-proposal.md) has separate
design approval for compatibility, atomic transitions, recovery and validation.
Metadata additions are approved in principle; exact DDL/digest rules and each
transition's guard-maintenance mechanisms remain subject to independent review.
This does not expand the original physical-catalog approval or relax enforcement.
Any dependency still open must be settled before the
implementation that requires it. This design approval does not authorize
implementation or publication.

## Catalog notation and universal rules

Each table has `id: ID` as an explicitly non-null primary key unless a different
primary key is stated. `ID` expands to SQLite `TEXT` with binary collation and the
specified UUID format checks. `T` is non-null `TEXT`; `B` is non-null `BLOB`; `I`
is non-null `INTEGER`; `U` is `I` interpreted as UTC microseconds since Unix epoch.
Suffix `?` means nullable. An ID column followed by `→table` is an explicit foreign
key to that table's `id`, unless a composite parent key is specified. Every other
column, including semantic enums, is non-null. No implicit current-time defaults,
sentinel dates, or implicit false values are used.
`T` values and present `T?` values are nonempty. Literal payload alternatives below
are exact case-sensitive values. These rules expand to explicit length/null checks.

UUID checks require 36 bytes and 36 characters, hyphens at positions 9/14/19/24, exactly 32
remaining lowercase hexadecimal characters, and no other characters. Generation
and UUID version/variant validation remain core responsibilities. External keys
are not UUIDs; preserve their source representation, with any canonicalization
governed by the identity profile. UUID ordering is only a display/pair-normalization
convention and never chronology or authority.

SHA-256 is accepted for the 32-byte content digests. The
[canonical framing and verification protocol](#canonical-digest-framing-and-seal-verification)
below is approved at the design level and implemented in the separately
authorized [offline foundation](executable-foundation.md), with selected canonical
vectors and guard checks rerun in [Milestone 2](storage-resilience-steam.md).
It defines precise byte encoding, record membership, and ordering. Digests verify
only their specified records; they do not authenticate source facts or make a
conclusion authoritative.

Path values are BLOBs with no Unicode normalization or case folding. The core
rejects NUL in Linux filesystem paths. Other byte locators are interpreted by an
explicit locator kind; they are not executed. Timestamps may be negative and source
times may be unknown. No cross-clock or even local wall-clock ordering is inferred
from arithmetic on collection timestamps.

Foreign keys use immediate enforcement and `RESTRICT` for updates/deletes,
except the explicitly deferred decision-to-seal completion reference below.
There are no cascades through evidence. Every composite parent tuple referenced
below has an explicit `UNIQUE` key in matching order/collation. Optional composite
references explicitly constrain their nullable activation fields: all must be
present when the reference is active. Required context columns reused in such
tuples stay non-null even when the optional reference is absent. Examples are
negative-justification/subject and prior-observation/attempt.
The detailed checks below distinguish intentional inactive references from a
required proof bypassed through NULL. Every required reference is non-null.

All tables below contain immutable identity, observation, request, or decision
facts. Reject updates/deletes with guards that abort without changing other rows.
The repository must never issue replacement-style inserts. These guards assume
enforcement is enabled and do not make an externally modified file trustworthy.
Duplicate delivery must compare existing immutable content and either return
idempotently or reject the conflicting delivery with a diagnostic. This approved
guard design does not establish an indefinite-retention policy.

## Rules, contexts, and scoped source keys

| Table | Columns in addition to `id` | Relational constraints |
| --- | --- | --- |
| `rule_versions` | `kind T`, `name T`, `version T`, `artifact_digest B`, `reference T?` | Unique `(kind,name,version)`; digest length 32. Kind is identity, interpretation, reconciliation, ordering, freshness, or transport. Metadata identifies supported rules; it does not store executable rule code. |
| `contexts` | `kind T`, `namespace T`, `resolution T`, `external_key T?` | Kind is subject, manager, machine, library, or source_instance. Resolution is identified or provisional. Identified requires a nonempty key; provisional requires NULL. Partial unique `(kind,namespace,external_key)` for identified rows. Every provisional row has its own UUID. |
| `identity_profiles` | `namespace T`, `version T`, `scope_kind T`, `key_role T`, `catalog_namespace T?`, `rule_id ID→rule_versions` | Unique `(namespace,version)` and `(id,namespace,scope_kind)`. Scope kind is global, account, instance, account_instance, or library; key role is catalog, reporter_local, or manual. Catalog role requires a catalog namespace; other roles require NULL. Rule-kind compatibility is validated by the core. |
| `identity_scopes` | `namespace T`, `scope_kind T`, `resolution T`, `subject_context_id ID?→contexts`, `instance_context_id ID?→contexts`, `library_context_id ID?→contexts`, `provisional_token ID?` | Unique `(id,namespace,scope_kind)`. Shape checks and unique indexes below prevent missing-context collapse. |
| `source_entries` | `catalog_namespace T?`, `created_evidence_id ID→evidence_records` | A namespace, when known, identifies the underlying catalog/store, not the discovery adapter. Unknown is NULL, not a fabricated Heroic storefront. |
| `entry_keys` | `entry_id ID→source_entries`, `namespace T`, `scope_id ID→identity_scopes`, `scope_kind T`, `external_key T`, `created_evidence_id ID→evidence_records` | Unique `(namespace,scope_id,external_key)` and `(id,namespace,scope_kind)`. Composite FK `(scope_id,namespace,scope_kind)` preserves scope shape. The creation reference records origin, not all later supporting evidence. |
| `entry_key_observations` | `key_id ID→entry_keys`, `reported_key T`, `profile_id ID→identity_profiles`, `namespace T`, `scope_kind T`, `evidence_id ID→evidence_records`, `collection_id ID→collections`, `coverage_part_id ID?→coverage_parts`, `item_key T`, `basis T` | Unique `(collection_id,item_key)` and `(id,collection_id)`; composite FKs `(key_id,namespace,scope_kind)`, `(profile_id,namespace,scope_kind)`, `(evidence_id,collection_id)`, and optional `(coverage_part_id,collection_id)` match the corresponding parent tuples. Each observation preserves the original reported key, exact identity profile and evidence. |

An identified context key must identify an instance/subject within its declared
namespace. A product name such as Steam or Heroic is not an instance key. Known
local machine/library contexts may use persisted generated identifiers rather
than hardware fingerprints. Where identity continuity is unproven, retain an
isolated provisional context instead of assigning a common key.

Resolved identity scopes have exactly the following populated context columns;
all other context columns and the provisional token are NULL:

| Scope kind | Required contexts | Unique resolved-scope index |
| --- | --- | --- |
| global | None | `(namespace)` where resolved/global. |
| account | subject | `(namespace,subject_context_id)` where resolved/account. |
| instance | instance | `(namespace,instance_context_id)` where resolved/instance. |
| account_instance | subject and instance | `(namespace,subject_context_id,instance_context_id)` where resolved/account_instance. |
| library | library | `(namespace,library_context_id)` where resolved/library. |

For an unresolved required context, resolution is provisional and the token is
non-null and globally unique. Known applicable context components can be retained,
but incompatible components are rejected. Global scopes are always resolved.
The core validates context kinds and the continuity evidence needed to reuse a
provisional token. No shared "unknown" context or NULL-based uniqueness is used.

Profile version is interpretation provenance, not part of the uniqueness tuple:
upgrading a rule cannot create a duplicate global App ID simply by changing its
version. A change in identity semantics needs an explicit resolution plan/new
namespace, not silent rewriting of existing keys. New observations can reference
a newer interpretation while retaining the same established key.

The first observed key creates one stable `entry_keys` row and one
`entry_key_observations` row atomically. Every genuine subsequent observation adds
another observation referencing that same key, including the new profile/version,
evidence, basis, collection, and coverage attribution. No replacement of creation
evidence or profile is needed. Redelivery of the same item reuses its existing
observation after exact content comparison; it does not renew any timestamp.
External key observations require a coverage part; manual input may be an explicit
point assertion. Origin and part validity are core checks.

An observation records the reported mapping, not unconditional endorsement of it.
The core validates supported profile semantics and source authority. Conflicting
or unsupported interpretations remain diagnosed/disputed/unknown, with all prior
observations preserved. A rejected interpretation can be recorded as a diagnostic
against its evidence instead of inventing positive support. A newer profile does
not become authoritative merely because it is newer. Reinterpreting closed old
evidence still requires a separately specified explicit operation; fresh collection
under a new profile does not rewrite the old observation.
Where a profile defines canonicalization, verify that its reported_key maps to
the persistent scoped external_key and retain the exact reported form. Do not
replace older raw keys or silently apply a different canonicalization rule.

The provisional Heroic reporter profile is not an approved seed definition. Its
key role remains reporter_local. Independently justified storefront identity uses
a separate supported catalog profile; it does not promote Heroic into storefront
identity authority. This design approval does not verify Heroic's key format.

## The five concepts and identity relationships

| Table | Columns in addition to `id` | Relational constraints |
| --- | --- | --- |
| `games` | `created_evidence_id ID→evidence_records` | Display labels are claims, not identity keys. |
| `game_links` | `game_id ID→games`, `entry_id ID→source_entries` | Unique `(game_id,entry_id)`. Candidate association only; linked/unlinked/disputed state comes from claims. |
| `entry_equivalences` | `left_entry_id ID→source_entries`, `right_entry_id ID→source_entries` | Distinct entries; binary `left_entry_id < right_entry_id` and unique pair avoid duplicate reversed pairs. The sort convention selects no factual winner. |
| `context_equivalences` | `left_context_id ID→contexts`, `right_context_id ID→contexts` | Distinct contexts; binary `left_context_id < right_context_id` and unique pair. Candidate only; supported equivalent/different/unknown claims supply its meaning. |
| `entitlements` | `entry_id ID→source_entries`, `subject_context_id ID→contexts`, `authority_scope_id ID→authority_scopes`, `entitlement_key T`, `created_evidence_id ID→evidence_records` | Unique `(entry_id,subject_context_id,authority_scope_id,entitlement_key)` and `(id,authority_scope_id)`. A source without a grant ID needs a documented summary-key rule, not a fabricated purchase record. |
| `installations` | `entry_id ID→source_entries`, `manager_context_id ID→contexts`, `machine_context_id ID→contexts`, `instance_key T`, `created_evidence_id ID→evidence_records` | Unique `(entry_id,manager_context_id,machine_context_id,instance_key)` and `(id,manager_context_id)`. Instance-key continuity needs evidence; a path alone is insufficient. |
| `launch_targets` | `entry_id ID→source_entries`, `launcher_context_id ID→contexts`, `target_key T`, `created_evidence_id ID→evidence_records` | Unique `(entry_id,launcher_context_id,target_key)` and `(id,launcher_context_id)`. Installation associations are separate candidates/claims, not an immutable nullable field that could never acquire new evidence. |
| `target_installation_links` | `target_id ID→launch_targets`, `installation_id ID→installations`, `manager_context_id ID→contexts` | Unique `(target_id,installation_id)`. Composite FKs `(target_id,manager_context_id)` and `(installation_id,manager_context_id)` reference target `(id,launcher_context_id)` and installation `(id,manager_context_id)`. The core validates same/equivalent source identity. |

Context roles, entitlement summary-key semantics, installation continuity, and
same-source identity compatibility are core checks, not consequences of a bare
foreign key. Target keys are immutable handoff identifiers, not imported shell
templates. No optional artwork or other metadata is required by any target FK.

### Manual-entry lifecycle

Add the finite claim dimension `manual_visibility`, targeting one source-entry
handle through the existing `claim_subjects.entry_id`. Its observed value is
visible or hidden; unknown has the normal explicit reason and NULL payload. It
is an organizational claim, not a negative entitlement/installation claim.

The core requires a supported manual-key observation for that entry in the same
library context, a user_assertion evidence origin, and authority namespace
`koine:manual` with that library context. A Steam/Heroic scan cannot author this
claim. The existing subject/dimension/authority uniqueness isolates each library.
The database stores the exact references; the core proves manual origin and scope.

Manual creation includes an observed visible claim with the title assertion.
Removal appends a hidden claim and a justified supersession decision in the same
manual-operation transaction. The decision cites the affected prior visibility
claim and explicit user action as its causal basis, not collection timestamp
precedence. Restoration is another explicit visible claim with the same safeguards.
Concurrent unordered visibility assertions remain disputed. Missing visibility
evidence means unknown, never inferred removal; no expiry policy is introduced.

Active-library reconstruction excludes only the applicable hidden manual handle
in its library context. It preserves its source keys, title/access claims,
organizational links, targets, and historical launch evidence. It does not delete
the game or hide a Steam/Heroic entry with a matching title, and does not propagate
hidden through game grouping or source equivalence. A game with another active
entry remains present through that entry. Unlinking and removal remain independent;
removal does not revoke access, uninstall, stop a game, or purge evidence.

This supplies the persistent removal semantics required by
`manual-title-without-access`. The separately approved initial retention policy
preserves this history; physical pruning and permanent erasure remain unselected.

Context-role checks are explicit core obligations: subject references have kind
subject; manager/launcher references manager; machine references machine; library
references library. An instance reference can be manager or source_instance as
declared by its supported profile. A discovery root has kind source_instance;
reporter contexts identify a manager, source instance, or manual library as
appropriate. A product label is never sufficient continuity evidence.

Context resolution adds supported claims on `context_equivalences` rather than
changing a provisional row's key. The core requires the same context kind and
compatible identity authority; the same person having two storefront accounts
does not make those accounts equivalent. Original context, scope, and entry-key
references survive. Subsequent scope compatibility or entry resolution cites that
proof explicitly. Missing proof leaves contexts separate. The optional target /
installation association deliberately requires the same stored manager handle: use the
established handle when evidence supports it, or retain an unbound target rather
than bypass the constraint or invent an installation association.

Target/installation associations can be added or revised through claims after a
target already exists. Multiple candidates do not assert which installation the
launcher will execute; supported source semantics must establish any such claim.
Missing or disputed association evidence does not invalidate an otherwise valid
handoff target, and unlinked does not mean an installation was removed.

### Heroic and independent storefront observations

A Heroic Epic report can initially reference entry A through a reporter-local key;
an independently obtained Epic observation can reference entry B through a supported
catalog key. Both handles, their observations, installations, and targets survive.
Evidence can later support an `entry_equivalence` claim between A and B. The
derived source identity then contains both handles, without rewriting them or
deleting either reporter's provenance. Conflicting or insufficient evidence keeps
the equivalence disputed/unknown rather than merging records.

When an authoritative key already resolves to an established entry, a reporter
may attach to that entry only with the supported identity proof. Key collisions
never authorize replacement inserts or deletion. Original namespace assertions
remain historical; an unknown catalog may be resolved by the identity evidence,
not by guessing from the manager name.

The core requires compatible storefront/catalog identity for source equivalence;
Steam and Epic records for the same game remain different source entries joined
through `game_links`. Source-equivalence components can have ordinary symmetric
graph cycles; the prohibition on cycles applies to directed claim supersession.
Neither equivalence nor game grouping merges distinct access or installation
instances. Corresponding cross-handle subjects need their own compatibility proof.
Equivalence validation examines whole connected components: A equivalent to B and
B equivalent to C cannot override conflicting evidence that A differs from C.
An inconsistent component remains identity-disputed; no edge is picked by arrival
order, and the component cannot justify subject/authority unification or
supersession until resolved. This applies to context equivalence as well.
Core validation permits at most one governing organizational game association per
resolved entry; multiple incompatible linked candidates remain disputed. Candidate
pair uniqueness is not an authoritative current-membership flag. Merely writing
an Epic authority label is also insufficient identity proof: the core must verify
the supported catalog-key semantics and evidence.

## Authority, collection coverage, and evidence

| Table | Columns in addition to `id` | Relational constraints |
| --- | --- | --- |
| `authority_scopes` | `authority_namespace T`, `scope_key B`, `subject_context_id ID?→contexts`, `manager_context_id ID?→contexts`, `machine_context_id ID?→contexts`, `library_context_id ID?→contexts`, `provisional_token ID?` | Unique `(authority_namespace,scope_key)`. The canonical framing section defines the specified role-qualified scope key. The core validates the encoding against the columns and isolates unknown required contexts. Manual visibility requires a library context. |
| `collections` | `kind T`, `reporter T`, `reporter_version T`, `reporter_context_id ID→contexts`, `interpretation_rule_id ID→rule_versions`, `started_at U`, `request_protocol T`, `request_digest B` | Kind discovery or manual; request protocol koine.request.v1; digest length 32. Reporter is separate from claim authority. Digest covers registered request metadata/units, not source authenticity. |
| `requested_units` | `collection_id ID→collections`, `unit_key T`, `authority_scope_id ID→authority_scopes`, `dimension T`, `root_context_id ID?→contexts`, `selector B?`, `selector_rule_id ID?→rule_versions` | Unique `(collection_id,unit_key)`, `(id,collection_id)` and `(id,collection_id,dimension,authority_scope_id)`. Selector and selector-rule reference are both present or both NULL. The core validates the selector, full request membership, and dimension. No credentials in selectors. |
| `coverage_results` | `unit_id ID→requested_units`, `collection_id ID→collections`, `dimension T`, `authority_scope_id ID→authority_scopes`, `outcome T`, `reason T?` | Unique `unit_id` and `(id,collection_id,unit_id,dimension,authority_scope_id)`. Composite FK `(unit_id,collection_id,dimension,authority_scope_id)` references the corresponding requested-unit tuple. Outcome complete, partial, failed, or inconclusive; non-complete requires a reason. Exact extents are in coverage_parts. |
| `coverage_parts` | `coverage_id ID→coverage_results`, `collection_id ID→collections`, `unit_id ID→requested_units`, `dimension T`, `authority_scope_id ID→authority_scopes`, `part_key T`, `parent_part_id ID?→coverage_parts`, `selector_kind T`, `selector B?`, `selector_rule_id ID?→rule_versions`, `inspection T`, `outcome T`, `reason T?` | Unique `(coverage_id,part_key)`, `(id,collection_id)`, `(id,collection_id,dimension,authority_scope_id)`, `(id,coverage_id,collection_id,unit_id,dimension,authority_scope_id)`, and `(id,collection_id,outcome,dimension,authority_scope_id)`. Composite coverage FK matches its full unique tuple. Optional parent FK includes coverage/collection/unit/dimension/authority; self-parent rejected. A partial unique index on coverage_id where parent_part_id is NULL permits at most one root. |
| `collection_closures` | **PK `collection_id ID→collections`**, `outcome T`, `finished_at U`, `reason T?`, `result_protocol T`, `result_digest B` | Outcome uses the same four values; non-complete requires a reason; result protocol koine.result.v1; digest length 32. Core validates aggregate outcome against the original request and actual results. |
| `evidence_records` | `collection_id ID→collections`, `origin_kind T`, `origin_namespace T`, `locator_kind T`, `locator B?`, `collected_at U`, `source_effective_at U?`, `source_revision T?`, `valid_until U?`, `interpretation_rule_id ID→rule_versions`, `item_key T` | Unique `(collection_id,item_key)` and `(id,collection_id)`. Origin external or user_assertion. Locator kind path, opaque, or unavailable; unavailable requires NULL, others non-null. A user assertion is not storefront proof. |
| `diagnostics` | `collection_id ID→collections`, `item_key T`, `target_kind T`, `unit_id ID?→requested_units`, `coverage_part_id ID?→coverage_parts`, `evidence_id ID?→evidence_records`, `key_observation_id ID?→entry_key_observations`, `claim_id ID?→claims`, `severity T`, `code T`, `message T`, `affected_dimension T?`, `recorded_at U` | Unique `(collection_id,item_key)`. Target kind collection, unit, coverage_part, evidence, key_observation, or claim. Collection target has all optional targets NULL; other kinds require exactly their matching target. Composite `(target_id,collection_id)` FKs prevent cross-collection attribution. Severity info, warning, or error. |

### Bounded coverage and diagnostic attribution

Every coverage result must have a root part representing exactly its registered
requested unit, with selector_kind=whole_unit and NULL selector/rule. The core
requires that root and matching root/result outcomes before closure; the unique
index enforces at most one root. Non-root parts require selector_kind=bounded or
residual, non-null selector bytes and selector-rule reference. Parent FKs preserve
scope. The specified insert guard rejects missing parents, self-parenting, a parent
chain reaching the new ID, or an already cyclic parent chain; core validation also
requires acyclicity on closure/read. An immediate FK by itself is not a cycle proof.
Children partition the parent's scope,
not its request timestamp or order of discovery.

The v1 selector is exactly E(list), without an envelope, using the framing
primitives defined below: path selectors are [path, path-bytes, item-or-subtree]; key selectors
are [keys, namespace, identity-scope-UUID, ordered-key-list]; residual selectors are
[remainder, parent-part-UUID, ordered-excluded-sibling-UUID-list]. The first value
is a literal text discriminator. Keys and exclusions have no duplicates and are
sorted by their encoded scalar bytes; item-or-subtree is literal text item or
subtree. Source-specific membership and whether a
subtree/key set is exhaustive require the cited supported selector/capability rule.
Selector bytes preserve actual extents, not a vague label such as "some files".

Inspection is inspected, uninspected, mixed, or unknown. Complete requires
inspected. Uninspected permits only failed/inconclusive; mixed requires children
and a non-complete outcome; unknown also requires a non-complete outcome.
Non-complete outcomes require a reason. An inspected part can still be partial or
inconclusive because inspection does not guarantee successful interpretation.
Outcome validity still follows the discovery contract: usable surviving work makes
the aggregate partial rather than failed, regardless of how much inspection occurred.

The core validates containment, disjointness and exhaustiveness for children.
If the residual extent cannot be enumerated, record an explicit residual selector
excluding the known bounded children, with unknown inspection and inconclusive
outcome. Never omit the remainder and call the parent complete. If supported rules
cannot establish a bounded part's extent, it cannot justify an omission negative.
Checks cover the whole relevant part tree, including source changes during reading.

For example, a partial root can contain an inspected/complete subtree A and an
uninspected/inconclusive subtree B with a permission diagnostic. Valid observations
from A remain usable. An exhaustive-omission proof can cite A only when its source
semantics justify it, and cannot extend to B or the partial root. If more unenumerated
scope may exist, a residual part records that gap. No real Steam directory format
or exhaustive capability is established by this synthetic example.

Diagnostics are repeatable rows: several codes/causes can affect the same target.
An observation-level issue targets its evidence record; a more specific claim issue
can target the claim, and an identity-interpretation issue can target the particular
key observation. A part identifies its unit and collection through its FKs.
Collection-wide failures target the collection. Use separate attributed rows when
one cause affects multiple targets. Codes are stable categories and messages are
sanitized explanations, not raw logs, credentials, or invented negative facts.
Severity alone never establishes completeness; the core must account for every
relevant gap when validating coverage. Unknown source records can retain an
evidence-level diagnostic without a fabricated valid claim.
If a diagnostic's relevance or a selector rule is unsupported/unknown, withhold
the affected negative conclusion rather than assume the gap is harmless.

Evidence can support several propositions, but each claim has its own basis and
value. No raw launcher configuration, credential, or complete source payload is
required. Source revision tokens carry no implicit numeric order. Cached source
age remains separate from collection time; unknown source age stays unknown.

Register a collection and its original requested units together. Final application
of evidence, coverage, claims, decisions, and the closure is a separate atomic
transaction. Insert the closure last. Specified insert guards reject new units,
results, parts, diagnostics, key observations, evidence, claims, negative
justifications, and collection-linked decisions
after closure. A request digest is checked in the core so registration cannot be
silently shrunk.
No closure means pending/incomplete operation, not an empty authoritative result.
Cold recovery reports inconclusive collection progress with prior library evidence
intact; it does not assert that a pending source operation completed.

Idempotency checks compare stable unit/part/item keys and immutable logical
content, including original times and interpretation versions, against the closed
stored result. Return its existing local IDs instead of allocating a second set.
A different payload under the same delivery identity is rejected/diagnosed outside
the closed record. The result digest seals the stored result bundle, not a fresh
allocation of IDs on replay. Do not globally deduplicate equal payloads: a genuine
later observation can provide different freshness evidence. No digest supplies
source authority or ordering proof.

## Claim subjects and typed values

`claim_subjects` has `id`, `dimension T`, `authority_scope_id ID→authority_scopes`,
and nullable FKs `game_id→games`, `entry_id→source_entries`,
`entitlement_id→entitlements`, `installation_id→installations`,
`target_id→launch_targets`, `game_link_id→game_links`, and
`equivalence_id→entry_equivalences`, plus
`context_equivalence_id→context_equivalences` and
`target_installation_link_id→target_installation_links`.

A CHECK requires exactly one non-null entity FK and the matching dimension from
the table below. A partial unique index for each entity column covers
`(entity_id,dimension,authority_scope_id)` where that entity is non-null. There is
also unique `(id,dimension,authority_scope_id)`. Entitlement subjects additionally
reference entitlement `(id,authority_scope_id)` as a composite FK. Core checks
validate all other authority/subject context compatibility.

`claims` has `id`, `subject_id ID→claim_subjects`, `dimension T`,
`authority_scope_id ID→authority_scopes`, `evidence_id ID→evidence_records`,
`collection_id ID→collections`, `knowledge T`, `value_text T?`, `value_bytes B?`,
`access_kind T?`, `reason T?`, `basis_code T`, and
`negative_justification_id ID?→negative_justifications`, plus
`coverage_part_id ID?→coverage_parts`.

Composite FKs match subject `(id,dimension,authority_scope_id)` and evidence
`(id,collection_id)`. An optional part reference matches
`(id,collection_id,dimension,authority_scope_id)`. External claims require the
part covering their dimension; manual point assertions can omit it. Origin,
membership and interpretation validity are core checks. Unique parent keys are
`(id,collection_id)`, `(id,dimension)` and
`(id,subject_id,dimension,authority_scope_id)`. Knowledge is observed,
confirmed_negative, or unknown. Disputed and stale are derived conclusions, not
new source assertions. The following CHECK branches fully define allowed payloads:

| Dimension | Entity FK | Observed `value_text` | Confirmed negative `value_text` |
| --- | --- | --- | --- |
| game_label | game | Nonempty display label | Not allowed. |
| entry_title | entry | Nonempty reported title | Not allowed. |
| entry_edition | entry | Nonempty reported edition | Not allowed. |
| manual_visibility | entry | visible or hidden | Not allowed; hidden is organizational removal, not external revocation. |
| access | entitlement | accessible | inaccessible |
| presence | installation | present | absent |
| completion | installation | complete or incomplete | Not allowed; incomplete is an observed fact. |
| location | installation | NULL, with non-null `value_bytes` path | Not allowed. |
| handoff_capability | target | supported or unavailable | Not allowed; unavailable is an observed capability fact. |
| target_installation | target_installation_link | linked or unlinked | Not allowed; unlinked does not mean uninstalled. |
| game_link | game_link | linked or unlinked | Not allowed. |
| entry_equivalence | equivalence | equivalent or different | Not allowed; explicit different requires its own identity evidence. |
| context_equivalence | context_equivalence | equivalent or different | Not allowed; explicit different requires its own context evidence. |

Unknown requires a nonempty reason and NULL value/access-kind/negative-justification
columns. Observed location requires nonempty bytes; all other observed/negative
dimensions require NULL bytes and a non-null allowed text value. Access kind is
nonempty only for observed accessible access, otherwise NULL; NULL means kind
unknown, not purchased. A confirmed negative requires a justification; other
knowledge states require NULL justification. Every branch explicitly checks null
shape as well as value, avoiding SQLite's NULL-passing CHECK behavior.

Claims contain no asserted current-state flags. An unknown report cannot
supersede prior positive evidence merely because its collection is newer.

## Negative justification and reconciliation decisions

| Table | Columns in addition to `id` | Relational constraints |
| --- | --- | --- |
| `negative_justifications` | `subject_id ID→claim_subjects`, `dimension T`, `authority_scope_id ID→authority_scopes`, `evidence_id ID→evidence_records`, `collection_id ID→collections`, `kind T`, `coverage_part_id ID?→coverage_parts`, `required_outcome T?`, `capability_rule_id ID→rule_versions`, `basis T` | Composite FKs match subject scope and evidence collection. Unique `(id,subject_id,dimension,authority_scope_id,collection_id)`. Kind explicit or exhaustive_omission. Explicit has NULL part/outcome; omission requires a part, outcome literal complete, and composite FK `(coverage_part_id,collection_id,required_outcome,dimension,authority_scope_id)` to the corresponding coverage-part tuple. |
| `reconciliation_decisions` | `kind T`, `rule_id ID→rule_versions`, `ordering_rule_id ID?→rule_versions`, `assessed_at U`, `reason T`, `collection_id ID?→collections` | Kind supersession, supersession_rejected, or dispute. Supersession requires an ordering-rule reference. Deferred FK from `id` to `decision_seals.decision_id` requires completion by commit. Rule meaning and scope compatibility are core checks. |
| `decision_claims` | **PK `(decision_id,claim_id,role)`**, `decision_id ID→reconciliation_decisions`, `claim_id ID→claims`, `role T` | Role older, replacement, identity_support, ordering_support, negative_support, or conflicting. A role alone is not proof that the claim establishes it. |
| `supersessions` | **PK `(older_claim_id,replacement_claim_id)`**, `older_claim_id ID→claims`, `replacement_claim_id ID→claims`, `dimension T`, `decision_id ID→reconciliation_decisions` | Different endpoints; composite FKs `(older_claim_id,dimension)` and `(replacement_claim_id,dimension)` reference claim `(id,dimension)`. Multiple edges can share a replacement. |
| `decision_seals` | **PK `decision_id ID→reconciliation_decisions`**, `protocol T`, `content_digest B` | Protocol koine.decision.v1; digest length 32. Seals the decision's metadata, claim-role links and edges using the canonical protocol below. Insert after those records, within the same transaction. |

Claims additionally have the composite FK
`(negative_justification_id,subject_id,dimension,authority_scope_id,collection_id)`
to the justification's matching unique key. A CHECK makes the justification ID
non-null for confirmed negatives, so nullable composite-FK semantics cannot bypass
the reference. The negative evidence and the justification's evidence must support
the same conclusion; that is a core check, not an equality of collection IDs.

The database can establish matching references, dimension, authority-scope ID, and
a recorded complete coverage flag. The core must establish actual exhaustive
source semantics, precise coverage membership, permission/filter/parse consistency,
and authority. A local installation scan cannot justify a negative access claim
regardless of SQL validity.
An omission justification cites a complete bounded part, not the incomplete
aggregate. Its conclusion is restricted to that part's proven extent. The core
also validates the aggregate/part tree and any relevant diagnostics before allowing
that proof; a complete flag cannot conceal a gap affecting the same subject.

Before inserting a supersession, the core proves every approved validity condition
and includes the cited proof claims. Where distinct handles are involved, it must
prove subject compatibility through valid source identity evidence; equivalence of
source entries alone does not equate two accounts, installations, or entitlements.
The specified insertion guard rejects a new edge when a path already exists from
replacement back to older. The core also validates the whole graph on reading;
invalid/cyclic stored decisions produce scoped diagnostics and no definitive winner.
This approved guard obligation now has a bootstrap trigger and selected cycle tests
in the offline foundation; the report identifies the validation limits.

Unordered contradiction creates a dispute decision with conflicting claim links,
not a supersession. Decision timestamps record reasoning, never source ordering.
Corrections require new evidence and explicit decisions; this design supplies
no silent deletion of a bad decision. Recovery/repair mechanics remain open.
Insert guards reject additional claim-role links or supersession edges for sealed
decisions. The deferred completion FK requires a seal before commit; the seal's
own FK to its decision is immediate. This intentional pair permits metadata,
inputs, edges, then seal in one transaction, without leaving a committed partially
assembled decision. The core checks content digest and semantic completeness;
the seal is not evidence that the conclusion is correct. Reading unsealed or
invalid decisions from a damaged/unsupported file produces scoped uncertainty.
Standalone assessment decisions use NULL collection_id and cite existing claims;
they do not reopen a closed source result. Emitting newly interpreted claims from
closed evidence needs a separately specified reinterpretation operation. That
operation is not included here, and must never bypass the closure guard or pretend
that old evidence was freshly collected.

## Canonical digest framing and seal verification

**Protocol status: Approved design; independent offline vectors and selected checks pass.**
The following exact serialization, manifests and verification behavior are approved.
The frozen definitions below retain their design authority. Executable vectors and
selected conformance results are now recorded in the offline foundation report.

### Byte grammar

A framed value consists of the four ASCII bytes KOIN (hex 4b 4f 49 4e), the two
bytes 00 01 identifying framing version 1, E(protocol), then E(payload). There is
no padding, separator, terminal newline, BOM, or trailing data. Protocol is the
exact ASCII text named below. E recursively encodes one typed value:

| Value | Exact bytes emitted by E |
| --- | --- |
| NULL | One byte 00 and no payload. |
| INTEGER | One byte 01, then exactly eight bytes of signed two's-complement integer, most significant byte first. Range is minus 2^63 through 2^63 minus 1; timestamps use this same representation in their accepted microsecond unit. |
| TEXT | One byte 02, then an unsigned eight-byte big-endian UTF-8 byte length, then exactly those UTF-8 bytes. Strict UTF-8; no normalization, trimming, case folding, newline conversion, or escaping. |
| BLOB | One byte 03, then an unsigned eight-byte big-endian byte length, then the exact bytes. Never decode a BLOB as text. |
| List | One byte 04, then an unsigned eight-byte big-endian element count, then E of each element in list order. |

Unused tags, REAL values, out-of-range integers/counts/lengths, invalid UTF-8,
truncation, and trailing bytes are invalid. Integers have no variable-length form.
NULL, empty TEXT, empty BLOB, and an empty list have different encodings; the
catalog's nonempty/shape checks can independently forbid them in particular fields.
UUIDs are encoded as their canonical stored TEXT, not converted to 16-byte values.
TEXT containing an otherwise allowed character is encoded exactly; path BLOBs
retain the separate Linux-path validation rules.

For record bundles, payload is an ordered list of table records. Each table record
is [table-name TEXT, ordered-column-name list, ordered-row list]. Each row is a
list of typed values matching exactly those columns. All listed tables appear,
including empty child sets. No extra or omitted columns are permitted. Row lists
are sorted by their primary-key tuples: compare E of each key component as unsigned
byte strings, lexicographically, with a shorter equal-prefix string first. Duplicate
primary-key tuples are invalid, not silently deduplicated. This sort grants no
factual authority to a key. The following manifests freeze table order and columns.

### Decision manifest: koine.decision.v1

For decision D, include exactly its metadata row, every decision_claims row with
decision_id D, and every supersessions row with decision_id D, in this table order:

| Table | Ordered columns | Primary-key sort |
| --- | --- | --- |
| reconciliation_decisions | id, kind, rule_id, ordering_rule_id, assessed_at, reason, collection_id | id |
| decision_claims | decision_id, claim_id, role | decision_id, claim_id, role |
| supersessions | older_claim_id, replacement_claim_id, dimension, decision_id | older_claim_id, replacement_claim_id |

There must be exactly one metadata row. Supersession decisions require at least
one edge and matching older/replacement role links for every edge, plus the actual
supported proof required by the logical contract. Rejected-supersession and dispute
decisions have no supersession edges; a dispute requires at least two conflicting
claim links. Counts and digests do not replace semantic validation.

The seal is the raw 32-byte SHA-256 digest of this complete framed value. Store
protocol=koine.decision.v1 and those bytes in decision_seals. Do not hash hexadecimal
text, rowid, query order, SQL text, the seal row, or the enclosing collection digest.
Referenced claim/rule/evidence bodies are not recursively hashed by this manifest.
Their references and semantics must be validated independently. A matching seal
does not detect every alteration elsewhere in a database and is not a signature.

### Request manifest: koine.request.v1

For collection C, include its one collections row and all registered requested_units
rows with collection_id C, in the following order. Request registration fixes
their IDs and content. The request_digest column is excluded to avoid self-hashing.

| Table | Ordered columns | Primary-key sort |
| --- | --- | --- |
| collections | id, kind, reporter, reporter_version, reporter_context_id, interpretation_rule_id, started_at, request_protocol | id |
| requested_units | id, collection_id, unit_key, authority_scope_id, dimension, root_context_id, selector, selector_rule_id | id |

Hash the framed value and store its raw digest with request_protocol=koine.request.v1.
The core checks registered membership against this digest before final reconciliation.
It still proves scope meaning and request coverage independently.

### Result manifest: koine.result.v1

The result digest now seals the **persisted result bundle** for collection C.
It is not a universal fingerprint of adapter input independent of assigned local
IDs. Replay compares original stable item/part/unit keys and logical payloads,
including evidence times and profile versions, and returns stored IDs. It must
not allocate another set of IDs and compare a different byte stream.

Select every row whose collection_id is C for the directly collection-scoped
tables below. For decision_claims, supersessions and decision_seals, select rows
whose referenced reconciliation_decisions.collection_id is C. Include the one
closure row for C, excluding its result_digest. Use exactly this table order:

| Table | Ordered columns | Primary-key sort |
| --- | --- | --- |
| coverage_results | id, unit_id, collection_id, dimension, authority_scope_id, outcome, reason | id |
| coverage_parts | id, coverage_id, collection_id, unit_id, dimension, authority_scope_id, part_key, parent_part_id, selector_kind, selector, selector_rule_id, inspection, outcome, reason | id |
| evidence_records | id, collection_id, origin_kind, origin_namespace, locator_kind, locator, collected_at, source_effective_at, source_revision, valid_until, interpretation_rule_id, item_key | id |
| entry_key_observations | id, key_id, reported_key, profile_id, namespace, scope_kind, evidence_id, collection_id, coverage_part_id, item_key, basis | id |
| claims | id, subject_id, dimension, authority_scope_id, evidence_id, collection_id, knowledge, value_text, value_bytes, access_kind, reason, basis_code, negative_justification_id, coverage_part_id | id |
| negative_justifications | id, subject_id, dimension, authority_scope_id, evidence_id, collection_id, kind, coverage_part_id, required_outcome, capability_rule_id, basis | id |
| diagnostics | id, collection_id, item_key, target_kind, unit_id, coverage_part_id, evidence_id, key_observation_id, claim_id, severity, code, message, affected_dimension, recorded_at | id |
| reconciliation_decisions | id, kind, rule_id, ordering_rule_id, assessed_at, reason, collection_id | id |
| decision_claims | decision_id, claim_id, role | decision_id, claim_id, role |
| supersessions | older_claim_id, replacement_claim_id, dimension, decision_id | older_claim_id, replacement_claim_id |
| decision_seals | decision_id, protocol, content_digest | decision_id |
| collection_closures | collection_id, outcome, finished_at, reason, result_protocol | collection_id |

Compute the digest before inserting the closure: supply the intended closure values
as the final row of the manifest, then insert that closure and its raw digest in
the same transaction. Request rows are checked with their separate request digest.
Shared identity/context/profile rows are referenced, not recursively included;
their validity is independently checked. The result manifest does not hash launch
attempts or standalone decisions, which are separate operations.

### Scope-key framing and protocol evolution

Authority scope_key is the complete framed value, not its digest, with protocol
koine.authority-scope.v1 and payload [authority_namespace, subject_context_id,
manager_context_id, machine_context_id, library_context_id, provisional_token],
using explicit NULLs for absent roles. Compare it with the corresponding columns.
Provisional tokens isolate unknown required contexts; equivalent contexts are
resolved through evidence, never by rewriting key bytes.

Rule artifact digests hash the exact bytes of the identified supported artifact,
without this envelope; they are distinct from request/result/decision manifests.
No executable rule or hashing extension is stored in SQLite.

The frame and each manifest have explicit versions. The protocol name stored with
a digest must select the exact manifest above; it cannot select the reader's latest
column list. A covered table/column change needs a new reviewed manifest/version.
Old records remain associated with their original version, and a reader must retain
that manifest to verify them. Unknown versions produce an unsupported-protocol
diagnostic; there is no guessed fallback or silent resealing.
Request/result protocol columns must match the protocol in their envelope; the
decision seal's protocol must match the selected decision envelope. A disagreement
is invalid, not a reason to try another manifest until a digest happens to match.

### Decision-seal write and read protocol

1. In one write transaction, insert decision metadata, all claim-role links and
   edges. Validate the full logical proof, references, scope, and acyclicity.
   The deferred decision-to-seal FK temporarily permits the missing seal.
2. Read exactly the decision manifest from that transaction's consistent view,
   validate types/shapes/membership, frame the values, and compute SHA-256 in the
   future core. Insert its protocol and raw digest as the seal. The seal's immediate
   FK requires the decision already to exist. Guards reject further additions to
   sealed decision children and updates/deletes of its records. The core may create
   a seal only after validating unsealed, complete, semantically valid content.
3. Commit only after every decision has its seal. Deferred FK enforcement proves
   seal presence, not correct hash computation. A collection closure, when applicable,
   follows all decision seals and includes them in its result manifest.
4. On reading a decision for reconstruction, take one consistent snapshot, require
   a supported protocol and exactly one seal, reconstruct the exact metadata/link/
   edge sets, recompute the framed digest, and compare all 32 stored bytes. Also
   validate referenced evidence, rules, authority, ordering and graph integrity.
   Digest equality is necessary for this protocol but insufficient for
   factual validity.
5. A missing seal, invalid shape, mismatch, unsupported protocol, or invalid proof
   produces a scoped integrity/uncertainty diagnostic. Do not apply its supersession
   as authoritative, choose a row-order winner, mutate the digest, or replay a
   launch. Preserve records for review; repair mechanics remain open.

There is no assumed SQLite SHA-256 function. Guards reject structural mutation;
the Rust core performs canonical hashing and semantic verification. The prose
guard obligations and verification protocol have design approval. SQL definitions and selected validation now exist under the separate milestone
authorization; this prose remains the approved design specification. Collection-time diagnostics are
sealed with their result; later read
diagnostics are returned to the caller and do not append to/reseal a closed bundle.
Persistent post-closure repair/audit operations are not selected here.

## Attempt-scoped handoff and process evidence

| Table | Columns in addition to `id` | Relational constraints |
| --- | --- | --- |
| `launch_attempts` | `target_id ID→launch_targets`, `created_at U`, `intent_reason T` | This row records intent only. It proves neither issuance nor acceptance. Every explicit retry gets a new ID. |
| `handoff_records` | `attempt_id ID→launch_attempts`, `item_key T`, `recorded_at U`, `transport_rule_id ID→rule_versions`, `boundary T`, `issuance T`, `disposition T`, `exit_status I?`, `termination_signal I?`, `reason T?` | Unique `(attempt_id,item_key)`. Issuance issued, not_issued, or unknown; disposition pending, accepted, failed, inconclusive, or not_requested. Checks described below. |
| `process_observations` | `attempt_id ID→launch_attempts`, `item_key T`, `observed_at U`, `method_rule_id ID→rule_versions`, `observer_epoch ID`, `state T`, `identity_token B?`, `identity_basis T?`, `pid I?`, `prior_observed_id ID?→process_observations`, `prior_state T?`, `reason T?` | Unique `(attempt_id,item_key)` and `(id,attempt_id,state)`. State observed, inconclusive, or exited. Composite prior reference and payload checks described below. |

Accepted and pending handoff records require issued issuance. Not_requested
requires not_issued. Inconclusive permits issued or unknown; failed can carry any
issuance result with its actual basis. Exit status or termination signal is present
only when an actual command termination was observed, with accepted, failed, or
inconclusive disposition. The two fields are mutually exclusive; signals must be
positive and exit status nonnegative. Successful forwarding can retain exit zero;
not every failure has a command exit status. Neither field is game-exit evidence.
Failed, inconclusive, and not_requested require a nonempty reason. Submission
failure may be definite even when the launcher itself was never reached. Boundary
names and transport-rule references preserve which fact was established; no
particular Steam/Heroic transport is selected by these columns.

Observed and exited process rows require nonempty identity token and basis.
Inconclusive requires a reason and no newly asserted identity/PID fields; it may
reference a prior observation. A PID, when present, must be positive and is never
sufficient identity. Exited requires `prior_observed_id` and `prior_state=observed`;
the composite FK `(prior_observed_id,attempt_id,prior_state)` references process
`(id,attempt_id,state)`. The two nullable activation fields, prior_observed_id and
prior_state, are either both NULL or both present with prior_state=observed;
attempt_id always remains non-null. A CHECK also rejects self-reference. The core
checks identity continuity, target association, observation ordering, and exit
evidence. No method or observation-window duration is chosen here.

Persist intent in one transaction, issue external work outside it, then persist
actual transport evidence in another. Cold restart with intent only reconstructs
unknown issuance and inconclusive disposition, never a requested/accepted handoff
or an automatic retry. A durable definite status-127 failure reconstructs failed
handoff without observed running state or a fabricated session. Lost failure
evidence cannot be reconstructed from intent. Process facts are independent and
do not inherit acceptance from handoff results.

Observer epoch identifies the runtime observation context; it is not proof of
continuity. Cold restart preserves historical observations and reports current
process observation inconclusive until identity continuity is re-established.
Contradictory handoff records at incompatible/unresolved boundaries are diagnosed,
not resolved by their row order. No game-session or playtime table is included.

## Index and guard catalog

In addition to the primary, unique, and partial-unique indexes specified above,
use non-unique indexes for every FK's child columns in the declared order,
unless an existing index already has those columns as its leftmost prefix. Add
`claims(subject_id,evidence_id)`, `evidence_records(collection_id,collected_at)`,
`supersessions(replacement_claim_id,older_claim_id)`,
`handoff_records(attempt_id,recorded_at)`, and
`process_observations(attempt_id,observed_at)`. Time indexes support retrieval, not
factual precedence. No full-text/artwork index or performance claim is included.

The specified guards are: reject update/delete on immutable catalog records; reject
new collection content after closure; reject additions to sealed decisions; and
reject cyclic supersession insertion and invalid/cyclic coverage parenting.
They abort rather than mutate the row being checked. Closing a collection after
adding its content in one transaction supports the collection guards. Decisions
use the one deliberately deferred completion reference specified above.
Cross-table semantic meaning still belongs to core validation.
Startup/read validation must not assume constraints prove the file is uncorrupted.

## Transaction and reconstruction obligations

| Operation | Specified atomic boundary / recovery behavior |
| --- | --- |
| Manual creation/removal/restoration/grouping | Identities as needed, user-assertion evidence, visibility/grouping claims, justified supersession and decision seals together. Hidden manual records preserve their history and do not hide matching storefront entries. |
| Collection registration | Collection and original requested-unit records together. No library conclusions are replaced. |
| Bounded result application | Independently valid evidence, repeated key observations, claims, coverage parts, diagnostics, negative proofs, decisions, edges, seals and closure together. Failed/partial outcomes retain prior claims in gaps. |
| Launch intent | Attempt row committed before external operation. If this fails, no external handoff is issued. |
| Handoff result | Persist actual result separately. If persistence fails after submission, report uncertainty about durable recovery, not that submission never happened. |
| Process observation | Observation and its identity/continuity references together, independently of handoff. |

No database transaction spans filesystem enumeration, command waiting, or launcher
execution. Blocking storage work also stays outside GTK's UI thread. Large-result
batching is not included; it would need explicit consistency/coverage rules.
Connection ownership, write-transaction mode, durability and busy handling are
separately approved in the [operational policy](sqlite-operational-policy.md).
Its transaction-uncertainty and no-replay obligations apply without changing the
atomic boundaries above. Selected offline checks now pass as recorded in the report; full conformance remains incomplete.

Reconstruction takes a consistent snapshot, supported profile/interpretation
versions, selected reconciliation and freshness policy versions, assessment time,
and recovery context. Validate references/interpretations, resolve supported
identities, validate supersession, evaluate comparable surviving claims, assess
coverage independently, then assess freshness and derive library/handoff/process
conclusions. Same inputs produce the same conclusions. Never use insertion order,
UUID order, newest collection time, or decision time as a factual tie-breaker.

Current output includes knowledge, last-known evidence, freshness and its basis,
supersession/dispute references, collection health/limitations, and diagnostics.
Unknown does not become false; failed refresh does not erase a positive or renew
its time; incomplete installation is not absent; stale access is not revocation.
Unsupported versions or invalid decisions yield scoped uncertainty. No automatic
reinterpretation under a new decoder is allowed. Presentation sorting can be
stable without granting authority to its sort key.

## Paper review cases and validation limits

The existing 24 [acceptance specifications](acceptance-cases.md) remain unchanged.
The following are additional review obligations, not executable tests or claims
that SQLite has accepted the specified constraints:

| Review input | Required result |
| --- | --- |
| Same external key in two required account/instance contexts | Two identities; missing context produces isolated provisional scope. |
| Target predates a known installation association | Add a candidate and supported link claim; preserve the target's identity. Missing association evidence never becomes installation absence. |
| New profile version for an established global key | No duplicate identity solely due to version change. |
| Heroic Epic reporter key plus independent Epic catalog key | Separate until supported equivalence; afterward retain both handles and all original relationships/provenance. |
| Provisional account context later receives a claimed identity | Resolve only through supported context-equivalence evidence; keep original scope/key references and never equate two stores' accounts from a shared person/name. |
| A=B, B=C, and unresolved A-different-from-C identity evidence | Identity-disputed component; no naive transitive merge or downstream supersession justified by that component. |
| Steam and Epic titles with equivalent game identity | Game grouping only; no source-entry equivalence or merged entitlement. |
| Negative with NULL justification, mismatched subject, or non-complete omission coverage | Rejected structurally where catalog constraints apply; semantic authority/coverage still checked in the core. |
| Comparable contradiction without ordering proof | Dispute, not supersession. A cycle is rejected; stored corruption cannot pick a winner. |
| Intent persisted; crash before/after actual submission | Unknown issuance and inconclusive handoff; no replay or accepted/running claim. |
| Definite status-127 result durably recorded | Failed handoff; no invented process exit or running session. |
| Equivalent snapshot presented in different row orders | Identical conclusions for the same rules, assessment time, and recovery context. |
| Old process positive loaded during cold restart | Historical evidence retained; current observation inconclusive. |

Later validation must check UUID/path/time round trips; every CHECK null branch;
composite parent-key/collation compatibility; foreign-key enforcement on every
connection; duplicate-content handling; guard interactions; rollback and recovery;
identity conflicts; and reconstruction determinism. The executable report records
selected completed checks; it does not claim exhaustive coverage or authority for
live integrations, released migrations or publication.

## SQLite evidence and remaining gate

Primary documentation inspected for this design:

- [STRICT tables](https://www.sqlite.org/stricttables.html): support begins at
  3.37.0 and permitted type names are restricted. The owner accepted this schema
  baseline; compatibility in the eventual distribution model remains unverified.
- [CREATE TABLE](https://www.sqlite.org/lang_createtable.html): NULL passes a CHECK
  and NULLs are distinct for uniqueness; explicitly constrain null shape and keys.
- [Foreign keys](https://www.sqlite.org/foreignkeys.html): enable and verify them
  per connection; nullable composite children can skip enforcement, and parent
  tuples need matching primary/unique keys and collations. Mere creation of a
  table is not sufficient verification of every foreign-key relationship.
- [Triggers](https://www.sqlite.org/lang_createtrigger.html): rejecting guards can
  raise constraint errors. Specified guards must not modify the row they inspect.
- [Transactions](https://www.sqlite.org/lang_transaction.html): database atomicity
  and single-writer behavior do not determine application connection ownership or
  make an external launcher action atomic with storage.

The physical-design review gate is satisfied. The catalog, canonical protocols and
specified guard/seal behavior form the approved implementation design baseline.
The executable report records passing independent serialization vectors and
selected guard/conformance checks. The full required matrix remains incomplete. The
[operational policy](sqlite-operational-policy.md) separately resolves connection
ownership, journal/durability settings, busy handling, checkpointing, routine crash
recovery and backup mechanics as design. The
[migration policy](sqlite-migration-proposal.md) separately resolves migration
strategy at design level, with metadata approved in principle and independent
per-transition review required. The separately approved initial retention policy
keeps all committed structured evidence live and uses validated full archival
copies without pruning or relocation. Migration's detailed open decisions,
archival administration, pruning/cold relocation, optional-cache cleanup, permanent
erasure, repair/restore tooling, backup scheduling/destinations/finalization policy
and packaging/runtime supply remain open. Distribution compatibility is still unverified.
No current ownership, integration success or publication clearance follows from
documenting the schema. The later milestone separately authorized the offline
implementation; its measured test results and limitations are recorded in the
executable report, independently of this design approval.

## Revision record

- Initial catalog: local commit `d951846`, Proposed; no executable validation.
- Owner physical review: based on the uploaded proposal, accepted the limited
  encoding/algorithm/baseline choices above and deferred sealing in principle;
  withheld complete catalog approval for three modeling gaps and canonical digest
  details. This revision proposes the corrections. It records no SQL/runtime
  success, distribution compatibility, implementation authorization, or publication.
- Independent review of the uploaded revised proposal at reported baseline
  `e728114`: the owner found all four corrections satisfied and approved the
  physical catalog, canonical protocols and specified guard/seal behavior at the
  design level. The owner did not inspect local Git state or execute SQL. Required
  reproducible serialization vectors and guard/conformance validation remain
  unperformed. Operational decisions and implementation/publication authorization
  were not advanced. The preceding Proposed/withheld entries describe historical
  checkpoints, not the current design status.
- Subsequent operational review at reported baseline `5f578cc`: the owner approved
  the uploaded [operational policy](sqlite-operational-policy.md) at design level,
  including initial values and retained conformance obligations. This separately
  resolves the operational decisions left open at the physical-review checkpoint;
  no physical catalog, executable-validation or publication status changed.
- Subsequent migration review, recorded 2026-10-27: the owner independently reviewed
  the uploaded [migration policy](sqlite-migration-proposal.md) against this design,
  the operational policy and SQLite documentation, and approved it at design level.
  Identity/history metadata is approved in principle; exact definitions, artifact
  digest rules and per-transition mechanisms still require review. Original catalog
  approval, executable-validation, implementation and publication boundaries are
  unchanged.

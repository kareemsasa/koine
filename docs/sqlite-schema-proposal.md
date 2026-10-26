# Physical SQLite schema proposal

**Status: Proposed — independent owner review required.** Prepared 2026-10-26.
The [logical mapping](persistence-mapping.md) is approved; every table, encoding,
constraint, index, and database mechanism below is an engineering proposal.
[A–D](architecture-options.md) approve SQLite and relational separation, not this
physical design. This document contains a column/constraint catalog, not an SQL
migration. No database has been created and no executable schema tests have run.

## Review choices and alternatives

| Choice needing review | Concrete recommendation | Alternative / practical consequence |
| --- | --- | --- |
| Encodings | Canonical lowercase UUID text, UTC integer microseconds, byte-preserving filesystem paths; typed, length-delimited digest inputs. | UUID BLOBs are smaller; text is easier to inspect. Exact digest byte serialization and scope-key framing still need specification before implementation, alongside round-trip validation. |
| SQLite baseline | `STRICT` tables, requiring SQLite 3.37.0 or later. | Ordinary tables plus explicit storage-class checks preserve an older baseline but need a separate reviewed constraint expansion. The recommendation is a proposed distribution minimum, not a selected one. |
| Claim layout | A closed claim-subject registry and one finite typed-payload claim table, described below. | Separate tables per claim family reduce nullable payload fields but multiply common provenance and supersession structures. No unrestricted attribute/plugin schema is proposed. |
| Source identity resolution | Preserve source-entry handles and reporter-local keys; derive equivalence only from supported identity claims. | Destructive canonicalization would require rewriting references and loses the original distinction. |
| Current state | Derive from evidence and validated decisions; no authoritative current-state cache initially. | Materialized projections need explicit invalidation and consistency rules. |
| Mutation protection | Immutable facts and decisions, with rejecting update/delete guards; revisions add claims. | Relying solely on application convention gives weaker database protection. A later approved retention/migration path must account for these guards. |
| Connections and durability | Leave connection ownership, journal mode, synchronous setting, busy handling, and crash-backup mechanics open. | Serialized application mutation alone does not select any of them. |
| Migration and retention | Leave version transitions, recovery tooling, pruning, and archival open. | No migration or automatic purge is included in this proposal. |

Exact catalog approval can be separate from distribution, connection, migration,
and retention decisions. Any dependency still open must be settled before the
implementation that requires it; approving this document would not itself authorize
implementation or publication.

## Catalog notation and universal rules

Each table has `id: ID` as an explicitly non-null primary key unless a different
primary key is stated. `ID` expands to SQLite `TEXT` with binary collation and the
proposed UUID format checks. `T` is non-null `TEXT`; `B` is non-null `BLOB`; `I`
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

Propose SHA-256 for the 32-byte artifact/request/result digests. This is a content
comparison convention, not an authentication or source-authority mechanism.
Digest input must be versioned, type-tagged and length-delimited, with explicit
NULL representation. Unordered sets are sorted by table/field identity and stable
keys solely for hashing. Request digests cover original request metadata/units;
result digests cover the adapter's immutable logical result, not a second allocation
of local IDs on redelivery. Decision seals cover metadata, claim-role links and
edges, excluding the seal itself. Hashing SQLite file/page bytes is not proposed.
The exact byte framing remains an encoding review item; no operational digest
protocol is approved or implemented here.

Path values are BLOBs with no Unicode normalization or case folding. The core
rejects NUL in Linux filesystem paths. Other byte locators are interpreted by an
explicit locator kind; they are not executed. Timestamps may be negative and source
times may be unknown. No cross-clock or even local wall-clock ordering is inferred
from arithmetic on collection timestamps.

Foreign keys propose immediate enforcement and `RESTRICT` for updates/deletes,
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
idempotently or reject the conflicting delivery with a diagnostic. This guard
design remains proposed; it is
not an approved indefinite-retention policy.

## Rules, contexts, and scoped source keys

| Table | Columns in addition to `id` | Relational constraints |
| --- | --- | --- |
| `rule_versions` | `kind T`, `name T`, `version T`, `artifact_digest B`, `reference T?` | Unique `(kind,name,version)`; digest length 32. Kind is identity, interpretation, reconciliation, ordering, freshness, or transport. Metadata identifies supported rules; it does not store executable rule code. |
| `contexts` | `kind T`, `namespace T`, `resolution T`, `external_key T?` | Kind is subject, manager, machine, library, or source_instance. Resolution is identified or provisional. Identified requires a nonempty key; provisional requires NULL. Partial unique `(kind,namespace,external_key)` for identified rows. Every provisional row has its own UUID. |
| `identity_profiles` | `namespace T`, `version T`, `scope_kind T`, `key_role T`, `catalog_namespace T?`, `rule_id ID→rule_versions` | Unique `(namespace,version)` and `(id,namespace,scope_kind)`. Scope kind is global, account, instance, account_instance, or library; key role is catalog, reporter_local, or manual. Catalog role requires a catalog namespace; other roles require NULL. Rule-kind compatibility is validated by the core. |
| `identity_scopes` | `namespace T`, `scope_kind T`, `resolution T`, `subject_context_id ID?→contexts`, `instance_context_id ID?→contexts`, `library_context_id ID?→contexts`, `provisional_token ID?` | Unique `(id,namespace,scope_kind)`. Shape checks and unique indexes below prevent missing-context collapse. |
| `source_entries` | `catalog_namespace T?`, `created_evidence_id ID→evidence_records` | A namespace, when known, identifies the underlying catalog/store, not the discovery adapter. Unknown is NULL, not a fabricated Heroic storefront. |
| `entry_keys` | `entry_id ID→source_entries`, `profile_id ID→identity_profiles`, `namespace T`, `scope_id ID→identity_scopes`, `scope_kind T`, `external_key T`, `evidence_id ID→evidence_records` | Unique `(namespace,scope_id,external_key)`. Composite FKs `(profile_id,namespace,scope_kind)` and `(scope_id,namespace,scope_kind)` prevent profile/scope namespace or shape mismatch. Keys are nonempty. |

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

The provisional Heroic reporter profile is not an approved seed definition. Its
key role remains reporter_local. Independently justified storefront identity uses
a separate supported catalog profile; it does not promote Heroic into storefront
identity authority. This proposal does not verify Heroic's key format.

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
| `authority_scopes` | `authority_namespace T`, `scope_key B`, `subject_context_id ID?→contexts`, `manager_context_id ID?→contexts`, `machine_context_id ID?→contexts`, `provisional_token ID?` | Unique `(authority_namespace,scope_key)`. Proposed key encoding is a length-prefixed tuple of role-qualified context UUIDs plus a provisional token where needed. The core validates the encoding against the columns and requires isolation of unknown required contexts. |
| `collections` | `kind T`, `reporter T`, `reporter_version T`, `reporter_context_id ID→contexts`, `interpretation_rule_id ID→rule_versions`, `started_at U`, `request_digest B` | Kind discovery or manual; digest length 32. Reporter is separate from claim authority. Digest covers the registered request and all requested units; it does not prove source authenticity. |
| `requested_units` | `collection_id ID→collections`, `unit_key T`, `authority_scope_id ID→authority_scopes`, `dimension T`, `root_context_id ID?→contexts`, `selector B?`, `selector_rule_id ID?→rule_versions` | Unique `(collection_id,unit_key)` and `(id,collection_id,dimension,authority_scope_id)`. Selector and selector-rule reference are both present or both NULL. The core validates the selector, full request membership, and dimension. No credentials in selectors. |
| `coverage_results` | `unit_id ID→requested_units`, `collection_id ID→collections`, `dimension T`, `authority_scope_id ID→authority_scopes`, `outcome T`, `reason T?`, `inspected_locator B?` | Unique `unit_id` and `(id,collection_id,outcome,dimension,authority_scope_id)`. Composite FK `(unit_id,collection_id,dimension,authority_scope_id)` references the corresponding requested-unit tuple. Outcome complete, partial, failed, or inconclusive; non-complete requires a reason. |
| `collection_closures` | **PK `collection_id ID→collections`**, `outcome T`, `finished_at U`, `reason T?`, `result_digest B` | Outcome uses the same four values; non-complete requires a reason; digest length 32. Core validates aggregate outcome against the original request and actual results. |
| `evidence_records` | `collection_id ID→collections`, `origin_kind T`, `origin_namespace T`, `locator_kind T`, `locator B?`, `collected_at U`, `source_effective_at U?`, `source_revision T?`, `valid_until U?`, `interpretation_rule_id ID→rule_versions`, `item_key T` | Unique `(collection_id,item_key)` and `(id,collection_id)`. Origin external or user_assertion. Locator kind path, opaque, or unavailable; unavailable requires NULL, others non-null. A user assertion is not storefront proof. |

Evidence can support several propositions, but each claim has its own basis and
value. No raw launcher configuration, credential, or complete source payload is
required. Source revision tokens carry no implicit numeric order. Cached source
age remains separate from collection time; unknown source age stays unknown.

Register a collection and its original requested units together. Final application
of evidence, coverage, claims, decisions, and the closure is a separate atomic
transaction. Insert the closure last. Proposed insert guards reject new units,
results, evidence, claims, negative justifications, and collection-linked decisions
after closure. A request digest is checked in the core so registration cannot be
silently shrunk.
No closure means pending/incomplete operation, not an empty authoritative result.
Cold recovery reports inconclusive collection progress with prior library evidence
intact; it does not assert that a pending source operation completed.

Idempotency checks compare the same collection/result identity and immutable
content, including original timestamps. A different payload under the same identity
is rejected/diagnosed. Do not globally deduplicate equal payloads: a genuine later
observation can provide different freshness evidence. None of the digests is used
as an authority or ordering proof.

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
`negative_justification_id ID?→negative_justifications`.

Composite FKs match subject `(id,dimension,authority_scope_id)` and evidence
`(id,collection_id)`. Unique parent keys are `(id,dimension)` and
`(id,subject_id,dimension,authority_scope_id)`. Knowledge is observed,
confirmed_negative, or unknown. Disputed and stale are derived conclusions, not
new source assertions. The following CHECK branches fully define allowed payloads:

| Dimension | Entity FK | Observed `value_text` | Confirmed negative `value_text` |
| --- | --- | --- | --- |
| game_label | game | Nonempty display label | Not allowed. |
| entry_title | entry | Nonempty reported title | Not allowed. |
| entry_edition | entry | Nonempty reported edition | Not allowed. |
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
| `negative_justifications` | `subject_id ID→claim_subjects`, `dimension T`, `authority_scope_id ID→authority_scopes`, `evidence_id ID→evidence_records`, `collection_id ID→collections`, `kind T`, `coverage_id ID?→coverage_results`, `required_outcome T?`, `capability_rule_id ID→rule_versions`, `basis T` | Composite FKs match subject scope and evidence collection. Unique `(id,subject_id,dimension,authority_scope_id,collection_id)`. Kind explicit or exhaustive_omission. Explicit has NULL coverage/outcome; omission requires coverage, outcome literal complete, and composite FK `(coverage_id,collection_id,required_outcome,dimension,authority_scope_id)` to the corresponding coverage tuple. |
| `reconciliation_decisions` | `kind T`, `rule_id ID→rule_versions`, `ordering_rule_id ID?→rule_versions`, `assessed_at U`, `reason T`, `collection_id ID?→collections` | Kind supersession, supersession_rejected, or dispute. Supersession requires an ordering-rule reference. Deferred FK from `id` to `decision_seals.decision_id` requires completion by commit. Rule meaning and scope compatibility are core checks. |
| `decision_claims` | **PK `(decision_id,claim_id,role)`**, `decision_id ID→reconciliation_decisions`, `claim_id ID→claims`, `role T` | Role older, replacement, identity_support, ordering_support, negative_support, or conflicting. A role alone is not proof that the claim establishes it. |
| `supersessions` | **PK `(older_claim_id,replacement_claim_id)`**, `older_claim_id ID→claims`, `replacement_claim_id ID→claims`, `dimension T`, `decision_id ID→reconciliation_decisions` | Different endpoints; composite FKs `(older_claim_id,dimension)` and `(replacement_claim_id,dimension)` reference claim `(id,dimension)`. Multiple edges can share a replacement. |
| `decision_seals` | **PK `decision_id ID→reconciliation_decisions`**, `content_digest B` | Digest length 32. Seals the decision's metadata, claim-role links and edges. Insert after those records, within the same transaction. |

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

Before inserting a supersession, the core proves every approved validity condition
and includes the cited proof claims. Where distinct handles are involved, it must
prove subject compatibility through valid source identity evidence; equivalence of
source entries alone does not equate two accounts, installations, or entitlements.
Propose an insertion guard that rejects a new edge when a path already exists from
replacement back to older. The core also validates the whole graph on reading;
invalid/cyclic stored decisions produce scoped diagnostics and no definitive winner.
This is a proposed guard obligation, not an implemented or tested recursive query.

Unordered contradiction creates a dispute decision with conflicting claim links,
not a supersession. Decision timestamps record reasoning, never source ordering.
Corrections require new evidence and explicit decisions; this proposal supplies
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
not resolved by their row order. No game-session or playtime table is proposed.

## Index and guard catalog

In addition to the primary, unique, and partial-unique indexes specified above,
propose non-unique indexes for every FK's child columns in the declared order,
unless an existing index already has those columns as its leftmost prefix. Add
`claims(subject_id,evidence_id)`, `evidence_records(collection_id,collected_at)`,
`supersessions(replacement_claim_id,older_claim_id)`,
`handoff_records(attempt_id,recorded_at)`, and
`process_observations(attempt_id,observed_at)`. Time indexes support retrieval, not
factual precedence. No full-text/artwork index or performance claim is included.

The proposed guards are: reject update/delete on immutable catalog records; reject
new collection content after closure; reject additions to sealed decisions; and
reject cyclic supersession insertion.
They abort rather than mutate the row being checked. Closing a collection after
adding its content in one transaction supports the collection guards. Decisions
use the one deliberately deferred completion reference specified above.
Cross-table semantic meaning still belongs to core validation.
Startup/read validation must not assume constraints prove the file is uncorrupted.

## Transaction and reconstruction obligations

| Operation | Proposed atomic boundary / recovery behavior |
| --- | --- |
| Manual creation/grouping | Identities, user-assertion evidence, and claims together; revision preserves prior evidence. |
| Collection registration | Collection and original requested-unit records together. No library conclusions are replaced. |
| Bounded result application | Independently valid evidence/claims, coverage, negative proofs, decisions, edges, and closure together. Failed/partial outcomes retain prior claims in gaps. |
| Launch intent | Attempt row committed before external operation. If this fails, no external handoff is issued. |
| Handoff result | Persist actual result separately. If persistence fails after submission, report uncertainty about durable recovery, not that submission never happened. |
| Process observation | Observation and its identity/continuity references together, independently of handoff. |

No database transaction spans filesystem enumeration, command waiting, or launcher
execution. Blocking storage work also stays outside GTK's UI thread. Large-result
batching is not proposed; it would need explicit consistency/coverage rules.
Connection ownership, write-transaction mode, durability and busy handling remain
independent decisions.

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
that SQLite has accepted the proposed constraints:

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
identity conflicts; and reconstruction determinism. No such executable work is
authorized or represented as complete by this document.

## SQLite evidence and remaining gate

Primary documentation inspected for this proposal:

- [STRICT tables](https://www.sqlite.org/stricttables.html): support begins at
  3.37.0 and permitted type names are restricted. This motivates the proposed
  baseline and explicit type notation; it does not select a distribution minimum.
- [CREATE TABLE](https://www.sqlite.org/lang_createtable.html): NULL passes a CHECK
  and NULLs are distinct for uniqueness; explicitly constrain null shape and keys.
- [Foreign keys](https://www.sqlite.org/foreignkeys.html): enable and verify them
  per connection; nullable composite children can skip enforcement, and parent
  tuples need matching primary/unique keys and collations. Mere creation of a
  table is not sufficient verification of every foreign-key relationship.
- [Triggers](https://www.sqlite.org/lang_createtrigger.html): rejecting guards can
  raise constraint errors. Proposed guards must not modify the row they inspect.
- [Transactions](https://www.sqlite.org/lang_transaction.html): database atomicity
  and single-writer behavior do not determine application connection ownership or
  make an external launcher action atomic with storage.

The review gate is approval/revision of this physical catalog and its proposed
mechanisms, with explicit decisions on encodings and SQLite baseline. Connection,
durability, migration/recovery and retention details remain open as listed above.
No current ownership, integration success, executable correctness, or publication
clearance follows from documenting the schema. Implementation needs separate
authorization after the relevant design decisions are settled.

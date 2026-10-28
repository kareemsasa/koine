# Approved evidence retention and archival policy

**Status: Approved initial design; selected offline retention/copy checks now recorded.**
Originally prepared 2026-10-27 as a documentation-only engineering proposal.
Owner design-level approval is recorded 2026-10-27 from the explicit instruction
to record the initial retention and copy-only archival policy, its nonblocking
limitations and remaining open decisions. This establishes neither independent
inspection of local Git state nor an executable result. Approval does not modify
the approved catalog or authorize implementation, database operations, deletion,
migration, executable tests, commit or publication.

The [logical mapping](persistence-mapping.md),
[physical design](sqlite-schema-proposal.md),
[operational policy](sqlite-operational-policy.md), and
[migration policy](sqlite-migration-proposal.md) retain their recorded authority.
Migration approval is documented at design level, with metadata approved in
principle and independent review required for each actual transition. The existing
24 [acceptance specifications](acceptance-cases.md) remain unchanged and now have
passing executable translations in the separately owner-authorized
[offline foundation](executable-foundation.md). Selected retention, backup and
recovery checks pass; full operational correctness remains unestablished. This
later execution does not retroactively change the original approval evidence.
[Milestone 2](storage-resilience-steam.md) adds executed backup-cancellation,
failed-copy isolation and corrupted-candidate checks without expanding retention
or restoration authority.

## Decision and provenance boundaries

The owner directed the scope of this proposal and excluded implementation and
publication. Existing preservation requirements come from the approved contracts
cited below. Retention choices, eligibility rules and archival recommendations
originated as engineering inferences. Owner approval establishes the initial
design obligations identified below without changing their engineering provenance.
Future pruning, cold relocation, inventory formats and administrative mechanisms
remain subject to separate decisions; they are not approved by implication.
SQLite references support only the attributed engine behavior; they do not
establish a working Koiné mechanism.

The question is which information Koiné must preserve to explain and recover its
conclusions, which additional history it may retain, and whether moving or
deleting information can leave those guarantees intact. Age, staleness, hidden
visibility, supersession, and backup availability are different facts. None alone
makes evidence dispensable.

## Approval assessment

| Area | Owner disposition | Approved initial policy and boundary |
| --- | --- | --- |
| Structured evidence retention | Approved at design level | Retain all committed structured identity, evidence, claims, collections, diagnostics, decisions, launch/process records and database history in the live database. No automatic age-, count-, size- or staleness-based pruning and no ordinary physical deletion. |
| Referenced evidence | Approved at design level | Preserve full dependencies, supersession bases, original canonical bundle membership, protocols and seals. Uncertain eligibility means preservation; a digest, summary or archival copy never substitutes for required evidence. |
| Raw material and optional caches | Approved initial boundary | No raw source payload collection without separate justification/review. Optional assets, derived stores and temporary files require explicit ownership and cleanup rules; no cache subsystem or cleanup mechanism is approved here. |
| Manual lifecycle | Existing semantics retained | Removal remains a reversible, scoped hidden visibility claim with user provenance and justified supersession; restoration preserves the stable handle and original evidence times. Neither operation purges evidence or changes external launcher facts. |
| Copy-only archival | Approved at design level | Use independently validated full-database copies under the approved Online Backup API contract. Keep live records and local references intact; no relocation, source pruning or automatic restore follows copying. |
| Storage pressure and future changes | Approved initial boundary | Report storage pressure and actual transaction outcomes without discarding evidence or bypassing guards. Selective pruning, cold relocation and explicit erasure require separate owner decisions and reviewed mechanisms. |

Approval covers the initial policy, not executable correctness, implementation
readiness of every administrative feature, or future deletion mechanisms. The
retained alternatives and paper review scenarios remain design rationale and
validation obligations, not executed checks.

### Nonblocking limitations of the approved initial design

These documented limitations do not block design-level approval. They do not waive
the decisions or validation required before implementing the affected function.

- Structured history, indexes, WAL work and backup storage can grow; the policy
  promises no bounded database size or measured performance. Archival copies
  duplicate evidence and do not reduce live storage.
- Some retained structured history might eventually be proved optional. Keeping
  it initially is a conservative choice, not a requirement for unlimited raw
  collection or an assertion that every old row is intrinsically mandatory forever.
- Copy-only archival is not cold relocation, restore tooling or a guarantee that
  an artifact remains readable indefinitely. Recovery still depends on compatible
  schemas, original protocols, supported rule artifacts and later validation.
- Hiding, logical deletion and compaction do not establish complete erasure from
  archives, backups, filesystem snapshots or external sources. No secure-erasure
  guarantee is selected.
- Archive administration, optional-cache cleanup, restore procedures and any
  future pruning/relocation mechanisms remain unselected as detailed below.
  Their absence does not reopen the initial design decision or authorize work.

## Terms

| Term | Meaning in this proposal |
| --- | --- |
| Mandatory preservation | Information needed to satisfy the approved identity, provenance, reconciliation, manual lifecycle and recovery contracts. Preservation lasts while those obligations or dependencies remain; it is not a universal requirement to collect every raw source file forever. |
| Optional retention | Additional material or history whose absence can be proved not to weaken a retained fact, proof or recovery obligation. Optional metadata as a feature is not automatically optional evidence once its claims are stored. |
| Active library | The entries currently presented under visibility and grouping rules. Exclusion from this view changes neither evidence existence nor storage location. |
| Archival copy | An independently validated historical snapshot retained separately. The live database keeps its evidence; no reference is redirected to the copy. |
| Cold relocation | Moving evidence out of the live database so an archive becomes necessary to interpret retained records. This requires a new storage/reference design. |
| Pruning | Physical removal of selected stored records or payloads. This is distinct from hiding, unlinking, making a copy, or reclaiming unused pages. |
| Backup / restore | A recovery copy and the separate act of replacing a live state from a copy. A historical archive can also be a backup if it meets the backup contract; its name does not prove recoverability. |

## Preservation floor from approved contracts

These are existing obligations. The approved initial retention policy below is
deliberately broader than this floor because no safe pruning mechanism is approved.

| Information | What must survive and why |
| --- | --- |
| Stable identities and relationships | The five concepts, scoped source keys and repeatable key observations, contexts, identity profiles, original reported forms, and creation evidence. Grouping, unlinking and equivalence must preserve original handles and source-specific facts. See domain D1–D3 and the logical identity contract. |
| Current, last-known, unknown and disputed claims | Values, reasons, source and collection times, validity limits, authority scope and claim-level basis. Failed refresh, unsupported interpretation, staleness or conflicting reports cannot erase prior evidence or manufacture a negative. See D6–D8 and R1–R2/R5–R7. |
| Supersession history | Both older and replacement claims, decision metadata, ordering/identity/negative support, edges, seals and referenced rules. R4 and the logical supersession contract require the older basis and replacement reason, not just a winning value. |
| Bounded negative proofs | Original request, coverage results and complete part trees including gaps/residuals, source capability/selector rules, attributed diagnostics, closure and negative justification. A saved complete flag or omission count cannot replace the proof required by R3. |
| Verification membership | Exact records and fields in each retained request/result/decision manifest, original protocol tags and digests, and referenced bodies needed for independent semantic validation. A digest is not a substitute for its preimage or its referenced evidence. |
| Manual lifecycle | User provenance, stable manual key/library scope, visible/hidden assertions, earlier visibility claims and causal supersession basis. Removal/restoration must remain distinguishable from external revocation or uninstall. |
| Launch and process recovery | Attempt intent, actual handoff results including durable failure status, independent process observations, identity/continuity references and transport/method rules. Intent-only and definite failure must stay distinct; historical positives cannot become current running claims. |
| Incomplete work and database history | Registered but unclosed collections and their original scope; immutable database identity and committed migration history under the migration policy. Interruption cannot be converted into successful completion or permission to replay an external action. |

The [domain evidence contract](domain-model.md#evidence-belongs-to-claims) does not
require entire source files, credentials or unlimited raw history. The physical
catalog stores structured claims, locators, rules and diagnostics rather than raw
launcher configuration. A locator records provenance; it is not a promise that an
external file remains available or unchanged. Re-reading that file later cannot
replace the original stored basis or renew its source time.

An optional artwork/title claim still needs its provenance while retained. Cache
image bytes may be disposable when no proof depends on those exact bytes; the
stored metadata claim and any sealed diagnostic are a separate matter. A datum
cannot be called a cache merely to avoid a preservation obligation. If a future
interpretation needs exact raw bytes to substantiate a claim, its reviewed evidence
design must preserve those bytes or sufficient structured evidence before treating
the claim as justified. Raw capture is not introduced by this proposal.

## Alternatives and consequences

| Alternative | Benefit | Provenance, recovery and implementation cost | Assessment |
| --- | --- | --- | --- |
| Keep all structured evidence in the live database; make validated full snapshots | Fits immutable records and local references. Complete prior bases, duplicate-delivery comparisons and crash interpretation remain available without another storage tier. | Database, indexes, WAL work and backup storage grow; there is no bounded-size promise. Snapshots duplicate personal library evidence. | Approved initial policy. Avoid collecting unnecessary raw material; optional caches can have separate future policies. |
| Keep only the latest value, or prune all evidence older than a fixed interval such as 90 days | Predictable-looking history window and simple selection. | Can erase last-known claims, disputes, supersession endpoints, omission proofs and durable handoff failures. Age does not prove dispensability. Keeping a digest or summary cannot verify deleted bundle members. | Reject for authoritative evidence. No global retention or freshness interval is selected. |
| Prune only closed, dependency-independent historical units after a proof of eligibility | Could reduce storage while preserving the evidence that remains required. | Requires semantic graph analysis, exact bundle membership, replay guarantees, maintenance authorization and independently reviewed guard handling. Shared identity/creation references may make the removable set very small or empty. | Plausible later work, not an approved deletion path. A graph with no obvious FK references is insufficient. |
| Relocate complete historical units to a separate SQLite archive and resolve references across stores | Could reduce live working-set size while retaining history. | Needs archive identity, availability, schema/protocol support, reference resolution, joint snapshots, backup/restore and crash rules. Existing local FKs and seals do not acquire these properties by moving rows. | Defer until measured need and a separate reviewed design. Whole-database snapshots provide archival copies without this dependency. |
| Replace old history with checkpoints or compact summaries | Smaller reconstruction input and potentially lower storage. | A checkpoint of current state loses original claim bases, conflicts and ordering explanations unless a new proof protocol establishes equivalent guarantees. It cannot verify old canonical manifests. | Reject under current contracts; would require an explicitly approved change to historical guarantees and protocols. |

The approved initial policy retains some structured history that might eventually
be proven optional. That is a conservative policy choice, not a claim that every
old row is intrinsically mandatory forever. Unlimited raw capture is unnecessary,
and keeping structured history is not a commitment to an event-sourcing system.

## Approved initial policy

The following are approved design obligations for a separately authorized
implementation:

1. Retain all committed structured identity, evidence, claim, collection,
   diagnostic, decision, launch/process and database-history records in the live
   database. No automatic age-, count-, size- or staleness-based pruning; no
   ordinary user operation physically deletes these records.
2. Keep raw source payload collection disabled unless separately justified and
   reviewed. Optional downloaded assets, derived indexes and temporary files need
   their own explicit ownership and regeneration rules before cleanup is offered.
   This proposal adds no cache subsystem or cleanup job.
3. Treat manual removal as the approved reversible visibility operation. Provide
   no implication that hiding removes history or erases personal data.
4. Offer archival copies only through the approved validated full-backup model
   when implementation is separately authorized. Keep live references local;
   archival success never authorizes source pruning.
5. On storage pressure, report the problem and the actual transaction outcome.
   Do not silently discard evidence, disable guards, split an atomic result, or
   promise that future writes will fit. Existing library evidence remains subject
   to ordinary recovery/validation rules.
6. Revisit selective pruning or cold relocation only with concrete storage evidence,
   a dependency-preservation proof, independently reviewed schema/maintenance
   mechanisms and explicit owner approval. Approval of this policy alone
   authorizes none of those mechanisms or their implementation.

No retention duration is selected for optional caches or archival/backup copies.
Freshness is assessed by its explicit policy and evidence, independently of how
long bytes are retained. Backup scheduling, destination, rotation, encryption,
durable-finalization details and general restore tooling remain separate decisions.

## Referenced evidence and deletion eligibility

The approved initial eligibility rule is conservative: uncertainty about a dependency
means preservation. Under the initial policy, structured records are
ineligible for physical deletion even if a future analysis might find them optional.
The analysis below bounds a future pruning proposal; it does not override guards.

### Preservation closure

A future eligibility analysis would begin with every retained identity, current
or last-known conclusion, unresolved conflict, manual lifecycle, launch/recovery
record, incomplete operation, required historical explanation and database-history
obligation. It would then expand repeatedly to include:

- All referenced subjects, contexts, keys and key observations, creation evidence,
  authority scopes, identity/equivalence support and rule/profile versions.
- Every decision's claims, roles, edges, ordering/identity/negative proof and seal,
  including both ends and the relevant chain of supersessions. Evidence required
  to explain supersession cannot be discarded because it is superseded.
- Every evidence record's collection, registered request, coverage tree,
  limitations/diagnostics, negative justifications and closure needed to interpret
  it. Incomplete collections stay incomplete; they are not expired successes.
- Every member of any retained canonical request, result or decision bundle,
  including members not used by the currently displayed claim, followed by those
  members' dependencies. Result membership includes diagnostics and key
  observations, not merely claims.
- Attempt targets, handoff/process rule references and prior process observations
  needed for identity continuity, durable outcome interpretation and duplicate
  delivery comparison. Open or ambiguous attempts cannot be discarded as old work.

Repeat until no required members are added; cycles in the dependency graph do not
permit omitting a member. Shared rows may connect several collections. Retaining
one collection or decision can therefore retain more than its immediate payload.
This is an engineering analysis requirement, not an implemented graph algorithm.

A matching decision seal hashes references, not the recursively referenced claim
bodies. Those bodies still need preservation and validation. Conversely, deleting
an otherwise unused diagnostic from a closed collection changes its result
manifest. Preserving just the digest, replacing a record with a tombstone,
recomputing an old digest or resealing a reduced bundle cannot make the original
proof valid. See the approved
[canonical protocols](sqlite-schema-proposal.md#canonical-digest-framing-and-seal-verification).

### Eligibility by category

| Candidate | Proposed disposition |
| --- | --- |
| Stale, superseded, hidden, disputed, unknown or failed structured evidence | Preserve. These labels are not deletion criteria; they often identify the information recovery needs. |
| An apparently orphaned structured row, closed collection or completed attempt | No deletion under the initial policy. Future eligibility requires proof that it is outside every preservation closure, is not contractually required history and is not needed for delayed/duplicate delivery handling. Unknown rule/protocol meaning blocks that proof. |
| Diagnostic included in a sealed result, even if its text looks like a log | Preserve exact content and attribution. Cleanup cannot alter manifest membership. New diagnostics must be sanitized at capture under the existing contract. |
| Derived projection, search index or thumbnail outside authoritative records | Potentially disposable only when authoritative inputs suffice to recreate it and removal cannot affect identity, truth, launchability or a digest. No such store is defined here. |
| Unreferenced downloaded asset bytes or redundant raw attachment outside sealed records | Potentially optional, after proving no retained claim, proof or recovery obligation requires those bytes. Asset loss may reduce presentation quality; it cannot invalidate a supported launch target. No raw attachment store is selected. |
| Temporary export/backup that failed or was cancelled | Not a usable archive. Future cleanup must distinguish it from live/recovery files and valid copies, preserve needed failure evidence and avoid overwriting the last good artifact. No cleanup tooling is authorized here. |
| Valid archive, routine backup or migration safety copy | Rotation is a separate policy. Proposed minimum: do not remove the only validated copy needed by an unresolved migration, recovery or investigation. A pre-upgrade copy alone does not recover facts committed afterward. |

Any later structured-pruning design would also need a validated pre-change safety
copy, source correspondence checks, a complete removal inventory, preserved
identities/protocols, unchanged retained reconstruction under identical inputs,
and pre-commit/post-reopen validation. It must identify exactly how enabled
constraints and immutable-record guards are respected. The approved migration
exception is for reviewed representation changes preserving historical facts; it
is not retention permission. Disabling FKs, dropping guards ad hoc, replacement
inserts or cascades are not an available pruning mechanism.

A future deletion inventory or audit record would itself require a reviewed
schema/protocol and must not reopen or reseal old collections. A record saying
something was deleted cannot stand in for evidence still required by a conclusion.
Neither a backup nor a user request to hide an entry satisfies these conditions.

## Archival mechanisms

| Mechanism | Concrete behavior and limits | Recommendation |
| --- | --- | --- |
| Full SQLite archival snapshot through the Online Backup API | Copies the complete database; use the operational policy's completion, reader fairness, reopen and structural/FK/canonical/semantic checks. Retains original record IDs, database identity, history, protocols and source times. Requires storage for the additional copy. | Initial archival mechanism. Never delete live evidence as a follow-up. |
| Read-only historical view in the live database | Hides old records from routine presentation while keeping them available to reconstruction. Does not reduce database storage and is not an independent recovery copy. | Compatible presentation option; detailed UI and queries remain outside this proposal. |
| Selective export to JSON or another interchange format | Can be useful for inspection or portability, but omitting typed bytes, original IDs, rules or exact membership loses verification/recovery information. A readable export is not automatically a restorable database. | No archival-authority claim until a reviewed lossless format and importer are validated. |
| Separate cold SQLite database with removal from live storage | Would make the archive part of the authoritative evidence graph; existing local references cannot simply point to a filename. Needs independently reviewed schema, availability, joint consistency and restore rules. | Defer; a validated copy does not solve relocation. |
| VACUUM or VACUUM INTO | Repacking is a physical-space operation; VACUUM INTO produces a copy with the same logical content. Neither chooses dispensable evidence or supplies a semantic pruning proof. | Not a retention mechanism or replacement for the approved backup contract. Any future maintenance use needs its own authorization and validation. |

SQLite documents incremental full-database copying in the
[Online Backup API overview](https://www.sqlite.org/backup.html). The approved
Koiné backup policy additionally requires completion at `SQLITE_DONE`, independent
validation and durable finalization before success; see the
[API completion rules](https://www.sqlite.org/c3ref/backup_finish.html).

For a future archival facility, recommend a companion inventory identifying the
archive artifact, database UUID, schema/history position, copy/validation times,
supported protocols/rule artifacts, content digest and validation outcome. Exact
inventory encoding and durable binding to the artifact remain unselected. Copy
time is not observation time and does not promise an exact request-time snapshot.
Verification after a copy is made does not prove that the copy remains readable
indefinitely. A content digest detects differences against a trusted recorded
digest; it does not authenticate source facts or protect against coordinated edits.

No archive-only dependency is introduced into ordinary reconstruction. A missing
optional snapshot therefore does not change live conclusions. In a future cold
relocation design, missing/corrupt archives must instead withhold affected
conclusions or block operation where isolation cannot be proved; they cannot mean
false, absent, revoked or successfully purged.

SQLite restricts foreign keys to a schema, and WAL transactions across attached
databases are not atomic as a set. A separate archive would need explicit
application-level reference and recovery rules, rather than assumed cross-file
FKs or a single commit. These engine limits are documented in
[foreign-key limitations](https://www.sqlite.org/foreignkeys.html#limits_and_unsupported_features)
and [WAL limitations](https://www.sqlite.org/wal.html#overview). This is the
engineering reason to defer relocation, not evidence that a particular archive
implementation has been tested.

## Manual removal, restoration and explicit destruction

The approved [manual lifecycle](sqlite-schema-proposal.md#manual-entry-lifecycle)
already defines removal as appending a hidden `manual_visibility` claim with user
evidence and justified supersession in one transaction. Preserve the entry, key,
library scope, prior visibility, title/access claims, links, targets, decisions and
launch evidence. Restoration appends an explicit visible claim for the same stable
manual handle, with the same provenance and decision safeguards.

Only the applicable manual handle in that library context is excluded from the
active view. Removal does not propagate through game grouping or source
equivalence, hide matching Steam/Heroic entries, uninstall anything, revoke access,
stop a process or delete evidence. Unlinking is separate. Unordered visibility
assertions remain disputed; missing visibility evidence means unknown. A scan
cannot restore a hidden manual entry or author a manual visibility claim.

Restoration restores visibility, not freshness or access. Prior claims keep their
source times and authority. Retention and archival cannot themselves restore an
entry, infer a removal or suppress a newly discovered independent source entry.
The approved initial policy preserves the evidence needed for restoration
without promising that an external launcher still has the target or installation.

If a user deliberately requests permanent erasure, that is a different product
operation requiring a separately approved scope and design. It must state which
history and recovery guarantees would be surrendered, what related records and
copies are affected, and whether later discovery can recreate facts. This proposal
does not invent a purge command or reinterpret ordinary removal as that consent.
Manually editing/deleting database rows outside Koiné is not a supported removal
operation; missing/invalid records trigger recovery diagnostics, not an invented
user action or external negative. Deleting a whole database likewise does not
prove external uninstall/revocation, and an expected missing database must not be
silently recreated as though no history existed.

Logical deletion, freed-page reclamation and secure erasure are separate claims.
SQLite's [VACUUM documentation](https://www.sqlite.org/lang_vacuum.html) describes
free-page reuse, repacking and removal of deleted content from a rebuilt database.
That does not establish erasure of backups, archives, filesystem snapshots or
external source files. Koiné must not promise complete erasure from a hide action,
row deletion, compaction or removal of one copy. No secure-erasure mechanism is
selected here.

## Recovery consequences

| Event | Required or proposed consequence |
| --- | --- |
| Ordinary restart or crash | Existing operational recovery applies using the database and associated files. Preserve incomplete collection scope, intent-only uncertainty, durable status-127 failure and historical process evidence; never auto-replay handoffs. Retention does not renew freshness. |
| Missing or invalid referenced evidence/seal | A surviving value/digest is insufficient. Withhold the affected conclusion with diagnostics; structural corruption blocks normal writes under the operational policy. Do not guess a prior value or silently fetch a replacement as if it were the original. |
| Archival copy fails, is cancelled or fails validation | Live evidence remains authoritative and unchanged. Report no usable new archive; do not prune, overwrite a good copy or treat successful cleanup as completed backup. |
| Archive is later lost or corrupted | Under the approved copy-only design, live reconstruction remains possible; historical backup coverage is reduced. Preserve other good copies. No automatic restore or live-history deletion follows. |
| Restoring an older valid snapshot | Would recover only its captured evidence. Later observations, manual removals/restorations and launch results may be absent; a hidden entry can appear visible according to that older history. This is recovery rollback, not a fresh user action. Restore tooling must disclose the gap and validate compatibility, proofs and reconstruction before ordinary work; it remains separately unapproved. |
| Snapshot has intent but lacks a later handoff result | Issuance remains unknown and handoff inconclusive even if the external action happened after the snapshot. Never replay it. An old process observation is historical, not current running evidence. |
| Migration fails before/around commit or after reopen | The approved migration recovery gate applies. Preserve source/target files and validated safety copy as appropriate; inspect immutable identities/history. Failed post-commit semantic validation stays blocked without automatic restore. Archival approval does not widen migration permissions. |
| Disk full or uncertain database commit | Keep outcome uncertainty, inspect immutable identities/content and follow the operational error policy. No emergency evidence purge, fabricated rollback or replay of launcher submission. |

Retained bytes alone are not sufficient for recovery: the reader needs compatible
schemas, original canonical manifests and supported rule implementations. Preserve
their recorded versions/digests; archive compatibility and availability of the
corresponding release artifacts need a future operational plan. Unsupported
historical rules keep their bytes and explicit uncertainty; do not reinterpret or
reseal under a newer rule. No automatic merging of restored copies, archive
imports or divergent histories is selected.

Checkpointing concerns SQLite recovery/log management; it is not historical
evidence pruning. Never delete or rename live recovery sidecars as an archival or
cleanup shortcut. SQLite treats the WAL as part of the persistent database state;
see [WAL file handling](https://www.sqlite.org/wal.html#the_wal_file). The approved
operational policy remains authoritative for checkpoints and recovery.

## Paper review obligations and remaining gate

These are additional design-review scenarios, not additions to the numbered
acceptance suite, executable tests or completed validations.

| Review scenario | Expected policy result |
| --- | --- |
| Old positive replaced by a justified bounded negative | Keep both claims, scope/order justification, complete bounded coverage, relevant gaps/diagnostics and seals. No age-based pruning of the older endpoint. |
| One useful claim in a collection containing unrelated diagnostics/observations | Preserve the complete canonical result membership and its dependencies, not just that claim's row. |
| Failed refresh, unknown source age, unordered conflict or unresolved account/identity scope | Preserve original times, last-known facts and conflict/isolation evidence; different accounts remain independent, and neither retention nor cleanup produces a new negative or a winner. |
| Hidden manual entry with a matching Steam title, followed by restoration | Same manual handle/history; Steam remains independent, and restoring visibility does not renew evidence. |
| Completed failure, ambiguous handoff and process observation with prior continuity evidence | Preserve independent attempt results and observation links. Cleanup cannot turn failure into intent-only uncertainty or historical observation into a current claim. |
| Repeated or delayed collection/attempt delivery after years | Initial retention keeps identities/content available for comparison; no duplicate facts, freshness renewal or external replay. Future pruning must define safe behavior before weakening this guarantee. |
| Candidate appears unreferenced but uses an unsupported protocol/rule | Eligibility cannot be proved; preserve and diagnose rather than delete. |
| Asset bytes disappear while valid target evidence remains | Optional presentation can degrade; access/installation/target validity and launch outcome stay independent. |
| Cancellation, storage error or crash while producing an archival snapshot | Source survives unchanged; incomplete destination is not reported usable and no evidence removal follows. |
| Restore predates manual removal or a durable handoff failure | Disclose missing later history, preserve uncertainty and avoid handoff replay. No inferred new visibility action or invented failure result. |

The initial retention and copy-only archival design-review gate is satisfied.
The following administrative/extension decisions remain open; the offline report
records the caller-selected local copy/finalization mechanism already implemented:

- Archival inventory encoding, artifact identity/digest binding, and validation
  records; the companion inventory above remains a future design recommendation.
- Backup/archive scheduling, default destination selection, rotation, encryption,
  general/portable durable-finalization policy and retention durations for copies.
- Restore and repair administration, compatible release/rule artifact availability,
  and any import or divergent-history reconciliation mechanism.
- Optional-asset, derived-store and temporary-file ownership, regeneration and
  cleanup rules; any raw capture still requires separate justification/review.
- Any structured-pruning exception, removal inventory/audit protocol, guard and
  maintenance mechanism, or cold-storage reference/consistency/recovery design.
- Explicit permanent-erasure scope, semantics and treatment of related copies and
  surrendered recovery guarantees.

No age window, pruning mechanism, cold relocation or secure-erasure mechanism is
approved in advance. Existing migration-specific and packaging/runtime decisions
remain open under their own approval records.

The later executable-foundation instruction separately authorized offline code,
disposable databases, synthetic checks and local commits. Selected canonical,
guard, retention and backup checks now pass; the report identifies remaining
operational and migration validation. Future retention/archive extensions still
need their own design authority, conformance and failure-recovery checks.
Documentation review, link checks and diff inspection
establish consistency only. They do not establish executable correctness or
publication clearance.

## Decision history

- **Proposal prepared, 2026-10-27:** Owner-directed documentation-only scope;
  alternatives, preservation/deletion criteria, archival mechanisms, manual
  semantics and recovery consequences prepared as engineering recommendations.
  Retention approval remains pending. Existing migration approval and all 24
  acceptance specifications retain their prior status; no implementation,
  database, runtime test, commit or publication is claimed.
- **Owner design approval, recorded 2026-10-27:** The owner explicitly directed
  recording design-level approval of the initial retention and copy-only archival
  policy. All committed structured evidence remains live, required dependencies
  and canonical membership remain preserved, and validated full archival copies
  authorize neither source pruning nor relocation. Manual visibility semantics
  remain reversible and separate from erasure. The nonblocking limitations and
  remaining decisions are identified above. The policy retains its engineering
  provenance; no independent local Git inspection or executable result is inferred.
  Only documentation recording is authorized. No implementation, database operation,
  deletion, migration, executable test or publication is authorized; the existing
  24 acceptance specifications remain unchanged and unexecuted, and executable
  correctness remains unverified.

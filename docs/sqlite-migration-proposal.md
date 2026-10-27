# Approved SQLite migration and schema-upgrade policy

**Status: Approved at the design level; executable correctness unverified.**
Originally prepared 2026-10-27 as a documentation-only engineering proposal.
Owner approval is recorded 2026-10-27 following independent review of the uploaded
`sqlite-migration-proposal.md` against the approved physical schema, operational
policy and relevant SQLite documentation. The owner found no blocking design
contradiction. This review does not establish independent inspection of local Git
state or any executable result; no repository baseline was supplied for the review.

The policy originated as engineering recommendations. Owner approval establishes
the design obligations below without changing that provenance. Migration metadata
is approved in principle; exact DDL and artifact-digest rules remain open. No
migrations, SQL scripts, application code, databases, or executable tests are
supplied or authorized by this approval or its documentation record.

The [logical mapping](persistence-mapping.md),
[physical design](sqlite-schema-proposal.md), and
[operational policy](sqlite-operational-policy.md) retain their approved authority.
This policy extends them with a version gate, transactional upgrades, interrupted
upgrade recovery, and a gate before normal operation resumes. It does not select
evidence retention, general repair/restore tooling, routine backup administration,
or packaging/runtime supply.

## Approval assessment

| Review area | Owner decision | Approved scope |
| --- | --- | --- |
| Version compatibility | Approved | Exact source versions, upgrade paths, schema catalogs, protocols and rule digests. |
| Migration metadata | Approved in principle | Immutable database identity and append-only committed transition history; exact definitions and artifact-digest rules remain open. |
| Transaction boundary | Approved | One atomic transaction for the entire supported upgrade path. |
| Mandatory safety copy | Approved | Independently validated backup required before transformation. |
| Interrupted recovery | Approved | Durable database state determines whether migration committed. |
| Unsupported versions | Approved | Refuse unsupported schemas without automatic downgrade or recreation. |
| Evidence preservation | Approved | Stable identities, provenance, uncertainty, historical protocols and seals retained. |
| Validation gate | Approved | Full pre-commit and post-reopen checks before ordinary operations resume. |

### Retained implementation conditions

The owner emphasized three existing requirements without requiring conceptual
redesign or authorizing unrestricted schema transformation:

1. **Independent per-transition review.** Every actual migration must identify
   affected records, transformations, constraints, guards and preservation proofs.
   Approval of this policy does not approve future migration scripts or a specific
   guarded-table rebuild mechanism.
2. **Safety-copy correspondence.** Before the write transaction transforms the
   database, verify that source content still corresponds to the independently
   validated backup. A matching version number alone is insufficient.
3. **Recovery after commit.** A committed target that fails reopening or semantic
   validation stays blocked. Automatic restoration of the previous copy is not
   authorized; it could discard subsequently committed evidence.

These conditions remain implementation and conformance obligations, not executed
checks. Implementation authorization and publication clearance remain separate.

## Approved policy and retained alternatives

| Area | Approved design policy | Alternative and tradeoff |
| --- | --- | --- |
| Compatibility | Each release declares one writable target schema and exact supported upgrade sources and protocol/rule versions. | Broad version ranges simplify declarations but can accept untested layouts or interpretations. |
| Upgrade unit | One transaction for the entire supported source-to-target path, including version metadata and history. | Per-step commits permit smaller transactions but expose intermediate durable schemas and complicate recovery. |
| Availability | Upgrade at startup under exclusive application ownership, before workers accept ordinary operations. | Online upgrades require mixed-schema readers/writers and additional compatibility contracts. |
| Safety copy | Require a completed, independently validated pre-upgrade Online Backup API artifact. | Relying on transaction rollback alone offers no separate copy for a committed transformation defect. |
| Interrupted work | Let SQLite recover, then inspect durable schema, history and content before retry. | A separately committed progress flag can disagree with the actual schema. |
| Unsupported versions | Stop normal operation with an actionable compatibility diagnostic. | Best-effort writes, downgrade, or automatic recreation risk evidence loss. |
| Completion | Validate inside the transaction and again after closing/reopening, before releasing the maintenance gate. | Trusting a successful commit omits decoder, guard, configuration and reconstruction checks. |

The cost is startup downtime, backup space and potentially large WAL growth. If a
future database cannot be upgraded within practical storage/time bounds, require a
separate reviewed staged-copy design; do not silently split this atomic boundary.

## Distinct version domains

| Domain | Representation and compatibility rule |
| --- | --- |
| Application release | Human-facing release/build identity. It does not itself determine database compatibility. Multiple releases can share a schema. |
| Database family | A fixed Koiné `application_id` plus immutable database identity metadata. The numeric application ID still needs selection and collision review; a matching ID is identification, not authentication. |
| Physical schema | Positive, monotonically increasing integer in `PRAGMA user_version`; recommend version 1 for the first released schema, including the migration metadata outlined here. No released schema or transition exists yet. |
| Migration implementation | Stable migration ID, exact source/target versions, and a digest of the reviewed migration artifact, recorded in an append-only history. An already released artifact cannot be silently replaced. |
| Evidence encodings | Stored canonical protocol tags, including `koine.request.v1` and `koine.result.v1`, and the physical design's decision/seal protocols. Schema changes do not imply new encodings. |
| Meaning and reconstruction | Exact identity profiles and interpretation, reconciliation, ordering, freshness and transport rules with their artifact digests. A recognized name with an unsupported version/digest is not supported. |
| SQLite engine | Verify the loaded runtime and required features separately from the application schema. Retain STRICT/3.37.0 as feature floor and the approved requirement for a verified WAL-reset-fixed build; an engine version floor alone does not prove a supported build. |

SQLite reserves `user_version` for application use; `schema_version` is its own
schema-change counter. Do not use or write `schema_version` as Koiné's migration
number. See [SQLite version pragmas](https://www.sqlite.org/pragma.html#pragma_user_version)
and [application identification](https://www.sqlite.org/pragma.html#pragma_application_id).

Each release must publish a compatibility manifest containing its target schema,
recognized source layouts, ordered upgrade paths, migration IDs/artifact digests,
expected catalogs (tables, columns, constraints, indexes, triggers and views),
supported protocol/rule artifacts, and engine/build requirements. Validate actual
catalog structure against that manifest; a version integer alone is insufficient.
There is no assumed “last two versions” window, direct jump, or automatic inference
that every lower integer is upgradeable. Release-specific support remains to be
enumerated once executable schemas exist.

## Migration metadata: approved in principle

The identity/history model below has approval in principle as a separate extension
to the physical catalog. Initial DDL and artifact-digest rules still require
review before implementation; no executable definitions are approved here:

- One immutable database-identity record with a generated database UUID, family
  identifier and initial schema version. A backup retains this identity; copying
  it does not establish that two files are the same live database instance.
- Append-only migration-history records containing a unique operation UUID,
  migration ID, source/target schema versions, migration artifact digest,
  application build identity and recorded application time. Time is audit
  information, not proof of source ordering or external observation freshness.
  Paths have unique successive transitions; duplicate or divergent history fails
  validation. Initial creation records a bootstrap transition from 0 to 1.
- History entries describe committed transitions only. Their presence, data
  changes and `user_version` advance together in the same transaction. They do
  not claim that later reopen validation succeeded. External attempt diagnostics
  may report failure, but never decide whether a transition committed.

Use the approved UUID, digest and time encodings. Protect identity and history
with immutable-record guards. `user_version` is deliberately mutable only within
bootstrap/upgrade; normal domain operations cannot alter it. No mutable
“migration in progress” database row or ordinary evidence claim is introduced.
Exact metadata DDL and artifact-digest framing need review before implementation.

## Startup compatibility gate and unsupported databases

Acquire and retain the approved process-lifetime database ownership lock before
opening for maintenance. Keep the UI responsive with storage status, but block
ordinary reconstruction for display, discovery, edits, launch handoff, and process
observation jobs until the gate passes. Perform compatibility inspection with
minimal access and no create-if-missing flag for an existing database. Do not
change journal mode or version metadata before identifying it. SQLite-managed
recovery may need access to associated WAL/recovery files; never bypass recovery
by treating a live or interrupted database as immutable.

| Observed state | Required behavior |
| --- | --- |
| Explicit new-library request at a confirmed unused destination | Bootstrap schema, identity, history and version in one transaction, then use the same validation/reopen gate. Failure cannot turn an existing library into a new one. |
| Existing file with version 0, missing identity/history, or foreign application ID | Refuse adoption or initialization, including an existing empty SQLite file. No implicit legacy format exists. Report unrecognized/uninitialized database. |
| Exact target version and recognized catalog/history | No upgrade. Validate runtime, configuration, encodings, semantics and reconstruction before ordinary work. |
| Recognized older version with a complete supported path | Enter the controlled upgrade sequence below. Being numerically older is insufficient. |
| Older version outside the declared support set, missing transition, or newer version | Stop normal operation. Report detected version and this release's support set; suggest a compatible release or separately reviewed recovery procedure. Never decrement the version or try reverse migrations. |
| Invalid version value, catalog/history mismatch, unknown migration digest, foreign/corrupt file | Stop normal operation and preserve the database and recovery files. Do not patch metadata, recreate the library, or guess a schema from available tables. |
| Unsupported SQLite runtime/build/features | Stop before upgrade or normal writes. Report required versus loaded runtime/build evidence; do not silently weaken WAL/FULL or STRICT requirements. |
| Known schema with an unsupported historical protocol/rule | Preserve the record and withhold affected conclusions with a scoped diagnostic under the approved reconstruction contract. Do not substitute a current decoder. If safe scope isolation or upgrade preservation cannot be proved, block normal operation or the upgrade respectively. |

Unsupported whole-database schemas do not get a normal read-only library mode:
even reading them as current state requires a compatible decoder. A diagnostic
surface may expose safely inspected version information without authoritative
library conclusions. It must distinguish incompatibility, structural corruption,
semantic proof failure, contention and I/O/permission failure.

Application downgrade is not database downgrade. After a successful upgrade, an
older binary must reject an unsupported newer schema. Recovery from a safety copy
requires a separately reviewed restore action with explicit treatment of any
post-upgrade facts; this policy never automatically replaces the live database.

## Controlled, atomic upgrade sequence

1. **Quiesce and inspect.** Upgrade at startup before accepting ordinary work.
   An already running application must finish/resolve accepted semantic operations
   and external handoff outcomes, close snapshots and stop workers, then restart
   through this gate. Do not drop accepted evidence or interrupt a launcher and
   replay it later. Retain process ownership throughout maintenance.
2. **Validate the source.** After SQLite recovery, validate family, source version,
   catalog/history and the entire supported path. Run full structural and
   foreign-key checks and source-version canonical/semantic validation. Retain
   source-version validators in the release for every supported upgrade source.
   A pre-existing invalid proof blocks upgrade; an unsupported historical record
   is allowed only if the manifest explicitly preserves it opaquely and proves
   that affected conclusions remain withheld.
3. **Secure a safety copy.** While domain mutations remain stopped, use the approved
   Online Backup API procedure, require `SQLITE_DONE`, close/reopen and validate the
   destination with source-version validators, and durably finalize it. Identify
   it by database UUID, source schema and a unique attempt identity. Never overwrite
   an earlier successful copy or copy the live main file alone. Require a writable
   private local destination and capacity for backup, transaction/WAL growth and
   temporary rebuilds. Check available space without promising it cannot run out.
   If the copy, validation, finalization, or destination selection fails, stop
   before migration. The mandatory upgrade safety copy is approved as design;
   backup destination and durable-finalization policy still need decisions before
   implementing this step. General backup scheduling and copy rotation remain open;
   the separately approved [initial retention policy](evidence-retention-proposal.md)
   permits archival copies without pruning or relocation of live evidence.
4. **Begin the whole path.** Use the dedicated writer off the UI thread with verified
   WAL/FULL and foreign keys enabled, then acquire `BEGIN IMMEDIATE`. No ordinary
   reader snapshot remains open. Revalidate source version/catalog/history under
   the transaction before applying anything, including confirming that source
   content still matches the validated safety copy. The reader worker may own the
   backup source connection in step 3, but closes it for the schema transition.
   Ownership plus SQLite locking is required; unrelated clients must not modify
   the database.
5. **Transform and validate.** Apply only the release's reviewed transitions in
   their declared order. Each step has exact preconditions, expected catalog and
   data postconditions, and preservation checks. Validate intermediate results
   without committing them. Append history entries and advance `user_version` to
   the final target in this same transaction. The path is one unit: no step may
   commit independently, launch a process, write external launcher configuration,
   switch journal modes, run VACUUM, or depend on atomicity across attached files.
6. **Commit or resolve failure.** Run the pre-commit gate below, finalize statements,
   and commit once. Apply the approved cumulative five-second contention policy;
   it bounds lock waiting, not migration duration. Cancellation before commit
   rolls back the complete path. BUSY with an active commit retries that same
   commit within the remaining budget. I/O, disk-full, cancellation or other errors
   require inspecting transaction state; do not assume rollback or replay the body.
   An uncertain outcome enters interrupted-upgrade recovery below.
7. **Reopen before release.** Close the maintenance connection, reopen reader/writer
   connections and reapply their settings. Run the post-reopen gate below. Only
   then start ordinary workers and report the library available. A checkpoint is
   neither proof of migration success nor an extra prerequisite for a FULL commit.

DDL, transformed content and migration bookkeeping must be transactional in the
single database. SQLite's [transaction documentation](https://www.sqlite.org/lang_transaction.html)
supports this boundary and its error handling; it provides no atomicity with
launcher execution or the separately finalized backup artifact.

## Preserving evidence, protocols and guards

Migration is a reviewed representation change, not new discovery. Preserve stable
IDs, source keys/contexts, original path bytes, evidence times and provenance,
claims, disputes, negative justifications, coverage, closures, decisions, seals,
attempts and process observations. Incomplete collections remain incomplete;
intent-only attempts remain issuance-unknown and handoff-inconclusive. Previously
observed processes remain historical after restart. Never renew freshness, collapse
unknown into false, merge provisional identities, or infer ownership/installation
from the migration. Never issue or replay a launcher handoff.

Keep old canonical tags, exact framed values, record membership, digests and seals
verifiable with their original protocol. Adding a column does not add it to an old
digest's membership. A new protocol applies only to explicitly new records under
its own reviewed rules; do not relabel, recompute or reseal old decisions as though
they were newly justified. A meaning-changing reinterpretation requires a separate
traceable evidence/decision plan. Such a change is outside an automatic structural
upgrade unless separately reviewed and approved.

Immutable-record guards remain active by default. If a transition needs to replace
a guard or rebuild a table, its design must enumerate the affected objects and
copy mappings, preserve every historical fact, and recreate all required guards,
indexes, constraints and seals before commit, within the same transaction. This
design permits only a narrowly scoped maintenance exception to ordinary
immutability, subject to independent review of each actual transition and its
guarded-table rebuild mechanism. Approval of this policy grants no general
update/delete or retention permission. Replacing whole tables must not cause
cascades or silently drop referencing objects. Replacement-style inserts remain
forbidden.

Do not use writable-schema edits, ignored CHECKs, blanket FK deferral or foreign-key
disablement. SQLite's generalized [table-rebuild procedure](https://www.sqlite.org/lang_altertable.html#otheralter)
includes disabling foreign keys outside the transaction. That procedure is not
automatically authorized under Koiné's approved enforcement policy. A transition
that cannot preserve enabled enforcement is unsupported by this initial policy
and needs a separate reviewed exception or staged-copy strategy before release.

## Interrupted-upgrade recovery

Every restart begins gated. Let SQLite recover from the database and associated
files; never delete/rename sidecars, manually finish partially executed DDL, or
trust a UI progress message as evidence of commit. Inspect the database with the
same release manifest and validators after recovery.

| Interruption or durable result | Required recovery |
| --- | --- |
| Before migration begins, including incomplete/failed backup | Source remains the candidate live database. Partial backups cannot be reported as usable. Validate source and create/validate a safety copy before any new attempt. |
| During DDL, copying, history insertion, or version update before commit | SQLite recovery should expose the complete source state, with no committed entries for this attempted path. Confirm source catalog, content and history; then a fresh transaction may retry. This rollback behavior requires fault-injection validation. |
| Commit returned uncertain outcome or process died during commit | Reopen and inspect version, full history including immutable operation IDs/artifact digests, catalog and data. Source state means the path did not commit; complete target state means it did. An unreadable or inconsistent result remains uncertain and blocks retries. |
| Target committed, but interruption occurred before reopen validation or UI success | Do not rerun transitions or append duplicate history. Run the full post-reopen gate; commit history alone cannot reopen normal operation. |
| Intermediate version, mixed layout, missing/extra history, or failed invariant | Not an expected recoverable partial state under the whole-path transaction. Preserve files, stop normal operation and diagnose corruption, unsupported tooling, or a defective migration. No automatic metadata correction or restore. |
| Reopen validation fails after a known target commit | Keep the committed target and safety copy intact; block normal operation. A rollback is no longer possible. Separately reviewed repair/restore is needed. |

A retry compares durable immutable identities and content; it is not permission to
repeat external actions. Once normal operation has resumed, automatic restoration
of the old safety copy could discard new user facts and is expressly excluded.

## Validation gate before normal operation

All checks below are approved design obligations, not performed validations.
Run checks on consistent snapshots while ordinary mutations remain blocked.

| Layer | Before commit | After reopen, before ordinary work |
| --- | --- | --- |
| Identity and compatibility | Expected family/database UUID, target catalog, version and complete history path match the release manifest. | Independently read committed metadata/catalog/history; no temporary migration objects or incomplete path. Verify loaded runtime/build support again. |
| Connection policy | Writer has required enforcement, WAL/FULL and approved constraint timing; all required guards are restored. | Verify reader/writer ownership and effective per-connection settings, including FK enforcement, dirty-read prohibition, durability on writer/checkpoint connections and checkpoint threshold. |
| Structure | Full `integrity_check` returns only `ok`; full `foreign_key_check` returns no rows. Errors or interrupted checks fail the gate. | Repeat both checks on committed data. A quick check alone is insufficient. |
| Historical preservation | Compare all source records to explicit mappings, stable IDs, values, relationships and canonical content, not just row counts. Validate every stored digest/seal, coverage proof, supersession/reference and rule compatibility under its own version. | Repeat full canonical/semantic validation. Unknown historical protocols may pass only through the explicitly reviewed opaque-preservation/scoped-uncertainty rule above. No recognized invalid proof is accepted as valid. |
| Reconstruction | Compare source and target results for identical assessment time, rule versions and recovery context, allowing only reviewed representation differences. Verify last-known, unknown, disputed, stale and launch states. | Reconstruct committed data with explicit inputs and cold-restart process context. Reopen alone changes neither source times nor uncertainty; historical process positives do not prove current running state. |

SQLite's [integrity checking](https://www.sqlite.org/pragma.html#pragma_integrity_check)
does not find foreign-key violations; a separate
[foreign-key check](https://www.sqlite.org/pragma.html#pragma_foreign_key_check)
is required. Neither proves Koiné's semantic validity. Expected guard definitions
must match the reviewed catalog; executable rejection/conformance tests belong to
release validation on disposable fixtures, not destructive probes on a live library.

Migration validation is deliberately stricter than routine scoped read recovery:
a known invalid proof or unproved preservation check prevents upgrade completion.
Normal-operation scoped uncertainty for unsupported historical rules remains
available only when its safe isolation is demonstrated. Diagnostics identify the
failed layer and available next action without exposing personal evidence payloads.

## Paper validation matrix and approval boundary

These additional cases do not renumber or change the existing 24
[acceptance specifications](acceptance-cases.md). Later authorized implementation
must exercise the whole release support matrix and its failure points:

| Synthetic case | Required result |
| --- | --- |
| Each declared source schema, including a multi-step path and an already-current schema | Same evidence/identities and reconstruction inputs yield the same conclusions; current schema does not rerun migrations. |
| Empty new destination versus existing version-0/foreign file | Explicit bootstrap only for the unused destination; existing file never silently adopted or overwritten. |
| Too-old/new schema, missing path, forged version or altered migration digest | Clear refusal before transformation; no downgrade, recreation or ordinary writes. |
| Insufficient SQLite feature/build support | Fail compatibility gate independently of the schema integer. |
| Unsupported historical rules/protocols; invalid digest/seal or supersession cycle | Preserve bytes and withhold only provably isolated unsupported conclusions; invalid proof or unproved isolation blocks upgrade. |
| Kill before/after every transition, history insertion, version change and commit boundary, including power-loss simulation | Complete source or complete target after recovery; no intermediate committed schema; no duplicated history or facts. |
| Commit error with active transaction, automatic rollback, and unresolved outcome | Retry the same commit only when appropriate; inspect immutable IDs/content before body retry; unresolved outcome stays blocked. |
| Backup cancellation, incomplete finish, destination validation failure, disk-full or I/O error | No transformation before successful safety copy; no partial copy overwrites a good artifact. |
| Cancellation, contention, second instance or source change between preflight and write lock | Bounded contention, whole-path rollback where established, ownership enforcement and in-transaction source revalidation. |
| Dropped/altered guard, lost index, broken FK, unexpected temporary table or missing historical row | Gate fails even if commit succeeds or row counts match. |
| Target commit followed by failed reopen check or crash before availability | No normal operation or migration replay; preserve target/safety copy and require diagnosis. |
| Partial scan, stale access, distinct accounts, manual removal/restoration, intent-only launch, durable status-127 result and old process positive | Preserve contract distinctions from the existing acceptance cases; no freshness renewal or external handoff replay. |

The design-review gate is satisfied within the approval assessment above. This
does not approve future migration scripts or authorize implementation. The
following decisions remain open before implementing the relevant functionality:

- Exact Koiné SQLite `application_id`.
- Initial database identity/history DDL and artifact-digest rules.
- Released schema compatibility manifests and supported transitions.
- Backup destination and durable-finalization policy.
- Per-transition mechanisms for rebuilding guarded tables, with independently
  reviewed records, transformations, constraints, guards and preservation proofs.
- Packaging verification of the approved SQLite runtime.

General restore tooling remains a separate approval gate. The separately approved
[initial retention and archival policy](evidence-retention-proposal.md) preserves
all committed structured evidence live and uses validated full copies without
pruning or relocation. Archive administration, future pruning/cold relocation,
optional-cache cleanup and permanent-erasure mechanisms remain open. This does
not widen migration permissions or approve a future transition or guard exception.

Documentation checks establish only consistency, links, scope and formatting.
Canonical serialization vectors, physical guard/conformance checks, operational
conformance and the migration fault matrix remain unexecuted. No implementation,
database creation, runtime success, live integration, commit or publication is
claimed by this policy or the approval record.

## Primary references and limits

SQLite documentation inspected for this proposal supports engine behavior only:

- [Transactions](https://www.sqlite.org/lang_transaction.html): explicit transaction
  boundaries, IMMEDIATE acquisition, commit contention and storage-error outcomes.
- [WAL](https://www.sqlite.org/wal.html): recovery files, checkpoint separation,
  local-host constraints and lack of cross-database atomicity for attached files.
- [Online Backup API](https://www.sqlite.org/c3ref/backup_finish.html): copying must
  reach `SQLITE_DONE`; cleanup alone does not prove completion. Independent backup
  validation and durable finalization remain Koiné obligations.
- [Pragmas](https://www.sqlite.org/pragma.html): application/user version metadata,
  engine schema counter, enforcement settings and structural/FK checks.
- [Schema alteration](https://www.sqlite.org/lang_altertable.html): table-rebuild
  dependencies and enforcement caveats; no blanket adoption of its optional bypasses.

These references establish engine behavior; the owner's review establishes
design-level approval. Neither establishes executable migration correctness,
tested distribution compatibility, implementation authorization or publication
clearance.

## Decision history

- **Initial proposal, 2026-10-27:** Prepared as engineering recommendations at the
  owner's request, with migration strategy still unapproved. Documentation checks
  did not establish executable behavior or implementation authorization.
- **Independent owner review, recorded 2026-10-27:** The owner reviewed the uploaded
  proposal against the approved physical schema, operational policy and relevant
  SQLite documentation, found no blocking contradiction, and approved the migration
  policy at design level. Migration metadata was approved in principle. The owner
  retained independent per-transition review, source-content correspondence with
  the validated safety copy, and blocked operation after failed post-commit reopen
  or semantic validation, without automatic restoration. The open decisions above,
  unexecuted validation obligations and 24 unchanged synthetic acceptance
  specifications remain. Only documentation recording was authorized; no local
  Git inspection, executable result, implementation authorization or publication
  clearance is inferred.

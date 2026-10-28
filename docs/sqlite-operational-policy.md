# Approved SQLite operational policy

**Status: Approved design; selected offline operational checks now recorded.**
Recorded 2026-10-27 from the owner's review of the uploaded operational proposal
at reported repository baseline `5f578cc`. The owner reviewed the proposal against
Koiné's approved persistence contracts and current SQLite documentation, found no
blocking contradiction, and explicitly approved the operational design. This does
not establish independent inspection of local Git state or successful execution.

The policy originated as an engineering proposal; owner approval establishes its
design obligations without changing that provenance. It supplements
[architecture A–D](architecture-options.md), the
[logical mapping](persistence-mapping.md), and the
[physical schema](sqlite-schema-proposal.md). Their identity, evidence, canonical
digest, sealing, atomicity, and launch-recovery contracts remain authoritative.
No separate database service is introduced. The later owner-authorized
[offline foundation](executable-foundation.md) implements the initial worker model
and selected checks. The later [Milestone 3 report](controlled-steam.md)
is authoritative for current executable evidence;
the full operational fault/stress matrix remains incomplete.

## Approval decisions

| Area | Approved policy | Remaining validation boundary |
| --- | --- | --- |
| Connection ownership | One dedicated writer and one dedicated reader, each exclusively owned by its worker. | Dedicated worker ownership is implemented; reads during write contention are tested. Full stress validation remains open. |
| Mutations | Bounded writer queue and serialized, complete semantic transactions. | Serialized/idempotent writes, capacity-32 saturation and queued/active cancellation pass; workload/soak validation remains open. |
| Write transaction mode | `BEGIN IMMEDIATE`. | Actual BUSY/FULL, a raw-reader BUSY_SNAPSHOT, rollback and deferred-seal checks pass; simulated I/O/rollback faults exercise handling, not failing hardware. |
| Journaling | WAL on supported local filesystems; no silent fallback. | Effective WAL configuration and local ownership are checked; other filesystems/packages remain unverified. |
| Durability | `synchronous=FULL`. | Durability depends on SQLite's documented filesystem/hardware assumptions; selected process-kill recovery passes, while physical power loss remains untested. |
| Runtime | A verified SQLite build containing the WAL-reset fix. | Packaging and the mechanism for supplying that runtime remain open. |
| Contention | Initial five-second cumulative lock-contention budget per database operation, measured with a monotonic clock. | This is an approved initial policy value, not an empirically validated latency setting. |
| Checkpointing | Passive automatic checkpoints at 1,000 WAL pages, with controlled maintenance. | Reader-pinned growth beyond this threshold and later checkpoint completion pass; this is not a tuning result or WAL size cap. |
| Crash recovery | SQLite-managed recovery followed by structural and semantic validation. | Selected intent/result process-kill boundaries pass; the complete fault matrix remains unverified. |
| Backups | Online Backup API with completed-copy and destination validation before success. | Completion, partial-copy cancellation, queued read service and corrupted-candidate rejection pass; finish/validation failure paths are also simulated. Sustained fairness and actual VFS finish/I/O failures remain unverified. |
| Process ownership | One active Koiné process per database. | Second-owner and symlink-alias rejection pass locally; distribution behavior remains unverified. |

Approval covers design only. The five-second budget and 1,000-page threshold must
not be described as measured performance settings.

## Connection and process ownership

The GTK thread, adapters, and launcher transports submit typed requests and receive
owned results. They do not receive connections, statements, cursors, or open
transactions. Blocking storage work stays outside the UI thread.

Use private connection caches with dirty reads disabled. Each reconstruction uses
one explicit read transaction, including its referenced records and seal checks,
and releases the snapshot before returning results. Reader connections are not
promoted into writers. Keep read transactions bounded and release cancelled
snapshots promptly.

A process-lifetime ownership lock, released by the operating system on exit,
enforces one active Koiné process per database. A second instance fails clearly or
delegates to the existing instance. SQLite locking remains necessary protection
against other clients. The offline foundation uses a process-lifetime File::try_lock on the database file;
its local ownership and symlink-alias tests pass.

## Serialized semantic transactions

Route every mutation through the bounded writer queue. Process accepted mutations
in queue order without treating arrival order as source chronology or authority.
Queue saturation produces backpressure or explicit rejection; accepted evidence
must never disappear silently. The offline foundation selects capacity 32 and standard sync_channel queues;
deterministic saturation/cancellation checks pass in Milestone 2. The full
operational stress matrix remains incomplete.

Acquire write-transaction status before reading database state used to validate a
mutation. Validate database-dependent assumptions again inside that transaction.
Preserve all [physical atomic boundaries](sqlite-schema-proposal.md#transaction-and-reconstruction-obligations):
manual operations include evidence, claims, decisions and seals; collection
registration is separate from complete result application, with closure last;
launch intent commits before external submission and actual handoff evidence
commits afterward; process observations remain independent of handoff disposition.

Filesystem enumeration, launcher execution, and command waiting stay outside
transactions. Large result bundles cannot be split into separately authoritative
batches without separately approved consistency and coverage rules.

## Initialization, durability, and runtime requirements

Establish WAL during controlled storage initialization and verify the actual
returned mode. WAL persists in the database; connection-specific settings must be
established again on opening a connection. Failure to establish required operating
conditions stops normal storage operation rather than silently weakening policy.

Explicitly enable and verify foreign-key enforcement before transactions. Preserve
immediate constraints and the schema's deliberately deferred seal reference;
do not enable blanket foreign-key deferral. Require FULL durability on the writer
and any connection performing checkpoints. Keep normal locking mode, checks, and
guard enforcement enabled.

Active databases are restricted to local filesystems with working locking,
shared-memory, and synchronization behavior. Network shares and live
file-synchronization directories are outside the initial supported configuration.

SQLite 3.37.0 remains the approved schema feature floor. Independently require a
verified runtime containing the WAL-reset fix: SQLite documents the fix in 3.51.3
and later, with backports in 3.44.6 and 3.50.7. Packaging must establish that the
shipped or loaded library contains the fix; the schema floor alone is insufficient.
No bundling choice or distribution compatibility is established by this approval.

A successful FULL commit supports durability under SQLite's filesystem and
hardware assumptions. A checkpoint or clean shutdown is not an additional
prerequisite for acknowledging that commit.

## Contention, cancellation, and transaction uncertainty

The five-second budget is cumulative lock-contention time for one operation, not
a fresh allowance for every statement or retry. Queue waiting is separate and
visible; the budget is not a total operation-latency guarantee. Use one
deadline-aware busy handler per connection and handle direct BUSY returns even
when SQLite does not invoke that handler.

- Before a transaction starts, retry lock acquisition within the remaining budget.
- If COMMIT returns BUSY and the transaction remains active, retry that same commit
  within the remaining budget rather than replaying its body.
- On contention expiry or cancellation, roll back any active transaction and
  report the established outcome. Expiry or cancellation alone is not proof that
  an operation was rolled back or never committed.
- BUSY_SNAPSHOT requires a fresh transaction and revalidation. LOCKED indicates a
  connection/statement coordination problem rather than ordinary external
  contention. Constraint or seal-validation failure rolls back the entire semantic
  operation.
- Inspect transaction state following storage errors rather than assuming an
  automatic rollback. Preserve the distinction between a definitely rolled-back
  operation and an uncertain commit outcome. Inspect immutable operation
  identities and content before any database retry; reopen when necessary to
  resolve uncertainty. No retry may silently duplicate facts or refresh their
  original times.

External launcher submissions must never be replayed as a consequence of storage
retry or recovery. A failure to persist an already submitted handoff does not
prove that submission did not occur.

## Checkpointing and crash recovery

Configure passive automatic checkpoints at 1,000 WAL pages. Additional checkpoint
work runs through the writer worker between semantic operations. Monitor
checkpoint progress, WAL growth, and disk space. The threshold is not a size cap;
long readers may prevent complete checkpointing. Routine checkpoint contention
does not invalidate an already committed transaction: defer maintenance and report
persistent pressure. Reserve truncating checkpoints for controlled idle periods.

Let SQLite recover using the database and its associated recovery files after a
crash. Never manually delete, rename, or substitute sidecar recovery files.
Before normal operation, verify configuration and supported schema/protocol
versions, then perform structural and foreign-key checks. Validate canonical
digests and semantic proofs in the consistent snapshots used for reconstruction;
structural checks alone do not establish valid reconciliation decisions.

An unclosed collection stays incomplete. Intent without durable handoff evidence
means unknown issuance and inconclusive disposition. Historical process evidence
does not establish a currently running process. Structural corruption stops normal
writes; scoped proof failures withhold affected authoritative conclusions and
produce diagnostics. Automatic repair, restore, and schema conversion remain
separate decisions.

## Backup completion and reader availability

Requested backups run off the UI thread, with the reader worker owning the source
and a separate temporary destination connection. Work proceeds in bounded steps,
supports cancellation, and yields so ordinary reads on the single reader worker
cannot be indefinitely starved. Concurrent source modifications can restart
copying; do not promise a snapshot corresponding exactly to the request time.

Success requires copying to reach `SQLITE_DONE`, successful resource cleanup and
closure, and independent reopening and validation of the destination. Require
structural, foreign-key, canonical-digest, and semantic checks. A return of
`SQLITE_OK` from `sqlite3_backup_finish()` alone does not prove completion: it can
also occur when an incomplete backup is abandoned. Durably finalize the validated
artifact before reporting availability. Failed or partial backups must not
overwrite an existing successful backup.

Backup scheduling, destination selection, rotation, encryption, and restoration
procedures are not selected. Copying evidence does not authorize pruning it.

## Retained alternatives and tradeoffs

| Alternative | Reason for the approved choice |
| --- | --- |
| One storage worker for all access | Simpler ownership, but reads queue behind writes; separate reader/writer workers permit concurrent reads. |
| General connection pool or independently writing workers | Adds coordination and contention without increasing SQLite's single-writer capacity. |
| DEFERRED or EXCLUSIVE write transactions | DEFERRED can encounter a later read-to-write upgrade failure; EXCLUSIVE adds no transaction isolation advantage in WAL mode. |
| Rollback DELETE journaling | Simpler file handling, but readers and writers exclude one another during parts of a write; WAL accepts sidecar/checkpoint costs for concurrency. |
| WAL with NORMAL or EXTRA durability | NORMAL can lose acknowledged transactions after power loss or OS crash; FULL accepts synchronization cost for irreplaceable evidence. EXTRA adds no benefit over FULL in WAL mode. |
| Immediate contention failure or unlimited retries | Immediate failure exposes transient locks; unlimited retries can stall indefinitely. A cumulative budget bounds contention handling. |
| Ordinary live copying or quiesced file copying | Live copying risks inconsistency; quiesced copying interrupts service. Online Backup API provides a consistent copy with destination validation. |

These are retained engineering rationales, not benchmarks or executable results.

## Attached conformance obligations and unresolved decisions

The owner highlighted two existing obligations without reopening design approval:
backup copying must reach `SQLITE_DONE` and yield fairly to ordinary reads;
transaction expiry, cancellation and storage errors must preserve commit
uncertainty, inspect immutable identities before database retry, and never replay
launcher submissions. Selected completion, cancellation and no-replay checks now pass in the offline
foundation. Sustained backup fairness and commit-uncertainty fault injection remain
unverified; the report distinguishes code mechanisms from measured coverage.

Full operational validation must still cover connection confinement, queue
saturation/cancellation, rollback and deferred-seal enforcement, contention
deadlines, crash recovery, checkpoint starvation, disk/I/O failures, and
backup/reopen behavior. These supplement the physical design's canonical vectors
and guard/conformance obligations. All 24 existing
[acceptance specifications](acceptance-cases.md) remain unchanged and now have
passing executable translations; the report records selected conformance results.

The separately approved [retention policy](evidence-retention-proposal.md) keeps all
committed structured evidence live and uses validated full archival copies without
pruning or relocation. Archival inventory, pruning/cold relocation, optional-cache
cleanup, permanent erasure, backup scheduling, destination selection,
durable-finalization policy and restoration, packaging and the runtime-supply
mechanism remain open. Routine crash recovery is approved as
design; repair and restore tooling are not. No implementation, database creation,
executable SQL, migrations, Rust code, runtime tests, live integrations or
publication are authorized by this approval or its documentation record.

The separately approved [migration/schema-upgrade policy](sqlite-migration-proposal.md)
establishes a maintenance gate, whole-path transaction, mandatory validated safety
copy, interrupted-upgrade recovery and pre-operation validation at design level.
Metadata was approved in principle; initial identity/history DDL, artifact digest
and application ID are now implemented for unreleased bootstrap. Released
compatibility manifests/transitions, general backup
destination/finalization, guarded-table rebuild mechanisms and runtime packaging
verification remain open. Each actual migration requires independent review.
Source content must still correspond to the validated backup before transformation;
failed post-commit reopen or semantic validation blocks operation without automatic
restore. This approval does not relax enforcement, approve future scripts or
authorize restore tooling, ordinary evidence mutation or implementation.

## Primary references and evidence limits

The owner reported reviewing current SQLite documentation. The references support
the narrow engine behaviors below, not executable correctness of Koiné:

- [Isolation](https://www.sqlite.org/isolation.html): WAL reader/writer concurrency
  and SQLite's single-writer limit.
- [Transactions](https://www.sqlite.org/lang_transaction.html): IMMEDIATE write
  acquisition, WAL EXCLUSIVE equivalence, commit contention, and error handling.
- [Synchronous settings](https://www.sqlite.org/pragma.html#pragma_synchronous):
  FULL/NORMAL/EXTRA durability differences under SQLite's assumptions.
- [WAL](https://www.sqlite.org/wal.html): local shared-memory requirements,
  persistent mode, and WAL-reset fixes in 3.51.3, 3.44.6 and 3.50.7.
- [Foreign keys](https://www.sqlite.org/foreignkeys.html): explicit per-connection
  enforcement.
- [Busy handlers](https://www.sqlite.org/c3ref/busy_handler.html) and
  [transaction state](https://www.sqlite.org/c3ref/get_autocommit.html): direct
  BUSY returns and the need to inspect error outcomes.
- [Automatic checkpoints](https://www.sqlite.org/c3ref/wal_autocheckpoint.html) and
  [checkpoint modes](https://www.sqlite.org/c3ref/wal_checkpoint_v2.html): passive
  behavior and incomplete progress under contention.
- [Recovery-file hazards](https://www.sqlite.org/howtocorrupt.html): sidecars may
  be necessary to recover a database.
- [Online Backup API](https://www.sqlite.org/c3ref/backup_finish.html):
  `SQLITE_DONE`, incomplete-backup cleanup, incremental-copy restarts, and source
  connection reuse between steps. This does not establish reader fairness in Koiné.

## Decision history

- Proposal at reported baseline `5f578cc`: operational alternatives and a
  recommendation were presented without repository changes or runtime validation.
- Owner operational review, recorded 2026-10-27: approved the uploaded proposal at
  design level, including its initial policy values, and highlighted backup
  completion/reader availability and transaction uncertainty as conformance
  obligations. Authorized documentation recording only. Implementation,
  executable-validation and publication status did not advance.
- Subsequent migration review, recorded 2026-10-27: the owner independently reviewed
  the uploaded [migration policy](sqlite-migration-proposal.md) and approved its
  design, with metadata approved in principle, independent per-transition review
  and retained implementation conditions. This does not expand the original
  operational approval or establish executable correctness, implementation
  authorization or publication clearance.

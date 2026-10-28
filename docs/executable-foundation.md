# First executable foundation — engineering report

Recorded 2026-10-27. **Offline implementation and synthetic verification completed;
no live launcher integration or publication.** This preserves the first milestone
implementation and validation checkpoint; the design documents retain their
approval histories. [Milestone 2](storage-resilience-steam.md) and the current
[Milestone 3 report](controlled-steam.md) supersede the historical next-step and
limitation statements below where they record further work.

The owner authorized one engineering milestone: Rust application/test code,
disposable SQLite databases, synthetic execution, routine reversible implementation
choices, documentation and local commits. That instruction supersedes earlier
next-action restrictions to documentation only. It does not authorize real launcher
execution, personal-library access, authentication, destructive pruning/erasure,
restoration, released-schema upgrade transitions or public publication.

## Implemented modules

| Module | Implemented behavior |
| --- | --- |
| [koine-core/domain](../crates/koine-core/src/domain.rs) | Finite normalized records for the five concepts, scoped identities/contexts, provenance, claims, coverage and independent launch/process evidence; typed knowledge, freshness, collection, identity-component and attempt conclusions. No GTK, SQL, filesystem discovery or process execution dependency. |
| [koine-core/validation](../crates/koine-core/src/validation.rs) | Supported rule/artifact checks; claim/authority/context isolation; scoped negative proofs, attributed diagnostic gaps, coverage consistency, manual origin, supersession ordering/acyclicity, decision proof/seal validation and process continuity. |
| [koine-core/reconcile](../crates/koine-core/src/reconcile.rs) | Source-sequence supersession planning, explicit manual causal decisions, unordered disputes, stable reconstruction with explicit policy IDs/time/observer epoch, last-known references, scoped manual visibility and component-wide identity conflict detection. |
| [koine-core/canonical](../crates/koine-core/src/canonical.rs) | Exact typed KOIN v1 framing, strict payload decoding, SHA-256, frozen request/result/decision manifests and role-qualified authority-scope bytes. No hash grants source authority. |
| [koine-sqlite/catalog](../crates/koine-sqlite/src/catalog.rs) | Explicit bootstrap, finite parameterized record I/O, immutable redelivery comparison, digest/seal preparation, complete catalog/identity/history compatibility checks and consistent read snapshots. |
| [koine-sqlite/workers](../crates/koine-sqlite/src/lib.rs) | Dedicated reader/writer threads and bounded queues; serialized IMMEDIATE transactions; verified connection settings; cumulative contention budget; ownership locking; passive checkpoint reports; intent reservation/preflight in the writer transaction, then fake transport outside transactions. No automatic external replay. |
| [Bootstrap v1 DDL](../crates/koine-sqlite/src/bootstrap_v1.sql) | 35 STRICT tables, scoped/composite keys and immediate RESTRICT FKs, immutable update/delete guards, replacement rejection, closed-collection/sealed-decision guards, cycle checks and the deliberately deferred decision-to-seal completion reference. |
| [koine-sqlite/backup](../crates/koine-sqlite/src/backup.rs) | Incremental Online Backup API copies, explicit SQLITE_DONE/finish checks, cancellation, queued reads between steps, independent reopen/structural/FK/canonical/semantic validation and no-overwrite local finalization. No source pruning. |
| [koine-cli](../crates/koine-cli/src/main.rs) | Explicit init/inspect/demo commands. Demo persists a synthetic SPRAWL status-127 failure with inconclusive process evidence. There is no real Steam/Heroic command implementation. |
| [Synthetic harness](../crates/koine-sqlite/tests/common/mod.rs) | Fake discovery reports, explicit source sequence/coverage/authority inputs, user assertions, fake transports and synthetic process identities. Disposable paths only. |

The normalized record API is finite and validated; it is not an arbitrary SQL or
plugin attribute API. Derived conclusions are not stored as authoritative flags.
Registration commits separately from result application, whose closure is last.
New decision proofs are checked before their seals are generated. Result evidence,
coverage, decisions, seals and closure commit together or are rolled back.

## Implementation decisions within the authorization

These are engineering choices under the owner's milestone authorization, not
retroactively owner-authored design requirements:

- Standard `sync_channel` queues have capacity 32. Workers exclusively own their
  connections; snapshots are owned values, not cursors crossing worker boundaries.
- SQLite is system-linked through rusqlite 0.41.0. The initial runtime gate requires
  at least 3.51.3, conservatively excluding older fixed backport branches. Schema
  feature floor remains 3.37.0. WAL/FULL, FK enforcement, dirty-read prohibition,
  recursive guards, query-only reader and 1,000-page checkpoint settings are checked.
- A process-lifetime `File::try_lock` on the database file supplies ownership;
  canonical paths also unify symlink aliases. No administrator privileges are used.
- This unreleased bootstrap uses application ID `0x4b4f494e`, family
  `koine.offline.foundation`, schema 1 and UUIDv4 IDs. This is not a claim of a
  globally registered application ID or a released compatibility promise.
- Immutable identity and bootstrap history are created in the same transaction as
  schema/version metadata. The bootstrap artifact digest is SHA-256 of the exact
  checked-in DDL bytes; history time is explicit UTC microseconds. Bootstrap is
  validated and reopened before workers start. No existing schema is upgraded.
- Synthetic rules are named `koine.synthetic.v1`, version 1; each kind's artifact
  digest hashes the exact UTF-8 bytes `koine.synthetic.v1:<kind>`. A source revision
  `sequence:<unsigned integer>` has ordering meaning only under that explicit
  fixture rule and compatible source context. Collection timestamps never supply it.
- A caller-selected new local backup destination is finalized by syncing the
  validated staging file, creating a no-overwrite hard link in the same directory
  and syncing that directory. The staging name is retained; cleanup/rotation is
  not introduced. This mechanism is an offline implementation choice, not a general
  archive administration or portable filesystem guarantee.
- Backup steps copy at most 32 pages and service a queued ordinary read between
  steps. Queue waiting is separate from the five-second lock-contention budget;
  that budget accumulates monotonic lock waiting rather than ordinary computation.

## Acceptance traceability and results

All 24 original [Markdown specifications](acceptance-cases.md) remain byte-identical.
Their executable translations are in
[acceptance.rs](../crates/koine-sqlite/tests/acceptance.rs). Each row below names the
actual Rust test function. **24 pass, 0 fail, 0 blocked** in the synthetic suite.
This does not verify external launcher formats or transports.

| Specification | Executable test | Outcome |
| --- | --- | --- |
| [multiple-store-entitlements](acceptance-cases.md#multiple-store-entitlements) | `multiple_store_entitlements` | Pass |
| [editions-and-title-collision](acceptance-cases.md#editions-and-title-collision) | `editions_and_title_collision` | Pass |
| [heroic-managed-installation](acceptance-cases.md#heroic-managed-installation) | `heroic_managed_installation` | Pass |
| [heroic-handoff-unavailable](acceptance-cases.md#heroic-handoff-unavailable) | `heroic_handoff_unavailable` | Pass |
| [steam-installation-incomplete](acceptance-cases.md#steam-installation-incomplete) | `steam_installation_incomplete` | Pass |
| [steam-completion-unknown](acceptance-cases.md#steam-completion-unknown) | `steam_completion_unknown` | Pass |
| [incomplete-source-scan](acceptance-cases.md#incomplete-source-scan) | `incomplete_source_scan` | Pass |
| [failed-and-inconclusive-empty-results](acceptance-cases.md#failed-and-inconclusive-empty-results) | `failed_and_inconclusive_empty_results` | Pass |
| [complete-scan-without-negative-authority](acceptance-cases.md#complete-scan-without-negative-authority) | `complete_scan_without_negative_authority` | Pass |
| [supported-bounded-absence](acceptance-cases.md#supported-bounded-absence) | `supported_bounded_absence` | Pass |
| [stale-ownership-evidence](acceptance-cases.md#stale-ownership-evidence) | `stale_ownership_evidence` | Pass |
| [independent-source-and-account-authority](acceptance-cases.md#independent-source-and-account-authority) | `independent_source_and_account_authority` | Pass |
| [duplicate-delayed-and-conflicting-results](acceptance-cases.md#duplicate-delayed-and-conflicting-results) | `duplicate_delayed_and_conflicting_results` | Pass |
| [cancelled-and-changing-scan](acceptance-cases.md#cancelled-and-changing-scan) | `cancelled_and_changing_scan` | Pass |
| [manual-title-without-access](acceptance-cases.md#manual-title-without-access) | `manual_title_without_access` | Pass |
| [multiple-installations-and-missing-mount](acceptance-cases.md#multiple-installations-and-missing-mount) | `multiple_installations_and_missing_mount` | Pass |
| [persistent-library-recovery](acceptance-cases.md#persistent-library-recovery) | `persistent_library_recovery` | Pass |
| [missing-artwork-valid-target](acceptance-cases.md#missing-artwork-valid-target) | `missing_artwork_valid_target` | Pass |
| [sprawl-exit-127](acceptance-cases.md#sprawl-exit-127) | `sprawl_exit_127` | Pass |
| [accepted-without-process-confirmation](acceptance-cases.md#accepted-without-process-confirmation) | `accepted_without_process_confirmation` | Pass |
| [timeout-is-not-definite-failure](acceptance-cases.md#timeout-is-not-definite-failure) | `timeout_is_not_definite_failure` | Pass |
| [observed-process-and-confirmed-exit](acceptance-cases.md#observed-process-and-confirmed-exit) | `observed_process_and_confirmed_exit` | Pass |
| [observer-loss-and-pid-reuse](acceptance-cases.md#observer-loss-and-pid-reuse) | `observer_loss_and_pid_reuse` | Pass |
| [retry-and-unrelated-running-game](acceptance-cases.md#retry-and-unrelated-running-game) | `retry_and_unrelated_running_game` | Pass |

## Canonical, relational and recovery evidence

**58 tests pass overall:** 24 acceptance, 28
[database conformance](../crates/koine-sqlite/tests/conformance.rs), four
[canonical](../crates/koine-core/tests/canonical.rs), and two
[recovery](../crates/koine-sqlite/tests/recovery.rs) tests. No ignored or blocked tests
are counted as passes. The recovery test runs three separate child-kill boundaries.

- Framing vectors cover NULL, signed integers including the minimum, UTF-8 text,
  non-UTF-8 bytes, empty/nested lists and distinct empty-value encodings. Actual
  request/result/decision preimages and SHA-256 digests match independent Python
  `struct`/`hashlib` vectors. Invalid tags, truncation, invalid UTF-8, trailing bytes,
  unknown protocols and duplicate manifest keys are rejected. Record order does
  not change digests. Schema/manifests and independent bundle vectors regenerate
  without changing their checked-in bytes.
- Disposable databases pass full `integrity_check` and `foreign_key_check` during
  bootstrap/open, writes and completed backups. Tests reject orphan references,
  missing negative bases, mismatched scope, malformed UUIDs, forbidden null shapes,
  immutable mutation, post-closure additions, missing seals at commit, sealed-child
  insertion and cyclic supersession. Non-UTF-8 paths round-trip as BLOBs; NUL paths
  fail. These are meaningful checks, not exhaustive proofs of every CHECK branch.
- Failed scans preserve prior evidence. Explicit negative proofs cannot cross
  installation roots or hide attributed gaps behind a complete flag. Cached access
  retains its original source age/validity. Profile upgrades retain original keys
  and observations; connected identity conflicts remain disputed. Manual assertions
  cannot acquire external authority.
- Concurrent duplicate writes remain idempotent on immutable IDs; concurrent
  duplicate handoff requests reserve intent once and call the fake transport once.
  A held external write lock leaves reads available, then permits one commit after
  release. Exhausting the contention budget adds no attempt and leaves the writer
  usable afterward. A reader snapshot can hold back a passive checkpoint; after it
  is released, a later checkpoint completes.
- The fault child is killed without destructors before intent commit, after intent
  commit and after a durable synthetic failure result. Reopening finds respectively
  no attempt, intent-only unknown issuance/inconclusive handoff, and failed handoff
  with status 127. No external replay occurs. A separate interrupted result
  transaction rolls back while the original registered scope remains incomplete
  and prior library evidence survives.
- Validated backup copies preserve the complete logical snapshot and never prune
  the source. Existing destinations are refused. Cancellation produces no reported
  usable destination. Sustained fairness under large concurrent backup workloads
  and arbitrary finish/I/O failures have not been stress/fault-injection tested.

## Commands and observed environment

The following checks completed successfully:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --offline
cargo clippy --workspace --all-targets --offline -- -D warnings
cargo test --workspace --offline
python3 tools/generate_schema.py
python3 tools/generate_vectors.py
git diff --check
```

The generated artifacts were compared before/after regeneration. Cargo.lock is
committed. Verification used stable rustc 1.99.1, Cargo 1.99.1 and system-linked
SQLite 3.54.0 on this Linux environment. Rust was initially absent; the official
toolchain and dependency cache were installed under `/tmp` without changing system
configuration. For this session, `. /tmp/koine-cargo-env` selects them; normal
contributors need their own Rust installation and SQLite development library.
The declared Rust minimum is 1.89; that older toolchain was not separately tested.

The CLI demo and a subsequent independent inspect invocation both reported one
synthetic attempt as `handoff=Failed`, `issuance=issued`, `status=Some(127)`,
`process=Inconclusive`. Both used a new disposable `/tmp` database. No personal
library was read, no Steam/Heroic command was executed and no external launcher
configuration was modified.

## Limits and remaining work

This is a functioning offline foundation, not a completed desktop launcher or a
blanket certification of the approved persistence designs:

- Only explicit synthetic rule artifacts and fake transports are implemented.
  There is no real manual UI, Steam/Heroic parser/transport, authentication,
  positive operating-system process monitor, GTK integration or packaging.
- Supported chronology uses fixture source sequences or explicit manual causal
  actions. General cross-handle supersession, context-equivalence resolution for
  authority unification, source-specific path/residual coverage semantics and
  arbitrary historical rule versions are not implemented. Unsupported proofs are
  refused or yield uncertainty; they are never guessed into supported behavior.
- Canonical snapshots currently load all retained records in memory. When semantic
  isolation cannot be proved, reconstruction withholds all definitive conclusions;
  startup refuses invalid/unsupported evidence. Finer scoped recovery and scale
  optimization remain work. This is more conservative than granting an unsupported
  conclusion and does not delete the underlying evidence.
- Immutable redelivery uses registered operation/local record identities. Future
  real adapters need stable delivery/item reattachment under their reviewed source
  profiles; this suite does not establish real-source duplicate recognition.
- SQL/catalog enforcement and selected crash boundaries have been exercised;
  physical power loss, disk-full/I/O error injection, every constraint null branch,
  long-running starvation, worker panic/commit-uncertainty fault injection and the
  full operational/migration matrix remain unverified. There is no writer-job
  cancellation API beyond safe operation errors; backups support cancellation.
- Local storage with working locks/shared memory/sync is a deployment precondition.
  Automatic detection of network or live-sync directories and distribution-level
  runtime provenance verification are not implemented. A reported engine version
  and successful local tests do not certify other filesystems or packages.
- Initial identity/history DDL and digest mechanics exist for bootstrap only.
  There are no released upgrades, downgrade, restore, repair, erasure, selective
  pruning or cold relocation operations. Backup/archive schedules, destinations,
  encryption, rotation, inventory and administrator workflows remain open.

No acceptance specification was weakened or rewritten. The design approval
histories remain historical; these local executable results neither retroactively
validate an earlier checkpoint nor establish live integration or owner acceptance
of the implementation. No task-time or estimate values are invented.

## Local commits and next milestone

- `9657393` — migration and initial evidence-retention approval documentation.
- `a85d165` — offline Rust evidence core, SQLite persistence, CLI and synthetic tests.
- The documentation commit containing this report records the current results and
  reconciles earlier current-state summaries; its hash is available in local Git
  history. No commit in this milestone is pushed.

Recommended next substantial milestone: harden the storage fault matrix and add a
reviewed read-only Steam discovery adapter using versioned synthetic file fixtures,
then connect it to the durable core. Resolve real source identity, format and
coverage semantics before making real installation/absence claims. Keep actual
launcher execution, GTK work and publication under their respective scopes.

# Milestone 2 — storage resilience and offline Steam discovery

Recorded 2026-10-27. **Implemented, locally executed and committed; no live launcher
integration or publication.** The [Milestone 3 report](controlled-steam.md) supersedes
current-capability and next-step statements below where it records further work;
this dated checkpoint remains unchanged. Starting HEAD was `de3d8ba` on `main`, with an empty
working tree. The owner authorized Milestone 2 application changes,
synthetic fixtures, disposable fault databases, local tests and local commits.
The [first milestone report](executable-foundation.md) remains historical evidence;
its design approvals and 58-test checkpoint are preserved.

## Implemented capabilities

| Module | Result |
| --- | --- |
| [SQLite jobs](../crates/koine-sqlite/src/jobs.rs) and [worker](../crates/koine-sqlite/src/lib.rs) | Nonblocking admission to the existing capacity-32 queue, explicit full/disconnected rejection, cancellable accepted jobs and receipts distinguishing not-started, rolled-back, cancelled, committed and uncertain outcomes. Exact immutable operation inspection distinguishes absent, committed, partial and conflicting content before retry. No automatic transaction-body or launcher replay. |
| SQLite transaction handling | Preserve extended SQLite codes; distinguish BUSY from BUSY_SNAPSHOT, active transactions from implicit rollback, and known rollback from uncertain commit. Quarantine the writer if a transaction remains unresolved after rollback failure. A pure record planner runs inside the same IMMEDIATE transaction as validation/application, preventing stale adapter ID allocation. |
| [Fault laboratory](../crates/koine-sqlite/src/faults.rs) | Opt-in `fault-injection` feature with deterministic gates, simulated errors/cancellation/lost acknowledgements and private backup-candidate corruption. Injection is refused for ordinary and reopened stores; only a newly created fault-lab store accepts it. No destructive production operation or schema migration API. |
| [Backup](../crates/koine-sqlite/src/backup.rs) | Existing incremental copy protocol retained: at most 32 pages per step, explicit DONE/finish checks, queued read service, cancellation before finalization, independent structural/FK/canonical/semantic validation and no-overwrite finalization. New databases and private candidates start with mode 0600. Failed candidates remain private `.partial` files; no cleanup/pruning is introduced. |
| [Steam parser/scanner](../crates/koine-steam/src/lib.rs) and [filesystem boundary](../crates/koine-steam/src/fixture_fs.rs) | Read-only, bounded, versioned interpretation of explicitly marked synthetic containers; no home-directory search. Scoped AppIDs, distinct per-library installations, complete/incomplete/unknown completion, attributable malformed/unreadable/unsupported diagnostics and partial-result preservation. Descriptor-relative no-follow access confines reads to the container. |
| [Steam persistence integration](../crates/koine-steam/src/integration.rs) | Commit requested scope before scanning; atomically apply evidence, coverage, decisions, seals and closure through the existing serialized writer. Identical delivery replay retains local IDs and evidence time; changed content under an existing delivery identity is rejected. Reopen reconstructs identical results under equivalent assessment inputs. |
| [Core rules](../crates/koine-core/src/rules.rs) | Exact versioned identity, interpretation, reconciliation and freshness artifacts. Steam evidence cannot assert entitlements, negative installation facts or handoff capability. Opaque content digests cannot acquire the synthetic sequence profile's chronology authority. Unordered conflicts remain disputed. |
| [Offline CLI](../crates/koine-cli/src/main.rs) | Existing init/demo/inspect retained; `steam-fixture` takes an existing database, explicit absolute fixture, delivery UUID and logical collection time. Prints collection limitations, knowledge, freshness, supporting/last-known claims and evidence provenance. No launcher transport, account access or GTK dependency. |

No physical-schema or canonical-protocol transition was needed. Bootstrap v1,
35 STRICT tables, all immutability/replacement guards, immediate foreign keys,
deferred decision sealing and closure-last atomicity remain in force. WAL/FULL,
foreign keys, query-only reader, recursive guards, ownership and runtime-version
gates continue to be verified. The existing five-second cumulative lock budget
and 1,000-page passive checkpoint threshold are unchanged policy values, not
performance recommendations derived from these tests.

## Steam formats and interpretation evidence

The [format record](steam-discovery-format.md) was prepared from pinned public
implementation evidence before encoding semantics. It links steamlocate revision
`d740b423aee82e8e742595d0cdb4edcc91c24216` (application and library models), Lutris
revision `1f32fe0e46e2a5d92fad9db2846c7cb7becd20c3` (state flags), and Valve's
installation/ownership API documentation. These sources establish inspected
implementation facts and documentation, not live Koiné compatibility. No upstream
code or personal configuration was copied.

Interpretation v1 supports quoted UTF-8 text KeyValues, modern object-valued
numeric `libraryfolders` entries and `AppState` manifests. Legacy scalar library
entries, unsupported syntax, unsafe paths and ambiguous values remain explicit
gaps. Exact StateFlags 4 means manifest-reported complete; selected noninstalled
missing/corrupt/download/update states mean incomplete; unsupported/mixed states
stay unknown. This is manager-reported metadata, not a game-file audit or a
launchability assertion. Optional malformed title data does not discard a usable
installation observation. Numeric AppID spelling is retained with canonical
`steam:app` global keys. Root-relative keys distinguish installation instances.

No inspected source establishes exhaustive absence authority. Empty enumeration,
missing/unmounted roots, corrupt records and cancellations never revoke prior
installation facts. No entitlement, executable, launch target or process claim is
created. `LastOwner` and other ignored account fields are not persisted. Source
effective time stays unknown; collection time, mtime, LastUpdated and content
hashes cannot establish chronology or renew source freshness. Explicit fixture
event times are retained. Different unordered values can remain disputed even
when a later scan appears more plausible.

The scanner caps individual files, parsing depth/tokens, directory enumeration
and aggregate manifest input. Coverage is explicit at requested root/dimension
and index-item bounds; it is not an exhaustive per-file negative proof. Changing
listings/indexes are diagnosed when observed, without claiming an atomic filesystem
snapshot. A dimension containing only unknown values is inconclusive even if
other dimensions have usable facts. The fixture format's UUID is synthetic context identity, not a real
machine/account identity. Linux `/proc/self/fd` is currently required.

## Executed verification

**All-features: 97 passed, 0 failed, 0 ignored, 0 blocked.** The default-feature
suite separately passed **81 tests**, with the 16 feature-gated laboratory tests
excluded from that invocation and executed by the all-features invocation. Do not
add those two overlapping totals together.

| Test target | Pass / fail / blocked | Traceability |
| --- | --- | --- |
| [Original acceptance](../crates/koine-sqlite/tests/acceptance.rs) | 24 / 0 / 0 | All 24 cases, including `sprawl_exit_127`, retain their original executable translations. The [complete 24-case mapping](executable-foundation.md#acceptance-traceability-and-results) remains valid; each test was rerun. |
| [Database conformance](../crates/koine-sqlite/tests/conformance.rs) | 28 / 0 / 0 | Referential/relational guards, sealed decisions, immutable closure, canonical tampering, scoped negative proofs, duplicate writes, backup, ownership and checkpoint regression coverage retained. |
| [Canonical vectors](../crates/koine-core/tests/canonical.rs) | 4 / 0 / 0 | Exact scalar/framing vectors and independent request/result/decision preimages/digests; unchanged protocols. |
| [Recovery](../crates/koine-sqlite/tests/recovery.rs) | 3 / 0 / 0 | Existing recovery retained; new process-kill collection-result boundary test. Five child-kill boundaries overall, plus interrupted connection rollback. |
| [Storage resilience](../crates/koine-sqlite/tests/resilience.rs) | 16 / 0 / 0 | Controlled scenarios detailed below; requires `fault-injection`. |
| [Steam discovery](../crates/koine-steam/tests/discovery.rs) | 20 / 0 / 0 | Parser/identity/completion, multiple roots, missing/inaccessible/unsafe paths, corrupt/unsupported/special/oversized records, changing index, cancellation, partial coverage, aliases/conflicts, delayed/duplicate/concurrent reports, exact rule artifacts, provenance and restart. |
| [CLI integration](../crates/koine-cli/tests/offline.rs) | 2 / 0 / 0 | Independent process inspect matches discovery conclusions, replay is idempotent, fixture bytes remain unchanged, unmarked roots refused, missing DB never created by discovery. |

The Markdown acceptance specifications and their Rust translations compare
byte-for-byte with `de3d8ba`. No case was edited to fit implementation behavior.

### Fault methods and limits

All scenarios use newly created temporary directories/databases. Process-kill
scenarios terminate only their own `koine-crash-probe` child. None establish
physical power-loss behavior or storage-device guarantees.

| Scenario | Executed method and conclusion | Verification boundary |
| --- | --- | --- |
| Queue pressure | Gate an active writer, fill 32 slots, observe explicit Full; cancel an accepted queued job and verify remaining jobs commit once. | Deterministic bounded scheduling, not a long-duration workload benchmark. |
| Cancellation | Before start/before COMMIT rolls back; cancellation after known COMMIT reports committed. | Cooperative safe boundaries; not arbitrary CPU/instruction interruption. |
| BUSY/deadline | Real second SQLite connection holds IMMEDIATE lock; cancellation and five-second contention expiry do not replay or add rows. | Actual SQLITE_BUSY; local filesystem only. |
| Snapshot conflict | Real DEFERRED reader snapshot becomes stale after another commit; promotion returns SQLITE_BUSY_SNAPSHOT (517), remains active until rollback. Production writer remains IMMEDIATE. | Real SQLite result through a raw test connection, not an observed production-reader promotion. |
| Full database | Writer `max_page_count` is clamped on a fresh lab database; a large insert returns actual SQLITE_FULL, rolls back and worker recovers after the test limit is raised. | Genuine SQLite allocation limit, not filling the host disk. |
| I/O/full/snapshot error paths | Inject labeled SQLite codes after the first insert, verify complete rollback and absence by operation identity before a deliberate retry. | Simulated response, not a failing VFS, fsync or device. |
| Rollback failure | Inject failure before rollback after invalid input; report uncertain and quarantine the connection. Reopen validates the empty durable result. | Controlled unresolved transaction; SQLite close performs actual cleanup, not proof of recovery from physical I/O damage. |
| Lost acknowledgement | Commit really succeeds, then inject response loss. Receipt is uncertain; exact identity inspection/reopen finds committed content once and rejects conflicting content. | Simulated acknowledgement loss after real COMMIT, not an actual OS-level COMMIT I/O ambiguity. |
| Interrupted writes | Kill children after uncommitted intent, committed intent, committed synthetic failure, uncommitted coverage insert and committed complete collection result. Prior evidence and registered request survive at correct boundaries. | Abrupt process exit without destructors; uncommitted result probe uses raw SQL, committed result uses normal Store application. No kill during hardware sync. |
| Deferred sealing/guards | Actual deferred seal FK prevents COMMIT, leaves transaction active until rollback; existing conformance rejects bad seals, sealed-child mutation and partial closures. | Selected relational/proof cases, not exhaustive mutation testing. |
| WAL pressure | Hold a real reader snapshot; write more than 1,000 WAL pages and observe incomplete passive checkpoint and >4 MB WAL. Reader release allows checkpoint completion; committed evidence stays readable. | Real reader interference/growth; no sustained disk-space exhaustion or automatic quota policy. |
| Partial backup/cancellation | Pause after first actual 32-page step on a larger DB, queue a read, cancel; trace confirms intervening read service, no final destination, next backup succeeds. | Real partial copy and cancellation, bounded fairness observation. |
| Finish/validation rejection | Inject labeled failure after real finish/before validation; source and last-good copy remain identical and no final candidate is reported usable. | Actual backup_finish error from a failing VFS remains unverified. |
| Corrupted backup | Damage only the private completed candidate header; actual reopen validation rejects it, source is unchanged and next validated copy reconstructs the original snapshot. | Deliberate candidate corruption; not arbitrary damaged-source repair or restoration. |

### Quality and database checks

The following commands completed successfully on Linux with stable Rust/Cargo
1.99.1 and system-linked SQLite 3.54.0, as in milestone 1. Dependencies came from
a local cache; tests ran offline.

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --offline
cargo check --workspace --all-targets --all-features --offline
cargo clippy --workspace --all-targets --offline -- -D warnings
cargo clippy --workspace --all-targets --all-features --offline -- -D warnings
cargo test --workspace --offline
cargo test --workspace --all-features --offline
cargo doc --workspace --no-deps --offline
python3 tools/generate_schema.py
python3 tools/generate_vectors.py
python3 tools/check_docs.py
git diff --check
```

Five generated schema/manifest/vector artifacts were SHA-256 compared before and
after regeneration and remained byte-identical. Full `integrity_check` and
`foreign_key_check` run at bootstrap/open, normal result writes and successful
backup validation. Reopened Steam snapshots and reconstructed views compare
exactly under equivalent assessment inputs, including unknown freshness and
provenance. Existing decision-seal/digest tampering checks still pass. Rustdoc and
local Markdown target/fragment validation pass; the 24 acceptance mappings are
checked against actual test names. External links are evidence references, not a
claim that a link checker executes live integrations.

A separate CLI demonstration used `tools/make_steam_fixture.py` to create two new
libraries, then init → discover → separate-process inspect. It reported 2 roots,
2 observations, 0 diagnostics, 74 immutable records, 2 entries, 1 collection and
0 attempts. Inspection output was identical to the persisted conclusions from
discovery. Only synthetic data in temporary storage outside the repository was
used. Generated runtime data is excluded
from Git; `.partial` candidates have an explicit ignore rule.

## Remaining limits and decisions

No failing executed test or known unresolved defect was found in this validation
scope. These are material limits, not claims that the complete policies are proven:

- No live Steam/Heroic library, account, transport, game process, launcher config,
  authentication, GTK or packaging integration was exercised or introduced.
  Real client builds, package layouts, native file mutations, mounted volumes and
  permission models need separately authorized integration evidence. The
  supported subset is interpretation-versioned, not certified for every Steam
  client version. Legacy scalar indexes remain unsupported.
- No observed VFS write/sync/backup_finish failure, actual host ENOSPC, hardware
  power-loss test, exhaustive crash instruction sweep or soak/starvation workload.
  Injected codes remain labeled simulation. Runtime/library version checks do not
  establish distribution build provenance or filesystem/hardware compliance.
- Checkpoint reports expose pressure, but disk-free telemetry, operator reporting,
  scheduling and sustained-pressure controls remain application work. Thresholds
  are unchanged initial policy. No automatic truncation, deletion or quota is added.
- Raw record submission has cancellation/typed receipts; the pure `mutate` planner
  retains the existing blocking queue backpressure/result API. A full scheduler,
  GUI task lifecycle and cancellation of expensive in-memory validation are not
  implemented. An accepted report is never silently dropped.
- Full retained snapshots and semantic validation are still in-memory. Invalid or
  unsupported evidence is conservatively refused/withheld rather than repaired;
  large-library scaling and finer scoped recovery remain open. Opaque Steam
  revisions provide no supersession chronology. General source ordering and
  historical interpreter compatibility require explicit supported proofs.
- Initial retention preserves all committed structured evidence; validated copies
  authorize no pruning, erasure, cold relocation, restore, released-schema upgrade
  or backup administration. Failed private candidates remain for explicit future
  administration. Archive scheduling, inventory, encryption, rotation and recovery
  workflows remain open exactly as in the approved policies.
- Declared Rust 1.89 minimum was not separately executed. Linux descriptor-relative
  fixture confinement and `/proc` availability were tested here; portability and
  network/live-sync storage detection remain unimplemented.

## Local history and recommended next milestone

Local implementation commits:

- `99baf75` — cancellable writer jobs, uncertainty inspection, fault laboratory,
  backup and process-kill recovery tests.
- `b7485dd` — versioned Steam fixtures, domain rules, durable integration, offline
  CLI, format evidence and synthetic coverage.
- `fead928` — final per-dimension coverage refinements, exact deferred-seal
  assertion, Unicode fixture generation and offline documentation validation.

These follow the verified `de3d8ba` baseline. The source checkpoint at `b7485dd`
had a clean working tree before final refinements/documentation reconciliation.
Documentation reconciliation is the subsequent commit containing this report; its
hash is available in Git history, and the final post-commit Git status was observed
after it. No push, publication or deployment occurred.
Approval histories and first-milestone results remain intact. No unmeasured
performance claims were introduced.

Recommended next substantial milestone: explicitly configured read-only Steam
discovery on owner-authorized real roots, with package/layout compatibility
fixtures, bounded scan scheduling and user-visible provenance/limitations. Obtain
that live-data authorization in the next milestone's scope; retain independent
entitlement, handoff and process-observation contracts. Do not add authentication
or launch execution merely to validate installation discovery.

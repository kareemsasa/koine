# Milestone 3 — controlled Steam integration

Recorded 2026-10-27. Local implementation and synthetic validation; **real-library
verification remains blocked by owner authorization**. No real Steam root was
supplied or authorized during this milestone. No launcher was executed. This is
the current implementation report; the dated [Milestone 1](executable-foundation.md)
and [Milestone 2](storage-resilience-steam.md) checkpoints remain historical.

## Authority and verified baseline

The owner directed configured, read-only Steam discovery, bounded application-local
workers, durable reconciliation, synthetic verification, documentation and local
commits. Real filesystem access requires a separate exact-root approval; publication,
authentication, launch execution, migration of existing user databases and pruning
remain outside scope. Routine implementation choices below are engineering
decisions within the milestone's stated scope, not new architecture approvals.

Before edits, Git was clean at `c761c32`. Re-execution confirmed **97 all-feature
tests passing**, including all 24 acceptance translations.

## Implemented capabilities

| Module | Result |
| --- | --- |
| [Configuration](../crates/koine-steam/src/config.rs) | Explicit absolute root settings, byte-preserving lexical normalization, bounded configuration, exact-path authorization, persistent scope generations and descriptor-derived physical bindings. Setting a root performs no discovery or root probe. |
| [Filesystem](../crates/koine-steam/src/fixture_fs.rs) | Existing descriptor-relative reader extended with Linux `openat2` confinement for v2. No symlinks or descendant mount crossings; no default-directory lookup or recursive search. |
| [Configured adapter](../crates/koine-steam/src/controlled.rs) | One explicit library boundary per collection, native/Flatpak layout labels, bounded modern index interpretation, no automatic root expansion, root-change checks and attributable uncertainty. Reuses the manifest parser. |
| [Scheduler](../crates/koine-steam/src/scheduler.rs) | Explicit refresh, one worker, two additional admitted jobs, overlap rejection, progress tickets, cancellation, bounded admission, graceful shutdown and restart recovery. |
| [Recovery journal](../crates/koine-steam/src/journal.rs) | Private structured-result journal, immutable request UUID bindings, checksum validation, idempotent replay and a scheduler ownership lease. No raw source-file capture. |
| [Integration](../crates/koine-steam/src/integration.rs) | Atomic request registration, original selectors, per-dimension coverage, stable source keys, claims, diagnostics and closure through the existing SQLite writer. Unfinished attempts recover as inconclusive. |
| [Core rules](../crates/koine-core/src/rules.rs) | Separate `koine.steam.controlled.v2`, version `2`, exact semantic artifact digest. Both Steam interpreters are forbidden to assert negatives, entitlement or handoff. Existing rule artifacts remain unchanged. |
| [CLI](../crates/koine-cli/src/steam.rs) | Root configuration, exact authorization, refresh, interactive progress/cancellation, durable status, installation evidence and recovery. No GTK, service or launcher invocation. |

The physical schema, bootstrap artifact, canonical request/result/decision
protocols, writer ownership, WAL/FULL policy, guards, migration gates and retention
policy are unchanged. Settings and recovery files are application-owned operational
metadata; committed domain evidence remains in SQLite. A result transaction ends
with closure, preserving the approved all-or-nothing result boundary.

## Steam formats, layouts and external evidence

The [v1 format record](steam-discovery-format.md) remains the manifest-semantics
reference: quoted UTF-8 KeyValues, modern numeric object-valued library entries,
`AppState`, nonzero decimal u32 AppID, matching manifest filename, safe single
`installdir` component and optional title/flags. No upstream code was copied.

Primary implementation evidence inspected for this milestone:

- [steamlocate app model at d740b423](https://github.com/WilliamVenner/steamlocate-rs/blob/d740b423aee82e8e742595d0cdb4edcc91c24216/src/app.rs)
  establishes manifest field shapes and its named flag-bit decoding. Koiné retains
  its conservative selection: exact 4 means manifest-reported complete; supported
  download/missing/corrupt states without the installed bit mean incomplete;
  ambiguous combinations and unknown bits mean completion unknown.
- [steamlocate library implementation](https://github.com/WilliamVenner/steamlocate-rs/blob/d740b423aee82e8e742595d0cdb4edcc91c24216/src/library.rs)
  is the pinned Milestone 2 evidence for `steamapps`, modern `path` objects and
  manifest enumeration. The current public implementation was also reinspected;
  its index may be stale, so Koiné enumerates manifest names independently.
- [Flathub wrapper at 6e7846ff](https://github.com/flathub/com.valvesoftware.Steam/blob/6e7846ff7743da4ae2ea28fbfc7ba117182610e8/steam_wrapper/steam_wrapper.py)
  identifies its per-app state directory, `.local/share` data directory, a `Steam`
  data child and configurable XDG prefixes. This supports an explicit host-side
  Steam-library layout; it does not establish a universal path or certify a live
  client. Inspected file SHA-256:
  `44cb2f0dbdbb07a5eaa181d2f3ca0303042c3cfa523b373dfa9847239cb20f73`.
- The same pinned [Flathub manifest](https://github.com/flathub/com.valvesoftware.Steam/blob/6e7846ff7743da4ae2ea28fbfc7ba117182610e8/com.valvesoftware.Steam.yml)
  identifies `com.valvesoftware.Steam` and its XDG prefix setting. Neither its
  filesystem permissions nor a Steam index reference authorizes Koiné access.
- [Linux openat2 documentation](https://man7.org/linux/man-pages/man2/openat2.2.html)
  establishes the resolver flags used to reject links, escapes and mount crossings.
  Synthetic tests exercise symlink/traversal refusal. Actual bind-mount alias and
  unsupported-kernel behavior were not exercised by creating mounts or changing
  the running kernel.

| Configuration | Supported subset | Limit |
| --- | --- | --- |
| Native Linux Steam | Explicit actual library directory with immediate `steamapps` child; modern index and manifest subset above. | No default-path detection, distribution/client version detection or live-client certification. |
| Flatpak Steam | Explicit actual host-visible library directory, including a synthetic `.var/app/com.valvesoftware.Steam/.local/share/Steam` layout. Same manifest subset. | Package label is owner supplied. No sandbox-path translation, prefix discovery or wrapper execution. Custom paths require exact configuration. |
| Multiple libraries | Up to 16 explicitly configured boundaries; independent collection/coverage and installation instances. | An index reference is only a diagnostic. Every additional root requires its own configuration and real-root approval. |
| Legacy scalar index, Snap, other package labels, binary/unquoted/directive VDF | Explicit limitations; independently usable supported manifests can survive index errors. | No guessed interpretation or package fallback. |

Neither a manifest nor an `apps` index proves account ownership. Presence is a
manager-reported record, not a game-file audit. Complete means supported
manifest-reported completeness, not launchability. No entitlement, launch target,
process observation, exhaustive negative or revocation is generated. Unknown
source time stays unknown; mtime, collection order and content hashes are not
source chronology. Conflicting unordered claims remain disputed.

## Root authorization, identity and confinement

Configuration and discovery are separate operations. Real-mode refresh refuses
source access until `steam-authorize` records the exact configured path and fixed
read categories. Configuration changes rotate the scope generation and clear
approval/binding. Removal retains all historical SQLite evidence. To adopt a
replaced physical directory at the same spelling, remove and configure that root
again, then obtain fresh real-root authorization.

The adapter opens only the explicit boundary and its immediate `steamapps`
directory. It enumerates directory names and reads `steamapps/libraryfolders.vdf`
and matching `steamapps/appmanifest_*.acf`, plus file/directory metadata. Synthetic
mode additionally requires the marker before Steam-file reading. It does not read
`config`, `userdata`, credentials, account metadata files, game directories,
executables, artwork or arbitrary descendants. Ignored manifest fields, including
`LastOwner`, are not retained.

Root paths must be absolute, at most 4,096 bytes and 128 components. `.` and repeated
separators normalize lexically; parent traversal, NUL and `/` are rejected. There
is no canonicalization that silently follows a root alias. Symlinks in the supplied
path fail explicitly; the owner must separately name/approve the actual target.
The held descriptor anchors access. Descendant opens use `O_RDONLY`, `O_NOFOLLOW`,
`O_NONBLOCK`, regular-file checks and `RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS |
RESOLVE_NO_MAGICLINKS | RESOLVE_NO_XDEV`. Unsupported confinement fails closed.
Directory enumeration uses the held descriptor through `/proc/self/fd`.

Root scope UUIDs are independent of path spelling. First successful access pins
device/inode and available birth-time metadata in private settings before any
manifest is read. Each observation also retains an opaque physical-binding digest
in its immutable claim basis. Changed identity is a gap requiring explicit reconfiguration.
Established duplicate physical bindings are refused across batches; duplicate
physical identities within a batch are also refused. Symlink aliases are refused
before scanning. These are local continuity checks, not universal installation or
volume identities: inode/device reuse without birth-time support, copied databases,
filesystem namespace changes and machine moves require further identity work.

Missing, unreadable, inaccessible, temporarily unavailable, unsupported and
identity-changed paths have separate diagnostic codes. Missing mounts and unknown
layouts are inconclusive. Definite unreadability without usable evidence is failed;
valid observations alongside gaps are partial. Successfully enumerating an empty
root cannot erase previous installations. Original absolute root selectors and
reported location bytes remain in SQLite even after settings change.

Real-root approval must identify the exact path, the file categories above,
possible external index references, and read-only confinement before requesting
owner approval. No such scope was authorized here. The optional path request
received no root, so no default Steam directory, home directory or extra mount was
probed. Real verification is blocked by authorization, independently of the
completed synthetic implementation.

## Scheduling, acceptance and recovery

Initial reversible bounds:

| Resource | Bound |
| --- | --- |
| Discovery workers | 1 application-local standard thread; no persistent daemon. |
| Admitted requests | 3 total: one running/reserved slot plus two waiting slots. Full/overlap rejection occurs before registration or source reads. |
| Configured roots | 16 per database/settings snapshot; one collection per selected root. |
| Source reads | 1 MiB per file, each read twice through one descriptor; 16 MiB logical manifest input per root, plus bounded index reads and one possible over-budget file read. |
| Enumeration/parser | 8,192 entries per `steamapps`; depth 32; 32,768 KeyValues tokens. |
| Operational metadata | 128 KiB settings; 64 MiB per structured journal report; at most 48 report files and 128 immediate journal entries during recovery. Request bindings use direct lookup, not an ever-growing directory scan. |
| CLI progress cache | 64 tickets before completed tickets are evicted; durable outcomes remain in SQLite. |

Admission captures a configuration snapshot and binds the request UUID to the exact
root set, paths, modes, authorization, layouts and supplied collection time. The
private ledger contains a digest only. All requested units register in one writer
transaction before acceptance is returned or Steam source files are read. Multi-root
requests derive attributable per-root collection UUIDs from request/scope identity.
Equivalent pending/running scopes are rejected; one worker also prevents physically
equivalent scans from executing concurrently. A second scheduler cannot recover or
close the first scheduler's live work because of the journal lease.

Progress distinguishes pending/running from terminal complete/partial/failed/
inconclusive. `inspected_records` is provisional scan progress, not an accepted claim count.
Counts shown as `durable_observations` advance after database commit.
A persistence failure is visible, blocks further admission, preserves a journaled
report and requires recovery. Earlier useful root results remain committed; an
aggregate with useful work is partial. Scheduling and delivery order never decide
claim ordering or renew source freshness.

*Update 2026-10-29:* a configuration change after admission (root removed,
reconfigured or its authorization changed) is not a persistence failure. It is
detected when each attempt starts, before its root is opened: only that attempt
closes inconclusively with the diagnostic `configuration-changed-after-admission`
through the journaled report path; other roots in the same request still run and
later requests are admitted. A change made after an attempt has begun reading is
not detected by this check.

*Update 2026-10-30:* a change made after an attempt has begun reading is now
detected as well. The same check runs again before each later source access (the
library index read, each `steamapps` listing and manifest read, the closing index
and directory re-reads and the root reopen) and once more after scanning, with the
settings lock held from that final check through the journal write that accepts
the report. Every removal, reconfiguration or authorization change is therefore
ordered either before acceptance or after it. A change ordered before acceptance
discards everything the attempt read: it closes inconclusively with only its
admitted units and the diagnostic `configuration-changed-during-discovery`, and
none of its observations or inspected coverage is committed. Unlike cancellation,
no partial result survives. This applies to synthetic and real configured roots
alike; other roots in the same request still run and later requests are admitted.
A change ordered after acceptance leaves the accepted report valid: it is applied,
or replayed by recovery, unchanged. Each check is made before the operations it
guards start, and a filesystem operation in progress cannot be interrupted: an
operation whose check preceded the change may still run, or stay blocked on an
unavailable filesystem, and its result is discarded. The physical-binding step reports a root that is no longer configured
with the same diagnostic. Fixture-v1 discovery has no configured roots and is
unchanged.

Cancellation is cooperative between bounded operations, roots and manifests.
Cancellation before scanning closes all original units inconclusively. After a
valid observation, that observation survives in a partial result. Shutdown cancels
active/queued requests, drains their outcomes and joins the worker. A kernel call
blocked on an unavailable filesystem cannot be forcibly cancelled by this thread;
there is no hard wall-clock shutdown bound or filesystem latency guarantee.

After scanning, a complete structured report is synced to a private journal file
and its directory before database application. This is the durable acceptance
boundary for observations. Result replay checks original request identities and
closed-report fingerprints through the existing writer planner. A known commit
followed by a lost acknowledgment or interrupted journal cleanup produces no new
claims or observation times. Only the duplicate recovery file is removed after
confirmed SQLite application; authoritative evidence is never pruned.

On restart, recovery reads application-owned files only: replay durable reports
first, then close remaining registered attempts inconclusively with their original
units. It never re-scans roots or automatically resumes collection. A crash before
journal synchronization can lose unacknowledged in-memory observations; the pending
attempt retains uncertainty rather than inventing an empty successful library.
Journal or settings corruption blocks work and preserves files for diagnosis.

## CLI workflow

All examples use disposable synthetic data. Do not substitute a personal root
without the exact-root authorization gate.

```sh
scratch=$(mktemp -d)
python3 tools/make_steam_fixture.py "$scratch/fixture"
cargo run -p koine-cli -- init "$scratch/library.sqlite"
cargo run -p koine-cli -- steam-root-set "$scratch/library.sqlite" 11111111-1111-4111-8111-111111111111 native synthetic "$scratch/fixture"
cargo run -p koine-cli -- steam-roots "$scratch/library.sqlite"
cargo run -p koine-cli -- steam-refresh "$scratch/library.sqlite" 22222222-2222-4222-8222-222222222222 1000000
cargo run -p koine-cli -- steam-status "$scratch/library.sqlite"
cargo run -p koine-cli -- steam-evidence "$scratch/library.sqlite"
cargo run -p koine-cli -- steam-session "$scratch/library.sqlite"
```

In a session: `refresh REQUEST_UUID TIME [ROOT_UUID ...]`, `status REQUEST_UUID`,
`cancel REQUEST_UUID`, `wait REQUEST_UUID`, `recover`, `quit`. EOF cancels and joins.
One process owns a database; use the session for cancellation while work runs,
then `steam-status` for durable history. `steam-recover DATABASE` performs startup
replay/closure without source inspection. Reusing a completed request does not
read a changed manifest. A fresh refresh requires a fresh UUID.

For a separately approved real boundary: configure `real` instead of `synthetic`,
then explicitly run `steam-authorize DATABASE ROOT_UUID EXACT_PATH
--read-manifests-and-index-only`. That is an operational owner acknowledgment,
not automatic approval based on an index reference. No authorization command was
run against a real root in this milestone.

Root listing intentionally shows configured paths. Routine refresh/status/evidence
output uses opaque IDs, outcomes, diagnostic codes, interpretation versions,
observation times, presence and completeness; it omits library inventories, titles
and raw locations. Generic inspection now withholds raw evidence locator bytes.
Private SQLite/settings/journal files still contain structured personal evidence
when used on approved real data; they are not anonymized exports.

## Validation results

**138 all-feature tests pass; 117 default-feature tests pass.** No tests fail or
are ignored. These include the 97-test baseline plus 41 new all-feature tests.
All 24 acceptance specifications and their executable translations are
byte-identical to `c761c32`.

| Suite | All-feature tests passing |
| --- | ---: |
| CLI end-to-end workflows | 4 |
| Independent canonical vectors | 4 |
| Unchanged acceptance cases | 24 |
| Existing database conformance | 28 |
| Existing SQLite process recovery | 3 |
| Existing storage resilience/fault laboratory | 16 |
| Steam configuration/journal/scheduler unit tests | 15 |
| Configured Steam integration/regression tests | 22 |
| New discovery process-kill recovery | 2 |
| Existing fixture-v1 discovery regressions | 20 |
| **Total** | **138** |

The new checks cover explicit native/Flatpak synthetic layouts; multiple roots;
private persistent configuration and exact authorization denial; lexical path and
symlink boundaries; unapproved/internal references; missing/unreadable/replaced
roots; malformed/legacy/unsupported formats; incomplete and unknown completion;
original locators; byte/entry bounds; duplicate requests/results; unordered claim
conflicts and reversed snapshot delivery; queue saturation, overlapping and
concurrent requests; cancellation before/after observations; shutdown and panic
recovery; scheduler ownership; corrupted settings/journal refusal; and reconstruction
after successful, partial, failed and interrupted collections.

An actual SQLite `SQLITE_FULL` (13), induced through its page limit on disposable
fault-lab databases, rolls back discovery persistence. The scheduler blocks further
refresh, retains its journal, and recovers the exact observations after capacity is
restored, including after reopen/source removal. A separately labeled lost-commit-
acknowledgment simulation checks identity-based replay after an actual successful
commit. This is not a genuine VFS write/sync failure or host disk exhaustion.

Actual SIGKILL tests terminate disposable child processes after durable request
registration and after report journal synchronization. Reopen preserves original
scope; registration-only interruption becomes inconclusive; the journaled case
recovers observations without source files. Neither test simulates hardware power
loss. Existing WAL/FULL, SQLite guards, integrity/FK, canonical and backup fault
checks also remain passing.

The following exact commands completed successfully in the same Linux
validation environment as the earlier milestones, with stable Rust/Cargo 1.99.1
and system-linked SQLite 3.54.0:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --offline
cargo check --workspace --all-targets --all-features --offline
cargo clippy --workspace --all-targets --offline -- -D warnings
cargo clippy --workspace --all-targets --all-features --offline -- -D warnings
cargo test --workspace --offline
cargo test --workspace --all-features --offline
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps --offline
python3 tools/generate_schema.py
python3 tools/generate_vectors.py
python3 tools/check_docs.py
git diff --check
```

Five schema/manifest/vector artifacts were SHA-256 compared before and after
regeneration and stayed byte-identical. The documentation checker validates local
links/fragments and all 24 acceptance mappings. The declared Rust 1.89 minimum was
not separately executed. Full SQLite integrity and foreign-key checks remain part
of normal bootstrap/open/write/backup validation.

A separate-process disposable CLI demonstration configured two synthetic roots,
refreshed, then inspected durable status/evidence. It produced two collections,
two source entries, two installation instances and two evidence records; complete
and incomplete observations remained distinct. The parent index's additional
library reference remained an explicit gap even though that library was separately
configured/scanned. Outcomes were partial/complete; the batch was partial with two
durable observations. Source bytes remained identical. Independent SQL checks
returned `integrity_check = ok`, zero foreign-key violations, WAL mode, zero
entitlements and zero negative claims. Logs and runtime data stayed in temporary
storage outside the repository.

During development, tests exposed mismatched synthetic authority provenance and a
failed aggregate lacking failed coverage. Both were corrected in the new v2 path;
the final suites pass. An initial validation shell lacked Cargo on PATH, and one
artifact-check script initially referenced a nonexistent directory; validation
then succeeded with Cargo available and the corrected artifact list.
No failed check is being presented as passing. No synthetic result is represented
as a real Steam observation.

*Update 2026-10-28:* after review fixes `f66af51` and `46460c5` in PR #2, 140
all-feature and 119 default-feature tests pass, with none failed or ignored (Steam
configuration/journal/scheduler unit tests 16; configured Steam
integration/regression tests 23). CI on `main` at merge commit `8054216` passed.
The counts and table above remain the Milestone 3 checkpoint.

*Update 2026-10-28:* the authorized real-mode tests added in `46460c5` exercise the
real-mode code path with synthetic data in temporary test databases. Their
`koine.steam.configured` and `steam:local` records are test artifacts, not
observations of a real library. No real Steam library has been read.

## Storage and compatibility limits

- SQLite retains all committed structured evidence and still loads retained
  snapshots for semantic validation/reconstruction. Scan bounds are not a bound on
  database size, total history, validation cost, WAL growth or backup storage.
- Settings are a private checksummed canonical file beside the database; the private
  journal and request-binding ledger are separate operational files under the
  same database ownership regime. Atomic rename/link, file sync and directory sync
  depend on supported local filesystem behavior. They are not a new SQLite schema
  or a distributed transaction. Failure before acknowledgment remains explicit.
- The Online Backup API still copies SQLite only. Configuration, unresolved journal
  results and request bindings need coordinated application-state backup work.
  Do not relocate/copy a live database and assume a complete application backup.
  Losing settings disables configured access; losing the request ledger removes
  multi-root group-request binding history, while SQLite collection/result
  identities remain protected. No restore/repair or released migration is added.
- Tests establish local Linux behavior and selected process-kill/actual SQLite
  failures. They do not establish genuine VFS sync/I/O failure, host ENOSPC,
  hardware power-loss durability, adversarial same-user file tampering, bind-mount
  behavior, kernel fallback support, sustained throughput or starvation freedom.
- No real package/client build, permission model, mounted library, Steam process,
  entitlement channel, handoff or process identity mechanism was verified. No GTK,
  Heroic integration, authentication, publication or system-wide installation was
  introduced. Unsupported semantics remain limitations rather than guessed facts.

Recommended next substantial milestone: controlled external-launcher handoff and
process-evidence lifecycle, beginning with independently researched transport
semantics and synthetic failure/recovery coverage. Obtain separate authorization
before actual launcher execution. Exact-root Steam verification can proceed as a
separate bounded follow-up whenever the owner supplies and approves a scope;
application-state backup coordination and measured large-history scaling remain
substantial storage work.

## Local commits and repository state

- `3199aab` — configured Steam discovery, v2 interpretation and confinement,
  bounded scheduling, operational recovery files, CLI, synthetic integration,
  crash/fault tests and fixture tooling. All validation above passed before this
  local implementation commit.
- `35f2511` — consolidated engineering report, compatibility/evidence boundaries,
  exact validation results and synchronization of current README/contracts links.

At the post-documentation checkpoint `35f2511`, `git status --short` produced no
output: the working tree was clean. The subsequent documentation-only audit
commit records these observed hashes and status. Its own hash is available
in this file's Git history; final Git status is checked again after that commit.
No commit was pushed.

This is local implementation, local validation and local commit. No publication,
deployment, real-library validation or owner acceptance is inferred. Historical
milestone results and approval records remain preserved. Exact-root real-library
verification remains blocked pending separate exact-root approval; other
verification recorded above as unperformed also remains open.

*Update 2026-10-28:* the Milestone 3 commits `3199aab`, `35f2511` and `0f91bc6`
were later pushed and, with review fixes `f66af51` and `46460c5`, merged through
PR #2 into `8054216` on 2026-10-28, under the owner-authorized publication of
2026-10-25. Exact-root real-library verification remains blocked.

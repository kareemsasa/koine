# Steam fixture discovery interpretation v1

Engineering interpretation prepared for Milestone 2 on 2026-10-27, before encoding
format semantics. This is an offline, read-only subset, not Valve certification or
live-client verification. No upstream implementation code is copied.

## Inspected primary implementation evidence

- [steamlocate app model, revision d740b423](https://github.com/WilliamVenner/steamlocate-rs/blob/d740b423aee82e8e742595d0cdb4edcc91c24216/src/app.rs):
  an `AppState` text object, decimal u32 app ID, installation-directory string,
  optional title, flags and update metadata. Its flag decoder identifies the
  installed, missing/corrupt-files and update/download states used as evidence below.
- [steamlocate library parser, same revision](https://github.com/WilliamVenner/steamlocate-rs/blob/d740b423aee82e8e742595d0cdb4edcc91c24216/src/library.rs):
  modern `libraryfolders` objects have numeric children containing `path` and
  optional `apps` objects. A manifest is addressed within a library's `steamapps`.
- [Lutris state enumeration, revision 1f32fe0e](https://github.com/lutris/lutris/blob/1f32fe0e46e2a5d92fad9db2846c7cb7becd20c3/docs/steam.rst):
  independently corroborates the named state bits. These are public implementation
  findings, not a stable Valve promise for all current/future client versions.
- [Valve ISteamApps](https://partner.steamgames.com/doc/api/ISteamApps#BIsAppInstalled):
  installation and ownership are separate API concerns. This adapter calls no API,
  performs no authentication and treats no local manifest as entitlement proof.

## Selected subset and limits

The following rules are Koiné engineering choices based on that evidence:

- Text KeyValues uses quoted UTF-8 strings, braces, whitespace, `//` comments and
  the supported quote/backslash/newline/tab escapes. Keys compare without ASCII
  case; duplicate keys are rejected, including case collisions. Includes,
  directives, binary VDF, unquoted tokens, unsupported escapes, excessive depth or
  size and ambiguous structure produce diagnostics instead of guessed values.
- Support modern object-valued numeric library entries with a string `path`.
  Legacy scalar-valued library entries are explicitly unsupported. `apps` indexes
  are never used to infer presence, absence, ownership or completeness.
- An application manifest is `steamapps/appmanifest_<decimal-u32>.acf` with one
  `AppState` object. The filename and `appid` must agree; zero/overflow/nondecimal
  IDs fail. The reported spelling is preserved while `steam:app` keys use canonical
  decimal. Title is optional; a missing, empty or non-text title produces unknown title
  without discarding a usable installation record. `installdir` is required and
  must be one safe directory component, at most 255 UTF-8 bytes.
- Presence means a valid manager-reported installation record in that bounded
  library, not a game-file audit. Location is the reported `steamapps/common`
  locator. Neither means the executable exists, a handoff is supported or the title
  is currently launchable. Zero, uninstalled-bit or uninstalling-bit flags produce
  unknown presence, never an absence assertion.
- Exact StateFlags 4 means manifest-reported complete. Known missing/corrupt or
  active update/download/staging states without the installed bit mean incomplete.
  Mixed installed/update states, unknown bits, absent/invalid flags and unsupported
  combinations mean completion unknown. A diagnostic marks that interpretation gap.
  No process observation is derived from the app-running flag.
- `LastUpdated`, file mtime and collection time do not establish source chronology.
  Source effective time stays unknown; an exact content digest is an opaque
  revision token, not an ordering number. Conflicting observations remain disputed
  unless another explicitly supported ordering proof exists. Re-reading a cache
  cannot renew an underlying source claim's freshness.
- No exhaustive negative capability is established by these sources. Even a
  successful local directory enumeration produces no absence or entitlement claim.
  Empty, failed, cancelled, missing-root and partial results preserve prior facts.

## Fixture confinement and collection boundary

Only explicitly supplied fixture containers bearing `.koine-synthetic-root` with
two lines `koine-fixture-v1` and a lowercase UUIDv4 are accepted. The UUID identifies
the synthetic manager/machine context; root-relative paths distinguish libraries.
The generator creates a fresh UUID. Copying a fixture with its UUID deliberately
retains that source identity. There is no home-directory discovery.
Modern index paths are followed only inside that container; symlinks, parent
traversal and special files are refused. Rejected paths remain scope gaps and are
never probed outside the container. The parser itself performs no filesystem I/O.

The preflight captures a bounded index plan, including the supplied default root,
all declared roots and index gaps. The original requested units are committed
before manifest scanning. Re-reading the index/directory listing detects observed
changes; it does not promise an atomic source snapshot. Result application uses
the existing writer queue and one transaction with closure last. Each root and
claim dimension has its own coverage/diagnostics. Source evidence retains its
collection time, exact interpretation rule, opaque content revision and locator.
No complete raw source file or account metadata is stored. Retained typed claim
values, locators and diagnostics are still structured source evidence; they are
not anonymized data. Ignored fields such as `LastOwner` are not persisted.
Fixture event time is supplied explicitly in UTC microseconds; collection and
termination use that logical fixture time, never invented source-effective time.

Delivery UUIDs identify operations. Reapplying an identical report reuses stored
local IDs and compares immutable content; a changed payload under the same delivery
identity is rejected. New collections preserve original facts and provenance.

## Exact interpretation and resource bounds

The recognized flag bit offsets are 0–12 and 16–23 in the pinned decoder. The
incomplete mask uses offsets 5, 7–10 and 18–23, requires the installed bit (2) to
be clear, rejects uninstalled/uninstalling (0/11), and rejects any unknown bit.
Exact 4 is the only complete combination. Reconfiguring/validating alone and
other unsupported combinations stay unknown. This conservative selection is a
Koiné rule, not an assertion that Steam guarantees these combinations exhaustively.

The parser caps input at 1 MiB, nesting at 32 and tokens at 32,768. Directory
enumeration caps at 8,192 entries and the scan stops after a 16 MiB manifest-input
budget. Files are read twice through the same descriptor and directory/index
revisions are rechecked; concurrent changes can still escape observation. Source
freshness consequently remains unknown. The Linux fixture filesystem uses
`openat`, `O_NOFOLLOW`, regular-file checks and `/proc/self/fd` enumeration. It
refuses symlinks throughout the supplied path, outside-container index paths,
special files and unreadable records. Permission-bit refusal is also exercised
under privileged test environments. No Windows, Flatpak, Snap or live desktop
layout is certified by these fixtures.

Coverage is bounded to each requested root and dimension, plus the index item.
A root with usable facts and gaps is partial; gaps without usable facts are
inconclusive. A successful enumeration with no facts can be complete only for
that requested enumeration, with no negative authority. Diagnostics target the
registered unit and retain attributable fixture-relative locator bytes in hex.
No per-file exhaustive-negative proof is implemented. Unknown/rejected roots
remain registered scope gaps rather than disappearing from the request.

The domain recognizes exact rule names/version/digests in
[rules.rs](../crates/koine-core/src/rules.rs). The artifact digests hash the frozen
UTF-8 semantic descriptors in that module; they identify interpretation policy,
not a hash of the Rust executable or a claim of source authenticity. Steam AppIDs
use the `steam:app` global catalog profile. No ordering rule is registered for this
source. Numeric sequence ordering remains exclusive to the older explicit
synthetic profile; SHA-256 revisions and timestamps cannot borrow that authority.
The report fingerprint frames the delivery, time, scope, index and interpreted
observations/diagnostics under `koine.steam.report.v1`. It is an internal
idempotence check, not an independently supported import protocol. Reports are
trusted in-process adapter values, with consistency checks before persistence.

Executable results and remaining runtime limits are recorded separately in the
milestone report. Raw fixture inputs were handwritten for this repository; public
sources establish interpretation evidence only.

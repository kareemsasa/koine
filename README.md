# Koiné

Koiné is a planned open-source Linux game library and launcher.

**All your worlds. One library.**

## Purpose

Bring games from multiple storefronts and launchers into one unified,
searchable collection without replacing the applications that manage them.

## Architectural principles

1. External launchers remain authoritative for game configuration and execution.
2. Discovery is read-only by default.
3. Game identity, source entries, ownership, installation, and launch authority
   are distinct concepts.
4. Library observations retain provenance and freshness.
5. Unknown state must not be represented as a negative fact.
6. A successful launch handoff is not proof that a game process started.
7. Missing artwork or optional metadata must never block game launching.
8. No workstation-specific assumptions belong in the application architecture.

## Initial scope

- Manual and Steam library discovery.
- Persistent library records.
- Installed and owned-but-uninstalled distinctions where supported by evidence.
- Steam launch delegation.
- Observable launch handoff and process state.
- Heroic discovery and delegation as the next integration target.

## Explicit non-goals

- Replacing Steam, Heroic, or other storefront clients.
- Managing Wine or Proton prefixes.
- Downloading or managing compatibility runners.
- Modifying external launcher configuration.
- Implementing independent storefront authentication.
- Reimplementing game installation or update mechanisms.

## Project status

Milestone 2 adds storage fault hardening and read-only Steam discovery confined to
explicitly supplied synthetic fixtures. The [engineering report](docs/storage-resilience-steam.md)
records 97 passing tests with all features enabled, including all 24 unchanged
acceptance cases, actual SQLite error/recovery checks and durable fixture discovery.
The [first milestone report](docs/executable-foundation.md) preserves the original
58-test checkpoint. Live Steam/Heroic integration and GTK remain unverified.

Approved architecture remains stable Rust (edition 2024, Cargo), a GTK-independent
application core with built-in adapters and bounded standard workers, serialized
mutations, SQLite, and GTK4/libadwaita for the eventual interface. The
[engineering foundation](docs/README.md) and linked approval records retain their
design authority. Initial retention keeps structured evidence live; validated
archival copies authorize neither pruning nor relocation.

The owner's executable-foundation authorization supersedes the earlier
documentation-only next action. Implementation choices and local results are
recorded separately from design approvals and live integration evidence. Released
schema upgrades, restore/repair, destructive retention, authentication, packaging
and real launcher transports remain outside this implementation.

## Build and exercise offline

Use stable Rust (declared minimum 1.89; tested with 1.99.1) and a local SQLite
development library/runtime at least 3.51.3. Cargo.lock is committed. After fetching
dependencies, the checks can run with `--offline`.

```sh
cargo check --workspace --all-targets
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
scratch=$(mktemp -d)
cargo run -p koine-cli -- demo "$scratch/demo.sqlite"
cargo run -p koine-cli -- inspect "$scratch/demo.sqlite"
```

The demo uses fake discovery and transport evidence only. It persists a definite
status-127 failure and inconclusive process state. `init` and `demo` require an
unused destination; `inspect` refuses missing or unsupported databases. The
[report](docs/storage-resilience-steam.md#remaining-limits-and-decisions) explains supported
synthetic semantics, conservative recovery and remaining failure/integration limits.

CI runs these checks, plus default-feature tests and the documentation check, as
[`tools/ci_checks.sh`](tools/ci_checks.sh) on every pull request and on `main`. The
[workflow](.github/workflows/ci.yml) uses an Arch Linux container pinned by image
digest and a dated package snapshot, which supply a SQLite runtime of at least
3.51.3. Report security issues privately as described in [SECURITY.md](SECURITY.md).

Exercise Steam discovery only on a newly generated synthetic fixture:

```sh
python3 tools/make_steam_fixture.py "$scratch/steam-fixture"
cargo run -p koine-cli -- init "$scratch/steam.sqlite"
cargo run -p koine-cli -- steam-fixture "$scratch/steam.sqlite" "$scratch/steam-fixture" 11111111-1111-4111-8111-111111111111 1000000
cargo run -p koine-cli -- inspect "$scratch/steam.sqlite" 1000000
```

The fixture must be an absolute, explicitly marked container; outside paths and
symlinks are refused. The [format record](docs/steam-discovery-format.md) defines
the supported subset and limits. No local manifest grants ownership or absence
authority. The fault laboratory requires `--all-features` and accepts only newly
created disposable lab databases.

The repository is intended for eventual public release. Publication requires
a separate review of licensing, repository contents, and disclosure boundaries.

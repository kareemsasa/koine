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

Early architecture and requirements definition.

The owner has approved stable Rust (edition 2024, Cargo), an application-local core
with built-in adapters, bounded workers and serialized state mutations,
GTK4/libadwaita through Rust bindings, and SQLite with relational separation of
records and evidence. Standard workers are approved without Tokio initially.

The [architecture decision record](docs/architecture-options.md) defines A–D's
approval boundaries. The [physical SQLite design](docs/sqlite-schema-proposal.md)
is separately approved as the implementation design baseline. The
[SQLite operational policy](docs/sqlite-operational-policy.md) also has design-level
approval, including dedicated reader/writer ownership, serialized writes, WAL/FULL,
a verified WAL-reset-fixed runtime, and initial contention/checkpoint values.
Executable correctness remains unverified. Migration/schema upgrades, evidence
retention, backup scheduling/destinations/restoration, packaging/runtime supply,
and other listed decisions remain open. The current action is documentation only;
implementation, database creation, and live launcher execution require separate
authorization.

The [engineering foundation](docs/README.md) defines the domain, discovery and
reconciliation contracts, launch state semantics, and synthetic acceptance cases.
These contracts do not select an implementation stack or claim working integrations.

The repository is intended for eventual public release. Publication requires
a separate review of licensing, repository contents, and disclosure boundaries.

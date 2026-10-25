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

No application framework, programming language, database schema, or deployment
mechanism has been selected.

The repository is intended for eventual public release. Publication requires
a separate review of licensing, repository contents, and disclosure boundaries.

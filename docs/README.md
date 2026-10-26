# Engineering foundation

Koiné's [project README](../README.md) defines product identity, scope, and
architectural boundaries. These documents elaborate those boundaries into logical
contracts; they do not supersede them. External launchers remain authoritative for
game configuration and execution.

## Reading order

1. [Domain model](domain-model.md): identity, relationships, evidence, and uncertainty.
2. [Discovery and reconciliation](discovery-contract.md): adapter inputs, outcomes,
   scope, and safe reconciliation.
3. [Launch delegation](launch-contract.md): handoff and independent process evidence.
4. [Synthetic acceptance cases](acceptance-cases.md): Given/When/Then specifications.
5. [External evidence and integration boundaries](integration-evidence.md): public
   references, their limits, and work still requiring integration verification.

## Authority and status

The following distinctions apply throughout these documents:

| Kind | Authority and meaning |
| --- | --- |
| Product requirement / established decision | Owner-directed scope, reflected in the project README and the foundation mandate: five distinct concepts, launcher authority, evidence preservation, truthful launch reporting, optional metadata, and the initial integration order. |
| Interface contract | Engineering elaboration of those decisions. Required behavior for an implementation; terminology and logical fields are not a programming API or physical schema. |
| Implementation detail | Mechanism chosen within approved architecture. None of the examples chooses an application stack. |
| External evidence | Referenced launcher documentation or inspected public evidence; establishes only the specifically attributed behavior and limits. |
| Test specification | Synthetic input and required output; does not establish that software exists or passed a runtime test. |
| Open question | Unselected architecture or unverified external behavior. Must not be treated as an accepted decision. |

`Must` denotes a contract obligation. Logical identifiers and state names make
cases precise; their encoding, types, storage layout, and UI wording remain
implementation details. Contract elaborations include scoped keys, per-claim
evidence, bounded completeness, and independent handoff/process state axes. They
preserve the approved distinctions without prescribing a database or event system.

The repository remains documentation only. A–D have architectural approval, but
this review authorizes only recording the decisions. No adapter is implemented,
no real Steam/Heroic verification is recorded, and the 24 synthetic acceptance
cases remain specifications rather than executed tests.

## Approved architecture and open decisions

The [architecture decision record](architecture-options.md) is authoritative for
the owner's A–D approval and its boundaries: stable Rust (edition 2024, Cargo),
standard workers without Tokio initially, an application-local core with built-in
adapters, bounded workers and serialized state mutations, GTK4/libadwaita through
Rust bindings, and SQLite with relational separation of records and evidence.
It retains the alternatives and engineering rationale. The owner reviewed the
comparison in the message and did not claim independent inspection of that file.

Physical schema, migration strategy, connection model, retention policy,
packaging/distribution, ownership authentication, specific Steam/Heroic transports,
synchronization scheduling, process identity mechanisms, and detailed UI design
remain open. No expiry interval or automatic matching algorithm is selected.

The first vertical slice remains manual and Steam discovery, persistent records,
distinct ownership and installation, Steam handoff, and observable states. Heroic
discovery and delegation follow. Local installation discovery cannot promise a
complete owned-game library.

The language/runtime, application structure, toolkit, and engine decisions are
resolved within those boundaries. Implementation remains outside this
documentation-only next action. The core must be independently testable without
GTK, SQLite, or live
launchers; architectural approval does not establish reconciliation correctness.
Blocking discovery, storage operations, and handoff work must stay outside GTK's
UI thread, with results returned safely for presentation. Remaining consequential
decisions still need concrete alternatives, tradeoffs, a recommendation, and an
explanation of affected work when they become necessary.

## Maintaining the contracts

Keep examples aligned with the contracts and retain the difference between a
requirement, external observation, and test result. Changes to product authority
or scope require an owner decision. Keep fixtures synthetic; never commit account
tokens, runtime logs, personal libraries, or workstation configuration. Ordinary
library functionality must not require administrator privileges. Discovery must
not modify external launchers, including Steam's `localconfig.vdf` or Heroic game
configuration.

For documentation changes, review every acceptance case against the contracts,
check relative links, run `git diff --check`, and inspect the diff for disclosure
and scope. Do not introduce a programming stack just to execute these cases.
Later implementations should turn applicable cases into executable tests in the
approved environment, then separately verify external integration behavior.

Local edits, validation, commits, public push, and real integration verification
are separate events. This work is authorized for local commits. Public push still
requires explicit publication authorization and the README's separate review.

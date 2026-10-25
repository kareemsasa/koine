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

The foundation is documentation only. There is no approved executable application
environment, implemented adapter, or real Steam/Heroic verification in this pass.
The cases deliberately use semantic inputs rather than purported launcher files.

## Decisions still open

Application language, UI framework, desktop toolkit, database technology, physical
persistence schema, packaging/distribution, authentication strategy for ownership
APIs, background synchronization architecture, and process-monitoring implementation
remain undecided. No expiry interval, automatic matching algorithm, launcher path,
or universal launcher transport has been selected here either.

The [language/runtime and application architecture comparison](architecture-options.md)
contains concrete alternatives and an engineering recommendation for owner approval.
Its status is proposed; it does not change the decisions listed above.

The first vertical slice remains manual and Steam discovery, persistent records,
distinct ownership and installation, Steam handoff, and observable states. Heroic
discovery and delegation follow. Local installation discovery cannot promise a
complete owned-game library.

Documentation and acceptance specifications can proceed without selecting a stack.
Executable product implementation needs a language/runtime decision; UI and durable
storage work will also need toolkit and persistence decisions. When that next work
is authorized, present concrete alternatives, tradeoffs, a recommendation, and the
work blocked by each necessary decision. No decision request is needed to finish
this foundation, and completing it does not authorize choosing the stack.

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

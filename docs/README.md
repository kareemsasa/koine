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
6. [Approved logical-to-relational mapping](persistence-mapping.md): scoped identity,
   supersession validity, launch recovery, and deterministic reconstruction.
7. [Approved physical SQLite design baseline](sqlite-schema-proposal.md): catalog,
   canonical protocols and guard/seal requirements.
8. [Approved SQLite operational policy](sqlite-operational-policy.md): connection
   ownership, serialized writes, durability, contention, checkpoints, crash recovery,
   backup obligations, and decisions still open.
9. [Approved migration and schema-upgrade policy](sqlite-migration-proposal.md):
   version compatibility, atomic upgrades, interrupted recovery, unsupported
   databases, and validation before normal operation; design approval only,
   with initial bootstrap metadata now implemented and released upgrades
   unimplemented.
10. [Approved evidence retention and archival policy](evidence-retention-proposal.md):
    mandatory preservation versus optional retention, deletion eligibility,
    referenced evidence, archival alternatives, manual removal and recovery
    consequences; initial retention and copy-only archival approved at design
    level, with nonblocking limitations and further decisions retained.
11. [First executable foundation report](executable-foundation.md): implemented modules,
    all 24 executable acceptance mappings, canonical/database/recovery evidence,
    implementation decisions, commands, local commits and remaining limitations.
12. [Steam fixture interpretation v1](steam-discovery-format.md): pinned public
    format evidence, supported subset, conservative semantics and read confinement.
13. [Milestone 2 report](storage-resilience-steam.md): storage faults, versioned
    Steam fixtures, durable CLI integration, 97 executed tests and remaining limits.

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
cases precise; their meaning is independent of physical encoding and UI wording.
Accepted physical choices are recorded below. Contract elaborations include scoped keys, per-claim
evidence, bounded completeness, and independent handoff/process state axes. They
preserve the approved distinctions without prescribing a database or event system.

The repository now contains the offline Rust foundation and the owner-authorized
storage resilience/Steam fixture milestone. The [current report](storage-resilience-steam.md)
is authoritative for 97 passing all-features tests, including all 24 unchanged
acceptance translations. The [first report](executable-foundation.md) preserves
its 58-test checkpoint. Real Steam/Heroic integration and publication remain
unverified and outside the completed milestone's authorization.
A–D, logical mapping, physical design, operational, migration and initial retention
approvals retain their recorded boundaries; test execution does not expand them.

## Approved architecture and open decisions

The [architecture decision record](architecture-options.md) is authoritative for
the owner's A–D approval and its boundaries: stable Rust (edition 2024, Cargo),
standard workers without Tokio initially, an application-local core with built-in
adapters, bounded workers and serialized state mutations, GTK4/libadwaita through
Rust bindings, and SQLite with relational separation of records and evidence.
It retains the alternatives and engineering rationale. The owner reviewed the
comparison in the message and did not claim independent inspection of that file.

The owner separately approved the [clarified logical mapping](persistence-mapping.md)
from the discussion. All four review conditions are satisfied as behavioral
obligations. Heroic's reporter-local identity profile remains provisional; manager
authority is distinct from storefront identity authority. The
[physical SQLite design baseline](sqlite-schema-proposal.md) now has independent
owner approval of its catalog, canonical digest protocols and specified guard/seal
behavior. The owner inspected the uploaded revision, not local Git state, and did
not execute SQL. This closes the design-review gate, not executable validation.

The owner subsequently accepted lowercase UUID TEXT, UTC integer microseconds,
path BLOBs, SHA-256 as content-digest algorithm, and STRICT/3.37.0 as schema baseline
subject to distribution verification. Deferred sealing, canonical protocols and
specified guard behavior are approved as design. The
[physical review disposition](sqlite-schema-proposal.md#physical-review-disposition)
defines the limits and attached requirements: reproducible canonical-serialization
vectors and guard/conformance checks for incomplete decisions, invalid references,
post-closure mutations and inconsistent coverage. Selected offline vectors and
conformance checks now pass as recorded in the implementation report; exhaustive
validation remains incomplete.

The [operational approval record](sqlite-operational-policy.md) is authoritative
for dedicated reader/writer ownership, a bounded writer queue, IMMEDIATE writes,
WAL/FULL on supported local storage, a verified WAL-reset-fixed runtime, one active
process per database, crash recovery and validated Online Backup API copies.
The five-second cumulative lock-contention budget and 1,000-page passive checkpoint
threshold are approved initial policy values, not empirically validated settings.
Backup completion must reach `SQLITE_DONE` and yield fairly to ordinary reads;
transaction failures must preserve commit uncertainty and inspect immutable
identities before database retry, without replaying launcher submissions.
Selected offline operational checks, including queue/cancellation, real SQLite
errors, backup faults and WAL pressure now pass; the current report identifies
untested fault and stress cases. The owner reviewed the uploaded
proposal against persistence contracts and SQLite documentation; approval does not
establish local Git inspection or executable behavior.

Repair/restore tooling, archival administration, pruning/cold relocation,
backup scheduling/destination selection, packaging/runtime supply, ownership
authentication, specific Steam/Heroic transports,
synchronization scheduling, process identity mechanisms, and detailed UI design
remain open. No expiry interval or automatic matching algorithm is selected.

The [retention approval record](evidence-retention-proposal.md#approval-assessment)
establishes the initial design policy: keep all committed structured evidence in
the live database and use independently validated full-database archival copies
without removing live records or redirecting references. Manual removal remains
reversible visibility, not erasure. Storage growth, duplicated copies and the
absence of bounded-size or complete-erasure guarantees are nonblocking design
limitations. Inventory/destination/finalization, rotation/restore administration,
optional-cache cleanup, future pruning/cold relocation and permanent-erasure
semantics remain open. This does not change the physical catalog or establish
executable correctness, implementation authorization or publication clearance.

The [migration approval record](sqlite-migration-proposal.md) establishes design
approval for exact compatibility checks, a whole-path transaction, mandatory
validated safety copy, interrupted recovery, evidence preservation and validation
before ordinary operations resume. The owner independently reviewed the uploaded
proposal; no local Git inspection or executable result follows. Metadata is approved
in principle. Each actual migration still needs independent review, including
guarded-table mechanisms and preservation proofs. Source content must still match
the validated safety copy before transformation; failed post-commit reopen/semantic
validation keeps operation blocked without automatic restore.

The implementation report records the initial application ID, bootstrap
identity/history DDL and artifact digest, and local copy finalization mechanism.
Released compatibility manifests/transitions, general backup administration,
per-transition guarded-table rebuild mechanisms and distribution runtime
verification remain open. Routine crash recovery remains distinct from migration
recovery and separately unselected repair/restore tooling. Released-upgrade
conformance remains unexecuted; only new-database bootstrap and compatibility
refusal are implemented. Publication is not authorized.

The first vertical slice remains manual and Steam discovery, persistent records,
distinct ownership and installation, Steam handoff, and observable states. Heroic
discovery and delegation follow. Local installation discovery cannot promise a
complete owned-game library.

The language/runtime, application structure, toolkit, engine, logical mapping,
physical-design, operational-policy, migration-policy and initial retention/archival
decisions are resolved within those boundaries. The separately authorized offline
implementation is recorded in the executable report; real launcher execution and
publication remain outside its scope.
The core is independently testable without GTK, SQLite or live launchers. The
executable report records its selected checks; architectural approval alone does
not establish reconciliation correctness.
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
The executable report traces the existing cases to synthetic tests. Maintain that
mapping as implementation evolves and verify external integration separately.

Local edits, validation, commits, public push, and real integration verification
are separate events. This work is authorized for local commits. Public push still
requires explicit publication authorization and the README's separate review.

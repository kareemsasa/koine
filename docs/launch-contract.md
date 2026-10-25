# Launch delegation contract

This logical contract follows the [domain model](domain-model.md). Steam executes
Steam-managed games. Heroic executes Heroic-managed games with its own runner,
prefix, and launch options. Koiné requests a handoff and reports what it can observe.
It does not reconstruct either launcher's execution configuration.

## Target and attempt

A target identifies its source entry, authoritative launcher and instance/context,
source-specific target key, related installation if known, and the evidence for
the supported handoff capability. Resolve the actual launcher transport within the
environment; a bare executable name or imported template is not a universal
availability guarantee. No particular command, URI, Flatpak invocation, or binary
path is selected by this contract.

An attempt has a unique local reference, selected target, request time, handoff
events/results, and independent process observations. Preserve evidence and reason
codes for each result. A retry is a new attempt. Results belonging to one target,
account, or attempt must not overwrite another merely because the game identity
is shared. A launcher client or forwarding helper PID is not a game PID.

Preflight may establish that the target or transport is unsupported/unavailable,
in which case report that no handoff was issued. Stale/unknown entitlement or
installation evidence is not itself a definite handoff-command failure. Optional
metadata never participates in target validity. Koiné must not repair a target by
launching Wine/Proton or a launcher-managed game executable directly.

## Two independent state axes

**L1 — Handoff is not process evidence.** Record handoff progress separately from
process progress, rather than using one mutable game status.

| Handoff state | Evidence required |
| --- | --- |
| `not-requested` | No transport request issued, including a preflight refusal. |
| `requested` | Koiné issued the handoff request; disposition still pending. |
| `accepted` | The selected transport reported acceptance under its documented semantics. This can be a successful forwarding-command result; it does not promise that the launcher or game started. Record which boundary acknowledged it. |
| `failed` | A definite transport/command failure, including inability to execute the handoff command or a command result defined as failure. Record the failure and available exit status. |
| `inconclusive` | The request's disposition cannot be established, such as an ambiguous timeout, interruption, or loss of result. |

Ordinary handoff transitions are `not-requested → requested → accepted | failed |
inconclusive`. A preflight refusal stays `not-requested` with its reason; an issue
failure during submission is `requested → failed`. A later definitive result can
resolve an inconclusive disposition with new evidence. Preserve earlier events;
do not silently rewrite their meaning. Duplicate delivery must not create another
attempt or repeat the external command. Duplicate-request suppression mechanics
remain implementation details.

Command creation alone establishes only a request. Exit zero can establish
acceptance only when the transport contract gives it that meaning; it never proves
game startup. A timeout does not prove command failure or game failure. Exit of
the forwarding command, even with a nonzero status, is not an observed game exit.
Classify nonzero/exception behavior according to the selected transport, with
definite command failure reported as failed. No automatic retry follows an
ambiguous disposition, because the original request may already have acted.

| Process observation state | Evidence required |
| --- | --- |
| `unobserved` | No game process has been established for the attempt; observation may not yet have been attempted. |
| `observed` | Positive identity evidence associates a live game process with this target/attempt, with observation time and method. |
| `inconclusive` | Observation is unavailable, unsupported, ambiguous, timed out, or lost continuity. Retain any earlier confirmed observation with its timestamp. |
| `exited` | A previously identified game process is observed to have exited with continuity of identity. This says only what happened to that observed process. |

Process transitions start at `unobserved`. A supported positive observation moves
to `observed`; insufficient evidence moves to `inconclusive`. From `observed`,
confirmed exit moves to `exited`, while lost visibility moves to `inconclusive`.
From `inconclusive`, positive evidence can establish `observed`, or evidence of
exit can establish `exited` only for a previously identified process with justified
continuity. An exited process observation stays historical; a new process needs
new identity evidence, not revival of its old PID record. Observations may arrive
while handoff disposition is pending and must retain their own timing.

**L2 — Honest absence.** Not finding a matching process during a bounded check is
inconclusive about startup. It is not proof of launch failure, exit, or session
completion. A stopped observer, app restart, reused PID, or vanished name match
must not manufacture an exit event.

**L3 — Evidence before running.** Present a current running claim only with valid
positive process evidence, limited by its observation time and continuity. Name
similarity, a command returning zero, or a persistent launcher process is
insufficient. If visibility is lost, present the last observation and current
uncertainty, rather than an indefinitely asserted running game.

**L4 — Failure creates no session.** Definite handoff failure must not create an
observed process or unsupported running session. If there is independent evidence
of a game already running, retain that evidence separately; do not attribute its
startup to the failed command or erase an earlier valid observation.

**L5 — Process exit is bounded.** Observing one game's identified process exit
does not establish a complete play session's end, all descendant exits, or playtime.
The first implementation need not calculate sessions or monitor complex process
trees. An attempt's bookkeeping can finish with observation inconclusive without
claiming that the game finished.

## Minimum observation boundary

The first implementation must expose these distinctions and support inconclusive
observation. No complex process-tree monitoring is required by this foundation.
Before claiming positive process observation, its implementation must explain how
it identifies the game and maintains identity across checks; PID alone is
insufficient because it can be reused. Method, capabilities, and limits must be
testable. When no reliable association method exists, report inconclusive and
retain the handoff result. Do not introduce an unapproved monitoring architecture
to avoid admitting uncertainty.

This contract does not select an observation timeout or freshness window. A
bounded observer can stop with an explicit reason, without leaving the request
apparently pending forever or inventing process exit. On restart, prior process
evidence is historical until continuity is re-established.

## Required reporting combinations

| Evidence | Required interpretation | Forbidden inference |
| --- | --- | --- |
| Definite handoff-command failure; no independent game observation | Handoff failed; no observed game process. | Running game, completed game session, or a fabricated game exit. |
| Transport acceptance; no confirmed game process | Handoff accepted; process unobserved/inconclusive as appropriate. | Game successfully launched, game failed, or game exited. |
| Acceptance and positively identified live game process | Handoff accepted; game process observed as of the observation time. | Proven session ownership or complete process-tree identity. |
| Previously observed process; visibility lost | Last observed running at its recorded time; current observation inconclusive. | Definite exit or indefinitely current running state. |
| Previously observed process; identified process exit confirmed | Observed process exited. | Whole session finalized or forwarding command was the game. |

The [acceptance cases](acceptance-cases.md) include the required
`sprawl-exit-127` regression. Its historical command template and status are test
inputs, not a selected Koiné transport or a diagnosis of the original failure.

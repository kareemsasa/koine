# Synthetic acceptance cases

These are technology-neutral specifications under the [foundation](README.md),
not executed application tests or real Steam/Heroic observations. Every title,
key, account, root, and event below is synthetic except the explicitly identified
historical identifiers used in `sprawl-exit-127`. No command in this document is
an instruction to execute a launcher.

## Conventions and oracle

`t0 < t1 < t2` denotes a justified observation order within a fixture, not a
cross-source clock-synchronization policy. `account-A`, `manager-H`, `root-A`, and
process identity tokens are opaque fixture references, not actual paths or PIDs.
`E-steam` and `E-epic` are distinct source entries for the synthetic game Meridian.
All observations carry the [domain evidence](domain-model.md) information.

After each case, inspect source entries and relationships, entitlement and
installation claims with scope/freshness, scan coverage/diagnostics, and attempt
handoff/process evidence. Each Then clause is required. No implicit mutation,
negative fact, external configuration write, or invented session is permitted.
Transport acceptance/failure and reliable process identity are explicit fixture
inputs, not assumptions about real command behavior.

## Discovery and domain cases

### multiple-store-entitlements

**Given:** `E-steam` has positive Steam access evidence for `account-A` at t0;
`E-epic` has independent Epic access evidence at t1. Edition evidence justifies
linking both entries to game `G-meridian`. Only `E-steam` has an observed complete
installation and Steam target.

**When:** Reconcile the observations and group the entries.

**Then:** One organizational game links two source entries, two access claims,
one installation, and one target. The Epic entry does not gain a Steam
installation or target. Selecting the Steam target delegates only to Steam.
Ungrouping the entries preserves every source-specific record. (D1–D5, R1)

### editions-and-title-collision

**Given:** Two entries have the same title but no equivalence evidence; another
pair explicitly represents a base edition and a remaster.

**When:** Discover them or receive an unsupported suggestion to merge them.

**Then:** Equal display names do not merge records. Edition distinctions and the
unresolved/disputed match evidence remain available. No entitlement or installation
moves between entries. No automatic matching service is required. (D1–D3, D8)

### heroic-managed-installation

**Given:** `E-epic` is present and complete in `manager-H`, a Heroic context. Access
is unknown. A fixture capability explicitly identifies a valid Heroic handoff for
that entry. Heroic owns its runner, prefix, and options; Koiné has not imported them.

**When:** Select that target and request a handoff.

**Then:** The request goes to Heroic with the entry/context identity. Koiné neither
constructs a Legendary/Wine/Proton command nor copies or edits Heroic configuration.
Access remains unknown, and process state remains unobserved until independently
established. This fixture does not verify a real Heroic interface. (D4–D5, R8, L1)

### heroic-handoff-unavailable

**Given:** The same observed Heroic installation, but no supported Heroic handoff
capability is established.

**When:** Evaluate the target for launch.

**Then:** Retain the installation evidence and report the unavailable/unverified
handoff capability. If preflight refuses, no handoff is requested. Do not bypass
Heroic using its backend metadata. (R1, R8, launch preflight)

### steam-installation-incomplete

**Given:** A synthetic semantic Steam observation reports presence at `root-A`
and explicitly incomplete installation. No account access evidence is supplied.

**When:** Reconcile a complete installation scan for that bounded root.

**Then:** Record presence and incomplete completion state, not fully installed or
absent. Access stays unknown. Do not infer executable readiness or game startup.
An adapter's later file fixtures must establish how actual flags support this
semantic observation. (D4–D6, R1)

### steam-completion-unknown

**Given:** A supported record establishes installation presence, but an unsupported
completion marker prevents interpretation of completeness.

**When:** Reconcile that independently usable presence observation.

**Then:** Presence is retained with completeness unknown and a diagnostic. Coverage
for interpreting completion is partial/inconclusive as appropriate, not complete.
Do not infer fully installed or uninstalled. (D6, discovery coverage)

### incomplete-source-scan

**Given:** Earlier complete installations `I-A` and `I-B` in `root-A` and `root-B`.
A new scan requests both roots; `root-A` yields a valid observation, while
`root-B` is unreadable and yields none.

**When:** Reconcile the partial result.

**Then:** Apply the valid observation from `root-A`. Preserve `I-B` as last-known
with its original observation time and the coverage gap. Do not remove it or claim
it is currently absent. The aggregate scan is partial, even if `root-A` is complete.
(D6–D7, R2, R7)

### failed-and-inconclusive-empty-results

**Given:** Prior access and installation evidence. Three separate fixture scans
return no observations: a definite read failure; a locator whose absence may mean
an unmounted volume; and an unsupported capability.

**When:** Reconcile each result independently.

**Then:** Record failed, inconclusive, and inconclusive respectively, with reasons.
None means empty library, no access, or uninstalled. Keep prior claims and their
times. The requested unsupported capability does not become complete. (D6–D7, R2)

### complete-scan-without-negative-authority

**Given:** An adapter successfully reads every local Steam manifest in its requested
scope. An earlier access claim refers to an entry with no manifest there. The
adapter has no account-ownership enumeration capability.

**When:** Reconcile the complete local installation scan.

**Then:** Completeness applies only to declared local discovery. It cannot revoke
access, prove an empty account library, or classify every missing title as not
owned. No authentication flow is introduced. (D4, D6, R1, R3)

### supported-bounded-absence

**Given:** A hypothetical fixture source explicitly guarantees exhaustive
installation enumeration within `root-A`, which is available and consistently
readable. Its complete t1 result omits a previously present t0 installation there.
An independent installation in `root-B` and an access claim also exist.

**When:** Reconcile using that explicit negative-evidence capability.

**Then:** Record justified absence only for the installation in `root-A` at t1,
retaining the older observation and supersession basis. Preserve `root-B`, access,
the source entry, and its organizational link. If the same scan is partial with
no proven complete sub-scope, omission cannot support that absence. This is not
a claim that a particular real launcher already supplies this guarantee. (R2–R4)

### stale-ownership-evidence

**Given:** An access claim at t0 has a known validity limit before t1, with no new
access evidence. A refresh at t1 fails; at t2 the adapter rereads the same old cache.

**When:** Assess freshness and reconcile the failed refresh and cache observation.

**Then:** Keep the access claim as last-known/stale with its validity basis and
failure diagnostic. Do not turn it into revoked access or renew its source time
to t1/t2. Cache collection time is separate. No global expiry interval is implied.
(D7, R2, R5–R6)

### independent-source-and-account-authority

**Given:** A positive access claim for Steam `account-A`, an explicit supported
negative for Steam `account-B`, and a Heroic observation for `E-epic`.

**When:** Reconcile all three.

**Then:** The account claims are scoped independently and are not contradictory.
The Heroic result changes no Steam claims. Identical bare IDs in different source
namespaces remain different entries. (D1, D8, R1)

### duplicate-delayed-and-conflicting-results

**Given:** A justified t1 claim and a delayed t0 claim for the same scope; a second
delivery of t1; then incompatible claims whose effective order cannot be resolved.

**When:** Reconcile in that delivery order.

**Then:** t0 does not overwrite t1 because it arrived later. The duplicate creates
no logical duplicate or refreshed timestamp. The unordered conflict retains both
bases and is disputed, without arbitrary winner selection. (D2, D8, R5–R6)

### cancelled-and-changing-scan

**Given:** A scan is cancelled after collecting one valid positive observation;
another scan sees source changes that prevent a coherent absence conclusion.

**When:** Reconcile each.

**Then:** The cancelled scan is partial with cancellation noted and can contribute
its valid positive. It cannot erase unseen records. The changing scan reports
partial/inconclusive coverage for affected claims; it cannot claim authoritative
absence from inconsistent enumeration. (R2–R3, R7)

### manual-title-without-access

**Given:** A user adds a manual title with no explicit access, installation, or
target claim; later the user explicitly asserts access, then removes the manual
record. A separate Steam entry has the same display title.

**When:** Apply each manual action.

**Then:** The title alone leaves access unknown. The later access assertion has
user provenance and does not become storefront proof. Manual removal neither
revokes Steam access nor uninstalls anything. (D1, D4, R1, R4, R8)

### multiple-installations-and-missing-mount

**Given:** One entry has distinct manager-scoped installations in `root-A` and
`root-B`. `root-B` becomes unavailable, with no evidence distinguishing unmount
from removal.

**When:** Refresh discovery.

**Then:** Keep the instances distinct; `root-B` is last-known with an inconclusive
current observation. Do not combine instances by title, claim uninstall, or
substitute `root-A`'s target without an explicit target selection. (D1–D6, R2)

### persistent-library-recovery

**Given:** A persisted game group with two source entries, independent access
claims, distinct installations and targets, and their provenance. One claim is
stale; another dimension is unknown; a justified bounded negative retains its
supersession basis.

**When:** Close and reopen Koiné without new discovery evidence.

**Then:** Recover the same stable identities, relationships, claims, evidence,
and uncertainty. Reopening does not renew freshness, change unknown to false,
merge the installations, or assert a currently running game from historical
process evidence. No specific storage engine is required. (D1–D8, L2–L3)

## Launch and optional metadata cases

### missing-artwork-valid-target

**Given:** A valid target with an available supported handoff capability. Artwork
is absent; an optional screenshot lookup fails.

**When:** Request the selected handoff.

**Then:** Metadata failure neither prevents the request nor alters target/access/
installation evidence. Handoff and process outcomes depend on their own evidence.
No placeholder-artwork implementation is required by this case. (D9, R1, L1)

### sprawl-exit-127

**Given:** A synthetic entry named SPRAWL with Steam App ID `1549690`. The historical
imported template is `steam -applaunch {app_id}`. There is no prior positively
identified game process or supported running session for this attempt. The fixture
transport models a definite handoff-command failure with exit status `127`.

**When:** Request handoff; receive the definite failure; a bounded observation
finds no positively identified game process.

**Then:** Handoff transitions `not-requested → requested → failed`, preserving
status `127`. No game process is observed. If the bounded observation result is
recorded, process state is inconclusive, not exited. No running game or unsupported
running session is created or retained, and no completed session or playtime is
invented. Reporting cannot continue to show a pending handoff after the definite
result. Do not infer the cause of status `127` or real game execution history.
Do not execute this template as a test. (L1–L5)

### accepted-without-process-confirmation

**Given:** A fixture transport whose successful forwarding result explicitly means
acceptance. No game process is established for the attempt.

**When:** Request handoff; receive acceptance; finish a bounded observation without
reliable game identity evidence.

**Then:** Handoff is accepted; process observation is inconclusive. Report both.
Do not claim game startup, game failure, game exit, or session completion. Ending
the observer does not leave handoff pending or create a running session. (L1–L3, L5)

### timeout-is-not-definite-failure

**Given:** A handoff request whose disposition becomes unavailable at a timeout;
there is no definitive acceptance/failure or game process observation.

**When:** End the bounded wait.

**Then:** Handoff is inconclusive with the timeout reason. Do not claim definite
handoff failure or game failure. Do not automatically reissue the command. Later
definitive evidence may resolve disposition without inventing process evidence.
(L1–L2, handoff transitions)

### observed-process-and-confirmed-exit

**Given:** Accepted handoff and subsequent fixture evidence identifying a live game
process with identity continuity beyond PID alone.

**When:** Record that positive observation, then an exit confirmed for the same
process identity.

**Then:** Process state moves `unobserved → observed → exited`. Record observation
times and identity basis. The helper's exit cannot substitute for this evidence.
Do not infer the whole process tree ended or finalize playtime. (L1, L3, L5)

### observer-loss-and-pid-reuse

**Given:** A positively observed game process; visibility is subsequently lost or
Koiné restarts. Later the same numeric PID belongs to an unrelated process.

**When:** Assess current process state without continuity evidence.

**Then:** Preserve the historical positive observation and mark current observation
inconclusive. Neither PID reuse nor lost visibility means the game is still
running or has definitely exited. Do not attach the unrelated process. (L2–L3)

### retry-and-unrelated-running-game

**Given:** Attempt A has reliable independent evidence of a running game. A new
attempt B for that title encounters definite handoff-command failure. A delayed
result for A and a duplicate result for B then arrive.

**When:** Record all results.

**Then:** B remains failed with no process invented or attributed to B. Preserve
A's valid evidence within its observation limits. Delayed/duplicate results cannot
overwrite B, duplicate attempts, reissue the command, or erase A's evidence. A
further explicit retry would receive a new attempt identity. (L1, L3–L4)

## Review coverage

The cases exercise D1–D9, R1–R8, and L1–L5. Required examples map to
`multiple-store-entitlements`, `heroic-managed-installation`,
`steam-installation-incomplete`, `incomplete-source-scan`,
`stale-ownership-evidence`, `missing-artwork-valid-target`, `sprawl-exit-127`, and
`accepted-without-process-confirmation`. Additional cases challenge identity,
negative-evidence scope, result ordering, and process continuity.

Document review can establish consistency of these specifications. It cannot
establish executable behavior, Steam manifest semantics, Heroic configuration
preservation in practice, or process-monitoring accuracy. Those need later tests
in an approved environment and explicitly identified integration verification.

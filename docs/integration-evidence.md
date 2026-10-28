# External evidence and integration boundaries

These references were inspected for this foundation on 2026-10-25. They support
the narrow propositions below, not a tested Koiné integration. No external source
code was copied. Koiné remains MIT-licensed; inspecting prior art does not grant
permission to copy differently licensed implementations without their obligations.

## Public primary references

| Reference | Established by the reference | Limits for Koiné |
| --- | --- | --- |
| [Steamworks: IPlayerService / GetOwnedGames](https://partner.steamgames.com/doc/webapi/IPlayerService#GetOwnedGames) | Documents a key and player ID, visibility-dependent owned-game results, and filters including free-game handling. | A possible ownership channel, not proof of completeness from local manifests. API endpoint suitability, permissions, authentication, and response coverage need review before adoption. No key handling or login flow selected. |
| [Heroic upstream README](https://github.com/Heroic-Games-Launcher/HeroicGamesLauncher#readme) | Describes Epic support through Legendary, GOG through gogdl, Amazon through Nile, and Wine/Proton support. | Those execution responsibilities belong to Heroic. Knowing a backend's installation metadata does not establish that invoking the backend preserves Heroic configuration. No Heroic transport or schema established by this reference. |
| [Heroic wiki: Adding Games to Steam on Linux](https://github.com/Heroic-Games-Launcher/HeroicGamesLauncher/wiki/Adding-Games-to-Steam-on-Linux) | The inspected page, last edited in 2022, describes a script-based integration with full launch commands. | Historical integration advice, not a versioned guarantee of a current Heroic delegation interface. Koiné does not adopt extracted execution commands. |

The Valve Developer Community command-line and browser-protocol pages could not
be retrieved during this pass (HTTP 403). Their exact behavior was not verified.
No claims of universal `steam` executable availability, command completion
semantics, or specific current URI behavior depend on that failed lookup.

## Owner-supplied historical evidence

The foundation mandate supplies prior-art findings about GameIndex: Steam Web API
ownership, local library/manifests, incomplete installations, and independent
process monitoring. These motivate the contracts; they are not independently
reproduced integration results. No GameIndex code or schema is adopted.

The mandate also supplies the OpenBoxGL SPRAWL event: App ID `1549690`, imported
template `steam -applaunch {app_id}`, exit status `127`, no remaining game process
found in the reported inspection, and inconsistent frontend state. That evidence
does not identify the cause of the command failure or prove a particular process
history. The [synthetic regression](acceptance-cases.md#sprawl-exit-127) preserves
the required definite-failure behavior without importing logs or local state.

## Milestone 2 format evidence

On 2026-10-27, pinned steamlocate application/library models and Lutris state-flag
documentation were independently inspected before implementing the offline Steam
subset. The [format record](steam-discovery-format.md) links exact revisions,
selected rules and unsupported variants. The [executed report](storage-resilience-steam.md)
records synthetic filesystem, reconciliation, persistence and restart tests.
These establish fixture behavior, not live client/package compatibility, account
ownership, negative installation authority or launcher execution. No personal
library was inspected and no external implementation code was copied.

## Implementation evidence still needed

These are integration verification tasks, not reasons to change launcher authority:

- Extend the inspected Steam format subset to reviewed real discovery locations,
  client/package variants and live filesystem behavior under separate authorization.
  Completion/malformed/missing-root behavior is tested synthetically; none of the
  inspected sources currently establishes absence authority from file omissions.
- Verify a Steam handoff mechanism against supported launcher contexts, including
  absence, transport failure, and acceptance without a game process. Preserve
  Steam's configured execution behavior without editing `localconfig.vdf`.
- Establish Heroic discovery format/version coverage and a supported handoff that
  enters Heroic's own launch path. Verify that runner, prefix, and options remain
  Heroic's responsibility. Do not substitute direct Legendary, gogdl, Wine, or
  Proton invocation when the Heroic handoff is unavailable.
- Evaluate ownership API capability, permission, visibility, filtering, and
  authentication boundaries separately. Request an owner decision before adding
  an authentication strategy or external login flow.
- Select and validate process identity evidence before reporting an observed game
  process. No live installation or process-monitoring verification has occurred.

Any later external code reuse needs a license review and required attribution.
Public documentation, source inspection, synthetic testing, and live integration
testing establish different things and must be recorded separately.

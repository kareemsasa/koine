# Security policy

Koiné is pre-release software. There are no published releases or packages, so no
version is supported yet; fixes land on `main`.

## Reporting a vulnerability

Report a suspected vulnerability privately through GitHub's private vulnerability
reporting: **Security → Report a vulnerability** on this repository. Do not open a
public issue, pull request or discussion for an undisclosed vulnerability.

Include the affected revision, the steps or synthetic input that reproduce the
problem, and the effect you observed. Use synthetic data only: do not send personal
game libraries, account identifiers, launcher configuration or tokens.

Reports are handled by the maintainer on a best-effort basis. No response time is
guaranteed.

## Scope

In scope:

- Koiné reading files outside the root it was explicitly given, including through
  symlinks, path traversal or mount aliases;
- Koiné modifying launcher files or configuration, which discovery must never do;
- launcher manifests or library indexes that make Koiné crash, hang, or corrupt its
  library database;
- Koiné persisting launcher or account data that its documentation says is not
  retained;
- committed library records being lost or corrupted, or recovery silently repairing
  state that it should refuse.

Out of scope:

- vulnerabilities in Steam, Heroic or other launchers themselves;
- attacks that require prior control of the user's account or of Koiné's own
  database files.

The [discovery](docs/discovery-contract.md) and [launch](docs/launch-contract.md)
contracts define the intended behavior.

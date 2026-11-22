# Security policy

## Reporting a vulnerability

Please report security problems **privately**, not in a public issue. Use
GitHub's "Report a vulnerability" button on the repository's **Security** tab.
Include what you found, how to reproduce it, and the version you tested. This is
a small project, so please allow a few days for a reply.

## Supported versions

Only the latest release receives fixes.

## By design

These are intended behaviours, not vulnerabilities:

- The MCP server listens on the loopback address only; other computers cannot reach it.
- File and project tools over MCP are **deny by default**: they work only inside
  folders you list in `MODELER3D_AI_APPROVED_ROOTS`.
- MCP clients cannot send arbitrary URLs, paths or commands to the updater.
- The update check (one request to GitHub at startup) is the only network call.
  Installers are checked against their SHA-256 digest before use.
- There is no account, telemetry or crash upload.

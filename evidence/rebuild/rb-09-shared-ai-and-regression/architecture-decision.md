# RB-09 integration architecture decision

Date: 2026-11-02

## Decision

Keep `CommandState` and its `Scene` as the desktop document owner. Run the MCP
catalog and semantic handlers from `modeler3d-mcp`, but route a local client
through an authenticated loopback bridge owned by the Rust desktop. The bridge
listener parses requests off-thread and queues bounded requests to the eframe
owner thread; only that thread may attach a document to the adapter or commit
an isolated operation result. The `r4_mcp` executable remains the stdio MCP
gateway and proxies each tool call to the runtime descriptor published by the
desktop. It must return a bounded unavailable result when the desktop is not
running.

For synchronous tools, temporarily move the exact live `Scene` into the R4
adapter on the UI thread, synchronize the desktop path/dirty/view/clipboard
metadata, call the existing validated handler, and move that same scene back
before returning. This preserves the core undo/redo history, project identity,
revision, change log, and object IDs. UI panels and the renderer read the same
`CommandState` after the queued request completes.

For long operations, create a private adapter over a `SceneSnapshot` captured
from the desktop, including project ID, revision, and document generation.
The worker is isolated from the live document. The UI polls the bounded
operation record and, on completion, commits one reversible core scene command
only if project ID, document generation, and starting revision still match.
Failure, cancellation, or a stale result leaves the live scene and history
untouched. File-only operations do not create scene history. Save/open/new
metadata is synchronized on the UI owner thread after the operation succeeds.

The 60-tool catalog, ordering, profile visibility, schemas, structured
results, replay bounds, approved-root policy, and operation lifecycle remain
owned by the existing R4 adapter. The app treats bridge startup failure as an
optional-service failure and keeps ordinary GUI use available. The listener
binds only to IPv4 loopback, uses a per-run secret, publishes its process and
startup identity atomically, and removes only a descriptor it still owns.

The desktop's bridge worker reads bounded HTTP requests and queues them through
a bounded synchronous channel; only `McpDesktopIntegration::poll` on the app's
owner thread dispatches tools. Requests require the per-run bearer secret,
exact loopback Host, JSON content type, and no browser Origin. Body, header,
queue, and response waits are capped. The stdio gateway verifies the descriptor
schema, PID, loopback host, port, API version, startup token, and secret shape,
then checks `/status` identity before forwarding `/mcp`. It returns a structured
retryable unavailable error if the desktop is absent or stale.

Shutdown sets the worker stop flag, wakes the nonblocking listener with a
loopback connection, joins the worker, and removes the runtime descriptor only
when its PID and startup ID still match the descriptor this bridge wrote.
Queued requests that time out or whose client disconnects are marked canceled
and skipped by the UI poll if they have not begun. Bridge startup is optional;
an initialization error is reported in desktop status and does not block normal
GUI use.

## Rejected approaches

- A second long-lived MCP scene is not authoritative and would split undo,
  dirty state, and UI rendering.
- Direct worker mutation of `Modeler3dApp` or its widgets violates eframe
  thread ownership and the stale-result protections already used by desktop
  jobs.
- Cloning the desktop scene for every synchronous request would lose core
  history/change-log state and multiply mesh memory.
- A generic arbitrary-action endpoint would weaken the fixed 60-tool
  contract.

## Verification boundary

The pre-change direct R4 probe passed 14 MCP library tests, but it exercised an
independent `R4Server` scene and was not accepted as proof of desktop sharing.
After integration, focused checks pass 19 MCP library tests, 5 app integration
tests, and 3 core snapshot-commit/history tests. The full workspace passes 327
tests and all documented engineering gates exit 0; exact commands and logs are
in `verification.md`.

The installed official Python MCP SDK 2.2.0 initialized the Release `r4_mcp`
stdio gateway and listed the ordered 60-tool catalog. With a deliberately
absent runtime descriptor, `get_status` returned the expected structured,
retryable `desktop_unavailable` error. This validates the MCP client/gateway
and closed-desktop response only. At final inspection, Windows `LockApp.exe`
PID 9808 was active and no Rust desktop process was available. Live forwarding,
shared-document behavior, cross-adapter undo/save/file/cancel behavior, and
paired native capture remain unverified and mandatory before RB-09 can close.

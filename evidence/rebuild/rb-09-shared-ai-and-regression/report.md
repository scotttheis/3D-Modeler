# RB-09 Shared AI and Regression Report

**Date:** 2026-11-02  
**Status:** implementation and automated verification pass; the mandatory
live-client/native desktop gate is unverified, so RB-09 is not closed.

## Delivered

- The Rust desktop remains the owner of the live `CommandState` and `Scene`.
  Synchronous MCP requests attach that exact document to the existing R4
  handlers on the eframe owner thread and return the same document, preserving
  identity, revision, dirty state, selection, view state, and undo/redo history.
- `r4_mcp` now runs a fixed-catalog stdio gateway. It validates the per-user
  runtime descriptor and forwards calls over an authenticated IPv4 loopback
  bridge owned by the desktop. The bridge checks process/startup identity,
  host, content type, origin, request size, queue capacity, and its per-run
  secret. It publishes no remote listener and emits no credential logs.
- Long operations work on bounded isolated scene copies. The UI thread commits
  a completed scene through one reversible core command only after checking
  project identity, document generation, starting revision, and path baseline.
  Failure, cancellation, and stale results leave the live scene/history
  untouched. Project replacement keeps the dirty-document guard, including
  same-identity open behavior.
- MCP startup is optional for the desktop. A failed bridge start leaves normal
  desktop startup available; a stdio client with no desktop receives a bounded,
  structured `desktop_unavailable` response.
- The public 60-tool catalog, order, schemas, profiles, protocol response
  shapes, replay behavior, approved-root policy, and operation status/cancel
  tools remain the existing R4 contract. No generic action tool was added.

## Verification

- Focused Rust checks pass: 19 MCP library tests, 5 app integration tests, and
  3 core isolated-commit/history tests.
- The Rust workspace passes all 327 tests. The documented format, check,
  Clippy, deny, metadata, app smoke, Release builds, and bloat commands all
  exited 0. The workspace bloat invocation emitted the installed tool's
  existing warning that `--workspace` is unused; the app executable was also
  measured explicitly.
- The read-only Python reference focus passed 12 tests using its existing
  Python 3.12.10 environment. Pytest scratch and logs were directed to this
  project on E:.
- The official Python MCP SDK 2.2.0 launched the Release Rust stdio executable,
  initialized MCP, listed all 60 tools in order, and got the expected structured
  retryable error when calling `get_status` with no desktop runtime descriptor.
  This verifies the client/gateway and closed-desktop path only; it does not
  verify forwarding into a live Rust GUI.
- The component catalog, parity matrix, evidence references, JSON manifests,
  capture metadata, and accepted/rejected evidence journals pass the
  project-local validator.

## Native gate and remaining product gaps

Windows `LockApp.exe` PID 9808 was active at the final process inspection. No
Rust desktop, Rust stdio server, or Python desktop window was available for the
paired live workflow. No native input or screenshot was attempted. The
following handoff cases therefore remain **unverified**: live cross-adapter
reads and edits, viewport/Items updates, shared GUI/MCP undo and redo,
save/reopen, project New/Open, approved and denied file paths, replay and
revision races through the bridge, long-operation cancellation/failure/stale
commit, and bridge shutdown/startup behavior in a running desktop. No paired
captures are accepted for RB-09.

The owner-reported **interface gap** and **floor gap** remain open. No visual
review or owner acceptance is claimed. Earlier visual and owner-review statuses
remain pending. RB-08's paired Rust native interaction matrix and completion
snapshot are still pending; this RB-09 evidence does not satisfy them.

## Recovery and boundary

The pre-edit additive RB-09 baseline is recorded in
`baseline-snapshot-receipt.md`. The final additive copy is
`G:\Builder3D-Rust_snapshot_20261102-152935_RB-09-handoff`; its staging copy
and final receipt are recorded in `snapshot-receipt.md`. It is explicitly an
RB-09 handoff snapshot because the required native gate is pending, not an
RB-09 completion receipt. The worktree remains uncommitted and unstaged.
`L:\Builder3D` remains read-only. RB-10 has not started.

## Owner pause and current project path (2026-11-03)

The owner paused native work and set the active Rust source/build/run path to
`E:\Builder3D-Rust`. Use `G:\` for primary recovery snapshots; removable USB
copies are occasional one-offs. The one-off USB copy is documented in
`usb-backup-receipt.md`. G: snapshots and USB copies are recovery copies, not
working trees. `L:\Builder3D` remains the read-only Python reference/shipping
build. RB-09 native evidence is still pending and RB-10 has not started.

When the desktop session is available, resume with the native sequence in
`walkthrough.md` and `requirement-test-map.md`. Do not begin RB-10 until that
gate and its evidence are resolved.

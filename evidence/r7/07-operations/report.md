# R7-07 — application operation lifecycle coverage

Component: `modeler3d.app.operations`

## Implemented contract

- Added a thread-safe, single-active `OperationRegistry` for desktop workers.
- Added typed queued/running/committing/cancelling/completed/failed/cancelled
  phases, bounded progress and messages, structured results/errors, and
  cooperative cancellation handles.
- Terminal records are retained for a bounded count and retention window;
  terminal updates remain idempotent and cannot be reopened as active work.
- Connected the registry to `CommandState` as the application-owned seam so
  future worker adapters do not need to own parallel lifecycle logic.
- Focused tests cover normal completion, busy rejection, cancellation races,
  failure payloads, and bounded history.

The MCP registry remains its existing adapter-local implementation; this slice
keeps both surfaces on the same public phase contract without introducing a
crate dependency cycle.

No Python source, R1–R6 evidence, or existing snapshot was modified.

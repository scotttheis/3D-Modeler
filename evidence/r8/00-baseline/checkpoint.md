# R8 checkpoint — baseline

- Objective: establish a recoverable post-R7 baseline before implementing the
  native eframe shell polish pass.
- Completed: read workspace/project instructions and the R8 brief; inspected
  the current shell, R7 catalog/evidence/log, and Python UI oracle; ran the
  focused app tests and native smoke; launched and captured the current native
  window; closed the native process.
- Snapshot: `G:\Builder3D-Rust_snapshot_20261030-114800_R8-baseline`
  (168 files, 1,644,529 bytes; source and snapshot totals match).
- Unresolved UI gaps: ribbon customization and grouping, welcome/recent
  surface, complete dialogs/settings, object-panel semantics and feedback,
  numeric/accessibility workflow, viewport cues, and operation/print feedback.
- Constraints: no Python source, existing snapshot, or commit was changed.
- Next slice: implement the shell foundation and persisted presentation state
  (welcome/recent routing, ribbon favorites/order/pinning, theme surfaces,
  keyboard/status infrastructure) with focused state tests.

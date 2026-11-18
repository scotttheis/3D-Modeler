# Resize Settle and robot-claw cursors

Captured with the real application's review harness in both themes.
These images are engineering evidence; owner acceptance is pending.

- `before-resize-settle-before-*` / `after-resize-settle-before-*`: same
  tilted, nonuniformly resized input (the PNG pairs match byte for byte).
- `before-resize-settle-*` / `after-resize-settle-*`: old local rotation
  and corrected rigid floor placement. Existing floor-axis Resize can shear
  a tilted object; Settle now preserves that input shape exactly.
- `before-resize-drag-*`: original ready/held semantics (native OS cursors
  are excluded from framebuffer captures).
- `after-grab-ready-*` / `after-grab-held-*`: actual Resize handle hover and
  staged drag with the shared open/closed vector claw, jaw-gap hotspot.
- `release-*`: final Release captures.

The claw adapter runs after egui's built-in drag-and-drop pass, so every
Grab/Grabbing request has the same artwork. Move/Rotate/Resize hit testing
is unchanged; camera orbit/pan and Grab brush use the same held feedback.
Non-grab pointers retain their normal cursor; focus loss and pointer leave
are covered by the existing gesture cancellation and new cursor tests.
The review-only resting pointer is focused for unattended captures.

Geometry verification compares every pair of world vertices before/after
Settle, checks Y=0, repeats with both Resize bases and selection sizes,
and verifies one undo/redo and repeat no-op. MCP additionally compares exact
matrices with desktop single/multi Settle and checks grouped undo/no-op.
An invalid-target test proves the shared service leaves history and transforms
unchanged. Historical Python single-object matrices are retained as evidence
of the corrected bug; group fixture parity remains checked.

Read E:\RustDeveloper.md and project AGENTS.md for future software work.

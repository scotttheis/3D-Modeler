# R7-08 — application view-state coverage

Component: `builder3d.app.view-state`

## Implemented contract

- Added a typed application controller over the existing renderer-neutral
  camera with finite/range validation for orbit pose, distance, and center.
- Ported reset, top, front, and side presets while preserving display toggles.
- Ported frame-all and frame-selected behavior, including the Python fallback
  to all objects when the requested selection is empty.
- Connected the existing `CenterView`, `FrameSelection`, `ResetView`, `ViewTop`,
  `ViewSide`, grid, wireframe, and x-ray commands to real camera/display state
  instead of orientation labels alone.
- Added complete camera-state replacement to the viewport camera seam and
  focused tests for bounds, presets, framing, fallback, and empty-scene no-op.

No Python source, R1–R6 evidence, or existing snapshot was modified.

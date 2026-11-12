# R7-04 — renderer-neutral snapping

Component: `builder3d.viewport.snapping`

## Selection

The Python snapping module is shared by the gizmo and application transform
service. It captures authoritative geometry at drag start and performs later
pointer updates over scalar landmarks. The Rust viewport did not yet expose
that seam, so object placement could not consume the reference snap policy.

## Implemented contract

- Added immutable `SnapSceneSnapshot`, `SnapTarget`, and `SnapBounds` values
  that exclude moving objects and retain deterministic scene order.
- Added typed object, origin, and build-floor landmark candidates with
  minimum/center/maximum moving landmarks.
- Added fixed, floor-fixed, and camera-scaled tolerance policies with finite
  validation and a typed `SnapViewContext`.
- Added nearest scalar correction with strict tolerance capture and the
  reference crossing rule, plus `SnapState` capture/release hysteresis.
- Added `CommandState::snap_selected_translation`, which builds a snapshot
  from the live scene and commits the corrected movement through the atomic
  transform service.
- Added focused viewport tests for target exclusion, nearest correction,
  floor/view tolerance resolution, and capture/release behavior.

The implementation is renderer-neutral; it does not add shell polish or scan
mesh vertices during pointer updates.

# R7-03 — atomic object-transform workflows

Component: `builder3d.app.commands.transforms`

## Selection

The Python application keeps numeric transforms and multi-object manipulation
in `TransformService`, rather than embedding them in the toolbar. The Rust
command surface previously applied several object transforms independently and
used a fixed X-only arrangement path. This slice ports the application seam
and keeps the core scene as the only mutation owner.

## Implemented contract

- Added typed `Axis`, `Alignment`, `GroupTransform`, and finite world-bounds
  values in a dedicated application transform module.
- Group translation, rotation, scaling, and reflection compute all target
  transforms before committing one `GroupedCommand`.
- Alignment and arrangement operate on transformed world bounds, preserve
  linear transforms, and support the build's X/Y/Z coordinate contract.
- Settle centers the selection on X/Z and places its lowest world Y at the
  build bed, matching the Python reference's coordinate convention.
- Numeric position and world-size updates validate finite/positive values and
  remain undoable.
- Existing desktop `Mirror`, `Settle`, `Align`, and `Arrange` dispatch now use
  the shared atomic transform seam.

No Python source, R1–R6 evidence, or existing snapshot was modified.

## Deliberate boundary

Viewport snapping and camera-derived tolerance remain the next independent
conversion slice. The shell transform bar is still intentionally deferred
until conversion coverage is complete.

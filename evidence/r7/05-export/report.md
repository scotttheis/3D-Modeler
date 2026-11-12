# R7-05 — application export workflow coverage

Component: `builder3d.app.export`

## Selection

The Python workflow service exports the selected visible objects, or all
visible objects when there is no selection. The previous Rust command exported
only the first selected mesh and did not route the application-level 3MF,
glTF/GLB, or WRL scene-item contracts.

## Implemented contract

- Added a typed export selection seam that filters hidden objects and rejects
  an empty eligible set before any writer runs.
- Routed STL, OBJ, and PLY through one concatenated world-space mesh while
  preserving scene order.
- Routed glTF/GLB, 3MF, and WRL through their core scene-item writers with
  object names, transforms, colors, and mesh identity preserved at the format
  boundary.
- Added typed output reports and extension/path errors.
- Updated `CommandState::export_to_path` to use the shared workflow instead of
  exporting only the first selected object.
- Added focused tests for selection fallback, hidden-object filtering,
  multi-object glTF export/reopen, and unsupported extensions.

No Python source, R1–R6 evidence, or existing snapshot was modified.

# R3f local mesh brushes

## Completed subtask

Implemented the four ordered topology-preserving local brush operations:
Elastic Grab, Crease Polish, Smooth Brush, and Blob. Each has typed bounded
parameters, Python-compatible falloff/direction/symmetry choices, finite
center/path validation, resource work checks, bounded cumulative displacement,
topology preservation, reports, and undoable scene integration.

## Changed files

- crates/modeler3d-core/src/brushes.rs
- crates/modeler3d-core/src/lib.rs
- crates/modeler3d-core/src/commands.rs
- crates/modeler3d-core/src/scene.rs
- crates/modeler3d-core/tests/mesh_ops_brushes.rs

## Behavioral evidence

- Elastic Grab applies smooth weighted local offsets, mirrors both centers and
  offsets for symmetry, preserves faces, and enforces the 2,000 mm offset cap.
- Crease Polish follows a path toward/away from centers with smooth/linear/
  constant falloff and a radius-derived cumulative displacement cap.
- Smooth Brush uses shared adjacency and vertex normals, retaining tangent
  smoothing plus configurable normal shrinkage.
- Blob recomputes normals per path center and inflates/deflates along them;
  outward and inward volume directions are both covered.
- Scene application replaces only mesh data, preserves object identity, and
  supports undo/redo.

## Verification

- Python: tests/test_mesh_ops.py -k "elastic_grab or crease_polish or
  smooth_brush or blob" -q: 5 passed, 78 deselected.
- Python: tests/test_scene.py -k "elastic_grab_command or crease_polish_command
  or smooth_brush_command or blob_command" -q: 4 passed, 65 deselected.
- Rust: tests/mesh_ops_brushes.rs: 6 passed.
- Rust: cargo fmt --all -- --check, cargo check -p modeler3d-core, and
  cargo clippy -p modeler3d-core --all-targets -- -D warnings passed.
- New brush/R3 production files contain zero forbidden panic-style or
  debugging markers.

## Snapshot

R3f snapshot:
G:\Builder3D-Rust_snapshot_20261029-144529_R3f-brushes
It excludes E:\3D-Modeler\target. Exact verification is in
verification.md.

## Remaining R3 work

Density normalization, then final R3 verification and backup.


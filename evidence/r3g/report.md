# R3g density normalization

## Completed subtask

Implemented the final ordered R3 mesh operation, `normalize_face_density`,
with bounded local face selection, conforming selected-region edge splits,
conservative internal edge collapses, cancellation/progress callbacks,
resource budgets, finite-result validation, boundary preservation evidence,
reports, and scene/command history integration.

## Changed files

- crates/builder3d-core/src/density.rs
- crates/builder3d-core/src/lib.rs
- crates/builder3d-core/src/commands.rs
- crates/builder3d-core/src/scene.rs
- crates/builder3d-core/tests/mesh_ops_density.rs

## Behavioral evidence

- Center selection includes faces whose triangle center plus vertex-radius
  reaches a valid local center and always includes the nearest face per center.
- Partial selections split only selected-region internal edges, retaining the
  original selection boundary vertices; closed dense regions may collapse
  short internal edges toward the requested target length.
- The source mesh is cloned before all work, and scene replacement happens
  only after the complete result and report exist.
- Cancellation and progress are checked at topology boundaries. Invalid
  centers, face indices, target/radius parameters, source/result limits, and
  work budgets return typed errors before scene mutation.
- Command dispatch uses `normalize_face_density`; scene application preserves
  object identity and supports one-step undo/redo.

## Verification

- Python: `tests/test_mesh_ops.py -k "normalize_face_density or
  select_face_region" -q`: 6 passed, 77 deselected.
- Python: `tests/test_scene.py -k normalize_face_density -q`: 1 passed,
  68 deselected.
- Python: `tests/test_ai_commands.py -k normalize_face_density -q`: 1 passed,
  64 deselected.
- Python: `tests/test_ai_qt_bridge.py -k live_normalize -q`: 1 passed,
  13 deselected.
- Rust: `tests/mesh_ops_density.rs`: 6 passed.
- Rust R3 focused suites: booleans 5, repair 5, simplification 5,
  subdivision 4, smoothing 4, brushes 6, density 6; all passed.
- `cargo fmt --all -- --check`, core check, and all-target clippy with
  `-D warnings` passed after the density integration.
- New R3 production operation files contain zero forbidden panic-style or
  debugging markers.

## Snapshot

R3g snapshot:
`G:\Builder3D-Rust_snapshot_20261029-145400_R3g-density`

It excludes `E:\Builder3D-Rust\target`. Exact count, byte, path, length, and
SHA-256 verification is recorded in `verification.md`.

## Remaining R3 work

Final workspace verification, relevant Python reference verification, and the
final R3 backup remain. The next authoritative project stage after R3 is R4
MCP server work.

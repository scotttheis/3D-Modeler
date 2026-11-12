# R3e mesh smoothing

## Completed subtask

Implemented bounded uniform Laplacian smoothing in the core. The operation
uses shared vertex adjacency, applies finite half-step updates for each
iteration, preserves topology, reports displacement and volume evidence, and
returns an exact no-op for zero iterations. Iteration, work, memory, and mesh
limits are checked before a scene replacement.

## Changed files

- crates/builder3d-core/src/smooth.rs
- crates/builder3d-core/src/lib.rs
- crates/builder3d-core/src/commands.rs
- crates/builder3d-core/src/scene.rs
- crates/builder3d-core/tests/mesh_ops_smooth.rs

## Behavioral evidence

- One iteration changes a box's positions while preserving vertex/face counts
  and finite coordinates.
- Zero iterations returns an unchanged mesh/report.
- Excess iterations and work budgets return typed resource errors.
- Scene integration preserves object identity and undo/redo.

## Verification

- Python: tests/test_chunk7.py -k Smooth -q: 2 passed, 31 deselected.
- Rust: tests/mesh_ops_smooth.rs: 4 passed.
- Rust: cargo fmt --all -- --check, cargo check -p builder3d-core, and
  cargo clippy -p builder3d-core --all-targets -- -D warnings passed.
- New R3 operation files contain zero forbidden panic-style or debugging
  markers.

## Snapshot

R3e snapshot:
G:\Builder3D-Rust_snapshot_20261029-144108_R3e-smooth
It excludes E:\Builder3D-Rust\target. Exact verification is in
verification.md.

## Remaining R3 work

All five brushes; density normalization; final R3 verification and backup.


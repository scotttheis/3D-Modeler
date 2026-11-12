# R3d mesh subdivision

## Completed subtask

Implemented global conforming triangle subdivision from the Python reference.
The operation selects sorted interior edges first and boundary edges only when
needed to reach the approximately doubled face budget, creates one shared
midpoint per selected edge, and emits all eight global face cases. It checks
limits before midpoint allocation and returns volume, extents, topology, and
selection evidence.

## Changed files

- crates/builder3d-core/src/subdivide.rs
- crates/builder3d-core/src/lib.rs
- crates/builder3d-core/src/commands.rs
- crates/builder3d-core/src/scene.rs
- crates/builder3d-core/tests/mesh_ops_subdivide.rs

## Behavioral evidence

- A closed box grows from 12 to 24 faces, remains watertight, preserves
  extents and volume, and selects no boundary edges.
- Scene integration replaces only the authoritative mesh, preserves object
  identity, and supports undo/redo.
- Unsupported topology and face/vertex/work limits return typed errors before
  a scene command commits.

## Verification

- Python: tests/test_chunk7.py -k Subdivide -q: 1 passed, 32 deselected.
- Rust: tests/mesh_ops_subdivide.rs: 4 passed.
- Rust: cargo fmt --all -- --check, cargo check -p builder3d-core, and
  cargo clippy -p builder3d-core --all-targets -- -D warnings passed.
- New subdivision/simplification/repair/boolean production files contain zero
  forbidden panic-style or debugging markers.

## Snapshot

R3d snapshot:
G:\Builder3D-Rust_snapshot_20261029-143911_R3d-subdivide
It excludes E:\Builder3D-Rust\target. Exact verification is in
verification.md.

## Remaining R3 work

Smoothing; elastic grab, crease polish, smooth brush, blob, and density
normalization; then final R3 verification and backup.


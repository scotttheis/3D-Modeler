# R3c mesh simplification

## Completed subtask

Implemented deterministic bounded triangle simplification with a conservative
shortest-edge collapse policy. Interior edges are preferred to preserve
closed topology, every collapse removes degenerate/duplicate triangles, and
the operation stops at or below the requested target or returns a bounded
warning when no safe collapse remains. A no-op clone/report is returned when
the source is already at or below the target.

## Changed files

- crates/builder3d-core/src/simplify.rs
- crates/builder3d-core/src/lib.rs
- crates/builder3d-core/src/commands.rs
- crates/builder3d-core/src/scene.rs
- crates/builder3d-core/tests/mesh_ops_simplify.rs

## Behavioral evidence

- A Rust sphere fixture reduces to 100 or fewer faces with non-empty finite
  geometry.
- Small meshes remain unchanged and report changed=false.
- Resource limits fail before a replacement is committed.
- Scene integration preserves object identity and supports undo/redo through
  the existing ReplaceMesh history command.
- Reports include before/after counts, target, accepted collapses, work,
  conservative deviation, volumes, and warnings.

## Verification

- Python: tests/test_chunk7.py -k Simplify -q: 2 passed, 31 deselected.
- Rust: tests/mesh_ops_simplify.rs: 5 passed.
- Rust: cargo fmt --all -- --check, cargo check -p builder3d-core, and
  cargo clippy -p builder3d-core --all-targets -- -D warnings passed.
- New simplifier/repair/boolean production files contain zero forbidden
  panic-style or debugging markers.

## Snapshot

R3c snapshot:
G:\Builder3D-Rust_snapshot_20261029-143659_R3c-simplify
It excludes E:\Builder3D-Rust\target. Exact verification is in
verification.md.

## Remaining R3 work

Subdivision; smoothing; elastic grab, crease polish, smooth brush, blob, and
density normalization; then final R3 verification and backup.


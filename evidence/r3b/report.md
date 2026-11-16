# R3b mesh repair

## Completed subtask

Implemented the non-mutating Python-compatible repair pipeline and connected
it to boolean operand preparation. The extended profile follows the captured
12-stage order; the simple profile follows its four-stage order; enhanced is
bounded to the same NMR-compatible cleanup family. Repair performs tolerance
welding, winding normalization, small-hole closure, duplicate/degenerated-face
cleanup, tiny-shell removal, safe compaction, and bounded planar cleanup.

## Changed files

- crates/modeler3d-core/src/mesh_ops.rs
- crates/modeler3d-core/src/repair.rs
- crates/modeler3d-core/src/lib.rs
- crates/modeler3d-core/tests/mesh_ops_repair.rs

## Behavioral evidence

- Reversed box winding is repaired to a valid, positive-volume closed mesh
  without mutating the source.
- An open box with one square side removed is closed by the bounded hole
  repair and reports the exact Python extended stage names.
- Duplicate faces and a tiny disconnected shell are removed while two
  substantial disconnected shells remain a safe best-effort candidate rather
  than being silently discarded.
- Flat fan cleanup adds an interior split without changing volume or boundary
  connectivity; dense open flat grids remain open, duplicate-free, and within
  their original face count.
- Invalid boolean operands now receive a non-mutating extended repair attempt;
  valid operands are cloned directly and are not repaired.

## Verification

- Python: tests/test_mesh_ops.py -k "repair or validate or health or
  open_surface or Conversion" -q: 17 passed, 66 deselected, one known
  trimesh invalid-value warning.
- Python: tests/test_chunk7.py -k Repair -q: 3 passed, 30 deselected.
- Rust: focused boolean and repair suites: 10 passed.
- Rust: cargo fmt --all -- --check, cargo check -p modeler3d-core, and
  cargo clippy -p modeler3d-core --all-targets -- -D warnings passed.
- New production operation files contain zero .unwrap(), .expect(),
  dbg!, todo!, panic!, or unimplemented! matches.

## Snapshot

The R3b snapshot is
G:\Builder3D-Rust_snapshot_20261029-143343_R3b-repair; it excludes
E:\3D-Modeler\target. Exact path, byte, and SHA-256 verification is in
verification.md.

## Remaining R3 work

Simplification; subdivision; smoothing; elastic grab, crease polish, smooth
brush, blob, and density normalization; then final R3 verification and
backup.


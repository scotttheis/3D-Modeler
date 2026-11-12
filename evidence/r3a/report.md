# R3a boolean mesh operations

## Completed subtask

Implemented the first R3 operation: bounded boolean union, difference, and
intersection through the `manifold-csg` Rust kernel. The operation boundary
validates mesh health, complexity, memory, and work budgets before entering
the kernel; it reports health, volume, source-face provenance, estimated peak
memory, and regularized cut-cap patches. Scene integration applies the result
in the target object's local space, removes the tool atomically, preserves the
target identity/state, and records one undoable grouped command.

## Changed files

- `Cargo.toml`, `Cargo.lock`
- `crates/builder3d-core/Cargo.toml`
- `crates/builder3d-core/src/error.rs`
- `crates/builder3d-core/src/lib.rs`
- `crates/builder3d-core/src/mesh_ops.rs`
- `crates/builder3d-core/src/transform.rs`
- `crates/builder3d-core/src/commands.rs`
- `crates/builder3d-core/src/scene.rs`
- `crates/builder3d-core/tests/mesh_ops_booleans.rs`

## Behavioral evidence

- Box union, difference, and intersection volumes match the expected
  `1.5`, `0.5`, and `0.5` cubic millimetres within the focused Rust test
  tolerance and remain closed manifolds.
- Inputs remain unchanged, including when a boolean fails resource checks or
  rejects an open operand.
- Source-face labels identify target/tool result contributions.
- A transformed target/tool scene operation preserves target name, color,
  metadata, visibility, transform, selection, and undo/redo behavior.
- Difference can regularize a flat tool cut-cap patch without changing the
  result volume.

## Verification

- Python reference: `tests/test_mesh_ops.py -k "boolean or Conversion" -q`:
  11 passed, 72 deselected, one known `trimesh` invalid-value warning.
- Rust: `cargo fmt --all -- --check` passed.
- Rust: `cargo check -p builder3d-core` passed.
- Rust: `cargo clippy -p builder3d-core --all-targets -- -D warnings` passed.
- Rust: `cargo test -p builder3d-core` passed, including the five focused
  boolean tests and all existing core golden tests.
- R3 production files have zero matches for `.unwrap()`, `.expect()`,
  `dbg!`, `todo!`, `panic!`, or `unimplemented!`. Existing panic markers in
  `crates/builder3d-core/src/lib.rs` are confined to its pre-existing test
  module and were not added by this subtask.

## Snapshot

The required pre-R3 baseline is
`G:\Builder3D-Rust_snapshot_20261029-140736_R3-baseline`, verified at 178
files and 1,842,915 bytes with matching relative paths and SHA-256 hashes.
The first completed R3a copy was
`G:\Builder3D-Rust_snapshot_20261029-142339_R3a-booleans`. After correcting
the manifest wording in the log, the final additive R3a snapshot is
`G:\Builder3D-Rust_snapshot_20261029-142448_R3a-booleans-final`; it excludes
`E:\Builder3D-Rust\target`. Its exact manifest verification is recorded in
`verification.md`.

## Remaining R3 work

Mesh repair; simplification; subdivision; smoothing; elastic grab, crease
polish, smooth brush, blob, and density-normalization brushes; then final
R3 verification and backup.


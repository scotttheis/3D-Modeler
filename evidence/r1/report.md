# 3D Modeler Rust R1 core evidence

This records the R1 core acceptance checks performed on 2026-10-29.

- `cargo fmt --all -- --check`: passed.
- `cargo test --workspace`: passed; the R1 core crate has 7 passing tests.
- `cargo clippy --all-targets -- -D warnings`: passed.
- `cargo deny check`: passed. It reports existing non-blocking duplicate
  `hashbrown`/`syn` versions and one unmatched BSD-3-Clause allowance.
- `cargo audit`: unavailable on this host.

## Python project-format comparison

The fixture was produced by the Python reference using its existing Python
3.12 virtual environment and `trimesh` 4.12.2. The source archive was 606
bytes with members `meshes/mesh_0.stl` and `scene.json`.

- Rust `load_project_bytes` followed by unchanged `save_project_bytes` was
  byte-identical to the Python archive.
- The 684-byte mesh member SHA-256 was
  `74120293f87d683a176db536c1d993ba71ebba56c5b10383b0e31e0375d19ae1` in
  both the Python source and Rust's changed-color archive.
- A Rust color mutation produced a 608-byte archive that reopened in the
  Python reference with the expected float32 RGBA values and scene revision.

The temporary comparison archives were kept under `target/`, which is ignored
and excluded from release snapshots.

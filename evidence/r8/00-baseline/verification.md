# R8 baseline verification

All commands were run from `E:\3D-Modeler` before the first R8 source
change.

| Check | Result |
|---|---|
| `cargo test -p modeler3d-app --locked` | PASS — 51 unit tests, doctests pass |
| `cargo run -p modeler3d-app --locked -- --smoke` | PASS — `objects=2 selected=1 tabs=5` |
| Native launch | PASS — `modeler3d-app.exe` opened a responsive `3D Modeler` window |
| Python source/snapshots | unchanged; Python was used only as the read-only oracle |

The first failed desktop screenshot capture was not used as evidence; the
PrintWindow capture is the authoritative baseline image.

Baseline recovery snapshot:

- `G:\Builder3D-Rust_snapshot_20261030-114800_R8-baseline`
- robocopy exit: 1 (files copied; success range)
- source files: 168; snapshot files: 168
- source bytes: 1,644,529; snapshot bytes: 1,644,529
- verification: PASS

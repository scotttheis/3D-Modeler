# R7-11 verification

## Focused behavior

- `cargo test -p modeler3d-platform --locked`: pass; known-slicer discovery
  and typed launch-boundary tests pass.
- `cargo test -p modeler3d-app --locked`: pass; staged 3MF, Bambu individual
  STL, empty-visible-scene, and shared export-routing tests pass.

## Workspace gates

The final locked workspace gate results and recovery-snapshot counts are
recorded here after the full R7-11 verification run.

- `cargo fmt --all -- --check`: pass.
- `cargo check --workspace --locked`: pass.
- `cargo test --workspace --locked`: pass, 164 tests plus zero-test doctest
  runs.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`: pass.
- `cargo deny check`: pass for advisories, bans, licenses, and sources; the
  existing duplicate-version diagnostics remain warnings.
- `cargo metadata --locked --format-version 1 --no-deps`: pass.
- Catalog TOML/path validation: pass, 23 unique IDs and 150 referenced paths.
- Native smoke: pass, `3D Modeler shell smoke: objects=2 selected=1 tabs=5`.

No commit was created. Python source and existing snapshots were not written.

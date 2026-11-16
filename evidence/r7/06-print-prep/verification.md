# R7-06 verification

## Focused behavior

- `cargo test -p modeler3d-app --locked`: pass, 32 application tests.
- The print-preparation tests cover transformed bed bounds, build-surface
  warnings, scale/hidden severity, clean geometry, and typed invalid input.

## Workspace gates

- `cargo fmt --all -- --check`: pass.
- `cargo check --workspace --locked`: pass.
- `cargo test --workspace --locked`: pass, 151 tests plus zero-test doctest
  runs.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`: pass.
- `cargo deny check`: pass for advisories, bans, licenses, and sources; the
  existing duplicate-version diagnostics remain warnings.
- `cargo metadata --locked --format-version 1 --no-deps`: pass.
- Catalog TOML/path validation: pass, 17 unique IDs and all referenced paths
  present.
- Native smoke: pass, `3D Modeler shell smoke: objects=2 selected=1 tabs=5`.

No commit was created. Python source and existing snapshots were not written.

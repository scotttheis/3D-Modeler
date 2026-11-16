# R7-05 verification

## Focused behavior

- `cargo test -p modeler3d-app --locked`: pass, 28 application tests.
- The export tests verify selected-visible precedence, visible fallback,
  hidden-object rejection, multi-object glTF routing and reopen, and typed
  unsupported-extension rejection.
- Core format golden suites remain green for STL/OBJ/PLY, glTF/GLB, 3MF, and
  WRL.

## Workspace gates

- `cargo fmt --all -- --check`: pass.
- `cargo check --workspace --locked`: pass.
- `cargo test --workspace --locked`: pass, 147 tests plus zero-test doctest
  runs.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`: pass.
- `cargo deny check`: pass for advisories, bans, licenses, and sources; the
  existing duplicate-version diagnostics remain warnings.
- `cargo metadata --locked --format-version 1 --no-deps`: pass.
- Catalog TOML/path validation: pass, 16 unique IDs and all referenced paths
  present.
- Native smoke: pass, `3D Modeler shell smoke: objects=2 selected=1 tabs=5`.

No commit was created. Python source and existing snapshots were not written.

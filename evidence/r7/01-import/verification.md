# R7-01 verification

## Focused behavior

- `cargo test -p modeler3d-app --locked`: pass, 20 tests.
- The import tests load the Python STL fixture, verify reference placement,
  verify centimetre scaling, select the imported object, and undo the whole
  import as one history transition.
- Unsupported WRL import returns a typed error with zero objects, zero scene
  revision, and no dirty state.
- Native smoke: `3D Modeler shell smoke: objects=2 selected=1 tabs=5`.

## Workspace gates

- `cargo fmt --all -- --check`: pass.
- `cargo check --workspace --locked`: pass.
- `cargo test --workspace --locked`: pass, 136 tests plus zero-test doctest
  runs.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D
  warnings`: pass.
- `cargo deny check`: pass for advisories, bans, licenses, and sources. The
  existing duplicate-version diagnostics remain warnings.
- `cargo metadata --locked --format-version 1 --no-deps`: pass.
- The component catalog parses as TOML, contains 12 unique IDs, and every
  referenced Rust, test, and evidence path exists.

No commit was created. R1–R6 source and evidence were not changed, and no
Python source was written.

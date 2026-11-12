# R7-03 verification

## Focused behavior

- `cargo test -p builder3d-app --locked`: pass, including 25 application
  tests.
- Focused transform tests pass for atomic group translation and undo, world
  bounds alignment and arrangement, Y-axis settling, one-entry mirroring,
  and numeric position/size updates.
- Existing application command tests pass with `Mirror`, `Settle`, and the
  single-selection `Arrange` no-op contract preserved.

## Workspace gates

- `cargo fmt --all -- --check`: pass.
- `cargo check --workspace --locked`: pass.
- `cargo test --workspace --locked`: pass, 141 tests plus zero-test doctest
  runs.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`: pass.
- `cargo deny check`: pass for advisories, bans, licenses, and sources; the
  existing duplicate-version diagnostics remain warnings.
- `cargo metadata --locked --format-version 1 --no-deps`: pass.
- Catalog TOML/path validation: pass, 14 unique IDs and 80 referenced paths.
- Native smoke: pass, `Builder3D shell smoke: objects=2 selected=1 tabs=5`.

No commit was created. Python source and existing snapshots were not written.

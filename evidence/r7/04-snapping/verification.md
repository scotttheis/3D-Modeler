# R7-04 verification

## Focused behavior

- `cargo test -p builder3d-viewport snapping --locked`: pass, 3 focused
  snapping tests.
- `cargo test -p builder3d-app --locked`: pass, 25 application tests.
- `cargo clippy -p builder3d-viewport -p builder3d-app --all-targets --all-features --locked -- -D warnings`: pass.

## Workspace gates

- `cargo fmt --all -- --check`: pass.
- `cargo check --workspace --locked`: pass.
- `cargo test --workspace --locked`: pass, 144 tests plus zero-test doctest
  runs.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`: pass.
- `cargo deny check`: pass for advisories, bans, licenses, and sources; the
  existing duplicate-version diagnostics remain warnings.
- `cargo metadata --locked --format-version 1 --no-deps`: pass.
- Catalog TOML/path validation: pass, 15 unique IDs and all referenced paths
  present.
- Native smoke: pass, `Builder3D shell smoke: objects=2 selected=1 tabs=5`.

No commit was created. Python source and existing snapshots were not written.

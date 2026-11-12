# R7-12 verification

## Focused behavior

- `cargo test -p builder3d-platform --locked`: pass, 15 platform tests.
- Tests cover Windows and XDG path policy, descriptor override precedence,
  current-process discovery, invalid process IDs, and case-insensitive
  offscreen rejection.

## Workspace gates

The final locked workspace gate results and recovery-snapshot counts are
recorded here after the full R7-12 verification run.

- `cargo fmt --all -- --check`: pass.
- `cargo check --workspace --locked`: pass.
- `cargo test --workspace --locked`: pass, 169 tests plus zero-test doctest
  runs.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`: pass.
- `cargo deny check`: pass for advisories, bans, licenses, and sources; the
  existing duplicate-version diagnostics remain warnings.
- `cargo metadata --locked --format-version 1 --no-deps`: pass.
- Catalog TOML/path validation: pass, 24 unique IDs and 156 referenced paths.
- Native smoke: pass, `Builder3D shell smoke: objects=2 selected=1 tabs=5`.

No commit was created. Python source and existing snapshots were not written.

# R7-13 verification

## Focused behavior

- `cargo test -p modeler3d-app --locked`: pass; 51 application tests cover
  typed signals, numeric suffix selection, preference persistence/validation,
  and the existing native shell/dialog contracts.
- `cargo test -p modeler3d-mcp --locked`: pass; R4 server, security, replay,
  and operation lifecycle tests remain green.

## Workspace gates

The final locked workspace gate results and recovery-snapshot counts are
recorded here after the full R7-13 verification run.

- `cargo fmt --all -- --check`: pass.
- `cargo check --workspace --locked`: pass.
- `cargo test --workspace --locked`: pass, 173 tests plus zero-test doctest
  runs.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`: pass.
- `cargo deny check`: pass for advisories, bans, licenses, and sources; the
  existing duplicate-version diagnostics remain warnings.
- `cargo metadata --locked --format-version 1 --no-deps`: pass.
- Catalog TOML/path and coverage audit: pass, 29 IDs; all 41 product Python
  modules and all 19 reference/support scripts are mapped, with 197 referenced
  Rust/test/evidence paths. No `planned` or `polish` catalog statuses remain.
- Native smoke: pass, `3D Modeler shell smoke: objects=2 selected=1 tabs=5`.
- `cargo audit`: skipped because the command is not installed in this
  environment.

No commit was created. Python source and existing snapshots were not written.

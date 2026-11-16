# R7-02 verification

## Focused behavior

- `cargo test -p modeler3d-app selection_commands_cover_group_cut_and_non_history_selection --locked`: pass.
- The focused test verifies two selected primitives can be grouped and
  ungrouped, selection changes do not advance the scene revision, cut removes
  the selected objects, and paste restores them.
- Native smoke: `3D Modeler shell smoke: objects=2 selected=1 tabs=5`.

## Workspace gates

- `cargo fmt --all -- --check`: pass.
- `cargo check --workspace --locked`: pass.
- `cargo test --workspace --locked`: pass, 137 tests plus zero-test doctest
  runs.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`: pass.
- `cargo deny check`: pass for advisories, bans, licenses, and sources. The
  existing duplicate-version diagnostics remain warnings.
- `cargo metadata --locked --format-version 1 --no-deps`: pass.
- The native application smoke command passed using the `modeler3d-app` binary.
- Catalog TOML/path validation passed with 13 unique IDs and all referenced
  Rust, test, and evidence paths present.

No commit was created. R1–R6 source and evidence were not changed, and no
Python source was written.

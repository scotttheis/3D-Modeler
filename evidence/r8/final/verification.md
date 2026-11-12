# R8 final verification

> SCOPE CORRECTION 2026-10-30: historical automated results below are retained.
> Native launch/capture did not establish interactive or aesthetic parity.
> R8 product acceptance is withdrawn; see
> [the replacement plan](../../../docs/product-rebuild-plan.md). The snapshot
> below excluded evidence/work log and is not the new recovery standard.

All commands were run from `E:\Builder3D-Rust` on 2026-10-30.

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo check --workspace --locked` | PASS |
| `cargo test --workspace --locked` | PASS — all workspace tests and doctests |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | PASS |
| `cargo deny check` | PASS — existing duplicate-version warnings only; advisories/bans/licenses/sources pass |
| `cargo metadata --locked --format-version 1 --no-deps` | PASS |
| `cargo run -p builder3d-app --bin builder3d-app -- --smoke` | PASS — `objects=2 selected=1 tabs=5` |
| Catalog/path/coverage audit | PASS — 30 unique IDs; 41/41 product modules; 19/19 support scripts; all references exist |
| `cargo audit --locked` | UNAVAILABLE — `cargo-audit` is not installed |
| `cargo bloat --release --workspace` | PASS — command completed; tool warned that `--workspace` is unused and analyzed the release target selected by cargo-bloat |
| `cargo bloat --release -p builder3d-app --bin builder3d-app` | PASS — release app analyzed; `.text` 10.7 MiB, file 13.2 MiB |
| Native manual launch | PASS — final welcome/status capture; scene/build-bed/object-panel capture in R8 slice evidence |
| Python source and existing snapshots | unchanged |
| Git commit | none created |

Final recovery snapshot verification:

- Path: `G:\Builder3D-Rust_snapshot_20261030-123828_R8-final`.
- Copy result: robocopy exit 1 (success range).
- Source/snapshot totals: 169 files and 1,730,290 bytes each.
- Exclusions: `target`, `evidence`, and `WORK_LOG.md`.

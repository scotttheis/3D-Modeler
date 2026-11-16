# RB-08 Verification

## Automated verification

- Python reference focus: **12 passed, 0 failed**. The run covered settings
  migration, slicer preference and chooser, theme persistence, favorites,
  shortcuts with text focus, and narrow/wide ribbon layouts. It used the
  read-only `L:\Builder3D` venv and stored pytest temp files under this RB-08
  evidence directory.
- Rust focused tests: **178 passed, 0 failed** (`modeler3d-app` 159,
  `modeler3d-platform` 19).
- Rust workspace tests: **314 passed, 0 failed**, including doc-test targets.

One initial focused Rust command did not initialize the project’s Windows
libclang path and failed in `qhull-sys` before test execution. That attempt is
preserved as `verification/cargo-test-focused-missing-libclang-failure.txt`.
The project’s `scripts/Initialize-BuildClang.ps1` was then run in the same
PowerShell session; the focused test, workspace test, check, Clippy, smoke and
release build all completed successfully.

## Ten documented Rust gates

| Gate | Result | Evidence |
|---|---|---|
| `cargo fmt --all -- --check` | Pass | `verification/cargo-fmt.txt` |
| `cargo check --workspace --locked` | Pass | `verification/cargo-check-workspace-final.txt` |
| `cargo test --workspace --locked` | Pass, 314 tests | `verification/cargo-test-workspace-final.txt` |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | Pass | `verification/cargo-clippy-final.txt` |
| `cargo deny check` | Pass: advisories, bans, licenses and sources | `verification/cargo-deny-final.txt` |
| `cargo metadata --locked --format-version 1 --no-deps` | Pass; 5 packages and 5 workspace members; no stderr | `verification/cargo-metadata-final.json` |
| `cargo run --locked -p modeler3d-app --bin modeler3d-app -- --smoke` | Pass; 2 objects, 1 selected, 5 tabs | `verification/cargo-smoke-final.txt` |
| `cargo build --release --locked -p modeler3d-app --bin modeler3d-app` | Pass | `verification/cargo-build-release-final.txt` |
| `cargo bloat --release --workspace` | Exit 0; see scope note below | `verification/cargo-bloat-workspace.txt` |
| `cargo bloat --release -p modeler3d-app --bin modeler3d-app` | Pass; 12.2 MiB `.text`, 14.9 MiB executable report | `verification/cargo-bloat-app.txt` |

The installed `cargo-bloat` is 0.12.1 and does not implement `--workspace`;
the command warns that it ignores that argument and analyzes the default
`modeler3d-app.exe`. The explicit app-target size command also passed. The final
release file is **15,653,888 bytes** and its SHA-256 is
`db406708cf19efe9ed62c3c53d02684dc49c4458d44c59bd940e6bc45ca81f75`, below the
40 MiB application target. `cargo audit --locked` was not run because
`cargo-audit` is not installed; this is recorded in
`verification/tool-availability.txt`.

`cargo deny` passes with three exact informational unmaintained-advisory IDs
documented in `deny.toml` for the optional AccessKit Unix AT-SPI dependency
chain. The Windows application uses AccessKit’s Windows provider. No advisory
was suppressed by a broad crate or wildcard rule.

## Native checks

The initial Rust release process imported the reference registry settings into
the isolated JSON runtime. Its accessibility tree exposed 38 descendants, 36
with names. The first visual capture was rejected because Windows was at the
lock screen; it is diagnostic only. The paired native interaction matrix is
pending desktop unlock and will be recorded here before RB-08 is closed.

# RB-03 verification record

Recorded 2026-10-31 from `E:\Builder3D-Rust`. Command output is in `logs/`; the latest integrated run used the post-grid-fix source.

## Integrated Rust workspace gates

| Command | Result | Notes |
|---|---|---|
| `cargo fmt --all -- --check` | Pass, exit 0 | `logs/cargo-fmt-integrated-final.log`. |
| `cargo check --workspace --locked` | Pass, exit 0 | All five workspace crates checked. |
| `cargo test --workspace --locked` | Pass: 228 passed, 0 failed across 27 test targets | Includes the recent-card grid regression. |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | Pass, exit 0 | Strict warnings denied. |
| `cargo deny check` | Pass, exit 0 | Non-fatal duplicate `bitflags` lock-entry warning (1.3.2 and 2.13.2). |
| `cargo metadata --locked --format-version 1 --no-deps` | Pass, exit 0 | JSON parses; five packages and five workspace members. |
| `cargo run -p builder3d-app --bin builder3d-app -- --smoke` | Pass, exit 0 | `objects=2 selected=1 tabs=5`. |
| `cargo build --release --locked -p builder3d-app --bin builder3d-app` | Pass, exit 0 | Release app built; binary 14,386,688 bytes, SHA-256 `de547662b3fca993d631b7ef931aaabd30c28f4f17dbfa629db4adc19949a999`. |
| `cargo bloat --release --workspace` | Pass, exit 0, with CLI warning | `--workspace` is reported unused; the command analyzed `r4_mcp.exe` (`.text` 3.8 MiB, file 4.5 MiB). |
| `cargo bloat --release -p builder3d-app --bin builder3d-app` | Pass, exit 0 | App `.text` 11.2 MiB, file 13.7 MiB. |
| `cargo audit --locked` | Not available | Cargo returned “no such command: audit” (exit 101); it was not installed. |

The installed `cargo-bloat` CLI's `--workspace` behavior is recorded as observed; the app-specific scan measured the Builder3D executable directly.

## Focused Rust checks

- Recent-grid layout regression: one focused test passed (`logs/recent-grid-focused-test.log`).
- `cargo test --offline --locked -p builder3d-app`: 92 passed, 0 failed (`logs/recent-grid-app-tests.log`).
- Changed app crate strict Clippy and the isolated debug rebuild passed (`logs/recent-grid-clippy.log`, `logs/recent-grid-build.log`).
- Independent native export verification passed for all seven formats, selection/visibility cases, overwrite cancellation, transformed bounds, supported colors/names, and WRL indexed geometry (`logs/native-export-verification.log`).
- The Rust native walkthrough used `target/rb03-recents-build/debug/builder3d-app.exe`, SHA-256 `dec750d4a6ef65a1a038fd5221f5b941b9799789ec21ec31600becd18d1cd8b1`.

## Read-only Python reference checks

The reference tests used `L:\Builder3D\venv` Python 3.12.10 with bytecode/cache disabled and temp files under ignored Rust `target/`:

| Command group | Result |
|---|---|
| Project format, 3MF, glTF, and WRL focused suites | 35 passed. |
| Unitless-format selected-unit test | 3 passed, 80 deselected. |
| Python source status after the UI walkthrough | Exactly matches the pre-capture baseline (116 lines); no Python source change. |
| Original QSettings after walkthrough | Restored: Light theme; no recent-files key; no default-import-unit key. |

The broad Python desktop suite and its offscreen OpenGL widget were not run.

## Visual and interactive evidence

The Python-reference and Rust app captures were taken sequentially at 1280 × 840 client pixels, 96 DPI / 100% display scale. The old app was visibly operated for lifecycle prompts, Save As, model picker, mm/in imports, 3MF/glTF/GLB imports, malformed input, 3MF export, overwrite decisions, recents, missing/corrupt/WRL recent errors, and Light/Dark welcome states. Rust native flows covered every import/export format and required selection/cancel/recent paths. The observed differences and state-matching caveats are in `visual-comparison.html` and `visual-comparison.md`.

The Python native Export Mesh picker was captured with 3MF selected and completed one 3MF export. A second attempt to capture a separate old-app picker interaction for each output format could not proceed after Windows locked the desktop (`GetForegroundWindow` returned 0); no locked-desktop input or capture was claimed. The Rust native seven-format journey and independent output verification passed.

All 32 RB-03 ledger rows have implementation and automated status `pass`. Native interactive status is `pass` for 30 rows and `partial` for the live saved-checkpoint history marker and full filesystem-failure audit. Visual review and owner acceptance remain `pending` until the owner reviews the comparison board. No visual pass is claimed from unreviewed captures.

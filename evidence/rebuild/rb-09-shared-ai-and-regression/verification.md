# RB-09 Verification

**Scope:** paired live Python/Rust desktop and MCP verification, scoped UI-sync repair, regression gates, and evidence validation. The verified host was Windows 11 Enterprise build 10.0.26200, 2560x1440 at 96 DPI.

## Post-repair engineering gates

All commands below ran from the Rust project root after the presentation-sync repair. Exit codes and full stdout/stderr are retained under logs/postfix-*.

| Command | Result | Evidence |
|---|---|---|
| cargo fmt --all -- --check | Pass, exit 0 | logs/postfix-cargo-fmt.* |
| cargo test -p builder3d-app mcp_ | 7 passed, 0 failed | logs/postfix-focused-app-mcp.* |
| cargo test --workspace --locked | 329 passed, 0 failed, 0 ignored | logs/postfix-workspace-test.* |
| cargo clippy --all-targets -- -D warnings | Pass, exit 0 | logs/postfix-cargo-clippy.* |
| cargo build --release -p builder3d-app --locked | Pass, exit 0 | logs/postfix-release-app.* |
| cargo build --release -p builder3d-mcp --bin r4_mcp --locked | Pass, exit 0 | logs/postfix-release-mcp.* |

The workspace count is the prior 327-test result plus the two new UI-presentation regression tests. The 19 focused MCP tests, 3 core external-commit tests, 12 focused Python reference tests, and earlier dependency/metadata/smoke/bloat gates are retained in their original logs. cargo deny passed with 28 duplicate-version warnings and no advisory, ban, license, or source-policy failure. cargo audit was unavailable. cargo bloat --workspace emitted the installed tool’s unused --workspace warning; the explicit app-target report is the usable size measurement.

## Live desktop and MCP

The official Python MCP SDK 2.2.0 used stdio clients for each app and enumerated all 60 tools. Host-desktop probes confirmed the current Windows session was active and interactive before UI input and capture; LockApp did not own the session during the live runs. The owner’s Edge video remained in the background and is not evidence. No Windows power or lock settings changed.

| Requirement | Outcome | Evidence |
|---|---|---|
| Light, Dark, and alternate-size scene states | Pass. Python default captures are 1296x859; Rust final-build defaults are 1296x879; both at 96 DPI. Python Light and Rust Dark alternate captures are 1600x900. | capture-metadata.json; paired-comparison-board.md |
| Live MCP reads/edits and immediate GUI presentation | Pass for Python and repaired Rust. Rust MCP Sphere color change at revision 34 appeared in viewport and Items. | logs/rb09-python-client.jsonl; logs/rb09-rust-client.jsonl; captures/rust-post-fix-mcp-edit.png |
| Shared undo/redo, stable IDs, revisions, no-op | Pass. Final Rust build GUI and MCP history shared the same object ID; no-op transform did not advance revision. | logs/rb09-rust-client-2.jsonl; logs/rb09-rust-client-final-pair.jsonl; captures/rust-post-fix-final-history-*.png |
| MCP save, GUI close/reopen, New/Open, dirty guard | Pass. Reopen restored the same Rust project and four base IDs; dirty New cancellation preserved the live document. | logs/rb09-python-client.jsonl; logs/rb09-rust-client.jsonl; captures/rust-post-fix-gui-reopened.png; captures/rust-post-fix-dirty-new.png |
| Approved/denied files and failed operation atomicity | Pass. Denied, malformed, unsupported, and overwrite operations failed without changing document identity, path, revision, dirty state, history, or sentinel file. | logs/rb09-python-client.jsonl; logs/rb09-rust-client.jsonl; fixture-manifest.json |
| Idempotency, revision conflicts, competing clients | Pass in Python and Rust; identical replay returned the original ID, mismatched replay arguments were rejected, and each competing-client race produced one commit and one revision conflict. | logs/rb09-python-client.jsonl; logs/rb09-python-client-2.jsonl; logs/rb09-rust-client-2.jsonl |
| Real long-operation status/progress/cancel/failure/stale result | Pass. Canceled results and bounded-resource failures did not mutate scene/history; stale results were discarded after a concurrent edit. | client JSONL files; fixture-manifest.json; captures/python-light-stale-edit.png; captures/rust-post-fix-stale-edit.png |
| Optional MCP, closed desktop, bridge cleanup | Pass. GUI remained usable with MCP startup blocked; closed clients received desktop_unavailable; process, listener, and descriptor cleanup passed after normal and failed startup shutdown. | logs/rb09-python-optional-client.jsonl; logs/rb09-rust-optional-client.jsonl; captures/python-mcp-startup-failed.png; captures/rust-optional-mcp-unavailable.png; client-server-log.md |

## Release artifacts

The final Rust source and binaries are identified in build-manifest.json. The final app SHA-256 is d680cf0ba8e4c8bf7d5ccd1536bf7d5720987d676a1a0946c6c3505b613b0cb9. The live Rust MCP process used SHA-256 a1dba8751c5a15cafbf6e26fac8db0b8a96f8464bf750ac62336f573b729545b; the post-run Release rebuild passed with SHA-256 9b8817015ab5c8195013ee788fb095e9393848be74c7da0221ebd33342fc99d3. The post-run binary then passed an official SDK 60-tool and closed-desktop probe recorded in rust_stdio_client_probe-result.json.

## Wrap-up recheck (2026-11-04)

The following commands were rerun from `E:\Builder3D-Rust`, after sourcing
`scripts/Initialize-BuildClang.ps1` in the same PowerShell session. Exit 0
means the command passed. Full wrap-up stdout/stderr is in ignored
`target/rb09-wrapup-gates/` and is not part of the recovery snapshot.

| Command | Result and key output |
|---|---|
| `cargo fmt --all -- --check` | Pass, exit 0 |
| `cargo check --workspace --locked` | Pass, exit 0 |
| `cargo test --workspace --locked` | Pass, 329 passed, 0 failed, 0 ignored across 28 test-result records |
| `cargo clippy --all-targets -- -D warnings` | Pass, exit 0 |
| `cargo deny check` | Pass, exit 0; 28 duplicate-version warnings, no policy errors |
| `cargo metadata --locked --format-version 1 --no-deps` | Pass, exit 0 |
| `cargo test -p builder3d-mcp --locked` | Pass, 19 passed, 0 failed, 0 ignored |
| `cargo run -p builder3d-app --bin builder3d-app -- --smoke` | Pass, `Builder3D shell smoke: objects=2 selected=1 tabs=5` |

The earlier locked Release app and MCP builds passed in `logs/postfix-release-*`.
`cargo bloat --release -p builder3d-app --bin builder3d-app` reported 12.7 MiB
`.text` and a 15.4 MiB file. `cargo bloat --release --workspace` exited 0 but
warned `unused arguments left: ["--workspace"]` and analyzed the default
`r5_viewport.exe`; it is not a workspace-size measurement. `cargo audit` was
unavailable because `cargo-audit` is not installed.

The gate-time evidence validator passed with exit 0 in
`logs/evidence-ledger-validation.exit`; it verified 50 PNG hashes/dimensions,
two clean scene pairs, 10 fixture hashes, the 96-file source manifest, and the
structured evidence. A wrap-up rerun of
`python evidence/rebuild/rb-09-shared-ai-and-regression/validate_rb09_evidence.py`
exited 1 with `release binary hash mismatch: live Rust app`. The recorded live
app hash is
`d680cf0ba8e4c8bf7d5ccd1536bf7d5720987d676a1a0946c6c3505b613b0cb9`; current
`target/release/builder3d-app.exe` is
`0bc4f1a0fd0327e51ecd8acdb75a01120b3c53325b11738a7b3fb6ce198cc75a`. The
current MCP executable hash is
`aff0af548a3f8055e2ebe1d218135a774cc9b88f210a94f19627a9f9b22e1c1e`; it is
different from both the live gateway hash and the separately recorded post-run
build hash. No Rust source changed after the live run. The mismatch is a failed
current-artifact identity check; this report does not present either rebuilt
executable as the live-tested binary.

An initial wrap-up PowerShell wrapper stopped when it treated Cargo's successful
`Finished` stderr line as a terminating `NativeCommandError`. The gates were
rerun with native stderr handling corrected; all eight commands above exited 0.

## Reference protection and status boundary

L:/Builder3D source files were not written. Its QSettings were restored and read back exactly: object_snapping_enabled=true, ribbon_favorites=[], theme_mode=dark; the added recent_files setting from the live run was removed. The initial Python reference Git status is retained in logs/python-reference-git-status.txt and the behavior-critical source hash is unchanged.

The evidence validator checks TOML/JSON/NDJSON, live interactive status, pair membership, screenshot hashes, and evidence paths. Its final output is logs/evidence-ledger-validation.log.

## Additive recovery snapshot

G:/Builder3D-Rust_snapshot_20261104-182021_RB-09-complete was created only after the paired evidence and ledgers passed validation. The final independent snapshot check passed with zero missing, extra, SHA/size-different, or reparse entries. Exact included file/byte counts and root exclusions are in snapshot-receipt.md.

Owner interface/floor gaps, visual review, owner acceptance, and the RB-08 native matrix/completion snapshot remain pending. RB-10 has not started.

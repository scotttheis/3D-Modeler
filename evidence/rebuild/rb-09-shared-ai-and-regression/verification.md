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
structured evidence. A later wrap-up run of
`python evidence/rebuild/rb-09-shared-ai-and-regression/validate_rb09_evidence.py`
exited 1 with `release binary hash mismatch: live Rust app`. That validator
compared current target paths to historical full-gate records.

The recorded full-gate app hash is
`d680cf0ba8e4c8bf7d5ccd1536bf7d5720987d676a1a0946c6c3505b613b0cb9`; the
paired live gateway hash is
`a1dba8751c5a15cafbf6e26fac8db0b8a96f8464bf750ac62336f573b729545b`. The
post-run gateway hash is
`9b8817015ab5c8195013ee788fb095e9393848be74c7da0221ebd33342fc99d3`. The
workspace bloat log shows later recompiles of builder3d-mcp and builder3d-app;
the app restore log shows another completed app Release build. Copies are
retained in `logs/identity-recheck-bloat-workspace.stderr.log` and
`logs/identity-recheck-restore-release-app.stderr.log`. These builds explain
why the current paths no longer contain the historical binaries. No exact
copy of the old executables was found under `target/release` or the RB-09
evidence directory; none was restored from a G: snapshot.

The current Release app hash is
`0bc4f1a0fd0327e51ecd8acdb75a01120b3c53325b11738a7b3fb6ce198cc75a`; the
gateway hash is
`aff0af548a3f8055e2ebe1d218135a774cc9b88f210a94f19627a9f9b22e1c1e`. Both
hashes were unchanged before and after the identity-only live run. Host probes
confirmed the Default desktop was active and Builder3D was foreground before
the screenshot and Alt+F4 close. The official Python MCP SDK 2.2.0 listed all
60 tools; read-only `get_status` succeeded with `app_state=ready`,
`dirty=false`, and revision 0. The app, gateway, and runtime descriptor were
absent after shutdown. The 1296x879 capture is labelled identity-only and is
not part of the accepted 50-capture inventory. Details and hashes are in
`artifact-identity-recheck.json`, `artifact-identity-recheck/current-app.png`,
and `artifact-identity-recheck/client.jsonl`.

The updated validator keeps historical build identity tied to
`capture-metadata.json` and verifies current target identity against the
recheck record. It also checks the two rebuild logs, screenshot, SDK event log,
60-tool list, and successful read-only status result. The validator now passes;
stdout, stderr, and exit are retained in
`logs/identity-recheck-validator.*`. The source manifest remains 96 Rust files
with SHA-256
`9846a4a6dee484186283e169a9b7cdbbf74007bdca772efa5981f438ce78b457`. No Rust
application source changed. No Rust regression gates were rerun because the
source tree is unchanged.

An initial wrap-up PowerShell wrapper stopped when it treated Cargo's successful
`Finished` stderr line as a terminating `NativeCommandError`. The gates were
rerun with native stderr handling corrected; all eight commands above exited 0.

## Reference protection and status boundary

L:/Builder3D source files were not written. Its QSettings were restored and read back exactly: object_snapping_enabled=true, ribbon_favorites=[], theme_mode=dark; the added recent_files setting from the live run was removed. The initial Python reference Git status is retained in logs/python-reference-git-status.txt and the behavior-critical source hash is unchanged.

The evidence validator checks TOML/JSON/NDJSON, historical build identity,
current Release identity, live interactive status, pair membership, screenshot
hashes, rebuild logs, official-client events, and evidence paths. The original
gate output is logs/evidence-ledger-validation.log; the current validator run
is in logs/identity-recheck-validator.stdout.log and
logs/identity-recheck-validator.exit.

## Additive recovery snapshot

G:/Builder3D-Rust_snapshot_20261104-182021_RB-09-complete passed its
independent check when created after the original paired evidence. It predates
the later target rebuilds. The latest G: review snapshot is
G:/Builder3D-Rust_snapshot_20261104-193530_RB-09-wrapup-review; it is not a
completion snapshot. The identity follow-up review snapshot is
G:/Builder3D-Rust_snapshot_20261104-215904_RB-09-artifact-identity-review.
Robocopy exited 1 without failures, and independent verification passed with
3,904 files / 701,101,053 bytes on both sides and zero missing, extra,
different, or reparse entries. The direct G: path exceeded Windows PowerShell
path resolution for one nested shader-cache item, so the same verifier was run
through temporary drive Q: and the mapping was removed afterward. Root target,
.cache, and logs were excluded; nested evidence logs and Git metadata were
included. This remains a review copy, not a completion snapshot; exact details
are in snapshot-receipt.md.

Owner interface/floor gaps, visual review, owner acceptance, and the RB-08 native matrix/completion snapshot remain pending. RB-10 has not started.

## Current Rust Release recheck (2026-11-06)

- Current app SHA-256: `0bc4f1a0fd0327e51ecd8acdb75a01120b3c53325b11738a7b3fb6ce198cc75a`.
- Current gateway SHA-256: `aff0af548a3f8055e2ebe1d218135a774cc9b88f210a94f19627a9f9b22e1c1e`.
- Current official SDK client listed the ordered 60-tool catalog and exercised the Rust desktop/MCP workflow against the current app. Observe/model/files/full profile counts were 7/52/59/60 and ordered names matched saved Python catalogs.
- Saved project SHA-256: `e6046a8d3ed7ea949b00a13928311b2fc88e476bda6d4d859af6d866246d9262`; project `e5d81c5226e64f58b838d55d9395afa9`, revision 25, clean. The Rust GUI reopened it and showed the saved Cube and Sphere.
- Optional startup failure left the GUI interactive. Client calls to `get_status` and `scene_summary` returned retryable `desktop_unavailable`; no runtime descriptor or app-owned listener remained after shutdown.
- `schema-comparison.json` compares current Rust live catalogs and responses with saved Python catalog snapshots. Exact input/output schema matches are 0 in all four profiles. It records the input-field, required-field, additional-properties, annotation, output-schema, and top-level result differences. The Python product was not launched and the saved catalogs were not refreshed.
- The comparison checks canonical schema equality and only top-level required/additional-properties constraints on sample successful output objects; it is not a complete nested JSON Schema validation.

The current run did not change Rust application source, so no Rust test suite was rerun. Previously recorded Rust tests remain historical: workspace 329/329 and focused MCP 19/19. The focused command py -3.13 evidence/rebuild/rb-09-shared-ai-and-regression/validate_rb09_evidence.py --tool-name-self-test passed with exit 0; the full validator passed with exit 0 and reported schema parity pending while validating the nine current-run capture records. The manifest writer, schema comparator, and git diff --check also exited 0. git diff --check emitted only line-ending normalization warnings.

## Current additive recovery snapshot

The new G:/B3D_R09C_20261106 snapshot was synchronized after the final evidence and docs updates. The independent verifier reports 3991 files / 756841099 bytes for source and snapshot, with zero missing, extra, differing, or reparse entries. Critical file SHA-256 values match. Only root target, .cache, and logs were excluded; nested evidence logs and Git metadata were copied. See snapshot-receipt.md.

## RB-09 wrap-up rerun (2026-11-06)

After the current Release recheck above, the wrap-up reran `cargo fmt --check`,
`cargo check --workspace --locked`, `cargo test --workspace --locked --quiet`
(329 passed, 0 failed, 0 ignored), `cargo test -p builder3d-mcp --locked`
(19 passed, 0 failed, 0 ignored), `cargo clippy --all-targets -- -D warnings`,
and `cargo deny check`; all exited 0. Cargo Deny reported 28 duplicate-version
warnings and no advisory, ban, license, or source-policy failures. `cargo audit`
was unavailable because `cargo-audit` is not installed; cargo bloat and full
installed-package size were not rerun.

The current-run manifest and schema-comparison generator commands exited 0.
Profile counts and ordered names matched (7/52/59/60), while exact input- and
output-schema matches remained zero in every profile. The focused 60-name
self-test and full evidence validator passed; the latter reports schema parity
pending. The Rust application source was not changed.
## Current-reference schema-parity repair (2026-11-07)

The current read-only Python reference identity and tools/list captures are in
reference-current-20261107-084248/. Rust matches all input/output JSON Schemas
and official annotations recursively across four profiles. The schema
contract performs complete Draft 2020-12 validation for every tool.

Focused MCP tests passed 26/26; the captured current Release result-validation
test passed on 25 successful structured results. Workspace tests passed
336/336 and current-reference MCP tests passed 9/9. Format, workspace check,
Clippy with warnings denied, Cargo metadata, Cargo Deny, and explicit app
Release bloat passed. The app text section measured 16.0 MiB and the executable
19.6 MiB. cargo-audit was unavailable because it is not installed. Exact
commands and logs are indexed in schema-parity-run-manifest.json.

The rebuilt Release app served GUI/MCP calls, including new_project and
completed operation_status; the nested unknown-action probe returned
invalid_input. The app closed normally and its runtime descriptor was removed.
One custom emboss_text _meta.com.builder3d/longRunning difference remains
disclosed without owner approval. Full RB-09 acceptance, RB-08 native evidence,
interface/floor gaps, visual review, owner acceptance, and overall product
acceptance remain pending. RB-10 has not started.
## Recovery snapshots for the schema-parity repair

The pre-edit G: snapshot was independently verified before Rust source changes
at 3,743 files / 755,179,156 bytes, with zero path/hash/reparse differences and
17 critical SHA-256 matches. After its final synchronization, the additional
G:\B3D_R09S_20261107 copy passed independent full-tree verification at 4,079
files / 766,761,893 bytes on both sides, with zero missing/extra/different
paths, zero reparse entries, and no SHA-256 differences. The initial copy had
23 separately recorded critical SHA-256 matches. The full receipt and hash
records are under snapshot-receipt.md.
G:\B3D_R09W_20261106 and all older snapshots were left untouched.

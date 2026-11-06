# RB-09 Shared AI and Regression Report

**Date:** 2026-11-06
**Status:** The historical Python/Rust paired gate passed on its recorded build. A current Rust Release GUI/MCP recheck and optional-MCP failure check also passed on 2026-11-06. The current schema comparison found unresolved catalog differences against saved Python catalogs, so RB-09 schema compatibility and overall acceptance remain pending. Stop before RB-10.

## Historical paired gate evidence

- The official Python MCP SDK 2.2.0 connected to both the running Python reference app and the Rust stdio gateway. Each live app exposed the fixed 60-tool catalog and edited its current desktop document through the app-owned bridge.
- Python and Rust used project ID e5d81c5226e64f58b838d55d9395afa9 and stable Cube/Sphere IDs 3f449b78621c467b818e8c69925e2b6c and dff9cc124c6d4d27b4ea9b07d6e4ae6b. Geometry and transforms match. The initial Python capture is revision 4 with the Cube blue; the later saved shared fixture is revision 23 with the Cube gold after a Python MCP edit. Rust’s final-build paired capture opens that revision-23 file. This color/revision difference is recorded in the comparison board.
- Rust MCP New/Open from Welcome exposed the Scene immediately after a scoped repair. MCP edits then appeared in both the viewport and Items panel without restart.
- Shared GUI/MCP history, stable IDs, revision and no-op behavior, save/reopen, New/Open, dirty protection, approved-root and file-failure behavior, replay, revision races, long-operation progress/cancel/failure/stale-result handling, optional MCP startup, closed-desktop responses, and bridge cleanup were exercised with real clients and visible app windows.
- An initial Rust Welcome-to-Scene visibility defect was found, repaired, and retested. Two focused regression tests cover an MCP edit from Welcome and replacing the document with an empty project.

## Historical live gate outcomes

| Area | Observed result |
|---|---|
| Paired scenes and display | Python Light/Dark captures: 1296x859 px; Rust final-build Light/Dark captures: 1296x879 px. Both were captured on the 2560x1440 host at 96 DPI. Alternate 1600x900 captures were also recorded. |
| Live document | Both clients read project e5…fa9. Cube and Sphere IDs and geometry were stable across Python and Rust. Rust’s post-repair MCP edit changed the Sphere color at revision 34 and updated viewport and Items immediately. |
| History and no-op | Python no-op transform stayed at revision 4; undo/redo exercised revisions 5→6→7. Final Rust build added object 42b03db2996a4e79bdac9fba29af4033 at revision 35; GUI undo/redo reached revisions 36/37 and preserved that ID; MCP undo/redo reached 38/39; undo restored the clean four-object baseline at 40. |
| Lifecycle | Python MCP Save, New, Open, and GUI dirty-guard cancellation preserved project identity and state. Rust saved RB09-Rust-Live.3dp at revision 34, GUI close/reopen restored the same project and four IDs, MCP New created e6b3ff7558cf465198ddb2c4d410ebb0 at revision 0, and MCP Open restored e5…fa9 at revision 34. Dirty New cancellation preserved the document. |
| File policy and atomicity | Both apps rejected denied paths, malformed projects, unsupported imports/exports, and overwrite attempts. Python failures preserved e5…fa9 revision 15; Rust failures preserved e5…fa9 revision 34. The overwrite sentinel stayed SHA-256 b14dc62457f04cec9b732d6b2bab2bfd3a5827b4fed640f70c61516e1443f72f. |
| Replay and concurrency | Same idempotency key replayed the original object/result; conflicting arguments were rejected. Stale expected revisions returned revision_conflict. Competing clients produced one commit and one conflict in each app. |
| Long operations | Python canceled a 600k-face STL import at progress 85 with revision 15 unchanged; completed 50k imports were undoable; a stale result was discarded at revision 22 with revision_conflict. Rust reported progress, canceled normalize at progress 90→95 with revision 45 unchanged, rejected bounded-resource failures, and discarded a stale normalize result at revision 46 after a GUI edit. |
| Optional MCP and cleanup | Python and Rust stayed usable when MCP startup failed. Official clients received structured desktop_unavailable after desktop closure. Rust normal and optional-failure shutdown removed the descriptor, listener, client, gateway, and app processes. |

Exact operation IDs, tool results, paths, and revision observations are retained in the client JSONL files and walkthrough.

## Repair and regression verification

The first Rust live launch installed/opened the project over MCP from Welcome but left the visible page on Welcome. The scoped fix synchronizes shell presentation after MCP document installation and successful scene commits, and resets to Scene after project replacement. The final Release build passed the focused app MCP tests (7/7), full workspace tests (329/329), formatting, Clippy, and Release app/MCP builds. Gate logs are under logs/postfix-*.

## Build and capture identity

- Rust source manifest: 96 Rust files; SHA-256 9846a4a6dee484186283e169a9b7cdbbf74007bdca772efa5981f438ce78b457, using sorted project-relative paths and each file’s SHA-256 digest.
- Final Release app: target/release/builder3d-app.exe, 16,172,032 bytes, SHA-256 d680cf0ba8e4c8bf7d5ccd1536bf7d5720987d676a1a0946c6c3505b613b0cb9.
- Live Rust MCP gateway used for the final paired run: target/release/r4_mcp.exe, SHA-256 a1dba8751c5a15cafbf6e26fac8db0b8a96f8464bf750ac62336f573b729545b. A post-run locked Release rebuild passed and produced SHA-256 9b8817015ab5c8195013ee788fb095e9393848be74c7da0221ebd33342fc99d3; both were built from the same recorded source manifest.
- Python reference identity: L:/Builder3D, Git HEAD 3a0160a95c55f68ce9f1e89d17c792816ac001df, active entrypoint source hash recorded in build-manifest.json, Python 3.12.10, MCP SDK 2.2.0. Its saved QSettings were restored to the values in logs/python-settings-before.json.
- Every PNG in captures/ has its SHA-256, byte length, image dimensions, host DPI, theme/context, and evidence role in capture-metadata.json. The settings-dialog image rust-final-paired-dark-initial.png is explicitly excluded as a clean scene capture; rust-final-paired-dark-scene.png is the accepted Dark scene.

## Wrap-up artifact identity recheck

The recorded full-gate app hash is `d680cf0ba8e4c8bf7d5ccd1536bf7d5720987d676a1a0946c6c3505b613b0cb9`; its paired live gateway hash is `a1dba8751c5a15cafbf6e26fac8db0b8a96f8464bf750ac62336f573b729545b`. The post-run gateway hash was `9b8817015ab5c8195013ee788fb095e9393848be74c7da0221ebd33342fc99d3`. The gate-time evidence validator passed with exit 0 and its output remains in `logs/evidence-ledger-validation.log` and `.exit`.

The later target-path mismatch came from subsequent Release builds. The recorded `cargo bloat --release --workspace` run warned that `--workspace` was unused and recompiled `builder3d-mcp` and `builder3d-app`; the app restore log records another completed Release build. Copies of both original stderr logs are retained as `logs/identity-recheck-bloat-workspace.stderr.log` and `logs/identity-recheck-restore-release-app.stderr.log`. The current target app hash is `0bc4f1a0fd0327e51ecd8acdb75a01120b3c53325b11738a7b3fb6ce198cc75a`; the current gateway hash is `aff0af548a3f8055e2ebe1d218135a774cc9b88f210a94f19627a9f9b22e1c1e`. The previous wrap-up validator failure was `release binary hash mismatch: live Rust app`: it compared the rebuilt target path to the historical full-gate app record.

The historical executables are no longer at the current Release paths; no copy of those exact binaries was found under `target/release` or this RB-09 evidence directory. The original hashes remain tied to the paired capture metadata and gate-time validator pass. No binary was restored from a G: snapshot.

The current Release pair was then launched and checked separately. The app process (PID 8516) ran from the current Release path with app hash `0bc4f1a0fd0327e51ecd8acdb75a01120b3c53325b11738a7b3fb6ce198cc75a`; the MCP gateway hash stayed `aff0af548a3f8055e2ebe1d218135a774cc9b88f210a94f19627a9f9b22e1c1e`. Host preflight confirmed the active Default desktop and Builder3D foreground. An official Python MCP SDK 2.2.0 client listed 60 tools and its read-only `get_status` call returned `app_state=ready`, `dirty=false`, and scene revision 0. The 1296x879 screenshot and client JSONL are under `artifact-identity-recheck/`; neither is part of the accepted 50-capture paired inventory. The app, gateway, and runtime descriptor were absent after shutdown.

The validator now checks the historical app/gateway hashes against the full-gate capture metadata, and checks current target hashes against `artifact-identity-recheck.json`. It verifies both rebuild-log copies, the identity screenshot, and the read-only official-client result. The updated validator passed with exit 0; its stdout, stderr, and exit are recorded in `logs/identity-recheck-validator.*`. The 96-file Rust source manifest remains `9846a4a6dee484186283e169a9b7cdbbf74007bdca772efa5981f438ce78b457`; no Rust application source changed.

This check resolves the target-path identity mismatch without repeating the full paired workflow or the 50-capture matrix. It is not visual review or owner approval. The interface and floor gaps, RB-08 native matrix and snapshot, and overall product acceptance remain open. RB-10 has not started.

## Additive recovery snapshot

The earlier additive snapshot `G:/Builder3D-Rust_snapshot_20261104-182021_RB-09-complete` passed verification when created, before the later Release rebuilds. The newer `G:/Builder3D-Rust_snapshot_20261104-193530_RB-09-wrapup-review` is a review snapshot, not a completion snapshot.

The identity follow-up snapshot is
`G:/Builder3D-Rust_snapshot_20261104-215904_RB-09-artifact-identity-review`.
Robocopy exited 1 with no failures. The independent verifier passed with
3,904 files / 701,101,053 bytes on each side and zero missing, extra,
differing, or reparse entries. A direct-path verifier attempt was blocked by
Windows PowerShell path-length resolution on one nested shader-cache path; the
same verifier passed through a temporary Q: drive mapping, removed afterward.
Root target, .cache, and logs were excluded; nested RB-09 evidence logs and Git
metadata were included. This is a review snapshot, not a completion snapshot.
The receipt is in snapshot-receipt.md.

## Boundary

The owner-reported interface and floor gaps remain open. Earlier visual review and owner acceptance remain pending. RB-08’s paired Rust native matrix and completion snapshot remain independently open. No overall product acceptance is claimed. RB-10 has not started.

## Current Rust Release recheck (2026-11-06)

Run `currpair-20261106-072716` exercised the current Rust Release app and gateway. The app SHA-256 is `0bc4f1a0fd0327e51ecd8acdb75a01120b3c53325b11738a7b3fb6ce198cc75a`; the gateway SHA-256 is `aff0af548a3f8055e2ebe1d218135a774cc9b88f210a94f19627a9f9b22e1c1e`.

The official Python MCP SDK 2.2.0 was used only as a stdio client to the Rust gateway. The Python Builder3D application was not launched, and the saved Python catalog snapshots were not refreshed. The historical Python/Rust paired gate above remains tied to its recorded older build.

The Rust client opened `RB09-Paired-Live.3dp`, edited and queried the shared desktop document, saved the result, and the GUI reopened it. The saved output is 82,178 bytes, SHA-256 `e6046a8d3ed7ea949b00a13928311b2fc88e476bda6d4d859af6d866246d9262`, project ID `e5d81c5226e64f58b838d55d9395afa9`, revision 25, clean, with the stable Cube and Sphere IDs. The final GUI capture is `currpair-20261106-072716/captures/rust-current-live-gui-reopened-after-restart-final.png` (SHA-256 `b1223ef8a48c17c3ece09ac77ed263620709bb53a150b18fe4266814ac37e5c5`).

The current optional-MCP startup failure case kept the Rust GUI usable for New Scene, Cube creation, and Undo. The SDK listed 60 tools and got retryable `desktop_unavailable` from `get_status` and `scene_summary`. No runtime descriptor or app-owned listener remained after shutdown. App PID 28536 and optional-failure app PID 5400 exited normally; the gateway and clients also exited. No unrelated process was terminated.

The profile counts and ordered tool names match the saved Python catalogs: observe 7, model 52, files 59, full 60. Exact input-schema and output-schema matches are zero in every profile. Rust adds `return_detail` on 19 tools without server-side use; `orbit_view` requires two fields that its handler defaults; all Rust inputs set `additionalProperties=false`, while Python omits it and differs in bounds/nullability. Rust outputs use one 104-property untyped superset instead of Python tool-specific schemas. Two annotations differ. Top-level result checks found required-field differences for `get_status`, `scene_summary`, `add_primitive`, `undo`, `redo`, `set_transform`, `set_color`, and `new_project`; this was not full nested JSON Schema validation.

Schema details are in `currpair-20261106-072716/schema-comparison.json`. They are findings against saved Python catalogs, not approved deviations or a refreshed current-Python comparison. Keep schema compatibility and RB-09 acceptance pending. The interface/floor gaps, visual review, owner acceptance, RB-08 native matrix/completion snapshot, and overall product acceptance remain open. RB-10 has not started.

## Current additive recovery snapshot

The final snapshot is G:/B3D_R09C_20261106, a new short-name directory on G:. After the final synchronization, the independent verifier reports 3991 files / 756841099 bytes on both sides, zero missing/extra/differing paths, zero reparse entries, and matching critical SHA-256 values. Repository-root target, .cache, and logs are excluded; nested RB-09 evidence logs and Git metadata are included. Full receipt: snapshot-receipt.md.

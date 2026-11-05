# RB-09 Shared AI and Regression Report

**Date:** 2026-11-04  
**Status:** Required paired live desktop/MCP verification, evidence validation, and additive G: snapshot passed after a scoped Rust presentation-sync repair. Stop before RB-10. This is not owner visual approval or overall product acceptance.

## Delivered and verified

- The official Python MCP SDK 2.2.0 connected to both the running Python reference app and the Rust stdio gateway. Each live app exposed the fixed 60-tool catalog and edited its current desktop document through the app-owned bridge.
- Python and Rust used project ID e5d81c5226e64f58b838d55d9395afa9 and stable Cube/Sphere IDs 3f449b78621c467b818e8c69925e2b6c and dff9cc124c6d4d27b4ea9b07d6e4ae6b. Geometry and transforms match. The initial Python capture is revision 4 with the Cube blue; the later saved shared fixture is revision 23 with the Cube gold after a Python MCP edit. Rust’s final-build paired capture opens that revision-23 file. This color/revision difference is recorded in the comparison board.
- Rust MCP New/Open from Welcome exposed the Scene immediately after a scoped repair. MCP edits then appeared in both the viewport and Items panel without restart.
- Shared GUI/MCP history, stable IDs, revision and no-op behavior, save/reopen, New/Open, dirty protection, approved-root and file-failure behavior, replay, revision races, long-operation progress/cancel/failure/stale-result handling, optional MCP startup, closed-desktop responses, and bridge cleanup were exercised with real clients and visible app windows.
- An initial Rust Welcome-to-Scene visibility defect was found, repaired, and retested. Two focused regression tests cover an MCP edit from Welcome and replacing the document with an empty project.

## Live gate outcomes

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

The recorded live app hash remains `d680cf0ba8e4c8bf7d5ccd1536bf7d5720987d676a1a0946c6c3505b613b0cb9`. After the size-check/rebuild work, the current `target/release/builder3d-app.exe` hash is `0bc4f1a0fd0327e51ecd8acdb75a01120b3c53325b11738a7b3fb6ce198cc75a`; the current gateway hash is `aff0af548a3f8055e2ebe1d218135a774cc9b88f210a94f19627a9f9b22e1c1e`. The final wrap-up run of `python evidence/rebuild/rb-09-shared-ai-and-regression/validate_rb09_evidence.py` exits 1 with `release binary hash mismatch: live Rust app`. The original gate-time validator output and exit 0 remain in `logs/evidence-ledger-validation.log` and `.exit`. No Rust source changed after the live sequence, and the recorded hashes continue to identify the binaries actually tested. This wrap-up does not claim the current Release outputs were live-tested.

## Additive recovery snapshot

The new additive snapshot is G:/Builder3D-Rust_snapshot_20261104-182021_RB-09-complete. Its final independent path, byte-count, SHA-256, and reparse verification is recorded in snapshot-receipt.md. Root target, .cache, and logs are excluded; nested RB-09 evidence logs are included.

## Boundary

The owner-reported interface and floor gaps remain open. Earlier visual review and owner acceptance remain pending. RB-08’s paired Rust native matrix and completion snapshot remain independently open. No overall product acceptance is claimed. RB-10 has not started.

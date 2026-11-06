# RB-09 Handoff Checkpoint

**Date:** 2026-11-06
**Status:** the historical Python/Rust paired live gate passed on its recorded build. A current Rust Release GUI/MCP workflow and optional-MCP failure check ran on 2026-11-06. Schema compatibility against saved Python catalogs remains unresolved. Keep RB-09 acceptance, visual review, owner acceptance, and overall product acceptance pending. Stop before RB-10.

## Current state

- Active Rust source/build/run tree: E:/Builder3D-Rust. At the live-gate start the repository worktree was uncommitted and unstaged; no reset, clean, or snapshot restore was used. The later wrap-up explicitly authorized a task-scoped commit.
- Keep every current tracked/untracked file. The only application-code repair in this live gate is the Welcome-to-Scene presentation synchronization in shell.rs and mcp_bridge.rs, with two focused regression tests.
- L:/Builder3D remains the read-only Python reference/shipping tree. No Python source or fixture file was changed. Its external QSettings were restored exactly to object_snapping_enabled=true, ribbon_favorites=[], theme_mode=dark; the test-created recent_files key was removed.
- Before each host UI input/capture, host_desktop.ps1 confirmed the Default desktop was active and interactive. LockApp did not own the live test session. Edge YouTube remained background only; Windows power/lock settings were not changed.
- The live Rust app and gateways are closed. No builder3d-app or r4_mcp process, owned runtime descriptor, or listener remained at final inspection. Edge remains running.
- Python and Rust live clients used the official Python MCP SDK 2.2.0 and listed all 60 tools. Shared project ID: e5d81c5226e64f58b838d55d9395afa9. Stable Cube ID: 3f449b78621c467b818e8c69925e2b6c. Stable Sphere ID: dff9cc124c6d4d27b4ea9b07d6e4ae6b.
- Rust final Release app: 16,172,032 bytes; SHA-256 d680cf0ba8e4c8bf7d5ccd1536bf7d5720987d676a1a0946c6c3505b613b0cb9. Rust source manifest: 96 files; SHA-256 9846a4a6dee484186283e169a9b7cdbbf74007bdca772efa5981f438ce78b457.
- The live matrix passed: MCP edits were immediately visible in viewport and Items; shared GUI/MCP history and no-op behavior passed; save/close/reopen/New/Open and dirty guard passed; file failure atomicity, replay/conflicts, competing clients, long operations, optional startup, closed-desktop errors, and shutdown cleanup passed.
- Post-repair gates: fmt pass; focused app MCP 7/7; workspace 329/329; Clippy pass; locked Release app and gateway builds pass. The evidence validator checked 50 PNG hashes/dimensions, two clean theme pairs, 10 fixture hashes, 96 Rust source files, TOML/JSON/NDJSON, and evidence references.
- Wrap-up rerun: fmt, check, workspace tests, Clippy, deny, metadata, focused MCP tests, and smoke all exited 0; workspace tests were 329 passed, 0 failed, 0 ignored and deny reported 28 duplicate-version warnings. The current app hash `0bc4f1a0fd0327e51ecd8acdb75a01120b3c53325b11738a7b3fb6ce198cc75a` and gateway hash `aff0af548a3f8055e2ebe1d218135a774cc9b88f210a94f19627a9f9b22e1c1e` differ from the historical full-gate artifacts. The later `cargo bloat --workspace` rebuild and app restore build are retained under `logs/identity-recheck-*.stderr.log`; no application source changed.
- Current identity-only recheck: the current app and gateway hashes remained stable before, during, and after the run. Host preflight confirmed the active Default desktop; the official Python MCP SDK 2.2.0 listed 60 tools and `get_status` returned `app_state=ready`, `dirty=false`, revision 0. The 1296x879 screenshot is explicitly identity-only and is not part of the 50 accepted captures. The app, gateway, and runtime descriptor were gone after shutdown. `artifact-identity-recheck.json` records the details.
- The updated evidence validator now checks the historical live hashes against `capture-metadata.json` and the current target hashes against the identity-recheck record. It passed; the previous wrap-up failure (`release binary hash mismatch: live Rust app`) remains documented as the stale path-role check.

## Additive snapshots

The earlier snapshot `G:/Builder3D-Rust_snapshot_20261104-182021_RB-09-complete` passed independent verification when it was created, before the later wrap-up rebuilds and identity recheck. The newer `G:/Builder3D-Rust_snapshot_20261104-193530_RB-09-wrapup-review` is an additive review snapshot, not a completion snapshot.

This follow-up created `G:/Builder3D-Rust_snapshot_20261104-215904_RB-09-artifact-identity-review`. Robocopy copied 3,904 files with no copy failures. The verifier's first direct-path attempt hit a Windows PowerShell path-length resolution error on one nested shader-cache path; the same independent verifier then passed through a temporary `Q:` mapping, which was removed after verification. Source and snapshot both had 3,904 files / 701,101,053 bytes, with zero missing, extra, differing, or reparse entries. The copy excludes only root `target`, `.cache`, and `logs`; nested RB-09 evidence logs and Git metadata are included. It is a review snapshot, not a completion snapshot. No existing G: copy was modified.

## Boundary

The owner-reported interface and floor gaps remain open. Visual review, owner acceptance, and overall product acceptance remain pending. RB-08’s paired Rust native matrix and completion snapshot remain independently open. RB-10 has not started.

## Current continuation checkpoint (2026-11-06)

- Worktree: `E:\Builder3D-Rust`; existing tracked changes and untracked files were preserved. No stage, commit, push, reset, or restore was performed.
- Current Rust Release app/gateway hashes: `0bc4f1a0fd0327e51ecd8acdb75a01120b3c53325b11738a7b3fb6ce198cc75a` / `aff0af548a3f8055e2ebe1d218135a774cc9b88f210a94f19627a9f9b22e1c1e`.
- Live run `currpair-20261106-072716` saved project `e5d81c5226e64f58b838d55d9395afa9` at clean revision 25 with the stable Cube/Sphere IDs; Rust GUI reopen was visible in the accepted capture.
- Optional-MCP startup failure preserved usable GUI work and produced retryable `desktop_unavailable` for two client reads. Both app instances and their clients/gateways shut down normally; no unrelated process was terminated.
- Official Python MCP SDK 2.2.0 was used only against Rust `r4_mcp.exe`; the Python product and saved catalog snapshots were not refreshed.
- Schema names/order match for all four saved profiles, but exact input/output schema matches are zero in every profile. Schema compatibility and RB-09 acceptance remain pending.
- No Rust source changed; no Rust tests were rerun. Previously recorded 329 workspace and 19 focused MCP tests remain historical.
- Final validator self-check/full-validator outcomes and the short-name G: snapshot verification are recorded in `WORK_LOG.md` and `snapshot-receipt.md`.

- Final additive snapshot: G:/B3D_R09C_20261106; independent verification reports 3991 files / 756841099 bytes on both sides, with zero path/hash/reparse differences. Root target, .cache, and logs are excluded; nested RB-09 evidence logs and Git metadata are included. Full receipt and critical hashes are in snapshot-receipt.md.

# RB-09 Handoff Checkpoint

**Date:** 2026-11-04  
**Status:** the paired live desktop/MCP run and its gate-time evidence/snapshot checks passed. The wrap-up recheck later found a current Release app hash mismatch; see `verification.md`. Stop before RB-10.

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
- Wrap-up rerun: fmt, check, workspace tests, Clippy, deny, metadata, focused MCP tests, and smoke all exited 0; workspace tests were 329 passed, 0 failed, 0 ignored and deny reported 28 duplicate-version warnings. The current app hash `0bc4f1a0fd0327e51ecd8acdb75a01120b3c53325b11738a7b3fb6ce198cc75a` differs from the recorded live app hash `d680cf0ba8e4c8bf7d5ccd1536bf7d5720987d676a1a0946c6c3505b613b0cb9`; the current evidence validator rerun therefore exits 1. No application source changed after the live test.

## Additive snapshot

The additive completion snapshot is G:/Builder3D-Rust_snapshot_20261104-182021_RB-09-complete. The independent verifier reported equal included file counts and byte totals, zero missing/extra/different files, and zero included reparse entries. Exact values and exclusions are recorded in snapshot-receipt.md. The snapshot is not a working tree and no existing G: copy was overwritten.

## Boundary

The owner-reported interface and floor gaps remain open. Visual review, owner acceptance, and overall product acceptance remain pending. RB-08’s paired Rust native matrix and completion snapshot remain independently open. RB-10 has not started.

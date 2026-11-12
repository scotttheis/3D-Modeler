# Builder3D Rust — status

The only place current status is written. Keep it to one screen. Plan:
[`product-rebuild-plan.md`](product-rebuild-plan.md).

Updated 2026-11-12.

## Now

Stage **S0 Stabilise**. Nothing in S1–S4 starts until the fix-first list is
closed. Stop before RB-10 work (S3) as before.

## Gates on the working tree (2026-11-12, debug build)

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass |
| `cargo test --workspace --locked --no-fail-fast` | Pass (re-run 2026-11-12 after each OF commit; F2 commit `f1b581d` fixed the six below) |
| `cargo deny check` | Not run |
| Release (fat LTO) build | Not run this session; failed with LLVM out-of-memory on 2026-11-11 |
| `review` profile build | Pass (3m56s clean build; capture verified) |

Failing tests, all present before the 2026-11-12 finish pass and all
unreviewed — each needs a decision: is the test stale, or is the code wrong?

| Test | Symptom | Likely cause |
|---|---|---|
| `builder3d-app` `commands::rb04_gesture_tests::cancelled_noop_stale_and_replaced_document_keep_scene_safe` | Y is 20, expected 0 | Primitives now rest on the floor; test predates it |
| `builder3d-app` `transforms::tests::group_translation_is_atomic_and_undoable` | Position mismatch | Same |
| `builder3d-mcp` `r4::server::tests::core_calls_are_structured_and_replay_is_idempotent` | Y is 5, expected 20 | Same change, seen from the MCP side — **check the catalog contract before editing the test** |
| `builder3d-app` `view_state::tests::state_updates_and_presets_keep_camera_bounds` | Yaw 0, expected 45 | Default view changed on 2026-11-09 |
| `builder3d-app` `shell::tests::gizmo_release_commits_once_and_cancel_or_noop_keeps_history` | Assertion mismatch | Gizmo rework 2026-11-10/11; may be a real one-undo-per-gesture regression |
| `builder3d-app` `shell::tests::modal_pointer_input_cannot_duplicate_or_apply_background_transform` | Count 2, expected 1 | Same rework; may be real |

## Fix-first list (plan section 4)

| # | Item | State |
|---|---|---|
| F1 | App tests compile | **Done** 2026-11-12 |
| F2 | Whole suite green | **Done** 2026-11-12 (`f1b581d`; re-verified after OF-1) |
| F3 | Clippy on all targets | **Done** 2026-11-12 |
| F4 | Baseline commit of 220+ uncommitted paths | **Done** 2026-11-12 on `rebuild/work` |
| F5 | Release link reliability | Open |
| F6 | Reconcile the 147-row feature ledger with the app | Open |
| F7 | Clear ad hoc `target/` directories (`E:` had 40 GB free) | **Done** 2026-11-12 — archived to `G:\Builder3D-Rust_target-adhoc-archive_20261112`, `target/` 91 → ~55 GB |
| F8 | `scripts/Invoke-Gate.ps1` | Open |
| F9 | Depth fight where meshes meet the floor | Open |
| F10 | `emboss_text` long-running metadata | Blocked on decision D5 |
| F11 | Status narrative out of `AGENTS.md` | **Done** 2026-11-12 |

## Ledger (`evidence/rebuild/parity-matrix.toml`, 147 features, not yet reconciled)

Implemented 81 · partial 69 · owner-accepted 3 · visual review pending on all
but 2. Slice history: RB-02 owner-accepted; RB-03 to RB-07 engineering recorded,
owner review pending; RB-08 native interaction matrix pending; RB-09 schema
parity complete, full acceptance open.

## Owner decisions waiting (plan section 8)

D1, D2, D3 approved 2026-11-12. Still open: D4 light viewport in dark theme ·
D5 `emboss_text` metadata.

## Owner feedback queue

- 2026-11-02: interface and floor "still missing" — floor addressed
  2026-11-09/10; interface finish is stage S1.
- 2026-11-11: move / resize / rotate arrows "feel cheap" — handled as one
  reviewed set in S1 task 7, not iteratively.
- 2026-11-12: the app "looks kind of home made" — first finish pass landed
  (captures in `evidence/rebuild/captures/2026-11-12-finish-pass/`); remaining
  work is S1 tasks 1–9. **Not yet reviewed by the owner.**
- 2026-11-12 (second batch): nine items OF-1 to OF-9 in
  [`owner-feedback-2026-11-12.md`](owner-feedback-2026-11-12.md), worked in
  the owner's order (OF-1 floor, OF-1 resize + OF-3, OF-4, OF-6, OF-9, OF-2,
  OF-7/8; OF-5 waits for shots), one commit each, **none owner-reviewed
  yet**. Done so far: OF-1 floor numbers (flat on the floor plane, depth-
  tested); OF-1 resize view + OF-3 (3D Builder box, dots, chevrons, size
  label, drag band); OF-4 top-bar hover. Next: OF-6 wireframe. Shots still owed by the owner: X-ray with two objects,
  Paint/Color/Material, Merge.
- Review harness: `BUILDER3D_REVIEW_CAMERA=yaw,pitch,distance` poses the
  camera after auto-framing; state `resize-drag` holds the +X handle mid-drag;
  `BUILDER3D_REVIEW_POINTER=x,y` rests the pointer to show hover states.

## Last recovery point

Git: branch `rebuild/work`, baseline commit of 2026-11-12. Tree snapshot:
`G:\Builder3D-Rust_snapshot_20261112-042842_plan-v2-finish-pass` (4,133 files,
781,834,354 bytes, counts verified).

# Builder3D Rust — status

The only place current status is written. Keep it to one screen. Plan:
[`product-rebuild-plan.md`](product-rebuild-plan.md).

Updated 2026-11-13. Full gate passed 2026-11-13 (fmt, clippy, tests, deny, audit, MCP parity, smoke, 28-image capture board; the board and the other steps ran in two runs because the first run's smoke step had a script bug, since fixed). Release exe rebuild for this work is **not done**: the running `targeteleaseuilder3d-app.exe` locks the file.

## Now

Stage **S0 Stabilise**. Nothing in S1–S4 starts until the fix-first list is
closed. Stop before RB-10 work (S3) as before.

## Gates on the working tree (2026-11-12, debug build)

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass |
| `cargo test --workspace --locked --no-fail-fast` | Pass (2026-11-12, after the shadow follow-up and the MCP Merge color change) |
| `cargo deny check` | Pass 2026-11-12 (advisories, bans, licenses, sources ok; warning: `async-executor 1.14.0` yanked, Linux accessibility path only) |
| `cargo audit` | Pass 2026-11-12 — 0 vulnerabilities in 493 crates; 5 allowed warnings (4 unmaintained already accepted in `deny.toml`, 1 yanked as above). Tool installed 2026-11-12 |
| MCP catalog parity check | Pass 2026-11-13 (`cargo test -p builder3d-mcp --test ledger_parity`; all 147 ledger rows map to a catalog tool or "visual only") |
| Release (fat LTO) build | `cargo build --release -p builder3d-app` passed twice on 2026-11-12 (4m50s, 3m43s); failed with LLVM out-of-memory on 2026-11-11 |
| `review` profile build | Pass (3m56s clean build; capture verified) |

## Fix-first list (plan section 4)

| # | Item | State |
|---|---|---|
| F1 | App tests compile | **Done** 2026-11-12 |
| F2 | Whole suite green | **Done** 2026-11-12 (`f1b581d`; re-verified after OF-1) |
| F3 | Clippy on all targets | **Done** 2026-11-12 |
| F4 | Baseline commit of 220+ uncommitted paths | **Done** 2026-11-12 on `rebuild/work` |
| F5 | Release link reliability | Open — three clean release builds 2026-11-12 (third 4m05s, after the snap fix); not yet shown reliable |
| F6 | Reconcile the 147-row feature ledger with the app | Open |
| F7 | Clear ad hoc `target/` directories (`E:` had 40 GB free) | **Done** 2026-11-12 — archived to `G:\Builder3D-Rust_target-adhoc-archive_20261112`, `target/` 91 → ~55 GB |
| F8 | `scripts/Invoke-Gate.ps1` | **Done** 2026-11-13 — `-Tier fast\|full\|ship`; ship does not script package staging or the performance run |
| F9 | Depth fight where meshes meet the floor | **Done, owner review pending** 2026-11-13 — depth bias on floor surface, shadows and text; captures `evidence/rebuild/captures/2026-11-13-f9-floor-depth/` (before/after, both themes). Dark serration gone; a faint pale dotted fringe on the rim remains and does not change with the bias or the reflection (cause not found). CPU fallback not touched |
| F10 | `emboss_text` long-running metadata | Blocked on decision D5 |
| F11 | Status narrative out of `AGENTS.md` | **Done** 2026-11-12 |
| F12 | MCP 100% desktop parity (plan 3.7) | **Done** 2026-11-13 — Merge color; `rename_object` and `set_visibility` tools (catalog now 62, each with a desktop-vs-MCP test); `launch_slicer` falls back to the saved slicer; ledger `mcp_tool` / `mcp_exempt` column and parity check. Judgement calls for owner: theme, display units, clear recents, About, ribbon layout, sticky selection, recents and exit are marked "visual only" |
| F13 | Run `cargo deny` and `cargo audit` | **Done** 2026-11-12 — both pass; now in the Full gate |

## Ledger (`evidence/rebuild/parity-matrix.toml`, 147 features, not yet reconciled)

Implemented 81 · partial 69 · owner-accepted 3 · visual review pending on all
but 2. Slice history: RB-02 owner-accepted; RB-03 to RB-07 engineering recorded,
owner review pending; RB-08 native interaction matrix pending; RB-09 schema
parity complete, full acceptance open.

## Owner decisions waiting (plan section 8)

D1, D2, D3, D6 (MCP 100% parity, catalog may grow) and D7 (Merge takes the
top-most color, desktop and MCP) approved 2026-11-12. Still open: D4 light
viewport in dark theme · D5 `emboss_text` metadata.

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
  yet**. All committed: OF-1 floor numbers `6fbb5b7`; OF-1 resize view and
  OF-3 `53a45f1`; OF-4 hover `8797405`; OF-6 wireframe `fc70679`; OF-9 Merge
  color `38d6437`; OF-2 shadow and reflection `f7e19aa`; OF-7/OF-8
  `89c1e6f`. Cause of OF-7/OF-8: the Paint tab and the sculpt tools were
  disabled whenever nothing was selected, and every slider was invisible.
  Details, known limits and lessons are in the batch summary at the end of
  the feedback file. Open: OF-5 (waits for shots), face/region painting, a
  comparison with Blender's brushes, the shadow and reflection on the CPU
  fallback, and Merge of more than two objects. Shots still owed by the
  owner: X-ray with two objects, Paint/Color/Material, Merge.
- 2026-11-12 (shadow follow-up, committed `43f0fd0`): floor
  shadow now follows the object's convex outline instead of its bounding
  box; shadows and floor text hide with the floor when the camera is below
  it; a dragged object's shadow hides during the gesture and returns once
  it lands (owner: "way better"; ideal would be a shadow that follows the
  object). Details and open points at the end of
  [`owner-feedback-2026-11-12.md`](owner-feedback-2026-11-12.md). Full
  workspace gate passed before commit.
- 2026-11-12: owner wants MCP at 100% (F12, D6, D7); `cargo deny` and
  `cargo audit` now run (F13). Not stressed about module size, lint
  enforcement or more comments; those are not work items.
- 2026-11-12 (snap re-capture, approved deviation from Python): owner
  reported that after missing a snap point while dragging, nothing snapped
  again for the rest of the drag. Cause: the released snap point stayed
  excluded until the pointer crossed back past it (Python behaviour). Now the
  exclusion lifts as soon as the pointer leaves the point's capture band, so
  coming back near it snaps again from either side. Desktop drag only; the
  MCP `snap_translate` tool is one-shot and already snaps whenever near.
  Test `missed_snap_recaptures_when_the_pointer_comes_back_near_it`.
  Commit `2c7b319`. **Owner-approved** in the release build 2026-11-12
  ("fantastic").
- 2026-11-12 (third batch): OF-10 to OF-14 **owner-approved** in the
  release build ("you did amazing. i say yes to it all") — icon-only
  shortcuts, Slicer button with remembered slicer (no dialog), labelled
  Save / Save As, light/dark toggle (`0f3dd09`); Shift+drag box select,
  additive, touching counts (`907f9b0`). Full workspace gate passed.
  Captures `evidence/rebuild/captures/2026-11-12-of10-14-topbar/`. Table in
  [`owner-feedback-2026-11-12.md`](owner-feedback-2026-11-12.md).
- 2026-11-12 (fourth batch): OF-15 to OF-19 **owner-approved** in the
  release build 2026-11-12 ("You did great. i approve of it all") — darker light-mode top bar, Settings follows the top-bar theme
  toggle, a press outside Settings closes it, one Close button (Done
  removed), menu icon centred with the tab text. Full workspace gate passed.
  Captures `evidence/rebuild/captures/2026-11-12-of15-19-settings-topbar/`.
  Table in [`owner-feedback-2026-11-12.md`](owner-feedback-2026-11-12.md).
- 2026-11-13 (sixth batch): OF-26 menu rows highlight under the pointer
  (also the Slicer chooser); OF-27 the active tab is joined to the open
  ribbon like a folder tab. **Owner-approved** in the release build
  2026-11-13 ("dang, that looks good. well done!"). Fast gate on
  `builder3d-app` passed. Captures
  `evidence/rebuild/captures/2026-11-13-of26-27-menu-hover-tab/`. Table in
  [`owner-feedback-2026-11-12.md`](owner-feedback-2026-11-12.md).
- 2026-11-13 (test release RC1): `packaging/msi/Build-Msi.ps1 -Version 0.1.1
  -Name Builder3D-RC1` builds a per-user WiX 5 MSI (no admin; installs to
  `%LOCALAPPDATA%\Programs\Builder3D`; Start-menu + desktop shortcuts; VC++
  runtime app-local; bump the version's third field per candidate).
  Output `E:\Builder3D-RC\Builder3D-RC1.msi` (50 MB). Silent install,
  launch, smoke and clean uninstall verified on the dev PC; not yet tried on
  a clean PC. Fixed `--package` artwork staging (arguments swapped; never
  worked) with a regression test. The script stages directly because the
  package is 64 MB against the 40 MB S3 budget: the action icons are
  1254 px PNGs (~32 MB). Shrinking them is an S3 owner decision.
- Owner merge report 2026-11-13: Merge of a sphere and a cylinder that only
  touched gave one object with 2 disconnected shells (no warning); smoothing
  then melted the sparse stock cylinder (two rings only). Options and owner
  decision pending.
- **Paused by the owner 2026-11-12** after batch 3. On resume, suggested
  next: MCP `launch_slicer` falls back to the saved slicer when no
  `slicer_key` is given (owner said yes; F12 parity), then F12 rename and
  show/hide tools, then F9.
- Review harness: `BUILDER3D_REVIEW_CAMERA=yaw,pitch,distance` poses the
  camera after auto-framing; state `resize-drag` holds the +X handle mid-drag;
  `BUILDER3D_REVIEW_POINTER=x,y` rests the pointer to show hover states;
  `BUILDER3D_REVIEW_VIEW=wireframe,xray` turns view toggles on; state
  `merge` merges a red and a blue cube; `unselected`, `material` states;
  `BUILDER3D_REVIEW_INTERACTIVE=1` stays open with isolated settings for
  hands-on checks.

## Last recovery point

Git: branch `rebuild/work`, see `git log` (F12 `67334e7`, `789b282`, `d402d86`; F9 `3211c00`; F8 `91b6431`, `456f1d5`). Tree snapshot:
`G:\Builder3D-Rust_snapshot_20261113-140753_F8-F9-F12` (5,947 files,
851,687,128 bytes, counts verified; excludes `target\`).

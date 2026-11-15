# Builder3D Rust — status

The only place current status is written. Keep it to one screen. Plan:
[`product-rebuild-plan.md`](product-rebuild-plan.md).

Updated 2026-11-15. Full gate passed 2026-11-15 after Phase 3 (fmt, clippy, tests, deny, audit, MCP parity, smoke, 62-image capture board; the board now also takes `grab`, `crease`, `smooth-brush`, `simplify`, `subdivide`, `hollow`). Release exe rebuilt from the same source for the Phase 3 review pack.

Test releases (how-to: [`packaging.md`](packaging.md)): last is **RC1**,
version 0.1.1, 2026-11-13, `E:\Builder3D-RC\Builder3D-RC1.msi` (50 MB),
`-Verify` passed on the dev PC. Next: RC2 / 0.1.2.

## Now

**Owner batch 7** ([`owner-feedback-2026-11-14.md`](owner-feedback-2026-11-14.md),
OF-28 to OF-58, five phases; Emboss OF-56/57 deferred by the owner).
Phase 1 done (OF-28 to OF-45), pack
`evidence/rebuild/captures/2026-11-14-review-phase1/`. Phase 2: OF-44 Paint
and OF-47 X-ray approved by the owner 2026-11-15 (pack
`evidence/rebuild/captures/2026-11-14-review-phase2/`). Owner answers
2026-11-15: OF-35 keep shade A as built; OF-43 confirmed (10 mm gap beside the
original); OF-44 keep the "Done" label; OF-46 store the texture image inside
the project file (`.3dp` format change approved), test textures in
`C:\Users\josiph\Pictures\textures\` (owner-supplied; do not commit them).
OF-46 done 2026-11-15 (`654872a`) and approved by the owner 2026-11-15 ("it
works, good enough for now"); the texture dialog is to be revisited later
(last item below). Phase 2 is closed.

Phase 3 (OF-48 to OF-54) built 2026-11-15, `e95dbd3` to `114a676`, pack
`evidence/rebuild/captures/2026-11-14-review-phase3/`; **stopped for owner
review**. Open owner answers (ask again at the next stop): OF-53 Normalize
— rebuild it, fold it into Subdivide/Simplify, or remove it (findings in the
row); OF-52 — is brushing over an area the right way to choose "selected
faces"; OF-50 — should the whole-object Smooth button also keep sharp edges
(only the Smooth Brush does now). Also seen, not yet fixed: garbled text
("Ã¢â‚¬â€") in six status messages in `shell.rs`, present before Phase 3.

**Known work, in order** (a session takes the first item not done):

1. Phase 3 — owner review of the pack; then OF-53 as the owner decides.
2. Phase 4 — Measure ruler (OF-55).
3. Phase 5 — MCP does everything (OF-58).
4. F5 — release link reliability; F6 — reconcile the 148-row ledger
   (fix-first list below).
5. Revisit the Material texture dialog (OF-46 follow-up): the owner wants to
   look at its layout again; ask what to change before building.
6. Stop before RB-10 (S3). OF-56/57 (Emboss) stay deferred until the owner
   brings them back.

Each phase ends with its review pack and a stop for owner review, as
[`owner-feedback-2026-11-14.md`](owner-feedback-2026-11-14.md) says. When the
owner starts a new session and asks to carry on, that counts as having looked
at the previous pack; open owner answers (above) are kept and asked again in
the next stop message, not guessed.

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

## Last review pack

`evidence/rebuild/captures/2026-11-14-review-phase3/` (Phase 3: OF-48 to
OF-54), not yet reviewed. `evidence/rebuild/captures/2026-11-14-review-phase2/` (Phase 2: OF-44 and
OF-47 approved 2026-11-15; OF-46 added and approved 2026-11-15). `evidence/rebuild/captures/2026-11-14-review-phase1/`
(Phase 1), not yet reviewed. Before that:

`evidence/rebuild/captures/2026-11-13-review-of1-9-f9/`, reviewed by the
owner 2026-11-14. Approved: floor look, F9, resize view, OF-2 shadow, OF-6
wireframe (for now), OF-9 Merge color, Smooth / Smooth Brush / Blob,
Intersect, Subtract. Follow-ups became OF-28 to OF-58. OF-4 hover: owner did
not know what it meant; explain it in the next pack.

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
| F9 | Depth fight where meshes meet the floor | **Owner-approved 2026-11-14** ("works exactly as I asked"); done 2026-11-13 — depth bias on floor surface, shadows and text; captures `evidence/rebuild/captures/2026-11-13-f9-floor-depth/` (before/after, both themes). Dark serration gone; a faint pale dotted fringe on the rim remains and does not change with the bias or the reflection (cause not found). CPU fallback not touched |
| F10 | `emboss_text` long-running metadata | Decided (D5): keep the flag; progress indicator is OF-57, deferred by the owner 2026-11-14 |
| F11 | Status narrative out of `AGENTS.md` | **Done** 2026-11-12 |
| F12 | MCP 100% desktop parity (plan 3.7) | **Done** 2026-11-13 — Merge color; `rename_object` and `set_visibility` tools (catalog now 62, each with a desktop-vs-MCP test); `launch_slicer` falls back to the saved slicer; ledger `mcp_tool` / `mcp_exempt` column and parity check. Owner 2026-11-14 (D10): no "visual only" exemptions — MCP gets tools for all of them (OF-58) |
| F13 | Run `cargo deny` and `cargo audit` | **Done** 2026-11-12 — both pass; now in the Full gate |

## Ledger (`evidence/rebuild/parity-matrix.toml`, 147 features, not yet reconciled)

Implemented 81 · partial 69 · owner-accepted 3 · visual review pending on all
but 2. Slice history: RB-02 owner-accepted; RB-03 to RB-07 engineering recorded,
owner review pending; RB-08 native interaction matrix pending; RB-09 schema
parity complete, full acceptance open.

## Owner decisions waiting (plan section 8)

D1, D2, D3, D6 (MCP 100% parity, catalog may grow) and D7 (Merge takes the
top-most color, desktop and MCP) approved 2026-11-12. D4 (somewhat darker
viewport in dark theme), D5 (progress indicator for emboss), D8 (non-overlapping
Merge groups + 4 s popup) and D9 (smoothing like Blender's sculpt Smooth)
decided 2026-11-13, none built yet. D10 (MCP does everything,
no "visual only" rows) decided 2026-11-14. Build order for D4/D5/D8/D9/D10:
owner batch 7.

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
  fallback, and Merge of more than two objects. X-ray and Paint reference shots received 2026-11-14 (OF-47, OF-44); Merge
  needs none now (owner approved it).
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
- 2026-11-13 (test release RC1; how-to in [`packaging.md`](packaging.md)):
  `packaging/msi/Build-Msi.ps1 -Version 0.1.1 -Name Builder3D-RC1 -Build
  -Verify` builds a per-user WiX 5 MSI (no admin; installs to
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
  then melted the sparse stock cylinder (two rings only). Decided 2026-11-13:
  D8 and D9 (plan section 8). Not built yet.
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
  hands-on checks. States `texture`, `texture-dialog` take pictures from
  `BUILDER3D_REVIEW_TEXTURES` (`;`-separated paths).

## Last recovery point

Git: branch `rebuild/work`, see `git log` (Phase 3: `e95dbd3` OF-48,
`e9aa73d` OF-49, `7248d91` OF-50, `7432d0b` OF-51, `54b6f2a` OF-52,
`e283f15` OF-53 findings, `114a676` OF-54; Phase 2: `654872a` (OF-46), `344e88e` (OF-44),
`82f4a58` (OF-47), pack `06699d2`; Phase 1: `566ba48` to `3288a55`). Tree
snapshot: `G:\Builder3D-Rust_snapshot_20261115-091718_OF48-54-phase3` (7,386
files, 1,028,974,653 bytes, counts verified; excludes `target\` and `logs\`).
Previous: `G:\Builder3D-Rust_snapshot_20261115-055440_OF46-phase2`.

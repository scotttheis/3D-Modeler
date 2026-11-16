# 3D Modeler Rust — status

The only place current status is written. Keep it to one screen. Plan:
[`product-rebuild-plan.md`](product-rebuild-plan.md).

Updated 2026-11-16. Full gate passed 2026-11-16 after OF-65 (fmt, clippy, tests, deny, audit, MCP parity, smoke, capture board). Before that, full gate passed 2026-11-16 after Phase 6 OF-63/OF-64 (fmt, clippy, tests, deny, audit, MCP parity, smoke, capture board). Earlier: full gate passed 2026-11-15 after Phase 5 OF-58 (fmt, clippy, tests, deny, audit, MCP parity with no exemptions, smoke, capture board); a last small fix then passed the Fast gate. Release exe rebuilt from that source (3m49s) and driven live through the real MCP gateway for the Phase 5 review pack.

Test releases (how-to: [`packaging.md`](packaging.md)): last is **RC1**,
version 0.1.1, 2026-11-13, `E:\3D-Modeler-RC\Builder3D-RC1.msi` (50 MB),
`-Verify` passed on the dev PC. Next: RC2 / 0.1.2.

## Now

**Renamed 2026-11-16:** the app is now **3D Modeler** (was Builder3D); hidden names use `Modeler3D` / `modeler3d`. Where the old name stays, and why, is in the "Name" section of [`AGENTS.md`](../AGENTS.md). Pre-rename backup: `G:\Builder3D-Rust_snapshot_20261116-094924_pre-rename-3D-Modeler`.

**Shortcut bar 2026-11-16 (owner request):** a first start now gets Cube, Sphere, Settle, Mirror, Merge, Subtract. After that the bar is the person's own: every change is saved and kept, and an emptied bar stays empty (before, an empty bar got the defaults back at the next start). Existing settings keep their bar; Customize ribbon → Reset defaults gives the new six. Test `shortcut_bar_starts_with_the_owner_set_and_then_keeps_what_the_person_chose`. An empty bar shows "Drag shortcuts here" in small dim text after the View tab; **owner 2026-11-16: approved ("I love it. shade, size, location, all of it")**. Capture state `no-shortcuts`. **Drag on the bar, 2026-11-16 (owner request):** while a shortcut is dragged, an accent-coloured bar shows the gap it will land in and its icon follows the pointer (no bar where the drop would change nothing); a ribbon button can be dragged up onto the bar and is pinned at that gap (a click still runs it). On a full bar a new shortcut dropped at the end lands before the last visible one, not hidden in "More". `place_favorite_at` (`ui_state.rs`) with tests; `favorites_drop` (`shell.rs`). **Owner 2026-11-16: approved ("great job")** in the release build.

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

Phase 3 (OF-48 to OF-54) built 2026-11-15, pack
`evidence/rebuild/captures/2026-11-14-review-phase3/`. **Owner review
2026-11-15:** Grab, Crease, Smooth Brush, Simplify, Subdivide approved;
Hollow works at a 2 mm wall; Normalize to be removed; the whole-object Smooth
button stays as it is. Follow-ups OF-59 to OF-61 (OF-62 garbled text done).

Follow-ups OF-59 to OF-61 built 2026-11-15 (`57671fb`, `fbbfff8`,
`0238ccf`), pack `evidence/rebuild/captures/2026-11-15-review-phase3-followups/`.
**Owner review 2026-11-15: all three approved.** MCP `normalize_face_density`
stays (owner: keep it in AI for possible later use). Phase 3 is closed.

Phase 4 (OF-55 Measure ruler) built 2026-11-15 (`cfc96a7`), pack
`evidence/rebuild/captures/2026-11-15-review-phase4-measure/`. Measure is now
a click-to-click ruler with snapping; new MCP tool `measure` (catalog 66).
**Owner 2026-11-15: approved ("better than I imagined"; "call it all done").**
Owner's one request, Esc closes Measure in one press, done the same day.
Phase 4 is closed.

Phase 5 (OF-58, the AI does everything) built 2026-11-15, pack
`evidence/rebuild/captures/2026-11-15-review-phase5-mcp/`. Eight new MCP
tools (catalog 74): `app_settings`, `set_app_settings`, `customize_ribbon`,
`show_screen`, `recent_files`, `open_recent`, `capture_window`, `exit_app`,
carried out by the desktop shell with its own code. No "visual only" ledger
rows remain. **Owner 2026-11-16: approved ("it tests great … call it done").**
Phase 5 is closed.

Phase 6 (OF-63 Items double-click centres, OF-64 settings note) built
2026-11-16, pack `evidence/rebuild/captures/2026-11-16-review-phase6/`.
MCP `frame_view` in the live app now frames with the desktop camera (same
camera as a double-click); `app_settings` gains `settings_notice`,
`set_app_settings` gains `dismiss_settings_notice`. **Waiting for owner
review**; the owner may add more Phase 6 rows. OF-65 (transform strip: the
"N selected / mm" note removed; each Move / Rotate / Resize label centred over
its X / Y / Z box (owner follow-up)) built 2026-11-16, pack
`evidence/rebuild/captures/2026-11-16-of65-transform-strip/`, test
`transform_strip_has_no_count_and_each_tab_is_centred_over_its_box`. **Waiting
for owner review.** Noticed while capturing, not changed: Rotate shows
"-0.000 deg" for an unrotated cube's Y.

**Known work, in order** (a session takes the first item not done):

1. Done and approved 2026-11-15: Phase 3 follow-ups OF-59 to OF-61.
2. Done and approved 2026-11-15: Phase 4 — Measure ruler (OF-55).
3. Done and approved 2026-11-16: Phase 5 — MCP does everything (OF-58).
4. Phase 6 — more fixes before F5 (owner 2026-11-16: "let's finish fixing
   everything before f5"): OF-63 double-click an Items row to centre on it,
   OF-64 settings-migration notice overlapping the ribbon — both built
   2026-11-16, owner review pending. OF-65 transform strip clean-up built
   2026-11-16, owner review pending. No more Phase 6 rows to build; the
   owner may add more; finish them all before item 5.
5. F5 — release link reliability; F6 — reconcile the 148-row ledger
   (fix-first list below).
6. Revisit the Material texture dialog (OF-46 follow-up): the owner wants to
   look at its layout again; ask what to change before building.
7. Stop before RB-10 (S3). OF-56/57 (Emboss) stay deferred until the owner
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
| MCP catalog parity check | Pass 2026-11-15 (`cargo test -p modeler3d-mcp --test ledger_parity`; all 148 ledger rows map to catalog tools; exemptions now fail, D10) |
| Release (fat LTO) build | `cargo build --release -p modeler3d-app` passed twice on 2026-11-12 (4m50s, 3m43s); failed with LLVM out-of-memory on 2026-11-11 |
| `review` profile build | Pass (3m56s clean build; capture verified) |

## Last review pack

`evidence/rebuild/captures/2026-11-16-of65-transform-strip/` (OF-65), owner review pending. `evidence/rebuild/captures/2026-11-16-review-phase6/` (OF-63, OF-64), owner review pending. `evidence/rebuild/captures/2026-11-15-review-phase5-mcp/` (OF-58), approved
by the owner 2026-11-16. `evidence/rebuild/captures/2026-11-15-review-phase4-measure/` (OF-55),
approved by the owner 2026-11-15. `evidence/rebuild/captures/2026-11-15-review-phase3-followups/` (OF-59 to
OF-61), approved by the owner 2026-11-15. `evidence/rebuild/captures/2026-11-14-review-phase3/` (Phase 3: OF-48 to
OF-54), reviewed by the owner 2026-11-15. `evidence/rebuild/captures/2026-11-14-review-phase2/` (Phase 2: OF-44 and
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
  `modeler3d-app` passed. Captures
  `evidence/rebuild/captures/2026-11-13-of26-27-menu-hover-tab/`. Table in
  [`owner-feedback-2026-11-12.md`](owner-feedback-2026-11-12.md).
- 2026-11-13 (test release RC1; how-to in [`packaging.md`](packaging.md)):
  `packaging/msi/Build-Msi.ps1 -Version 0.1.1 -Name Builder3D-RC1 -Build
  -Verify` builds a per-user WiX 5 MSI (no admin; installs to
  `%LOCALAPPDATA%\Programs\Modeler3D`; Start-menu + desktop shortcuts; VC++
  runtime app-local; bump the version's third field per candidate).
  Output `E:\3D-Modeler-RC\Builder3D-RC1.msi` (50 MB). Silent install,
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
- Review harness: `MODELER3D_REVIEW_CAMERA=yaw,pitch,distance` poses the
  camera after auto-framing; state `resize-drag` holds the +X handle mid-drag;
  `MODELER3D_REVIEW_POINTER=x,y` rests the pointer to show hover states;
  `MODELER3D_REVIEW_VIEW=wireframe,xray` turns view toggles on; state
  `merge` merges a red and a blue cube; `unselected`, `material` states;
  `MODELER3D_REVIEW_INTERACTIVE=1` stays open with isolated settings for
  hands-on checks. States `texture`, `texture-dialog` take pictures from
  `MODELER3D_REVIEW_TEXTURES` (`;`-separated paths).

- 2026-11-15 (found during the OF-58 live run): the settings-migration
  notice drawn over the ribbon labels — now OF-64.

## Last recovery point

Git: branch `rebuild/work`, see `git log` (OF-65: `02fc0ce`; Phase 6: `1ac2583` OF-63/OF-64; Phase 5: `2f6aaf8` OF-58; Phase 4: `cfc96a7` OF-55,
pack `ff8b28f`, one-press Esc `229f0d2`; Phase 3 follow-ups: `57671fb`
OF-59, `fbbfff8` OF-60, `0238ccf` OF-61; Phase 3: `e95dbd3` OF-48,
`e9aa73d` OF-49, `7248d91` OF-50, `7432d0b` OF-51, `54b6f2a` OF-52,
`e283f15` OF-53 findings, `114a676` OF-54; Phase 2: `654872a` (OF-46), `344e88e` (OF-44),
`82f4a58` (OF-47), pack `06699d2`; Phase 1: `566ba48` to `3288a55`). Tree
snapshot: `G:\3D-Modeler_snapshot_20261116-121325_OF65-transform-strip` (9,683
files, 1,275,301,662 bytes, counts verified; excludes `target\` and `logs\`).
Previous: `G:\Builder3D-Rust_snapshot_20261116-091132_OF63-64-phase6`.

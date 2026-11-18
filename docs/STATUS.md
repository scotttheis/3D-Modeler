# 3D Modeler Rust — status

The only place current status is written. Keep it to one screen. Plan:
[`product-rebuild-plan.md`](product-rebuild-plan.md).

Updated 2026-11-18 (Resize Settle and robot-claw grab cursors). F5 and F6 closed 2026-11-16 (ledger parity test passed after F6). Full gate passed 2026-11-16 after OF-65 (fmt, clippy, tests, deny, audit, MCP parity, smoke, capture board). Before that, full gate passed 2026-11-16 after Phase 6 OF-63/OF-64 (fmt, clippy, tests, deny, audit, MCP parity, smoke, capture board). Earlier: full gate passed 2026-11-15 after Phase 5 OF-58 (fmt, clippy, tests, deny, audit, MCP parity with no exemptions, smoke, capture board); a last small fix then passed the Fast gate. Release exe rebuilt from that source (3m49s) and driven live through the real MCP gateway for the Phase 5 review pack.

Test releases (how-to: [`packaging.md`](packaging.md)): last is **RC1**,
version 0.1.1, 2026-11-13, `E:\3D-Modeler-RC\Builder3D-RC1.msi` (50 MB),
`-Verify` passed on the dev PC. **RC2 / 0.1.2, 2026-11-16, Linux:**
`E:\3D-Modeler-RC\3D-Modeler-0.1.2-linux-x86_64.tar.gz` (52 MB), also on the
USB card under `3D-Modeler-packaging\linux\`; full Windows gate passed after
the fixes (2026-11-16 16:38); 487 tests, clippy, fmt passed in
WSL Ubuntu 24.04; packaged app run under WSLg: insert, save, reopen through
MCP, settings in `~/.config/Modeler3D`. Not checked: a real Linux GPU and
desktop (WSL only has the CPU route), the AppImage (needs the owner's OK to
download `appimagetool`). **Mac RC2 / 0.1.2, 2026-11-17:**
`3D-Modeler-packaging/mac/3D-Modeler-0.1.2.dmg` (62 MB) on the USB card;
universal arm64 + x86_64 app, built and `--smoke` passed; DMG checksum
verified. Mac fmt and Clippy passed. Workspace tests: 253 passed; one loopback
bridge test resets its connection in full-suite runs but passes alone. The
owner installed and launched the DMG app on the Mac on 2026-11-17 and reported
"It works amazingly." This confirms an installed interactive launch; the
save/reopen flow, settings location, and MCP gateway were not separately
checked on the Mac. The app is ad-hoc signed, not notarized. No Windows MSI
for RC2. Next after RC2: RC3 / 0.1.3.

## Now

**Resize Settle and robot-claw cursors (2026-11-18):** Settle now uses a
shared core service: rigid world-space support-face rotation, Y=0 contact,
X/Z centering, one undo entry, repeat no-op. Desktop and MCP agree for single
and multiple selections. Open/closed vector claws cover Move/Rotate/Resize,
shortcut dragging, camera orbit/pan and Grab brush. Full gate passed
(`logs/gate/20261118-120020-full`): 521 workspace tests, fmt, Clippy, deny,
audit, MCP parity, smoke and 104 captures. Independent code and both-theme
capture review found no blockers (cross-family review unavailable).
Tasks: [`resize-settle.md`](tasks/resize-settle.md),
[`grab-cursors.md`](tasks/grab-cursors.md). Pack:
`evidence/rebuild/captures/2026-11-18-settle-claw/`. Owner visual review pending.
Release build, smoke and both-theme Release captures passed (23,396,352-byte EXE).
Cargo bloat: 22.3 MiB file / estimated 18.1 MiB code. Recovery: `G:\3D-Modeler_snapshot_20261118-122146_settle-claw`
(additive copy; file/byte counts verified).


**Resize reference toggle (owner trial, 2026-11-18):** icon-only
floor/object switch above Apply, with tooltips. Floor uses fixed floor axes;
Object follows imported local axes as the object turns. Multiple objects use
the first object's frame in model order. Handles, live dimensions and numeric
Apply use the chosen frame; switching does not dirty the document or add Undo.
Full gate passed (`logs/gate/20261118-112226-full`), including 274 app and 57
viewport tests, MCP parity and 96 captures. Release built and smoke passed
(23,402,496-byte executable). Pack: `evidence/rebuild/captures/2026-11-18-resize-basis/`.
Task: [`resize-basis-toggle.md`](tasks/resize-basis-toggle.md).
Built for the owner to try. Recovery: `G:\3D-Modeler_snapshot_20261118-112934_resize-basis-toggle`.

**Rotate grips from owner reference (2026-11-18):** `erros/rotate.png`
sets the grip style: filled double-ended tips joined by a narrower solid
stem. Grips follow short curved arcs; rings stay thin. Mesh depth testing
hides rear strokes and filled grips on GPU and CPU. Placement avoids folded
edge-on grips and favors space between axes. Rotation picking, preview and
history pass. Full gate passed (`logs/gate/20261118-104915-full`), with
271 app and 57 viewport tests and 92 captures. Release built and smoke passed
(23,393,792-byte executable). Both-theme comparisons and Release captures:
`evidence/rebuild/captures/2026-11-18-rotate-reference/`.
Task: [`rotate-reference.md`](tasks/rotate-reference.md). Owner-approved 2026-11-18 ("awesome!"). Done.
Recovery: `G:\3D-Modeler_snapshot_20261118-105552_rotate-reference`.

**Rotation ring finish (owner, 2026-11-18):** continuous shaded tube paths
with soft depth edge and specular glint; curved grips receive matching edge
definition; rings use 96 segments for smoother arcs. Picking and transform
behavior are unchanged. Fast gate passed (269 app + 56 viewport tests); Release
build and smoke passed. Both-theme before/after pack:
`evidence/rebuild/captures/2026-11-18-rotate-finish/`.
**Owner rejected the visual result, 2026-11-18:** "it still look awful (like a child did it)".
This earlier treatment is superseded by the owner-approved reference grips
above. The rotation finish task is closed.
Task: [`rotation-finish.md`](tasks/rotation-finish.md). Recovery:
`G:\3D-Modeler_snapshot_20261118-085850_rotation-finish`.

**Rotate ring occlusion (owner, 2026-11-18):** rear ring strokes and curved
grips clip at the rotated selection silhouette, so no lines cross the object
faces. Picking, visible grips and transform behavior are unchanged. Fast gate
passed for the app and viewport crates (269 + 56 tests); Release build and
smoke passed. Both-theme captures:
`evidence/rebuild/captures/2026-11-18-rotate-occlusion/`.
Task: [`rotation-occlusion.md`](tasks/rotation-occlusion.md).
Superseded by the owner-approved reference grips above. Recovery:
`G:\3D-Modeler_snapshot_20261118-rotate-occlusion-final` (13,104 files,
1,882,744,593 bytes verified).

**Rotation controls follow the object (owner, 2026-11-18):** local rings and
arrow grips follow committed orientation and the live drag. Painting, picking
and drag axes share the same frame; multiple selected objects follow the first
selected object's orientation. Regression covers single/multiple selection,
local-axis picking, preview, cancellation, commit, undo and redo. Independent
diff review found no remaining blockers. Full gate passed (`logs/gate/20261118-071412-full`), including 46 states
in both themes. Release build, smoke and both-theme captures passed. Pack: `evidence/rebuild/captures/2026-11-18-rotate-follow/`.
Task: [`rotate-follow.md`](tasks/rotate-follow.md). Included in the owner-approved Rotate result above.
Recovery: `G:\3D-Modeler_snapshot_20261118-072319_rotate-follow-checkpoint`.


**OF-69 Move arrows (2026-11-17):** flat axis-colored stems and solid
triangular heads consistent with Resize. Axis colors, directions, dimensions,
picking, hover/drag emphasis and gesture/history paths retained. MCP translation
semantics unchanged. Independent code and both-theme capture review found no
blockers (requested reviewer models unavailable; inherited-model review used).
Fast and Full gates passed (`logs/gate/20261117-183431-fast`,
`logs/gate/20261117-183452-full`), including the 44-state, both-theme board.
Release rebuilt (4m14s, 23,369,216 bytes); Release smoke and captures passed.
Pack: `evidence/rebuild/captures/2026-11-17-of69-move/`.
Task: [`move-arrows.md`](tasks/move-arrows.md).
**Done, awaiting owner review. OF-70 remains queued; Rotate approved 2026-11-18.**
Baseline: `G:\3D-Modeler_snapshot_20261117-183334_OF69-baseline`
(12,195 files, 1,770,275,521 bytes verified).
Recovery: `G:\3D-Modeler_snapshot_20261117-184100_OF69`
(additive copy; file and byte counts verified).

**OF-68 rotation arrows (2026-11-17):** original curved grips with solid flat
triangular heads pointing both ways, inspired by the owner reference and Resize.
Axis colors, rings, hover emphasis and existing picking retained; MCP rotation
semantics unchanged. Independent diff review found no remaining blockers after
viewport-edge fixes (requested reviewer models unavailable; inherited-model
independent review used). Fast and Full gates passed, including the 44-state,
both-theme capture board (`logs/gate/20261117-175642-full`); final clipping
changes also passed Fast (`logs/gate/20261117-175825-fast`).
Pack: `evidence/rebuild/captures/2026-11-17-of68-rotation/`.
Task: [`rotation-arrows.md`](tasks/rotation-arrows.md).
Final Release rebuilt; Release smoke and both-theme captures passed.
Recovery: `G:\3D-Modeler_snapshot_20261117-180620_OF68`
(additive copy, file and byte counts verified).
**Owner-approved through the reference Rotate grips, 2026-11-18. OF-69 and OF-70 remain queued.**
Baseline: `G:\3D-Modeler_snapshot_20261117-175442_OF68-baseline`
(11,995 files, 1,734,687,424 bytes verified against the pre-edit tree).


**Resize OF-66 / OF-67 (owner, 2026-11-17):** removed the size label above
Resize; the existing XYZ boxes follow the uncommitted mesh preview in display
units, including all axes affected by corner scaling. Escape, focus loss and
blocked input restore scene sizes. Numeric entry and gesture/history math
remain unchanged. Fast and Full gates passed: 508 workspace tests, fmt,
Clippy, deny, audit, MCP parity, smoke, 44 states x 2 themes
(`logs/gate/20261117-155921-full`). Independent code review found no blockers.
Release rebuilt (4m12s, 23,370,240 bytes); Release smoke passed. Both-theme
before/after and Release captures:
`evidence/rebuild/captures/2026-11-17-of66-67-resize/`.
**Done, awaiting owner review. OF-68 follows below; OF-69 and OF-70 remain queued.**
Feedback: [`owner-feedback-2026-11-17.md`](owner-feedback-2026-11-17.md).
Task: [`resize-live-sizes.md`](tasks/resize-live-sizes.md).
Recovery: `G:\3D-Modeler_snapshot_20261117-160600_OF66-67`
(copy and file/byte counts verified).
Baseline: `G:\3D-Modeler_snapshot_20261117-155214_OF66-67-baseline`
(11,778 files, 1,697,810,506 bytes; counts verified).

**Edit tools independent of selection (owner, 2026-11-17):** all 12 Edit
buttons can be chosen, switched and dismissed with no selection, including an
empty scene, from both the ribbon and favorites. Switching closes previous
controls. Smooth and Boolean tools chosen without a target offer nonmodal
Apply controls; invocation with an existing selection retains its behavior.
Mesh application still validates its target; tool changes add no undo entry.
Full gate passed: 507 tests, fmt, Clippy, deny, audit, MCP parity, smoke and
44 states x 2 themes (`logs/gate/20261117-130618-full`). Tests cover actual
egui clicks on both button surfaces, all tool pairs, clicking off the model,
selection resumption and one-step undo/redo. Both-theme before/after and Release
captures: `evidence/rebuild/captures/2026-11-17-edit-tools/`. Release rebuilt
(4m16s, 23,368,704 bytes); Release smoke passed. **Owner review pending.**
Task: [`edit-tools-selection.md`](tasks/edit-tools-selection.md).
Recovery: `G:\3D-Modeler_snapshot_20261117-131300_edit-tools`.

**Import (owner, 2026-11-17):** menu Import sits above New Scene and opens
an all-files native picker. First use resolves Documents (including Windows
Known Folder redirection); later uses remember the selected folder across
restarts. All eight export formats import: STL, OBJ, PLY, 3MF, glTF, GLB,
WRL and 3DP. Desktop and MCP share one atomic import/placement service.
WRL supports static triangulated IndexedFaceSet scenes, including app exports;
other VRML nodes return an explicit error. Project imports preserve groups,
metadata, textures and visibility with fresh IDs. Final checks passed:
502 tests, fmt, Clippy, deny, audit, MCP parity and smoke
(`logs/gate/20261117-123551-full`); all 40 states x 2 themes captured
separately in `gate-20261117-123113`, including the new menu.
Release rebuilt (4m15s, 23,362,048 bytes), smoke and both menu captures passed.
Live Release checks through the stdio MCP gateway passed all eight formats
(export/import/undo/redo) and failed-import scene/revision preservation.
Task: [`import.md`](tasks/import.md). **Native picker hands-on and owner
visual review pending.** Recovery: `G:\3D-Modeler_snapshot_20261117-124653_import`.


**Bottom bar and interface text (owner, 2026-11-17):** removed the permanent
status bar; Move/Rotate/Resize sits 12 px above the bottom. Active operations
retain a temporary progress/Cancel window. Home slogan is "Imagine, Design,
Play"; corrupted brush punctuation, face-preview arrow and shortcut-order
arrows fixed. Workspace checks passed (`logs/gate/20261117-111144-full`):
498 tests, fmt, Clippy, deny, audit, parity and smoke. The 40-state x 2-theme
capture board passed separately (`gate-20261117-110823`); its initial Clippy
failure was corrected before the final checks. Final slogan/layout captured
again from Release in both themes. Release rebuilt (4m06s, 23,335,936 bytes),
release smoke passed. Task and before/after evidence: [`bottom-text.md`](tasks/bottom-text.md).
**Owner visual review pending.** Recovery: `G:\3D-Modeler_snapshot_20261117-111816_bottom-text`.

**Caption and modifier toggles (owner, 2026-11-17):** Windows title bar follows
the menu theme. Clicking an active brush or modifier again closes its controls,
including after selection is lost. Full gate passed
(`logs/gate/20261117-104840-full`): 497 tests, fmt, Clippy, deny, audit, parity,
smoke, 40 states x 2 themes. Task/captures: [`toolbar-toggle.md`](tasks/toolbar-toggle.md).
Release rebuilt (5m58s, 23,341,056-byte exe) and launched from the active workspace.
Native caption/text setters passed in both themes; native colour readback is
unsupported. **Owner visual review pending.** Recovery:
`G:\3D-Modeler_snapshot_20261117-105524_toolbar-toggle` (robocopy 1;
11,119 files / 1,590,685,157 bytes matched).

**Startup updates (owner, 2026-11-17):** mirror the prompt inside the individual
Craft apps (confirmed in installed CADCraft), checking GitHub at launch and
asking before downloading/installing. Implementation and release preparation:
[`updates.md`](updates.md), task [`startup-updates.md`](tasks/startup-updates.md).
Feed remains unconfigured until the public repository is chosen; configure and
rebuild every platform before the first public build. **Full gate passed**
2026-11-17 (`logs/gate/20261117-101438-full`): 494 tests, fmt, Clippy, deny,
audit (informational warnings), parity, smoke, 38 states × 2 themes. Linux
helper success/invalid-payload/rollback passed in WSL; real stdio client against
an isolated desktop passed (76 tools, unavailable install refused, normal close).
Review pack: `evidence/rebuild/captures/2026-11-17-startup-updates/`, including
the actual CADCraft reference. **Owner visual review pending.** Mac helper and
live Windows/Mac/Linux installed upgrades are not yet validated; do those with
the real feed before public launch. MCP catalog is 76 tools (`2026-11-17.1`).
Recovery snapshot: `G:\3D-Modeler_snapshot_20261117-101915_startup-updates`
(robocopy plus matching source/destination file and byte counts).
Baseline snapshot: `G:\3D-Modeler_snapshot_20261117-094734_pre-updater`
(robocopy 1, 10,114 files / 1,396,721,684 bytes matched). USB Mac commits merged
as `ff8e43e`; DMG copied to `E:\3D-Modeler-RC\3D-Modeler-0.1.2.dmg`, SHA-256
`c90ba253986d89b1a0eba842b367b1db1b6eaf94f847620f3e56bca65d7b600d` matches USB.

**GitHub, 2026-11-16 (owner):** goal is a public repository named "3D Modeler", free to use, donations on; "almost" ready, not uploaded. Commit email chosen; licence (Apache 2.0 suggested) and artwork origin still to confirm. Checklist and open questions: [`github-upload.md`](github-upload.md).

**Bug sweep 2026-11-16 (owner request before the upload):** Full gate passed (all steps, capture board 37 states × 2 themes). New test `crates/modeler3d-core/tests/malformed_inputs.rs`: about 5,000 cut-short or corrupted copies of every importable format and of `.3dp` projects; every one is refused with an error, none crashes (debug and release). Last two `expect()` calls removed from library code (`legacy_settings.rs`). No new bugs found. Still open from before: the faint pale dotted rim at the floor (F9 note) and Emboss (deferred).

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
`set_app_settings` gains `dismiss_settings_notice`. OF-65 (transform strip: the
"N selected / mm" note removed; each Move / Rotate / Resize label centred over
its X / Y / Z box (owner follow-up)) built 2026-11-16, pack
`evidence/rebuild/captures/2026-11-16-of65-transform-strip/`, test
`transform_strip_has_no_count_and_each_tab_is_centred_over_its_box`. Rotate now
shows "0.000 deg" instead of "-0.000 deg" (owner yes). **Owner 2026-11-16:
OF-63, OF-64, OF-65 approved ("love it all call it all done").** Phase 6 is
closed.

**Known work, in order** (a session takes the first item not done):

1. Done and approved 2026-11-15: Phase 3 follow-ups OF-59 to OF-61.
2. Done and approved 2026-11-15: Phase 4 — Measure ruler (OF-55).
3. Done and approved 2026-11-16: Phase 5 — MCP does everything (OF-58).
4. Done and approved 2026-11-16: Phase 6 — OF-63 to OF-65.
5. Done 2026-11-16: F5 release links and F6 ledger reconciliation (fix-first
   list below).
6. Linux and Mac test packages for the owner's own machines, before the
   GitHub release: [`packaging-linux-mac.md`](packaging-linux-mac.md).
   Linux done 2026-11-16 and Mac packaged and owner-run 2026-11-17 (RC2 above).
   Found and fixed on the way (all platforms): MCP
   `frame_view` failed its output check through the gateway (nested `view`),
   and the CPU viewport route never showed its frames (grey viewport on
   software-only adapters).
7. Resize reference toggle (owner, 2026-11-18). Built for owner trial: an icon-only
   switch above Apply for floor-front or imported-object-front sizing, with
   tooltips. Trial: [`resize-basis-toggle.md`](tasks/resize-basis-toggle.md).
8. Resize -> Settle implemented 2026-11-18; shared desktop/MCP placement,
   shape and history regression verified. Owner review pending; see Now.
9. Open/closed robot-claw grab cursors implemented 2026-11-18 throughout the
   app's grab feedback. Both-theme review captures checked; owner review pending.
10. Revisit the Material texture dialog (OF-46 follow-up): the owner wants to
   look at its layout again; ask what to change before building.
11. Stop before RB-10 (S3). OF-56/57 (Emboss) stay deferred until the owner
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

`evidence/rebuild/captures/2026-11-16-of65-transform-strip/` (OF-65) and `evidence/rebuild/captures/2026-11-16-review-phase6/` (OF-63, OF-64), approved by the owner 2026-11-16. `evidence/rebuild/captures/2026-11-15-review-phase5-mcp/` (OF-58), approved
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
| F5 | Release link reliability | **Done** 2026-11-16 — two consecutive forced fat-LTO release links, both clean (3m23s, 3m18s; 64 GB host), exe 21.9 MB. Plan exit met; `lto = "fat"` kept. Earlier: three clean links 2026-11-12; one LLVM out-of-memory 2026-11-11 |
| F6 | Reconcile the 148-row feature ledger with the app | **Done** 2026-11-16 — 69 stale rows (mostly the October RB-00 audit: "no Ctrl+N", "no pan gesture", "text menu button" and so on) checked against the code and named tests and rewritten; owner approvals from this file copied in with the owner's words (`owner_quote`). Code and tests only, nothing re-driven live. Counts in the Ledger section |
| F7 | Clear ad hoc `target/` directories (`E:` had 40 GB free) | **Done** 2026-11-12 — archived to `G:\Builder3D-Rust_target-adhoc-archive_20261112`, `target/` 91 → ~55 GB |
| F8 | `scripts/Invoke-Gate.ps1` | **Done** 2026-11-13 — `-Tier fast\|full\|ship`; ship does not script package staging or the performance run |
| F9 | Depth fight where meshes meet the floor | **Owner-approved 2026-11-14** ("works exactly as I asked"); done 2026-11-13 — depth bias on floor surface, shadows and text; captures `evidence/rebuild/captures/2026-11-13-f9-floor-depth/` (before/after, both themes). Dark serration gone; a faint pale dotted fringe on the rim remains and does not change with the bias or the reflection (cause not found). CPU fallback not touched |
| F10 | `emboss_text` long-running metadata | Decided (D5): keep the flag; progress indicator is OF-57, deferred by the owner 2026-11-14 |
| F11 | Status narrative out of `AGENTS.md` | **Done** 2026-11-12 |
| F12 | MCP 100% desktop parity (plan 3.7) | **Done** 2026-11-13 — Merge color; `rename_object` and `set_visibility` tools (catalog now 62, each with a desktop-vs-MCP test); `launch_slicer` falls back to the saved slicer; ledger `mcp_tool` / `mcp_exempt` column and parity check. Owner 2026-11-14 (D10): no "visual only" exemptions — MCP gets tools for all of them (OF-58) |
| F13 | Run `cargo deny` and `cargo audit` | **Done** 2026-11-12 — both pass; now in the Full gate |

## Ledger (`evidence/rebuild/parity-matrix.toml`, 148 features, reconciled 2026-11-16)

Implementation: pass 142 · partial 5 · not applicable 1. Automated: pass 139 ·
partial 4 · pending 5. Used live: pass 124 · pending 20. Owner-accepted 29
(visual pass on the same 29); 119 rows wait for the owner's
walkthroughs (S2 owner review session 2). Still partial: the five RB-01
look rows (theme foundation, welcome, Items panel, dark palette, artwork
conflicts) — they belong to the S1 finish, not to code gaps. Gaps found:
no test dispatches Center View, Frame or the Snap toggle directly, and none
covers the window-icon load.

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

Git: branch `rebuild/work`, see `git log` (F5/F6: `f0a6cb0`; OF-65: `02fc0ce`, centred tabs `e6e03bf`, 0.000 angles `02dba59`; Phase 6: `1ac2583` OF-63/OF-64; Phase 5: `2f6aaf8` OF-58; Phase 4: `cfc96a7` OF-55,
pack `ff8b28f`, one-press Esc `229f0d2`; Phase 3 follow-ups: `57671fb`
OF-59, `fbbfff8` OF-60, `0238ccf` OF-61; Phase 3: `e95dbd3` OF-48,
`e9aa73d` OF-49, `7248d91` OF-50, `7432d0b` OF-51, `54b6f2a` OF-52,
`e283f15` OF-53 findings, `114a676` OF-54; Phase 2: `654872a` (OF-46), `344e88e` (OF-44),
`82f4a58` (OF-47), pack `06699d2`; Phase 1: `566ba48` to `3288a55`). Tree
snapshot: `G:\3D-Modeler_snapshot_20261116-145640_F5-F6` (10,026
files, 1,338,525,537 bytes, counts verified; excludes `target\` and `logs\`).
Previous: `G:\3D-Modeler_snapshot_20261116-133446_OF65-followups`.

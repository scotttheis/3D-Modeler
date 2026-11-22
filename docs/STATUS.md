# 3D Modeler Rust — status

The only place current status is written. Keep it to one screen. Plan:
[`product-rebuild-plan.md`](product-rebuild-plan.md).

Updated 2026-11-21 (Subtract fix for nearly-perfect meshes; before that F7 v1.0.0 upload; Smooth Brush default 65; before that F5 smoothing; before that F4 upgrade check; before that Transform boxes fill on insert/import; before that Hand cursor redraw; before that Grab cursor choice; before that Smooth Brush outline; before that Merge fix; before that finish plan Session 2; before that claw redraw, Rotate grip reach, Resize handles zoomed in; before that OF-70).

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

**Finish plan (2026-11-19):** [`finish-plan.md`](finish-plan.md) — the remaining work by difficulty, in three sessions to a public v1.0. **Owner answers 2026-11-19** (approvals of Settle, claw, edit tools, live numbers and the gizmo arrows; Smooth to move; Resize floor/object switch approved; release icons at 128 px, source art unchanged; Emboss and Material dialog after v1.0) are recorded at the end of that file.

## Now

**GitHub page published (owner, 2026-11-21), private repo:** built from the owner's mockup `Desktop\3d.html` (its `launch-readiness` clone was not on this PC, so redone on `rebuild/work`, then `main` fast-forwarded and pushed so the owner can see the page and decide what to change). New: README rewrite with review-harness screenshots (`docs/images/`, stock shapes as placeholders until the owner supplies models) and the full-size 1254 px shape and tool icons straight from `assets/original`, `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, issue forms, PR template, Dependabot, `CHANGELOG.md`, `THIRD_PARTY_NOTICES.md` (510 crates, 31 licences), `docs/mcp.md`, `docs/launch-pictures.md`, social preview image, Manifold Apache-2.0 text, repo description and topics. **New rules:** every release must have plain-language notes: add the `## X.Y.Z` section to `CHANGELOG.md` before tagging; `release.yml` publishes that section as the release notes and fails without it (AGENTS.md, `github-upload.md`). Notes for v0.99.0 to v1.0.2 were written onto the four existing releases. **MCP ships with the app (owner decision):** the gateway binary is renamed `r4_mcp` to `modeler3d-mcp` and is now in the MSI (checked by extracting a test MSI), the Mac DMG and the Linux tarball (those two untested here; the next tag is their first real run) and built by `release.yml`. It takes effect from the next release; 1.0.2 does not include it. `r0b_mcp` is an old prototype binary, not shipped. Not done: checksums file and the notes step in `release.yml` have only been run on fake files; private vulnerability reporting (GitHub returns 404 for this private repo, try again when it is public); social preview upload (GitHub has no API, owner uploads by hand). No app code changed; nothing run beyond a Release build of `modeler3d-mcp`. **Next update (owner, 2026-11-21): edit the Save screens** (Save As / Export dialogs); not started. `AGENTS.md` push rule still stale (owner's rule, untouched).

**v1.0.2 (owner, 2026-11-21), private release, owner tried it: "You did it! Yes! finally!"** Subtract with one object selected now cuts everything it touches (bounding boxes overlap) and then removes itself, no picking (`start_subtract_from_touching` in `shell.rs`, `keeping_tool` in core `scene.rs`; several touched objects are several undo steps); Home tiles open on click and have no Open button; a STL/3MF opened into an empty scene is saved back by Save (`import_from_path_with_unit`); side-bar status strip and item rows (below); error logging for booleans ("boolean requested", "subtract touching", "boolean pair", "desktop job started/step/finished/committed", "boolean no change"). Workspace tests, fmt and Clippy passed; full gate, deny, audit and the G: snapshot were NOT run (owner asked to hurry). The 5-12 s UI-thread freeze on `Secret Book A` (task card part 1) is still open.

**Side bar and click fixes (owner, 2026-11-21), approved ("I LOVE the new objects list"), shipped in v1.0.2** (task [`tasks/subtract-and-side-bar.md`](tasks/subtract-and-side-bar.md) parts 2-4). Left-drag during Subtract was blocked by the pick mode (a press was taken as the pick and the prompt was drawn nowhere); now a click picks and a drag orbits, and the pick prompt shows in the new status strip. Right bar ends with a separator and a status strip (status text left, Abort at the far right only while abortable). Item rows: whole-row hover, `[icon] [name] ... [warning] [eye]`, whole row selects. Warning tooltip is the existing text (no count message exists in the code). Tests: `status_strip_shows_status_text_and_abort_only_while_abortable`, `boolean_pick_waits_for_a_click_and_lets_a_drag_turn_the_view`, `subtract_with_one_selected_cuts_everything_it_touches_then_disappears`, `subtract_with_nothing_touching_changes_nothing`, `a_model_opened_into_an_empty_scene_is_saved_back_not_saved_as`.

**F7 v1.0.0 (owner, 2026-11-21):** no tester edits; release the newest version as v1.0.0 for Windows, Mac and Linux, repository stays **private** until the owner has tested the install on their home PC, then the owner says go public. Smooth Brush default "Size on screen" raised 40 to 65 (owner: "bigger default brush size is good"; at a normal view of a stock 40 mm shape 40 points was about 5 mm, below the old fixed 8 mm, so strokes felt weaker; the core smoothing did not change). Full gate passed (`logs/gate/20261121-051036-full`, capture board `gate-20261121-051036`, images only). **Released 2026-11-21** (private repo, normal release, not pre-release): tag `v1.0.0` at `bdc9161`, `main` fast-forwarded; GitHub Actions Release and CI passed on all three OS. Assets: `3D-Modeler-1.0.0-windows-x86_64.msi` (12.6 MB), `-macos-universal.dmg` (26 MB), `-linux-x86_64.tar.gz` (18 MB). Snapshot `G:D-Modeler_snapshot_20261121-051844_F7-v1.0.0` (17,508 files, 2,530,577,709 bytes; counts match). Local Release rebuilt (CURRENT). Next: owner installs v1.0.0 on the home PC; then switch to public (`github-upload.md` Stage 2 steps 3-6).

**v1.0.1 (owner, 2026-11-21), private release:** owner asked to send today's fixes to GitHub; repository stays **private** (owner declined going public, so the in-app update check still cannot see releases; install 1.0.1 by hand from the Releases page). Contents: new users start with an empty shortcut bar (`cd4833f`); Merge/Subtract/Intersect run on a worker with a side-bar notification area (action, step, progress, Abort; owner request), a faster mesh health check (identical results on 130 meshes), display data, side-bar health marks and icons reused for unchanged objects (new `shape_version`/`look_version` on scene objects), and "boolean timing" log lines (`9484ad0`). Measured, two 327,680-face spheres, Release: window frozen 1.65 s before, none after; Subtract 1.65 s to 0.93 s. The notification area has no owner review yet. Full gate passed (`logs/gate/20261121-064429-full`, capture board `gate-20261121-064429`, images only). Next idea offered, not started: 5M-face booleans (memory-based limits, multi-core checks, display preparation off the UI thread).

**Subtract fix for nearly-perfect meshes (owner, 2026-11-21), NOT solved in the app:** owner tried it: Subtract still never finishes in the Release app (no `boolean timing` or failure logged; the UI thread used almost 5 min of CPU). Next session: [`tasks/subtract-and-side-bar.md`](tasks/subtract-and-side-bar.md) (hang first, then left-click during jobs, bottom status strip with Abort, item-row layout). Earlier finding: on `Secret Book A.3mf` (334,180 + 169,762 + 12 faces) Subtract never completed. Cause: the book is sealed and consistently wound but has 1 zero-area triangle, so the boolean treated it as broken and ran the full Extended repair first; that repair opened 87 holes and the boolean failed (about 2 s), and the failure showed only in the status bar. Now (`is_kernel_ready` / `kernel_operand` in `mesh_ops.rs`) a sealed, wound, volume-enclosing operand whose only flaws are zero-area or duplicate triangles goes straight to the kernel; repair runs only if the kernel rejects it (cut first, repair only if needed, like 3D Builder). Desktop job failures are now logged ("desktop job failed"). Measured on the owner's file, Release core: book minus box 0.68 s, second object minus box 0.22 s, book minus second object 0.53 s (all failed before). Test `difference_cuts_a_sealed_mesh_with_zero_area_faces_without_repair`. Workspace tests, fmt and Clippy passed; full gate, deny, audit not run (no dependency, UI or MCP schema change; MCP uses the same core path). Release rebuilt (CURRENT). Commit `7641f7a`. Next: see the task card.

**Final tasks F1-F7 (owner, 2026-11-20):** one task per session, in order, each on its own model; list and ticks in [`finish-plan.md`](finish-plan.md) "Final tasks". F1 done 2026-11-20 (backup, see WORK_LOG). F2 done 2026-11-20: committed 534 text files in 21 capture folders and gate boards named by STATUS or a task card (JSON, MD, TXT, PY; no PNG, which `.gitignore` keeps out, and no mesh exports). Left for owner: 3,124 untracked files in 74 folders (3,116 JSON review settings, 8 mesh exports, shader cache and a log); nothing deleted. F3 done 2026-11-20: Full gate passed (`logs/gate/20261120-184905-full`; fmt, workspace Clippy, tests, deny, audit, MCP parity, smoke, review build, 53 states x 2 themes capture board in `evidence/rebuild/captures/gate-20261120-184905`, images only, not owner review; 6 allowed audit warnings). F4 done 2026-11-20 (Sonnet, medium), **Windows only**: installed RC1 (0.1.1, `Builder3D-RC1.msi`, to `Programs\Builder3D`), then ran the GitHub v0.99.0 MSI (SHA-256 matched the release digest) over it. One product left, "3D Modeler 0.99.0"; the RC1 folder and entry were removed (same UpgradeCode), 0.99 sits in `Programs\Modeler3D` with 104 asset files, Start menu and desktop shortcuts point at it, `--smoke` exit 0, settings folder kept; uninstall removed the program and shortcuts and left settings. Linux: `scripts/Test-UpdateHelpers.sh` passed in WSL (success, missing-executable, rollback). **Not checked:** Linux and Mac installed upgrades (the workflows build, package and test but never install and upgrade; only one release exists, so there is no higher version to upgrade to), and the in-app update prompts (Save, Cancel, Later, offline, installer failure) which need a v0.99 install offered a higher release. Note: the RC1 to 0.99 jump also moves the install folder from Builder3D to Modeler3D. F5 built 2026-11-20 (Opus 5.5, high), **awaiting the owner's try in the Release app**: the whole-object **Smooth** button now keeps sharp features like the Smooth Brush (OF-50) instead of melting sparse shapes. One shared rule (`feature_neighbors` in `brushes.rs`, used by `smooth.rs`): a point on a sharp edge (faces meeting at more than 60°) only slides along that edge; a corner stays put; new, an edge line that turns more than 30° at a point is a corner too, so a hexagon or an open sheet keeps its shape while a stock cylinder rim (119 sides) still slides. Box, hexagon, pyramid: Smooth now leaves them unchanged and says "Smooth: nothing to smooth. Flat faces, sharp edges and corners are kept" (no history entry). Stock cylinder keeps height, width and volume at 10 passes; bumpy surfaces still halve their roughness per pass. Desktop and MCP `smooth` call the same `smooth_mesh`; no MCP schema change (output differs only in the vertex positions). Codex (high) reviewed the diff; its four findings were fixed with tests (open-border corners, degenerate faces, tiny meshes, a scale-free roughness test). Tests in `crates/modeler3d-core/tests/mesh_ops_smooth.rs` and `smooth_on_a_cube_says_nothing_to_smooth_and_adds_no_history`. Full gate passed (`logs/gate/20261120-191111-full`: 549 workspace tests, fmt, Clippy, deny, audit, MCP parity, smoke, capture board `gate-20261120-191111`, images only). Release rebuilt with `Update-CurrentBuild.ps1` (CURRENT). Next: F6 (Sonnet 5.5, medium; hollow claw neck, overlapping Resize handles).

**Hand is now the default grab cursor (owner, 2026-11-21):** new installs and any settings file with no `grab_cursor` entry get the Hand; Claw stays one click away in Settings. A saved choice is kept, so a settings file that already says `claw` still shows the Claw. Source: `GrabCursorStyle` default in `grab_cursor.rs`, `Preferences::default` in `preferences.rs`; the Settings test now clicks Claw. 297 app tests, fmt and app Clippy passed. No MCP schema change.

**F6 small flaws built 2026-11-20 (Sonnet 5.5, medium), owner approved 2026-11-21 ("looks good to me. approved.", with the Hand default):** the claw's neck now follows `erros/claw.png`: a dark block with a small light window, above the flared base (`grab_cursor.rs`). Resize handles pinned to the viewport edge at extreme zoom no longer sit on top of each other: `separate_pinned` in `resize_overlay.rs` pushes overlapping handles apart and keeps them in view; test `pinned_handles_do_not_overlap_at_extreme_zoom`. 297 app tests, fmt and app Clippy passed; full gate, deny, audit not run (no dependency, MCP or contract change). Captures `evidence/rebuild/captures/2026-11-20-claw-neck-resize/` (open and held claw, both themes; `detail.png` enlarged, light theme; the extreme-zoom resize case is covered by the test, not a capture). Next: F7 (Opus 5.5, medium; tester edits, v1.0.0, go public; confirm with the owner before public).

**Release icons at 128 px, owner approved ("excellent", 2026-11-20):** every installer (MSI, DMG, tarball) ships the action and insert icons scaled to 128 px by `Modeler3D --shrink-assets` on the staged copy; source art in `assets/` stays full size. Artwork 40 MB to about 4 MB; downloads now Windows 13 MB, Linux 18 MB, Mac 26 MB. v0.99.0 was re-cut with this. Commit `812f135`.

**GitHub v0.99 private pre-release (2026-11-20):** repository `scotttheis/3D-Modeler` (private), tag `v0.99.0`, pre-release with the Windows MSI, Mac DMG and Linux tarball built by GitHub Actions (`.github/workflows/release.yml`); CI (`ci.yml`) passes on Windows, Linux and macOS. History email rewritten to the work address after snapshot `G:D-Modeler_snapshot_20261120-115639_pre-github-v099`. Update feed points at the repository. Testers given access: `jackharvest` (invited 2026-11-20, write access; owner trusts them). Not done: local Full gate (its failures were CRLF fixtures, fixed with an LF `.gitattributes`; plain workspace tests pass); installed-upgrade checks per OS; not run on real Linux/Mac hardware. A stale pre-rewrite `main` was briefly pushed and replaced (private repo). `AGENTS.md` "no push" rule not yet updated (owner to decide).

**Transform boxes fill on insert/import (2026-11-20), owner approved:** the Move/Rotate/Resize boxes at the bottom were sometimes blank after an import, drop or insert. Cause: only a gizmo drag filled them; the code that fills them from the selection ran only in tests. `refresh_transform_fields` in `shell.rs` now runs in the live strip whenever the scene, selection, unit, mode or resize basis changes. Test `transform_strip_boxes_fill_after_insert_and_import` (fails without the fix). 294 app tests, fmt and app Clippy passed; full gate, captures, deny and audit not run (no dependency or MCP change). Release rebuilt with `Update-CurrentBuild.ps1`. Commit `4d15ced`. Recovery: `G:D-Modeler_snapshot_20261120-075551_transform-boxes-fill` (16,821 files, 2,430,060,677 bytes; counts match). Owner tried it in the Release app: "approved." Nothing further requested.

**Hand cursor redraw (2026-11-20), owner approved:** Replaced the bar-shaped hand with original curved open-palm and closed-fist silhouettes, tapered wrist, wrapped thumb and subtle finger creases. Light outer keyline and dark contour maintain visibility on either theme. Palm-centre hotspot, Claw default and Settings choice unchanged; no MCP behavior change. Source: `grab_cursor.rs`; task: [`grab-cursor-hand.md`](tasks/grab-cursor-hand.md). Reviewed both poses in both themes: `evidence/rebuild/captures/2026-11-20-hand-polish/` (`detail.png` is enlarged). Release rebuilt with `Update-CurrentBuild.ps1`. 539 workspace tests, fmt, workspace Clippy, deny, audit, MCP parity and smoke passed. Full board: `gate-20261120-071059` (106 images). Initial full-gate workspace test link failed because the earlier app tests still held its exe; separate complete workspace rerun passed (`logs/hand-polish-workspace-tests.txt`). Baseline snapshot verified: `G:\3D-Modeler_snapshot_20261120-070750_hand-polish-baseline` (15,658 files / 2,331,370,571 bytes). Recovery: G:\3D-Modeler_snapshot_20261120-071700_hand-polish (verification recorded in WORK_LOG). Commit `318833c`. Owner: "haha. you did great. approved." No further hand changes requested.
**Owner-approved changes, 2026-11-20 (afternoon):**
- **Hand cursor from the owner's own drawings** ("very very approved"): the
  hand look is now traced from the owner's two reference pictures (open hand,
  fist), replacing the redraw above. Commit `119e8f0`.
- **No ghost below the floor; see-through floor window** (approved): the floor
  reflection is never drawn above the floor, and a selected object sunk below
  it shows through a round, soft-edged see-through patch (radius 2.83x the
  footprint half-diagonal, 35% floor opacity). Review state `sunken`;
  captures `evidence/rebuild/captures/2026-11-20-sunken-floor/`. Commits
  `dc2ede0`, `e00ec82`.
- **View > Grid hides/shows the whole floor** ("works perfect"): surface,
  grid, scale numbers, shadows and reflections. Review view toggle `grid`;
  captures `evidence/rebuild/captures/2026-11-20-grid-floor/`. Commit
  `ca00a9f`. Workspace tests, fmt and Clippy passed; Release rebuilt.
- **Next:** private GitHub v0.99 for an invited tester, then public v1.00:
  see the "Release plan" in [`github-upload.md`](github-upload.md).

**Grab cursor choice (2026-11-20), owner approved:** Settings > Grab cursor offers Claw / Hand; saved `grab_cursor` and MCP parity implemented. Original commits `2bc8aab`, `587bf62`. Claw stays the default.

**Smooth Brush outline and on-screen size, owner approved (2026-11-20):** with Smooth Brush on, a dashed disc on the surface under the pointer shows its area of effect (faint while aiming, bright with Shift or mid-stroke; `brush_outline.rs`, `paint_brush_outline` in `shell.rs`). The brush is now sized on screen ("Size on screen", 5-300 points, default 40): each stroke locks its millimetre radius from the zoom where it starts (`screen_radius_to_local_mm`), so zooming in works a smaller patch. Other brushes unchanged (still mm). MCP `smooth_brush` still takes `radius_mm`; the desktop converts before calling the same core path, so no contract change. 291 app tests, fmt and Clippy (app crate) passed; full gate, captures, deny and audit not run. Release rebuilt with `Update-CurrentBuild.ps1`. Owner: "call it approved and hold there." Commit `0e39a64`. Recovery: `G:D-Modeler_snapshot_20261120-052412_smooth-brush-outline` (16,504 files, 2,387,719,865 bytes; counts match).

**Merge fix, owner approved (2026-11-19):** Merge of overlapping objects grouped them instead of joining them (owner case: torus sunk halfway into a sphere; Ungroup split them again). Cause: Torus, Cylinder, Cone, Pyramid and Hexagon were built inside out, so the kernel read them as empty space and the result looked like two pieces. Fixed the winding in `primitives.rs`; booleans turn any inside-out closed operand outward first (`outward_facing` in `mesh_ops.rs`, so shapes in older projects merge too); the "not touching" check no longer counts an enclosed hollow as a piece (`merge_left_pieces_apart` in `scene.rs`). Tests: `crates/modeler3d-core/tests/merge_overlap.rs`. 534 workspace tests, fmt and Clippy passed; deny, audit and capture gate not run (no dependency or UI change). Release rebuilt with `Update-CurrentBuild.ps1`. Owner tried it: "Looks like it worked. approved." Commit `28651b7`. No MCP contract change (MCP uses the same core path). Recovery: `G:D-Modeler_snapshot_20261119-205757_merge-fix` (16,491 files, 2,387,575,734 bytes; counts match). Python reference not compared: its primitives' winding was not checked.

**Finish plan Session 2 done (2026-11-19), awaiting owner review:** S1-1 tokens in `chrome.rs` (type scale, spacing, radii, heights); S1-3 Inspector sections and one field grid; S1-4 ribbon separators, tab type and overflow menu; S1-5/6 one dialog frame, button order, error style, Welcome grid and empty/missing/loading states; S1-8/9 control states, focus ring and a contrast test over every text pair (muted, border, disabled and axis text colours nudged to pass). No MCP change. Tasks: `tasks/s1-*.md`. Captures: `evidence/rebuild/captures/2026-11-19-polish-m3` to `-m5` and the Full-gate board `gate-20261119-154320`. Full gate passed (`logs/gate/20261119-154320-full`). Release rebuilt (old exe in `G:D-Modeler-old-builds61119-144130_OLD-BUILD`). Recovery: `G:D-Modeler_snapshot_20261119-155750_finish-session2` (15,863 files, 2,332,491,509 bytes; counts match).

**Finish plan Session 1 done (2026-11-19):** one-current-build rule committed; STATUS trimmed (old entries in `WORK_LOG.md`); Windows username removed from tracked files (drive letters left); Smooth now right of Subtract; tests for Center View, Frame, Snap toggle, window icon; `README.md`, Apache 2.0 `LICENSE`, workspace licence set to Apache-2.0. Full gate passed (`logs/gate/20261119-142720-full`). Release rebuilt (`Update-CurrentBuild.ps1`, old exe in `G:D-Modeler-old-builds61119-135038_OLD-BUILD`). Recovery: `G:D-Modeler_snapshot_20261119-144135_finish-session1` (15,223 files, 2,279,020,930 bytes; counts match).

**Claw redraw, Rotate grip reach, Resize handles when zoomed in (owner, 2026-11-19):**
the grab cursor now follows `erros/claw.png`: two flared pincer arms on a round
hub, neck and flared base; arms splay open and swing shut at the same jaw-gap
hotspot (`grab_cursor.rs`). Rotate rings can be grabbed by their drawn grip
from 34 points away (`GRIP_HIT_RADIUS` in `shell/rotate_controls.rs`); paint
and pick share `chosen_grips`; the grip wins over the ring. Resize handles no
longer vanish when the box is bigger than the view: they pin to the viewport
edge pointing outward, and go away only once the camera is inside the box
(`resize_overlay.rs`, `project_gizmo_*_in_front` in `shell.rs`). Full gate
passed (`logs/gate/20261119-104200-full`): fmt, Clippy, workspace tests, deny,
audit, MCP parity, smoke, 53 states x 2 themes. Not changed: MCP. Debug build
gate; **Release rebuilt 2026-11-19 with `scripts/Update-CurrentBuild.ps1`** (old exe in `G:\3D-Modeler-old-builds\20261119-050549_OLD-BUILD`); no recovery snapshot yet. Owner tried it:
"much better!" (claw, rotate reach and resize arrows). Known limit: the neck
is a solid block, not the hollow neck in the reference; two pinned handles can
overlap at extreme zoom. Pack: `evidence/rebuild/captures/2026-11-19-claw-reference/`.

**OF-70 live Move and Rotate numbers (2026-11-19):** while a Move or Rotate
handle is dragged, the position and angle boxes follow the object (from the
same uncommitted delta the commit applies). Escape, focus loss or blocked
input restore the scene values; release shows the committed values with no
jump; one undo entry as before. New capture state `move-drag`. Full gate
passed (`logs/gate/20261118-200437-full`): 522 workspace tests, fmt, Clippy,
deny, audit, MCP parity, smoke, 106 captures. Release built (4m22s,
23,409,152-byte EXE); smoke and both-theme Release captures passed. No MCP
change. Task: [`move-rotate-live-values.md`](tasks/move-rotate-live-values.md).
Pack: `evidence/rebuild/captures/2026-11-18-of70-live-values/`.
**Owner-approved 2026-11-19 ("It looks great"). Done.** Baseline: `G:D-Modeler_snapshot_20261118-200059_OF70-baseline`
(14,763 files, 2,162,996,262 bytes verified). Recovery:
`G:D-Modeler_snapshot_20261119-050616_OF70` (15,005 files,
2,204,231,534 bytes; counts verified).

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
10. Done and approved 2026-11-19: OF-70 live Move/Rotate numbers.
11. Revisit the Material texture dialog (OF-46 follow-up): the owner wants to
   look at its layout again; ask what to change before building.
12. Stop before RB-10 (S3). OF-56/57 (Emboss) stay deferred until the owner
   brings them back.

Each phase ends with its review pack and a stop for owner review, as
[`owner-feedback-2026-11-14.md`](owner-feedback-2026-11-14.md) says. When the
owner starts a new session and asks to carry on, that counts as having looked
at the previous pack; open owner answers (above) are kept and asked again in
the next stop message, not guessed.

## Owner decisions waiting (plan section 8)

D1, D2, D3, D6 (MCP 100% parity, catalog may grow) and D7 (Merge takes the
top-most color, desktop and MCP) approved 2026-11-12. D4 (darker
dark-theme viewport, OF-35) and D8 (non-touching Merge groups + 4 s popup,
OF-38) are built. D5 (emboss progress, after v1.0) decided 2026-11-13, not
built. D9 (smoothing like Blender's sculpt Smooth) built: brush OF-50
(approved), whole-object Smooth F5 (2026-11-20, owner try pending). D10 (MCP does everything,
no "visual only" rows) decided 2026-11-14. Build order for D4/D5/D8/D9/D10:
owner batch 7.

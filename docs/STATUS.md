# 3D Modeler Rust — status

The only place current status is written. Keep it to one screen. Plan:
[`product-rebuild-plan.md`](product-rebuild-plan.md).

Updated 2026-11-20 (Transform boxes fill on insert/import; before that Hand cursor redraw; before that Grab cursor choice; before that Smooth Brush outline; before that Merge fix; before that finish plan Session 2; before that claw redraw, Rotate grip reach, Resize handles zoomed in; before that OF-70).

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

**Transform boxes fill on insert/import (2026-11-20), owner approved:** the Move/Rotate/Resize boxes at the bottom were sometimes blank after an import, drop or insert. Cause: only a gizmo drag filled them; the code that fills them from the selection ran only in tests. `refresh_transform_fields` in `shell.rs` now runs in the live strip whenever the scene, selection, unit, mode or resize basis changes. Test `transform_strip_boxes_fill_after_insert_and_import` (fails without the fix). 294 app tests, fmt and app Clippy passed; full gate, captures, deny and audit not run (no dependency or MCP change). Release rebuilt with `Update-CurrentBuild.ps1`. Commit `4d15ced`. Recovery: `G:D-Modeler_snapshot_20261120-075551_transform-boxes-fill` (16,821 files, 2,430,060,677 bytes; counts match). Owner tried it in the Release app: "approved." Nothing further requested.

**Hand cursor redraw (2026-11-20), owner approved:** Replaced the bar-shaped hand with original curved open-palm and closed-fist silhouettes, tapered wrist, wrapped thumb and subtle finger creases. Light outer keyline and dark contour maintain visibility on either theme. Palm-centre hotspot, Claw default and Settings choice unchanged; no MCP behavior change. Source: `grab_cursor.rs`; task: [`grab-cursor-hand.md`](tasks/grab-cursor-hand.md). Reviewed both poses in both themes: `evidence/rebuild/captures/2026-11-20-hand-polish/` (`detail.png` is enlarged). Release rebuilt with `Update-CurrentBuild.ps1`. 539 workspace tests, fmt, workspace Clippy, deny, audit, MCP parity and smoke passed. Full board: `gate-20261120-071059` (106 images). Initial full-gate workspace test link failed because the earlier app tests still held its exe; separate complete workspace rerun passed (`logs/hand-polish-workspace-tests.txt`). Baseline snapshot verified: `G:\3D-Modeler_snapshot_20261120-070750_hand-polish-baseline` (15,658 files / 2,331,370,571 bytes). Recovery: G:\3D-Modeler_snapshot_20261120-071700_hand-polish (verification recorded in WORK_LOG). Commit `318833c`. Owner: "haha. you did great. approved." No further hand changes requested.

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
OF-38) are built. D5 (emboss progress, after v1.0) and D9 (smoothing like
Blender's sculpt Smooth, finish plan H1) decided 2026-11-13, not built. D10 (MCP does everything,
no "visual only" rows) decided 2026-11-14. Build order for D4/D5/D8/D9/D10:
owner batch 7.

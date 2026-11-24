# 3D Modeler Rust — status

The only place current status is written. Keep it to one screen. Plan:
[`product-rebuild-plan.md`](product-rebuild-plan.md).

Updated 2026-11-23 (Repair R7; before that Repair R6; before that Repair R5; before that Repair R4; before that Repair R3; before that Hide Emboss; home Open File tile; before that Subtract fix for nearly-perfect meshes; before that F7 v1.0.0 upload; Smooth Brush default 65; before that F5 smoothing; before that F4 upgrade check; before that Transform boxes fill on insert/import; before that Hand cursor redraw; before that Grab cursor choice; before that Smooth Brush outline; before that Merge fix; before that finish plan Session 2; before that claw redraw, Rotate grip reach, Resize handles zoomed in; before that OF-70).

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

**Where things stand (2026-11-24):** 3D Modeler is **public** on GitHub (`scotttheis/3D-Modeler`), releasing **v1.0.5** (orientation fix) after v1.0.4. Donations on through Ko-fi; GitHub billing checked ($0 owed). Owner to-dos: social preview upload, GitHub Sponsors/Discussions, tester access, reword the `AGENTS.md` no-push line. Release steps: [`github-upload.md`](github-upload.md).

**v1.0.5, models open the right way up (owner, 2026-11-24):** owner's 3D Builder file (`Secret Book A.3mf`) opened tipped on its side. Cause: STL, 3MF, OBJ and PLY are Z-up (3D Builder, slicers) and the scene is Y-up, and files were read and written with no axis change. Fix in core (`Transform::z_up_file_to_scene` / `scene_to_z_up_file`, used by `three_mf.rs` and `file_io.rs`): turned upright on open, back on save, lossless round trip; glTF, WRL and `.3dp` unchanged. Desktop and MCP both go through it. Regression test with a lid-on-base 3MF; golden tests now read the Python fixtures as Z-up files. Known: Blender OBJ defaults to Y-up and will now open tipped (matches 3D Builder; owner told). Full gate passed (`logs/gate/20261124-073124-full`, capture board images only). Tag `v1.0.5` pushed 2026-11-24; the Mac build was cancelled once for lack of GitHub Mac machines, rerun with `gh run rerun --failed`, then all four jobs passed and v1.0.5 published as Latest (msi, dmg, tar.gz, SHA256SUMS; notes show).

**Notes from the owner's other PC (2026-11-24):**
- **v1.0.4 "Failure" email was only billing.** The publish job was blocked because the repo was still private and the free monthly Actions minutes ran out (Mac counts 10x, Windows 2x). The three builds finished; v1.0.4 is published, marked Latest, with the `.msi`, `.dmg`, `.tar.gz` and `3D-Modeler-SHA256SUMS.txt` (published by hand). **Money owed: $0** (November usage $12.49, all covered by the free allowance). Spending limit is $0, so GitHub stops jobs instead of charging: **keep it at $0**. Public repos build free, so this should not recur. If a publish job fails again: `gh run rerun <run-id> --failed`. PayPal is not working on the owner's side and is not used.
- **Donations are on (Ko-fi).** Page `https://ko-fi.com/scotttheis`, payouts through Stripe (no PayPal); Ko-fi takes 0% on one-time donations, card fees still apply. Owner's wording: "help keep the updates and new features coming", **not** "buy me a coffee". Commit `3bd4282` on GitHub `main` added `.github/FUNDING.yml` (`ko_fi: scotttheis`) and the README `## Support` section (between Contributing and Licence). Repo setting Sponsorships is ticked, so the Sponsor button shows; GitHub lists the Ko-fi link. Nothing in the app asks for money. Merged into this PC's copy 2026-11-24.
- **Possible later (owner decides):** GitHub Sponsors (0% fee; apply at github.com/sponsors, needs Stripe and a tax form; once approved add `github: scotttheis` to `.github/FUNDING.yml`). Bitcoin only with a new address used just for donations, never the main wallet. A "Support" link in the app's About/Help screen changes the look, so it needs the owner's one-by-one approval ([`community-suggestions.md`](community-suggestions.md)).
- The notes asked to also update `Projects.md`; there is no `Projects.md` on this PC, so they are recorded here only.

**v1.0.4 and going public (owner, 2026-11-24):** owner asked to note that Repair uses the Windows built-in engine on Windows and our own, less thorough engine on Mac and Linux (README known limits, CHANGELOG), then to go public; owner chose to release 1.0.4 first so the public download has the new Repair. 1.0.4 = Repair R1-R7, Open File on the home page, Emboss button hidden. Full gate passed (`logs/gate/20261124-060611-full`, capture board images only). Tag `v1.0.4` pushed; GitHub built all three installers, but the publish job was refused by GitHub billing (private-repo minutes used up), so the release was published by hand from those same build files with `gh release create` (notes from CHANGELOG, `3D-Modeler-SHA256SUMS.txt`). **Repository public 2026-11-24.** Private vulnerability reporting turned on. Checked logged out: front page, latest release, MSI download, security policy and README images all load; the anonymous releases API returns v1.0.4 with SHA-256 digests, so the in-app update check now works. Left for the owner: social preview upload, Sponsors/Discussions, tester access (`jackharvest`). `AGENTS.md` GitHub section updated; its "Standing boundaries" no-push line is still stale (edit blocked by the session's safety check; owner to decide).

**Save screens / no more hanging on file windows (owner, 2026-11-23):** owner's real complaint was that automated runs hang on the native file window until it is clicked; the Save As and Export screens themselves are fine (owner: "there is nothing wrong"). Fix: during a review capture (`MODELER3D_REVIEW_CAPTURE` set) every native picker (Save/Export, Open project, Import and its Browse, image, texture, slicer) answers "cancelled" at once (`native_pickers_blocked` in `dialogs.rs`); normal use unchanged. Full gate passed (`logs/gate/20261123-222442-full`) and the capture board now runs all 53 states x 2 themes (it used to stop at Save). Release rebuilt, CURRENT. Before captures (Export dialog; Save As is the native window, not capturable): `evidence/rebuild/captures/2026-11-23-save-screens/before/`. No after captures: nothing visible changed. No Codex worker or task card (a seven-line guard). No G: snapshot yet. Redesign of the Save screens is not wanted unless the owner asks again.

**Repair (owner, 2026-11-22), not fixed, plan ready:** Repair can make models worse (book: 87 holes opened). Evaluation: ours fixed 3 of 10 test models and damaged 3; the Windows built-in engine (same as 3D Builder) fixed 10 of 10. Owner answered all decisions. Work runs in phases R1-R7 from [`tasks/repair-plan.md`](tasks/repair-plan.md); findings in [`tasks/fixrepair.md`](tasks/fixrepair.md). **R1 done (2026-11-22):** Windows engine behaviour measured through its public API only, written in [`tasks/repair-spec.md`](tasks/repair-spec.md) (snap grid 0.002 mm, weld 0.01 mm, min-area hole fill, union of all parts, tiny parts under 0.1 mm removed, time about F^1.28, 1M faces 128 s); no product code changed. Found: Windows fills every cavity, even a correct one (hollowed models turn solid), and an unwelded model with gaps over 0.01 mm comes back almost empty. Owner decided: keep correct cavities (Repair keeps hollow models hollow; Windows route skips them). **R2 done (2026-11-22):** Repair can no longer make a model worse: a before/after defect score in core gates desktop and MCP, an unhelpful repair shows "Repair couldn't improve this model. Nothing was changed." with no undo entry, thin triangles are no longer deleted, holes sharing a corner now close, and inside-out models are flagged and fixed. Fast gate passed; on the generated test models 4 are fixed, 4 refused unchanged, none worse. Release rebuilt. **R3 done (2026-11-23):** on Windows, Repair now calls the Windows engine itself (Printing3DModel.RepairAsync) with progress, cancel and a time limit, and falls back to our own engine if it fails, collapses the model or the model has a correct cavity; the result says which engine ran. On the test models and the book it fixed all of them (book 12.7 s). Full gate passed (captures not run); Release rebuilt (+45 KB). **R4 done (2026-11-23):** our own engine (Mac, Linux and the Windows fallback) now snaps points to a 0.002 mm grid, collapses thin triangles without opening holes, welds open edges within 0.01 mm, splits T-junctions and separates non-manifold edges and loose flat fins; on the book it now fixes the model (0 holes, points moved at most 0.0017 mm) instead of refusing, and f5, f7, f8 are fixed too; none worse. Fast gate passed. Not shipped (no Release rebuild; R6 ships). **R5 done (2026-11-23):** our own engine now fills holes of any size with no new points (smallest-total-area patch, flat holes stay flat), so the open cylinder, the holey sphere and a 500-edge hole are fixed; the book takes 2.3 s (was 5.2 s) with identical output. Fast gate passed. Not shipped (R6 ships). **R6 done (2026-11-23):** our own engine now merges overlapping or nested parts into one solid (Manifold union; the two overlapping boxes come back as one solid of exactly 15,000 mm3) and keeps correct cavities, so hollow models stay hollow and can now be repaired; parts that only touch stay separate. Health now checks self-intersections (book +0.26 s), so self-crossing models get the needs-repair marker. All other test models give exactly the same output as before; book 2.8 s; the Windows route still fixes all. Not resolved natively: one part crossing itself (detected, refused). Full gate passed (`logs/gate/20261123-170536-full`, capture board not run); Release rebuilt. Snapshot `G:D-Modeler_snapshot_20261123-171440_repair-R6` (18,191 files, 2,583,718,483 bytes, counts match). **R7 done (2026-11-23), owner approval pending:** Repair now says what it did in plain words, the same on desktop and MCP ("Repaired: closed 1 hole." / "Repaired: closed 87 holes and removed 1 zero-size triangle. Used the Windows repair engine."); refused and nothing-to-fix keep their messages; the dialog text is plain words; the needs-repair marker is tested for inside-out and self-crossing models; the test models are now core tests built in code; MCP golden test added. Captures both themes, fixed and refused: `evidence/rebuild/captures/2026-11-23-repair-r7/`. Full gate passed (`logs/gate/20261123-202434-full`, capture board not run). Release rebuilt. Snapshot `G:D-Modeler_snapshot_20261123-203632_repair-R7` (18,245 files, 2,588,981,660 bytes, counts match). Owner tried the book in the Release app (2026-11-23): "Repaired: removed 1 zero-size triangle. Used the Windows repair engine." (correct: the book is sealed with one zero-area face). Book looks unchanged (owner: yes). A downloaded broken model: "Repaired: fixed 2 places where the model passed through itself. Used the Windows repair engine." **Owner approved Repair 2026-11-23: "Approved for now. I may want to revisit it, but call it done."** Repair work is closed; reopen only if the owner asks.

**Home Open File tile (owner, 2026-11-22):** added between New Scene and Open
Project, using the existing Add/Open native file picker and import flow.
Tiles wrap in the same order on narrow windows. Fast gate passed
(`logs/gate/20261122-063623-fast`): fmt, app Clippy, 305 app tests passed,
2 existing tests ignored. Before/after home captures in both themes and
after captures with Big UI reviewed:
`evidence/rebuild/captures/2026-11-22-home-open-file/`. Big UI stacks the
tiles in the existing scroll area. Current Release rebuilt, smoke passed,
and both-theme home captures verified. The Full capture-board limitation
at Save is recorded below; no dependencies or model behavior changed.
Task: [`home-open-file.md`](tasks/home-open-file.md). Owner visual approval
pending. Recovery: `G:\3D-Modeler_snapshot_20261122-064200_home-open-file`.

**Hide Emboss (owner, 2026-11-22):** removed the Edit ribbon button until the
feature works correctly; implementation retained for later restoration.
Updated the existing ribbon availability test from 12 to 11 buttons.
Fast gate passed (`logs/gate/20261122-063301-fast`); all other workspace
tests, workspace Clippy, fmt, deny, audit, MCP parity and smoke passed in
`logs/gate/20261122-062801-full`. That Full run initially failed the old
button-count assertion, then its capture board was stopped at Save because
the native Save picker waits for input (14 images; remaining board not run).
Before/after Edit captures in both themes reviewed:
`evidence/rebuild/captures/2026-11-22-hide-emboss/`. Current Release rebuilt.
Task: [`hide-emboss-button.md`](tasks/hide-emboss-button.md). Owner visual
approval pending. Baseline snapshot verified:
`G:\3D-Modeler_snapshot_20261122-062640_hide-emboss-baseline`
(17,627 files / 2,570,001,107 bytes).
Recovery snapshot verified: `G:\3D-Modeler_snapshot_20261122-063535_hide-emboss`
(17,681 files / 2,576,309,343 bytes); local commit `8d01d8f`.

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
- **Next (done 2026-11-24):** private v0.99, then public; now public at v1.0.4.

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
12. Owner request 2026-11-21, DONE and approved ("FIXED! I love it"), released in v1.0.3: paste lands where the moved original
   was; Settle drops straight down from the current spot:
   [`paste-in-place-and-settle-down.md`](tasks/paste-in-place-and-settle-down.md).
13. Stop before RB-10 (S3). OF-56/57 (Emboss) stay deferred until the owner
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

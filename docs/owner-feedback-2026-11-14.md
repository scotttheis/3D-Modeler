# Owner feedback — 2026-11-14 (seventh batch)

The owner's hands-on review of the 2026-11-13 review pack, plus new requests.
Items are numbered OF-28 onward and grouped into phases. Work the phases in
order and the items within a phase top to bottom. Status of each item is kept
in the table rows here; `STATUS.md` only points at this file and names the
current phase.

## How to work this file

- One item (or one named pair) per task, one commit per verified task on
  `rebuild/work`, subject ending in the item id, e.g. `(OF-31)`.
- Every desktop change gets its matching MCP change in the same task (D6,
  D10). If an item has no MCP side, say why in the row.
- Anything visible: before/after captures in both themes with the review
  harness (plan 3.5), saved to `evidence/rebuild/captures/2026-11-14-of<NN>-<slug>/`.
  Add a harness state when an item needs one.
- Run the Fast gate per task (`scripts/Invoke-Gate.ps1 -Tier fast`), the Full
  gate at the end of each phase.
- At the end of each phase: rebuild the release exe, put the phase's captures
  in one review pack with a `README.md` (one line per item: what to look at),
  update `STATUS.md`, take a `G:\` snapshot, and **stop for owner review**.
  Do not start the next phase until the owner has looked.
- Write the owner a short message: the exe path, the pack path, and a
  numbered list of what to try. No engineering detail unless asked.
- Mark a row done only as "Done, awaiting owner review". Only the owner's own
  words move it to approved.
- Where the owner names 3D Builder or Blender as the model, read the owner's
  reference images in `erros/` first. If there is no image of the behaviour,
  describe the reference behaviour in the row before building it.

## Owner reference images (in `erros/`, not in Git)

| File | Shows |
|---|---|
| `paint dialogue.png` | 3D Builder's paint mode: a top bar with Cancel, Color (tick), a "Selected" colour swatch with drop-down, and a Coverage slider |
| `XRayObject.png` | 3D Builder's X-ray: the unselected sphere is pale and see-through, the selected cube stays solid and fully coloured with its edges outlined |
| `Cube1 light-mode cube icon.png` | The new light-mode Cube icon (copied from the owner's Downloads, 1254 px, transparent background) |

## Approved in this review (owner's words, 2026-11-14)

| Item | Owner said |
|---|---|
| OF-1 floor (look) | "The floor is fantastic, I love it!" (number placement: see OF-31) |
| F9 floor under object | "Floor under object works exactly as I asked it to." |
| OF-1 resize view / OF-3 | "Resize arrows are nice, but need to double in size" (size: OF-32) |
| OF-2 shadow | "Shadow looks great." |
| OF-6 wireframe | "Works as it is … count it as done for now." May revisit after testers. |
| OF-9 Merge color | "Merges appears to work as I wanted." (Ungroup bug: OF-37) |
| OF-8 Smooth, Smooth Brush, Blob | "seem to be working well." (Grab, Crease: OF-48, OF-49) |
| Intersect, Subtract | "seem to work great." |
| F12 / D10 | MCP must do everything the desktop does, including theme, recents, ribbon and shortcuts (OF-58). Replaces the "visual only" list. |

Not yet answered: **OF-4 top-bar hover** — the owner did not know what it
meant. In the next pack, explain it in one line ("buttons and tabs light up
when the pointer is over them") with the capture beside it.

## Phase 1 — Quick fixes and polish

Small, independent, mostly visible. One review pack at the end.

| # | Item | Owner's request | Notes | Status |
|---|---|---|---|---|
| OF-28 | No console window | "I don't want a background cmd window." | Release exe opens a console because `main.rs` has no `#![windows_subsystem = "windows"]`. Keep a way to see logs (log file / `--console` flag) for diagnostics; check the MCP stdio launch path still works. | Done, awaiting owner review. Release exe is windowed; terminal modes (--smoke, --package, captures) still print; `--console` opens a console and mirrors the log (log file always at `%LOCALAPPDATA%\Builder3D\logs`). No MCP side: startup only, and the MCP gateway is its own console program. |
| OF-29 | App icon on the exe | Make an icon from the logo PNG and use it instead of the generic .exe icon. | `crates/builder3d-app/assets/original/Builder3D.png` and `Builder3D.ico` exist; embed the .ico as a Windows resource at build time (license check with `cargo deny`), and the window/taskbar icon too. Check the MSI shortcuts use it. | Done, awaiting owner review. The logo .ico (16-256 px) is embedded in the exe (build script, `embed-resource`, MIT; deny and audit pass). Window/taskbar icon and MSI shortcuts already used the logo. Images: `evidence/rebuild/captures/2026-11-14-of29-app-icon/`. No MCP side: file icon only. |
| OF-30 | New light-mode Cube icon | Use `Cube1.png` for the light-mode Cube icon. | Source: `erros/Cube1 light-mode cube icon.png`. Replace `assets/original/insert_icons_light/Cube.png` (ribbon, favourites, Items list wherever the light Cube icon appears). Keep the dark set unchanged. Size it like its neighbours (S3 notes the icons are oversized; don't add another 1.2 MB file if a smaller export looks the same). | Done, awaiting owner review. Light `insert_icons_light/Cube.png` replaced with the new cube, cropped to the same fill as its neighbours and exported at their 256 px (80 KB, was 86 KB). Dark set unchanged. Captures `evidence/rebuild/captures/2026-11-14-of30-light-cube-icon/`. No MCP side: artwork only. |
| OF-31 | Floor numbers on the snap corners | Numbers should sit at the edge where snapping happens, at the corner of two squares. Today they sit mid-square on one side and on the line of the other. | Align each label's anchor to the grid intersection its distance refers to. | Done, awaiting owner review. Every label now starts just past the 50 mm grid corner it names (where the distance line meets the axis line), on both rows. Captures `evidence/rebuild/captures/2026-11-14-of31-floor-number-corners/` (framed and top-down). No MCP side: floor drawing only. |
| OF-32 | Resize arrows twice as big | "need to double in size" | Double the chevron handles and their hit area together. | Done, awaiting owner review. Chevrons, their gap from the box and their grab area are all 2x. Corner dots unchanged. Captures `evidence/rebuild/captures/2026-11-14-of32-resize-arrows/`. No MCP side: on-screen handles only (MCP resizes by value). |
| OF-33 | Grab cursor on every handle | Hover over any object arrow/handle: open hand. Press: closed hand. | Move, Rotate and Resize handles. egui `CursorIcon::Grab` / `Grabbing`. No MCP side (pointer only). | Done, awaiting owner review. Open hand over any Move arrow, Rotate ring or Resize handle; closed hand while one is held; normal pointer elsewhere and when a panel covers the viewport. The cursor is not in captures: tests `move_and_rotate_handles_show_the_open_hand` and the resize handle test; hands-on check in the pack. No MCP side (pointer only). |
| OF-34 | Subtle selection highlight | Selecting an object changes its colour drastically. It should be subtle, same colour family, "maybe just a bit darker". | Keep an outline if it reads well; capture several colours. | Done, awaiting owner review. Selected objects keep their own colour, 18 % darker (was a 38 % mix toward bright blue, which turned red purple). Same rule on the GPU and CPU paths. No outline yet: 3D Builder outlines the selected silhouette; that outline is built with X-ray (OF-47), which needs it too. New harness state `colors`. Captures `evidence/rebuild/captures/2026-11-14-of34-selection-shade/`. No MCP side: display only. |
| OF-35 | Darker viewport in dark theme (D4) | "a bit darker, but not so much as full dark mode" | Floor, grid, fog and numbers in the dark theme only; light theme unchanged. Offer two shades in the pack. | Done, awaiting owner review. Dark theme only: background, floor, grid lines and floor numbers darkened together (one setting, GPU and CPU paths); light theme unchanged. Shade A (84 %, built in) and shade B (70 %, darker) in `evidence/rebuild/captures/2026-11-14-of35-dark-viewport/`; owner picks. No MCP side: display only. |
| OF-36 | Movable dialogs | The Paint dialog (and others) don't move when dragged by the title bar. | Every dialog/popover with a title bar becomes draggable and stays on screen. | Done, awaiting owner review. Every dialog (Paint/Material, Repair, Print Prep, Torus, Hollow, Save, Export, Open, Import, Unsaved changes, Confirm overwrite, Mirror/Align/Arrange, Dimensions, About) opens centred, drags by its title bar and stays on screen; the last four gained a title bar for that. Settings stays docked between the bars as approved (OF-15 to OF-19). Test `dialogs_open_centred_and_move_by_their_title_bar`; harness states `mirror`, `align`. Captures `evidence/rebuild/captures/2026-11-14-of36-movable-dialogs/`. No MCP side: window placement only. |
| OF-37 | Ungroup after Merge | After Merge there is one object, but Ungroup still looks clickable. | Ungroup enabled only when the selection contains a group. Same rule in MCP. | Open |
| OF-38 | Merge of parts that don't touch (D8) | Group them and show a small ~4 s popup that the objects were not touching. | Desktop toast + MCP result message. Toast style reused by later items. | Open |
| OF-39 | Repair result readable | In dark mode the result text is dark blue and unreadable; it should be the normal text colour. The button should be **Close**, not Cancel. | Check every result dialog for the same colour bug. | Open |
| OF-40 | Repair with nothing to fix | Say plainly that the model was already fine and nothing was changed. | E.g. "No problems found — your model was already clean. Nothing was changed." No undo entry (no-op rule). MCP result says the same. | Open |
| OF-41 | Mirror axis colours | Colour X, Y and Z in the Mirror dialog to match the arrows. | Use the same axis colour tokens as the gizmo. | Open |
| OF-42 | Align and Arrange | Need 2+ objects selected; axis letters in the arrow colours. | Disabled (with a hint) under 2 objects; MCP returns a clear error. | Open |
| OF-43 | Paste and Duplicate placement | Put the copy 10 mm to the side of the original, other axes the same. Don't ask which axis. | "Side" = the screen-right axis of the default (loading) view. Repeat pastes keep stepping 10 mm. MCP same. | Open |
| OF-45 | Remove the Color button | "get rid of the color button for now. we can evaluate using it for something later." (2026-11-14) | Remove the desktop button (ribbon, favourites, any menu entry). If an MCP tool exists only for image-colour sampling, remove it too so desktop and MCP stay equal; keep the core code if other features use it. Ledger row updated. | Open |

## Phase 2 — Paint, Color, Material, X-ray (3D Builder model)

| # | Item | Owner's request | Notes | Status |
|---|---|---|---|---|
| OF-44 | Paint like 3D Builder | Choose a colour, get a paint-can (spray nozzle) cursor, click the object to colour it. | Reference `erros/paint dialogue.png`: top bar with Cancel, Color, "Selected" swatch + drop-down, Coverage slider. Coverage = whole object vs. region; face/region painting is part of this item. One undo entry per click. MCP: paint object / faces by id with a colour. | Open |
| OF-46 | Material = textures | Material should let you import a texture and apply it. Grab some free samples to try. | Larger feature: texture import, UV mapping on primitives/meshes, `.3dp` and export implications (3MF supports textures; STL does not). Use only CC0 samples (e.g. ambientCG, Poly Haven) and record their licences. If this changes the `.3dp` format, stop and ask first. May split into its own phase if big. | Open |
| OF-47 | X-ray like 3D Builder (OF-5) | Mostly transparent objects; the selected one(s) stay visible. | Reference `erros/XRayObject.png`: unselected objects pale and see-through, selected solid with an edge outline. Closes OF-5. | Open |

## Phase 3 — Sculpt and mesh tools

Compare each against its reference before building (Blender for brushes; the
owner's description where none exists). Item 13 of the earlier list (Blender
brush comparison) is done as part of OF-48 to OF-50.

| # | Item | Owner's request | Notes | Status |
|---|---|---|---|---|
| OF-48 | Grab = pinch and pull | Grab "smooshes" the object. It should feel like pinching the surface and pulling it in the direction the mouse moves. | Model: Blender's Grab / Snake Hook — vertices under the brush follow the pointer with smooth falloff. | Open |
| OF-49 | Crease, subtle | Crease slams in too deep and breaks the model. It should be subtle and move more surrounding surface to form the crease. | Model: Blender's Crease brush — gentle depth, wider falloff, pinch toward the stroke line. Never produce self-intersections or holes. | Open |
| OF-50 | Smoothing like Blender (D9) | Match Blender's default sculpt Smooth (the brush Shift switches to). | Fixes the "sparse stock cylinder melts" report. | Open |
| OF-51 | Simplify | "does not seem to do anything." Wants a simple drop-down with a slider removing about 10 % of faces per notch. | Find why it's a no-op first (likely no change on low-poly primitives). Show the face count before → after. MCP takes the same percentage. | Open |
| OF-52 | Subdivide | Feels uneven. Drop-down: whole object or selected faces. | Even subdivision across the whole mesh; selected-faces option. MCP same options. | Open |
| OF-53 | Normalize | "seems to do nothing or I have no idea how to use it." | Find out which. Either fix it or make the use obvious (hint text, visible brush area). Report back before redesigning. | Open |
| OF-54 | Hollow resource limit | Hollow failed: "exceeded its resource limit … above the 2,000,000 unit limit". Suggests simpler geometry inside the hollow. | Make hollow work on normal models: simplified inner shell, a better work estimate, or raise the limit after measuring. Error text in plain words if it still can't. | Open |

## Phase 4 — Measure

| # | Item | Owner's request | Notes | Status |
|---|---|---|---|---|
| OF-55 | Measure ruler | Measure should give a ruler you can run across objects, not a dialog. | Click-to-click on surfaces, live distance with axis components, snaps to vertices/edges. MCP: measure between two points/objects. | Open |

## Phase 5 — MCP does everything (D10)

| # | Item | Owner's request | Notes | Status |
|---|---|---|---|---|
| OF-58 | No "visual only" exemptions | "The AI MCP should be able to do anything." Theme, recent files, ribbon, shortcuts — "100 % yes. All of it." So a user can customise the app by talking to an AI. | Add tools for every ledger row now marked `mcp_exempt = "visual only"` (theme, display units, recents list/open/clear, ribbon pin/favourites/order, shortcuts, sticky selection, About, exit with the unsaved check, …). Remove the exemptions; the parity check then requires a tool for every row. A desktop-vs-MCP test per tool. | Open |

## Deferred — not now

The owner deferred these on 2026-11-14 ("don't do emboss right now"). Do not
work them until the owner brings them back.

| # | Item | Owner's request | Notes | Status |
|---|---|---|---|---|
| OF-56 | Emboss like 3D Builder | "Not working … not doing its job remotely like 3D Builder." No reference screenshots coming. | Known not doing its job. | Deferred |
| OF-57 | Emboss progress (D5) | Show a progress bar, or a working spinner in Builder3D's style if the time can't be estimated. | Keep MCP `longRunning: true`. If another long operation (e.g. Hollow, OF-54) needs a busy indicator first, build the reusable one there. | Deferred |

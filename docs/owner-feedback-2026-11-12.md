# Owner feedback — 2026-11-12 (second batch)

Raw owner requests, in the order given. Reference for "how it should look/feel"
is Microsoft 3D Builder unless an item names something else (Blender, the
Python build at `L:\Builder3D`). Each item becomes a task card before work
starts; record approved deviations from Python parity in `STATUS.md`.

| # | Area | What the owner wants | Reference | State |
|---|---|---|---|---|
| OF-1 | Viewport measurements | Remove the floor ruler numbers ("50 mm … 1600 mm" crowding into the distance, `erros/numbers.png`) — not wanted even when objects are out there. Show measurements the way 3D Builder does: premium-looking dimension tags on the selected object only. | 3D Builder | **Redo done 2026-11-12, awaiting owner review** (`6fbb5b7` floor numbers, `53a45f1` resize view). `e4c62b1` had removed the floor labels and added badge tags — both wrong per the 3D Builder captures. (a) Floor numbers: back, painted flat on the floor plane at 50 mm steps, foreshortened and depth-tested (2026-11-12, awaiting owner review; captures `evidence/rebuild/captures/2026-11-12-of1-floor-numbers/`). (b) Resize view: white screen-aligned box with corner dots, green/blue double chevrons, one grey size label, pale drag band (2026-11-12, awaiting owner review; captures `evidence/rebuild/captures/2026-11-12-of1-resize-view/`). |
| OF-2 | Contact shadow | The shadow under objects looks generic. Replace it with a convincing, grounded shadow. | 3D Builder | Done 2026-11-12, awaiting owner review: neutral grey soft shadow — tight darker contact shadow under the footprint plus a wider cast shadow away from the key light, one smooth penumbra gradient instead of 12 banded layers — and a faint floor reflection that fades with depth (off under X-ray and from below the floor). Commit `f7e19aa`. GPU path only; the CPU fallback keeps its layered shadow and has no reflection. Captures `evidence/rebuild/captures/2026-11-12-of2-contact-shadow/`. |
| OF-3 | Resize arrows | The arrows in Resize mode must look and behave like 3D Builder's scale handles. (Extends the 2026-11-11 "arrows feel cheap" item, S1 task 7.) | 3D Builder | Covered by the OF-1 resize-view redo (2026-11-12, `53a45f1`): double-chevron scale handles like 3D Builder's; awaiting owner review. |
| OF-4 | Top-bar hover | Hovering over a command or shortcut in the top bar must show a visible hover state. | 3D Builder | Done 2026-11-12 (`8797405`), awaiting owner review: tabs show a darker tile on the tab bar on hover, toolbar and top-bar buttons a visible light-grey tile (captures `evidence/rebuild/captures/2026-11-12-of4-topbar-hover/`). |
| OF-5 | X-ray | X-ray is too extreme and makes objects disappear. It should work like 3D Builder's: objects stay readable and you can see through them. | 3D Builder | Open — skipped until the owner provides shots (X-ray with two objects). |
| OF-6 | Wireframe | Wireframe should be a true wireframe like Blender's (clean edges over the model, not a shaded approximation). | Blender | Done 2026-11-12 (`fc70679`), awaiting owner review: GPU wireframe pass — anti-aliased constant-width triangle edges over the shaded model, depth-tested, every object complete (the old CPU overlay was truncated at its vertex budget); dense regions fade instead of going black. Meshes over 2 M faces get no edges (budget). Captures `evidence/rebuild/captures/2026-11-12-of6-wireframe/`. |
| OF-7 | Paint / Color / Material | Paint, Color and Material currently do nothing. They must apply to the selection (and paint to faces/regions). | 3D Builder | Fixed 2026-11-12 (`89c1e6f`), awaiting owner review. Cause: Material, Paint and Color were disabled (greyed, silent) whenever nothing was selected, and with a selection all three opened one bare RGBA dialog whose slider rails were invisible (theme drew them white on white). Now, as in the Python build: they stay clickable whenever the scene has objects and say "Select an object to paint"; Material/Paint open a color dialog with basic swatches and an HSV picker (one undo entry); Color samples an image's average color straight from a file picker. Still open: painting individual faces/regions (no Python reference; needs the owed Paint shots). Captures `evidence/rebuild/captures/2026-11-12-of7-of8-paint-edit/`. |
| OF-8 | Edit-mode sculpt tools | Grab, Crease and the other newer tools under Edit no longer work. Restore them. The owner liked them in the Python build and likes Blender's versions even more. | Python Builder3D, Blender | Fixed 2026-11-12 (`89c1e6f`), awaiting owner review. Cause: Grab, Crease, Smooth Brush, Blob and Normalize were disabled whenever nothing was selected (Python keeps them enabled and says "Select a mesh to use Grab"); the same invisible-slider theme bug hid their brush settings. The stroke itself (Shift+drag on the mesh, as in Python) works and commits one undo entry — now covered by a test. Not yet compared hands-on against Blender's brushes. |
| OF-9 | Merge color | Merging objects should give the result one color: that of the top-most selected object in the right-hand object list. | — | Done 2026-11-12 (`38d6437`), awaiting owner review: a desktop Merge result drops per-face source colors and takes the color of the operand higher in the Items list, in the same single undo entry. MCP booleans unchanged (they name target/tool explicitly). Note: Merge still joins exactly two objects (pre-existing). Captures `evidence/rebuild/captures/2026-11-12-of9-merge-color/`. |

## Batch summary (2026-11-12)

Every item except OF-5 is committed on `rebuild/work`, one commit each. Each
commit passed `cargo fmt --check`, `cargo clippy --workspace --all-targets
-- -D warnings` and `cargo test --workspace` (181 app tests at the end), and
has before/after captures in both themes. **None is owner-reviewed yet.**

| Commit | Item | Captures (`evidence/rebuild/captures/`) |
|---|---|---|
| `6fbb5b7` | OF-1 floor numbers | `2026-11-12-of1-floor-numbers/` |
| `53a45f1` | OF-1 resize view, OF-3 | `2026-11-12-of1-resize-view/` |
| `8797405` | OF-4 top-bar hover | `2026-11-12-of4-topbar-hover/` |
| `fc70679` | OF-6 wireframe | `2026-11-12-of6-wireframe/` |
| `38d6437` | OF-9 Merge color | `2026-11-12-of9-merge-color/` |
| `f7e19aa` | OF-2 shadow and reflection | `2026-11-12-of2-contact-shadow/` |
| `89c1e6f` | OF-7, OF-8 | `2026-11-12-of7-of8-paint-edit/` |
| `fe784f4` | Recovery snapshot recorded | — |

### What changed, where

- **Floor numbers** (`builder3d-viewport/src/floor.rs`, the renderer's
  `floor_text` pipeline, `builder3d-app/src/floor_labels.rs`):
  - egui glyph meshes are mapped onto the floor plane in two rows: one along
    X (at z = 24) and one along Z (at x = 24).
  - Labels sit every 50 mm, the text is 5.5 mm tall, and each axis shows at
    most 40.
  - Labels fade out as their on-screen letter size shrinks from 3.5 px to
    1 px.
  - The GPU path samples egui's font atlas (`set_text_atlas`). It is
    depth-tested but does not write depth.
  - The CPU fallback paints the same mesh through egui.
- **Resize view** (`builder3d-app/src/resize_overlay.rs`):
  - One layout drives both painting and hit-testing, so a handle responds
    exactly where it is drawn.
  - A handle whose axis points nearly at the camera (alignment above 0.8) is
    hidden on the far side and drawn on the face on the near side.
  - The old dimension tags and arrow helpers are removed.
- **Top-bar hover** (`theme.rs`, `render_tabs` in `shell.rs`):
  - Tabs are custom-painted with a `navigation_hover` tile.
  - The button hover color is `#E4E7F0` in the light theme and `#1B3A66` in
    the dark theme.
- **Wireframe** (the renderer's `WIREFRAME_SHADER` and `GpuWire`):
  - Each object gets a cached copy of its mesh with per-vertex barycentric
    coordinates.
  - The shader anti-aliases edges with `fwidth` and uses invariant positions
    to avoid z-fighting with the shaded model.
  - Edges fade in dense regions. Meshes over 2 M faces get no edges.
  - The buffers are tracked in the resource ledger.
  - The GPU path no longer uses the truncated CPU overlay wireframe.
- **Merge color** (`Scene::apply_boolean_with_color` in `builder3d-core`,
  `apply_boolean_pair` in `commands.rs`):
  - The result drops the per-face colors it inherited from the two objects.
  - It takes the color of whichever object is higher in the Items list,
    applied by a `SetColorCommand` in the same grouped undo entry.
- **Shadow and reflection** (the renderer's `push_soft_polygon` and
  `REFLECTION_SHADER`):
  - The shadow is a core polygon plus a soft edge (quads along the edges and
    fans at the corners).
  - Alpha is 0.30 for the contact shadow and 0.17 for the cast shadow.
  - The reflection mirrors objects about y = 0 at alpha 0.20 and fades with
    depth.
- **Paint, Color, Material and the sculpt tools** (`command_enabled` in
  `shell.rs`, `dialogs.rs`):
  - They are enabled whenever the scene has objects. With nothing selected
    they say "Select an object to paint" or "Select a mesh to use Grab".
  - Material and Paint get 16 basic swatches plus an HSV picker.
  - Color uses an injectable image picker, which tests stub out.
  - Theme fix: inactive slider rails use `surface_raised` with a trailing
    fill, so every slider is visible.

### Known limits and follow-ups

- Painting individual faces or regions is not implemented. The Python build
  has no reference for it, so it waits for the owner's Paint, Color and
  Material shots.
- The sculpt brushes have not been compared hands-on against Blender's.
- The CPU fallback renderer has neither the floor reflection nor the new
  soft shadow. It does use the new object-shaped outline (follow-up below).
- Merge still joins exactly two objects; 3D Builder joins every selected
  object. The result keeps the name of the object clicked first.
- Shots still owed by the owner: X-ray with two objects, Paint/Color/Material,
  Merge.

### Lessons from this batch

- The floor alone used 24,576 overlay vertices, so a "skip if past 3/4 of the
  budget" guard silently dropped every shadow. Check each cue against what
  it actually needs instead of using a blanket threshold.
- Scripted `mouse_event` input against our own app is unreliable: a lost
  button release left the camera orbiting and the window maximised. For
  pointer behaviour, prefer unit tests that feed real egui input through the
  viewport, plus the harness's `POINTER` and `INTERACTIVE` options.
- A native `rfd` file dialog blocks `cargo test` indefinitely. Any command
  that opens a picker must take the picker as an injectable function.
- PowerShell wrappers eat `-p`, `-D` and `--` from forwarded arguments. Pass
  the whole cargo argument string as one quoted argument.
- A restored file copy can keep its old modification time, and cargo then
  skips rebuilding it. Touch the file after restoring.

## Follow-up: shadow shape, below-floor view, drag (2026-11-12, uncommitted)

Owner asked, after reviewing the app live:

1. The square floor shadow should take the object's shape, or be removed if
   that is expensive.
2. Looking up through the floor, the shadow (and floor) should be gone.
3. A dragged object's shadow stayed at the old place until the move
   committed; it should vanish at drag start and return once the object lands.

The owner confirmed in the app that 3 is "way better". A shadow that follows
the moving object would be the ideal; hiding it is the accepted interim.
1 and 2 are not yet owner-reviewed. No review captures were made.

What changed (`builder3d-viewport` only):

- **Shape** (`floor.rs`: `silhouette_points`, `ContactCue::silhouette`):
  - The cause of the square: the shadow was projected from the 8 corners of
    the object's world bounding box.
  - When the cue cache settles, each object's display mesh is sampled and
    the extreme point along each of 122 support directions is kept. The
    sampling is capped at 200,000 vertices per scene, at least 2,048 per
    object.
  - Per frame, those points are projected along the key light and hulled.
    The contact polygon is their top-down hull. The bounding box remains the
    fallback for degenerate meshes.
  - The outline is convex, so concave objects lose their holes: a torus
    casts a disc, an L-shape a filled shape. An exact shadow would need a
    second draw of every mesh per frame; not done.
- **Budget** (`renderer.rs`): `MAX_OVERLAY_VERTICES` raised from 32,768 to
  65,536, because the floor alone takes 24,576. `limit_outline` thins each
  outline to 8–48 vertices according to the room left, so a crowded scene
  gets coarser shadows before it loses any.
- **Below the floor** (`renderer.rs`, `cpu.rs`): shadows and floor text are
  now gated on `FloorGeometry::visible`, as the floor surface and reflection
  already were. Before, shadows and floor labels still drew from below.
- **Drag** (`renderer.rs`: `cues_for_settled_objects`): each frame, a cue is
  dropped if its object's current world bounds differ from the cached ones by
  more than 0.001 mm, or if the object is the translation-preview target.
  This covers mouse and gizmo drags and any other previewed change. Other
  objects keep their shadows, and camera motion hides nothing.
- **Cost:** per frame, about 120 projected points and a 2D hull per object,
  plus a bounds check. The cache rebuild runs only when the scene revision
  changes. It is estimated at under 1 ms for normal scenes and about 10 ms at
  the 200,000-vertex cap; neither figure has been timed.
- **Tests added:** `round_objects_cast_round_shadows_not_bounding_squares`,
  `shadows_vanish_with_the_floor_when_the_camera_is_below_it`,
  `a_moving_object_has_no_shadow_until_it_settles`. The viewport crate
  passes (48 tests), with clippy `-D warnings` and fmt clean for the crate.
  The full workspace gate was not run.
- **Release build:** `cargo build --release -p builder3d-app` succeeded
  twice this session (4m50s, 3m43s). Relevant to F5.

Open from this follow-up: a shadow that tracks the dragged object; holes in
concave shadows; measured rebuild timings; owner review of 1 and 2; commit.

## Third batch (2026-11-12) — top bar and box select

Raw owner requests, in the order given. Worked in this order, one commit each.

| # | Area | What the owner wants | State |
|---|---|---|---|
| OF-10 | Shortcut bar | Remove the description and the name under each pinned shortcut; icon only. The tooltip is the tool's name. | **Owner-approved 2026-11-12** in the release build ("you did amazing. i say yes to it all"). Done: 36 px icon-only buttons, tooltip and accessible name = the command name; right-click still removes. captures `evidence/rebuild/captures/2026-11-12-of10-14-topbar/`. |
| OF-11 | Slicer button | The "3D Print" button becomes **Slicer**. First click: choose a slicer; it is remembered and every click after that exports straight to it with no confirmation. Changing it only happens in Settings. | **Owner-approved 2026-11-12** in the release build ("you did amazing. i say yes to it all"). Done: no saved slicer → a chooser under the button (installed slicers + Browse); the pick is saved and sent to at once. Saved slicer → stage visible objects and launch, no dialog (`launch_when_staged` job). Missing saved slicer → status says to change it in Settings. Settings keeps Change/Clear. Tests `slicer_button_*`. |
| OF-12 | Save buttons | The two top-right icons are **Save** and **Save As**, labelled as such and doing exactly that. | **Owner-approved 2026-11-12** in the release build ("you did amazing. i say yes to it all"). Done: icon + "Save" / "Save As" text buttons (Ctrl+S / Ctrl+Shift+S). |
| OF-13 | Box select | Shift + press on the workspace floor (empty space) and drag draws a selection box; objects inside it are selected. | **Owner-approved 2026-11-12** in the release build ("you did amazing. i say yes to it all"). Done: Shift + press on empty space (no object, no gizmo handle) starts a blue box; on release every visible object whose on-screen footprint touches the box is **added** to the selection (Shift adds, as Shift-click does). A box under 4 px is a plain Shift-click. No history entry. Shift on an object or handle keeps its old meaning. Test `shift_box_selects_objects_it_covers_without_history`. Not captured (the harness has no mid-drag state); hand-checked by the owner. |
| OF-14 | Theme toggle | A light/dark toggle in the top bar, between the Slicer button and the Save buttons. | **Owner-approved 2026-11-12** in the release build ("you did amazing. i say yes to it all"). Done: pill switch, knob shows sun (light) or moon (dark); saved to settings. Test `theme_toggle_flips_the_mode_and_saves_it`. |

Owner approved all five, and the choices made for them (box select adds to the
selection; touching the box is enough), on 2026-11-12. The same "yes to it all"
approved the offered follow-up: MCP `launch_slicer` may use the saved slicer
when no `slicer_key` is given (not started; next task).

MCP parity (plan 3.7): OF-10, OF-12, OF-14 are visual/shell only. OF-11 uses the
existing slicer handoff that MCP `launch_slicer` already exposes. OF-13 is a
pointer gesture; its result (a selection set) is what MCP `select` already
sets, so no new tool.

## Fourth batch (2026-11-12) — theme and Settings polish

Raw owner requests, in the order given.

| # | Area | What the owner wants | State |
|---|---|---|---|
| OF-15 | Light-mode top bar | In light mode the main-menu (top bar) background should be a little darker, just enough to make the icons pop. | Done 2026-11-12, awaiting owner review: light top bar `#E8EAF5` → `#DCDFEE` (tab hover `#D0D5EA` → `#C9CFE6`); the menu button's hover tile now uses the bar hover colour. Dark theme unchanged. captures `evidence/rebuild/captures/2026-11-12-of15-19-settings-topbar/`. |
| OF-16 | Theme sync | After switching to dark with the top-bar toggle, Settings still showed Light checked. Settings must show the current theme. | Done 2026-11-12, awaiting owner review. Cause: Settings is non-modal, so it could stay open while the toggle changed the theme, and it kept the theme it was opened with. Settings now re-reads the current theme every frame. Tests `settings_follows_a_theme_changed_elsewhere`, `open_settings_shows_the_theme_picked_with_the_top_bar_toggle`. |
| OF-17 | Settings dismiss | Clicking away from Settings closes it. | Done 2026-11-12, awaiting owner review: a mouse press anywhere outside the Settings panel closes it. Not while one of its drop-down lists or About is open. Test `pressing_outside_settings_closes_it_and_inside_does_not`. |
| OF-18 | Settings buttons | Settings has both Done and Close; keep only one. | Done 2026-11-12, awaiting owner review: removed "Done"; the single "Close" button stays at the bottom. Test `settings_has_one_close_button`. |
| OF-19 | Menu button | The hamburger (pancake) menu icon sits higher than the menu text beside it; centre it vertically. | Done 2026-11-12, awaiting owner review. Cause: the tabs sat in a nested row that centred lower than the menu button. Tabs now share the menu button's row, so the icon and the tab text centre on the same line (the bar is also ~4 px shorter). captures `evidence/rebuild/captures/2026-11-12-of15-19-settings-topbar/`. |

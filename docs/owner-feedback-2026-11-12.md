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
  soft shadow.
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

# RB-02: First Working Application Slice Handoff

**Prepared:** 2026-10-31  
**Status:** Conditional RB-02 execution handoff; implementation has not started.  
**Current gate:** RB-01 matched live captures and explicit owner approval remain pending.

This handoff turns the RB-02 row in the [product rebuild plan](product-rebuild-plan.md)
into a bounded, reviewable execution brief. Follow the owner's condition:
first confirm matched live RB-01 Light/Dark captures at 1280 x 840 logical
pixels and explicit owner approval. If either is missing, do safe RB-01 capture
and evidence work only, then stop before RB-02. If both are satisfied, implement
RB-02 and stop before RB-03. Do not claim an approval that has not happened.

## 1. Authority and scope

Use this order of authority:

1. `E:\AGENTS.md` and `E:\RustDeveloper.md`.
2. `E:\Builder3D-Rust\AGENTS.md`.
3. [`product-rebuild-plan.md`](product-rebuild-plan.md), which owns the current
   slice order, contracts, evidence and acceptance rules.
4. The historical [`product-rebuild-handoff.md`](product-rebuild-handoff.md)
   (RB-00 context only), the latest [`WORK_LOG.md`](../WORK_LOG.md), the
   [RB-01 handoff](rb-01-visual-foundations-handoff.md) for unresolved RB-01
   evidence only, and the
   [RB-01 checkpoint](../evidence/rebuild/rb-01-visual-foundations/checkpoint.md).
5. The [RB-00 audit](../evidence/rebuild/rb-00-reference-audit/report.md),
   [parity ledger](../evidence/rebuild/parity-matrix.toml), relevant Rust
   modules/tests, and the original project contracts in `L:\Builder3D`.

`L:\Builder3D` is a read-only behavioral and visual reference. Do not change
its source, project files, settings, assets, or checkout state. Keep using the
existing Rust workspace and eframe app. Do not stage or commit files.

RB-02 is the first integrated, user-visible model task. It combines the
current eframe shell and original artwork with the existing Rust document,
commands, import/save path, and R5 viewport renderer. It does not attempt the
full application rewrite or close the later file lifecycle, scene management,
viewport, tools, MCP, performance, packaging, or release slices.

## 2. Entry gate and one-window capture workflow

The current [RB-01 checkpoint](../evidence/rebuild/rb-01-visual-foundations/checkpoint.md)
records that matched live original/Rust captures and owner review are pending.
Recheck the evidence and current desktop state at the start of the next
session. Do not treat historical images or elapsed time as clearing this gate.

Do not begin RB-02 implementation until all of these are true:

- The original and Rust apps have valid live Light and Dark captures at a
  matched **1280 x 840 logical-pixel client area** and matching display scale.
- Each capture records actual logical client dimensions, physical image
  dimensions, display scale, window state, theme, source/build identity and
  capture time. Record the Python source's `MainWindow.resize(1280, 820)` as
  its requested startup outer size, not as a measured client size.
- The captures have been opened and inspected; the comparison board explains
  observed differences and labels historical images as historical.
- The owner has explicitly reviewed and approved the representative visual
  direction. A successful launch, unreviewed capture or elapsed time is not
  approval.

If a capture or owner review is unavailable, finish safe RB-01 capture and
documentation work, keep its visual and owner statuses pending, and stop
before RB-02. Do not start implementation while waiting for approval.

The one-window rule means one visible Builder3D target window at a time. Use
serial sessions for comparisons: launch the Python app, capture the needed
theme, close it normally and verify its window is gone; then launch the Rust
app, capture the matching state, and close it. Compare the saved images after
both windows are closed. Repeat for the other theme. Do not run the two target
apps side by side. This sequential workflow satisfies the one-window rule. A
background MCP gateway with no target window is not by itself a blocker; if a
concrete launch or capture conflict appears, investigate that conflict and
leave unrelated processes alone.

For the comparison board, aim for a matched **1280 x 840 logical-pixel client
area** and display scale. Record actual client logical dimensions, physical
image dimensions, display scale, window state, theme, build/source identity,
and capture time. The Python source's `MainWindow.resize(1280, 820)` is its
requested startup outer size; record that separately and never report it as a
measured client size. If the target cannot be matched, record the actual
measurements and delta and keep RB-01 pending. Do not cross the RB-02 gate
without the required captures and owner approval.

## 3. Required preflight and read order

Before implementation:

1. Read the authority documents in section 1 and the latest RB-01 report,
   verification, capture board, metadata, and checkpoint. Read the latest
   work-log entry and inspect `git status`; preserve the workspace's existing
   untracked user work. The current checkout has extensive untracked project
   content, so do not use reset, clean, or broad replacement operations.
2. Read the RB-01 report, verification, capture board, metadata and checkpoint.
   Recheck app windows and process state, then use the serial capture workflow.
   Confirm the full entry gate above before starting RB-02; leave unrelated
   processes alone.
3. Identify the last verified recovery point. The latest documented Rust
   recovery point is
   `G:\Builder3D-Rust_snapshot_20261030-192216_RB-01-historical-reference-review`;
   verify its receipt and current workspace state before creating any new
   baseline. Snapshots are additive and must never be overwritten or edited.
4. Read the `.3dp` and import contracts in the original project's
   `docs/project-format.md`, `docs/architecture.md`, and relevant source/tests
   without writing under `L:\Builder3D`.
5. Trace the current Rust flow from `Builder3dApp` and `CommandState` through
   `RenderScene`, camera, picking, gizmo, and `ViewportRenderer`. Map stable
   object IDs through rendering and back to the authoritative scene; never use
   a filtered render-list index as a scene index.
6. Verify the pinned eframe 0.37.0/wgpu versions and enabled features in the
   workspace manifests/lockfile before choosing the presentation seam. Check
   the current official API/license documentation for any new platform dialog
   dependency. Do not upgrade eframe/wgpu or switch toolkits speculatively.
7. Inspect the actual host display scale, GPU adapter/backend/driver and
   resource profile at runtime. The previous RB-01 checkpoint recorded a
   2560 x 1440 primary display, 2560 x 1392 work area and 96 DPI (100%); treat
   these as historical measurements to recheck, not as current runtime data.

## 4. RB-02 user outcome

In one ordinary eframe application window, a user can complete this bounded
task without a separate demo executable or an approximate preview:

1. Start from a clean document and use **New**.
2. Add a cube. Import the known `python_box.stl` fixture through a native
   desktop file picker and choose millimetres for its unitless mesh format.
   The fixture source is
   `crates/builder3d-core/tests/fixtures/formats/python_box.stl`; record its
   byte length and SHA-256 in the evidence manifest when exercised. Keep the
   fixture read-only and inside the Rust project. If it proves unsuitable,
   select a documented existing supported fixture and record the reason and
   replacement hash; do not manufacture a reference result.
3. See the imported geometry as shaded, perspective 3D in the main application
   viewport alongside the RB-01 shell and original ribbon artwork that passed
   the entry gate.
4. Orbit, pan and zoom using real pointer input. Select a visible mesh surface
   by a camera ray against actual geometry, and verify the hit maps to the
   correct stable object ID in the authoritative scene.
5. Move an object with a real translation-axis gizmo drag. Keep the existing
   numeric Move fields connected to the same command/document path and confirm
   they display the resulting values. A numeric edit may also be used to
   separate the cube and import for the pick check. Gizmo cancel/no-op must not
   create a history entry; a committed drag must undo as one operation.
6. Undo the geometry change, redo it for the persistence check, save a `.3dp`
   project, and reopen that saved project through the app. Verify object count,
   names, mesh geometry/bounds, colors, millimetre transforms, and persistent
   IDs against the saved state. Record the documented treatment of transient
   state such as selection, camera, and undo history rather than silently
   claiming it persists.
7. Switch between Light and Dark and confirm the accepted shell remains
   integrated around the live viewport. Capture both themes at the comparison
   target where the desktop allows it.

The user task is the proof. Unit tests, `--smoke`, the R5 binary, a standalone
renderer window, a saved offscreen image, or a CPU rectangle preview do not
prove an integrated native viewport.

## 5. Current Rust seams and implementation direction

Audit and adapt the narrowest useful seams; keep one authoritative document
and one command/history path.

| Area | Current seam / known gap | RB-02 direction |
|---|---|---|
| Application shell | `crates/builder3d-app/src/shell.rs`; `render_viewport` draws projected rectangles and `preview_object_rects` selects by rectangle | Replace the product viewport path with a real renderer presentation and geometry-based input. Keep the RB-01 eframe shell, theme and artwork in the same window. |
| Document and commands | `crates/builder3d-app/src/commands.rs` already routes New, Cube, import, transforms, undo/redo, `.3dp` open/save | Reuse `CommandState` and core commands. Widgets and renderer adapters must not mutate the scene directly. Route completed gizmo drags as one grouped command. |
| Import dialog | `crates/builder3d-app/src/dialogs.rs` currently asks for a typed path in an egui window | Add a native OS file-selection path for import with supported-format filters; retain unit choice for STL/OBJ/PLY and honor file-authoritative units for glTF/GLB/3MF. Keep cancel/error non-destructive. Do not claim WRL import; it is explicitly unsupported in the current Rust import seam. |
| Camera and picking | `crates/builder3d-viewport/src/camera.rs`, `picking.rs`, `math.rs` | Reuse the camera/ray math. Derive viewport dimensions and pointer coordinates from the actual client surface, account for logical/physical scale, and return stable IDs from exact geometry hits. |
| Gizmo | `crates/builder3d-viewport/src/gizmo.rs` builds/hit-tests renderer-neutral geometry; app input is not yet wired to a real viewport | Present and hit-test the same translation handle geometry; capture pointer through drag, preview the transform, commit once on release, restore on cancel, and test hidden/reordered objects. |
| Renderer | `crates/builder3d-viewport/src/renderer.rs` creates its own wgpu instance/device and offscreen color/depth textures. `r5_viewport` is a renderer spike and notes it has no native surface/text labels. | Adapt the existing renderer to eframe's native presentation seam and the pinned wgpu device/target format. Avoid creating a second device/window if the eframe render state can supply the shared device. Preserve budget accounting and renderer reuse. |
| Reference runner | `crates/builder3d-app/src/bin/r5_viewport.rs` renders offscreen R5 controls | Keep it as diagnostic evidence only; it cannot close native surface, resize, input, or app integration acceptance. |

Before coding the renderer adapter, verify the actual eframe 0.37.0 feature
configuration and documented wgpu paint-callback/render-state API. Preserve
device/context ownership, texture lifetime, surface format, resize, lost/outdated
surface recovery, close/reopen, and adapter selection. Do not solve integration
by reading the GPU target back to CPU on every displayed frame. Readback may be
used for an explicit diagnostic capture, with its cost and evidence role
recorded.

Use the existing `RenderScene::from_core` path or a smaller documented adapter
so rendering reflects the current `CommandState.scene()` revision. Define a
clear invalidation/update rule for scene geometry and camera changes. Keep
authoritative meshes, GPU copies, render targets, caches and in-flight work in
the shared resource ledger. On this host, use the Quadro RTX 4000 when runtime
discovery confirms it is usable; record the actual adapter, backend, driver,
memory route and fallback reason. A CPU fallback may be a bounded safety path,
but it is not evidence that GPU viewport integration passed when a usable GPU
is available.

Use one viewport input router. It must respect egui text-field, menu and modal
capture; map pointer positions correctly at the current display scale; preserve
focus and release capture on button-up, Escape, window focus loss and cancel.
Orbit/pan/zoom, exact pick and gizmo drag must share one camera/surface mapping.
Keep eframe responsible for the approved original-art shell and theme; do not
open a second top-level Builder3D window.

## 6. Focused acceptance matrix

| Gate | Required evidence |
|---|---|
| RB-01 prerequisite | Matched original/Rust Light and Dark board at 1280 x 840 logical client size; actual client sizes and scale recorded; explicit owner approval linked. |
| One-window native run | Build identity, process/window identity and actual GPU/runtime profile recorded; one app window contains shell, panels and live viewport. |
| New and Cube | Fresh New document and cube are visible in the authoritative Items/scene state and real shaded viewport; dirty prompt is exercised only when relevant. |
| Native import | Native picker opens; cancel leaves state unchanged; selecting the fixture imports the expected geometry and unit, names it, selects it, marks the document dirty, and reports errors without partial mutation. |
| Real viewport | Shaded geometry and depth appear on eframe's native surface; resize and both themes work; no rectangle-preview product path remains. |
| Camera | Real orbit, pan and zoom visibly update the same camera used by ray picking; text entry/modal input is not stolen. |
| Exact pick | Pick hits the intended front-visible geometry and stable object ID; test an overlapping/occluding case or document why fixture geometry cannot provide one. Hidden/reordered scene data must not redirect the hit to a different object. |
| Transform and history | One translation-axis gizmo drag commits exactly once; cancel/no-op has no history entry; undo restores the pre-drag transform and redo restores it. Numeric Move fields dispatch through the same authoritative commands. |
| Save/reopen | `.3dp` reopen reproduces object count, IDs, names, mesh/bounds, colors and transforms; save/reopen errors do not lose the in-memory project. Record which transient state is not persisted. |
| Visual/theme | Both Rust themes include the approved RB-01 art in the same live app. Matched captures use the 1280 x 840 logical client target; actual dimensions, physical image, scale and state are reported. |
| Toolkit decision | Record that eframe passed the representative surface/input test, or document the bounded experiment, precise failure, repair and alternative costs, licensing/size/runtime implications, and owner decision needed. Do not migrate without that decision. |

Keep automated, visual, interactive, and owner-acceptance statuses separate in
the parity ledger. An offscreen image may support rendering diagnostics but
cannot mark native/interactive status complete.

## 7. Comparison and runtime evidence

Use the RB-01 design board and capture metadata. Label the old 1280 x 820
images as historical, not matched acceptance captures. Capture the original
and Rust apps in serial sessions with the same fixture, task state, theme,
logical client area, display scale and window state. For each comparison, close
the first app before opening the second, then inspect the saved images side by
side. Do not resize, crop or rescale product content to claim a match. If exact
matching is impossible, record measured deltas and leave RB-01 pending.

For RB-02, capture these states in both themes:

- clean app/welcome and New empty document;
- Cube only and Cube plus imported fixture;
- shaded imported geometry after camera orbit/pan/zoom;
- exact-picked selection and its transform values;
- gizmo move before and after, undo, and redo;
- saved project after reopen.

For each capture record source app/build or source revision, date/time and
timezone, executable/window identity, theme, task state, logical client width
and height, physical image dimensions, display scale, maximized/restored state,
fixture hash, GPU adapter/backend/driver and any rendering fallback. Record the
original's requested `1280 x 820` outer startup size as its separate source
fact; actual original and Rust client dimensions must be measured.

Open every capture and reject blank, stale, occluded, partially painted,
wrong-window or wrong-state images. The intended serial workflow is one target
window at a time; a background MCP gateway by itself is not a blocker. If a
concrete capture issue prevents matching the required size or scale, record
it, keep RB-01 pending, and stop before RB-02 until the owner gate is met. Do
not substitute historical ribbon crops, generated mockups, headless runs, or
another window.

## 8. Tests and engineering checks for the future execution

Write focused regression tests alongside changed behavior. At minimum cover:

- render-scene synchronization after add/import/transform/undo/redo and save;
- exact ray hit to stable ID, including reordered/hidden scene cases;
- logical-to-physical surface mapping at the measured scale and resize;
- camera gestures and gizmo drag commit/cancel/no-op history boundaries;
- native picker cancel, supported filters, unit selection, parse failure and
  no partial scene mutation;
- `.3dp` save/reopen geometry, IDs and transforms for the walkthrough fixture.

For each patch, run formatting, focused app/viewport tests, workspace check,
changed-crate Clippy, and native interactive verification as appropriate.
Before RB-02 exit, run the integrated-milestone checks listed in section 9 of
the rebuild plan and follow its evidence rules. Do not claim native behavior
from headless tests or `--smoke`. This handoff-preparation session made no
application changes and ran no build, tests, or app captures.

## 9. Explicit non-goals and decision boundaries

- Do not begin RB-02 until the RB-01 matched-capture and explicit owner-approval
  gate in section 2 is satisfied. If it is not satisfied, finish safe RB-01
  evidence work only and stop. The owner must review the RB-02 result before
  proceeding to RB-03.
- Do not begin RB-03 or later work: full file lifecycle/export, all import
  formats, recent thumbnails, all selection/group/visibility behavior, all
  camera presets, snapping, complete gizmo modes, every geometry tool, print,
  MCP integration, full resource/platform matrix or release packaging.
- Keep the app on eframe 0.37.0 as the first path. If a representative seam
  fails, document the measured limitation, reproduction, bounded experiment,
  repair cost, alternative cost and tradeoffs; consult current official toolkit
  docs/licenses. Stop for an explicit owner decision before a materially
  different toolkit or runtime is introduced. Do not add Qt, Python or WebView
  runtime dependencies by default.
- Keep the known RB-01 artwork/package-size conflict visible: the 108 copied
  source assets total 43,340,965 bytes, above the 40 MiB target before the
  executable. RB-02 must not discard or alter art to hide that separate gate.
- Preserve all existing untracked work. Do not stage or commit. Do not modify
  existing `G:\` snapshots or anything in `L:\Builder3D`.

## 10. Deliverables and exit

Create `evidence/rebuild/rb-02-first-working-application/` with:

- `report.md`: implementation boundaries, architecture decision, actual task
  result, known differences, blockers, remaining risks and owner questions;
- `walkthrough.md`: exact human steps and observed before/after state;
- `capture-board.md`, capture files and `capture-metadata.toml`;
- a fixture manifest with source path, byte length, SHA-256 and format/unit;
- `verification.md` with focused checks, interactive results, build identity,
  runtime GPU data and failures;
- `checkpoint.md` with exact next slice and snapshot receipt.

Update only applicable parity rows and evidence links; preserve prior status
history. Append a dated `WORK_LOG.md` entry. For every completed bounded patch,
create a new timestamped G: recovery snapshot; include Git metadata and all
source/evidence/work-log files, exclude only approved reproducible outputs and
reparse trees, and never overwrite a snapshot. Verify the copy result, relative
path sets, file count, byte count and recovery-critical SHA-256 values. Append
the snapshot receipt after copying and state that the receipt postdates the
snapshot.

RB-02 exits only when the task above is demonstrated in the existing app, the
real viewport and shell operate together, required automated/native/visual
evidence is reviewed, the toolkit decision is recorded, and the owner has
explicitly reviewed the representative usable slice. If any capture, native
input, runtime, owner or toolkit gate is blocked, state exactly what remains
pending. Do not infer approval and stop before RB-03.

## 11. Short execution prompt

> Read `E:\AGENTS.md`, `E:\RustDeveloper.md`, `E:\Builder3D-Rust\AGENTS.md`,
> the current product rebuild plan, this RB-02 handoff, the latest checkpoint
> and work log. First verify RB-01 has live matched Light/Dark captures at
> 1280 x 840 logical client pixels, with actual client size and display scale
> recorded, and explicit owner approval. Capture the Python Builder3D app and
> Rust app sequentially: capture and close one before opening the other; a
> background MCP gateway alone is not a blocker. If any RB-01 requirement or
> owner approval is missing, do safe RB-01 capture/evidence work only, present
> it for review, and stop before RB-02. If approved, implement the bounded
> New/Cube/native-import/real shaded-viewport/orbit-pan-zoom/exact-pick/
> translation-gizmo/undo/save-reopen flow in the existing eframe app. Use the
> Python Builder3D at `L:\Builder3D` as the reference (not Microsoft's 3D
> Builder), keep it read-only, and capture Light and Dark for each app. Update
> evidence and the ledger, verify an additive G: snapshot,
> and stop before RB-03. Do not stage or commit.

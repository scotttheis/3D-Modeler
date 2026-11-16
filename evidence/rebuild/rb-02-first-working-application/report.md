# RB-02: First Working Application

**Status:** The bounded application workflow, runtime checks, and capture set are complete. The owner accepted the current Light/Dark visual baseline; fine-tuning may happen later. RB-03 has not started.

## Result

The existing eframe application now completes a real scene workflow in the same window: create a new scene, add a cube, import a model through the native file picker, inspect it in the shaded viewport, orbit/pan/zoom, select by exact geometry hit, move with the translation gizmo, undo and redo, save a `.3dp` project, and reopen it. The Light and Dark walkthroughs are recorded in [walkthrough.md](walkthrough.md); the 25 inspected captures are indexed in [capture-board.md](capture-board.md).

The walkthrough used the repository's `python_box.stl` fixture. It has local bounds of `[-1, -1.5, -2]` to `[1, 1.5, 2]` mm when imported with the default millimetre unit. The default cube is 40 mm on each axis, so it initially occludes this small fixture at the import placement. To make both objects visible for viewport review, the cube was resized to 5 mm on each axis and moved to X = -6 mm. The fixture was then selected and moved using its red X gizmo. This changes only the walkthrough project, not the source fixture.

## Implementation boundary

- `modeler3d-app` remains the application shell and authority for command dispatch. `modeler3d-core` retains scene, object identity, project persistence, and history behavior.
- The R5 renderer is initialized from eframe's existing `wgpu` instance, adapter, device, and queue. Its shaded output is registered with egui-wgpu and displayed as a GPU texture in the existing viewport; the normal displayed frame does not read the image back through the CPU.
- Viewport dimensions and pointer locations are converted using the live pixels-per-point value. Picking ray-tests the rendered scene and returns the durable object ID. It does not choose an object by its screen-space rectangle or a filtered render-list index.
- Orbit, pan, and zoom route through the shared camera. A translation drag previews while held and commits one ID-based position command on release. Escape/focus loss cancels, and a no-op drag does not create history.
- Import uses a native file picker with format filters and a millimetre-default unit selector. Cancelling the native Browse dialog preserves the path already in the import dialog and leaves the scene intact.
- Save/reopen uses the existing `.3dp` path. The walkthrough verified object identity, geometry, color, and transforms. Selection, camera, and undo history are transient and were not present after reopening.

The representative slice is enough to retain eframe: the existing app presents a real shaded scene through the same GPU adapter without a second windowing stack or a per-frame CPU readback. No Qt, Python, or WebView runtime was added. This records the RB-02 toolkit decision, not final toolkit or product acceptance.

## Runtime and capture evidence

The live renderer selected the Quadro RTX 4000 on Vulkan (`DiscreteGpu`, NVIDIA driver 581.42). It rendered the walkthrough geometry without a CPU-fallback status. Runtime details are in [runtime-adapter.txt](runtime-adapter.txt). The evidence executable SHA-256 is `4b197161771239c8943fb8fc2f73d6739245145769c301c32931b01e5480da87`.

Every screenshot is a GDI `CopyFromScreen` capture of the visible foreground 3D Modeler client rectangle. Light and Dark runs used separate app processes, closed between runs. The actual client area was 1280 x 840 logical and physical pixels at 96 DPI / 100% scale. The primary display was 2560 x 1440 physical pixels. Per-image timestamps, process IDs, dimensions, byte lengths, hashes, and build identity are recorded in [capture-metadata.toml](capture-metadata.toml).

## Known boundaries and review items

- The captures show the current Rust application in both themes; they are not a Python/Rust side-by-side task comparison. The separately recorded RB-01 matched Light/Dark captures remain the visual-foundation baseline.
- The empty-scene viewport still shows a floor line across its prompt. This is visible in the New-empty captures and is a presentation detail for owner review.
- The live import walkthrough exercised STL. The picker filters and unit choice do not establish runtime correctness for every supported model format; full file lifecycle and format coverage belong to later slices.
- The project reopen retained the saved geometry, object ID, color, and transforms. It reopened with no selection and did not restore the prior camera or undo history.
- This slice is verified at one Windows display scale (96 DPI / 100%). It does not claim the complete DPI, device, memory-pressure, or platform matrix.
- The existing artwork/package-size issue remains visible: the 43,340,965-byte source-art payload plus the measured 14,135,808-byte release executable totals 57,476,773 bytes (54.79 MiB) before installer/runtime sidecars, above the 41,943,040-byte (40 MiB) target. RB-02 did not remove or alter artwork to reduce that total.

The owner reviewed the Light/Dark captures and said they are close enough to pass for now, with fine-tuning later. This accepts the current RB-02 visual baseline. RB-03 remains unstarted under the original stop instruction.

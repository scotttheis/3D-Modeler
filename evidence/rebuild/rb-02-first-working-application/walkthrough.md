# RB-02 Native Workflow Walkthrough

The walkthrough ran in the existing `builder3d-app` eframe window. Input was delivered to the visible native window and each listed state was checked in the UI; this was not a headless or `--smoke` run. The same workflow was completed in Light and Dark. The native Browse-cancel case was exercised and captured in Light.

## Steps and observed state

1. Launch the app in Light. The Welcome surface appears. Choose **New Scene**. The app enters the empty scene. The corresponding captures are [Light Welcome](captures/rust-light-welcome-client-1280x840.png) and [Light New empty](captures/rust-light-new-empty-client-1280x840.png).
2. Add **Cube** from the existing Insert controls. The Items panel shows one cube and the viewport displays shaded geometry: [Light Cube](captures/rust-light-cube-client-1280x840.png).
3. Open **Import Model**. Use its native Browse picker, press Escape, and confirm that the import dialog remains open with its typed path and the scene still has one object: [Light picker cancel](captures/rust-light-native-picker-cancel-client-1280x840.png).
4. Browse to `crates/builder3d-core/tests/fixtures/formats/python_box.stl`, keep the unit at **mm**, and import. The scene count becomes two. The fixture's 2 x 3 x 4 mm local bounds are initially obscured by the 40 mm cube at the same placement; this initial state is captured in [Light import](captures/rust-light-cube-fixture-imported-client-1280x840.png) and [Dark import](captures/rust-dark-cube-fixture-imported-client-1280x840.png). Fixture provenance is in [fixture-manifest.toml](fixture-manifest.toml).
5. Use the numeric inspector to resize the cube to 5 x 5 x 5 mm and set its X position to -6 mm. Both objects are now visible. Orbit with primary-button drag over the viewport, pan with middle-button drag, and zoom with the wheel. The final camera state is captured in [Light camera](captures/rust-light-camera-orbit-pan-zoom-client-1280x840.png) and [Dark camera](captures/rust-dark-camera-orbit-pan-zoom-client-1280x840.png).
6. Click the visible imported geometry. The viewport ray hit selects the `python_box` object; the object row is highlighted and the inspector reports its millimetre position. See [Light exact pick](captures/rust-light-exact-pick-client-1280x840.png) and [Dark exact pick](captures/rust-dark-exact-pick-client-1280x840.png).
7. Drag the red X translation gizmo. The position changes from the imported `[0, 1.5, 0]` mm placement to approximately X = 4.2 mm (Light: 4.201 mm; Dark: 4.199 mm). Each drag is one committed move. Compare the [Light before](captures/rust-light-gizmo-before-client-1280x840.png) / [Light after](captures/rust-light-gizmo-after-client-1280x840.png) and [Dark before](captures/rust-dark-gizmo-before-client-1280x840.png) / [Dark after](captures/rust-dark-gizmo-after-client-1280x840.png) captures.
8. Make a no-motion gizmo click, then press **Ctrl+Z**. The object returns to its prior position, demonstrating that the no-op did not consume an undo entry. Press **Ctrl+Y** to restore the moved position. See the Light [undo](captures/rust-light-undo-client-1280x840.png) and [redo](captures/rust-light-redo-client-1280x840.png), and Dark [undo](captures/rust-dark-undo-client-1280x840.png) and [redo](captures/rust-dark-redo-client-1280x840.png) states.
9. Save to `rb-02-walkthrough.3dp` in Light and `rb-02-walkthrough-dark.3dp` in Dark. Reopen each project in a fresh app process. The reopened scene contains both objects with the saved transforms and color; selection is empty. Compare [Light saved](captures/rust-light-saved-before-reopen-client-1280x840.png) / [Light reopened](captures/rust-light-reopened-client-1280x840.png) and [Dark saved](captures/rust-dark-saved-before-reopen-client-1280x840.png) / [Dark reopened](captures/rust-dark-reopened-client-1280x840.png).

The app was closed before switching theme/process. The saved `.3dp` files are included beside this walkthrough so the demonstrated project state can be reopened directly:

| Theme | Project | Bytes | SHA-256 |
|---|---|---:|---|
| Light | [rb-02-walkthrough.3dp](rb-02-walkthrough.3dp) | 960 | `9b685e1d4a37a14a3a85b2ca88c7d1458d0a1d70d6f09a8c9af78ac17b10e40f` |
| Dark | [rb-02-walkthrough-dark.3dp](rb-02-walkthrough-dark.3dp) | 958 | `7403f74cbcefdb80bdcbcfcd3b9201776d613862caa87eacea128feb4534bc18` |

## Automated persistence contract

`millimetre_import_move_and_project_reopen_preserve_identity_and_geometry` imports the fixture with mm units, saves its object ID/name/mesh/bounds/color, applies a stable-ID move to `[12.5, 3.0, -4.0]`, saves and reopens the project, then asserts the single reopened object preserves those values and its translated bounds. The test removes its temporary project after verification. Related command and gizmo tests cover stable-ID move, undo/redo, and no-op history.

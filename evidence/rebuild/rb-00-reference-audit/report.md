# RB-00 Reference and Reuse Audit

**Date:** 2026-10-30  
**Scope:** source/test/capture audit only; RB-00 only  
**Result:** audit package assembled; live interaction, visual parity, Rust build, and owner acceptance remain pending.

## Scope and method

The feature ledger contains 114 distinct records covering the original file menu, ribbon commands, Welcome actions, utilities, Items panel, transform strip, settings, shortcuts, pointer gestures, progress/failure behavior, print handoff, artwork, and MCP surface. It includes audit records for source handlers that are not currently exposed (Front view, the unit-override helper, stale Custom/Cut parity rows) and the Rust-only visibility toggle. This separates visible behavior from dormant code.

The records describe source/test-derived contracts. They do not claim that any behavior was exercised in a live UI. Every feature keeps its interactive and owner-acceptance state pending; all automated states are pending because tests were not run. The two audit-only non-visual records use not-applicable for visual review. Static checks parsed the ledger and manifest, checked all referenced Python/Rust paths, focused-test names and component IDs, and compared the ordered MCP names. These checks are artifact/source checks, not product tests.

The original checkout was inspected read-only at `L:\Builder3D`. At 2026-10-30 14:25 local, its Git HEAD was `3a0160a95c55f68ce9f1e89d17c792816ac001df`; the worktree reported 69 tracked changes and 154 untracked paths. The audit did not write to that checkout. The Rust workspace had no commit at task start; existing untracked Rust work was preserved. No Rust build, Python test, application launch, commit, Python/settings/project/asset change, or existing-snapshot change was made.

Current source fingerprints used for this audit:

| Source | SHA-256 |
| --- | --- |
| `L:\Builder3D\src\main_window.py` | `3fc2b1c25a53f6f7d040d3321d331a22d7b8a462f8cb4fcd18a9631b960a7c41` |
| `L:\Builder3D\src\style.py` | `9b2e4d0d9761b5ee04784d5ed967bfca2c97492497e5f4e1eaff2bc9bd962056` |
| `L:\Builder3D\src\icon_assets.py` | `f060eae04b0d92076a6459589fd81eee27c410f45c6ee06a76779c70b1e9e791` |
| `L:\Builder3D\src\application_services.py` | `1c9724616ad20924a44a720176967c264fecd942622e04d37b548b9c332684d8` |
| `L:\Builder3D\src\mesh_ops.py` | `fb03df7143650c9414e4f7b67638aad1f9ff0b8191c56cfa05baf136581c28ee` |
| `L:\Builder3D\src\viewport.py` | `7e2af3293072f865a011b787d05e3c9afc426e5bf2072b3cd7590acda79df033` |
| `L:\Builder3D\scripts\mcp_catalog.py` | `a373a33b1d2640128d2b6ba05c75b5f4cfa7fad40a8e9e29067b9de8cfa6893a` |
| `L:\Builder3D\scripts\builder3d_mcp.py` | `7dd9b935ee4e7b498ee3e7e994067ba0dbbad6804d75fe0d7dae64d01923489` |
| `crates/modeler3d-app/src/shell.rs` | `c0b0c53b294a8c8391c55e870b45c13c3278051e3f445096ae7de4dafc32facf` |
| `crates/modeler3d-viewport/src/renderer.rs` | `e05f7f8934a798c7b7e186804d10fd24c4360f7c937caf4769efb2180843ff01` |
| `crates/modeler3d-mcp/src/r4/catalog.rs` | `116d6f1ec940c8c8b3f2fca7d2365bb1293e978f1460a39e6d0ab821a745cb62` |

## Original source and focused-test map

| Surface | Original source examined | Focused tests inspected (not run) |
| --- | --- | --- |
| Menu, ribbon, Welcome, Items panel, shortcuts, transforms | `src/main_window.py`, `src/application_services.py`, `src/toolbar.py` | `test_adapter_boundaries.py`, `test_desktop_actions.py`, `test_transform_bar.py`, `test_live_viewport_input.py` |
| Project lifecycle, imports, exports and failure preservation | `src/project_io.py`, `src/mesh_ops.py`, `src/three_mf.py`, `src/gltf_io.py`, `src/wrl_io.py` | `test_project_files.py`, `test_project_format.py`, `test_mesh_ops.py` |
| Mesh edits, brushes, repair, viewport and snapping | `src/mesh_ops.py`, `src/scene.py`, `src/viewport.py`, `src/gizmo.py`, `src/snapping.py` | `test_mesh_ops.py`, `test_live_viewport_input.py`, `test_desktop_actions.py` |
| Artwork, theme, settings, print and MCP/AI | `src/icon_assets.py`, `src/style.py`, `src/branding.py`, `src/slicer_launcher.py`, `src/print_prep.py`, `src/ai_commands.py`, `scripts/mcp_catalog.py`, `scripts/modeler3d_mcp.py` | `test_icon_assets.py`, `test_mcp_catalog.py`, `test_ai_commands.py`, `test_adapter_boundaries.py` |

The original surface is declared in `FILE_MENU_SPEC` and `RIBBON_SPEC` in `src/main_window.py`; the same file defines `ObjectPanel`, `WelcomeHub`, `TransformBar`, shortcut registration, dialogs and input handlers. The ledger links feature-level source symbols and focused tests rather than treating a module name as proof.

## Inspected reference baseline

Six inspected, byte-matched historical ribbon crops are preserved under `reference/`. They are 1440×138 captures documented as real-window ribbon evidence in the original 2026-10-20 work log. They show only a ribbon strip; they are not current full-window or matched Rust/Python evidence.

| Preserved capture | Original source | Bytes | SHA-256 |
| --- | --- | ---: | --- |
| `original-insert-dark-1440x138-2026-10-20.png` | `L:\Builder3D\logs\ribbon-icons-dark.png` | 32,943 | `305ecf4af1ee2ed13d481cf7bb3f58801a3f91022962464c94af87b342ed2127` |
| `original-insert-light-1440x138-2026-10-20.png` | `L:\Builder3D\logs\ribbon-icons-light.png` | 35,367 | `8269debce177d7ccba9a64b4c2e81a7b4199705417a1a20c965d9662b52b1b74` |
| `original-object-light-1440x138-2026-10-20.png` | `L:\Builder3D\logs\ribbon-object-light.png` | 39,896 | `99c190f49e12c1cceb9157794115b73a14f0dc34e88335af218594a42c92759e` |
| `original-edit-light-1440x138-2026-10-20.png` | `L:\Builder3D\logs\ribbon-edit-light.png` | 46,251 | `47b7b100a9a19052e6415f249b7c08ccd5de1af112bbbfd5493faecc861846e9` |
| `original-paint-light-1440x138-2026-10-20.png` | `L:\Builder3D\logs\ribbon-paint-light.png` | 21,127 | `1e05a26bb4dde19f61b46386ec67961041df9a584ce369d00a6c874077ade1e0` |
| `original-view-light-1440x138-2026-10-20.png` | `L:\Builder3D\logs\ribbon-view-light.png` | 36,598 | `b0540ad0d46a35b1977ed82c8751f60a1fde5f1c5d3aababad6b9b023b7e1687` |

A current live full-window reference was not captured. The original testing rules require one foreground 3D Modeler target at a time. Process inspection found the existing 3D Modeler MCP gateway process (`L:\Builder3D\venv\Scripts\python.exe ... scripts\modeler3d_mcp.py`) and its Python runtime; there was no established safe handoff to a serial native-window capture. I left those processes untouched and did not launch another 3D Modeler window. Rust was not launched either. Current full-window light/dark, Welcome, menu/settings, object-list states, transform strips/gizmos, import choice, brush popover, progress, error and print-prep captures remain pending; no UI result is inferred.

The old `evidence/r8/00-baseline/python-reference.png` was opened and explicitly disqualified: it is a blank white client area and does not establish the current original layout. Historical theme/settings/transform/icon-polish shots are stale because they show earlier ribbon/layout states; viewport-only trophy captures and `logs/floor-label-depth.png` do not establish a complete default shell. Referenced `L:\importcube.png` and `L:\squares.png` were absent. These are not treated as baseline evidence.

## Dated decision reconciliation

| Decision/evidence | RB-00 reading and Rust consequence |
| --- | --- |
| Owner acceptance, 2026-10-02, in original `WORK_LOG.md` and `docs/ui-style-guide.md`: default metric floor without the reflection-like projected contact cue; opaque shaded Wireframe with unlit black depth-tested edges; Move arrows relocate to the camera-nearest face while preserving world-axis meaning and drag behavior. | Treat as accepted design requirements. Rust `ViewportRenderer::render` currently builds contact-cue overlays through `ContactCueCache`; resolve this conflict before desktop integration. Rust shell rendering is separate from that renderer. |
| Snapping decisions recorded 2026-09-25/28: center is the world-bounds center; landmark/floor snapping, Shift bypass and persistent setting are the contract, not fixed open-space increments. | Keep source and tests as the contract; owner recheck remains open in the original issue notes. Rust snapping service is a reuse candidate, with actual drag feedback still needing integrated review. |
| Owner supplied 51 light-mode icon files on 2026-10-20; preparation produces 52 light outputs including composed `UndoRedo.png`. | Preserve the supplied names, theme variants and transparent prepared outputs. The source loader maps Pin/Snap PNGs before the root fallback silhouettes; `icon_assets.py` maps raster Undo/Redo files even though older style text describes generated curved arrows. Reconcile from the current map and accepted art, not stale wording. |
| Navigation modifiers and persistent brush popovers are documented in current code/style/tests, but no separate owner-acceptance date was found. | Treat as source/test-derived required behavior; keep Rust interaction and owner acceptance pending. |
| High-poly I10 remains a Python owner/reference acceptance issue in the original work log; synthetic 30M cases do not prove every real mesh. | Do not convert Python performance history into Rust verification or mark high-poly acceptance complete. |

Current-source discrepancies recorded in the ledger include: Python defaults to Dark with cyan/blue palette while Rust preferences default to Light; Rust Light tokens match known reference tokens but its Dark palette differs materially; Python's Front icon/handler and `_on_add_with_units` helper are not routed from the current ribbon; Python's MCP `DESKTOP_PARITY` table retains Custom and Cut rows even though Custom is not exposed and Cut is shortcut-only. `Paint` and `Material` are distinct ribbon items routed to one solid-color handler; `Color` samples average image color and does not create texture mapping. No Python Items row visibility, rename or reorder control was found; Rust currently adds an Eye/Show visibility control, which requires an explicit parity decision.

## Original asset provenance

`asset-manifest.toml` inventories 108 original files: 44 dark and 44 light action PNGs, eight dark and eight light Insert PNGs, and four shared branding/legacy files. It records source path, SHA-256, byte length, dimensions, mode, alpha/edge treatment, logical mapping, preparation inputs and use status. All 108 current asset hashes and all 51 owner input hashes matched the read-only source during audit. All 51 raw light inputs are fully opaque; the 52 prepared light outputs include the composed UndoRedo artwork. The dark art's individual upstream provenance is not stated in the inspected checkout. No art was copied into the Rust product.

Notable source mappings: X-Ray resolves to `ExRay.png`, Subtract to `Subtract.png`, and both Simplify and Subdivide resolve to `Simplify.png`. The source-mapped root `pin.jpg`/`snap.png` fallback files are shadowed by the current themed `Pin.png`/`Snap.png` entries. The manifest distinguishes source mapping from live display; runtime display was not captured.

## Rust reuse and gap map

| Area | Evidence and recommendation | Integration risk |
| --- | --- | --- |
| Scene, mesh operations, project/file I/O, history, typed commands, selection/transforms, preferences, image color, print preparation/handoff, resource planning, slicer discovery, snapping and MCP | Reuse the validated/ported services by their existing component IDs. Backend tests/contracts are useful evidence; they do not prove shell routes or live parity. | Low to medium for service reuse; behavior-specific regressions remain unverified in RB-00. |
| eframe shell and desktop adapters | Keep current eframe as the first UI candidate. `Cargo.toml` pins eframe 0.37.0. Catalog correctly leaves `modeler3d.app.shell` and `modeler3d.app.desktop-adapters` at `polish`; no toolkit experiment or catalog promotion is justified. | Medium to high: the shell must acquire the original layout, artwork, dialogs, interaction and persistence. |
| Viewport | Reuse `modeler3d.viewport.renderer` and camera/picking/gizmo services after integration proof. The renderer is currently a separate service/runner; `Modeler3dApp::render_viewport` instead paints grid/bed and object rectangles, with rectangle hit testing. It does not integrate shaded geometry, camera pointer gestures or exact mesh picking. | High: same-window GPU/context ownership and input/render lifecycle must work together. The owner-accepted no-contact-cue floor is an explicit integration conflict. |
| Artwork/theme | Treat `ThemeArtwork` as placeholder vector art, not the original raster source. Package approved original resources in Rust only in the later visual slice. Reconcile Python's Dark default and dark palette with Rust's Light default and materially different dark palette. | Medium: pixel/layout fidelity and theme states require matched captures and owner review. |
| Desktop-only shell paths | Repair/reuse dialogs, Welcome recents/thumbnails, native file pickers, preferred-slicer/clear-recents/About settings, Save As utility, keyboard/text-focus behavior and ribbon customization. Keep the additional Rust visibility toggle open for decision. | Medium; several gaps are wiring/presentation rather than missing core services. |
| MCP | Static extraction found the same 60 tool names in the same order in Python and Rust, version `2026-09-25.1`. This confirms catalog-name parity only; Rust/Python shared live-document behavior and desktop parity are not established. Python's parity table includes stale Custom/Cut entries. | High for shared live-document integration and exact end-to-end parity. |

The Rust component catalog needs no factual path/scope correction from this audit and was not edited. No component maturity was promoted.

## Risk-ranked gaps and exact next work

1. **High — trustworthy visual baseline and owner direction.** The six historical ribbon crops help with artwork comparison but cannot stand in for a current matched full-window reference. Obtain a safe serial original capture session and record light/dark, client dimensions and display scale before claiming visual comparison.
2. **High — integrated viewport.** The shell's rectangle preview and separate renderer leave the primary model-edit surface incomplete; resolve the default contact-cue conflict against the dated owner decision before integration.
3. **High — first working desktop workflow.** Prove the existing eframe window can host the original direction and real shaded renderer together, including actual pointer input and shared command/history state.
4. **Medium — original artwork and theme fidelity.** Use manifest mappings and approved assets, match the Python Dark default, and verify menus, ribbon, panels, utility controls and dialogs in both themes.
5. **Medium — lifecycle and desktop wiring.** Close gaps in recent cards/thumbnails, dialogs, settings, utility buttons, shortcut/focus behavior and failure feedback; distinguish current Python controls from Rust-only or missing controls.
6. **High — MCP and acceptance evidence.** Exact catalog-name match is not proof of a shared document; a later live client/GUI round trip and owner review remain mandatory.

**Exact next slice: RB-01 Visual Foundations.** When a safe serial reference window is available, capture the original at the initial 1280×820 window target (the Python source calls `resize(1280, 820)`; record actual client dimensions and DPI/scale), light and dark, plus representative ribbon/panel/Welcome/transform states. Build the matched design board and a bounded current-eframe presentation of the approved original artwork/tokens/layout, then request owner direction. Keep RB-01 limited to visual foundations and do not change toolkit without the evidence and owner decision required by the plan.

After RB-01 direction is accepted, RB-02 should demonstrate one unassisted workflow in the same eframe window: New Scene, add Cube, native import, real shaded viewport, orbit/pan/zoom, exact pick, numeric or gizmo Move, Undo, Save and reopen. Integrate the existing renderer rather than treating the rectangle preview as the product path. This is a proposal for the later slice, not work begun in RB-00.


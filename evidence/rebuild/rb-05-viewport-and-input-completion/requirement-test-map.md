# RB-05 requirement-to-test map

Automated implementation, focused regression, and native interaction evidence are complete. Visual review and owner acceptance remain pending.

| Requirement | Rust implementation / focused regression | Native evidence |
|---|---|---|
| Orbit sensitivity and camera-only state | `Camera::orbit`; `reference_orbit_and_zoom_clamps_match_python_contract` in [camera.rs](../../../crates/builder3d-viewport/src/camera.rs) | `orbit-after` Light/Dark |
| Middle/right pan | `Camera::pan`; shell `ViewportDrag::Pan` route | `pan-middle-after`, `pan-right-after` Light/Dark |
| Wheel and pinch zoom | `Camera::zoom_factor`; `ViewportInput::pinch_zoom_factor` | `wheel-after` plus focused camera tests |
| Reset, Center, Frame, Top, Side | `CommandState::frame_view`, `ViewStateController`, `ViewOrientation` | view-state command tests; capture walkthrough uses project framing |
| Visible/hidden/empty framing | `framing_ignores_hidden_objects_and_hidden_only_selection` in [view_state.rs](../../../crates/builder3d-app/src/view_state.rs) | `empty-rest`, `hidden-rest`, `move-hidden-*` |
| Exact double-click frame, no mutation | shell `frame_double_clicked`; exact `pick_scene` and durable ID route | `double-click-frame` Light/Dark |
| Move camera-nearest-face geometry | `build_gizmo_with_bounds`, `move_arrows_are_displaced_to_the_camera_facing_face` in [gizmo.rs](../../../crates/builder3d-viewport/src/gizmo.rs) | `move-single-before/after` |
| Rotate ring agreement | `rotate_ring_geometry_and_hit_use_the_same_bounds_scaled_radius` | `rotate-single-before/after` |
| Resize six faces/eight corners | `ResizeHandle::interactive_handles`, shared bounds-aware paint/hit path | `resize-edge-*`, `resize-corner-*` |
| One commit / safe cancel / no-op | `gizmo_release_commits_once_and_cancel_or_noop_keeps_history` in [shell.rs](../../../crates/builder3d-app/src/shell.rs) | `move-noop-*`, `move-cancel-*`, `move-focus-*` |
| Stale/group/hidden safety | project replacement cache reset and durable-ID paths | `move-group-*`, `move-hidden-*` |
| Persisted Snap toggle | `Preferences::object_snapping_enabled`, `CommandId::ToggleSnap`, command/display-state tests | Python `snap-toggled`; Rust status path and renderer/control captures retained in rejected supplemental Grid journal; primary Rust replay remains accepted |
| Landmark/floor snapping and Shift bypass | `select_snap_correction`, `snap_translation_result`, `TransformGesture::update_with_snap`; [snapping.rs](../../../crates/builder3d-viewport/src/snapping.rs) tests | Python snapping suite and Rust focused snapping tests |
| Actual bounded floor surface | `FloorSurface`, `build_floor`, `FloorGeometry.surface` | Rust `wire-grid-rest` Light/Dark |
| Adaptive metric grid spacing and fade | `FLOOR_MAJOR_GRID_SPACING`, adaptive spacing, bounded `grid_positions`; `floor_spacing_is_adaptive_and_hidden_below_camera` | Rust `wire-grid-rest`; Python `grid-off`/restore |
| Floor scale labels stay attached during pan | `build_floor_scale_labels`; `floor_scale_labels_remain_world_anchored_when_camera_pans` in [floor.rs](../../../crates/builder3d-viewport/src/floor.rs) | Focused Release captures [before](captures/rust-floor-label-pan-20261110-viewport-before.png) / [after](captures/rust-floor-label-pan-20261110-viewport-after.png); details and hashes in [verification.md](verification.md). Original RB-05 paired pan captures predate this repair |
| Authored colors and selection tint | `ViewportTheme`, `display_color`, renderer option test | Light/Dark `single-rest`, `wire-grid-rest` |
| Opaque depth-tested black wireframe | unique full-mesh `wireframe_segments`, renderer wire pipeline | Python `wireframe-xray`; Rust full-mesh native render plus renderer tests |
| Independent translucent X-Ray | `mesh_xray_pipeline`, `ViewportRenderOptions::xray`, alpha regression | Python `wireframe-xray`; Rust renderer tests and native render path |
| GPU/CPU display route | `render_with_options` and `CpuRenderOptions` floor/grid/theme fields | GPU native evidence; CPU fallback regression in [cpu.rs](../../../crates/builder3d-viewport/src/cpu.rs) |
| Geometry cache immutability | `MeshGeometry::geometry_identity`, cache identity regression | full workspace tests; no scene mutation in camera/display captures |

Source-of-truth capture metadata is [capture-metadata.json](capture-metadata.json); the paired image index is [paired-comparison-board.md](paired-comparison-board.md).

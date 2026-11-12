# RB-04 requirement-to-test map

Implementation, automated and native interaction evidence is complete for the29 RB-04 rows. Visual/owner review remains pending. The three existing RB-05 gizmo rows retain their scope: base Rotate/Resize native pass; Move partial for camera-near-face placement/style/snapping. No RB-05 implementation is included.

| Stable requirement | Native paired action | Focused Rust regression |
|---|---|---|
| `builder3d.ui.object.duplicate` | `duplicate` in both themes | [clipboard_offsets_ids_hidden_state_and_one_step_history_match_reference](../../../crates/builder3d-app/src/commands.rs) |
| `builder3d.ui.object.copy` | `copy` in both themes | [clipboard_offsets_ids_hidden_state_and_one_step_history_match_reference](../../../crates/builder3d-app/src/commands.rs) |
| `builder3d.ui.object.cut` | `cut` in both themes | [grouped_hidden_clipboard_duplicate_delete_and_undo_preserve_metadata_and_fresh_ids](../../../crates/builder3d-app/src/gesture_tests.rs) |
| `builder3d.ui.object.paste` | `paste-other-selection` in both themes | [native_clipboard_keys_paste_objects_and_keep_text_clipboard_routes](../../../crates/builder3d-app/src/shell.rs) |
| `builder3d.ui.object.delete` | `delete` in both themes | [selection_commands_cover_group_cut_and_non_history_selection](../../../crates/builder3d-app/src/commands.rs) |
| `builder3d.ui.object.group` | `group` in both themes | [grouped_hidden_clipboard_duplicate_delete_and_undo_preserve_metadata_and_fresh_ids](../../../crates/builder3d-app/src/gesture_tests.rs) |
| `builder3d.ui.object.ungroup` | `ungroup` in both themes | [grouped_hidden_clipboard_duplicate_delete_and_undo_preserve_metadata_and_fresh_ids](../../../crates/builder3d-app/src/gesture_tests.rs) |
| `builder3d.ui.items.select_row` | `shift-range` in both themes | [items_anchor_follows_identity_after_an_earlier_row_is_deleted](../../../crates/builder3d-app/src/shell.rs) |
| `builder3d.ui.items.select_all` | `select-all` in both themes | [selection_commands_cover_group_cut_and_non_history_selection](../../../crates/builder3d-app/src/commands.rs) |
| `builder3d.ui.items.deselect_all` | `deselect-all` in both themes | [selection_commands_cover_group_cut_and_non_history_selection](../../../crates/builder3d-app/src/commands.rs) |
| `builder3d.ui.items.invert` | `invert` in both themes | [selection_commands_cover_group_cut_and_non_history_selection](../../../crates/builder3d-app/src/commands.rs) |
| `builder3d.ui.items.sticky` | `sticky-empty` in both themes | [viewport_picks_toggle_plain_ctrl_shift_and_empty_sticky_without_history](../../../crates/builder3d-app/src/shell.rs) |
| `builder3d.ui.items.health_warning` | `health-repair` in both themes | [replacing_a_project_at_the_same_revision_resets_items_and_gizmo_caches](../../../crates/builder3d-app/src/shell.rs) |
| `builder3d.audit.items.visibility_gap` | `visibility-reference` in both themes | [selection_modifiers_and_visibility_are_presentation_and_model_seams](../../../crates/builder3d-app/src/commands.rs) |
| `builder3d.ui.object.settle` | `settle` in both themes | [settle_single_and_group_match_python_convex_hull_cases](../../../crates/builder3d-app/src/transforms.rs) |
| `builder3d.ui.object.mirror` | `mirror-y` in both themes | [mirror_changes_local_column_and_matching_translation_component_after_rotation](../../../crates/builder3d-app/src/commands.rs) |
| `builder3d.ui.object.measure` | `measure` in both themes | [measure_reports_each_objects_width_depth_height_without_history](../../../crates/builder3d-app/src/commands.rs) |
| `builder3d.ui.object.align` | `align-z-max` in both themes | [placement_all_axes_landmarks_gaps_and_anchor_orders_match_42_python_cases](../../../crates/builder3d-app/src/gesture_tests.rs) |
| `builder3d.ui.object.arrange` | `arrange-z-gap5` in both themes | [placement_all_axes_landmarks_gaps_and_anchor_orders_match_42_python_cases](../../../crates/builder3d-app/src/gesture_tests.rs) |
| `builder3d.ui.transform.move_numeric` | `move-inch` in both themes | [numeric_apply_preserves_unedited_exact_axes_and_noop_history](../../../crates/builder3d-app/src/shell.rs) |
| `builder3d.ui.transform.rotate_numeric` | `rotate-degree` in both themes | [absolute_rotation_is_repeatable_and_resize_preserves_translation](../../../crates/builder3d-app/src/commands.rs) |
| `builder3d.ui.transform.resize_numeric` | `resize-inch` in both themes | [multi_numeric_matches_32_python_toolbar_axis_unit_and_lock_cases](../../../crates/builder3d-app/src/gesture_tests.rs) |
| `builder3d.ui.transform.units` | `units-only` in both themes | [numeric_apply_preserves_unedited_exact_axes_and_noop_history](../../../crates/builder3d-app/src/shell.rs) |
| `builder3d.ui.transform.resize_basis` | `viewport-resize-object-face` in both themes | [all_python_handles_axes_bases_groups_and_hidden_targets_match_122_oracles](../../../crates/builder3d-app/src/gesture_tests.rs) |
| `builder3d.ui.transform.proportional_lock` | `multi-resize-locked` in both themes | [proportional_numeric_resize_uses_edited_y_in_inches_and_rejects_invalid_input](../../../crates/builder3d-app/src/shell.rs) |
| `builder3d.ui.viewport.exact_pick` | `pick` in both themes | [viewport_picks_toggle_plain_ctrl_shift_and_empty_sticky_without_history](../../../crates/builder3d-app/src/shell.rs) |
| `builder3d.ui.shortcut.delete` | `delete-text-focus` in both themes | [editing_shortcuts_respect_text_focus_and_modal_boundaries](../../../crates/builder3d-app/src/shell.rs) |
| `builder3d.ui.items.naming` | `rename-reference` in both themes | [rename_preserves_identity_mesh_and_history_on_invalid_or_unchanged_names](../../../crates/builder3d-app/src/commands.rs) |
| `builder3d.ui.items.thumbnails` | `health-warning` in both themes | [recent_and_items_draw_the_combined_group_once_and_keep_hidden_items_visible_in_rows](../../../crates/builder3d-app/src/recent_preview.rs) |

The paired board and native verification JSON files supply the complete related before/after/undo/redo/cancel cases. Rows sharing a command regression also have the independent 122 gesture,32 multi-numeric,42 placement and9 Euler/polar source cases. Current integrated gates pass261 workspace tests, none failed/ignored. See verification.md for commands/logs.

Real TextEdit and pointer-driven modal regressions verify focus containment. Native clipboard and Settings controls reproduce and verify fixes independently of command tests. Same-revision replacement clears thumbnail/health/gizmo caches. Resource bounds include Settle points/memory, health inspection and grouping depth.

Explicit source differences: Shift endpoints versus required contiguous range; prepared naming/visibility controls; hidden multi-lock; source Escape/focus behavior; Repair fan-point tessellation. These are recorded in report.md and do not imply owner acceptance. All32 RB-03 status fields remain unchanged.

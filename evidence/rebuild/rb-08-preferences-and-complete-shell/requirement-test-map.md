# RB-08 Requirement-to-Test Map

| Requirement | Implementation / contract | Automated verification | Native evidence |
|---|---|---|---|
| Partial, malformed, unsupported and migrated settings preserve independent valid values | `builder3d-app::preferences`; read-only Python QSettings adapter in `builder3d-platform` | `preferences.rs` parsing/migration tests; platform registry decoder tests | isolated fresh runtime, current/legacy precedence, restart and recovery warning |
| Theme/import/display/snap/recent/ribbon settings persist; internal lengths remain mm | preferences and shell load/save | preference serialization and unit-policy tests | Python/Rust settings and post-restart captures |
| Preferred slicer can browse/select/clear; 3D Print remembers only on explicit choice | Settings and print-prep actions; safe staged handoff | path validation, migration, action and print handoff tests | native chooser/settings/prepare/cancel; do not launch external slicer automatically |
| Clear Recent Files is non-destructive | bounded `RecentFiles::clear`, preview generation invalidation | recent-list tests plus shell state test | clear in Settings; verify only list/cache cleared and files still exist |
| About, menu, ribbon and dialog routes are reachable and cancellable | dialog actions and shell routing | route/action tests | menu, About/Settings nesting, picker cancellation and error cases |
| Keyboard shortcuts include New/Open/Save/Save As/Exit and yield to text/modal focus | `handle_shortcuts` and command dispatch | shortcut test matrix including dirty guards/focus | physical keyboard tests with scene, text field, dialog and modal focus |
| Keyboard focus, descriptive names, contrast, hover cursor and focus feedback | custom widget semantics and egui AccessKit integration | widget contract tests where possible | keyboard-only native focus walkthrough; screen reader status documented |
| Empty/recent welcome previews use safe fallback on invalid/stale sources | existing preview worker and fallback, plus invalidation on clear | `recent_preview.rs` and shell stale-generation tests | empty, valid, missing, corrupt, unsupported, stale and restart captures |
| Responsive shell and visual parity | existing original artwork and responsive ribbon/welcome layout | workspace tests | paired native Python/Rust captures in both themes, sizes/scales and max state |
| Rebuild record and final recoverability | ledgers, catalog, matrix, work log, checkpoint | TOML/JSON/path validation and documented Cargo gates | final receipt and additive `G:` snapshot manifest verification |

No RB-09 work is in this slice. Visual and owner acceptance are recorded as
pending unless separately approved by the owner.

## Executed focused reference tests

The final Python focus run passed 12 tests (see
`verification/python-focused-final.txt`):

- `tests/test_project_files.py::test_legacy_recent_files_are_migrated_without_overwriting_new_settings`
- `test_3d_print_preference_and_export_use_all_visible_scene_objects`
- `test_3d_print_button_uses_assigned_slicer_name`
- `test_slicer_chooser_returns_detected_slicer_and_remember_choice`
- `test_theme_setting_switches_palette_and_refreshes_live_window`
- `test_ribbon_tab_click_toggles_command_row_and_pin_keeps_it_open`
- `test_ribbon_favorites_persist_and_use_the_existing_command_identity`
- `test_renamed_ribbon_commands_migrate_saved_favorites_and_edit_order`
- `test_ribbon_favorites_can_reorder_and_persist_their_order`
- `test_edit_ribbon_order_is_validated_by_section_and_persists_button_order`
- `test_scene_edit_shortcuts_are_platform_bound_and_yield_to_text_fields`
- `test_ribbon_favorites_use_overflow_only_when_the_top_row_is_tight`

The Rust tests that directly cover the RB-08 boundary include
`preferences.rs::partial_settings_recover_fields_independently_and_preserve_unknown_keys`,
`preferences.rs::python_aliases_and_flat_ribbon_orders_migrate_to_rust_fields`,
`preferences.rs::current_empty_recent_list_wins_over_old_python_recent_paths`,
`preferences.rs::missing_json_migrates_python_values_but_malformed_json_is_not_overwritten`,
`legacy_settings.rs::decodes_qsettings_string_and_string_list_without_empty_elements`,
`legacy_settings.rs::decodes_integer_encodings_and_rejects_unsupported_registry_data`,
`legacy_settings.rs::legacy_presence_and_values_are_distinct_for_empty_current_recents`,
`dialogs.rs::escape_closes_about_before_closing_settings`,
`shell.rs::clear_recent_files_keeps_sources_and_invalidates_pending_preview_results`,
`shell.rs::file_shortcuts_are_available_in_text_fields_and_dirty_guards_still_apply`,
`shell.rs::editing_shortcuts_respect_text_focus_and_modal_boundaries`, and
`slicer.rs::manually_selected_slicer_paths_are_canonical_and_labeled_without_launching`.

The corrected Python reference run contains **50 accepted PNGs** under
`native/python-live-baseline-20261102-094708/captures/`, including both themes,
the available 980×640 and 1600×900 windows, maximized layout, settings and
About, picker and dirty dialogs, keyboard/text-field cases, and recent-preview
fallbacks. Pre-edit Python frames from the earlier non-isolated QSettings
launcher are diagnostic and are not counted.

The final Rust native interaction matrix is still pending desktop unlock. The
current release process has an isolated migrated settings JSON and a
diagnostic-only accessibility tree; neither replaces the required paired
foreground captures and real keyboard/modal interactions. See
`rejected-attempts.md` for the lock-screen capture rejection.

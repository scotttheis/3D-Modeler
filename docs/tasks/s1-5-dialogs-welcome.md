# S1-5 / S1-6 / M5: one dialog style; Welcome grid and states (Session 2)

Tier: Implementer (Codex medium). Files: `crates/modeler3d-app/src/dialogs.rs`,
`shell.rs` (Welcome rendering, recent grid), `chrome.rs` helpers.
Use chrome.rs tokens; palette colours only.

Part A, dialogs/popovers:
1. One dialog frame helper in `chrome.rs` (surface fill, RADIUS_SURFACE, hairline
   border, elevation shadow, SPACE_4 inner margin, title in FONT_TITLE) used by
   every dialog in `dialogs.rs` (Settings, About, Save, Export, Material, import
   units, hollow, repair, print, etc.) instead of per-dialog frames.
2. One button order for dialogs: secondary/Cancel/Close left of the primary
   action, primary rightmost, same height (CONTROL_HEIGHT). Tests assert some
   layouts (`save_dialog_offers_stl_and_3mf_with_save_and_cancel_on_one_line`,
   `export_has_a_type_list_and_export_beside_cancel...`): keep them passing; do
   not change which buttons exist.
3. One error style: same red-tinted text/icon row (palette error colour, FONT_SMALL)
   wherever a dialog shows an error; results keep the normal text colour
   (test `results_read_in_the_normal_text_colour_and_offer_close`).

Part B, Welcome:
4. Hero, action cards and the recent grid sit on one column grid (shared left
   edge and gutter SPACE_4/SPACE_5; recent cards equal width; test
   `recent_grid_allocates_a_visible_width_for_every_column`).
5. Designed empty state (no recents: friendly one-line message + the main
   actions), missing-file card (already a fallback icon: label it "File not
   found" in muted text), and a loading state for previews still being built
   (neutral placeholder tile, no layout jump). Reuse existing data; no new features.

No behaviour/MCP change. Verify (Fast, only these; Windows:
`. .\scripts\Initialize-BuildClang.ps1`, `cmd /c "cargo ... > file 2>&1"`, file outside target/):
`cargo fmt --all -- --check`; `cargo clippy -p modeler3d-app --all-targets --locked -- -D warnings`.
No cargo test, no captures, no commit. Report files changed and anything left alone.

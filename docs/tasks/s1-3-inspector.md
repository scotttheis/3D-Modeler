# S1-3 / M3: Inspector sections and one field grid (Session 2)

Goal: the right-hand panel (Items list + the transform inspector,
`render_transform_inspector` in `crates/modeler3d-app/src/shell.rs` ~4195 and its
callers) reads as labelled sections with one consistent field grid, and has no
big empty lower half. Tier: Implementer (Codex medium). Use the tokens in
`chrome.rs` (FONT_*, SPACE_*, RADIUS_*, CONTROL_HEIGHT, ROW_HEIGHT); add none that
already exist.

Do:
1. Replace default egui collapsing headers with one small `section(ui, palette, title, |ui| ..)`
   helper in `chrome.rs`: caption-size uppercase-free title (FONT_CAPTION, muted colour),
   hairline above, SPACE_2 gap, no arrow chrome.
2. Put the numeric fields (position, rotation, size, scale) in one grid helper:
   fixed label column, three equal axis fields (axis-coloured as now), same
   row height (CONTROL_HEIGHT), same gaps everywhere.
3. Stacked "Apply move/rotation/size/scale" buttons: keep the same actions, tooltips
   and tests, but lay them out consistently (one button style, aligned under their
   field grid, same width). Do not rename the labels tests look for.
4. Use the empty lower area sensibly: Items list takes the remaining height
   (scrolls inside), nothing floats mid-panel. Do not invent new features.
5. No behaviour or MCP change. Light and dark must both look right (use
   `ThemePalette`, no literal colours).

Verify (Fast gate, only these; Windows: `. .\scripts\Initialize-BuildClang.ps1`
first, `cmd /c "cargo ... > file 2>&1"` with the file outside target/):
`cargo fmt --all -- --check`;
`cargo clippy -p modeler3d-app --all-targets --locked -- -D warnings`.
Do NOT run cargo test (overseer does) or captures. Do not commit.
Existing tests in shell.rs that click "Apply" etc. must still be satisfiable.

Out: ribbon, dialogs, Welcome (M4/M5). Report: files changed, any test you
expect to need updating and why.

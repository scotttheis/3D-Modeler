# S1-8 / S1-9 / M6: control states, contrast, keyboard focus (Session 2)

Tier: Implementer (Codex medium). Files: `theme.rs` (`ThemePalette`, `apply`,
tests at ~437), `chrome.rs`, small fixes in `shell.rs`/`dialogs.rs`.
Palette colours only; the owner's palette identity stays (adjust only where a
test shows a pair fails).

Do:
1. Contrast test: extend `theme.rs` tests to cover every text/background pair
   the palette defines and the shell uses, in BOTH themes: body text on
   surface/panel/viewport-overlay, muted/weak text on surface (>= 4.5:1 for text,
   >= 3:1 for large text and control outlines/icons), disabled text (>= 3:1 or
   documented as exempt), accent-filled button text, error text on dialog surface,
   toast text. List every pair in a table in the test so a missing one is obvious.
   If a pair fails, nudge the palette value minimally (keep hue) and say which.
2. States: in `theme::apply` make sure egui widget visuals define distinct
   inactive / hovered / active (pressed) / disabled looks for buttons, fields,
   checkboxes, sliders, combo boxes, in both themes, using palette values (no
   literals), and a clear keyboard focus ring: 2 px accent stroke
   (`visuals.selection.stroke` / widget focus stroke) visible on every control.
   Our custom-painted controls in chrome.rs (resize basis toggle, section/grid
   helpers, glyph buttons, dialog buttons) must show hover/pressed/disabled and
   focus the same way and be reachable with Tab (`Sense::click()` + focusable).
3. Add a unit test that every custom control family in chrome.rs reports
   `Role::Button`/appropriate accessible info and is focusable.
4. No behaviour or MCP change.

Verify (Fast, only these; Windows: `. .\scripts\Initialize-BuildClang.ps1`,
`cmd /c "cargo ... > file 2>&1"`, file outside target/):
`cargo fmt --all -- --check`; clippy `-p modeler3d-app --all-targets --locked -- -D warnings`;
you MAY run `cargo test -p modeler3d-app --lib theme::` and `chrome::` only.
No captures, no commit. Report any palette value you changed.

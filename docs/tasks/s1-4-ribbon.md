# S1-4 / M4: Ribbon separators, tab type, overflow menu (Session 2)

Goal: the ribbon (tab row + command groups, `shell.rs` ribbon code, `commands.rs`
for the group lists) looks consistent. Tier: Implementer (Codex medium). Use the
`chrome.rs` tokens; no literal colours (`ThemePalette`).

Do:
1. Group separators: one hairline separator style (palette border colour, inset
   from top/bottom by SPACE_1) between every pair of command groups, the same
   everywhere (Add/Open currently has its own).
2. Group labels (if shown) and tab labels use the type scale: tabs FONT_BODY,
   group labels FONT_CAPTION muted. Active tab keeps its joined-to-ribbon look
   (test `active_tab_is_joined_to_the_ribbon_below`).
3. Pin control: align it with the tab row's vertical rhythm / right margin so it
   sits on the same baseline grid as the group labels (small nudge only).
4. Overflow: if commands overflow, they must open from an icon button as a styled
   popup menu (menu rows like the File menu: ROW_HEIGHT, hover highlight), not an
   egui ComboBox. If no ComboBox/overflow exists today, say so and change nothing.
5. No behaviour, command-order or MCP change (`ribbon_matches_reference_tab_and_command_order` must pass).

Verify (Fast, only these; Windows: `. .\scripts\Initialize-BuildClang.ps1`,
`cmd /c "cargo ... > file 2>&1"` with the file outside target/):
`cargo fmt --all -- --check`;
`cargo clippy -p modeler3d-app --all-targets --locked -- -D warnings`.
No cargo test, no captures, no commit. Report files changed and what you left alone.

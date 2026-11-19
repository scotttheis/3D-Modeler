# S1-1 / M2: design tokens in chrome.rs (finish plan, Session 2)

ID: S1-1. Goal: `chrome.rs` is the single source for type scale, spacing,
radii and control heights; the shell uses the tokens, not literals.
Tier: Implementer (Codex medium).

Files: `crates/modeler3d-app/src/chrome.rs` (add tokens),
`crates/modeler3d-app/src/theme.rs`, `shell.rs`, `shell/*.rs`, `dialogs.rs`,
`commands.rs` (use them). No other crate. Do not touch MCP.

Do:
1. Add to `chrome.rs`: `FONT_CAPTION=11.0`, `FONT_SMALL=12.5`, `FONT_BODY=13.0`,
   `FONT_TITLE=15.0`, `FONT_HERO=20.0`; `SPACE_1..SPACE_5 = 4/8/12/16/24`.
   (`RADIUS_*`, `CONTROL_HEIGHT`, `ROW_HEIGHT` already exist; plan also names 28
   as a height: use `ROW_HEIGHT`.)
2. Replace literal `FontId::proportional(<n>)` / font sizes with the nearest
   type token (12.7 and 12.0 -> FONT_SMALL, 11.2/11.5/10.5 -> FONT_CAPTION,
   16/17/18 -> FONT_TITLE). Large icon/display sizes (30) may stay if no token fits;
   say so in your report.
3. Replace literal spacing/margins/gaps that equal 4/8/12/16/24 with SPACE_n,
   and literal 6.0/10.0 corner radii and 26/28 control heights with the existing
   tokens, wherever it is a plain swap. Do NOT change values that are not on
   the scale (leave them, list the count in your report). Do not change layout.
4. No behaviour change. Keep diffs mechanical.

Verify (Fast gate, run only these; on Windows run
`. .\scripts\Initialize-BuildClang.ps1` first and redirect cargo output with
`cmd /c "cargo ... > file 2>&1"` in the scratch file, not in target/):
`cargo fmt --all -- --check`; `cargo clippy -p modeler3d-app --all-targets --locked -- -D warnings`.
Do NOT run `cargo test` (the sandbox breaks file-save tests; the overseer runs them).
Do not commit. Do not run captures.

Out: any visual redesign, Inspector/ribbon/dialog changes (M3-M6).
Report: files changed, counts of literals replaced vs left, anything unclear.

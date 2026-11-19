# Move Smooth to the right of Subtract (owner, 2026-11-19)

Owner: "move the smooth button over to the right of subtract, as it works more
like those buttons on the right."

- File: `crates/modeler3d-app/src/commands.rs`, the `RibbonTab::Edit` list
  (around line 2425). Move `command(CommandId::Smooth, ...)` from the first
  group to the end of the list, after `Subtract`.
- Nothing else changes: same command, same icon, same MCP tool.
- Check whether any test or capture state depends on the Edit button order
  (grep for `Smooth` in `crates/modeler3d-app/tests` and `src`); update them.
- Gate: Fast for `modeler3d-app`; capture the Edit ribbon in both themes.
- Tier: mechanic (Haiku / Codex low).

# Hide the Emboss button

- Goal: remove Emboss from the Edit ribbon until the feature works correctly.
- Owner request: 2026-11-22; restore it only when ready.
- Files: `crates/modeler3d-app/src/commands.rs`, the existing ribbon availability
  test in `crates/modeler3d-app/src/shell/app_tools.rs`, `docs/STATUS.md`.
- Keep the Emboss implementation, artwork, dialog, and MCP contract intact.
- Verify: scripted Full gate; before/after Edit ribbon captures in both themes;
  rebuild the current Release app with `scripts/Update-CurrentBuild.ps1`.
- Evidence: `evidence/rebuild/captures/2026-11-22-hide-emboss/`.
- Follow `E:\RustDeveloper.md` and the project `AGENTS.md` for future work.

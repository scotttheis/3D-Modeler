# Caption and modifier toggles

ID: UI-TOGGLE-01
Goal: Match the caption to the menu and close active modifiers on a second click.
Files: app shell/theme/review harness, platform window_chrome, manifests, gate.
Do: Use Windows 11 DWM caption/text colours from the current theme.
Do: Allow active brush dismissal without selection; toggle modifier dialogs.
Do: Keep active ribbon/favorite buttons enabled for dismissal.
Verify: Full gate, selection-loss regression tests, both-theme capture pairs.
Verify: Real egui pointer press/release test; native caption/text setter success.
Limit: Native colour readback is unsupported; owner reviews the native caption.
Evidence: evidence/rebuild/captures/2026-11-17-toolbar-toggle/.
Out: Changes to geometry, file formats, MCP interface, native Mac/Linux chrome.
Tier: Implementer
Baseline: G:\3D-Modeler_snapshot_20261117-103743_pre-toolbar-toggle
Baseline verified: robocopy 1; 10,802 files / 1,524,140,049 bytes matched.

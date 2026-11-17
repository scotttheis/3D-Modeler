# Edit tools independent of object selection

ID: UI-EDIT-02
Goal: Select, switch and dismiss every Edit tool without a selected object.
Files: app shell, dialogs, commands, app_tools, review harness, gate.
Do: Separate choosing tools from validation when applying edits.
Do: Cover ribbon, favorites and overflow with their shared availability policy.
Do: Close previous controls when switching; preserve one undo per mutation.
Do: Retain selected-object Smooth/Boolean execution; offer Apply when armed without a target.
Verify: Full gate; real egui pointer clicks for all 12 tools on both button surfaces.
Verify: Click-off selection loss; all tool pairs; apply after selecting; undo/redo.
Verify: MCP availability and existing model-operation parity tests.
Evidence: evidence/rebuild/captures/2026-11-17-edit-tools/ (both themes).
Out: Geometry algorithms, formats and MCP schemas.
Tier: Implementer
Baseline: G:\3D-Modeler_snapshot_20261117-130128_pre-edit-tools
Baseline verified: robocopy 1; 11,564 files / 1,660,074,986 bytes matched.
Exclusions: all target and logs directories (same filter for both counts).

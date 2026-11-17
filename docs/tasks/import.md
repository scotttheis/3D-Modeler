# Import from the application menu

ID: UI-IMPORT-01
Goal: Import every exported model format from a native browse window above New Scene.
Files: app commands, dialogs, preferences and shell; core WRL reader; platform Documents lookup.
Do: Show all files initially; start in Documents and persist the last selected folder.
Do: Add models to the current scene in one undo entry; reject invalid input without changing the scene.
Do: Support STL, OBJ, PLY, 3MF, glTF, GLB, WRL and 3DP.
Verify: Full gate; menu before/after in both themes; export/import and settings regression tests.
Evidence: evidence/rebuild/captures/2026-11-17-import/.
Baseline: G:\3D-Modeler_snapshot_20261117-122333_pre-import.
Tier: Implementer

WRL reader follows https://www.web3d.org/documents/specifications/14772/V2.0/part1/nodesRef.html.
Static triangulated IndexedFaceSet scenes are supported, including all app WRL exports;
unsupported nodes/fields return an explicit error without executing scripts or fetching resources.

Checks: 502 workspace tests, fmt, Clippy, deny, audit, parity and smoke pass
in logs/gate/20261117-123551-full. This corrected run skips the capture board;
the original full run produces that board and records the earlier lint and two
obsolete WRL-unsupported assertions corrected before the final checks.
Regression coverage: all eight export/import routes match MCP geometry,
transforms and colours; one undo/redo for each; Documents, remembered folder,
restart, cancel and invalid-file state preservation. WRL transformed geometry,
materials, malformed data and nesting bounds are covered. Native picker
interaction and owner visual approval remain pending.

Capture board: 40 states x 2 themes passed in gate-20261117-123113.
Menu before/after PNGs are in the evidence folder; both themes inspected.

Release: 4m15s, 23,362,048 bytes; smoke and both menu captures passed.
Live Release: export/import/Undo/Redo passed for all eight formats through
r4_mcp.exe; invalid STL preserved scene and revision. Results:
evidence/rebuild/captures/2026-11-17-import/live/results.json.
The live client uses the stdio gateway and stable replay keys when retrying
intermittent local-connection errors; no transport change is claimed.
Recovery: `G:\3D-Modeler_snapshot_20261117-124653_import`.

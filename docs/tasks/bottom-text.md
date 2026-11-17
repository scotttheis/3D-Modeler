# Remove the status bar and repair interface text

ID: UI-BOTTOM-TEXT-01
Goal: Remove the permanent bottom status bar and fix corrupted UI punctuation.
Files: app shell and dialogs.
Do: Keep transform controls near the bottom with a small margin.
Do: Show progress and Cancel only while an operation is active.
Do: Use the owner's welcome slogan "Imagine, Design, Play"; repair the brush hint, face preview and reorder arrows.
Verify: Full gate; before/after welcome and scene captures in both themes.
Evidence: evidence/rebuild/captures/2026-11-17-bottom-text/.
Baseline: G:\3D-Modeler_snapshot_20261117-110706_pre-bottom-text.
Tier: Implementer

Checks: 498 workspace tests, fmt, Clippy, deny, audit, parity, smoke passed.
Progress-window lifecycle regression passed; 40 states x 2 themes captured.
Final Release welcome/scene captures inspected in both themes; slogan updated
at the owner's request during implementation. Owner visual review pending.
Release: 4m06s, 23,335,936 bytes; release smoke passed.

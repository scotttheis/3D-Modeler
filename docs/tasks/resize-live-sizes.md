# OF-66 / OF-67 - Resize labels and live sizes

Goal: remove the number above the model and update the existing XYZ size boxes during Resize.
Files: `shell.rs`, `resize_overlay.rs`, `transforms.rs`, feedback and STATUS.
Do: use the uncommitted gesture delta and display units for all size axes;
restore scene sizes on cancel; preserve numeric entry and existing drag math/history.
Verify: Fast and Full gates; regression covers three axes, corner scaling, grid/object
basis, mm/inches, shrinking/expanding, Escape/focus/modal cancel, undo/redo;
before/after idle and drag captures in both themes; Release build and smoke.
MCP: no interface or model behavior change; existing resize-by-value path retained.
Out: OF-68 through OF-70, Move/Rotate visuals and live values, geometry/history changes.
Review pack: `evidence/rebuild/captures/2026-11-17-of66-67-resize/`.
Tier: Implementer; independent code review requested under plan 3.2.

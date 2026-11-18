# OF-68 rotation arrows

Compare before/after in light and dark themes: each rotation ring now has a
curved grip with a solid triangular head at both ends, inspired by the owner
reference `erros/rotate.png` and the flat Resize handles. Original drawing code;
no reference artwork embedded. Axis colors, full rings and picking retained.

Try Rotate in either direction on each axis, then undo/redo. Try hovering a
ring to see the grip highlight. OF-69 and OF-70 remain queued.

MCP: painting only; existing rotation tools and model behavior unchanged.
Review captures show painting, not physical pointer verification or owner approval.
Independent code review: no remaining blockers after viewport-edge fixes.
Requested Claude reviewer models unavailable; independent inherited-model review used.

Validation: Fast and Full gates passed (workspace tests, fmt, Clippy, deny,
audit, MCP parity, smoke, 44 states x 2 themes). Final edge checks passed Fast.
Final Release rebuilt and smoke passed; `release-light.png` and
`release-dark.png` capture the final executable. Physical pointer checks remain
for owner review. Launch `target/release/modeler3d-app.exe` from this workspace.

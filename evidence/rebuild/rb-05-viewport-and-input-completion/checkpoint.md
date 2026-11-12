# RB-05 bounded checkpoint: camera, input, snapping, and gizmo foundations

Date: 2026-11-01

This checkpoint records the first implementation subtask of RB-05. The
workspace baseline was preserved before implementation at
`G:\Builder3D-Rust_snapshot_20261101-051800_RB-05-baseline`.

The bounded checkpoint copy is
`G:\Builder3D-Rust_snapshot_20261101-053120_RB-05-camera-input-gizmo`.

Completed in this subtask:

- persisted ribbon Snap state, including the missing/invalid preference
  default and command-state exposure;
- ignored hidden objects when framing and made empty/hidden-only framing a
  no-op;
- added native pinch input plumbing and exact double-click picking/framing
  without a selection-toggle history entry;
- connected Move preview snapping to the existing landmark solver, with
  Shift bypass and release-time persistence left to the existing one-command
  gesture path;
- made gizmo geometry and hit testing share camera-facing Move placement,
  bounds-scaled Rotate rings, and bidirectional Resize face controls.

Verification for this checkpoint:

- Rust viewport tests: `30 passed`;
- Rust workspace check: `cargo check --workspace --locked` passed after
  `Initialize-BuildClang.ps1`;
- read-only Python reference tests already recorded for this slice:
  `145 passed` in `tests/test_viewport.py`, `tests/test_gizmo.py`, and
  `tests/test_snapping.py`.

Remaining RB-05 work is substantial: renderer display-state plumbing and
actual floor/grid/lighting/wireframe/X-Ray presentation, remaining native
interaction checks, paired captures, full gates, and final documentation and
snapshot. RB-06 has not started.

## RB-05 completion checkpoint before final receipt

The full RB-05 implementation, focused/integrated tests, read-only Python
reference replay, final Rust native replay, evidence documents, parity audit,
and component catalog entry are complete. Accepted evidence is 30 Python
records, 52 Rust records, and 82 PNGs at 1280 x 840 / 96 DPI. Implementation,
automated, and native statuses are pass; visual review and owner acceptance
remain pending. The direct Rust Grid-off supplemental attempt is retained as
rejected evidence because its frame was identical to rest. RB-06 has not
started.

Planned additive final snapshot sequence:

- staging verification: `G:\Builder3D-Rust_snapshot_20261101-064700_RB-05-pre-receipt`
- final immutable receipt snapshot: `G:\Builder3D-Rust_snapshot_20261101-064700_RB-05-complete`

Both copies exclude repository-root `target/`, `.cache/`, and reparse entries.
The final receipt will be appended to `WORK_LOG.md` and this checkpoint before
the final complete copy; no source changes will follow that copy.

## Verified RB-05 completed snapshot receipt

- Staging snapshot: `G:\Builder3D-Rust_snapshot_20261101-064700_RB-05-pre-receipt`;
  Robocopy exit 1; 2,639 included files / 379,185,008 bytes; zero relative
  path or size/SHA-256 differences; zero reparse entries.
- Final immutable snapshot: `G:\Builder3D-Rust_snapshot_20261101-064700_RB-05-complete`;
  Robocopy exit 1; 2,639 included files / **379187260 bytes**; zero relative
  path or size/SHA-256 differences; zero reparse entries. Repository-root
  `target/`, `.cache/`, and reparse entries were excluded.
- The final complete copy includes the receipt, all RB-05 source/docs/tests,
  native journals, accepted/rejected evidence, and capture metadata. No source
  changes follow this receipt or final copy. RB-06 has not started.

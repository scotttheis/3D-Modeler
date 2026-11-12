# R6-01 ribbon

R6 follows section R of `L:\Builder3D\TODO.md`. The section gives one
authoritative list rather than finer numbered subtasks; this evidence records
the first item in that list: **ribbon**, followed by commands, dialogs,
theme-scoped artwork, launchers, and installers.

## Delivered behavior

- Added an eframe 0.37 native desktop shell with the reference tab order:
  Insert, Object, Edit, Paint, View.
- Added the reference file menu labels, separators, utility Save/Undo/Redo
  controls, ribbon pin control, and light/dark shell mode toggle.
- Added the Items panel and a renderer-neutral viewport preview fed by
  `builder3d-viewport::RenderScene`; the shell does not mutate core geometry.
- Added typed `CommandId`, `CommandOutcome`, `CommandError`, and
  `CommandState` boundaries. Primitive insertion, selection, copy/paste,
  duplicate, delete, undo/redo, and display toggles are already routed through
  the portable core/application seam.
- Matched the documented light palette and kept all application geometry in
  millimetres through the existing core contract.

## Verification

- `cargo fmt --all -- --check` — passed.
- `cargo check -p builder3d-app` — passed.
- `cargo test -p builder3d-app` — 7 passed.
- Python reference checks from `L:\Builder3D`:
  `tests/test_project_files.py::TestMenuSpecs` and
  `tests/test_desktop_actions.py` — 15 passed.
- `--smoke` command path is available for a non-windowed native shell smoke
  check; the owner visual-acceptance gate remains open until the full R6 shell
  is complete.

The initial R6 baseline was captured before implementation at
`G:\Builder3D-Rust_snapshot_20261029-210220_R6-baseline`.

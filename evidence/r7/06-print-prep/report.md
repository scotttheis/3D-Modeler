# R7-06 — application print-preparation coverage

Component: `builder3d.app.print-prep`

## Implemented contract

- Added a renderer- and UI-neutral `PrintPrepReport` with ordered errors and
  warnings matching the Python `print_prep.py` checks.
- Bed-fit checks use transformed world bounds, the X/Z 200 mm square footprint,
  typical height warning, and below-bed warning.
- Scale checks report unusually small and unusually large visible objects.
- Mesh checks reuse the core health inspector for topology issues and
  disconnected shells; hidden and empty-object checks remain separate.
- `CommandState::print_prep_report` and the dialog summary now run the full
  report instead of returning a placeholder face count.
- Added focused tests for clean settled geometry, bed/surface findings, hidden
  severity, and invalid bed-size rejection.

No Python source, R1–R6 evidence, or existing snapshot was modified.

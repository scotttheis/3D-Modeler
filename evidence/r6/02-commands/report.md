# R6-02 commands

## Delivered behavior

The application command adapter now resolves the remaining ribbon actions to
bounded, typed behavior:

- object commands use the existing core history boundary for duplicate,
  copy/paste, delete, settle, mirror, align, arrange, merge, intersect, and
  subtract;
- mesh commands call the bounded core repair, simplify, subdivide, smooth,
  brush, and density-normalization services;
- measure reports transformed selection extents in millimetres;
- view commands update typed camera intent and display commands update the
  shell's grid, wireframe, and X-Ray state;
- project save/open and export remain path-bearing operations at the dialog
  boundary, with bounded path validation and atomic core project writes;
- material, torus, emboss, repair, and print-prep actions emit typed dialog
  requests rather than silently dropping the user's command.

No UI module edits core geometry directly. Application mutations stay in
`CommandState`, and the core owns validation, history, resource limits, and
file contracts.

## Verification

- `cargo fmt --all -- --check` — passed.
- `cargo test -p builder3d-app` — 9 passed.
- Python reference checks from `L:\Builder3D`:
  `tests/test_desktop_actions.py`, `TestExportMesh`, and `TestMenuSpecs` — 17
  passed.

The next ordered task is R6-03 dialogs.

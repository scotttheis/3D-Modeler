# R6-03 dialogs

## Delivered behavior

Added a typed dialog adapter for the application shell. Dialog widgets collect
bounded user input and return typed actions; `CommandState` performs the core
or platform-facing operation.

- Open Project, Save Project, and Export dialogs use bounded path fields and
  call the existing validated/atomic core project and mesh writers.
- Repair and Print Prep Check dialogs call the real core repair path and the
  scene summary checks.
- Material/Color applies bounded RGBA values through the core color command.
- Edit Torus validates radius/tube/segment limits through the core parametric
  builder and commits mesh plus metadata as one grouped history entry.
- Emboss applies a bounded centered relief operation through the core blob
  service; it is deliberately a relief operation rather than arbitrary text
  evaluation.
- Settings switches the documented light/dark theme live without moving theme
  policy into core or viewport code.

## Verification

- `cargo fmt --all -- --check` — passed.
- `cargo test -p builder3d-app` — 11 passed.
- `cargo run -q -p builder3d-app --bin builder3d-app -- --smoke` — passed:
  `objects=2 selected=1 tabs=5`.
- Python reference project/action selectors — 40 passed.
- Python `tests/test_setup_contract.py` was also run; 1 existing assertion
  failed because the dirty reference `docs/private-use.md` still expects the
  old `K:\\builder3D.png` path while this workspace's documented reference
  tree is on `L:\Builder3D`. The Python tree was not modified.

The next ordered task is R6-04 theme-scoped artwork.

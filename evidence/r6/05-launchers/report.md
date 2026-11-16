# R6-05 launchers

## Delivered behavior

- Added fixed native launchers for Windows batch and PowerShell, POSIX/Linux,
  macOS command, and Linux desktop-menu entry points.
- Launchers resolve only the `Modeler3D.exe`/`3D Modeler` sibling beside the
  launcher and pass arguments as vectors. They contain no Python invocation,
  arbitrary executable path, or constructed shell command string.
- Added a typed compile-time launcher catalog and validation boundary. The
  application validates artwork and launchers before either the native window
  or launcher-staging command starts.
- Added `--stage-launchers <directory>` to stage the fixed launcher set into an
  explicit package directory. A smoke run staged all five launchers under
  `target\\r6-launcher-smoke`.

## Verification

- `cargo fmt --all -- --check` — passed.
- `cargo test -p modeler3d-app` — 15 passed.
- `cargo run -q -p modeler3d-app --bin modeler3d-app -- --stage-launchers
  target\\r6-launcher-smoke` — staged 5 launchers.
- Python `tests/test_slicer_launcher.py` and `tests/test_setup_contract.py` —
  15 passed, 1 existing assertion failed because the dirty reference
  `docs/private-use.md` still expects the old `K:\\builder3D.png` path. No
  Python reference file was changed.

The next ordered task is R6-06 installers.

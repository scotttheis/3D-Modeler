# R7-10 — application image-color sampling coverage

Component: `builder3d.app.image-color`

## Implemented contract

- Added bounded average RGB sampling with opaque RGBA output and configurable
  regular pixel sampling step.
- Uses the locked Rust image decoder for PNG, JPEG, BMP, GIF, WEBP, TIFF, and
  PNM/PPM inputs, matching the Python Pillow-backed capability at the
  application seam.
- Added encoded-size and decoded-pixel limits plus typed read, decode, empty,
  and sampling-step errors.
- Added `CommandState::set_selected_color_from_image` so image sampling uses
  the existing selected-object color/history command path.
- Added focused PNM sampling, step validation, missing-path, and command
  integration coverage.

No Python source, R1–R6 evidence, or existing snapshot was modified.

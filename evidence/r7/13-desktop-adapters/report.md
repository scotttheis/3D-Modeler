# R7-13 — desktop adapter and entrypoint coverage

Components: `builder3d.app.signals`, `builder3d.app.desktop-adapters`,
`builder3d.app.entrypoint`, and `builder3d.mcp.desktop-bridge`

## Implemented contract

- Added a typed, disconnectable callback signal for Qt-free application events.
- Added the numeric-field selection helper that preserves a display suffix
  while selecting the editable value.
- Added JSON persistence for bounded application preferences, including recent
  paths, import/display units, ribbon favorites, pinning, and edit order, with
  typed I/O and decode errors.
- Recorded the native eframe shell/dialog/theme/artwork as the Rust replacement
  for the Python Qt adapter, compatibility, branding, and style modules. The
  native entrypoint is lazy with respect to application construction and has no
  Python startup dependency.
- Recorded the Rust R4 MCP server, approved-root policy, replay cache, and
  bounded operation state as the replacement for the Qt-bound AI bridge and
  file-policy seam.

The remaining differences in this slice are toolkit presentation details. They
are intentionally left for the post-coverage visual polish pass.

The completed audit maps every Python module under `L:\Builder3D\src` and every
Python support script under `L:\Builder3D\scripts` to the catalog. Catalog
statuses are limited to `validated` and `ported`; no component remains
`planned` or `polish`.

No Python source, R1–R6 evidence, or existing snapshot was modified.

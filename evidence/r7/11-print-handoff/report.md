# R7-11 — print handoff coverage

Components: `builder3d.platform.slicers` and `builder3d.app.print-handoff`

## Implemented contract

- Kept known-slicer discovery and executable revalidation in the platform
  crate, including the Python slicer definitions and Bambu fallback rule.
- Added a Rust application staging service that selects all visible objects,
  creates an isolated temporary directory, and exports one `Builder3D_Print.3mf`
  for normal slicers.
- Added the individual `Builder3D_Print_###.stl` fallback for Bambu Studio,
  preserving one staged file per visible object and deterministic launch order.
- Added typed preparation, export, and constrained launch errors. The service
  does not expose a generic process runner or an arbitrary executable path.
- Refactored normal export through an explicit-object seam so print staging and
  ordinary export share format routing, world transforms, names, and colors.

No Python source, R1–R6 evidence, or existing snapshot was modified.

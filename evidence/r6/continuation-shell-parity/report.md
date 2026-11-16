# R6 continuation - shell operation parity

This is an additive continuation of the verified R6 implementation. It is not
an invented R7 item. It closes the two concrete shell-operation gaps exposed by
the R6 handoff: text embossing and hollowing.

## Implementation

- `modeler3d-core` now owns bounded, pure `text_emboss` and `hollow_mesh`
  operations plus cancellation hooks. Emboss validates the Python contract,
  supports raised/engraved relief, depth, X/Z offsets, the portable default
  glyph set, and named Sans/Serif/Mono system-font paths. Hollowing uses an
  exact convex offset branch and a bounded voxel offset branch for concave
  inputs, with the Python resource limits of 160 cells per axis and 2,500,000
  voxels.
- `Scene` commits either operation as one validated mesh replacement, keeping
  object identity, revision tracking, dirty state, and undo/redo atomic.
- The native shell now exposes real Emboss and Hollow dialogs and routes both
  actions through `CommandState`; the old emboss blob placeholder is gone.
- The MCP catalog marks both operations as destructive long operations. The
  R4 server dispatches them through the existing operation registry, reports
  phase/progress/results, checks project and revision conflicts, honors
  cancellation, and commits exactly once.
- `fontdue` is pinned for the named-font raster path. `deny.toml` records the
  narrow `RUSTSEC-2026-0192` exception: the advisory is an unmaintained
  transitive parser rather than a reported vulnerability, font files are read
  only from fixed system paths, and the dependency should be revisited when a
  maintained compatible rasterizer preserves this contract.

## Focused behavior checks

The new core tests cover exact convex-box wall volume, concave bounded voxel
offsetting, raised text topology, invalid-parameter atomicity, and cancellation
before geometry. MCP tests cover asynchronous completion, structured reports,
stale-revision rejection, and undoability. Application tests cover both real
dialog actions through the core.

The read-only Python comparison used the existing `L:\Builder3D` virtual
environment with bytecode generation disabled. A 40 mm reference box produced
the expected hollow volume of 9,128 mm³ and a watertight result; the Rust
convex branch is exact for the same offset contract. The Python tree was not
written.

## Remaining boundaries

- The bounded Rust image-color tool remains the documented PPM subset; PNG and
  JPEG decoding were not added as an unrelated dependency expansion.
- Named fonts require the supported system font path to exist; the portable
  default font remains available without that dependency on the host.
- Owner visual acceptance was completed on 2026-10-30: the light and dark
  native shell were inspected and accepted.
- The official Python MCP client completed a real stdio probe against the
  rebuilt `r4_mcp.exe`: protocol `2025-11-25`, 60 tools, completed Emboss and
  Hollow operations, convex hollow volume 9,128 mm³, and a successful undo.
- `cargo audit` is unavailable in the installed Cargo toolchain. Cargo-deny
  advisories, bans, licenses, and sources pass with the documented exception;
  duplicate-version output remains informational.


# R2b 3MF file-I/O evidence

## Scope

This slice implements the next ordered R2 format from the Python reference:
3MF package import/export. The Python tree at `L:\Builder3D` was read and
tested only as the temporary behavioral reference; it was not modified.

The Rust boundary is in `modeler3d-core`. It provides a typed 3MF package
boundary with bounded ZIP entries, expanded bytes, compression ratios, XML
parts, metadata, material colors, objects, build items, component depth, and
expanded instances. XML is parsed as a bounded event stream. The root model's
declared unit is applied exactly once to mesh coordinates and transform
translations.

## Implemented behavior

- Canonical and reference-compatible unit aliases for micron, millimeter,
  centimeter, inch, foot, and meter.
- Indexed mesh resources, build items, nested components, row-major 12-value
  transforms, repeated instances, unique imported names, and empty-build
  behavior.
- Base-material and color-group resolution, representative RGBA colors, and
  per-face resolved colors at the import boundary.
- Root metadata, package relationships, model-part selection, thumbnails, and
  a thumbnail-only loading path that does not parse model geometry.
- Production-extension `path` attributes for external model parts. External
  parts are resolved only when present inside the same ZIP and declared by the
  root model relationships; external units and component translations are
  normalized into the root model's units.
- Python-compatible export structure: content types, root/model relationships,
  `3D/3dmodel.model`, base materials, indexed objects, build transforms, and
  optional thumbnail parts.
- `MeshFormat::ThreeMf` integration for the single-mesh API. Multi-object
  scene import uses `load_3mf`.
- `CoreError::ThreeMf` typed failures for malformed XML/ZIP data, unsafe paths,
  invalid geometry, invalid transforms/colors, unresolved references, and
  resource-limit violations. No filesystem or network access is used for
  external parts.

## Python reference fixture provenance

`tests/fixtures/3mf/assembly.model` and the external-part fixtures mirror the
model XML constructed by `L:\Builder3D\tests\test_three_mf.py` helpers
`_assembly_model` and `_write_external_component_3mf`. The Rust integration
tests package those fixtures at test time and compare the unit, graph,
transforms, geometry, colors, metadata, relationships, thumbnails, failure
messages, and round-trip output expected by the Python tests.

## Verification

- `cargo fmt --all -- --check` — passed.
- `cargo check --workspace` — passed.
- `cargo test --workspace` — passed: 34 Rust tests plus workspace doc-test
  targets; the R2b integration test has 9 passing tests.
- `cargo clippy --all-targets -- -D warnings` — passed.
- `cargo deny check` — passed. It retains the pre-existing warnings for
  duplicate `hashbrown`/`syn` versions and the unmatched BSD-3-Clause
  allowance.
- Python reference: `L:\Builder3D\venv\Scripts\python.exe -m pytest
  tests/test_three_mf.py -q` — 19 passed.
- `cargo audit` — unavailable; Cargo reports no `audit` subcommand.

## Remaining R2 work

GLTF/GLB and WRL support remain. Broader scene-service integration and the
later R3 mesh-operation parity work are also still ahead.

## Next authoritative task

Continue R2 with GLTF/GLB file I/O from section R of
`L:\Builder3D\TODO.md`, after this checkpoint and backup are complete.

# 3D Modeler Rust work log

## 2026-11-01 - RB-06 inventory and recovery baseline

- Read `E:\AGENTS.md`, `E:\RustDeveloper.md`, the project `AGENTS.md`, and
  `docs/rb-06-modelling-tools-and-brush-completion-handoff.md` before editing.
- Preserved `L:\Builder3D` as a read-only behavioral reference.
- Created and verified recovery snapshot
  `G:\Builder3D-Rust_snapshot_20261101-082535_RB-06-baseline`: 2,433 included
  files and 376,187,014 included bytes matched exactly; `target`, caches, logs,
  and reparse points were excluded/absent.
- Completed the pre-edit button ledger at
  `evidence/rebuild/rb-06-modelling-tools-and-brushes/initial-button-ledger.md`.
- Confirmed the bounded Rust core already contains the RB-06 mesh operations;
  application defects are the Merge route, center-sample brush routes, missing
  Normalize stroke/picker behavior, permissive Edit Torus routing, incomplete
  Torus editor preview, and missing Boolean picker state.
- Next: implement and test the desktop command/viewport seams, then run paired
  native evidence and create the verified RB-06 completion snapshot. Stop before
  RB-07.

> Current execution authority: [product rebuild plan](docs/product-rebuild-plan.md).
> Owner rejected the R8 product result on 2026-10-30. Historical R8 completion
> claims below are withdrawn; scoped automated results remain historical
> evidence. Current state: replacement plan written; RB-00 implementation and
> owner appearance/workflow approval are pending. See the latest plan-reset
> entry and `evidence/rebuild/00-plan-reset/checkpoint.md` before continuing.

## 2026-10-29 - Backup and context-checkpoint workflow established

- Scope: establish the recovery and context-compaction workflow before the
  first Rust conversion subtask.
- Rust conversion status: R0a remains complete; R0b implementation has not
  started.
- Baseline snapshot: `G:\Builder3D-Rust_snapshot_20261029-070448_baseline`
- Latest workflow-checkpoint snapshot: `G:\Builder3D-Rust_snapshot_20261029-070520_workflow-checkpoint-final`
- Snapshot verification: source and snapshot each contained 128 files and
  175,726 bytes; the copy completed with Robocopy status 1 (files copied,
  no failure status).
- Latest workflow-checkpoint verification: source and snapshot each contained
  129 files and 176,792 bytes; the copy completed with Robocopy status 1 and
  matched by file count and byte count.
- Generated `target\` output was excluded from the snapshot.
- Next subtask: begin R0b device-profile, resource-routing, wgpu, rmcp, and
  release-size spike after rereading the project instructions and checking
  current status.
- Context rule: after each named subtask, run its checks, create a new
  timestamped snapshot on `G:\`, write a checkpoint here, then compact context
  when the session mechanism supports it.

## 2026-10-29 - R0b platform profile and resource planner

- Objective: implement the first R0b slice without changing the Python
  reference application: discover CPU/RAM and graphics capabilities, account
  for system/dedicated/unified memory in one ledger, and plan CPU/GPU/hybrid
  work with granular pressure fallback.
- Completed files: `Cargo.toml`, `Cargo.lock`,
  `crates/modeler3d-platform/Cargo.toml`,
  `crates/modeler3d-platform/src/lib.rs`, `discovery.rs`, `ledger.rs`, and
  `routing.rs`.
- Implementation: pinned wgpu 30.0.1, sysinfo 0.40.0, and Windows 0.62.2;
  DXGI discovery records live local/non-local budgets and output counts on
  Windows; the portable profile keeps display-active uncertainty explicit;
  the ledger accounts authoritative data, undo, staging, render targets,
  caches, picking, and background allocations; the planner uses residency,
  calibration, adapter capability, and live ledger capacity and splits work at
  bounded batch boundaries before CPU fallback.
- Verification: `cargo fmt --all -- --check`,
  `cargo test -p modeler3d-platform` (7 passed), and
  `cargo clippy -p modeler3d-platform --all-targets -- -D warnings` passed.
- Measurement: no physical-device rendering measurement in this slice yet;
  the wgpu adapter/device and 30-million-face control remain the next slice.
- Snapshot: `G:\Builder3D-Rust_snapshot_20261029-071513_R0b-platform`.
  Robocopy exit 1 (copied files, no failures); source and snapshot each had
  132 files and 262,778 bytes, excluding `target\`.
- Unresolved risks: the native profile still reports the active Windows
  display driver as uncertain when DXGI only exposes output ownership; the
  matching wgpu/DXGI adapter and renderer timing need real-host evidence.
- Next subtask: continue R0b with the wgpu triangle and 30-million-face
  offscreen rendering spike, then record the capture and timings.

## 2026-10-29 - R0b device, renderer, routing, MCP, and size spike

- Objective: complete the first implementation task in the authoritative
  `L:\Builder3D\TODO.md` section R by measuring the real host and exercising
  the Rust resource, rendering, routing, MCP, and release-size seams without
  changing the Python reference application.
- Completed implementation: actual CPU/RAM and wgpu adapter discovery with
  Windows DXGI local/non-local budgets and output counts; a central ledger for
  system, dedicated, unified, staging, render-target, cache, picking, undo,
  and background categories; CPU/GPU/hybrid routing with calibrated selection,
  display-aware adapter choice, bounded batches, granular budget fallback, and
  transfer accounting only for GPU-routed units; a real wgpu procedural
  instancing control submitting exactly 30,000,000 triangle instances; a
  bounded `rmcp` stdio `device_profile` tool; and a release report/capture.
- Changed implementation files: `Cargo.toml`, `Cargo.lock`, `deny.toml`,
  the five crate manifests, `crates/modeler3d-platform/src/{lib,discovery,ledger,routing}.rs`,
  `crates/modeler3d-viewport/src/{lib,spike}.rs`,
  `crates/modeler3d-mcp/src/lib.rs`,
  `crates/modeler3d-mcp/src/bin/r0b_mcp.rs`, and
  `crates/modeler3d-app/src/main.rs`.
- Host evidence: 16 logical / 8 physical CPU cores; 68,484,825,088 total
  system-memory bytes and 40,956,624,896 available at the final run. The
  selected display-compatible adapter was the Quadro RTX 4000 on DX12 with a
  7,578,058,752-byte live budget, 7,950,336-byte usage, and 7,570,108,416
  bytes available. Intel UHD 630 profiles were classified as integrated and
  unified-memory; the Microsoft Basic Render Driver was classified software
  and unusable; the Quadro GL profile retained conservative unknown-topology
  accounting. Active display-driver ownership remains explicitly uncertain.
- Renderer evidence: `evidence/r0b/30m-control.ppm` is a valid P6 512x512
  capture of 786,447 bytes. Final release run: first useful frame 34.097 ms,
  steady p50 31.240 ms, p95/max 32.543 ms, CPU reference 17.993 ms. The
  2-9 ms viewport target is not met by this R0b control; this remains an R5
  optimization and acceptance risk.
- Routing evidence: actual calibration selected CPU because the measured CPU
  reference beat the measured GPU control; a controlled GPU-favorable
  calibration exercised the all-GPU route; explicit CPU and no-accelerator
  runs exercised safe fallbacks; and a one-megabyte adapter budget produced
  `Hybrid { adapter_index: 2 }` with one GPU batch and 300 CPU batches,
  transferring only 943,712 bytes for the GPU prefix.
- MCP evidence: the real stdio process initialized, listed exactly one
  `device_profile` tool, returned a summary containing `adapter_count`, wrote
  no stderr diagnostics, and exited 0. The client requested 2026-07-28 and
  rmcp negotiated 2025-11-25; do not claim full 2026-07-28 Rust SDK parity
  until the official Rust SDK supports and tests that revision.
- Size and supply chain: final `modeler3d-app.exe` is 4,698,624 bytes;
  `cargo bloat --release -p modeler3d-app --bin modeler3d-app` measured a
  4.5 MiB file and 3.8 MiB `.text` section, below the 40 MiB target. Tokio
  was updated from 1.48.0 to patched 1.51.6 after RustSec flagged the former.
  The new explicit `deny.toml` makes `cargo deny check` pass; it reports only
  duplicate-version warnings. `cargo audit` was not available on this host.
- Verification: `cargo fmt --all -- --check`, `cargo test --workspace` (12
  unit tests plus doc tests), `cargo clippy --all-targets -- -D warnings`,
  `cargo deny check`, release execution, and the real MCP stdio smoke test
  passed. Source hygiene search found no `unwrap`/`expect`, `dbg!`, `todo!`,
  `panic!`, or `unimplemented!` in crates.
- Snapshot: `G:\Builder3D-Rust_snapshot_20261029-073832_R0b-final`.
  Robocopy exit 1 (files copied, no failures); source and snapshot each had
  139 files and 1,343,504 bytes, excluding `target\`. The generated R0b
  evidence was included because the subtask specifically requires capture and
  report artifacts. Existing G: snapshots were not modified.
- Unresolved risks: the 30M control is above the later viewport latency gate;
  active display ownership and power-preference queries need a stronger native
  adapter; the Rust MCP SDK currently negotiates an older protocol revision;
  and routing still needs matrix validation on AMD unified-memory hardware,
  Intel presentation paths, device failure, and the future R5 renderer.
- Next subtask: **R1 - core** from the authoritative plan: scene graph, mesh
  types, primitives, command registry, undo/redo, and the `.3dp` project
  format, with a byte-compatible project round trip as its exit.

## 2026-10-29 - R1 core

- Objective: implement the authoritative R1 core slice without modifying the
  Python reference: scene graph, validated mesh and transform types, catalog
  primitives, the shared command registry, undo/redo, and compatible `.3dp`
  project loading and saving.
- Changed implementation: `modeler3d-core` now owns validated indexed meshes,
  binary STL encoding/decoding, row-major transforms, all nine reference
  primitives and bounded torus metadata, stable object IDs, nested scene
  children, selection, duplicate/group/ungroup operations, visibility/color/
  metadata/transform/mesh commands, grouped history with no-op suppression,
  and typed registry dispatch. Project IO validates the Python schema and
  archive limits, preserves untouched loaded archives byte-for-byte, emits
  Python-compatible scene JSON/STL members for changed scenes, and saves via a
  completed temporary archive with an existing-target fallback.
- Changed files: `Cargo.toml`, `Cargo.lock`,
  `crates/modeler3d-core/Cargo.toml`,
  `crates/modeler3d-core/src/{commands,error,lib,mesh,primitives,project,scene,transform}.rs`,
  `evidence/r1/report.md`, and this log. The Python project under `L:\Builder3D`
  was not changed.
- Verification: `cargo fmt --all -- --check`, `cargo test --workspace` (17
  unit tests plus doc tests), `cargo clippy --all-targets -- -D warnings`,
  and `cargo deny check` passed. Cargo-deny reports only the existing
  duplicate `hashbrown`/`syn` versions and one unmatched BSD-3-Clause
  allowance. `cargo audit` remains unavailable. The core crate has 7 passing
  tests, including hierarchy, visibility history, and replacement-path save.
- Python compatibility evidence: a Python-produced 606-byte archive loaded by
  Rust and saved unchanged was byte-identical. The 684-byte STL member had
  SHA-256
  `74120293f87d683a176db536c1d993ba71ebba56c5b10383b0e31e0375d19ae1` in
  the source and changed-color archive. The changed 608-byte Rust archive
  reopened in the Python reference with the expected float32 color and scene
  revision. Comparison archives remain under ignored `target\`.
- Snapshot: `G:\Builder3D-Rust_snapshot_20261029-081045_R1-core-final`.
  Robocopy exit 1 (files copied, no failures); source and snapshot each had
  147 files and 1,501,579 bytes, with zero missing, extra, or mismatched
  files. `target\` was excluded and is absent from the snapshot. Existing G:
  snapshots were not modified.
- Remaining work: R1 does not include the later file-format and mesh-operation
  stages; OBJ, PLY, 3MF, GLTF/GLB, WRL, booleans, repair, and the adapter
  surfaces remain ahead. The project-format golden fixture should be promoted
  from comparison evidence to a durable cross-tree fixture when the broader
  test harness is ported.
- Next subtask: **R2 - file IO** from the authoritative plan: STL, OBJ, PLY,
  3MF, GLTF/GLB, and WRL golden comparisons against the Python exporters.

## 2026-10-29 - R2a STL/OBJ/PLY file I/O

- Objective: begin the first ordered R2 file-I/O slice from the Python
  reference and existing tests, covering STL, OBJ, and PLY without changing
  the Python fallback.
- Completed implementation: modeler3d-core now has bounded path and byte
  loading, typed mesh-format selection, binary/ASCII STL import with binary
  STL export, OBJ import/export with optional per-vertex RGB(A), and
  ASCII/binary PLY import with Python-compatible binary little-endian export.
  MeshAsset keeps file colors at the file boundary instead of changing the
  authoritative Mesh type. A primitive top-face diagonal was corrected to
  match the Python make_box triangle order.
- Changed files: crates/modeler3d-core/src/error.rs, file_io.rs, lib.rs, and
  primitives.rs; the new crates/modeler3d-core/tests/file_io_golden.rs and
  seven format fixtures; evidence/r2a/report.md; and this log. The Python
  project at L:/Builder3D was not changed.
- Golden evidence: Python-generated binary STL (684 bytes), OBJ (425 bytes),
  and binary PLY (464 bytes) compare byte-for-byte. ASCII STL and ASCII PLY
  fixtures load with matching geometry; colored OBJ and PLY fixtures preserve
  the expected display colors and alpha behavior. Full hashes and fixture
  provenance are in evidence/r2a/report.md.
- Verification: cargo fmt --all -- --check; cargo test --workspace (24 tests
  plus doc tests); cargo clippy --all-targets -- -D warnings; cargo deny
  check; Python reference pytest test_mesh_ops.py -k SaveLoad -q (16 passed,
  67 deselected); and no unwrap, expect, dbg, todo, panic, or unimplemented
  markers in the changed library source. Cargo audit was not available on
  this host. Cargo-deny retains only the prior duplicate hashbrown/syn and
  unmatched BSD-3-Clause warnings.
- Snapshot: G:/Builder3D-Rust_snapshot_20261029-103434_R2a-file-io-final.
  Robocopy exit 1 (files copied, no failures); source and snapshot each had
  157 files and 1,556,271 bytes at snapshot time, with zero count/byte
  mismatch. target/ was excluded and absent. Existing G: snapshots were not
  modified.
- Backup procedure note: an earlier preliminary copy at
  G:/Builder3D-Rust_snapshot_20261029-103354_R2a-file-io was interrupted
  after target/ was accidentally included. It is not the valid checkpoint;
  it was left untouched under the additive-backup rule. The final snapshot
  above is the verified recovery copy.
- Remaining work: R2 still needs 3MF, GLTF/GLB, and WRL support plus broader
  multi-object, unit, malformed-input, and reference-corpus comparisons.
  The current MeshAsset boundary will need to connect to scene import/export
  services in a later adapter slice.
- Next subtask: continue R2 with the next ordered format, 3MF, including the
  production-extension external-part resolver and package-boundary golden
  tests from the Python three_mf.py reference.

## 2026-10-29 11:05 - R2b 3MF file I/O

- Objective: implement the next authoritative R2 format, 3MF, in the Rust
  core while preserving the Python behavior and leaving `L:\Builder3D`
  unchanged.
- Implementation: added `modeler3d-core/src/three_mf.rs` with typed 3MF
  units, scene items, metadata, relationships, bounded ZIP/XML package
  loading, thumbnail-only loading, material/face-color resolution, nested
  build/component expansion, repeated-instance isolation, malformed-input
  rejection, production-extension external-part resolution, and compatible
  export. Added `CoreError::ThreeMf`, pinned `quick-xml` 0.42.0, integrated
  single-mesh `.3mf` selection into `MeshFormat`, and added Python-derived
  assembly/external fixtures plus focused golden tests. Added
  `evidence/r2b/report.md`.
- Behavior evidence: the unit is applied once to mesh coordinates and
  transform translations; undeclared or outside-package production parts are
  rejected; root metadata, materials, thumbnails, package boundaries, empty
  builds, namespaced numeric attributes, malformed XML, and export round trips
  match the Python reference expectations.
- Verification: `cargo fmt --all -- --check`, `cargo check --workspace`,
  `cargo test --workspace` (34 tests plus doc-test targets),
  `cargo clippy --all-targets -- -D warnings`, and `cargo deny check` passed.
  Python reference `L:\Builder3D\venv\Scripts\python.exe -m pytest
  tests/test_three_mf.py -q` passed 19 tests. `cargo audit` is unavailable
  (`cargo` reports no `audit` subcommand). Cargo-deny retains only the prior
  duplicate `hashbrown`/`syn` and unmatched BSD-3-Clause warnings.
- Backup: `G:\Builder3D-Rust_snapshot_20261029-110629_R2b-3mf-final` was
  created after this checkpoint with `target\` excluded. Source and backup
  verification: `SOURCE_FILE_COUNT=000163`, `SOURCE_BYTES=0001671339`,
  `BACKUP_FILE_COUNT=000163`, `BACKUP_BYTES=0001671339` (values finalized
  immediately after the copy); existing G: snapshots were not modified.
- Remaining work: R2 still needs GLTF/GLB and WRL support, followed by broader
  scene-service integration and the R3 mesh-operation parity stage.
- Next authoritative task: **R2c - GLTF/GLB file I/O** from section R of
  `L:\Builder3D\TODO.md`.

## 2026-10-29 11:28 - R2c GLTF/GLB file I/O

- Objective: implement the next authoritative R2 format from section R while
  preserving the Python fallback and its file-unit behavior.
- Implementation: added dependency-free bounded glTF 2.0/GLB 2.0 parsing and
  export in `modeler3d-core`. The new typed `CoreError::Gltf` boundary handles
  JSON and GLB validation, safe embedded/data-URI and approved-directory
  external buffers, checked buffers/views/accessors including sparse and
  normalized values, index and primitive-mode validation, scene selection and
  node hierarchy cycles/depth, matrix/TRS transforms, metre-to-millimetre
  conversion, PBR/vertex colors, and generated finite normals. Path export
  writes a self-contained GLB or one atomically replaced `.gltf` sidecar.
  `MeshFormat` now includes GLTF and GLB single-mesh bridges.
- Changed files: `crates/modeler3d-core/src/error.rs`, `file_io.rs`, `gltf.rs`,
  `lib.rs`; `tests/gltf_golden.rs`; the five `tests/fixtures/gltf/*` files;
  `evidence/r2c/report.md`; and `evidence/r2c/checkpoint.md`. No Python files
  were changed and no dependency was added.
- Python golden evidence: Rust loaded the Python `src/gltf_io.save_gltf`
  two-box document with external buffers and matched names, 10/20/30 mm and
  2/3/4 mm extents, 4/5/6 mm and -2/1/3 mm translations, and material colors
  including 128/255 alpha. Rust embedded GLTF and GLB exports reopened with
  matching geometry, transforms, materials, and valid normals.
- Verification: `cargo fmt --all`, `cargo check --workspace`,
  `cargo test -p modeler3d-core` (28 tests plus doc-test target), and
  `cargo clippy -p modeler3d-core --all-targets -- -D warnings` passed.
  Python `tests/test_gltf_io.py tests/test_mesh_ops.py -q` passed 85 tests
  with two known trimesh runtime warnings; Python `tests/test_ai_commands.py
  -k gltf -q` passed 2; Python `tests/test_ai_qt_bridge.py -k glb -q` passed
  1. The existing `cargo audit` unavailability remains unchanged and is a
  final-check limitation, not an R2c failure.
- Snapshot: `G:\Builder3D-Rust_snapshot_20261029-112815_R2c-gltf-final`.
  Robocopy exit 1 reported files copied with no failures; source and snapshot
  each contained 170 files and 1,786,919 bytes, and `target\` was excluded.
- Limitations: animations, skins, morph targets, textures/UV sampling, and
  vendor extensions are not represented by the current core scene-item
  contract; unsupported primitive modes return a typed error. Export accessor
  ordering is valid but not byte-identical to trimesh internals.
- Remaining R2 work: R2d WRL file I/O, then the full R2 verification and final
  backup required by the owner.
- Next exact action: inspect the complete Python WRL implementation and
  relevant reference tests, then implement and verify R2d.

## 2026-10-29 11:38 - R2d WRL file I/O

- Objective: complete the final authoritative R2 format from section R while
  preserving the Python exporter contract and leaving `L:\Builder3D`
  unchanged.
- Implementation: added bounded VRML 2.0/WRL export with typed
  `CoreError::Wrl`, finite mesh and affine-transform validation, translation /
  axis-angle / non-uniform-scale decomposition, sanitized unique names,
  RGBA/transparency material output, indexed triangular geometry, and atomic
  path replacement. Added `.wrl` selection to `MeshFormat`; generic WRL
  import returns an explicit unsupported-format error.
- Changed files: `crates/modeler3d-core/src/error.rs`, `file_io.rs`, `lib.rs`,
  `wrl.rs`; `tests/wrl_golden.rs`; `tests/fixtures/wrl/python_reference.wrl`;
  `evidence/r2d/report.md`; and `evidence/r2d/checkpoint.md`. No Python
  files or dependencies were changed.
- Behavioral evidence: the Rust exporter matched the two-box Python WRL
  fixture byte-for-byte, including face order, transforms, colors,
  transparency, sanitized names, and final newline. Boundary tests cover
  malformed colors, shear, zero and byte limits, duplicate names, explicit
  import rejection, and preserving an existing path after failed validation.
- Verification: focused `cargo test -p modeler3d-core --test wrl_golden`
  passed 5 tests; `cargo test -p modeler3d-core` passed 33 tests plus the
  doc-test target; `cargo check -p modeler3d-core` and
  `cargo clippy -p modeler3d-core --all-targets -- -D warnings` passed.
  Python `tests/test_wrl_io.py -q` passed 4; Python AI command WRL selection
  passed 1 with 64 deselected; Python Qt bridge WRL selection passed 1 with
  13 deselected. Changed WRL library source contains no forbidden panic-style
  calls.
- Snapshot: `G:\Builder3D-Rust_snapshot_20261029-113945_R2d-wrl-final-verified`
  was created after this checkpoint with `target\` excluded. Source and
  snapshot each contained 177 files and 1,831,736 bytes; missing, extra, and
  SHA-256-mismatched file counts were all zero, and the snapshot had no
  `target\` directory. The earlier additive copy
  `G:\Builder3D-Rust_snapshot_20261029-113816_R2d-wrl-final` was left
  untouched.
- Limitations: WRL import remains unsupported as in the Python product;
  VRML animations, textures, and other non-mesh scene features are outside
  the 3D Modeler R2 scene-item contract. Final cargo-audit availability and
  workspace checks remain to be recorded.
- Remaining R2 work: none. R2a STL/OBJ/PLY, R2b 3MF, R2c GLTF/GLB, and R2d
  WRL are complete.
- Next authoritative action: run the complete workspace and Python reference
  verification required by the owner, then create the final R2 backup and
  record its exact verification before proceeding to R3 mesh operations.

## 2026-10-29 11:41 - R2 complete and final verification

- R2 status: all authoritative section-R file-I/O subtasks are complete:
  R2a STL/OBJ/PLY, R2b 3MF, R2c GLTF/GLB, and R2d WRL. Existing R2a and R2b
  work was preserved and reverified through the workspace and Python suites;
  R2c and R2d added the typed, bounded Rust implementations and golden
  fixtures recorded above.
- Behavioral evidence: STL supports bounded binary/ASCII import and binary
  export; OBJ supports bounded indexed geometry and vertex colors; PLY
  supports bounded ASCII/binary import and binary little-endian export; 3MF
  supports package-safe units, relationships, materials, metadata,
  thumbnails, components, production external parts, and export round trips;
  GLTF/GLB supports bounded JSON/container/resource parsing, embedded and
  approved-directory buffers, accessors, scene transforms, metre-to-
  millimetre conversion, normals, PBR colors, embedded GLTF and GLB export,
  and safe sidecars; WRL supports bounded VRML 2.0 export with transforms,
  colors, transparency, names, indexed geometry, and atomic replacement.
  Python-derived GLTF and WRL fixtures matched the Rust output/semantics;
  the WRL two-box fixture matched byte-for-byte.
- Full Rust verification: `cargo fmt --all -- --check`,
  `cargo check --workspace`, `cargo test --workspace` (43 tests plus all
  doc-test targets), `cargo clippy --all-targets -- -D warnings`, and
  `cargo deny check` all passed. Cargo-deny emitted only the existing
  duplicate `hashbrown`/`syn` and unmatched BSD-3-Clause allowance warnings.
  `cargo audit` is unavailable on this machine: cargo reports no `audit`
  subcommand.
- Python reference verification: the combined GLTF/3MF/WRL/mesh-operations
  suite passed 108 tests with two known trimesh runtime warnings; full
  `tests/test_ai_commands.py` passed 65; full `tests/test_ai_qt_bridge.py`
  passed 14; `tests/test_project_files.py` passed 29; and
  `tests/test_core_workflow.py` passed 9. No files under `L:\Builder3D` were
  modified.
- Final backup: `G:\Builder3D-Rust_snapshot_20261029-114224_R2-final` was
  created after this log entry with `E:\3D-Modeler\target` excluded.
  Robocopy exit code was 1 (files copied with no failures). Verification
  compared relative paths, byte lengths, and SHA-256 values: source and
  backup file counts and byte totals match, with zero missing, extra, or
  mismatched files; the backup has no `target\` directory. Source byte total:
  `1,835,140`.
- Limitations: WRL import is intentionally unsupported because the Python
  product is export-only. GLTF/GLB currently omits animations, skins, morph
  targets, textures/UV sampling, and vendor extensions outside the core
  scene-item contract. Export ordering for GLTF is valid but not guaranteed
  to match trimesh's internal byte ordering. The final verification does not
  include a GUI launch or platform-specific GPU behavior; cargo-audit is the
  unavailable check noted above.
- Remaining project work: R3 mesh operations and later section-R stages,
  including repair/manifold CSG/simplification/subdivision/smoothing/brushes,
  then subsequent stage gates, adapter completion, UI/viewport integration,
  and release hardening.
- Next authoritative plan task: **R3 - mesh operations** from section R of
  `L:\Builder3D\TODO.md`, beginning with the next ordered R3 subtask after
  the R2 file-I/O boundary.

## 2026-10-29 - R3a boolean mesh operations

- Objective: complete the first R3 operation in the authoritative order:
  manifold-backed boolean union, difference, and intersection with bounded
  validation, reports, source provenance, cut-cap regularization, and scene
  history integration.
- Implementation: added the pinned `manifold-csg` 0.4.1 dependency; added
  mesh health inspection, typed operation/resource/cancellation errors,
  boolean limits, preflight accounting, manifold conversion, result-source
  labels, and volume/health reports; added an affine inverse; and integrated
  transformed target/tool scene booleans through one atomic undoable grouped
  command.
- Changed files: `Cargo.toml`, `Cargo.lock`,
  `crates/modeler3d-core/Cargo.toml`, `src/error.rs`, `src/lib.rs`,
  `src/mesh_ops.rs`, `src/transform.rs`, `src/commands.rs`, `src/scene.rs`,
  and `tests/mesh_ops_booleans.rs`.
- Behavioral evidence: box union/difference/intersection volumes are
  1.5/0.5/0.5 cubic millimetres; inputs are unchanged; open operands and
  resource-limit violations return typed errors; provenance labels include
  target/tool; transformed scene execution preserves target identity/state,
  selection, and undo/redo; and flat cut-cap regularization preserves volume.
- Tests: Python `tests/test_mesh_ops.py -k "boolean or Conversion" -q`
  passed 11 tests (72 deselected, one known trimesh invalid-value warning).
  Rust `cargo fmt --all -- --check`, `cargo check -p modeler3d-core`,
  `cargo clippy -p modeler3d-core --all-targets -- -D warnings`, and
  `cargo test -p modeler3d-core` passed. R3 production-file forbidden-marker
  scan returned zero matches; pre-existing test-only panic markers remain in
  `src/lib.rs`.
- Baseline snapshot: `G:\Builder3D-Rust_snapshot_20261029-140736_R3-baseline`
  (178 files, 1,842,915 bytes, matching paths and SHA-256 hashes).
- Initial R3a snapshot copy: `G:\Builder3D-Rust_snapshot_20261029-142339_R3a-booleans`.
  The final additive R3a snapshot is
  `G:\Builder3D-Rust_snapshot_20261029-142448_R3a-booleans-final`, excluding
  `target\`. Its exact source/backup counts, byte totals, relative-path
  comparison, and SHA-256 comparison are recorded in
  `evidence/r3a/verification.md`.
- Remaining R3 work: repair; simplification; subdivision; smoothing; all
  five brushes; density normalization; final verification and backup.
- Next exact action: implement the non-mutating Python-compatible repair
  pipeline, then use it as the safe fallback for invalid boolean operands.

## 2026-10-29 - R3b mesh repair

- Objective: complete the ordered R3 repair task with non-mutating profiles,
  health/candidate reports, bounded topology cleanup, and safe boolean
  operand fallback.
- Implementation: added repair.rs with simple/enhanced/extended profiles;
  tolerance welding; winding normalization; trivial square/triangle hole
  closure; duplicate/degenerated-face removal; tiny-shell filtering; safe
  compaction; convex planar cleanup; per-stage error/count reports; candidate
  safety; and explicit operation limits. mesh_ops.rs now invokes an extended
  repair attempt only for invalid boolean operands and still clones valid
  operands directly.
- Changed files: crates/modeler3d-core/src/repair.rs,
  crates/modeler3d-core/src/mesh_ops.rs, src/lib.rs, and
  tests/mesh_ops_repair.rs.
- Behavioral evidence: reversed winding repairs to a positive-volume valid
  box; a square-hole box becomes closed; duplicate/tiny shells are removed;
  multi-shell best-effort results remain safe but invalid; flat fan
  regularization preserves volume and connectivity; dense open grids remain
  open and duplicate-free; valid boolean operands bypass repair.
- Tests: Python mesh-operation repair/health/conversion selection passed 17
  tests (66 deselected, one known trimesh invalid-value warning), Python
  chunk7 repair selection passed 3 tests (30 deselected), and the focused Rust
  boolean/repair suites passed 10 tests. Formatting, core check, and
  all-target clippy with -D warnings passed. New repair/boolean production
  files have zero forbidden panic-style/debug marker matches.
- R3b snapshot: G:\Builder3D-Rust_snapshot_20261029-143343_R3b-repair;
  source/backup verification is recorded in evidence/r3b/verification.md,
  with target\ excluded.
- Remaining R3 work: simplification; subdivision; smoothing; all five
  brushes; density normalization; final verification and backup.
- Next exact action: implement deterministic bounded mesh simplification with
  a no-op path for meshes already at or below the requested face target.

## 2026-10-29 - R3c mesh simplification

- Objective: complete ordered R3 simplification with deterministic bounded
  topology reduction, no-op behavior, resource checks, reports, and scene
  history integration.
- Implementation: added simplify.rs with optional/default face targets,
  shortest-edge collapse, interior-edge preference, degenerate/duplicate
  filtering, compacting, work/memory/size limits, volume/deviation evidence,
  and typed warnings. Added the simplify request to the command registry and
  dispatch and added Scene::apply_simplify through ReplaceMesh history.
- Changed files: crates/modeler3d-core/src/simplify.rs,
  src/lib.rs, src/commands.rs, src/scene.rs, and
  tests/mesh_ops_simplify.rs.
- Behavioral evidence: a subdivision-3 sphere reaches the requested 100-face
  bound with finite non-empty geometry; small meshes return an unchanged
  result; limits fail before mutation; and scene identity plus undo/redo are
  preserved.
- Tests: Python tests/test_chunk7.py -k Simplify -q passed 2 tests (31
  deselected). Rust simplification tests passed 5 tests. Formatting, core
  check, and all-target clippy with -D warnings passed. New operation files
  have zero forbidden panic-style/debug marker matches.
- R3c snapshot: G:\Builder3D-Rust_snapshot_20261029-143659_R3c-simplify;
  exact source/backup verification is recorded in evidence/r3c/verification.md
  with target\ excluded.
- Remaining R3 work: subdivision; smoothing; all five brushes; density
  normalization; final verification and backup.
- Next exact action: implement global conforming triangle subdivision with
  Python edge-selection behavior, preserved shared-edge topology, volume,
  bounds, and resource reports.

## 2026-10-29 - R3d mesh subdivision

- Objective: complete ordered R3 subdivision with shared midpoint topology,
  approximately doubled face count, resource checks, reports, and scene
  history integration.
- Implementation: added subdivide.rs with deterministic sorted interior-edge
  selection followed by boundary-edge selection, global midpoint indexing,
  all eight face split cases, expected-result limits, work accounting, volume
  and extent reports, and typed unsupported-topology errors. Added the
  subdivide command request/registry/dispatch and Scene::apply_subdivide.
- Changed files: crates/modeler3d-core/src/subdivide.rs,
  src/lib.rs, src/commands.rs, src/scene.rs, and
  tests/mesh_ops_subdivide.rs.
- Behavioral evidence: closed boxes grow from 12 to 24 faces while retaining
  watertightness, extents, and volume; object identity and undo/redo survive;
  resource limits fail before midpoint allocation.
- Tests: Python tests/test_chunk7.py -k Subdivide -q passed 1 test (32
  deselected). Rust subdivision tests passed 4 tests. Formatting, core check,
  and all-target clippy with -D warnings passed. New R3 operation files have
  zero forbidden panic-style/debug marker matches.
- R3d snapshot: G:\Builder3D-Rust_snapshot_20261029-143911_R3d-subdivide;
  exact source/backup verification is recorded in evidence/r3d/verification.md
  with target\ excluded.
- Remaining R3 work: smoothing; all five brushes; density normalization;
  final verification and backup.
- Next exact action: implement bounded iterative mesh smoothing with the
  Python zero-iteration no-op, finite vertex guarantees, and scene/history
  integration.

## 2026-10-29 - R3e mesh smoothing

- Objective: complete ordered R3 smoothing with bounded Laplacian iterations,
  exact zero-iteration no-op behavior, finite/topology guarantees, reports,
  resource checks, and scene history integration.
- Implementation: added smooth.rs with shared-vertex adjacency, conservative
  half-step Laplacian updates, finite checks, displacement/volume reports, and
  iteration/work/mesh limits. Added the smooth command request/registry/
  dispatch and Scene::apply_smooth through ReplaceMesh history.
- Changed files: crates/modeler3d-core/src/smooth.rs, src/lib.rs,
  src/commands.rs, src/scene.rs, and tests/mesh_ops_smooth.rs.
- Behavioral evidence: one iteration changes box positions while preserving
  topology and finite coordinates; zero iterations is an exact no-op; excess
  iterations and work limits return typed errors; object identity and
  undo/redo survive scene application.
- Tests: Python tests/test_chunk7.py -k Smooth -q passed 2 tests (31
  deselected). Rust smoothing tests passed 4 tests. Formatting, core check,
  and all-target clippy with -D warnings passed. New R3 operation files have
  zero forbidden panic-style/debug marker matches.
- R3e snapshot: G:\Builder3D-Rust_snapshot_20261029-144108_R3e-smooth;
  exact source/backup verification is recorded in evidence/r3e/verification.md
  with target\ excluded.
- Remaining R3 work: all five brushes; density normalization; final
  verification and backup.
- Next exact action: implement the five authoritative topology-preserving
  brushes—elastic grab, crease polish, smooth brush, blob, and normalize
  density—with shared validation, displacement bounds, and scene history.

## 2026-10-29 - R3f local mesh brushes

- Objective: complete the ordered four-operation local brush group before
  density normalization: Elastic Grab, Crease Polish, Smooth Brush, and Blob.
- Implementation: added brushes.rs with typed falloff, direction, symmetry,
  parameter, report, and result contracts; finite center/path validation;
  work/memory/result limits; smooth weighted grab; path crease motion;
  adjacency/normal smooth brushing; normal blob inflation/deflation; bounded
  cumulative displacement; and topology-preserving Mesh replacements. Added
  four command requests/registry entries/dispatch paths and Scene methods that
  use existing history.
- Changed files: crates/modeler3d-core/src/brushes.rs, src/lib.rs,
  src/commands.rs, src/scene.rs, and tests/mesh_ops_brushes.rs.
- Behavioral evidence: Elastic Grab symmetry mirrors centers and offsets;
  Crease Polish supports both directions; Smooth Brush retains topology and
  the 16 mm default displacement envelope; Blob changes volume outward and
  inward; bad paths/offsets/work limits fail before mutation; identity and
  undo/redo survive scene application.
- Tests: Python brush selection passed 5 tests (78 deselected), Python scene
  brush-command selection passed 4 tests (65 deselected), and Rust brush
  tests passed 6. Formatting, core check, and all-target clippy with
  -D warnings passed. New R3 operation files have zero forbidden
  panic-style/debug marker matches.
- R3f snapshot: G:\Builder3D-Rust_snapshot_20261029-144529_R3f-brushes;
  exact source/backup verification is recorded in evidence/r3f/verification.md
  with target\ excluded.
- Remaining R3 work: final ordered density normalization, final verification,
  and final backup.
- Next exact action: implement normalize_face_density with bounded local
  selection, conforming split/collapse passes, cancellation/resource checks,
  boundary preservation, and scene/history integration.

## 2026-10-29 - R3g density normalization

- Objective: complete the final ordered R3 operation with Python-compatible
  face/center selection, bounded local density normalization, cancellation,
  progress, resource checks, boundary evidence, and scene/history integration.
- Implementation: added density.rs with typed options/results and bounded
  selection, conforming selected-region edge splits, conservative internal
  short-edge collapses, finite/result validation, work/topology budgets, and
  callback hooks. Added public exports, the normalize_face_density command
  request/registry/dispatch path, and an atomic Scene replacement method.
- Changed files: crates/modeler3d-core/src/density.rs, src/lib.rs,
  src/commands.rs, src/scene.rs, tests/mesh_ops_density.rs, and evidence/r3g.
- Behavioral evidence: selected-region refinement increases faces without
  mutating the source; dense closed-region coarsening collapses internal
  edges; coarse center selection includes a hit; progress reaches 5..100;
  cancellation and work limits return typed errors; scene identity and
  undo/redo are preserved.
- Tests: Python mesh-operation density selection passed 6 tests (77
  deselected), Python scene density command passed 1 (68 deselected), Python
  AI command passed 1 (64 deselected), and Python live Qt bridge passed 1 (13
  deselected). Rust density tests passed 6. All focused R3 Rust suites passed
  35 tests total. Format/check/clippy passed and the R3 production marker
  scan found zero forbidden matches.
- R3g snapshot: `G:\Builder3D-Rust_snapshot_20261029-145400_R3g-density`,
  excluding `target\`; exact count/byte/path/length/hash verification is in
  evidence/r3g/verification.md.
- Remaining work: final full cargo verification, relevant Python reference
  verification, final R3 backup, and final handoff. `cargo audit` availability
  will be recorded explicitly as required by the plan.
- Next exact action: verify the R3g checkpoint snapshot, then run the complete
  final verification sequence and create the final additive R3 backup.

## 2026-10-29 - R3 complete and final verification

- Completed every authoritative R3 operation in order: manifold-csg
  booleans; bounded repair; simplification; subdivision; smoothing; Elastic
  Grab; Crease Polish; Smooth Brush; Blob; and density normalization.
- Final implementation includes typed reports/errors, bounded resource and
  memory/work checks, atomic core scene replacement, object identity/state
  preservation, and undo/redo integration. The operation-specific evidence
  and focused tests are recorded in evidence/r3a through evidence/r3g.
- Final checks: cargo fmt check, workspace check, workspace tests (79 passed),
  workspace clippy with `-D warnings`, and cargo-deny all passed. Cargo-deny
  retained existing duplicate hashbrown/syn and unmatched BSD allowance
  warnings. Cargo audit is unavailable because this Cargo installation has no
  audit subcommand.
- Final Python reference verification: 83 mesh-operation tests, 33 chunk-7
  tests, 69 scene tests, 65 AI-command tests, and 14 live Qt-bridge tests
  passed (264 total). The mesh-operation file emitted two known trimesh
  invalid-value runtime warnings; no failures occurred. Python tests ran with
  bytecode and cache writes disabled. A read-only Python git status still
  shows the pre-existing dirty Python worktree; no Python files were changed.
- Final snapshot: `G:\Builder3D-Rust_snapshot_20261029-145630_R3-final`,
  excluding `target\`; exact source/backup counts, bytes, paths, lengths, and
  SHA-256 results are in evidence/r3-final/verification.md.
- No commit was created. R3 has no remaining implementation work. Remaining
  project work is the later adapter/UI/release stages; the next authoritative
  task is R4 MCP server work.

## 2026-10-29 - R4a portable MCP connection and protocol

- Began the authoritative R4 task after the verified R3 snapshot; the R4
  baseline snapshot is `G:\Builder3D-Rust_snapshot_20261029-150453_R4-baseline`.
- Added the Rust R4 `rmcp` stdio server, trusted launch profiles, typed adapter
  errors, bounded operation records, canonical replay fingerprints, strict
  stdout protocol behavior, approved-root path policy, and reparse-point
  rejection. The Python tree remains unchanged.
- Verified the official Python client, raw MCP discovery, workspace format,
  check, test, and clippy gates. The official client negotiated 2025-11-25;
  the raw server discovery response advertises compatibility through
  2026-07-28. `cargo audit` is unavailable in the installed Cargo toolchain.
- Evidence: `evidence/r4a/report.md`, `evidence/r4a/checkpoint.md`, and
  `evidence/r4a/official_client_probe.py` plus
  `evidence/r4a/raw_discovery_probe.py`.
- R4a snapshot:
  `G:\Builder3D-Rust_snapshot_20261029-155500_R4a-protocol-final`; 122 files,
  2,177,988 bytes, zero missing/extra paths, zero SHA-256 mismatches, and no
  `target\` directory. Verification is in `evidence/r4a/verification.md`.
- Next exact ordered action: complete R4b catalog/profile/schema parity.

## 2026-10-29 - R4b catalog and contract parity

- Completed the exact 60-tool Rust catalog in Python reference order with
  profile minima, metadata/annotations, strict bounded input schemas, strict
  structured-output allow lists, revision/project checks, and idempotency
  keys. Added grouped actions, core clipboard commands, and platform slicer
  discovery coverage.
- Official behavior exercised scene creation, transforms, view state, print
  checks, mesh health, asynchronous smoothing, operation polling, grouped
  actions, clipboard, and slicer discovery. Python `test_mcp_catalog.py` and
  `test_mcp_sdk.py` passed 9 tests.
- Evidence: `evidence/r4b/report.md`, `evidence/r4b/checkpoint.md`, and
  `evidence/r4b/official_behavior_probe.py`.
- R4b snapshot:
  `G:\Builder3D-Rust_snapshot_20261029-155700_R4b-catalog-final`; its exact
  count/byte/path/hash verification is in `evidence/r4b/verification.md`.
- Next exact ordered action: complete R4c behavior, security, replay,
  revision, cancellation, and bounded asynchronous-operation hardening.

## 2026-10-29 - R4c behavior and security hardening

- Completed the ordered behavior/security subtask with real core-backed
  grouped mutations, selection/history, project lifecycle, bounded PPM color
  sampling, revision/project checks, replay conflict handling, bounded async
  operations, approved-root/reparse policy, and clipboard limits.
- Added a platform-owned known-slicer discovery and handoff seam. It searches
  only declared slicers, stages visible objects into a bounded temporary queue,
  uses 3MF or the Bambu STL fallback, and launches only the discovered
  executable with no shell or arbitrary command endpoint. Failed handoffs
  clean their owned staging directory.
- Added focused approved-root read/write/overwrite tests and platform slicer
  discovery tests. `modeler3d-mcp` focused tests: 13 passed; platform tests: 9
  passed. Workspace clippy with `-D warnings` passed.
- `emboss_text` and `hollow` remain explicit typed unsupported results because
  their Rust core operations are outside completed R3; they do not silently
  mutate or execute an unsafe fallback.
- Evidence: `evidence/r4c/report.md` and `evidence/r4c/checkpoint.md`.
- R4c snapshot:
  `G:\Builder3D-Rust_snapshot_20261029-155900_R4c-behavior-final`; exact
  count/byte/path/hash verification is in `evidence/r4c/verification.md`.
- Next exact ordered action: R4d full final verification, evidence, and backup.

## 2026-10-29 - R4 complete and final verification

- Completed the ordered R4 MCP server work: private stdio/protocol behavior,
  the exact 60-tool catalog and profiles, strict schemas and structured
  outputs, approved-root and replay rules, live core-backed commands, bounded
  asynchronous operations, clipboard/image limits, and known-slicer handoff.
- Final Rust gates passed: format, workspace check, workspace tests, clippy,
  cargo-deny, and release bloat analysis. The official Python MCP client and
  raw discovery probes passed; the client negotiated 2025-11-25 while the
  server advertised compatibility through 2026-07-28. The two relevant Python
  MCP reference modules passed 9 tests.
- Cargo audit remains unavailable because this Cargo installation has no
  `audit` subcommand. Cargo-deny retains only the existing unmatched
  BSD-3-Clause allowance and duplicate transitive hashbrown/syn warnings.
- The production forbidden-marker scan is clean; matches are confined to test
  modules. `emboss_text` and `hollow` are explicit typed unsupported results
  until their geometry kernels are ported into the Rust core; no unsafe or
  arbitrary fallback was added.
- Final evidence: `evidence/r4-final/report.md`,
  `evidence/r4-final/checkpoint.md`, and
  `evidence/r4-final/verification.md`.
- Final additive snapshot:
  `G:\Builder3D-Rust_snapshot_20261029-155958_R4-final`, excluding `target\`.
  The exact count/byte/path/hash verification is in the final evidence.
- No commit was created and the Python reference tree was not modified. The
  next authoritative task is R5 viewport work.

## 2026-10-29 - R5 viewport complete

- Started from the verified R4 snapshot
  `G:\Builder3D-Rust_snapshot_20261029-155958_R4-final` and created the
  additive R5 baseline
  `G:\Builder3D-Rust_snapshot_20261029-172908_R5-baseline`. The baseline was
  verified at 239 files, 2,360,737 bytes, with zero missing/extra paths or
  SHA-256 mismatches and no `target\` directory.
- Completed the authoritative R5 order: wgpu renderer; camera; gizmo;
  authoritative CPU picking; floor; cached contact cues; adaptive interaction
  tiers; central resource ledger integration; and CPU/GPU/hybrid workload
  planning. The ordered subtask evidence and continuation checkpoints are in
  `evidence/r5/01-wgpu-renderer` through `evidence/r5/09-workload-planner`.
- The viewport crate now owns renderer-neutral math, retained full and
  interaction geometry, camera rays/projection, renderer-neutral gizmo
  primitives and hits, exact full-resolution triangle picking, adaptive floor
  grid/orientation geometry, bounded settled contact-cue caching, a real
  indexed wgpu renderer with depth and overlay passes, and a bounded CPU
  rasterizer fallback. Core scene data remains authoritative and is not
  replaced by display LOD.
- The platform ledger now reports used/budget/pressure/available bytes and is
  consumed for authoritative scene data, render targets, GPU mesh caches, and
  transient overlays. The planner keeps batch-level CPU fallback and now
  defaults uncalibrated integrated/unified parallel or memory-bound work to
  CPU while retaining GPU render/compute candidates and measured GPU wins.
- R5 release evidence was generated by
  `cargo run --release -p modeler3d-app --bin r5_viewport --
  --output-dir evidence/r5`. The real renderer used the NVIDIA Quadro RTX
  4000 through DX12 (driver 32.0.15.8142), rendered an 81,932-face retained
  scene at 7.106 ms resting / 0.543 ms active in the final run, and emitted a
  512x512 capture with floor and gizmo overlay vertices. The procedural
  30,000,000-face control measured 30.599 ms p50, 32.062 ms p95/max; the
  3,600,000-face interaction control measured 3.492 ms p50, 3.505 ms p95/max.
  The 30,655,824-face adaptive probe target is 3,678,698 faces.
- Rust verification passed: `cargo fmt --all -- --check`,
  `cargo check --workspace`, `cargo test --workspace` (all workspace tests),
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  `cargo deny check`, and `cargo bloat --release --package modeler3d-app
  --bin r5_viewport -n 20`. `cargo audit` remains unavailable because the
  installed Cargo has no audit subcommand.
- Python reference checks ran through `L:\Builder3D\venv` with bytecode and
  pytest cache disabled: the focused R5 selectors passed, and the final
  relevant reference files (`test_renderer_math.py`, `test_gizmo.py`,
  `test_scene.py`, and `test_viewport.py`) passed 193 tests. The existing dirty
  Python worktree was preserved and no L-drive files were changed by R5.
- The production forbidden-marker scan is clean. Existing marker matches are
  confined to test modules. No commit was created.
- Final evidence: `evidence/r5/report.md`, `evidence/r5/checkpoint.md`, and
  `evidence/r5/verification.md`. All ten additive snapshots, including the
  final `G:\Builder3D-Rust_snapshot_20261029-191500_R5-final`, contain 275
  files including the verification artifact; the hidden-inclusive manifest
  and exact path/byte/SHA-256 comparison are recorded in
  `evidence/r5/verification.md`, with zero missing, extra, or path/byte/hash
  mismatches and no `target\`.
- Next exact authoritative task: **R6 - shell and packaging** in section R of
  `L:\Builder3D\TODO.md` (ribbon, commands, dialogs, theme-scoped artwork,
  launchers, and installers).

## 2026-10-29 - R6-01 ribbon complete

- Started R6 from the verified baseline
  `G:\Builder3D-Rust_snapshot_20261029-210220_R6-baseline`, captured before
  implementation. The authoritative R6 order is ribbon → commands → dialogs
  → theme-scoped artwork → launchers → installers.
- Added the eframe 0.37 native shell in `modeler3d-app`, with the reference
  Insert/Object/Edit/Paint/View tabs, file menu, utility Save/Undo/Redo
  controls, pin control, Items panel, status/transform strip, and a
  renderer-neutral preview sourced from `modeler3d-viewport::RenderScene`.
- Added typed `CommandId`, `CommandOutcome`, `CommandError`, and
  `CommandState` boundaries. Primitive insertion, selection, copy/paste,
  duplicate, delete, undo/redo, and grid/wireframe/X-Ray toggles route through
  the portable core rather than shell-local geometry mutations.
- Added the documented light palette and a matching dark palette. The shell
  includes a bounded `--smoke` path for native command startup verification.
- Rust R6-01 checks passed: formatting check, `cargo check -p
  modeler3d-app`, and `cargo test -p modeler3d-app` (7 tests). Python
  reference shell checks passed: `TestMenuSpecs` plus `test_desktop_actions.py`
  (15 tests), with Python bytecode/cache generation disabled.
- Per-subtask evidence is in `evidence/r6/01-ribbon/`. The next exact task is
  R6-02 commands.

## 2026-10-29 - R6-02 commands complete

- Completed the second item in the authoritative R6 order. The application
  command boundary now routes object actions through core history, bounded
  repair/simplify/subdivide/smooth/brush/density services, transformed
  selection measurement, view intent, display toggles, and boolean
  intersection/subtraction. File and parameter-bearing actions emit typed
  dialog requests for the next task.
- Added bounded path validation, core project save/open seams, selected-mesh
  export, repair, material color, and print-preparation summary methods.
- Rust verification passed: `cargo fmt --all -- --check` and
  `cargo test -p modeler3d-app` (9 tests). Python reference command checks
  passed: `test_desktop_actions.py`, `TestExportMesh`, and `TestMenuSpecs` (17
  tests).
- Per-subtask evidence is in `evidence/r6/02-commands/`. The next exact task
  is R6-03 dialogs.

## 2026-10-29 - R6-03 dialogs complete

- Added the typed dialog adapter for bounded project open/save/export paths,
  repair, print-prep, material/color, torus parameter editing, emboss relief,
  and live light/dark settings. Dialogs return actions; `CommandState` owns
  core/project mutations and typed errors.
- Torus edits rebuild validated parametric geometry and metadata through one
  grouped core history command. Emboss is a bounded centered relief operation
  through the existing core blob service; it does not evaluate arbitrary text.
- Rust verification passed: format, 11 application tests, and the native
  `--smoke` path (`objects=2 selected=1 tabs=5`). Python reference project and
  desktop-action selectors passed 40 tests. The separately run setup-contract
  selector retains one pre-existing stale `K:\\builder3D.png` expectation in
  the dirty L-drive reference docs; the reference tree was not modified.
- Per-subtask evidence is in `evidence/r6/03-dialogs/`. The next exact task is
  R6-04 theme-scoped artwork.

## 2026-10-29 - R6-04 theme-scoped artwork complete

- Added the compact light/dark artwork catalog and wired ribbon buttons to
  theme-scoped vector painting. Each theme contains 20 self-contained SVG
  assets, including `ExRay.svg` and `Subtract.svg`; the catalog rejects
  oversized, external, or raster-dependent artwork before startup.
- The 40 Rust artwork files total 4,650 bytes, so the visual contract does not
  consume the Python reference's large raster directories or threaten the
  40 MB installed-size gate.
- Rust verification passed: format, 13 application tests, and the native shell
  smoke path. Python icon/theme/ribbon selectors passed 119 tests with 32
  deselected.
- Per-subtask evidence is in `evidence/r6/04-artwork/`. The next exact task is
  R6-05 launchers.

## 2026-10-29 - R6-05 launchers complete

- Added fixed native Windows batch/PowerShell, POSIX, macOS command, and Linux
  desktop launchers. Each resolves the sibling native executable and forwards
  arguments without Python, arbitrary executable paths, or shell command
  strings.
- Added the typed launcher catalog, validation, and bounded
  `--stage-launchers <directory>` command. The smoke stage produced all five
  launchers under the excluded `target\\r6-launcher-smoke` directory.
- Rust verification passed: format, 15 application tests, and launcher
  staging. Python slicer/setup reference checks passed 15 tests; one existing
  stale `K:\\builder3D.png` assertion remains in the dirty setup-contract
  reference docs, which were not changed.
- Per-subtask evidence is in `evidence/r6/05-launchers/`. The next exact task
  is R6-06 installers.

## 2026-10-29 - R6-06 installers complete

- Added native Windows, Linux, and macOS installation/removal scripts with
  private-root guards and no Python runtime dependency. Windows scripts parse
  cleanly; POSIX syntax validation is pending a host shell because this
  Windows host has no `sh` command.
- Added bounded Rust package staging, fixed installer catalog, SHA-256 package
  manifest, deterministic ZIP member order, and explicit 40 MiB installed / 15
  MiB compressed gates. The size-profile package measured 9,425,015 installed
  bytes (8.988 MiB) and 4,190,185 archive bytes (3.996 MiB); all 12 manifest
  member hashes and 13 ZIP entries verified.
- Rust R6-06 checks passed: format, 17 application tests, clippy, package
  staging, PowerShell parsing, and static no-Python installer/launcher policy
  scan. Python slicer/icon checks passed 113 tests; the known separate setup
  contract selector retains one stale `K:\\builder3D.png` expectation in the
  dirty reference docs, which were not modified.
- Per-subtask evidence is in `evidence/r6/06-installers/`. R6 implementation
  is complete; run the full final verification plan and create the final R6
  snapshot next. Section R contains no R7 item, so after final verification the
  next authoritative action is owner visual acceptance/MCP parity promotion.

## 2026-10-29 - R6 complete and final verification

- Completed all six R6 tasks in order: ribbon, commands, dialogs,
  theme-scoped artwork, launchers, and installers. The Python reference tree
  was not modified.
- Final Rust gates passed: formatting, locked workspace check, locked
  workspace tests (109 tests plus doctest runs), workspace clippy with
  `-D warnings`, cargo-deny advisories/bans/licenses/sources, locked metadata,
  and release bloat analysis. The production marker scan found zero forbidden
  panic/debug paths. `cargo audit` is unavailable in the installed toolchain.
- Native shell smoke passed with `objects=2 selected=1 tabs=5`. The Python
  selectors produced 183 passes and one pre-existing stale
  `K:\\builder3D.png` setup-contract failure in the dirty L-drive docs.
- Final size-profile package: 13 files, 9,425,015 installed bytes (8.988
  MiB), 4,190,185 archive bytes (3.996 MiB), 12 manifest entries, zero
  manifest hash mismatches, and archive SHA-256
  `9f441174880d8041f8ee03143d9474e51b7ee9cb936d0f1d70479a5e5f3de8ce`.
  Both package size gates pass.
- Final additive snapshot:
  `G:\Builder3D-Rust_snapshot_20261029-214752_R6-final`. It excludes
  `target\`; its complete verification is in `evidence/r6/verification.md`.
- Section R contains no R7 item. Next authoritative action is owner visual
  acceptance and MCP-parity promotion, or a new owner-approved plan.

## 2026-10-30 - R6 continuation: shell operation parity

- Continued from the verified R6 handoff after owner authorization to finish
  the concrete parity gaps rather than stopping at the shell/package gate.
  No R7 item was invented and no Python reference or existing G-drive
  snapshot was modified.
- Replaced the Rust MCP `emboss_text` and `hollow` unsupported branches with
  bounded core-backed implementations. Emboss supports the reference text,
  font, depth, raised/engraved, and offset contract; hollow has exact convex
  and bounded concave paths. Both operations have cancellation hooks,
  structured reports, atomic Scene history, native dialogs, and MCP long
  operation dispatch with revision-conflict protection.
- Added the additive recovery baseline
  `G:\Builder3D-Rust_snapshot_20261030-075615_R6-parity-baseline` before these
  changes. Its source comparison is 356 files and 5,550,395 bytes with zero
  missing, extra, byte, or SHA-256 mismatches and no `target\\` content.
- Final Rust verification passed: format, locked workspace check, locked
  workspace tests (133 tests plus doctest runs), clippy with `-D warnings`,
  cargo-deny, locked metadata, native `--smoke`, and release bloat analysis.
  Cargo-deny uses the documented narrow `ttf-parser` maintenance advisory
  exception for the pinned named-font rasterizer; duplicate-version messages
  remain warnings. Cargo audit remains unavailable.
- Final size-profile package measured 9,621,111 installed bytes and 4,282,705
  ZIP bytes, with 12 manifest members and 13 verified ZIP entries. Both R6
  package gates pass. Archive SHA-256 is
  `a9754d7c8af8c0de0d97609c6aa50cc50d52c6d3358bf65cb88c412d84ac3c3f`.
- Final additive snapshot:
  `G:\Builder3D-Rust_snapshot_20261030-083051_R6-shell-parity-final`.
  Complete path/byte/SHA-256 verification is in
  `evidence/r6/continuation-shell-parity/verification.md`.
- Remaining owner-facing boundary: human visual acceptance of the light and
  dark native shell. The bounded PPM image-color limitation, system-font path
  requirement, unavailable cargo-audit subcommand, and unavailable POSIX
  shell execution on this Windows host remain documented.

## 2026-10-30 - R6 owner verification and MCP parity closeout

- Owner visually inspected and accepted the native shell in both light and
  dark themes. This closes the R6 visual stage-gate.
- Rebuilt `r4_mcp.exe` and ran the existing official Python MCP client probe:
  protocol `2025-11-25`, 60 tools, ready state, and completed asynchronous
  behavior.
- Ran a fresh official-client stdio probe for the new shell operations:
  Emboss completed with 72 occupied cells; Hollow completed through
  `convex-exact` with result volume `9128.000000000015` mm³; MCP undo returned
  `changed=true`. The first probe attempt had only a client-side report-key
  assumption error; the corrected probe passed without a server error.
- R6 is now verified for implementation, automated gates, native visual
  acceptance, and MCP parity. No commit was created. Remaining documented
  boundaries are the bounded PPM image-color subset, installed-font path
  dependency, unavailable cargo-audit subcommand, and lack of a POSIX shell
  for installer execution on this Windows host.

## 2026-10-30 - conversion-first plan and component catalog

- Owner directed that the partial R6 shell is not the next acceptance target:
  complete the full Python-to-Rust conversion first, then perform the broad
  usability, parity, and visual-repair pass.
- Updated `L:\Builder3D\docs\rust-rewrite.md` with that conversion-first
  continuation policy. The Python source tree remains unchanged; this was an
  explicitly requested documentation update.
- Added `docs\component-catalog.toml`, a machine-readable index with stable
  component IDs, Rust paths, symbol anchors, Python references, focused tests,
  contract names, lifecycle status, and follow-up patch IDs. It parses as TOML,
  has 11 unique IDs, and all referenced Rust/test paths exist.
- The catalog is now the navigation contract for targeted patches. Future
  conversion work must add or update a component entry instead of requiring a
  repository-wide read to locate one responsibility.

## 2026-10-30 - zero-Python and trim-code end state reaffirmed

- Owner reaffirmed the final product requirement: the shipped application must
  contain zero Python code and must not invoke a Python runtime. The Python
  checkout remains only a temporary, read-only behavioral oracle during the
  conversion.
- The rewrite document and component catalog now make that end state explicit.
  They also require cohesive modules, narrow typed seams, focused tests, stable
  component IDs, and catalog-based patch navigation so one behavior can be
  changed without reading the entire workspace.

## 2026-10-30 - R7-01 desktop model import command coverage

- Continued the owner-approved conversion-first plan from the catalog's next
  follow-up `modeler3d.app.commands.coverage`. The shell remains in deferred
  `polish`; no visual work was started.
- Added typed `ImportModel` command/dialog routing and `ImportUnit` handling for
  STL, OBJ, PLY, glTF/GLB, and 3MF. The application reuses the existing core
  loaders, centers imported world bounds with the reference placement contract,
  and commits all objects/transforms/selection as one grouped history command.
- Added focused Rust tests for Add/Open identity, dialog defaults, mm/cm
  import placement, atomic undo, and typed unsupported WRL rejection.
- Updated `docs/component-catalog.toml` with the validated
  `modeler3d.app.commands.coverage` entry and R7 evidence links. Evidence is in
  `evidence/r7/01-import/`.
- Verification passed: format check, locked workspace check, 136 locked
  workspace tests plus doctests, workspace Clippy with `-D warnings`,
  cargo-deny, locked metadata, catalog TOML/path validation, and native shell
  smoke. Cargo-deny retains the existing duplicate-version warnings.
- Recovery baseline before implementation:
  `G:\Builder3D-Rust_snapshot_20261030-085726_R7-command-coverage-baseline`;
  263 files and 1,620,912 bytes, with source/snapshot counts and bytes equal.
- Final additive snapshot created after this checkpoint:
  `G:\Builder3D-Rust_snapshot_20261030-090255_R7-command-coverage-final`;
  263 files and 1,632,683 bytes with source/snapshot counts and bytes equal,
  excluding `target`, `evidence`, and `WORK_LOG.md`.
  The next exact conversion slice is
  `modeler3d.app.commands.selection`; background import/preview and remaining
  workflow coverage remain explicitly unconverted.

## 2026-10-30 - R7-02 desktop selection and clipboard command coverage

- Continued the owner-approved conversion-first plan from the catalog's next
  follow-up `modeler3d.app.commands.selection`. Shell visual polish remains
  deferred.
- Added typed select-all, deselect-all, invert-selection, group, ungroup, and
  cut commands. Selection-only actions do not create scene-history entries;
  group/ungroup and cut/paste use the existing core scene graph and clipboard
  contracts.
- Added functional object-panel controls and a focused test covering selection
  history isolation, group/ungroup, cut, and paste.
- Updated `docs/component-catalog.toml` with the validated
  `modeler3d.app.commands.selection` entry and R7 evidence links. Evidence is
  in `evidence/r7/02-selection/`.
- Verification passed: format check, locked workspace check, 137 locked
  workspace tests plus doctests, workspace Clippy with `-D warnings`,
  cargo-deny, locked metadata, catalog TOML/path validation, and native shell
  smoke. Cargo-deny retains the existing duplicate-version warnings.
- Recovery baseline before implementation:
  `G:\Builder3D-Rust_snapshot_20261030-090808_R7-02-selection-baseline`;
  263 files and 1,632,683 bytes, with source/snapshot counts and bytes equal.
- Final additive snapshot created after this checkpoint:
  `G:\Builder3D-Rust_snapshot_20261030-091210_R7-02-selection-final`;
  263 files and 1,639,456 bytes with source/snapshot counts and bytes equal,
  excluding `target`, `evidence`, and `WORK_LOG.md`. No Python source, R1-R6
  evidence, or existing snapshot was modified. No commit was created.

## 2026-10-30 - R7-03 atomic object-transform workflows

- Continued the owner-approved conversion-first plan with
  `modeler3d.app.commands.transforms`. Shell visual polish remains deferred.
- Added typed world-space transform services for atomic group translation,
  rotation, scaling, reflection, alignment, arrangement, settling, numeric
  position, and numeric world-size updates. The selected objects' transformed
  bounds drive the workflow, and all changed transforms commit through one
  core `GroupedCommand`.
- Corrected desktop `Settle` to use the Python build-bed Y axis and replaced
  the prior per-object `Mirror`, `Align`, and `Arrange` mutations with the
  shared atomic seam. Single-selection arrange remains an explicit no-op.
- Updated `docs/component-catalog.toml` with the validated
  `modeler3d.app.commands.transforms` entry and R7 evidence links. Evidence is
  in `evidence/r7/03-transforms/`.
- Verification passed: format check, locked workspace check, 141 locked
  workspace tests plus doctests, workspace Clippy with `-D warnings`,
  cargo-deny, locked metadata, catalog TOML/path validation, and native shell
  smoke. Cargo-deny retains the existing duplicate-version warnings.
- Recovery baseline before implementation:
  `G:\Builder3D-Rust_snapshot_20261030-091425_R7-03-transforms-baseline`;
  263 files and 1,639,456 bytes, with source/snapshot counts and bytes equal.
- Final additive snapshot created after this checkpoint:
  `G:\Builder3D-Rust_snapshot_20261030-091745_R7-03-transforms-final`;
  264 files and 1,658,729 bytes with source/snapshot counts and bytes equal,
  excluding `target`, `evidence`, and `WORK_LOG.md`. No Python source, R1-R6
  evidence, or existing snapshot was modified. No commit was created.

## 2026-10-30 - R7-05 application export workflow coverage

- Continued the owner-approved conversion-first plan with
  `modeler3d.app.export`; shell visual polish remains deferred.
- Added selected-visible export with visible-object fallback, hidden-object
  filtering, multi-object STL/OBJ/PLY concatenation, and scene-item routing
  for glTF/GLB, 3MF, and WRL. World transforms, names, colors, and mesh
  identity now reach the existing core writers through one typed application
  seam. `CommandState::export_to_path` uses that seam instead of exporting
  only the first selected mesh.
- Added focused export selection, multi-object reopen, and extension/path
  error tests. Catalog and evidence are in `docs/component-catalog.toml` and
  `evidence/r7/05-export/`.
- Verification passed: format check, locked workspace check, 147 locked
  workspace tests plus doctests, workspace Clippy with `-D warnings`,
  cargo-deny, locked metadata, catalog TOML/path validation, and native shell
  smoke. Cargo-deny retains the existing duplicate-version diagnostics.
- Recovery baseline:
  `G:\Builder3D-Rust_snapshot_20261030-092520_R7-05-export-baseline`;
  265 files and 1,687,078 bytes according to the checkpoint capture.
- Final additive snapshot:
  `G:\Builder3D-Rust_snapshot_20261030-092745_R7-05-export-final`;
  158 files and 1,531,806 bytes after excluding `target`, `evidence`, and
  `WORK_LOG.md`; source and snapshot counts/bytes match.
- No Python source, R1-R6 evidence, or existing snapshot was modified. No
  commit was created. The next exact conversion slice is
  `modeler3d.app.print-prep`.

## 2026-10-30 - R7-06 application print-preparation coverage

- Continued the owner-approved conversion-first plan with
  `modeler3d.app.print-prep`; shell visual polish remains deferred.
- Added a typed, renderer-neutral `PrintPrepReport` matching the Python
  checks: transformed X/Z bed fit, height and below-bed warnings, small/large
  scale warnings, core mesh-health/disconnected-shell warnings, and
  hidden/empty checks. The desktop print-prep dialog now runs the full report
  instead of returning a placeholder face count.
- Added focused tests for clean settled geometry, world-bound and surface
  findings, hidden severity, and invalid bed-size rejection. Catalog and
  evidence are in `docs/component-catalog.toml` and
  `evidence/r7/06-print-prep/`.
- Verification passed: format check, locked workspace check, 151 locked
  workspace tests plus doctests, workspace Clippy with `-D warnings`,
  cargo-deny, locked metadata, catalog TOML/path validation, and native shell
  smoke. Cargo-deny retains the existing duplicate-version diagnostics.
- Recovery baseline:
  `G:\Builder3D-Rust_snapshot_20261030-093015_R7-06-print-prep-baseline`;
  158 files and 1,531,806 bytes with source/snapshot counts and bytes equal.
- Final additive snapshot:
  `G:\Builder3D-Rust_snapshot_20261030-093212_R7-06-print-prep-final`;
  159 files and 1,542,915 bytes after excluding `target`, `evidence`, and
  `WORK_LOG.md`; source and snapshot counts/bytes match.
- No Python source, R1-R6 evidence, or existing snapshot was modified. No
  commit was created. The next exact conversion slice is
  `modeler3d.app.operations`.

## 2026-10-30 - R7-07 application operation lifecycle coverage

- Continued the owner-approved conversion-first plan with
  `modeler3d.app.operations`; shell visual polish remains deferred.
- Added a thread-safe, single-active desktop `OperationRegistry` with typed
  queued/running/committing/cancelling/completed/failed/cancelled phases,
  bounded progress/messages, structured results/errors, cooperative
  cancellation, terminal idempotence, and bounded history. Connected it to
  `CommandState` as the application-owned background-work seam.
- Added focused lifecycle, busy/cancellation, failure, and retention tests.
  Catalog and evidence are in `docs/component-catalog.toml` and
  `evidence/r7/07-operations/`.
- Verification passed: format check, locked workspace check, 154 locked
  workspace tests plus doctests, workspace Clippy with `-D warnings`,
  cargo-deny, locked metadata, catalog TOML/path validation, and native shell
  smoke. Cargo-deny retains the existing duplicate-version diagnostics.
- Recovery baseline:
  `G:\Builder3D-Rust_snapshot_20261030-093430_R7-07-operations-baseline`;
  159 files and 1,542,915 bytes with source/snapshot counts and bytes equal.
- Final additive snapshot:
  `G:\Builder3D-Rust_snapshot_20261030-093604_R7-07-operations-final`;
  160 files and 1,563,910 bytes after excluding `target`, `evidence`, and
  `WORK_LOG.md`; source and snapshot counts/bytes match.
- No Python source, R1-R6 evidence, or existing snapshot was modified. No
  commit was created. The next exact conversion slice is
  `modeler3d.app.view-state`.

## 2026-10-30 - R7-08 application view-state coverage

- Continued the owner-approved conversion-first plan with
  `modeler3d.app.view-state`; shell visual polish remains deferred.
- Added a typed application controller over the Rust camera with complete
  validated pose state, reset/top/front/side presets, frame-all and
  frame-selected behavior with the Python empty-selection fallback, and
  display toggles. Connected existing view commands to real camera state and
  added `Camera::set_state` as the narrow viewport seam.
- Added focused camera/view tests. Catalog and evidence are in
  `docs/component-catalog.toml` and `evidence/r7/08-view-state/`.
- Verification passed: format check, locked workspace check, 158 locked
  workspace tests plus doctests, workspace Clippy with `-D warnings`,
  cargo-deny, locked metadata, catalog TOML/path validation, and native shell
  smoke. Cargo-deny retains the existing duplicate-version diagnostics.
- Recovery baseline:
  `G:\Builder3D-Rust_snapshot_20261030-093850_R7-08-view-state-baseline`;
  160 files and 1,563,910 bytes with source/snapshot counts and bytes equal.
- Final additive snapshot:
  `G:\Builder3D-Rust_snapshot_20261030-093913_R7-08-view-state-final`;
  161 files and 1,576,322 bytes after excluding `target`, `evidence`, and
  `WORK_LOG.md`; source and snapshot counts/bytes match.
- No Python source, R1-R6 evidence, or existing snapshot was modified. No
  commit was created. The next exact conversion slice is
  `modeler3d.app.preferences`.

## 2026-10-30 - R7-09 application preferences and recent-file coverage

- Continued the owner-approved conversion-first plan with
  `modeler3d.app.preferences`; shell visual polish remains deferred.
- Added pure preferences for bounded newest-first recent files, default
  ambiguous-import units, authoritative 3MF/glTF/GLB unit handling,
  mm/inch display units, ribbon pinning/favorites/order, and command-state
  integration for open/import/save recent paths and default imports.
- Added focused preference and recent-file tests. Catalog and evidence are in
  `docs/component-catalog.toml` and `evidence/r7/09-preferences/`.
- Verification passed: format check, locked workspace check, 161 locked
  workspace tests plus doctests, workspace Clippy with `-D warnings`,
  cargo-deny, locked metadata, catalog TOML/path validation, and native shell
  smoke. Cargo-deny retains the existing duplicate-version diagnostics.
- Recovery baseline reused the pre-change R7-08 final snapshot:
  `G:\Builder3D-Rust_snapshot_20261030-093913_R7-08-view-state-final`;
  161 files and 1,576,322 bytes with source/snapshot counts and bytes equal.
- Final additive snapshot:
  `G:\Builder3D-Rust_snapshot_20261030-094203_R7-09-preferences-final`;
  162 files and 1,587,789 bytes after excluding `target`, `evidence`, and
  `WORK_LOG.md`; source and snapshot counts/bytes match.
- No Python source, R1-R6 evidence, or existing snapshot was modified. No
  commit was created. The next exact conversion slice is
  `modeler3d.app.image-color`.

## 2026-10-30 - R7-10 application image-color coverage

- Continued the owner-approved conversion-first plan with
  `modeler3d.app.image-color`; shell visual polish remains deferred.
- Added bounded average RGB sampling with opaque RGBA output, regular sample
  steps, encoded-size/pixel limits, typed decode/read errors, and common image
  format support through the locked `image 0.25.10` dependency (PNG, JPEG,
  BMP, GIF, WEBP, TIFF, and PNM/PPM). Connected sampling to the selected
  object color/history command seam.
- Added focused image and command tests. Catalog and evidence are in
  `docs/component-catalog.toml` and `evidence/r7/10-image-color/`.
- Verification passed: format check, locked workspace check, 164 locked
  workspace tests plus doctests, workspace Clippy with `-D warnings`,
  cargo-deny, locked metadata, catalog TOML/path validation, and native shell
  smoke. Cargo-deny retains the existing duplicate-version diagnostics.
- Recovery baseline:
  `G:\Builder3D-Rust_snapshot_20261030-094203_R7-09-preferences-final`;
  162 files and 1,587,789 bytes with source/snapshot counts and bytes equal.
- Final additive snapshot:
  `G:\Builder3D-Rust_snapshot_20261030-094603_R7-10-image-color-final`;
  163 files and 1,594,668 bytes after excluding `target`, `evidence`, and
  `WORK_LOG.md`; source and snapshot counts/bytes match.
- No Python source, R1-R6 evidence, or existing snapshot was modified. No
  commit was created. The next exact conversion slice is
  `modeler3d.platform.slicers`.

## 2026-10-30 - R7-11 print handoff coverage

- Continued the owner-approved conversion-first plan with
  `modeler3d.platform.slicers` and `modeler3d.app.print-handoff`; shell visual
  polish remains deferred.
- Added explicit-object export routing and a typed application staging service
  for visible scene objects. Normal slicers receive one 3MF; Bambu Studio uses
  deterministic individual STL files. Launch remains constrained to a
  discovered/revalidated slicer executable.
- Added focused staging tests and catalog/evidence references in
  `docs/component-catalog.toml` and `evidence/r7/11-print-handoff/`.
- Verification passed: format check, locked workspace check, 164 locked
  workspace tests plus doctests, workspace Clippy with `-D warnings`,
  cargo-deny, locked metadata, catalog TOML/path validation, and native shell
  smoke. Cargo-deny retains the existing duplicate-version diagnostics.
- Recovery baseline:
  `G:\Builder3D-Rust_snapshot_20261030-095000_R7-11-print-handoff-baseline`;
  163 files and 1,594,668 bytes with source/snapshot counts and bytes equal.
- Final additive snapshot:
  `G:\Builder3D-Rust_snapshot_20261030-100100_R7-11-print-handoff-final`;
  164 files and 1,605,212 bytes after excluding `target`, `evidence`, and
  `WORK_LOG.md`; source/snapshot counts and bytes match.
- No Python source, R1-R6 evidence, or existing snapshot was modified. No
  commit was created. The next exact conversion slice is
  `modeler3d.platform.runtime`.

## 2026-10-30 - R7-12 platform runtime coverage

- Continued the owner-approved conversion-first plan with
  `modeler3d.platform.runtime`; shell visual polish remains deferred.
- Added injected Windows/macOS/XDG runtime path selection, MCP descriptor
  override resolution, read-only process liveness through `sysinfo`, and the
  typed offscreen GUI guard. Added focused platform tests and catalog/evidence
  references in `docs/component-catalog.toml` and `evidence/r7/12-runtime/`.
- Verification passed: format check, locked workspace check, 169 locked
  workspace tests plus doctests, workspace Clippy with `-D warnings`,
  cargo-deny, locked metadata, catalog TOML/path validation, and native shell
  smoke. Cargo-deny retains the existing duplicate-version diagnostics.
- Recovery baseline:
  `G:\Builder3D-Rust_snapshot_20261030-100300_R7-12-runtime-baseline`;
  164 files and 1,605,212 bytes with source/snapshot counts and bytes equal.
- Final additive snapshot:
  `G:\Builder3D-Rust_snapshot_20261030-100600_R7-12-runtime-final`;
  165 files and 1,616,429 bytes after excluding `target`, `evidence`, and
  `WORK_LOG.md`; source/snapshot counts and bytes match.
- No Python source, R1-R6 evidence, or existing snapshot was modified. No
  commit was created. The next exact conversion slice is
  `modeler3d.app.desktop-adapters`.

## 2026-10-30 - R7-13 desktop adapters and R7 final coverage audit

- Continued the owner-approved conversion-first plan with
  `modeler3d.app.signals`, `modeler3d.app.desktop-adapters`,
  `modeler3d.app.entrypoint`, and `modeler3d.mcp.desktop-bridge`.
- Added typed callback signals, numeric suffix selection, JSON preference
  persistence, and explicit catalog mapping for the native eframe/R4
  replacements of the remaining Qt-bound desktop and AI seams. Visual fidelity
  remains deferred to the post-coverage polish pass.
- The final catalog audit maps all 41 Python product modules and all 19
  reference/support scripts. It contains 29 unique IDs, only `validated` and
  `ported` statuses, and 197 existing referenced paths.
- Final verification passed: format check, locked workspace check, 173 locked
  workspace tests plus doctests, workspace Clippy with `-D warnings`,
  cargo-deny, locked metadata, catalog/coverage audit, and native shell smoke.
  `cargo audit` was skipped because it is not installed. Cargo-deny retains the
  existing duplicate-version diagnostics.
- Recovery baseline:
  `G:\Builder3D-Rust_snapshot_20261030-100800_R7-13-desktop-adapters-baseline`;
  165 files and 1,616,429 bytes with source/snapshot counts and bytes equal.
- Final recovery snapshot:
  `G:\Builder3D-Rust_snapshot_20261030-101500_R7-final`;
  167 files and 1,629,478 bytes after excluding `target`, `evidence`, and
  `WORK_LOG.md`; source/snapshot counts and bytes match.
- No Python source, R1-R6 evidence, or existing snapshot was modified. No
  commit was created. R7 conversion coverage is complete; stop here before
  visual polish.

## 2026-10-30 - R8 execution brief

- Added the comprehensive post-R7 R8 execution brief at
  `docs/r8-execution-brief.md`. It consolidates the owner-approved
  conversion-first context, current R7 baseline, native UI-parity workstreams,
  slice protocol, catalog/evidence rules, recovery requirements, verification
  matrix, completion criteria, and the short fresh-session prompt.
- Recovery baseline for this documentation subtask:
  `G:\Builder3D-Rust_snapshot_20261030-101500_R7-final`.
- Final additive snapshot:
  `G:\Builder3D-Rust_snapshot_20261030-102000_R8-brief-final`;
  168 files and 1,644,529 bytes after excluding `target`, `evidence`, and
  `WORK_LOG.md`; source/snapshot counts and bytes match.
- No Rust source, Python source, existing snapshot, or commit was changed.

## 2026-10-30 - R8-00 baseline

- Read the workspace, Rust, Python-project, rewrite-plan, catalog, R7 evidence,
  and R8 execution instructions before changing the rewrite.
- Launched the current native eframe shell for manual inspection. The captured
  baseline shows a thin five-tab ribbon, empty grid viewport, sparse Items
  panel, and no coherent welcome/recent or file/settings workflow. The native
  window was then closed before implementation.
- Baseline verification passed: `cargo test -p modeler3d-app --locked` (51
  tests plus doctests) and native smoke (`objects=2 selected=1 tabs=5`).
- Baseline recovery snapshot: `G:\Builder3D-Rust_snapshot_20261030-114800_R8-baseline`;
  robocopy exit 1, 168 files and 1,644,529 bytes, with source/snapshot counts
  and byte totals equal. Evidence is in `evidence/r8/00-baseline/`.
- No Python source, existing snapshot, or commit was changed. The next exact
  slice is the native shell foundation and persisted presentation state.

## 2026-10-30 - R7-04 renderer-neutral snapping

- Continued the owner-approved conversion-first plan with
  `modeler3d.viewport.snapping`. Shell visual polish remains deferred.
- Added immutable drag-start snap snapshots, object/origin/floor landmarks,
  fixed/floor-fixed/view-scaled tolerance policies, strict nearest correction,
  and capture/release hysteresis in the Rust viewport crate.
- Connected `CommandState::snap_selected_translation` to the shared solver;
  corrected movement is committed through the atomic transform service.
- Updated `docs/component-catalog.toml` with the validated
  `modeler3d.viewport.snapping` entry and R7 evidence links. Evidence is in
  `evidence/r7/04-snapping/`.
- Verification passed: format check, locked workspace check, 144 locked
  workspace tests plus doctests, workspace Clippy with `-D warnings`,
  cargo-deny, locked metadata, catalog TOML/path validation, and native shell
  smoke. Cargo-deny retains the existing duplicate-version warnings.
- Recovery baseline before implementation:
  `G:\Builder3D-Rust_snapshot_20261030-091930_R7-04-snapping-baseline`;
  264 files and 1,658,729 bytes, with source/snapshot counts and bytes equal.
- Final additive snapshot created after this checkpoint:
  `G:\Builder3D-Rust_snapshot_20261030-092326_R7-04-snapping-final`;
  265 files and 1,687,078 bytes with source/snapshot counts and bytes equal,
  excluding `target`, `evidence`, and `WORK_LOG.md`. No Python source, R1-R6
  evidence, or existing snapshot was modified. No commit was created.
- No Python source, R1–R6 evidence, or existing snapshot was modified. No
  commit was created.
## 2026-10-30 - R8-01 native shell and workflow foundation

- Implemented the first R8 native eframe slice after the protected R8 baseline:
  welcome/recent routing, ribbon favorites and per-tab ordering, pinning,
  overflow/file-menu presentation, theme surfaces, dirty/status state,
  object-panel actions, selection modifiers, grouped transforms, explicit
  numeric units, native settings/material/print-prep routes, build-bed/grid
  cues, keyboard shortcuts, and active-operation status presentation.
- Updated the component catalog with the native shell, UI-state, command, and
  desktop-adapter R8 contracts and evidence links.
- Launched the native executable for visual inspection. The welcome/status
  capture and prior scene/build-bed/object-panel capture are in
  `evidence/r8/01-shell-foundation/`. The Windows lock-screen foreground
  prevented reliable synthetic pointer interaction during the final capture;
  focused Rust tests cover the interaction seams.
- Verification passed: `cargo fmt --all -- --check`, 61 locked app tests plus
  doctests, app Clippy with `-D warnings`, and native launch. Workspace gates
  are reserved for the final R8 audit.
- No Python source, existing snapshot, or commit was changed.

## 2026-10-30 - R8-02 presentation parity polish

- Compared the native shell against the ribbon/camera presentation contracts
  in `L:\Builder3D\src\main_window.py` and tightened the remaining small R8
  presentation gaps.
- Added disabled-state handling to object-panel actions, a visible native
  camera-mode cue, a compact status transform summary, and a real unsupported
  recent-file test case.
- Focused verification passed: format, locked app check/tests (61 tests plus
  doctests), app Clippy with `-D warnings`, and a final native launch capture.
- Evidence is in `evidence/r8/02-parity-polish/`. The next step is the complete
  workspace acceptance matrix, catalog audit, final evidence, and verified
  additive recovery snapshot.
- No Python source, existing snapshot, or commit was changed.

## 2026-10-30 - R8 final acceptance

- Completed the native eframe shell and major R8 workflows over the existing
  Rust core/application command, viewport, platform, operation, color, and
  print seams. The final catalog records 30 unique IDs with only `ported` and
  `validated` statuses; the coverage audit maps all 41 product Python modules
  and 19 support scripts and found no missing referenced paths.
- Required verification passed: format, locked workspace check, locked
  workspace tests/doctests, all-target/all-feature Clippy with `-D warnings`,
  cargo-deny, locked metadata, and native smoke (`objects=2 selected=1
  tabs=5`). Cargo-deny reports the existing duplicate-version warnings while
  advisories, bans, licenses, and sources pass.
- Optional checks: `cargo audit` was unavailable because `cargo-audit` is not
  installed. `cargo bloat --release --workspace` completed with its existing
  unused-workspace-argument warning; the package-specific app report also
  completed (`.text` 10.7 MiB, file 13.2 MiB).
- Final verified additive recovery snapshot:
  `G:\Builder3D-Rust_snapshot_20261030-123828_R8-final`; robocopy exit 1,
  169 files and 1,730,290 bytes in both source and snapshot, excluding
  `target`, `evidence`, and `WORK_LOG.md`.
- Native manual inspection evidence is in `evidence/r8/`; the remaining
  work is post-R8 owner visual acceptance at additional display scales.
- No Python source, existing G: snapshot, or commit was changed/created.

## 2026-10-30 - Product rebuild plan reset

- Owner rejected the R8 result and requested a comprehensive replacement plan
  that reproduces the original aesthetic and functionality, reusing existing
  Rust work wherever that is the fastest dependable route.
- New execution authority: `docs/product-rebuild-plan.md`, RB-00 through RB-11.
  Retired `docs/r8-execution-brief.md`; superseded historical final acceptance/
  checkpoint claims and corrected the scope of the old verification record.
  Updated `AGENTS.md` routing and the catalog: shell/adapters `polish`, commands
  `ported`; individual backend contract evidence is retained.
- Corrected the previous blanket frontend-replacement recommendation. Default
  to reuse/repair of the current Rust workspace and eframe, with an early real
  renderer/original-design working slice before committing to broad UI work.
- Plan requires original owner assets and historical design decisions, a
  feature-level ledger, inspected matched captures, explicit early/final owner
  review, all major workflows, shared GUI/MCP state, real input/rendering,
  cancellation/history/failure checks, and measured resource/package/host gates.
- Evidence: `evidence/rebuild/00-plan-reset/`. Reviewed the original style guide,
  icon manifest, application source, dated notes/testing/contracts and old R8
  records. Inspected archived captures; the old Python baseline is blank and
  cannot certify visual parity. No new visual acceptance is claimed.
- Verification passed: catalog TOML/30 unique IDs/status/reference audit,
  document/link/fence/whitespace audit, unchanged-source comparison (250 source/
  package files plus five build/configuration files), format check and locked
  semantic app smoke (`objects=2 selected=1 tabs=5`). Full implementation gates
  and live interactive checks were not rerun for documentation-only changes.
- Verified pre-edit baseline:
  `G:\Builder3D-Rust_snapshot_20261030-132411_rebuild-plan-baseline`;
  robocopy exit 1, 443 files and 50,471,141 bytes in source and snapshot.
- Final recovery target:
  `G:\Builder3D-Rust_snapshot_20261030-132819_rebuild-plan-final`.
  Includes source/tests/assets/docs/evidence/work log/hidden Git metadata;
  excludes `target` and `.cache`, disables junction traversal. Copy, relative
  file sets, byte totals and SHA-256 verification follow; the appended receipt
  explicitly postdates the snapshot to avoid modifying an existing backup.
- Application implementation has not begun. Exact next work after the owner
  directs execution: RB-00 reference inventory, real original captures, asset
  provenance and feature-level gap/reuse ledger. No Python files, application
  source, existing snapshots or commits changed. Existing untracked work stays
  intact.

### Plan-reset final verification receipt (appended after snapshot)

- Snapshot: `G:\Builder3D-Rust_snapshot_20261030-132819_rebuild-plan-final`.
- Verified at `2026-10-30T13:28:55-06:00`: robocopy exit 1; source/snapshot
  each 447 files and 50,517,770 bytes; complete relative file sets and all 447
  SHA-256 hashes matched. Includes evidence, work log and hidden Git metadata;
  excludes only `target` and `.cache`; junction traversal disabled.
- Final document audit passed: 30 catalog IDs/references, ten documents,
  seven local Markdown links, and unchanged source/package path sets/hashes.
  Historical work-log whitespace was preserved.
- This receipt postdates the immutable snapshot; that snapshot includes the
  full plan, correction evidence and work log up to this receipt. It has not
  been modified to insert its own verification result.
- Handoff: planning/status reset complete; application implementation remains
  pending at RB-00. No claim of repaired functionality or visual acceptance.

## 2026-10-30 - RB-00 next-session handoff

- Added `docs/product-rebuild-handoff.md` to make the next slice executable:
  exact instruction/read order, current status, original source/test map,
  history reconciliation, valid capture rules, owner-art provenance fields,
  Rust reuse rubric, RB-00 output paths and feature-ledger schema, completion
  review, and a ready-to-paste prompt.
- Linked the handoff from `AGENTS.md` and section 12 of the rebuild plan.
- No source or application behavior changed; no Python files/assets/settings or
  projects and no existing snapshots were modified. No app launch or test suite
  was performed for this docs-only request. Existing component maturity is
  unchanged. No commit created.
- Pre-edit baseline snapshot:
  `G:\Builder3D-Rust_snapshot_20261030-134045_RB-00-handoff-baseline`; Robocopy
  exit 1; 447 files / 50,518,780 bytes; all 447 SHA-256 hashes matched; junction
  traversal disabled; `target` and `.cache` excluded.
- Evidence: `evidence/rebuild/00-plan-reset/handoff-checkpoint.md` and
  `handoff-verification.md`.
- Planned immutable final snapshot:
  `G:\Builder3D-Rust_snapshot_20261030-134308_RB-00-handoff-final`. Its
  verification receipt will be appended after copying so the immutable snapshot
  is not modified to include its own receipt.
- The next authorized task, when started, is RB-00 only. It does not include
  implementation of RB-01/RB-02.

### RB-00 handoff snapshot receipt (appended after copy)

- Immutable snapshot: `G:\Builder3D-Rust_snapshot_20261030-134308_RB-00-handoff-final`.
- Verified at `2026-10-30T13:43:28-06:00`: Robocopy exit 1; source and
  snapshot each 450 files / 50,539,566 bytes; relative file sets matched and
  all 450 SHA-256 hashes matched.
- Included source, manifests, tests, docs, evidence, work log and hidden Git
  metadata. Excluded `target` and `.cache`; junction traversal disabled.
- The receipt is appended after copying; the immutable snapshot contains the
  complete handoff and the work log up to the pre-copy snapshot statement.
- Handoff is ready. No implementation, app launch or test suite was performed.

## 2026-10-30 - RB-00 reference and reuse audit

- Completed the RB-00 evidence package in `evidence/rebuild/`: a 114-record
  requirement ledger, source/test map, dated decision reconciliation, inspected
  historical reference baseline, original asset provenance and a risk-ranked
  Rust reuse/gap map. See `evidence/rebuild/rb-00-reference-audit/report.md`
  and `checkpoint.md`.
- The ledger and asset manifest parsed successfully. Static checks found no
  missing referenced Python/Rust source paths, focused-test names or Rust
  component IDs. Python and Rust declare the same 60 MCP tool names in the same
  order; this does not prove live shared-document or UI parity.
- Six inspected historical ribbon-strip captures were copied byte-for-byte to
  `evidence/rebuild/rb-00-reference-audit/reference/`; each is 1440x138. A
  current full-window reference capture remains pending because an existing
  3D Modeler MCP gateway/runtime was active and the original project requires
  one foreground target at a time. The process was left untouched; no Python
  or Rust window was launched. The old blank Python capture is disqualified.
- Asset inventory: 108 original files, including 104 themed icon PNGs, four
  shared branding/legacy files, 51 owner-supplied light input PNGs and 52
  prepared light outputs. Original file hashes matched; no art was copied into
  the Rust product. Existing eframe 0.37.0 remains the default; no toolkit
  change, component maturity promotion, source change or commit was made.
- Focused Python/Rust tests were inspected, not run. No build, broad Python
  suite, app launch, source mutation, project/settings/asset change or existing
  snapshot modification was performed. Existing Rust untracked work was
  preserved. The original `L:\Builder3D` checkout was read-only; its observed
  14:25 local status had 69 tracked changes and 154 untracked paths.
- Exact next slice: RB-01 Visual Foundations, beginning with a safe serial
  current-original Light/Dark capture, then a matched design board and bounded
  eframe presentation for owner direction. RB-02 and all implementation remain
  unstarted.
- Planned immutable RB-00 recovery snapshot:
  `G:\Builder3D-Rust_snapshot_20261030-142550_RB-00-final`. Its verification
  receipt will be appended after copying; the existing snapshots remain
  unchanged.

### RB-00 final snapshot receipt (appended after copy)

- Immutable snapshot: `G:\Builder3D-Rust_snapshot_20261030-142550_RB-00-final`.
- Verified at `2026-10-30 14:30:21 -06:00`: Robocopy exit 1; source and snapshot
  each contained 460 files and 51,075,936 bytes. Relative file sets and all 460
  SHA-256/size pairs matched; no reparse directories were traversed.
- Included the completed RB-00 ledger, report, asset manifest, six inspected
  captures, checkpoint, existing Rust work and the work log through the
  pre-copy snapshot plan. Excluded root `target/` and `.cache/`.
- This receipt postdates the immutable snapshot. Existing G: snapshots remain
  unchanged; no commit was created and no application implementation began.

## 2026-10-30 - RB-01 visual foundations handoff

- Added `docs/rb-01-visual-foundations-handoff.md` with the RB-01 read order,
  boundaries, safe capture matrix, owner-art packaging, comparison protocol,
  focused verification, deliverables and exit gate.
- The handoff uses the plan's 1280x840 logical comparison target and records
  Python's `resize(1280, 820)` startup size separately. It does not treat the
  two as interchangeable.
- Updated `AGENTS.md` and section 12 of the rebuild plan to route the next
  session to RB-01. No application/source/assets/settings/projects or existing
  snapshots were changed. No tests/build were run and no commit was created.
- The RB-01 execution itself remains unstarted. The existing 3D Modeler MCP
  process state must be checked again before attempting live captures.

## 2026-10-30 - RB-01 Visual Foundations

- Executed the Visual Foundations handoff only. Read the product rebuild plan,
  RB-01 handoff, Rust/workspace and original project instructions. The original
  `L:\Builder3D` tree remained read-only; no commit or staging was performed.
- Updated the existing eframe shell with the source Light/Dark palettes and Dark
  default, platform UI font selection, source ribbon geometry and original icon
  maps, responsive favorites overflow, original Welcome artwork, Items-panel
  artwork, and the Move/Rotate/Resize presentation strip. The separate renderer
  and viewport workflows were not integrated.
- Copied the 108 source assets byte-for-byte into
  `crates/modeler3d-app/assets/original`. The RB-01 manifest records source and
  package destinations and all SHA-256 values. The 43,340,965-byte art set is
  1,397,925 bytes above the existing 40 MiB installed limit before the app is
  counted; no source images were changed or omitted to force the size gate.
- Added `evidence/rebuild/rb-01-visual-foundations/` with the report, incomplete
  comparison board, historical crop metadata, packaged-art manifest,
  verification, and checkpoint. Added five visual requirements and updated
  twelve existing records in `evidence/rebuild/parity-matrix.toml`; live visual,
  interactive, and owner-acceptance fields remain pending.
- The requested comparison target is 1280 x 840 logical px. The Python source
  separately calls `MainWindow.resize(1280, 820)` for its startup outer window;
  actual client sizes for both apps remain unknown. Host primary display is
  2560 x 1440 px with a 2560 x 1392 work area; system DPI is 96 (100%).
- Live captures were blocked by the one-window rule. At 15:44 MDT the original
  MCP wrapper PID 11432, server PID 29252, and Python 3.12 child PID 24544 were
  active. No application window was launched and no process was stopped. The
  inspected RB-00 Light/Dark ribbon crops remain historical reference only.
- `cargo fmt --all -- --check`, `cargo check --workspace --locked`, and
  `cargo clippy --workspace --lib --locked -- -D warnings` passed. Asset hashes,
  TOML manifests, ledger statuses, and board links were statically checked.
  Tests, GUI capture, and package staging were not run.
- RB-01 remains pending matched live Light/Dark captures and explicit owner
  review. Exact next work is to resume capture/review safely; stop before RB-02.
- Planned additive recovery snapshot:
  `G:\Builder3D-Rust_snapshot_20261030-154809_RB-01-visual-foundations`. It will include the source, copied artwork, evidence,
  work log, and Git metadata; exclude only root `target/`, root `.cache/`, and
  reparse directories; and be verified for Robocopy result, relative file sets,
  counts, bytes, and recovery-critical hashes. Append the verification receipt
  to `WORK_LOG.md` after the copy so the receipt postdates the snapshot.

### RB-01 recovery snapshot receipt (appended after copy)

- Immutable snapshot: `G:\Builder3D-Rust_snapshot_20261030-154809_RB-01-visual-foundations`.
- Verified at `2026-10-30T15:48:57-06:00`: Robocopy exit 1; source and snapshot each contain 575 files and 94631821 bytes.
- Relative file sets and every SHA-256/size pair matched. Reparse-point entries were not traversed; root `target/` and `.cache/` were excluded.
- Recovery-critical SHA-256 values (all matched):
  - `Cargo.toml`: `e6d742eb4fe2d7b0a1e227c229f6ef13261369c0d3747f87dd891d2c5facd98b`
  - `Cargo.lock`: `910bbe012c44a8b13195081831bf8ab581e09116284bfb229f21da03a2f0960d`
  - `AGENTS.md`: `b3d8b30ce2f3149cefa34c2e6a9d2486af75d016ec7cd8aef1b82a9d487b4927`
  - `docs/product-rebuild-plan.md`: `f7fc5ae48d874c3770c2793b4b6ed937f3680af7878a7beee16129acf2f01b19`
  - `docs/rb-01-visual-foundations-handoff.md`: `078362d3b853c34f8567220303cea35f96250c581e172359cd95b06039de20fb`
  - `crates/modeler3d-app/src/artwork.rs`: `8d273687633b69a09732ee417d543f221b48d305e2cc06044fafc3072354e337`
  - `crates/modeler3d-app/src/shell.rs`: `2abe463aa64094e1833ee54ad06c24fb41b2139d43f9f480b56833f2fc36f74e`
  - `crates/modeler3d-app/src/theme.rs`: `4796af22855c673befa1ffb746ddd52fa72447281f1ad5e5c760e0ef204790c2`
  - `crates/modeler3d-app/assets/original/Modeler3D.png`: `44bcc68aab7a4b7c78e92eab2dd2a1609dda18bb51e28b8e0d80cfd41d69162b`
  - `evidence/rebuild/rb-01-visual-foundations/asset-manifest.toml`: `ba5ab80acae320c63b698a4c403c21eaab0579de06bc5834b2e78d484387d87e`
  - `evidence/rebuild/rb-01-visual-foundations/report.md`: `8868bdf45518ccec262decf60e06419ddc1db908dc18108f8472e4171d8702d1`
  - `evidence/rebuild/rb-01-visual-foundations/checkpoint.md`: `26315869ae3a07e630b8c685cf506a4bd5f3e4f756a36878ba0a11630a9dd5d9`
  - `evidence/rebuild/parity-matrix.toml`: `04d06011237a8db9fe5e233b2cdb84a0cead886f895a6a491a0173633a641fc0`
  - `WORK_LOG.md`: `5d1f76e82f1f66de5798a70bc3eae5938ce76998048792825599e9574bb68894`
- The snapshot includes the RB-01 source, copied artwork, parity ledger, report, comparison board, verification, checkpoint, and pre-copy work log. Existing G: snapshots were not modified.
- This receipt was appended after copying and is not included in the immutable snapshot; the snapshot contains the complete RB-01 entry through the pre-copy snapshot plan.

## 2026-10-30 - RB-02 first working application handoff preparation

- Prepared `docs/rb-02-first-working-application-handoff.md` with the RB-01
  entry gate, read order, bounded user walkthrough, existing Rust integration
  seams, native renderer/input requirements, acceptance matrix, comparison
  evidence, future verification, non-goals, deliverables and a short prompt.
- Added an informational pointer in section 12 of
  `docs/product-rebuild-plan.md`. It explicitly preserves RB-01 as the next
  execution and says the RB-02 handoff does not clear its capture/owner gate.
- Source inspection confirmed the eframe shell still uses rectangle preview
  selection and typed-path dialogs; `CommandState` has the core command and
  project paths; `ViewportRenderer` is an offscreen wgpu renderer; reusable
  camera, exact-picking and gizmo helpers are present.
- RB-01 live Light/Dark captures and explicit owner approval remain pending.
  `L:\Builder3D` was not modified. No code, tests, build, application launch,
  capture, staging or commit was performed.
- Exact next work remains RB-01 capture/review completion; stop before RB-02
  if its one-window or owner gate remains blocked.
- Planned additive recovery snapshot:
  `G:\Builder3D-Rust_snapshot_20261030-160009_RB-02-handoff`. It will exclude
  only root `target/`, root `.cache/` and reparse entries; its copy result,
  file set, counts, bytes and recovery-critical hashes will be verified. The
  receipt will be appended after copying.

### RB-02 handoff recovery snapshot receipt (appended after copy)

- Immutable snapshot: `G:\Builder3D-Rust_snapshot_20261030-160009_RB-02-handoff`.
- Verified at `2026-10-30T16:02:22-06:00`: the Robocopy `/L` recheck returned
  exit code 0 (no pending copy differences). Source and snapshot each contain
  577 files and 94,659,156 bytes. Relative file sets and every SHA-256/size
  pair match; no reparse entries were found or traversed. Excluded root
  `target/` and `.cache/`.
- Recovery-critical SHA-256 values at copy time (all matched):
  - `Cargo.toml`: `e6d742eb4fe2d7b0a1e227c229f6ef13261369c0d3747f87dd891d2c5facd98b`
  - `Cargo.lock`: `910bbe012c44a8b13195081831bf8ab581e09116284bfb229f21da03a2f0960d`
  - `AGENTS.md`: `b3d8b30ce2f3149cefa34c2e6a9d2486af75d016ec7cd8aef1b82a9d487b4927`
  - `docs/product-rebuild-plan.md`: `a5eff727b0787d75c7d656f9f56a40ecdef3f709bd12727ba44cb258de48e32c`
  - `docs/rb-02-first-working-application-handoff.md`: `af9be8122be43ad47f4336107a1f6f02adea3bec48d6ace9044e9510272753a0`
  - `crates/modeler3d-app/src/shell.rs`: `2abe463aa64094e1833ee54ad06c24fb41b2139d43f9f480b56833f2fc36f74e`
  - `crates/modeler3d-viewport/src/renderer.rs`: `e05f7f8934a798c7b7e186804d10fd24c4360f7c937caf4769efb2180843ff01`
  - `evidence/rebuild/parity-matrix.toml`: `04d06011237a8db9fe5e233b2cdb84a0cead886f895a6a491a0173633a641fc0`
  - `evidence/rebuild/rb-02-first-working-application/checkpoint.md`: `b3a5862335c92b69e67853b6c1dab0d6ba4f752c54e5a89be12c8916b6b54cfd`
  - `WORK_LOG.md` before this receipt: `5b4e35521c85587dd74a562af032e8382e57b153356302144daa29c8c9f52071`
- The snapshot contains the RB-02 handoff, plan pointer, checkpoint, and
  work-log entry through the pre-copy snapshot plan. This receipt postdates
  the immutable snapshot and is not included in it. Existing G: snapshots were
  not modified.

### RB-01 gate-recheck recovery snapshot receipt (appended after copy)

- Immutable snapshot: `G:\Builder3D-Rust_snapshot_20261030-190116_RB-01-gate-recheck`.
- Verified at `2026-10-30T19:03:13-06:00`. Robocopy exit code 1 indicates the
  initial copy completed. Source and snapshot each contain 577 files and
  94,666,430 bytes; relative file sets and every relative-path/size/SHA-256
  tuple matched with zero differences.
- Root `target/` and `.cache/` were excluded. Robocopy `/XJ` was used and the
  verification walk skipped reparse points without traversing them.
- Recovery-critical SHA-256 values at copy time (all matched):
  - `Cargo.toml`: `e6d742eb4fe2d7b0a1e227c229f6ef13261369c0d3747f87dd891d2c5facd98b`
  - `Cargo.lock`: `910bbe012c44a8b13195081831bf8ab581e09116284bfb229f21da03a2f0960d`
  - `AGENTS.md`: `b3d8b30ce2f3149cefa34c2e6a9d2486af75d016ec7cd8aef1b82a9d487b4927`
  - `docs/product-rebuild-plan.md`: `a5eff727b0787d75c7d656f9f56a40ecdef3f709bd12727ba44cb258de48e32c`
  - `docs/rb-02-first-working-application-handoff.md`: `af9be8122be43ad47f4336107a1f6f02adea3bec48d6ace9044e9510272753a0`
  - `crates/modeler3d-app/src/shell.rs`: `2abe463aa64094e1833ee54ad06c24fb41b2139d43f9f480b56833f2fc36f74e`
  - `crates/modeler3d-viewport/src/renderer.rs`: `e05f7f8934a798c7b7e186804d10fd24c4360f7c937caf4769efb2180843ff01`
  - `evidence/rebuild/parity-matrix.toml`: `787800c8de476e9118a453dbc0fb1ffad7ce5dd8f9b01f8947910ce8edb565ec`
  - `evidence/rebuild/rb-01-visual-foundations/report.md`: `88e386aa60e84d50e39637468e9e6a002b612e4e2a77e6fd4b4502c94a023e81`
  - `evidence/rebuild/rb-01-visual-foundations/verification.md`: `3bc40969bb9a2619ff00dfc462d85749e274abeec4ddb94a87ae77f7da0e32b6`
  - `evidence/rebuild/rb-01-visual-foundations/capture-board.md`: `51ceed99cd5849f1ccc635d0bb3db66473ac3beba0f9b72b88825eb68382cb5f`
  - `evidence/rebuild/rb-01-visual-foundations/capture-metadata.toml`: `0e07a48475d97d2701245add204b57797f50e65bcf7656dde54ecedd48f93091`
  - `evidence/rebuild/rb-01-visual-foundations/checkpoint.md`: `beb9d61e403021b72eb6185caf63cf448b43dcbaef130f027a2ebfad55cfef1b`
  - `WORK_LOG.md` before this receipt: `44799e22f6269584d8496cffd908259ef2d0448bbd87fa8a49e00584ed0f3fed`
- The receipt postdates the immutable snapshot and is not included in it. The
  snapshot contains the work-log entry through the pre-copy snapshot plan;
  existing G: snapshots were not modified.

## 2026-10-30 - RB-01 historical reference review

- Continued only RB-01 work. Read the original Python visual evidence and
  cross-checked it against the Rust rebuild plan. `L:\Builder3D` remained
  read-only; no app was launched while its MCP gateway was active.
- Found the 2026-09-19 live Light/Dark previews referenced by the original
  `WORK_LOG.md` and `docs/testing.md`. Copied them unchanged to
  `evidence/rebuild/rb-01-visual-foundations/reference/`, opened and inspected
  both, and recorded their hashes and limits. Each image is 1280 x 820 px, but
  its logical client size, display scale, and window state are unknown. They
  are historical context, not matched acceptance captures.
- Corrected the RB-01 report's Undo/Redo note: Python's `make_icon` resolves
  mapped raster files, while the current style guide specifies generated
  curved vector arrows. This source/contract discrepancy remains pending live
  comparison; the Rust UI was not changed based on an uncertain visual result.
- Updated the comparison board, capture metadata, verification, checkpoint and
  parity audit. No tests, build, or application interaction were run.
- Planned additive recovery snapshot:
  `G:\Builder3D-Rust_snapshot_20261030-192216_RB-01-historical-reference-review`.
  It will include the source, documentation, evidence, work log and Git
  metadata; exclude root `target/`, root `.cache/` and reparse entries. Verify
  Robocopy status, relative file sets, counts, bytes and recovery-critical
  hashes, then append the receipt after copying.

### RB-01 historical-reference recovery snapshot receipt (appended after copy)

- Immutable snapshot: `G:\Builder3D-Rust_snapshot_20261030-192216_RB-01-historical-reference-review`.
- Verified at `2026-10-30T19:23:46-06:00`. Robocopy exit code 1 indicates the
  initial copy completed. Source and snapshot each contain 579 files and
  94,783,257 bytes; relative file sets and every path/size/SHA-256 tuple
  matched with zero differences.
- Root `target/` and `.cache/` were excluded. Robocopy `/XJ` was used; the
  verification walk did not descend into reparse points.
- Recovery-critical SHA-256 values at copy time (all matched):
  - `Cargo.toml`: `e6d742eb4fe2d7b0a1e227c229f6ef13261369c0d3747f87dd891d2c5facd98b`
  - `Cargo.lock`: `910bbe012c44a8b13195081831bf8ab581e09116284bfb229f21da03a2f0960d`
  - `AGENTS.md`: `b3d8b30ce2f3149cefa34c2e6a9d2486af75d016ec7cd8aef1b82a9d487b4927`
  - `docs/product-rebuild-plan.md`: `a5eff727b0787d75c7d656f9f56a40ecdef3f709bd12727ba44cb258de48e32c`
  - `docs/rb-02-first-working-application-handoff.md`: `af9be8122be43ad47f4336107a1f6f02adea3bec48d6ace9044e9510272753a0`
  - `crates/modeler3d-app/src/shell.rs`: `2abe463aa64094e1833ee54ad06c24fb41b2139d43f9f480b56833f2fc36f74e`
  - `crates/modeler3d-app/src/theme.rs`: `4796af22855c673befa1ffb746ddd52fa72447281f1ad5e5c760e0ef204790c2`
  - `crates/modeler3d-viewport/src/renderer.rs`: `e05f7f8934a798c7b7e186804d10fd24c4360f7c937caf4769efb2180843ff01`
  - `evidence/rebuild/parity-matrix.toml`: `3ab637dbbf9f365e5bdf57a9d2eab737f9837d6e623c32bb6125e123fbadc2f7`
  - `evidence/rebuild/rb-01-visual-foundations/report.md`: `34607c54c11848883f0e541c0df63de7450360b7ed040554f9c9887f00dd4d57`
  - `evidence/rebuild/rb-01-visual-foundations/verification.md`: `2f998a8ecb931621da8072eb6b18b0003a1a9a34ce1608782c646e270f97da5c`
  - `evidence/rebuild/rb-01-visual-foundations/capture-board.md`: `13771e63b9ed79eff27fadac75d1e357b5ef2152e0b236c8fd1b18fd8751262b`
  - `evidence/rebuild/rb-01-visual-foundations/capture-metadata.toml`: `e59bbfbdb8581a66713828f911b35389105bfd0c8baa1edab77ad022358f87a6`
  - `evidence/rebuild/rb-01-visual-foundations/checkpoint.md`: `f99fa946d8a5e6283ff61ee7abba45b17959b8483b493b42d73e8dfee57d0320`
  - Light reference image: `9f80268a821b23730fa8aaf8be873a47d2b08969382c1b1e703d0815ac271de5`
  - Dark reference image: `32a3270d0de263b848f6255664088fabb8274e462d608676dfc0ffac321b129f`
  - `WORK_LOG.md` before this receipt: `ec66cc932a7a78ba987b1314e0eebd56cfed7e2625a5c1e4f499eec0c1638381`
- This receipt postdates the immutable snapshot and is not included in it. The
  snapshot contains the pre-copy entry and all historical references; no
  existing G: snapshot was modified.

## 2026-10-30 - RB-01 live-capture gate recheck

- Read the current workspace/project instructions, rebuild plan, RB-01 report,
  verification, comparison board, metadata and checkpoint, and the RB-02
  handoff. The evidence confirms RB-01 has neither matched live Light/Dark
  captures at 1280 x 840 logical client pixels nor explicit owner approval.
- Rechecked the one-window process gate at 18:58 MDT. The original MCP gateway
  was active: `run_mcp_windows.bat` wrapper PID 30064, venv server PID 25284,
  and child PID 9440. No 3D Modeler or Rust target window was present. No process
  was stopped and neither app was launched.
- Re-measured the host primary display at 2560 x 1440, 96 system DPI (100%).
  Actual target-window client sizes and per-window display scale remain unknown.
  The active gateway blocks a safe serial capture, so RB-01 visual review and
  owner acceptance remain pending and RB-02 was not started.
- Updated RB-01 report, verification, comparison board, capture metadata,
  checkpoint, and the RB-01 audit summary in `parity-matrix.toml`. Acceptance
  fields remain pending. This evidence-only check ran no tests, build, or UI
  interaction. `L:\Builder3D` remained read-only; no staging or commit occurred.
- Planned additive recovery snapshot:
  `G:\Builder3D-Rust_snapshot_20261030-190116_RB-01-gate-recheck`. It will
  include the workspace source, documentation, evidence, work log, and Git
  metadata; exclude only root `target/`, root `.cache/`, and reparse entries.
  Verify Robocopy status, relative file sets, counts, bytes, and critical hashes.
  Append the receipt after the snapshot copy.

## 2026-10-31 - RB-02 conditional execution handoff

- Updated the RB-02 handoff, rebuild plan, workspace instructions, and current
  checkpoint to preserve the user's conditional gate: matched live RB-01
  Light/Dark captures at 1280 x 840 logical pixels plus explicit owner approval
  are required before RB-02. If either is missing, do safe RB-01 evidence work
  only and stop.
- Clarified the one-window workflow: capture the Python app, close it, then
  capture Rust, close it, and compare the saved files. This serial workflow is
  allowed; a background MCP gateway without a target UI window is not a blocker.
  The reference at `L:\Builder3D` remains read-only.
- Marked the older RB-00 handoff as historical and clarified the RB-01 handoff
  still owns its capture/approval gate. Added a copy-ready conditional prompt
  for the fresh session and kept the exit before RB-03.
- No application files changed; no tests, build or UI capture were run. No
  files were staged or committed.
- Planned additive snapshot:
  `G:\Builder3D-Rust_snapshot_20261031-074115_RB-02-conditional-handoff`. It will
  include source, Git metadata, docs, evidence and this work log; exclude only
  root `target/`, root `.cache/` and reparse entries. Verify Robocopy result,
  relative path set, file count, bytes and recovery-critical hashes. Append the
  snapshot receipt after copy and state that it postdates the snapshot.

### RB-02 conditional handoff snapshot receipt (appended after copy)

- Immutable snapshot: `G:\Builder3D-Rust_snapshot_20261031-074115_RB-02-conditional-handoff`.
- Verified at `2026-10-31T07:42:17-06:00`. Robocopy exit code 1 indicates the
  initial copy completed. Source and snapshot each contain 579 files and
  94,790,756 bytes. Full relative path sets and every path/size/SHA-256 tuple
  matched with zero differences.
- Root `target/` and `.cache/` were excluded. Robocopy `/XJ` was used; the
  verification walk skipped reparse points.
- Recovery-critical SHA-256 values at copy time (all matched):
  - `AGENTS.md`: `29a190ae10b3b7dd8f2aa47b78a9b46e03e80e7f26576afbd6cb0ef99e264095`
  - `Cargo.toml`: `e6d742eb4fe2d7b0a1e227c229f6ef13261369c0d3747f87dd891d2c5facd98b`
  - `Cargo.lock`: `910bbe012c44a8b13195081831bf8ab581e09116284bfb229f21da03a2f0960d`
  - `docs/product-rebuild-plan.md`: `147c70ed47785f3cf550050e11a4ebd72b4cec4387a736ab3e6b8c431fd1d450`
  - `docs/rb-02-first-working-application-handoff.md`: `ad8d0d33a93d6c3c3c7bf472058bc25ac9c0fc7bbb734d1f23c51e7633b1b553`
  - `evidence/rebuild/rb-02-first-working-application/checkpoint.md`: `e52822ea2dc283c3387750136d9df39dcfc296c60792a5b2960ca53ade2941c4`
  - `evidence/rebuild/parity-matrix.toml`: `3ab637dbbf9f365e5bdf57a9d2eab737f9837d6e623c32bb6125e123fbadc2f7`
  - `WORK_LOG.md` before this receipt: `5ab671cba314caf2cf62095e8862ffd1c61a7e7a03d622d0c22b636f9bfd8935`
- This receipt postdates the immutable snapshot and is not included in it.
  The snapshot contains the pre-copy handoff entry. Existing G: snapshots were
  not modified.

## 2026-10-31 - RB-01 matched live capture evidence

- Built the Rust app with cargo build --package modeler3d-app --locked (exit 0).
  No automated tests were run.
- Captured the Python reference and Rust app serially in Dark and Light at
  1280 x 840 logical client pixels. All four physical PNGs are 1280 x 840;
  system and per-window DPI are 96 (100%). The background MCP gateway remained
  running and untouched, with no target UI window during the capture sessions.
- Isolated Python QSettings/runtime paths and Rust app-data paths. The source
  at L:\Builder3D remained read-only. Both target apps were closed after capture.
- Added the current images, 1:1 comparison boards, per-image hashes and window
  measurements. Updated the RB-01 board, metadata, report, verification,
  checkpoint, and parity-matrix audit. The images show an empty Welcome with
  Insert active. Observed differences include Welcome alignment, Recent layout,
  the right panel, and the Rust hamburger glyph. Owner visual review remains
  pending; RB-02 was not started.
- Temporary isolated profiles and dialog diagnostics remain under the ignored
  target/rb01-capture-20261031-075324 directory and are excluded from recovery.
- Planned additive recovery snapshot:
  G:\Builder3D-Rust_snapshot_20261031-081447_RB-01-live-captures. It will include
  source, evidence, documentation, and Git metadata; exclude only root target/,
  root .cache/, and reparse entries. Verify Robocopy status, relative path sets,
  file count, bytes, and every path/size/SHA-256 tuple. Append the receipt after
  the copy.
### RB-01 live-capture evidence snapshot receipt (appended after copy)

- Immutable additive snapshot: G:\Builder3D-Rust_snapshot_20261031-081447_RB-01-live-captures.
- Verified at 2026-10-31T08:17:24-06:00. Robocopy exit code 1 indicates the initial copy completed.
  Source and snapshot each contain 585 files and 95,585,673 bytes. Relative path
  sets and every path/size/SHA-256 tuple matched with zero differences.
- Root target/ and .cache/ were excluded. Robocopy /XJ was used. The verification
  walk found and skipped zero reparse points in both trees.
- Recovery-critical SHA-256 values at copy time (all matched):
  - AGENTS.md: 29a190ae10b3b7dd8f2aa47b78a9b46e03e80e7f26576afbd6cb0ef99e264095
  - Cargo.toml: e6d742eb4fe2d7b0a1e227c229f6ef13261369c0d3747f87dd891d2c5facd98b
  - Cargo.lock: 910bbe012c44a8b13195081831bf8ab581e09116284bfb229f21da03a2f0960d
  - docs/product-rebuild-plan.md: 147c70ed47785f3cf550050e11a4ebd72b4cec4387a736ab3e6b8c431fd1d450
  - docs/rb-02-first-working-application-handoff.md: ad8d0d33a93d6c3c3c7bf472058bc25ac9c0fc7bbb734d1f23c51e7633b1b553
  - crates/modeler3d-app/src/lib.rs: 3e6471e163a06d58fd49d9e1b34c5390087c0e74baf7d7cae4a601b264265e33
  - crates/modeler3d-app/src/shell.rs: 2abe463aa64094e1833ee54ad06c24fb41b2139d43f9f480b56833f2fc36f74e
  - crates/modeler3d-app/src/theme.rs: 4796af22855c673befa1ffb746ddd52fa72447281f1ad5e5c760e0ef204790c2
  - crates/modeler3d-viewport/src/renderer.rs: e05f7f8934a798c7b7e186804d10fd24c4360f7c937caf4769efb2180843ff01
  - evidence/rebuild/parity-matrix.toml: 94ba7d7f3111d799c13b2dddd8e9fa3f27971a00d54e07e93a25800d971587a8
  - RB-01 report: 2bc0947f1ef11fa8b33d3e974a075151e21bdbdbb0f84fccba9e7d2f193c5e40
  - RB-01 verification: 6bc4f93f61478538a71c2d3f4c201ff2de0175d44b3ef4005f3e6a22f8c4a2f9
  - RB-01 comparison board: d9d99fa5e7756ae90909679ae4cde31da42ea1f14d9d4a32ea91074fd58f322f
  - RB-01 capture metadata: 3f1fa84334a69bac2e693424c9015dfbfeab6f976dbe9a58c81103445f1e8bd1
  - RB-01 checkpoint: ab7c94f72a847838bfa02c1a03a067a592a49af92db9af5a197b30532b4e8ce5
  - Python Dark capture: 2846c63cdc73d8b711b93a0f41832bd6451a0a2239328ab7c481fa7a041abe01
  - Python Light capture: 7dcfa0bd6e20c385d503b8ef9ba93e06dd658e5ad9c4fbd0d75c4d2e56610a84
  - Rust Dark capture: 8ed1a388c5da8a5a64970f8ee76be424665706637555227cdeaed9cf3b51de51
  - Rust Light capture: 3ea77d9a4cda2dde18e680bc729bb36a21a916f758b4d7a9fdb9a455e7da5e0b
  - Dark comparison: df25efb7cfff99c99dc3096e56414af8ba9c8fc0979c97347a4a61bcf2920c8e
  - Light comparison: 88ac994881db445e76f8b4ac24e934f140db679f072ef9c8b7ccfd77337252a8
  - WORK_LOG.md before this receipt: 4194e1516f7a5b486aefed3c91720ceba96276059ebdccc49dd87714ed04010f
- This receipt postdates the immutable snapshot and is not included in it. The
  snapshot contains the pre-copy work-log entry. Existing G: snapshots were
  not modified.

### RB-02 authorization and pre-change baseline — 2026-10-31

- The owner’s latest instruction, “Why are you stopping? Finish all of rb-02,”
  directly authorizes proceeding with the bounded RB-02 implementation. It does
  not approve RB-02’s final result; representative owner review remains an exit
  gate, and work stops before RB-03.
- The RB-01 checkpoint contains the matched serial Python/Rust Light and Dark
  captures at 1280 x 840 logical pixels, measured actual client size and 96 DPI
  scale, and comparison findings. Its design review had been pending before
  this direct RB-02 instruction.
- Pre-change baseline: `G:\Builder3D-Rust_snapshot_20261031-094935_RB-02-baseline`.
  Source and snapshot each contain 585 files and 95,588,534 bytes; relative
  paths match exactly. Robocopy list-only verification returned 0. Eight
  recovery-critical SHA-256 values matched, including Cargo files, app shell
  and commands, viewport renderer, RB-01 checkpoint, and `.git/HEAD`.
- Snapshot exclusions: only root `target/`, root `.cache/`, and reparse-point
  trees. The verification walk found no reparse points. Existing G: snapshots
  and `L:\Builder3D` were not modified. This receipt postdates the immutable
  snapshot.
- No RB-02 application code has changed and no RB-02 checks have run yet.
- Exact next subtask: connect the existing R5 renderer to eframe’s existing
  wgpu device and native egui texture presentation, preserving renderer budget
  accounting and removing per-frame GPU waits for normal display.

### RB-02 shared-device renderer patch — 2026-10-31

- Added `ViewportRenderer::new_on_shared_device`, which clones eframe’s existing
  wgpu handles and maps the actual adapter to the discovered capability profile.
  Added a native egui-compatible `Rgba8Unorm` target, sRGB shader encoding,
  bounded resize with overlapping target budget reservation, theme clear-color
  setter, and texture/adapter accessors. Ordinary rendering no longer waits for
  GPU completion; explicit diagnostic capture retains its bounded readback wait.
- Verification: `cargo fmt --all -- --check`; `cargo check -p
  modeler3d-viewport --locked`; `cargo test -p modeler3d-viewport --locked`
  (24 passed); `cargo clippy -p modeler3d-viewport --all-targets --locked --
  -D warnings` all passed.
- Additive recovery snapshot:
  `G:\Builder3D-Rust_snapshot_20261031-095659_RB-02-01-shared-renderer`.
  Robocopy exit code 1; source and snapshot both contain 585 files and
  95,598,379 bytes; relative path differences 0; six recovery-critical hashes
  matched. Root `target/`, root `.cache/`, and reparse-point trees were excluded.
- This receipt postdates the snapshot; the snapshot contains the preceding
  RB-02 baseline receipt and the renderer source before this receipt.
- Exact next subtask: initialize the renderer from `CreationContext`, register
  and resize its texture with egui-wgpu, render the scene in the existing
  viewport, and route physical-pixel camera and exact-pick input through the
  authoritative scene.

### RB-02 native viewport integration — 2026-10-31

- Replaced the rectangle viewport with the R5 shaded renderer on eframe's existing adapter, device, and queue; the renderer texture is registered with egui-wgpu and resized in physical pixels from the live logical viewport and pixels-per-point.
- Added shared pointer routing for orbit, pan, zoom, exact geometry pick by durable object ID, and temporary translation-gizmo preview. A committed drag uses one ID-based position command; Escape/focus loss cancels without a scene-history entry.
- Added the native import file picker and format filters while retaining the typed path on picker cancel and the explicit unit selector (default mm). Open/import requests frame the loaded scene.
- Added regression coverage for stable-ID move/undo/redo, hidden/reordered occlusion picks, pointer scale mapping, camera gestures and modal capture, picker cancel, and fixture import/move/project save-reopen persistence.
- Verification: cargo test -p modeler3d-app -p modeler3d-viewport --locked (66 app and 25 viewport tests passed); cargo check --workspace --locked; strict app/viewport Clippy with warnings denied; formatting applied. Native run and evidence are next.
- Planned additive recovery snapshot: G:\Builder3D-Rust_snapshot_20261031-101417_RB-02-02-native-viewport-integration; it will include source, evidence, work log, and Git metadata; exclude only root target/, root .cache/, and reparse entries. Its Robocopy result, relative paths, file count, bytes, and path/size/SHA-256 tuples will be verified before appending the receipt.
### RB-02 native viewport snapshot receipt -- appended after copy

- Immutable additive snapshot: G:\Builder3D-Rust_snapshot_20261031-101417_RB-02-02-native-viewport-integration.
- Verified at 2026-10-31T10:15:10-06:00. Robocopy exit code 1 indicates files copied. Source and snapshot each contain 585 files and 95,629,619 bytes.
- Relative path differences: 0. File-size differences: 0. SHA-256 differences across every included file: 0.
- The verification walk excluded only root target/, root .cache/, and reparse entries. Robocopy /XJ was used; the walk found and skipped zero reparse points.
- Recovery-critical SHA-256 values at copy time (all matched):
  - AGENTS.md: 29a190ae10b3b7dd8f2aa47b78a9b46e03e80e7f26576afbd6cb0ef99e264095
  - Cargo.toml: 90add5a7220389861e2436aa770b08c2096bec6487b0b5b2a05119c8484e9180
  - Cargo.lock: 66a38f81c099c6f2ab6a6e3380429007cf085969c6bf931212a96633a98551fb
  - WORK_LOG.md: bc84e4b25aef2c90a52bae2a0f7b081b33074612a809241885eccbde464fb458
  - crates/modeler3d-app/src/shell.rs: 56e4702c559aaca5e6e746c4e5a488ec8d91e80cd7751191bc4be6c01fadb520
  - crates/modeler3d-app/src/commands.rs: 4ab86197cce17cbc44f112e95b4e919303e679400f35859dff9316a794c91a8a
  - crates/modeler3d-app/src/dialogs.rs: 069452058573acff7f3b0bbafdd5dc2a9322893e1589b0a2acf0128f48b9c691
  - crates/modeler3d-viewport/src/renderer.rs: a1fe44004737ec1cb9ea37abddaabc2f4a9c2348db64317318c754724b6e1bbc
  - crates/modeler3d-viewport/src/picking.rs: e1a84584e3c0ceffc405070f421e15a1673b9de4500041e4da66fbc515fd33b6
  - evidence/rebuild/rb-02-first-working-application/checkpoint.md: 2cf6f4f35916fa01d81e15a8cc04f8d545c4472b0837bfe6c20a1b41c6fd072c
  - .git/HEAD: 28d25bf82af4c0e2b72f50959b2beb859e3e60b9630a5e8c603dad4ddb2b6e80
- This receipt and checkpoint update postdate the immutable snapshot. The snapshot contains the pre-receipt work-log entry and current checkpoint before this update. Existing G: snapshots were not modified.

### RB-02 first working application - 2026-10-31

- Completed the bounded real workflow in the existing eframe app: New, Cube, native STL import in mm, shaded viewport, orbit/pan/zoom, exact stable-ID pick, translation-gizmo move, undo/redo, save, and reopen. Kept L:\Builder3D read-only.
- The eframe renderer uses the app's existing wgpu adapter/device/queue and presents its scene texture through egui-wgpu. The runtime selected the Quadro RTX 4000 with Vulkan and NVIDIA driver 581.42.
- Captured and inspected 25 visible-window GDI CopyFromScreen client screenshots in Light and Dark, all 1280 x 840 logical/physical pixels at 96 DPI / 100%. Light and Dark app processes were closed between runs. Per-capture timestamps, PIDs, dimensions, byte lengths, and SHA-256 values are in evidence/rebuild/rb-02-first-working-application/capture-metadata.toml.
- The visible app walkthrough was driven by a task-local Windows input helper; results were checked in the app UI. The native Browse cancel case was exercised in Light. Saved projects and the fixture manifest are included in the evidence folder.
- Verification passed: cargo fmt --all -- --check; cargo test --workspace --locked (203 passed: app 67, core 81, MCP 14, platform 15, viewport 26); cargo check --workspace --locked; cargo clippy --workspace --all-targets --locked -- -D warnings; cargo deny check; TOML parsing of the ledger and evidence manifests.
- Cargo Deny reported duplicate-version dependency warnings; advisories, bans, licenses, and sources passed. cargo audit is not installed. Release cargo bloat reported an 11.0 MiB .text section and a 14,135,808-byte (13.48 MiB) executable. Source artwork plus the release executable totals 57,476,773 bytes (54.79 MiB) before installer/runtime sidecars, exceeding the 41,943,040-byte target.
- Updated the RB-02 feature row and audit in evidence/rebuild/parity-matrix.toml. RB-01 audit/history remains unchanged. Visual review and explicit owner acceptance remain pending; stop before RB-03.
- Additive snapshot planned: G:\Builder3D-Rust_snapshot_20261031-111700_RB-02-03-complete-workflow. Include .git and all source/evidence/log files; exclude only repository-root target/, repository-root .cache/, and reparse-point entries. The full file-set/size/SHA-256 receipt will be appended after copy.

### RB-02 complete workflow snapshot receipt -- appended after copy

- Immutable additive snapshot: G:\Builder3D-Rust_snapshot_20261031-111700_RB-02-03-complete-workflow.
- Verified at 2026-10-31T11:17:52-06:00. Robocopy exit code 1 means files were copied without failure. Source and snapshot each contain 619 files and 100,355,053 bytes.
- Relative path differences: 0. File-size differences: 0. SHA-256 differences across every included file: 0.
- Only repository-root target/, repository-root .cache/, and reparse-point entries were excluded. Robocopy /XJ was used; zero reparse points were found or skipped in the verification walk.
- Recovery-critical SHA-256 values at copy time (all matched):
  - AGENTS.md: 29a190ae10b3b7dd8f2aa47b78a9b46e03e80e7f26576afbd6cb0ef99e264095
  - Cargo.toml: 90add5a7220389861e2436aa770b08c2096bec6487b0b5b2a05119c8484e9180
  - Cargo.lock: 66a38f81c099c6f2ab6a6e3380429007cf085969c6bf931212a96633a98551fb
  - WORK_LOG.md (pre-receipt): e1e96061449caf50473e33e13d1ff5cb2e587de399bb0ac3817eab9c54b45f84
  - crates/modeler3d-app/src/shell.rs: b3ddba7f867dcbdaef845657e311afe06842a271e0a39958392c9712ab5124e3
  - crates/modeler3d-app/src/commands.rs: 4ab86197cce17cbc44f112e95b4e919303e679400f35859dff9316a794c91a8a
  - crates/modeler3d-app/src/dialogs.rs: 069452058573acff7f3b0bbafdd5dc2a9322893e1589b0a2acf0128f48b9c691
  - crates/modeler3d-viewport/src/renderer.rs: a1fe44004737ec1cb9ea37abddaabc2f4a9c2348db64317318c754724b6e1bbc
  - crates/modeler3d-viewport/src/picking.rs: e1a84584e3c0ceffc405070f421e15a1673b9de4500041e4da66fbc515fd33b6
  - evidence/rebuild/parity-matrix.toml (pre-receipt): d75a7608a74d17894ea2a926db8f9fc1b3cc7b24f9e5b394bef13a3d4c66676f
  - evidence/rebuild/rb-02-first-working-application/checkpoint.md (pre-receipt): dfee3116c40c2f4ddb4e957665f3c1c318db09f1d3dd72d6a84c4f8e73cb445c
  - evidence/rebuild/rb-02-first-working-application/capture-metadata.toml: 4cbd9f3e2f47ac41739ec1428a983b6f4b523b2cb1a913a4f6300a56b1f18f59
  - .git/HEAD: 28d25bf82af4c0e2b72f50959b2beb859e3e60b9630a5e8c603dad4ddb2b6e80
- This receipt, the final checkpoint update, and the post-copy parity snapshot statistics postdate the immutable snapshot. The snapshot contains the pre-receipt work-log entry, checkpoint, and parity ledger. Existing G: snapshots were not modified.

### RB-02 owner visual review - 2026-10-31

- The owner reviewed the Light/Dark reopened-app captures and said they are close enough to pass for now, with fine-tuning later. Recorded the current RB-02 visual baseline as accepted.
- Updated the RB-02 parity feature and audit, report, capture board, verification, and checkpoint to record acceptance.
- This acceptance note and the related documentation updates postdate the verified G: snapshot. No application code changed.
- RB-03 remains unstarted because the original task said to stop before RB-03; wait for the user's instruction before proceeding.

### RB-03 handoff preparation - 2026-10-31

- Created docs/rb-03-lifecycle-and-file-completion-handoff.md as a complete, stepwise handoff for project lifecycle, supported imports and exports, units, recents/previews, failure handling, native walkthroughs, verification, evidence, and recovery.
- Updated AGENTS.md, docs/product-rebuild-plan.md, the RB-02 checkpoint, and the parity ledger so the next slice points to RB-03 and reflects the owner's RB-02 visual acceptance.
- RB-03 implementation has not started. No application code changed. Nothing was staged or committed.
- Additive pre-RB-03 handoff baseline planned at G:\Builder3D-Rust_snapshot_20261031-115217_RB-03-handoff-baseline; append the copy and verification receipt after it is created.

### RB-03 handoff baseline snapshot receipt - appended after copy

- Immutable additive snapshot: G:\Builder3D-Rust_snapshot_20261031-115217_RB-03-handoff-baseline.
- Verified at 2026-10-31T11:54:22-06:00. Robocopy exit code 1; source and snapshot each contain 620 files and 100,376,505 bytes.
- Relative-path differences: 0. File-size differences: 0. Per-file SHA-256 differences: 0.
- Excluded repository-root target, repository-root .cache, and reparse-point entries. The verification walk found 0 reparse entries.
- Recovery-critical SHA-256 values at copy time (all matched):
  - AGENTS.md: 94274d03f67d37478ca78e06f1f077002d25198fbe84fb246ca8d27097650714
  - docs/product-rebuild-plan.md: 9f01efc6487892f4115c938f35ec57dea74406652204f6b4c9453acaca14fed7
  - docs/rb-03-lifecycle-and-file-completion-handoff.md: ac66247447c59344587f33a87ff2bdfd60cd2b509bf8737d28ad4e2b6ebf26f6
  - evidence/rebuild/parity-matrix.toml: eb042c2d061d9286b88b2a8b7378c9434fd42890ffd59ebc1ae24d9675eb8f8c
  - evidence/rebuild/rb-02-first-working-application/checkpoint.md: ff607e2fc7688dfc8786c0bc8391025b3fee35571dad5e56a73fc3f0463ec5fa
  - WORK_LOG.md (pre-receipt): d25490c0d61a79bf7af0f32a2947d28b5c8e6434a67acf4e4a1c1e65718e6a7f
  - .git/HEAD: 28d25bf82af4c0e2b72f50959b2beb859e3e60b9630a5e8c603dad4ddb2b6e80
- This receipt and its copy-time hash list postdate the immutable snapshot. The snapshot contains the pre-receipt work log, handoff, and checkpoint.

## 2026-10-31 - RB-03 requirement audit and pre-change baseline

- Read the RB-03 handoff, workspace/project instructions, Rust guide, rebuild
  plan, RB-02 evidence, parity matrix, work log, and the referenced Python file,
  import, export, recent-preview, project-format, and focused-test contracts.
- Wrote `evidence/rebuild/rb-03-lifecycle-and-file-completion/requirement-test-map.md`
  before implementation. Existing lifecycle/import/export/recent IDs remain
  stable; the map identifies missing file-format, native-picker, failure,
  selection, recent-dispatch and preview IDs for the parity ledger.
- Audited initial Git/process state: repository remains entirely untracked and
  unstaged; no Rust target window was running. Left the existing Python MCP
  gateway process untouched. `L:\Builder3D` was read-only.
- Fresh pre-change baseline: `G:\Builder3D-Rust_snapshot_20261031-115829_RB-03-pre-baseline`.
  Robocopy exit code 1; source and snapshot each contain 620 files and
  100,378,754 bytes. Relative-path, size, and per-file SHA-256 differences: 0;
  reparse points: 0. Excluded repository-root `target/`, `.cache/`, and reparse
  entries. This receipt postdates the immutable baseline snapshot.
- Recovery-critical SHA-256 values at copy time (all matched):
  - `AGENTS.md`: `94274d03f67d37478ca78e06f1f077002d25198fbe84fb246ca8d27097650714`
  - `Cargo.toml`: `90add5a7220389861e2436aa770b08c2096bec6487b0b5b2a05119c8484e9180`
  - `Cargo.lock`: `66a38f81c099c6f2ab6a6e3380429007cf085969c6bf931212a96633a98551fb`
  - `WORK_LOG.md` (pre-receipt): `5e632778cedec96cd4b3e885b7b2d81c520df5ee6ff89a80a054ba579161f4a7`
  - `docs/product-rebuild-plan.md`: `9f01efc6487892f4115c938f35ec57dea74406652204f6b4c9453acaca14fed7`
  - `docs/rb-03-lifecycle-and-file-completion-handoff.md`: `4096af2d79c9be994e7f46638cee675f6fb442e9f1acb6af5cbd6e7d18b379c7`
  - `evidence/rebuild/parity-matrix.toml`: `eb042c2d061d9286b88b2a8b7378c9434fd42890ffd59ebc1ae24d9675eb8f8c`
  - `evidence/rebuild/rb-02-first-working-application/checkpoint.md`: `0fcd55591673aeafda28831cf4b2a1377cf465a9726ceb04fcc2088bd2c57a6b`
  - `crates/modeler3d-core/src/project.rs`: `e666f43b618369222293a9a4a8a2902241e39e25736b82f6bb03bc6d09637767`
  - `crates/modeler3d-app/src/commands.rs`: `4ab86197cce17cbc44f112e95b4e919303e679400f35859dff9316a794c91a8a`
  - `crates/modeler3d-app/src/shell.rs`: `b3ddba7f867dcbdaef845657e311afe06842a271e0a39958392c9712ab5124e3`
- Exact next subtask: complete `.3dp` lifecycle/safety and its native Save,
  Discard, Cancel and picker flows before moving to model imports/units. The
  RB-03 audit checkpoint is in the same evidence directory.
- No code checks or tests were run in this documentation-only audit. Nothing
  was staged or committed.

### RB-03 project lifecycle and `.3dp` safety -- 2026-10-31

- Completed the bounded lifecycle/project patch: Windows atomic replacement,
  `.3dp` path validation and extension handling, native project pickers, dirty
  New/Open/Exit Save-Discard-Cancel guards, title/path/dirty updates, and failure
  preservation. A pending Open now completes only after Save/Save As succeeds.
- Added Python-reference current, legacy, future, invalid-transform, missing-
  mesh, and malformed-JSON fixtures. The golden tests assert persisted project
  and object fields, geometry, schema migration/rejection, validation errors,
  and exact bytes for an untouched current Python archive.
- Verification passed: `cargo fmt --all -- --check`; `cargo test --offline
  --locked -p modeler3d-core -p modeler3d-app` (74 app and 81 core tests);
  strict changed-crate Clippy with `-D warnings`; and
  `cargo build --offline --locked -p modeler3d-app --bin modeler3d-app`.
- The visible Rust app was exercised using an isolated `LOCALAPPDATA` profile
  at 1280 x 840 client pixels, 96 DPI / 100% scale. New, Save As, Save, Open,
  and dirty New/Open/Exit Save-Discard-Cancel paths were exercised, including
  native picker selection/cancel, overwrite No, corrupt Open, failed state
  preservation, and close Discard. Pending Open + Save As completed and then
  loaded the selected two-object project. The saved intermediate project was
  validated by the Python reference reader; temporary project output was
  removed after validation. Native captures are in
  `evidence/rebuild/rb-03-lifecycle-and-file-completion/captures/`.
- The ledger and requirement map now record the lifecycle/project pass. The
  checkpoint records the exact next bounded patch: imports and unit handling.
  Paired Python/Rust Light/Dark comparison and owner review remain pending until
  the integrated RB-03 evidence pass.
- Additive lifecycle snapshot planned on `G:` after this receipt is written;
  append its full path/file/size/SHA-256 verification and recovery-critical
  hashes after the copy. No files were staged or committed; `L:\Builder3D`
  remains read-only.
### RB-03 lifecycle snapshot receipt -- appended after copy

- Immutable additive snapshot: `G:\Builder3D-Rust_snapshot_20261031-123718_RB-03-lifecycle`.
- Verified at 2026-10-31T12:37:21-06:00. Robocopy exit code 1 means files
  were copied without failure. Source and snapshot each contain 665 files and
  108,408,854 bytes.
- Relative-path differences: 0. File-size and SHA-256 differences across
  every included file: 0. Reparse entries: 0 in both source and snapshot.
- Excluded only repository-root `target/`, repository-root `.cache/`, and
  reparse-point entries. Robocopy `/XJ` was used. The snapshot was new and no
  existing G: snapshot was modified.
- Recovery-critical SHA-256 values at copy time (all matched):
  - `AGENTS.md`: `94274d03f67d37478ca78e06f1f077002d25198fbe84fb246ca8d27097650714`
  - `Cargo.toml`: `e979ca898876a5abf5770b4ae55c7c6e4ecaa68d8bb6d695d896c87cf8d75cf2`
  - `Cargo.lock`: `0286e32ea60066b420f2b3c103aa66844492a6aafb4465d479974e3b0444fd2b`
  - `WORK_LOG.md` (pre-receipt): `9e09b465a8a1e72957b48da4b98dd91c017373ee56f576079a1284be84086b75`
  - `.git/HEAD`: `28d25bf82af4c0e2b72f50959b2beb859e3e60b9630a5e8c603dad4ddb2b6e80`
  - `docs/rb-03-lifecycle-and-file-completion-handoff.md`: `4096af2d79c9be994e7f46638cee675f6fb442e9f1acb6af5cbd6e7d18b379c7`
  - `evidence/rebuild/parity-matrix.toml`: `3a86c1abd6aea5b459fc683392ea249ac119e21fd6dc7d7366a2bb8b88aee690`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/checkpoint.md` (pre-receipt): `ca72caf2909f6791d79b537deba87ab9f2db52626853a06c41ad729667a25f35`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/requirement-test-map.md`: `be51b488d64040aaa1f5d450f11e400acfef7749d6c2a38c167cdf7fd97bf1a8`
  - `crates/modeler3d-core/src/project.rs`: `cbac8d8d250d1532891fc6e170a34e4cd519fcbe2c8d58e8ae66a25a43eddff7`
  - `crates/modeler3d-core/tests/project_golden.rs`: `28577b83dbe38767c3bd2722bc13a74932335d25b593ef5a1b4cd90b22752dea`
  - `crates/modeler3d-app/src/commands.rs`: `c07a87e48e1ca2da7bdbe5775060f561fae6269b34890c7ef638fa2898f5e2c4`
  - `crates/modeler3d-app/src/dialogs.rs`: `431c7d14a9545dc8b077a117ecc5990d2fb3102cf4a6449615e622181d009256`
  - `crates/modeler3d-app/src/shell.rs`: `26bce3ccd8df690265d31edc4efdc96df39f1021824f602e29342621e9253f3d`
- This receipt and the checkpoint update postdate the immutable snapshot. It
  contains the pre-receipt work-log entry and checkpoint; all included files
  matched at copy time.

## 2026-10-31 - RB-03 import and unit implementation

- Added the missing reference import unit, um, in order with mm, cm, in, ft,
  and m. Verified factors are 0.001, 1, 10, 25.4, 304.8, and 1000 mm. The
  persisted preference defaults to mm and accepts um, micro sign, and Greek mu
  spellings while preserving the typed settings schema.
- Added the accurate import dialog hint that 3MF, glTF, and GLB carry their
  own units. Existing import loading keeps STL/OBJ/PLY on the chosen/default
  unit and leaves authoritative 3MF/glTF/GLB loader conversion unchanged.
- Generated Python reference fixtures under
  crates/modeler3d-core/tests/fixtures/imports/: two-object glTF and GLB with
  sidecars, and a two-object 3MF declared in centimetres. Focused calls to the
  read-only Python reference loaders verified object names, colors, extents,
  transforms, and declared-unit restoration.
- Added app tests for each supported format, colored OBJ/PLY, all six STL
  conversion factors and actual dimensions, millimetre default behavior,
  authoritative-unit override rejection, multi-object names/colors/transforms,
  combined placement with world minimum Y=0, selection, one grouped undo,
  successful recent recording, and malformed/unsupported import preservation
  of scene, selection, path, dirty state, history, and recents.
- Verification passed: cargo fmt --all -- --check; cargo test --offline --locked
  -p modeler3d-app (80 tests); cargo clippy --offline --locked -p modeler3d-app
  --all-targets --all-features -- -D warnings; cargo build --offline --locked
  -p modeler3d-app --bin modeler3d-app; and parity TOML parse/unique-ID check
  (129 feature IDs).
- Native attempt: the app process/window was present and PrintWindow captured
  its 1280 x 840 client. The current session returned no foreground window,
  SetForegroundWindow returned false, and click attempts did not change the
  window. The visible import journey, mm/in dimensions, and import picker
  cancellation are not claimed as passed. The previous lifecycle walkthrough
  remains valid and is documented above.
- Planned additive recovery snapshot:
  G:\Builder3D-Rust_snapshot_20261031-125541_RB-03-import-unit. It includes
  Git metadata, source, fixtures, docs and evidence; excludes repository-root
  target, repository-root .cache, and reparse entries. Append the exact copy
  and source/snapshot SHA-256 comparison receipt after verification.
### RB-03 import/unit recovery snapshot receipt -- appended after copy

- Immutable additive snapshot: G:\Builder3D-Rust_snapshot_20261031-125541_RB-03-import-unit.
- Copied at 2026-10-31 12:56:44 MDT and verified at 2026-10-31T12:57:27-06:00. Robocopy exit code 1 indicates copied files without failure.
- Source and snapshot each contain 673 files and 108,448,668 bytes. Relative-path differences: 0. File-size differences: 0. SHA-256 differences across every included file: 0. Reparse entries: 0 in both source and snapshot.
- Excluded repository-root target/, repository-root .cache/, and reparse-point entries. Robocopy /XJ was used. The destination was new; no existing G: snapshot was modified.
- Recovery-critical SHA-256 values at copy time (all matched):
  - .git/HEAD: 28d25bf82af4c0e2b72f50959b2beb859e3e60b9630a5e8c603dad4ddb2b6e80
  - AGENTS.md: 94274d03f67d37478ca78e06f1f077002d25198fbe84fb246ca8d27097650714
  - Cargo.toml: e979ca898876a5abf5770b4ae55c7c6e4ecaa68d8bb6d695d896c87cf8d75cf2
  - Cargo.lock: 0286e32ea60066b420f2b3c103aa66844492a6aafb4465d479974e3b0444fd2b
  - WORK_LOG.md (pre-receipt): cdb1c04134f0db8527674e545b44fc690ddebc1194660a9842868435ee9f75c8
  - docs/rb-03-lifecycle-and-file-completion-handoff.md: 4096af2d79c9be994e7f46638cee675f6fb442e9f1acb6af5cbd6e7d18b379c7
  - evidence/rebuild/parity-matrix.toml: b05806802324f21eac43f3f2e869284cb0a069767be3e5061e57fac7732af392
  - evidence/rebuild/rb-03-lifecycle-and-file-completion/checkpoint.md (pre-receipt): 4b4a3d13c46aadc4f8bc9000927e3f23c1c4b65ad1d4ff5ed62286d0c122f189
  - evidence/rebuild/rb-03-lifecycle-and-file-completion/requirement-test-map.md: 08d8b0e3fe6253f173445c7e98a129c4c0375f710c2bbc589ca8ffcee633ddf8
  - crates/modeler3d-app/src/commands.rs: f6f314e9f6d7462a921bab6674697d760b2e5b3b9f19bd527cab68397ac4b421
  - crates/modeler3d-app/src/preferences.rs: bb98c527b49133663dfa86d16d53cc366ce9d346e311144734f1c804669ad5
  - crates/modeler3d-app/src/dialogs.rs: a81e26a4a745a7e8f728b2b21b1bf4ede3543d4412c68cd71d28fb160791ad8b
  - crates/modeler3d-core/tests/fixtures/imports/README.md: c28fbda96eb10e000724156bda2b58d67c95f62488c59eaa87a5c25bfaa7abeb
  - crates/modeler3d-core/tests/fixtures/imports/python_multi_object.gltf: 3258784e59dddb07630ca01b26df1c4222dc9e5c7d3310e5f2fc7b9104b58fbb
  - crates/modeler3d-core/tests/fixtures/imports/python_multi_object.glb: 688e0f61b597c7badd8f2d622e6cbcc0fc49862f03d3fd9c3221e2d302dc9bbb
  - crates/modeler3d-core/tests/fixtures/imports/python_multi_object_cm.3mf: f5c8ea614505befb0c29b8b0e266e74b9bfb503cf097a0eb4ad7f1c0b77b2563
- This receipt and the updated checkpoint postdate the immutable snapshot. The snapshot contains the pre-receipt work log and checkpoint; all included files matched at copy time.

## 2026-10-31 - RB-03 export implementation and verification

- Completed all seven app export routes: STL, OBJ, PLY, 3MF, glTF, GLB, and
  WRL. Merged OBJ and PLY exports now receive one RGBA value per mesh vertex
  from each object's scene color. STL remains colorless; OBJ stores RGB and
  PLY stores RGBA. Multi-object 3MF/glTF/GLB/WRL retains separate names,
  geometry, transforms, and supported display colors.
- Added app tests with two differently sized, colored, translated box objects.
  The tests reopen every output, check world bounds, object names/local extents/
  transforms/colors, and lossy color behavior. They cover no selection, one and
  multiple selected objects, hidden selection, mixed visibility, and no visible
  objects. Export command success and failure preserve scene objects,
  selection, project path, dirty state, history, and recent paths.
- Added an app overwrite confirmation for typed Save As and Export paths. The
  Cancel tests verify existing `.3dp` and `.ply` destination bytes remain
  unchanged and that current scene state remains intact. Native RFD Save dialogs
  keep their OS overwrite confirmation. Extensionless export names default to
  `.stl`; explicit supported extensions retain their format.
- Added `crates/modeler3d-app/examples/rb03_export_fixture.rs` to reproduce a
  two-object fixture under `target/rb03-export-fixture`. Python 3.11.9 from the
  reference virtual environment reopened STL/OBJ/PLY/3MF/glTF/GLB through the
  read-only `L:\Builder3D` loaders. Geometry, names, dimensions, transforms,
  world bounds, and supported colors passed. The Python glTF reader returns the
  two nodes in a different traversal order; the check matches them by name.
- Added `verify_wrl_subset.py`, an independent standard-library VRML 2.0
  subset reader. It read the actual app-produced WRL and verified the header,
  two named Transform/Shape/IndexedFaceSet objects, local vertices and triangle
  indices, materials/transparency, transforms, and combined world bounds. Its
  limitation is documented: it reads the subset emitted by this writer, not
  arbitrary VRML documents.
- Verification passed: `cargo fmt --all -- --check`; `cargo test --offline
  --locked -p modeler3d-app` (84 tests); `cargo clippy --offline --locked
  -p modeler3d-app --all-targets --all-features -- -D warnings`; the fixture
  generator; the Python reference-reader script; and the independent WRL
  subset-reader script. The default-target app build could not replace the
  executable because the earlier walkthrough window still holds it open; the
  app build passed to `target/rb03-export-build` without closing that window.
  The target export artifacts are reproducible and excluded from recovery
  snapshots.
- The active Windows session still does not expose a usable foreground input
  window. Native export picker selection/cancellation, the seven-format visible
  journey, and paired Light/Dark review remain pending; they are not claimed as
  passed.
- Planned additive recovery snapshot:
  `G:\Builder3D-Rust_snapshot_20261031-<time>_RB-03-exports`. It will include
  Git metadata, source, tests, docs, and evidence while excluding repository-
  root `target/`, `.cache/`, and reparse entries. Append the exact copy and
  source/snapshot comparison receipt after verification.
### RB-03 export recovery snapshot receipt

- Immutable additive snapshot: G:\Builder3D-Rust_snapshot_20261031-132239_RB-03-exports.
- Robocopy copied at 2026-10-31 13:22:48 MDT with exit code 1 (files copied, no failures). Verified at 2026-10-31T13:24:12-06:00.
- Source and snapshot each contain 676 included files and 108,512,354 bytes. Relative-path differences: 0. File-size differences: 0. Per-file SHA-256 differences: 0. Reparse entries: 0 in both.
- Excluded repository-root target/, repository-root .cache/, and reparse-point entries. Robocopy /XJ was used. The destination was new; no existing G: snapshot was modified.
- Recovery-critical SHA-256 values at copy time (source/snapshot matched):
  - .git/HEAD: 28d25bf82af4c0e2b72f50959b2beb859e3e60b9630a5e8c603dad4ddb2b6e80
  - AGENTS.md: 94274d03f67d37478ca78e06f1f077002d25198fbe84fb246ca8d27097650714
  - Cargo.toml: e979ca898876a5abf5770b4ae55c7c6e4ecaa68d8bb6d695d896c87cf8d75cf2
  - Cargo.lock: 0286e32ea60066b420f2b3c103aa66844492a6aafb4465d479974e3b0444fd2b
  - WORK_LOG.md (pre-receipt): f75150475e370aafadb40f39a1a5327c279c6f9bc8b6fb574fde03f16a84cc56
  - docs/rb-03-lifecycle-and-file-completion-handoff.md: 4096af2d79c9be994e7f46638cee675f6fb442e9f1acb6af5cbd6e7d18b379c7
  - evidence/rebuild/parity-matrix.toml: fb7fd2a265579c58187d73ab8eb97cc8d14bd3dca8a25b17a98ae59e29420a64
  - evidence/rebuild/rb-03-lifecycle-and-file-completion/checkpoint.md (pre-receipt): 73a208a802968f172538458a71f7fd52012f1d9c7404cb44fe0e9be903577df9
  - evidence/rebuild/rb-03-lifecycle-and-file-completion/requirement-test-map.md: ff78cc5fa799ff4cfdcb7b36cc3915ce98792a02669316d061c177713878442e
  - crates/modeler3d-app/src/commands.rs: 5d8f11e5a416315f787ee7c425f592cca94e1aae16e36d9688569f713a2870ad
  - crates/modeler3d-app/src/dialogs.rs: bd019a0d41dacc068c4a6b103c327e92fa37d6660f89b014728a22770289eac6
  - crates/modeler3d-app/src/shell.rs: 0f13528d7cd2a58702f1e041b575ad4c2a3aab2b72bfcd4faa82d782b8a17905
  - crates/modeler3d-app/src/export.rs: 8344a98015964432c840f5c3bd26696744ed59986ea250033f6b7f7c647ac776
  - crates/modeler3d-app/examples/rb03_export_fixture.rs: 355d76a355d147e026ece27c8297f32207c050acd9391a4b2c265f1c3b335bda
  - evidence/rebuild/rb-03-lifecycle-and-file-completion/verify_python_reference_exports.py: 6a4843c1453c0999f566adcacc69e4c81add1d05f88affdccc53050bae7712d0
  - evidence/rebuild/rb-03-lifecycle-and-file-completion/verify_wrl_subset.py: 17a80aea260d8eb617bb80faec64c3829b4344dddf66a83ac5d5db083cb1fd34
- This receipt and the updated checkpoint postdate the immutable snapshot. They record its pre-receipt state and do not modify the snapshot.
## 2026-10-31 - RB-03 recent-file dispatch and previews

- Added bounded newest-first recent behavior coverage for ten-entry persistence and settings reload. Exact duplicate paths move to the front, successful project/model paths are recorded, and persisted paths are normalized on load.
- Recent .3dp entries dispatch through validated project Open; model entries dispatch through the additive import command. Missing paths are removed and reported. Corrupt project and WRL import failures preserve the live scene, project path, dirty flag, history, and recent list. WRL remains a reference recent entry, but importing it still reports unsupported.
- Added crates/modeler3d-app/src/recent_preview.rs and welcome texture caching. Image and mesh preview decoding runs on one background worker; generation checks discard stale queued results. Visible .3dp scenes and STL/OBJ/PLY/3MF/glTF/GLB imports render bounded 64 x 64 isometric thumbnails. Raster recents and embedded 3MF image thumbnails use the existing 64 MiB / 16 Mi-pixel bounds. Hidden-only projects and missing, unsupported, corrupt, or undecodable entries keep the file-type icon.
- Added shell/prefs tests for restart persistence, project/model dispatch, missing removal, corrupt project and WRL preservation; preview tests cover visible/hidden project data, all six import formats, generated raster/embedded-3MF PNGs, and fallback states.
- The first focused test run failed because the selected Python current-project fixture intentionally marks its only object hidden. That matches the reference preview filter. The test now verifies that hidden-only fallback and generates a visible .3dp fixture for the preview-success case.
- Final focused verification passed: cargo fmt --all -- --check; cargo test --offline --locked -p modeler3d-app (90 tests); cargo clippy --offline --locked -p modeler3d-app --all-targets --all-features -- -D warnings. The parity and component catalogs parsed with 139 unique feature IDs and 31 unique component IDs, and recent-row evidence/source paths exist.
- Native restart persistence, recent dispatch/removal/error interactions, visible tile previews, and paired Light/Dark comparison are not claimed as passed. The active Windows session has no usable foreground input window. The prior lifecycle native walkthrough remains valid; import/export and recent UI walkthroughs are pending.
- Planned additive recovery snapshot: G:\Builder3D-Rust_snapshot_20261031-<time>_RB-03-recents. It includes Git metadata, code, tests, docs, fixtures, and evidence, excluding repository-root target/, repository-root .cache/, and reparse entries. Append copy and source/snapshot comparison receipt after verification.
### RB-03 recent/preview recovery snapshot receipt

- Immutable additive snapshot: G:\Builder3D-Rust_snapshot_20261031-133824_RB-03-recents.
- Robocopy copied at 2026-10-31 13:38:24 MDT with exit code 1 (files copied, no failures). Verified at 2026-10-31T13:38:40-06:00.
- Source and snapshot each contain 677 included files and 108,559,570 bytes. Relative-path differences: 0. File-size differences: 0. Per-file SHA-256 differences: 0. Reparse entries: 0 in both.
- Excluded repository-root target/, repository-root .cache/, and reparse-point entries. Robocopy /XJ was used. The destination was new; no existing G: snapshot was modified.
- Recovery-critical SHA-256 values at copy time (source/snapshot matched):
  - .git/HEAD: 28d25bf82af4c0e2b72f50959b2beb859e3e60b9630a5e8c603dad4ddb2b6e80
  - AGENTS.md: 94274d03f67d37478ca78e06f1f077002d25198fbe84fb246ca8d27097650714
  - Cargo.toml: e979ca898876a5abf5770b4ae55c7c6e4ecaa68d8bb6d695d896c87cf8d75cf2
  - Cargo.lock: 0286e32ea60066b420f2b3c103aa66844492a6aafb4465d479974e3b0444fd2b
  - WORK_LOG.md (pre-receipt): 872c10f8b1f3f429a90d79bdad0633114f5ee29017f110798cac5f57c5e7f9b5
  - docs/rb-03-lifecycle-and-file-completion-handoff.md: 4096af2d79c9be994e7f46638cee675f6fb442e9f1acb6af5cbd6e7d18b379c7
  - docs/component-catalog.toml: d75b358a11dcfba93389a9d6b487b592ff999e2d4d3fa16a692bbdfdb034d5c5
  - evidence/rebuild/parity-matrix.toml: 103fa1116fbe7de85320dad1e2bedf04c743691456cc1879de9e1b09e3bd729e
  - evidence/rebuild/rb-03-lifecycle-and-file-completion/checkpoint.md (pre-receipt): e420104260981a9f79b9ee28bc15a40753ecdc670e64ddbeaaa93b832ae1c489
  - evidence/rebuild/rb-03-lifecycle-and-file-completion/requirement-test-map.md: adbdf38b4995662b76ff3b9129d6a34af1284fbffb813ca0c11ef7c8faec1b2e
  - crates/modeler3d-app/src/lib.rs: a15587e5e4768e46202e3748a296547b2bcc817375aedbfbb9078649ef57a5ad
  - crates/modeler3d-app/src/commands.rs: 5d8f11e5a416315f787ee7c425f592cca94e1aae16e36d9688569f713a2870ad
  - crates/modeler3d-app/src/preferences.rs: 05c69efda0d7213b5894b7d9f9199dee97239eaefd786f7005389ff0d9361bc6
  - crates/modeler3d-app/src/ui_state.rs: 56e637ea90725a5f749f8c836e53b2c3059bb45cd36e23e45c2979aff77e9096
  - crates/modeler3d-app/src/shell.rs: 2a2fa172f2b2507a9ab8c6e4b6640bcbd0bd9f6c98feb0556c15cb127a0ba375
  - crates/modeler3d-app/src/recent_preview.rs: bcf3e104d7b7528f48a249a8f6aa7e84396b4c79388aeee701c219c3b57aaf71
- This receipt and the checkpoint update postdate the immutable snapshot. They record its pre-receipt state and do not modify the snapshot.

## 2026-10-31 - RB-03 integrated verification and remaining native evidence

- The complete RB-03 implementation remains in place: project lifecycle and
  atomic `.3dp` handling, all six import units and six importable model
  formats, all seven exports, selection/visibility and failure preservation,
  ten-entry recent dispatch/persistence, and bounded preview/fallback loading.
- Added saved-history checkpoint tracking to the app dirty marker. Undoing to
  the last saved history state clears the marker; redo and edits on new history
  branches set it correctly. The focused regression and all 91 app tests passed.
- Integrated Rust gates passed: `cargo fmt --all -- --check`,
  `cargo check --workspace --locked`, `cargo test --workspace --locked`
  (227 passed, 0 failed), strict workspace Clippy, `cargo deny check`, locked
  metadata generation/JSON parsing, and the release app build.
- The exact default-target smoke command could not replace
  `target/debug/modeler3d-app.exe` because lifecycle window PID 3208 holds it
  open. The smoke passed using `target/rb03-recents-build` and reported
  `objects=2 selected=1 tabs=5`. `cargo deny` passed with a non-fatal duplicate
  `bitflags` lock-entry warning. `cargo audit` is not installed.
- `cargo bloat --release --workspace` returned 0 but warned that
  `--workspace` is unused and analyzed only the default `r0b_mcp` executable.
  Additional bloat scans passed for every workspace binary: `r0b_mcp` 3.6 MiB
  `.text` / 4.1 MiB file, `r4_mcp` 3.8 / 4.5 MiB, `r5_viewport` 4.0 / 4.7
  MiB, and `modeler3d-app` 11.2 / 13.7 MiB.
- Read-only Python reference tests passed: 35 project/3MF/glTF/WRL tests and
  3 focused unitless-import tests. Python 3.12.10 ran with bytecode and pytest
  cache disabled; all test temporary data remained under the Rust workspace's
  ignored `target/`, and `L:\Builder3D` stayed unchanged.
- The updated debug executable used for the recent-preview/savepoint attempt is
  `target/rb03-recents-build/debug/modeler3d-app.exe`, SHA-256
  `9d079c6c81ea7a2b28d48055cb2656167ea8f283c3355b3eddfdc579720acf5e`.
  The window remained visible, but Windows Default Lock Screen owned the
  foreground and `SetForegroundWindow` returned false. No native import,
  export, or recent walkthrough and no direct paired capture are claimed.
- The prior lifecycle walkthrough and its Dark captures remain valid. The
  new report, verification, walkthrough, format/unit table, and capture
  metadata are under
  `evidence/rebuild/rb-03-lifecycle-and-file-completion/`. The parity ledger
  records implementation/automation as pass while import/export/recent
  interaction, paired Light/Dark visual comparison, and owner acceptance stay
  pending.
- The latest immutable source snapshot remains
  `G:\Builder3D-Rust_snapshot_20261031-133824_RB-03-recents`. No final RB-03
  snapshot has been made because required native and owner evidence remains.
  Resume with the visible unit/import/edit/export/recent walkthrough, capture
  Python and Rust sequentially in Light and Dark, obtain owner review, and
  snapshot the finished RB-03 evidence. Then stop at RB-04 Scene and precise
  transforms; no RB-04 implementation has started.

## 2026-10-31 - RB-03 saved-history dirty-marker checkpoint

- Fixed the dirty marker to compare the current stable scene history ID with
  the last successfully saved history ID. Undo to the saved checkpoint clears
  the marker; redo and a new branch set it correctly. The regression test
  covers all three transitions.
- Focused checks passed: rustfmt, the new regression, all 91 app tests, and
  strict app Clippy. Integrated checks passed: workspace check and tests (227
  passed), strict workspace Clippy, cargo deny, locked metadata, release app
  build, and isolated-target smoke. Thirty-five Python reference tests and
  three focused unitless-import tests passed with the reference tree read-only.
- TOML validation passed for the parity matrix, component catalog, and RB-03
  capture metadata. The matrix has 140 unique feature IDs and 32 rows assigned
  to RB-03; all 32 have implementation and automated status pass. Interactive
  status is 6 pass, 23 pending, and 3 partial; visual and owner review remain
  pending for all 32 rows. RB-03 audit evidence links resolve.
- Native import/export/recent walkthrough and paired visual review remain
  blocked while Windows Default Lock Screen owns the desktop. No RB-03
  completion or final snapshot is claimed. No RB-04 work has started.
- Planned immutable interim snapshot:
  G:\Builder3D-Rust_snapshot_20261031-143631_RB-03-saved-marker. Include Git
  metadata, source, manifests, tests, docs, and evidence; exclude repository
  root target/, .cache/, and reparse-point entries. Append the exact copy and
  full source/snapshot verification receipt after the copy.
### RB-03 saved-history dirty-marker interim snapshot receipt

- Immutable additive snapshot: G:\Builder3D-Rust_snapshot_20261031-143631_RB-03-saved-marker.
- Robocopy completed at 2026-10-31 14:38:00 MDT per the log; exit code 1 (713 files copied; 0 failures, mismatches, or extras).
  Source/snapshot verification completed at 2026-10-31 14:39:03 -06:00.
- Source and snapshot each contain 713 included files and 109,050,037 bytes.
  Relative-path differences: 0; file-size differences: 0; per-file SHA-256
  differences: 0. Reparse entries: 0 in source and snapshot.
- Excluded repository-root target/, repository-root .cache/, and reparse-point
  entries; Robocopy /XJ was used. The destination was new and remains immutable.
- Recovery-critical SHA-256 values at copy time (all matched):
  - .git/HEAD: 28d25bf82af4c0e2b72f50959b2beb859e3e60b9630a5e8c603dad4ddb2b6e80
  - AGENTS.md: 94274d03f67d37478ca78e06f1f077002d25198fbe84fb246ca8d27097650714
  - Cargo.toml: e979ca898876a5abf5770b4ae55c7c6e4ecaa68d8bb6d695d896c87cf8d75cf2
  - Cargo.lock: 0286e32ea60066b420f2b3c103aa66844492a6aafb4465d479974e3b0444fd2b
  - WORK_LOG.md (pre-receipt): 22b4614351768e71beb502ec3fb657e1510fc47dd5e5cec7b9370cf82fe3dd8c
  - crates/modeler3d-app/src/commands.rs: 26c4cd4b23091c2f32f7e8ba508f326a4cfc8f9bf08e91dc5020adf472af6499
  - evidence/rebuild/parity-matrix.toml: e5ec7335ba454e5023f63c4b9e0d42bb8b4b70d1b6ea533fc0b5d8d2d894ba6d
  - evidence/rebuild/rb-03-lifecycle-and-file-completion/checkpoint.md (pre-receipt): 93170e9976f8f86ee1f85a997935bc6224b2304351e135641c8feed44dfe9205
  - evidence/rebuild/rb-03-lifecycle-and-file-completion/verification.md: 2144d688a636941792c9fa325f1f79908597e332737bff29da1da02d0ea3d7f3
- The first verification script hit unsupported Convert.ToHexString on Windows PowerShell; a read-only rerun using BitConverter completed the full per-file SHA-256 comparison successfully.
- This receipt and the checkpoint entry below postdate the immutable snapshot.
  The snapshot contains the planned pre-receipt state. It is an interim patch
  recovery point, not the final RB-03 snapshot; native import/export/recent
  walkthroughs, paired Light/Dark review, and owner acceptance are pending.

- A post-receipt source/snapshot delta check found exactly two changed files:
  WORK_LOG.md and this checkpoint. No included file was missing or extra; those
  two differences are the post-copy receipts. The latest read-only foreground
  query returned no window handle/title (PID 0); no interaction or capture was
  attempted, and native acceptance remains pending.

## RB-03 reference walkthrough, visual comparison, and integrated completion record — 2026-10-31

- Responded to the owner's direction to compare how the Python application looked while each action was being performed. Captured Python and Rust sequentially at 1280 x 840 client pixels, 96 DPI / 100% scale; created 26 side-by-side pairs and a clickable full-size board at `evidence/rebuild/rb-03-lifecycle-and-file-completion/visual-comparison.html`. Python is the left image in every pair. Each pair includes the operation and an observed-difference note.
- The old Python UI was visibly operated for New/Save As/dirty prompts, project save/open, Add/Open, unitless imports in mm/in, 3MF/glTF/GLB import, picker cancel, malformed input, the shared Export Mesh picker, successful no-selection 3MF, overwrite prompt/cancel, recent dispatch, and missing/corrupt/WRL recent errors. The old app's native picker/message-box appearance and two-column wireframe recent tiles are recorded alongside Rust's in-app dialogs and compact recent cards.
- A second Python export-picker replay for each format was blocked when Windows locked the desktop; `GetForegroundWindow` returned 0. No input or capture was claimed while locked. Only the Python process started for the failed attempt was stopped. The original Python QSettings were restored (Light theme; no recent-files or default-import-unit key), and its Git status matches the 116-line pre-capture status baseline. The Python source remains unchanged.
- Rust native import/unit and seven-format export journeys, selection/visibility cases, cancellation/failure handling, restart-persisted recents, previews, and recent failures were exercised. Independent verification passed for all seven output formats. The native visible saved-checkpoint undo/redo/new-branch and complete filesystem-error sequences remain partial; their focused automated coverage passes.
- Latest integrated gates: `cargo fmt --all -- --check`, workspace check, 228 workspace tests, strict workspace Clippy, `cargo deny check`, locked metadata (five packages), isolated smoke (`objects=2 selected=1 tabs=5`), release build, and both prescribed bloat commands passed. `cargo deny` logged a duplicate `bitflags` warning. `cargo bloat --release --workspace` ignored the `--workspace` argument and analyzed `r4_mcp.exe`; app-specific bloat measured 3D Modeler at 11.2 MiB `.text` / 13.7 MiB file. `cargo audit --locked` is unavailable (`no such command`). Exact logs are under the RB-03 evidence folder.
- `evidence/rebuild/parity-matrix.toml` now keeps implementation/automation, native interaction, visual review, and owner acceptance distinct for 32 RB-03 rows. Visual and owner states remain pending until owner review; no visual pass is claimed. The review report and checkpoint are updated, and no RB-04 implementation started.
- Planned additive review snapshot: `G:\Builder3D-Rust_snapshot_20261031-165222_RB-03-review`. It will include Git metadata, source, manifests, tests, docs, logs, and captures; exclude repository-root `target/`, repository-root `.cache/`, and reparse entries. Append exact copy/verification receipts after copying.

### RB-03 additive review snapshot receipt

- Immutable additive review snapshot: G:\Builder3D-Rust_snapshot_20261031-165222_RB-03-review.
- Robocopy completed at 2026-10-31 16:52:25 -06:00; exit code 1. The destination was new; the full verification walk found no failed or extra files.
- Source and snapshot each contain 901 included files and 139,803,911 bytes. Relative-path differences: 0; file-size differences: 0; per-file SHA-256 differences: 0. Reparse entries: 0 in source and snapshot.
- Excluded repository-root target/ (present), repository-root .cache/ (absent; both paths were explicitly supplied to Robocopy /XD), and reparse-point entries (/XJ). Git metadata, source, manifests, tests, docs, logs, and captures are included.
- Recovery-critical SHA-256 values at copy time (all matched):
  - .git/HEAD: 28d25bf82af4c0e2b72f50959b2beb859e3e60b9630a5e8c603dad4ddb2b6e80
  - AGENTS.md: 94274d03f67d37478ca78e06f1f077002d25198fbe84fb246ca8d27097650714
  - Cargo.toml: e979ca898876a5abf5770b4ae55c7c6e4ecaa68d8bb6d695d896c87cf8d75cf2
  - Cargo.lock: 0286e32ea60066b420f2b3c103aa66844492a6aafb4465d479974e3b0444fd2b
  - WORK_LOG.md (pre-receipt): 697e6051e45aff3d255b66a667ab25b0d456793d5a832d741cb5e957c1cae077
  - crates/modeler3d-app/src/commands.rs: 26c4cd4b23091c2f32f7e8ba508f326a4cfc8f9bf08e91dc5020adf472af6499
  - evidence/rebuild/parity-matrix.toml: b8d4812cefafea80a9c1dca4ec4fcb6099541c828c6735bd259743e04f11ccff
  - evidence/rebuild/rb-03-lifecycle-and-file-completion/checkpoint.md (pre-receipt): c689b11b3e3db3ec1620969410a2f035ef0cda45d2d2df730465225e2ecafa66
  - evidence/rebuild/rb-03-lifecycle-and-file-completion/verification.md: f7b7027404f2299b690adeabf78497f42afeeb2ea0874ac279b05eca440266d5
  - evidence/rebuild/rb-03-lifecycle-and-file-completion/visual-comparison.html: b5402f25d97f62746bb9447d220dd815057a01ce3a6efff7d5419d7b358c6742
- This receipt postdates the immutable snapshot; the snapshot contains the pre-receipt versions of this log and the checkpoint. A post-receipt source/snapshot delta check follows.
- Final post-receipt tree comparison: no included relative paths are missing or extra. Exactly `WORK_LOG.md` and `evidence/rebuild/rb-03-lifecycle-and-file-completion/checkpoint.md` differ in size and SHA-256 because their receipts were appended after the copy; no other included file differs. Reparse entries remain 0 in both trees.

## 2026-10-31 - RB-04 continuation handoff

- Prepared docs/rb-04-through-rb-11-completion-handoff.md for a fresh session. It captures the RB-03 evidence state, defines the complete next RB-04 scope, emphasizes old-Python/Rust visual comparison during each operation, and maps the remaining RB-05 through RB-11 completion route.
- Updated AGENTS.md, docs/product-rebuild-plan.md, and the RB-03 handoff/checkpoint/report to point to RB-04. RB-03 owner visual review and two partial native-interaction rows remain pending; the current owner direction allows RB-04 work to proceed without treating those pending fields as a stop.
- Documentation-only change: no Rust source or Python files changed, no tests run, and nothing staged or committed. The existing Rust files remain untracked.
- Exact next fresh-session task: implement and verify all of RB-04, create its evidence and additive recovery snapshot, then stop before RB-05.
- Planned additive documentation recovery snapshot: G:\Builder3D-Rust_snapshot_20261031-193357_RB-04-handoff. Include repository and Git metadata; explicitly exclude repository-root target/, repository-root .cache/, and reparse entries; append the exact copy and verification receipt after copying.

### RB-04 continuation handoff snapshot receipt

- Immutable additive documentation snapshot: `G:\Builder3D-Rust_snapshot_20261031-193357_RB-04-handoff`.
- Robocopy completed at 2026-10-31 19:35:01 -06:00; exit code 1.
- Source and snapshot each contain 902 included files and 139,824,114 bytes. Relative-path differences: 0; size differences: 0; per-file SHA-256 differences: 0. Reparse entries: 0 in both trees.
- Repository-root `target/` was present and excluded; `.cache/` was absent and explicitly excluded; reparse entries were excluded with `/XJ`. The snapshot includes Git metadata, source, tests, documentation, and evidence.
- Recovery-critical SHA-256 values at copy time (all matched):
  - `.git/HEAD`: 28d25bf82af4c0e2b72f50959b2beb859e3e60b9630a5e8c603dad4ddb2b6e80
  - `AGENTS.md`: 79ca023e8ef8cc3d49700956dd25643d79ae57958567f221e8f0278908078d2c
  - `Cargo.toml`: e979ca898876a5abf5770b4ae55c7c6e4ecaa68d8bb6d695d896c87cf8d75cf2
  - `Cargo.lock`: 0286e32ea60066b420f2b3c103aa66844492a6aafb4465d479974e3b0444fd2b
  - `WORK_LOG.md` (pre-receipt): e4811689a43e6a920167d6d0b29328b127778fe79b010b29d457e41db588875d
  - `docs/rb-04-through-rb-11-completion-handoff.md`: 085560648cd28636fb157cf973797baad50bf4488530cf27e7fec151df78ae3e
  - `docs/product-rebuild-plan.md`: f57f47ca4401fc3da27df0a25307df519629f1c6443a7e2d8e8db0e7ff364a75
  - `evidence/rebuild/parity-matrix.toml`: b8d4812cefafea80a9c1dca4ec4fcb6099541c828c6735bd259743e04f11ccff
  - RB-03 checkpoint: e7b4cfbe204a37bf4d878ec097b159d5e9e89a66a2464b79216b541db25964cf
  - RB-03 report: 51dfd4480ef2822ec072a7519c82b0af3848dfe82fff3d6aed63b8203c900286
- This receipt postdates the immutable snapshot. The snapshot contains the pre-receipt `WORK_LOG.md`; final post-receipt comparison follows.
- Post-receipt comparison at 2026-10-31 19:36 -06:00, before recording this line: each tree had 902 included paths; missing and extra paths were 0. Only `WORK_LOG.md` differed (1,769 bytes and SHA-256); reparse entries were 0 in both. The final recheck follows.
## 2026-10-31 — RB-04 scene/numeric groundwork and native comparison checkpoint

- Continued the complete RB-04 objective in the existing untracked tree; Python `L:\Builder3D` stayed read-only. Corrected paste offset, additive/ordered selection and history order, sticky selection, geometry thumbnails, naming, absolute Euler rotation with signed stretch, translation-preserving resize, proportional numeric sizing, numeric-strip access, reference mirror rule, axis/landmark/gap placement with modal cancellation, per-object Dimensions, health-to-Repair reachability and focus-guarded editing shortcuts.
- Focused Python: 11 service + 7 numeric/control + 4 adapter tests passed with bytecode/cache writes disabled and an isolated profile. Before/after/final Python Git status matches. Rust post-native workspace run: 235 passed, 0 failed, 1 explicitly ignored exact group Settle/QHull parity regression. Strict workspace Clippy and fmt passed after the last UI fix. The nine-case absolute-rotation oracle passes. Full verification commands and initial failures/reruns are retained in the RB-04 logs.
- Captured Python and Rust sequentially in Light/Dark at 1280 x 840 / 96 DPI. The current board has 50 pairs and 110 hash-validated images; full-size selected captures and all four contact sheets were inspected. Differences include wireframe tiles versus shaded rows, compact reference strip versus Rust inspector/strip, modal presentation, lighting/grid and selection feedback; accumulated state/build differences are explicit. Final native Dimensions captures cover both themes; final Light unhealthy-item warning opens Repair. The warning glyph currently appears as a missing-symbol box. Some ribbon changes required another pointer event before repaint.
- Updated the parity matrix and component catalog with scoped evidence, retaining pending owner/visual fields. All 108 reference/packaged asset hashes match. RB-03 remains 32 implementation/automated pass, 30 native pass and two native partial, with all visual/owner fields pending. The historical validation script's file::symbol false positive was resolved by the focused validator; no RB-03 status was changed.
- RB-04 is not complete: exact group Settle parity, native Rotate/Resize/object-grid basis, grouped drag/mixed numeric/focus/selection cases and remaining final-build paired evidence still require work. QHull compatibility research is in the checkpoint; no QHull dependency has been installed. Preserve this bounded progress before the next implementation patch, then continue RB-04. RB-05 has not started; nothing staged or committed.
- Baseline recovery snapshot: `G:\Builder3D-Rust_snapshot_20261031-194408_RB-04-baseline`, 902 files / 139,826,146 bytes, zero relative-path/size/SHA-256 differences. The verified additive progress snapshot receipt follows after the running size gates complete. This is not a completed RB-04 or owner-acceptance claim.

Planned additive RB-04 progress recovery snapshot (slice incomplete): `G:\Builder3D-Rust_snapshot_20261031-201620_RB-04-progress`. Root target/.cache and reparse entries excluded; exact receipt follows after verification.


### RB-04 groundwork verified additive progress snapshot

- Immutable snapshot: `G:\Builder3D-Rust_snapshot_20261031-201620_RB-04-progress`. Verified 2026-10-31T20:16:24-06:00; Robocopy exit 1.
- Source and snapshot: 1,092 files / 164,875,562 bytes each; relative-path differences 0; size/SHA-256 differences 0; reparse entries 0 in each tree. Root target/, .cache/ and reparse entries excluded. Source, Git metadata, tests, documentation and evidence included.
- This is an interim RB-04 recovery point, not slice completion. Continue with the QHull parity probe and remaining viewport transforms before RB-05.
- Pre-receipt recovery-critical SHA-256 values:
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/report.md`: `5e41b02096a3f0ffd854a30e8a95c0dbe5ceef82d42355c5b9a6b58ff92c19e2`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/checkpoint.md`: `97585395a82f26874aaace788b2eb1e46b8402d5d8f8acaef78b2ccd58046b92`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/verification.md`: `bd514517f4b7b2b04ec3e1ac95c20df705bf486a80abe85252ec0dbde9a986f1`
  - `crates/modeler3d-app/src/rotation.rs`: `7e93584dd28270382f13565bfaba28b6ea8b6b7789c5209aaff58c6a72d50e06`
  - `crates/modeler3d-app/src/transforms.rs`: `1e1c752d845de82c99b505baa53ff3ee0ef20347f22db7805da8712ad4ca1b73`
  - `AGENTS.md`: `79ca023e8ef8cc3d49700956dd25643d79ae57958567f221e8f0278908078d2c`
  - `Cargo.toml`: `e979ca898876a5abf5770b4ae55c7c6e4ecaa68d8bb6d695d896c87cf8d75cf2`
  - `Cargo.lock`: `0286e32ea60066b420f2b3c103aa66844492a6aafb4465d479974e3b0444fd2b`
  - `WORK_LOG.md`: `dfb7afe47f44b9e876e845a87bca20c2476cbc2126beb9e62f18e79b27ebca43`
  - `.git/HEAD`: `28d25bf82af4c0e2b72f50959b2beb859e3e60b9630a5e8c603dad4ddb2b6e80`
  - `docs/rb-03-lifecycle-and-file-completion-handoff.md`: `5d166c80c6a9a8ab4d3085a9d4ff872cc98efc7b6469ffe0583d48eedb6ff222`
  - `evidence/rebuild/parity-matrix.toml`: `c1a8729cc047e6b51fd60bad83678e9735801c90ccb78b9cfd689ede7e7179eb`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/checkpoint.md`: `e7b4cfbe204a37bf4d878ec097b159d5e9e89a66a2464b79216b541db25964cf`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/requirement-test-map.md`: `e03765801daa74cc500f5948c15065de0e013e7398491c2f18f325cc71c875da`
  - `crates/modeler3d-core/src/project.rs`: `cbac8d8d250d1532891fc6e170a34e4cd519fcbe2c8d58e8ae66a25a43eddff7`
  - `crates/modeler3d-core/tests/project_golden.rs`: `28577b83dbe38767c3bd2722bc13a74932335d25b593ef5a1b4cd90b22752dea`
  - `crates/modeler3d-app/src/commands.rs`: `5a3e1fd6b032879cc55232284e4a522bf60f489d6ee02f7f7d501495dbf75bf1`
  - `crates/modeler3d-app/src/dialogs.rs`: `bd019a0d41dacc068c4a6b103c327e92fa37d6660f89b014728a22770289eac6`
  - `crates/modeler3d-app/src/shell.rs`: `73c5ed71c956f6c947b18e0bf1fdb2251bc3e0da8ac27269ce80a1b8966a571c`

The receipt postdates the immutable snapshot. Only WORK_LOG.md and the RB-04 checkpoint are expected to differ afterward.

Post-receipt comparison: 1092 included paths in each tree; no missing or extra paths; exactly WORK_LOG.md and the RB-04 checkpoint differ from their post-copy receipts; no other size/SHA-256 differences; reparse entries 0 in each.

## 2026-10-31 — RB-04 exact Settle engineering repair

- Repaired and enabled the retained oracle: the first failure was the single-object case, before the group case ran. Python uses post-multiplied object rotation for a single target and world composition for groups. Rust now matches both unchanged expected matrices and both support-normal fixtures. Settle and Measure use scene order; Align/Arrange retain user anchor order.
- Added bounded adapter-neutral placement_rotation in modeler3d-core using pinned qhull-rs revision 62e810ccf93b458470f03cae6ef4df26023d2260. The released wrapper's pointer print and scaled-buffer index assertion were demonstrated only in the disposable probe; the accepted dependency removes that print and uses safe point_id lookup. Resource/nonfinite validation and the core panic boundary preserve the model on failure. No temporary-file capture is used.
- Added a verified Windows build-only libclang helper under scripts/ and docs/building.md. It uses a pinned/hash-checked wheel and DLL under ignored target/, sets only process LIBCLANG_PATH, installs nothing globally, and adds no product Python/libclang runtime dependency. Native QHull and MIT wrapper notices are included unchanged in package staging. Source Git whitelist is narrow and the dependency is revision-pinned.
- Current gates: 239 workspace tests passed, 0 failed, 0 ignored; changed-crate strict Clippy, fmt and deny passed. Logs and exact exit codes are in settle-gate-results.json. The earlier 235-pass/one-ignored checkpoint remains a historical recovery record, not current test status.
- Windows locked before the new Python single/group Settle Light/Dark replay; foreground was zero and no input/capture was accepted. Asked asynchronously for unlock and continued independent code work. Final Settle before/after/undo/redo captures remain pending. RB-03 pending review states and Python source remain unchanged; nothing staged or committed.
- Engineering checkpoint next: grouped identity-safe viewport previews, root/child picking, Rotate/Resize, grid/object basis and fixed opposite face anchor. RB-04 remains incomplete; RB-05 has not begun. Additive Settle-engineering recovery receipt follows.

Planned additive RB-04 progress recovery snapshot (slice incomplete): `G:\Builder3D-Rust_snapshot_20261031-203813_RB-04-settle-engineering`. Root target/.cache and reparse entries excluded; exact receipt follows after verification.


### RB-04 Settle-engineering verified additive snapshot

- Immutable snapshot: `G:\Builder3D-Rust_snapshot_20261031-203813_RB-04-settle-engineering`. Verified 2026-10-31T20:38:17-06:00; Robocopy exit 1.
- Source and snapshot: 1,122 files / 165,302,826 bytes each; relative-path differences 0; size/SHA-256 differences 0; reparse entries 0 in each tree. Root target/, .cache/ and reparse entries excluded. Source, Git metadata, tests, documentation and evidence included.
- This is an interim RB-04 recovery point, not slice completion. Continue with identity-safe grouped previews and Rotate/Resize before RB-05; native Settle replay remains pending while Windows is locked.
- Pre-receipt recovery-critical SHA-256 values:
  - `crates/modeler3d-core/src/placement.rs`: `87c767a0e97adf72e422a340f92cb00c4a6194361128261395b4377e1218b8a6`
  - `scripts/Initialize-BuildClang.ps1`: `73856cc160105f1a9c00bf845b14e29a3002dfdf6ba3efc5aed2742ed43a3d3f`
  - `docs/building.md`: `750a226f595466b2e5fc15f4fb453fe284c4c7f199dd9632e3e90233ec44495a`
  - `packaging/licenses/QHULL-COPYING.txt`: `106d55c931fd6a84822e5345d900273d059f1c27310d02567ccb313c5d18c55d`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/settle-fix.md`: `63a903bbb24d71770f971142e33fa8d063a3ad5155520c91fd442bd4a0c51fc1`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/report.md`: `81ce83acf828fc25070c074d52ed342966a894111bb7a8fc077c85c7e87b4618`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/checkpoint.md`: `67c3792cb34ce5cccc07e39f37436e70ad459e5adc760fcd0569cfe58453d406`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/verification.md`: `ef02ef955e39a3611841a4a34882d99d4a43f4cc7b1a50a255cd7789bc16a74e`
  - `crates/modeler3d-app/src/rotation.rs`: `7e93584dd28270382f13565bfaba28b6ea8b6b7789c5209aaff58c6a72d50e06`
  - `crates/modeler3d-app/src/transforms.rs`: `6fd1bb2e1a6fb725921c536a4836b290626fd0f9db871f26266cda718f49a1c8`
  - `AGENTS.md`: `986ba68a9e458b5c5bc9a7c9122e4a741d3dfc5f35228b431a2bc974c7f45a1b`
  - `Cargo.toml`: `d51c5bfb69d2e0cdf88fc252d58baafc54c99a926bef7e84074a44b8154e98ba`
  - `Cargo.lock`: `9cb2d94bb7ceb84f614228b7e79b060e08e46a10253573ca424f70dbf7459f2a`
  - `WORK_LOG.md`: `798edc769d7dd2e283ff1bc8deae61853c7c377ac18f4946a20b18432f78d378`
  - `.git/HEAD`: `28d25bf82af4c0e2b72f50959b2beb859e3e60b9630a5e8c603dad4ddb2b6e80`
  - `docs/rb-03-lifecycle-and-file-completion-handoff.md`: `5d166c80c6a9a8ab4d3085a9d4ff872cc98efc7b6469ffe0583d48eedb6ff222`
  - `evidence/rebuild/parity-matrix.toml`: `d4223bbf399a7b0d57f2f2074fca3f1c3a2209e6b162ec1e5da0eb5541cf73bb`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/checkpoint.md`: `e7b4cfbe204a37bf4d878ec097b159d5e9e89a66a2464b79216b541db25964cf`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/requirement-test-map.md`: `e03765801daa74cc500f5948c15065de0e013e7398491c2f18f325cc71c875da`
  - `crates/modeler3d-core/src/project.rs`: `cbac8d8d250d1532891fc6e170a34e4cd519fcbe2c8d58e8ae66a25a43eddff7`
  - `crates/modeler3d-core/tests/project_golden.rs`: `28577b83dbe38767c3bd2722bc13a74932335d25b593ef5a1b4cd90b22752dea`
  - `crates/modeler3d-app/src/commands.rs`: `39f583c70c453b921c1a727874736f3d6f62648af2e948c748ace51a91b2735e`
  - `crates/modeler3d-app/src/dialogs.rs`: `bd019a0d41dacc068c4a6b103c327e92fa37d6660f89b014728a22770289eac6`
  - `crates/modeler3d-app/src/shell.rs`: `73c5ed71c956f6c947b18e0bf1fdb2251bc3e0da8ac27269ce80a1b8966a571c`

The receipt postdates the immutable snapshot. Only WORK_LOG.md and the RB-04 checkpoint are expected to differ afterward.

Post-receipt comparison: 1122 included paths in each tree; no missing or extra paths; exactly WORK_LOG.md and the RB-04 checkpoint differ from their post-copy receipts; no other size/SHA-256 differences; reparse entries 0 in each.


## Grouped viewport transform engineering checkpoint

Combined-group rendering/picking and shared CPU/GPU previews, grouped Move/Rotate/Resize, 26 resize handles, Grid/Object basis, opposite-face anchoring, identity/stale/cancel guards and 32-level group limit are implemented. See [viewport-transforms.md](viewport-transforms.md). All 122 independent Python gizmo oracle cases pass. Current workspace verification: 245 tests passed, none failed/ignored; fmt, strict changed core/viewport/app Clippy and deny passed. Python gizmo tests: 52 passed. Initial encoding/Clippy failures remain in logs/gesture-initial-*.txt; corrected source passes. Native final-code evidence remains pending while Windows is locked. No RB-03 review status was changed; no staging/commit or RB-05 work.

Planned additive RB-04 progress recovery snapshot (slice incomplete): `G:\Builder3D-Rust_snapshot_20261031-205354_RB-04-viewport-engineering`. Root target/.cache and reparse entries excluded; exact receipt follows after verification.


### RB-04 viewport-engineering verified additive snapshot

- Immutable snapshot: `G:\Builder3D-Rust_snapshot_20261031-205354_RB-04-viewport-engineering`. Verified 2026-10-31T20:53:58-06:00; Robocopy exit 1.
- Source and snapshot: 1,139 files / 165,729,202 bytes each; relative-path differences 0; size/SHA-256 differences 0; reparse entries 0 in each tree. Root target/, .cache/ and reparse entries excluded. Source, Git metadata, tests, documentation and evidence included.
- This is an interim RB-04 recovery point, not slice completion. Continue with mixed-selection numeric and stable selection/focus before RB-05; final native transform replay remains pending while Windows is locked.
- Pre-receipt recovery-critical SHA-256 values:
  - `crates/modeler3d-core/src/placement.rs`: `87c767a0e97adf72e422a340f92cb00c4a6194361128261395b4377e1218b8a6`
  - `crates/modeler3d-app/src/transform_gesture.rs`: `b7bd3fee78090c11eb2362fa6a21960d4fc6dc0a36d8e32b6aa24411208a8887`
  - `crates/modeler3d-app/src/gesture_tests.rs`: `69294ee5530b8e02d68ecfbd5a99ff377424b6dbfdde991b3c964e2f2ba5942e`
  - `crates/modeler3d-viewport/src/scene.rs`: `68cd984b002793be27e19ad62596ae8c25761fcda6dc362ca0a46b430d7eac0d`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/gesture-oracle.json`: `a093e6129c0ea8db88e167a0fafd8629c92ba63544ded89574e47355b57fd843`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/viewport-transforms.md`: `1c4fde4097aadcc278829cd7c36bdecd8fbeb96ce6a4867b626447245fc1b8ea`
  - `scripts/Initialize-BuildClang.ps1`: `73856cc160105f1a9c00bf845b14e29a3002dfdf6ba3efc5aed2742ed43a3d3f`
  - `docs/building.md`: `750a226f595466b2e5fc15f4fb453fe284c4c7f199dd9632e3e90233ec44495a`
  - `packaging/licenses/QHULL-COPYING.txt`: `106d55c931fd6a84822e5345d900273d059f1c27310d02567ccb313c5d18c55d`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/settle-fix.md`: `63a903bbb24d71770f971142e33fa8d063a3ad5155520c91fd442bd4a0c51fc1`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/report.md`: `dc0441b6b3183c6ec2cbb3c29b5ce0af3f1f470d900ae83f10b5af2ab8a49960`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/checkpoint.md`: `17db734f61d08f4b81f66c637b20b2ed28bae4bf343c0d120df56f46c3c8229d`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/verification.md`: `d74894148541519ac8786b2199b21fe98bdeb936efc6c4a0b333b68455371b83`
  - `crates/modeler3d-app/src/rotation.rs`: `dc9c80002d7aa507e88d48fab024158b4cb3eff0638c6f179f6fcc450aba5484`
  - `crates/modeler3d-app/src/transforms.rs`: `b9550eaf8a99e14e81d8d00653a10c636fe8c26ea17948120c9b586ce185fa79`
  - `AGENTS.md`: `986ba68a9e458b5c5bc9a7c9122e4a741d3dfc5f35228b431a2bc974c7f45a1b`
  - `Cargo.toml`: `d51c5bfb69d2e0cdf88fc252d58baafc54c99a926bef7e84074a44b8154e98ba`
  - `Cargo.lock`: `9cb2d94bb7ceb84f614228b7e79b060e08e46a10253573ca424f70dbf7459f2a`
  - `WORK_LOG.md`: `a06b163b6834b75ec6a95e32087cb9799a265646d96a86a71302ceeb262125fe`
  - `.git/HEAD`: `28d25bf82af4c0e2b72f50959b2beb859e3e60b9630a5e8c603dad4ddb2b6e80`
  - `docs/rb-03-lifecycle-and-file-completion-handoff.md`: `5d166c80c6a9a8ab4d3085a9d4ff872cc98efc7b6469ffe0583d48eedb6ff222`
  - `evidence/rebuild/parity-matrix.toml`: `6a03d4586adafebe761dbdc74f3973bc71a792622bc502ca540c27cbf9b42c77`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/checkpoint.md`: `e7b4cfbe204a37bf4d878ec097b159d5e9e89a66a2464b79216b541db25964cf`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/requirement-test-map.md`: `e03765801daa74cc500f5948c15065de0e013e7398491c2f18f325cc71c875da`
  - `crates/modeler3d-core/src/project.rs`: `cbac8d8d250d1532891fc6e170a34e4cd519fcbe2c8d58e8ae66a25a43eddff7`
  - `crates/modeler3d-core/tests/project_golden.rs`: `28577b83dbe38767c3bd2722bc13a74932335d25b593ef5a1b4cd90b22752dea`
  - `crates/modeler3d-app/src/commands.rs`: `b4932e48fb13edd53931a515e94311c7809eaf9e62feb8c3b19b97560aa135e9`
  - `crates/modeler3d-app/src/dialogs.rs`: `bd019a0d41dacc068c4a6b103c327e92fa37d6660f89b014728a22770289eac6`
  - `crates/modeler3d-app/src/shell.rs`: `6cfcf1e9ddcfd0d555ca79b504f1e564ec3473f23ba0c236e37bf24b869a1f7d`

The receipt postdates the immutable snapshot. Only WORK_LOG.md and the RB-04 checkpoint are expected to differ afterward.

Post-receipt comparison: 1139 included paths in each tree; no missing or extra paths; exactly WORK_LOG.md and the RB-04 checkpoint differ from their post-copy receipts; no other size/SHA-256 differences; reparse entries 0 in each.


## Numeric/selection/focus engineering checkpoint

Mixed hidden-selection numeric semantics, edited-axis/inch proportional lock, stable-ID Items Shift anchors, additive viewport picks, selection-order insertion/removal remapping, all modal/text shortcut guards and readable health warning are implemented. All 256 workspace tests pass with none failed/ignored; fmt, strict changed core/viewport/app Clippy and deny pass (numeric-gate-results.json). Real Python oracles: 122 gesture +32 multi-numeric +42 placement cases; focused Python scene tests 25 and gizmo tests 52 pass. No Python writes or RB-03 review promotions. The 50 paired historical actions /110 captures are now clearly labelled as predating the new repairs; historical capture source/build metadata is retained. Final native walkthrough/captures and visual review remain open because Windows is locked. RB-05 has not begun.

Planned additive RB-04 progress recovery snapshot (slice incomplete): `G:\Builder3D-Rust_snapshot_20261031-210342_RB-04-numeric-selection-engineering`. Root target/.cache and reparse entries excluded; exact receipt follows after verification.


### RB-04 numeric/selection-engineering verified additive snapshot

- Immutable snapshot: `G:\Builder3D-Rust_snapshot_20261031-210342_RB-04-numeric-selection-engineering`. Verified 2026-10-31T21:03:45-06:00; Robocopy exit 1.
- Source and snapshot: 1,158 files / 166,156,771 bytes each; relative-path differences 0; size/SHA-256 differences 0; reparse entries 0 in each tree. Root target/, .cache/ and reparse entries excluded. Source, Git metadata, tests, documentation and evidence included.
- This is an interim RB-04 recovery point, not slice completion. Continue the focused grouped recent-preview audit, then integrated boundary checks and final native evidence; Windows remains locked. RB-05 has not begun.
- Pre-receipt recovery-critical SHA-256 values:
  - `crates/modeler3d-core/src/placement.rs`: `87c767a0e97adf72e422a340f92cb00c4a6194361128261395b4377e1218b8a6`
  - `crates/modeler3d-app/src/transform_gesture.rs`: `b7bd3fee78090c11eb2362fa6a21960d4fc6dc0a36d8e32b6aa24411208a8887`
  - `crates/modeler3d-app/src/gesture_tests.rs`: `0f9116e0b2555a3aa45a4f3be5f88c030d9fa81cf50dc10f99fee06eb240074e`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/numeric-selection.md`: `2ff3d42317e8cf6487d7767e4bc258a810216d45466f0e7efad0e58038036e37`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/numeric-selection-oracle.json`: `13f72c05c129a3f8bf4005c38f70f10c617a17009ac4c60739219abe5e114852`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/placement-oracle.json`: `4da2aa0e86abff0610caa45171f952d4d95ac44b11e869341a25e52ba368ad66`
  - `crates/modeler3d-viewport/src/scene.rs`: `68cd984b002793be27e19ad62596ae8c25761fcda6dc362ca0a46b430d7eac0d`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/gesture-oracle.json`: `a093e6129c0ea8db88e167a0fafd8629c92ba63544ded89574e47355b57fd843`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/viewport-transforms.md`: `1c4fde4097aadcc278829cd7c36bdecd8fbeb96ce6a4867b626447245fc1b8ea`
  - `scripts/Initialize-BuildClang.ps1`: `73856cc160105f1a9c00bf845b14e29a3002dfdf6ba3efc5aed2742ed43a3d3f`
  - `docs/building.md`: `750a226f595466b2e5fc15f4fb453fe284c4c7f199dd9632e3e90233ec44495a`
  - `packaging/licenses/QHULL-COPYING.txt`: `106d55c931fd6a84822e5345d900273d059f1c27310d02567ccb313c5d18c55d`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/settle-fix.md`: `63a903bbb24d71770f971142e33fa8d063a3ad5155520c91fd442bd4a0c51fc1`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/report.md`: `f60e34f1573fdda44b5d82a706436d613eb37d39efc62ab390ddd81daf9c592f`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/checkpoint.md`: `79643ad79afa7b907d0aa6394bc57ab27f861147eac22761acdd14e7aa84d769`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/verification.md`: `e4352b32a15367cafcc567df931cc8567a465a6f02392c7d83285a72b502f36c`
  - `crates/modeler3d-app/src/rotation.rs`: `dc9c80002d7aa507e88d48fab024158b4cb3eff0638c6f179f6fcc450aba5484`
  - `crates/modeler3d-app/src/transforms.rs`: `dcbe4eebf97af53ba391c9a5d9070713a7737a9047b5d5f7089876de4a721861`
  - `AGENTS.md`: `986ba68a9e458b5c5bc9a7c9122e4a741d3dfc5f35228b431a2bc974c7f45a1b`
  - `Cargo.toml`: `d51c5bfb69d2e0cdf88fc252d58baafc54c99a926bef7e84074a44b8154e98ba`
  - `Cargo.lock`: `9cb2d94bb7ceb84f614228b7e79b060e08e46a10253573ca424f70dbf7459f2a`
  - `WORK_LOG.md`: `f74ec93257be511da703cfd5bd339e6b911a1db479d8cb55a83f6ba6f9a1175e`
  - `.git/HEAD`: `28d25bf82af4c0e2b72f50959b2beb859e3e60b9630a5e8c603dad4ddb2b6e80`
  - `docs/rb-03-lifecycle-and-file-completion-handoff.md`: `5d166c80c6a9a8ab4d3085a9d4ff872cc98efc7b6469ffe0583d48eedb6ff222`
  - `evidence/rebuild/parity-matrix.toml`: `03adb39857fd17d10040255b928eabbc11f23dad34cc2ec0e9a522e0024e9306`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/checkpoint.md`: `e7b4cfbe204a37bf4d878ec097b159d5e9e89a66a2464b79216b541db25964cf`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/requirement-test-map.md`: `e03765801daa74cc500f5948c15065de0e013e7398491c2f18f325cc71c875da`
  - `crates/modeler3d-core/src/project.rs`: `cbac8d8d250d1532891fc6e170a34e4cd519fcbe2c8d58e8ae66a25a43eddff7`
  - `crates/modeler3d-core/tests/project_golden.rs`: `28577b83dbe38767c3bd2722bc13a74932335d25b593ef5a1b4cd90b22752dea`
  - `crates/modeler3d-app/src/commands.rs`: `83fda1cc946b4c6fbfb4c73cd6076551654c9efab013ec0c29674a9ca673e0e1`
  - `crates/modeler3d-app/src/dialogs.rs`: `bd019a0d41dacc068c4a6b103c327e92fa37d6660f89b014728a22770289eac6`
  - `crates/modeler3d-app/src/shell.rs`: `bbefe90648b096e85f2ff4b03c747e9ead9f772ba9b80b746815cf72ec5b7586`

The receipt postdates the immutable snapshot. Only WORK_LOG.md and the RB-04 checkpoint are expected to differ afterward.

Post-receipt comparison: 1158 included paths in each tree; no missing or extra paths; exactly WORK_LOG.md and the RB-04 checkpoint differ from their post-copy receipts; no other size/SHA-256 differences; reparse entries 0 in each.


## Combined-group thumbnail correction

Recent-project thumbnails now draw each visible top-level combined mesh once, matching the viewport and Items. Retained children remain available for ungrouping; their geometry is not drawn twice. A differently colored, hidden-child, transformed-group regression matches the exact Items raster and verifies hidden root filtering without changing scene/history. All four recent-preview tests and strict app Clippy pass. The preceding workspace run passed 256 tests; the integrated boundary run will include this additional regression. RB-03 status proof (rb03-preserved-status-check.json): all 32 rows unchanged, including 30 native pass/two partial and visual/owner pending.

Planned additive RB-04 progress recovery snapshot (slice incomplete): `G:\Builder3D-Rust_snapshot_20261031-210453_RB-04-group-preview`. Root target/.cache and reparse entries excluded; exact receipt follows after verification.


### RB-04 group-preview verified additive snapshot

- Immutable snapshot: `G:\Builder3D-Rust_snapshot_20261031-210453_RB-04-group-preview`. Verified 2026-10-31T21:04:57-06:00; Robocopy exit 1.
- Source and snapshot: 1,161 files / 166,182,342 bytes each; relative-path differences 0; size/SHA-256 differences 0; reparse entries 0 in each tree. Root target/, .cache/ and reparse entries excluded. Source, Git metadata, tests, documentation and evidence included.
- This is an interim RB-04 recovery point, not slice completion. Continue integrated boundary checks and final native evidence; Windows remains locked. RB-05 has not begun.
- Pre-receipt recovery-critical SHA-256 values:
  - `crates/modeler3d-core/src/placement.rs`: `87c767a0e97adf72e422a340f92cb00c4a6194361128261395b4377e1218b8a6`
  - `crates/modeler3d-app/src/transform_gesture.rs`: `b7bd3fee78090c11eb2362fa6a21960d4fc6dc0a36d8e32b6aa24411208a8887`
  - `crates/modeler3d-app/src/gesture_tests.rs`: `0f9116e0b2555a3aa45a4f3be5f88c030d9fa81cf50dc10f99fee06eb240074e`
  - `crates/modeler3d-app/src/recent_preview.rs`: `5d4afde3bc674011a77f50ff95119de000563bf3cb1ab0a7d22b8c0223166051`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/rb03-preserved-status-check.json`: `6d35b75b8a9f952b12c07a80e7e0604cd685614b38d441e35c37357e4f3bd62a`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/numeric-selection.md`: `2ff3d42317e8cf6487d7767e4bc258a810216d45466f0e7efad0e58038036e37`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/numeric-selection-oracle.json`: `13f72c05c129a3f8bf4005c38f70f10c617a17009ac4c60739219abe5e114852`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/placement-oracle.json`: `4da2aa0e86abff0610caa45171f952d4d95ac44b11e869341a25e52ba368ad66`
  - `crates/modeler3d-viewport/src/scene.rs`: `68cd984b002793be27e19ad62596ae8c25761fcda6dc362ca0a46b430d7eac0d`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/gesture-oracle.json`: `a093e6129c0ea8db88e167a0fafd8629c92ba63544ded89574e47355b57fd843`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/viewport-transforms.md`: `1c4fde4097aadcc278829cd7c36bdecd8fbeb96ce6a4867b626447245fc1b8ea`
  - `scripts/Initialize-BuildClang.ps1`: `73856cc160105f1a9c00bf845b14e29a3002dfdf6ba3efc5aed2742ed43a3d3f`
  - `docs/building.md`: `750a226f595466b2e5fc15f4fb453fe284c4c7f199dd9632e3e90233ec44495a`
  - `packaging/licenses/QHULL-COPYING.txt`: `106d55c931fd6a84822e5345d900273d059f1c27310d02567ccb313c5d18c55d`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/settle-fix.md`: `63a903bbb24d71770f971142e33fa8d063a3ad5155520c91fd442bd4a0c51fc1`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/report.md`: `ec0e69109c541a8dc95ebd25a6ec0ba1d97e65ee4af9ae1ccca25c327ffb35fc`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/checkpoint.md`: `576d54e48415de8454c448af7174c6b96e1b1f9a572c3255af1e22a65b5514d0`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/verification.md`: `a19d8a66c9018f26ca4b65100ffbd3c0b6433158f029aaf4228b605cc47336de`
  - `crates/modeler3d-app/src/rotation.rs`: `dc9c80002d7aa507e88d48fab024158b4cb3eff0638c6f179f6fcc450aba5484`
  - `crates/modeler3d-app/src/transforms.rs`: `dcbe4eebf97af53ba391c9a5d9070713a7737a9047b5d5f7089876de4a721861`
  - `AGENTS.md`: `986ba68a9e458b5c5bc9a7c9122e4a741d3dfc5f35228b431a2bc974c7f45a1b`
  - `Cargo.toml`: `d51c5bfb69d2e0cdf88fc252d58baafc54c99a926bef7e84074a44b8154e98ba`
  - `Cargo.lock`: `9cb2d94bb7ceb84f614228b7e79b060e08e46a10253573ca424f70dbf7459f2a`
  - `WORK_LOG.md`: `906e2c0ad495d113ee3c88d42affbca867d1472d2fbd09e3da95af86ac2e24a7`
  - `.git/HEAD`: `28d25bf82af4c0e2b72f50959b2beb859e3e60b9630a5e8c603dad4ddb2b6e80`
  - `docs/rb-03-lifecycle-and-file-completion-handoff.md`: `5d166c80c6a9a8ab4d3085a9d4ff872cc98efc7b6469ffe0583d48eedb6ff222`
  - `evidence/rebuild/parity-matrix.toml`: `03adb39857fd17d10040255b928eabbc11f23dad34cc2ec0e9a522e0024e9306`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/checkpoint.md`: `e7b4cfbe204a37bf4d878ec097b159d5e9e89a66a2464b79216b541db25964cf`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/requirement-test-map.md`: `e03765801daa74cc500f5948c15065de0e013e7398491c2f18f325cc71c875da`
  - `crates/modeler3d-core/src/project.rs`: `cbac8d8d250d1532891fc6e170a34e4cd519fcbe2c8d58e8ae66a25a43eddff7`
  - `crates/modeler3d-core/tests/project_golden.rs`: `28577b83dbe38767c3bd2722bc13a74932335d25b593ef5a1b4cd90b22752dea`
  - `crates/modeler3d-app/src/commands.rs`: `83fda1cc946b4c6fbfb4c73cd6076551654c9efab013ec0c29674a9ca673e0e1`
  - `crates/modeler3d-app/src/dialogs.rs`: `bd019a0d41dacc068c4a6b103c327e92fa37d6660f89b014728a22770289eac6`
  - `crates/modeler3d-app/src/shell.rs`: `bbefe90648b096e85f2ff4b03c747e9ead9f772ba9b80b746815cf72ec5b7586`

The receipt postdates the immutable snapshot. Only WORK_LOG.md and the RB-04 checkpoint are expected to differ afterward.

Post-receipt comparison: 1161 included paths in each tree; no missing or extra paths; exactly WORK_LOG.md and the RB-04 checkpoint differ from their post-copy receipts; no other size/SHA-256 differences; reparse entries 0 in each.


## Current-source identity correction during boundary verification

Same-revision project replacement now clears Items health/textures and gizmo frames. Starting a new scene clears the same caches. Changing the selected set cancels a native drag, matching Python, while the semantic snapshot seam still preserves captured identities. Commit validates the captured IDs/poses at their unchanged-generation/revision indices in linear time. Regression and all 258 workspace tests pass, with none failed/ignored; strict all-workspace/all-target/all-feature Clippy passes (identity-final-gates.json). Current native replay is still unavailable while Windows is locked.

The initial boundary runner (tool session 29798) has passed fmt/check/257 tests/strict workspace Clippy/deny/metadata/smoke/release and is completing workspace/app bloat; those earlier app checks predate the identity correction and must not be represented as its final build. Current full tests/Clippy above supersede them. After that runner finishes, perform current-source fmt/check/smoke/release and explicit app bloat, preserve the canonical release executable/hash before bloat, merge exact current evidence, validate the unchanged Python/RB-03 rows, then create the verified additive native-pending recovery snapshot. Do not change source again without a concrete failure. Final native walkthrough and Light/Dark captures remain pending; no RB-05 implementation. Latest immutable snapshot is still G:\Builder3D-Rust_snapshot_20261031-210453_RB-04-group-preview and predates this correction.


## RB-04 engineering verified; native completion unavailable

Current source passes 258 workspace tests (0 failed/ignored), strict full-workspace Clippy, fmt/check/deny/metadata, debug/release smoke, release and both bloat commands. Cargo audit is unavailable. Ordinary release 14,916,096 bytes / SHA-256 b61bca76e1ba548b53c5f26829c5f108280fa2fcc65f8f398c10e203df356bb0; explicit app bloat 11.6 MiB .text /14.2 MiB file (separate artifact). 205 independent Python gesture/numeric/placement/Euler cases pass; focused reference runs and unchanged Python status are recorded. Evidence validation checks 108 original/packaged asset hashes, 142 feature IDs, 33 catalog IDs and all 50 historical pairs/110 capture hashes. All 32 RB-03 review rows are unchanged.

The full RB-04 task is still incomplete because Windows remains locked: no final-code native walkthrough or capture was sent. Exact remaining native/visual work and the RB-05 next slice are in evidence/rebuild/rb-04-scene-and-precise-transforms/boundary-verification.md and checkpoint.md. Historical captures are labelled; no acceptance is inferred. User-authorized implementation proceeded without routine confirmation, Python stayed read-only, and nothing was staged/committed. Next recovery is explicitly engineering/native-pending, followed by the remaining native exit evidence when unlocked; stop before RB-05.


## Native Settle replay after Windows unlocked

The owner replied that Windows would be unlocked, and the input desktop now verifies as Default. A nonzero foreground alone was briefly insufficient: the first resumed reference capture was rejected. No image from that attempt was accepted. The harness now checks the input desktop and reacquires/verifies the exact target before each action/capture. This follows the read-only Microsoft OpenInputDesktop/GetUserObjectInformation APIs; it does not switch desktops or bypass locking.

Python's real Qt/OpenGL window completed all 16 single/two-object before/after/undo/redo states in Light/Dark. It closed before Rust opened. Rust's current debug build SHA-256 3f84eb8c288f6db21107763ad289c4b9874bed5312f339ad679a18bd0df705cf completed the same 16 states through guarded native pointer/keyboard. Native Save As produced four independent .3dp states without overwriting inputs. Exact saved matrices match Python within 9.32e-9 per entry. Both applications preserve IDs within each case; Python history increases once and Rust scene revisions increase exactly three across Settle/Undo/Redo (2->5 single; 4->7 pair). Light/Dark saved geometry hashes are identical per case. See native-settle-verification.json. Remaining native actions are still in progress; no owner/visual acceptance or RB-03 review promotion.

Visible differences: Python uses large wireframe tiles, flat shading/selection tint, blue selection boxes, near-face arrows and a compact icon strip; Rust uses small shaded rows, softer green shading, a finer grid, centered arrows and larger text fields. The matching world values and geometry are verified independently from these appearance differences.

Planned additive RB-04 progress recovery snapshot (slice incomplete): `G:\Builder3D-Rust_snapshot_20261031-213906_RB-04-native-settle`. Root target/.cache and reparse entries excluded; exact receipt follows after verification.


### RB-04 native Settle verified additive snapshot

- Immutable snapshot: `G:\Builder3D-Rust_snapshot_20261031-213906_RB-04-native-settle`. Verified 2026-10-31T21:39:10-06:00; Robocopy exit 1.
- Source and snapshot: 1,237 files / 173,413,983 bytes each; relative-path differences 0; size/SHA-256 differences 0; reparse entries 0 in each tree. Root target/, .cache/ and reparse entries excluded. Source, Git metadata, tests, documentation and evidence included.
- This is an interim RB-04 recovery point, not slice completion. Native Settle is verified; Windows is unlocked. Continue the remaining full RB-04 native transform/scene/focus evidence before RB-05.
- Pre-receipt recovery-critical SHA-256 values:
  - `crates/modeler3d-core/src/placement.rs`: `87c767a0e97adf72e422a340f92cb00c4a6194361128261395b4377e1218b8a6`
  - `crates/modeler3d-app/src/transform_gesture.rs`: `ea55e0dc69e5bce2807271f2b89b5dbbde4e7638f4490447baabc275b52f7b50`
  - `crates/modeler3d-app/src/gesture_tests.rs`: `fd4f0e70b65dcc9c1c0aaa45e8479e7d3372b9738366395d7172544237e2dc93`
  - `crates/modeler3d-app/src/recent_preview.rs`: `5d4afde3bc674011a77f50ff95119de000563bf3cb1ab0a7d22b8c0223166051`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native-settle-verification.json`: `8be55d25bfbcbe6a124c4e39ee75dafd60df2aac8a335dc4c7ec138037f3bcd2`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/python-settle-results.json`: `b4d32acb5cf8af18bb458db7dd52e0865b02301deb3b342b64df59715c8c3e96`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native_desktop_guard.py`: `ab64c435ba857d1c415dd3eb56292f0600b26a66cd8c99fc91d23eaff8dd1ad0`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native-rust-workflow.ps1`: `43daaa3907eb2c913e4b76f5850be825614c05c46ab514c13b7cfb98378246bb`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/rb03-preserved-status-check.json`: `6d35b75b8a9f952b12c07a80e7e0604cd685614b38d441e35c37357e4f3bd62a`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/numeric-selection.md`: `e7a1f4bd2b897ebd72f92a0d627fd9df91597ef838966de34bc78bc80f2585ad`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/numeric-selection-oracle.json`: `13f72c05c129a3f8bf4005c38f70f10c617a17009ac4c60739219abe5e114852`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/placement-oracle.json`: `4da2aa0e86abff0610caa45171f952d4d95ac44b11e869341a25e52ba368ad66`
  - `crates/modeler3d-viewport/src/scene.rs`: `68cd984b002793be27e19ad62596ae8c25761fcda6dc362ca0a46b430d7eac0d`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/gesture-oracle.json`: `a093e6129c0ea8db88e167a0fafd8629c92ba63544ded89574e47355b57fd843`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/viewport-transforms.md`: `f34082f746b68512e9393889e807292638d848063af0d317e3d174bc1f90b3d6`
  - `scripts/Initialize-BuildClang.ps1`: `73856cc160105f1a9c00bf845b14e29a3002dfdf6ba3efc5aed2742ed43a3d3f`
  - `docs/building.md`: `750a226f595466b2e5fc15f4fb453fe284c4c7f199dd9632e3e90233ec44495a`
  - `packaging/licenses/QHULL-COPYING.txt`: `106d55c931fd6a84822e5345d900273d059f1c27310d02567ccb313c5d18c55d`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/settle-fix.md`: `63a903bbb24d71770f971142e33fa8d063a3ad5155520c91fd442bd4a0c51fc1`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/report.md`: `42089fb0f16d830925835315515505e899c029a91d864329fc183f5e7380e509`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/checkpoint.md`: `01bf81388b59918354195a62b7178662d51c09a8195d152a3b06ed331ff949df`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/verification.md`: `1f8cec516f1655e3bbe9f869e9e4ce1196412bc43c5e698e1f10ca1e308405b7`
  - `crates/modeler3d-app/src/rotation.rs`: `dc9c80002d7aa507e88d48fab024158b4cb3eff0638c6f179f6fcc450aba5484`
  - `crates/modeler3d-app/src/transforms.rs`: `dcbe4eebf97af53ba391c9a5d9070713a7737a9047b5d5f7089876de4a721861`
  - `AGENTS.md`: `986ba68a9e458b5c5bc9a7c9122e4a741d3dfc5f35228b431a2bc974c7f45a1b`
  - `Cargo.toml`: `d51c5bfb69d2e0cdf88fc252d58baafc54c99a926bef7e84074a44b8154e98ba`
  - `Cargo.lock`: `9cb2d94bb7ceb84f614228b7e79b060e08e46a10253573ca424f70dbf7459f2a`
  - `WORK_LOG.md`: `c9ecc887f7de19d5705b3dd91f496c707b07f771ee064d3aff7b89e2bfb208d2`
  - `.git/HEAD`: `28d25bf82af4c0e2b72f50959b2beb859e3e60b9630a5e8c603dad4ddb2b6e80`
  - `docs/rb-03-lifecycle-and-file-completion-handoff.md`: `5d166c80c6a9a8ab4d3085a9d4ff872cc98efc7b6469ffe0583d48eedb6ff222`
  - `evidence/rebuild/parity-matrix.toml`: `88af89d940f22db9e8a939f2c93e3c34aac864257e6fbf73f3a3c0cb33df6384`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/checkpoint.md`: `e7b4cfbe204a37bf4d878ec097b159d5e9e89a66a2464b79216b541db25964cf`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/requirement-test-map.md`: `e03765801daa74cc500f5948c15065de0e013e7398491c2f18f325cc71c875da`
  - `crates/modeler3d-core/src/project.rs`: `cbac8d8d250d1532891fc6e170a34e4cd519fcbe2c8d58e8ae66a25a43eddff7`
  - `crates/modeler3d-core/tests/project_golden.rs`: `28577b83dbe38767c3bd2722bc13a74932335d25b593ef5a1b4cd90b22752dea`
  - `crates/modeler3d-app/src/commands.rs`: `f095c3fd5f92c2858341fdff111cdde83f544c7cd808fb86556b2511f07aab9f`
  - `crates/modeler3d-app/src/dialogs.rs`: `bd019a0d41dacc068c4a6b103c327e92fa37d6660f89b014728a22770289eac6`
  - `crates/modeler3d-app/src/shell.rs`: `4ff26703670e3deae24ae122c151b6b8fd427882be1f9d222abe74c5ddfb3c99`

The receipt postdates the immutable snapshot. Only WORK_LOG.md and the RB-04 checkpoint are expected to differ afterward.

Post-receipt comparison: 1237 included paths in each tree; no missing or extra paths; exactly WORK_LOG.md and the RB-04 checkpoint differ from their post-copy receipts; no other size/SHA-256 differences; reparse entries 0 in each.

RB-04 active native continuation: fixed front-layer Move/Rotate controls; matched Python 14 visible Resize grips (internal edge math remains tested). Native inch X-only edit revealed Y rounding drift; edited-axis tracking now preserves untouched exact axes and no-edit Apply history. Native saved replay Y=15 exactly; workspace tests259/fmt/strict all-feature Clippy pass. Current native debug c5eac319490c962f9f89d18a6475d84815205a3051ea9b349d69269a1dce82c4. Paired viewport/scene native work continues; RB-03 review remains pending; no RB-05/staging/commit.

Planned additive RB-04 progress recovery snapshot (slice incomplete): `G:\Builder3D-Rust_snapshot_20261031-221207_RB-04-native-viewport-precision`. Root target/.cache and reparse entries excluded; exact receipt follows after verification.


### RB-04 native viewport/precision verified additive snapshot

- Immutable snapshot: `G:\Builder3D-Rust_snapshot_20261031-221207_RB-04-native-viewport-precision`. Verified 2026-10-31T22:12:12-06:00; Robocopy exit 1.
- Source and snapshot: 1,413 files / 192,760,764 bytes each; relative-path differences 0; size/SHA-256 differences 0; reparse entries 0 in each tree. Root target/, .cache/ and reparse entries excluded. Source, Git metadata, tests, documentation and evidence included.
- This is an interim RB-04 recovery point, not slice completion. Native Settle and initial Move/Rotate saved results are verified; inch untouched-axis precision is repaired. Windows is unlocked. Continue the remaining full RB-04 native transform/scene/focus evidence before RB-05.
- Pre-receipt recovery-critical SHA-256 values:
  - `crates/modeler3d-core/src/placement.rs`: `87c767a0e97adf72e422a340f92cb00c4a6194361128261395b4377e1218b8a6`
  - `crates/modeler3d-app/src/transform_gesture.rs`: `128d0a8f5033ce4df1875d5c1dc84c63731650ae8f1d5b902b2d3d54f510e3bc`
  - `crates/modeler3d-app/src/gesture_tests.rs`: `fd4f0e70b65dcc9c1c0aaa45e8479e7d3372b9738366395d7172544237e2dc93`
  - `crates/modeler3d-app/src/recent_preview.rs`: `5d4afde3bc674011a77f50ff95119de000563bf3cb1ab0a7d22b8c0223166051`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native-settle-verification.json`: `8be55d25bfbcbe6a124c4e39ee75dafd60df2aac8a335dc4c7ec138037f3bcd2`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/python-settle-results.json`: `b4d32acb5cf8af18bb458db7dd52e0865b02301deb3b342b64df59715c8c3e96`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native_desktop_guard.py`: `ab64c435ba857d1c415dd3eb56292f0600b26a66cd8c99fc91d23eaff8dd1ad0`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native-rust-workflow.ps1`: `11242b795f75c59f7a2e7e8bc0a9204a013bcf5f803fd0c8e7ada8116d48607b`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/rb03-preserved-status-check.json`: `6d35b75b8a9f952b12c07a80e7e0604cd685614b38d441e35c37357e4f3bd62a`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/numeric-selection.md`: `e7a1f4bd2b897ebd72f92a0d627fd9df91597ef838966de34bc78bc80f2585ad`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/numeric-selection-oracle.json`: `13f72c05c129a3f8bf4005c38f70f10c617a17009ac4c60739219abe5e114852`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/placement-oracle.json`: `4da2aa0e86abff0610caa45171f952d4d95ac44b11e869341a25e52ba368ad66`
  - `crates/modeler3d-viewport/src/scene.rs`: `68cd984b002793be27e19ad62596ae8c25761fcda6dc362ca0a46b430d7eac0d`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/gesture-oracle.json`: `a093e6129c0ea8db88e167a0fafd8629c92ba63544ded89574e47355b57fd843`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/viewport-transforms.md`: `f34082f746b68512e9393889e807292638d848063af0d317e3d174bc1f90b3d6`
  - `scripts/Initialize-BuildClang.ps1`: `73856cc160105f1a9c00bf845b14e29a3002dfdf6ba3efc5aed2742ed43a3d3f`
  - `docs/building.md`: `750a226f595466b2e5fc15f4fb453fe284c4c7f199dd9632e3e90233ec44495a`
  - `packaging/licenses/QHULL-COPYING.txt`: `106d55c931fd6a84822e5345d900273d059f1c27310d02567ccb313c5d18c55d`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/settle-fix.md`: `63a903bbb24d71770f971142e33fa8d063a3ad5155520c91fd442bd4a0c51fc1`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/report.md`: `42089fb0f16d830925835315515505e899c029a91d864329fc183f5e7380e509`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/checkpoint.md`: `e6eb0b3f2fdf502e32fff4d9df19eeb047781f356d0ae13671308b2c19cad50e`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/verification.md`: `1f8cec516f1655e3bbe9f869e9e4ce1196412bc43c5e698e1f10ca1e308405b7`
  - `crates/modeler3d-app/src/rotation.rs`: `dc9c80002d7aa507e88d48fab024158b4cb3eff0638c6f179f6fcc450aba5484`
  - `crates/modeler3d-app/src/transforms.rs`: `dcbe4eebf97af53ba391c9a5d9070713a7737a9047b5d5f7089876de4a721861`
  - `AGENTS.md`: `986ba68a9e458b5c5bc9a7c9122e4a741d3dfc5f35228b431a2bc974c7f45a1b`
  - `Cargo.toml`: `d51c5bfb69d2e0cdf88fc252d58baafc54c99a926bef7e84074a44b8154e98ba`
  - `Cargo.lock`: `9cb2d94bb7ceb84f614228b7e79b060e08e46a10253573ca424f70dbf7459f2a`
  - `WORK_LOG.md`: `41fb2b6df74a45b36ff445d746d96f9b9de39c86c16dc1873fc40e995bc84fea`
  - `.git/HEAD`: `28d25bf82af4c0e2b72f50959b2beb859e3e60b9630a5e8c603dad4ddb2b6e80`
  - `docs/rb-03-lifecycle-and-file-completion-handoff.md`: `5d166c80c6a9a8ab4d3085a9d4ff872cc98efc7b6469ffe0583d48eedb6ff222`
  - `evidence/rebuild/parity-matrix.toml`: `88af89d940f22db9e8a939f2c93e3c34aac864257e6fbf73f3a3c0cb33df6384`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/checkpoint.md`: `e7b4cfbe204a37bf4d878ec097b159d5e9e89a66a2464b79216b541db25964cf`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/requirement-test-map.md`: `e03765801daa74cc500f5948c15065de0e013e7398491c2f18f325cc71c875da`
  - `crates/modeler3d-core/src/project.rs`: `cbac8d8d250d1532891fc6e170a34e4cd519fcbe2c8d58e8ae66a25a43eddff7`
  - `crates/modeler3d-core/tests/project_golden.rs`: `28577b83dbe38767c3bd2722bc13a74932335d25b593ef5a1b4cd90b22752dea`
  - `crates/modeler3d-app/src/commands.rs`: `f095c3fd5f92c2858341fdff111cdde83f544c7cd808fb86556b2511f07aab9f`
  - `crates/modeler3d-app/src/dialogs.rs`: `bd019a0d41dacc068c4a6b103c327e92fa37d6660f89b014728a22770289eac6`
  - `crates/modeler3d-app/src/shell.rs`: `9b272d4c76385b45ce24220f540dbbbfff9134867b729e9b28ef1819f419e350`

The receipt postdates the immutable snapshot. Only WORK_LOG.md and the RB-04 checkpoint are expected to differ afterward.

Post-receipt comparison: 1413 included paths in each tree; no missing or extra paths; exactly WORK_LOG.md and the RB-04 checkpoint differ from their post-copy receipts; no other size/SHA-256 differences; reparse entries 0 in each.

RB-04 current native viewport checkpoint: all22 Light/Dark physical drag/termination cases pass saved pose/ID/mesh/history checks; all16 current C5 Settle states replayed with four exact v2 saves. 259 tests and final current-source release/smoke/explicit app size gates pass (final-native-* records). Source/Rust GUIs closed. Next is prepared full scene/widget/dialog source harness and Rust replay, followed by evidence/ledger/completion snapshot. No RB-03 review promotion, RB-05, staging or commit.

Planned additive RB-04 progress recovery snapshot (slice incomplete): `G:\Builder3D-Rust_snapshot_20261031-223059_RB-04-native-viewport-verified`. Root target/.cache and reparse entries excluded; exact receipt follows after verification.


### RB-04 native viewport/precision verified additive snapshot

- Immutable snapshot: `G:\Builder3D-Rust_snapshot_20261031-223059_RB-04-native-viewport-verified`. Verified 2026-10-31T22:31:03-06:00; Robocopy exit 1.
- Source and snapshot: 1,532 files / 208,234,310 bytes each; relative-path differences 0; size/SHA-256 differences 0; reparse entries 0 in each tree. Root target/, .cache/ and reparse entries excluded. Source, Git metadata, tests, documentation and evidence included.
- This is an interim RB-04 recovery point, not slice completion. Native Settle and all22 viewport and current Settle saved results are verified; inch untouched-axis precision is repaired. Windows is unlocked. Continue the remaining full RB-04 native transform/scene/focus evidence before RB-05.
- Pre-receipt recovery-critical SHA-256 values:
  - `crates/modeler3d-core/src/placement.rs`: `87c767a0e97adf72e422a340f92cb00c4a6194361128261395b4377e1218b8a6`
  - `crates/modeler3d-app/src/transform_gesture.rs`: `128d0a8f5033ce4df1875d5c1dc84c63731650ae8f1d5b902b2d3d54f510e3bc`
  - `crates/modeler3d-app/src/gesture_tests.rs`: `fd4f0e70b65dcc9c1c0aaa45e8479e7d3372b9738366395d7172544237e2dc93`
  - `crates/modeler3d-app/src/recent_preview.rs`: `5d4afde3bc674011a77f50ff95119de000563bf3cb1ab0a7d22b8c0223166051`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native-settle-verification.json`: `6aeaf2e2c80392ccfd60869b54696db8940a1d34cf49f4dd945998a7b0005a2a`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/python-settle-results.json`: `b4d32acb5cf8af18bb458db7dd52e0865b02301deb3b342b64df59715c8c3e96`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native_desktop_guard.py`: `ab64c435ba857d1c415dd3eb56292f0600b26a66cd8c99fc91d23eaff8dd1ad0`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native-rust-workflow.ps1`: `df5a35738cb1752cfdfecd10908fe0e4eb10d9d7e82c389eddaae2a15c84ed03`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/rb03-preserved-status-check.json`: `6d35b75b8a9f952b12c07a80e7e0604cd685614b38d441e35c37357e4f3bd62a`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/numeric-selection.md`: `e7a1f4bd2b897ebd72f92a0d627fd9df91597ef838966de34bc78bc80f2585ad`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/numeric-selection-oracle.json`: `13f72c05c129a3f8bf4005c38f70f10c617a17009ac4c60739219abe5e114852`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/placement-oracle.json`: `4da2aa0e86abff0610caa45171f952d4d95ac44b11e869341a25e52ba368ad66`
  - `crates/modeler3d-viewport/src/scene.rs`: `68cd984b002793be27e19ad62596ae8c25761fcda6dc362ca0a46b430d7eac0d`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/gesture-oracle.json`: `a093e6129c0ea8db88e167a0fafd8629c92ba63544ded89574e47355b57fd843`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/viewport-transforms.md`: `f34082f746b68512e9393889e807292638d848063af0d317e3d174bc1f90b3d6`
  - `scripts/Initialize-BuildClang.ps1`: `73856cc160105f1a9c00bf845b14e29a3002dfdf6ba3efc5aed2742ed43a3d3f`
  - `docs/building.md`: `750a226f595466b2e5fc15f4fb453fe284c4c7f199dd9632e3e90233ec44495a`
  - `packaging/licenses/QHULL-COPYING.txt`: `106d55c931fd6a84822e5345d900273d059f1c27310d02567ccb313c5d18c55d`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/settle-fix.md`: `63a903bbb24d71770f971142e33fa8d063a3ad5155520c91fd442bd4a0c51fc1`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/report.md`: `42089fb0f16d830925835315515505e899c029a91d864329fc183f5e7380e509`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/checkpoint.md`: `015781a6cd60a6e5d211073fb82260bb9951ece631c4e1a3b6738ecd971c7bd4`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/verification.md`: `1f8cec516f1655e3bbe9f869e9e4ce1196412bc43c5e698e1f10ca1e308405b7`
  - `crates/modeler3d-app/src/rotation.rs`: `dc9c80002d7aa507e88d48fab024158b4cb3eff0638c6f179f6fcc450aba5484`
  - `crates/modeler3d-app/src/transforms.rs`: `dcbe4eebf97af53ba391c9a5d9070713a7737a9047b5d5f7089876de4a721861`
  - `AGENTS.md`: `986ba68a9e458b5c5bc9a7c9122e4a741d3dfc5f35228b431a2bc974c7f45a1b`
  - `Cargo.toml`: `d51c5bfb69d2e0cdf88fc252d58baafc54c99a926bef7e84074a44b8154e98ba`
  - `Cargo.lock`: `9cb2d94bb7ceb84f614228b7e79b060e08e46a10253573ca424f70dbf7459f2a`
  - `WORK_LOG.md`: `f56ad3deeae9df618216c3c711d2f4d0bc65aea7fc06831cc05c1f3393fb0a50`
  - `.git/HEAD`: `28d25bf82af4c0e2b72f50959b2beb859e3e60b9630a5e8c603dad4ddb2b6e80`
  - `docs/rb-03-lifecycle-and-file-completion-handoff.md`: `5d166c80c6a9a8ab4d3085a9d4ff872cc98efc7b6469ffe0583d48eedb6ff222`
  - `evidence/rebuild/parity-matrix.toml`: `88af89d940f22db9e8a939f2c93e3c34aac864257e6fbf73f3a3c0cb33df6384`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/checkpoint.md`: `e7b4cfbe204a37bf4d878ec097b159d5e9e89a66a2464b79216b541db25964cf`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/requirement-test-map.md`: `e03765801daa74cc500f5948c15065de0e013e7398491c2f18f325cc71c875da`
  - `crates/modeler3d-core/src/project.rs`: `cbac8d8d250d1532891fc6e170a34e4cd519fcbe2c8d58e8ae66a25a43eddff7`
  - `crates/modeler3d-core/tests/project_golden.rs`: `28577b83dbe38767c3bd2722bc13a74932335d25b593ef5a1b4cd90b22752dea`
  - `crates/modeler3d-app/src/commands.rs`: `f095c3fd5f92c2858341fdff111cdde83f544c7cd808fb86556b2511f07aab9f`
  - `crates/modeler3d-app/src/dialogs.rs`: `bd019a0d41dacc068c4a6b103c327e92fa37d6660f89b014728a22770289eac6`
  - `crates/modeler3d-app/src/shell.rs`: `9b272d4c76385b45ce24220f540dbbbfff9134867b729e9b28ef1819f419e350`

The receipt postdates the immutable snapshot. Only WORK_LOG.md and the RB-04 checkpoint are expected to differ afterward.

Post-receipt comparison: 1532 included paths in each tree; no missing or extra paths; exactly WORK_LOG.md and the RB-04 checkpoint differ from their post-copy receipts; no other size/SHA-256 differences; reparse entries 0 in each.

RB-04 expanded reference scene subtask complete:37 cases x2 themes,238 live frames,74 verified before/after cases, dialogs and repair reports, exact undo/redo. Source tree read-only/status unchanged. Source Shift observed [0,2] versus explicit Rust range [0,1,2]; direct prepared source naming/visibility remain labelled. Source/Rust GUIs closed. Next is Rust shared-fixture scene replay; no RB-05/staging/commit/review promotion.

Planned additive RB-04 progress recovery snapshot (slice incomplete): `G:\Builder3D-Rust_snapshot_20261031-223608_RB-04-native-reference-scenes`. Root target/.cache and reparse entries excluded; exact receipt follows after verification.


### RB-04 expanded reference scene verified additive snapshot

- Immutable snapshot: `G:\Builder3D-Rust_snapshot_20261031-223608_RB-04-native-reference-scenes`. Verified 2026-10-31T22:36:14-06:00; Robocopy exit 1.
- Source and snapshot: 1,953 files / 248,474,302 bytes each; relative-path differences 0; size/SHA-256 differences 0; reparse entries 0 in each tree. Root target/, .cache/ and reparse entries excluded. Source, Git metadata, tests, documentation and evidence included.
- This is an interim RB-04 recovery point, not slice completion. Native transforms and all74 expanded reference scene results are verified; corresponding Rust scene replay is next. Windows is unlocked. Continue the remaining full RB-04 native transform/scene/focus evidence before RB-05.
- Pre-receipt recovery-critical SHA-256 values:
  - `crates/modeler3d-core/src/placement.rs`: `87c767a0e97adf72e422a340f92cb00c4a6194361128261395b4377e1218b8a6`
  - `crates/modeler3d-app/src/transform_gesture.rs`: `128d0a8f5033ce4df1875d5c1dc84c63731650ae8f1d5b902b2d3d54f510e3bc`
  - `crates/modeler3d-app/src/gesture_tests.rs`: `fd4f0e70b65dcc9c1c0aaa45e8479e7d3372b9738366395d7172544237e2dc93`
  - `crates/modeler3d-app/src/recent_preview.rs`: `5d4afde3bc674011a77f50ff95119de000563bf3cb1ab0a7d22b8c0223166051`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native-settle-verification.json`: `6aeaf2e2c80392ccfd60869b54696db8940a1d34cf49f4dd945998a7b0005a2a`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/python-settle-results.json`: `b4d32acb5cf8af18bb458db7dd52e0865b02301deb3b342b64df59715c8c3e96`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native_desktop_guard.py`: `ab64c435ba857d1c415dd3eb56292f0600b26a66cd8c99fc91d23eaff8dd1ad0`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native-rust-workflow.ps1`: `df5a35738cb1752cfdfecd10908fe0e4eb10d9d7e82c389eddaae2a15c84ed03`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/rb03-preserved-status-check.json`: `6d35b75b8a9f952b12c07a80e7e0604cd685614b38d441e35c37357e4f3bd62a`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/numeric-selection.md`: `e7a1f4bd2b897ebd72f92a0d627fd9df91597ef838966de34bc78bc80f2585ad`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/numeric-selection-oracle.json`: `13f72c05c129a3f8bf4005c38f70f10c617a17009ac4c60739219abe5e114852`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/placement-oracle.json`: `4da2aa0e86abff0610caa45171f952d4d95ac44b11e869341a25e52ba368ad66`
  - `crates/modeler3d-viewport/src/scene.rs`: `68cd984b002793be27e19ad62596ae8c25761fcda6dc362ca0a46b430d7eac0d`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/gesture-oracle.json`: `a093e6129c0ea8db88e167a0fafd8629c92ba63544ded89574e47355b57fd843`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/viewport-transforms.md`: `f34082f746b68512e9393889e807292638d848063af0d317e3d174bc1f90b3d6`
  - `scripts/Initialize-BuildClang.ps1`: `73856cc160105f1a9c00bf845b14e29a3002dfdf6ba3efc5aed2742ed43a3d3f`
  - `docs/building.md`: `750a226f595466b2e5fc15f4fb453fe284c4c7f199dd9632e3e90233ec44495a`
  - `packaging/licenses/QHULL-COPYING.txt`: `106d55c931fd6a84822e5345d900273d059f1c27310d02567ccb313c5d18c55d`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/settle-fix.md`: `63a903bbb24d71770f971142e33fa8d063a3ad5155520c91fd442bd4a0c51fc1`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/report.md`: `42089fb0f16d830925835315515505e899c029a91d864329fc183f5e7380e509`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/checkpoint.md`: `4db6cbd0938f3cd65ac36a3401e889880251e6ee830c7fcff0fc722d2e618818`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/verification.md`: `1f8cec516f1655e3bbe9f869e9e4ce1196412bc43c5e698e1f10ca1e308405b7`
  - `crates/modeler3d-app/src/rotation.rs`: `dc9c80002d7aa507e88d48fab024158b4cb3eff0638c6f179f6fcc450aba5484`
  - `crates/modeler3d-app/src/transforms.rs`: `dcbe4eebf97af53ba391c9a5d9070713a7737a9047b5d5f7089876de4a721861`
  - `AGENTS.md`: `986ba68a9e458b5c5bc9a7c9122e4a741d3dfc5f35228b431a2bc974c7f45a1b`
  - `Cargo.toml`: `d51c5bfb69d2e0cdf88fc252d58baafc54c99a926bef7e84074a44b8154e98ba`
  - `Cargo.lock`: `9cb2d94bb7ceb84f614228b7e79b060e08e46a10253573ca424f70dbf7459f2a`
  - `WORK_LOG.md`: `522d9a67209853123742b0a17d480bd3d074c39873de0fe2242f5c15b15f2d17`
  - `.git/HEAD`: `28d25bf82af4c0e2b72f50959b2beb859e3e60b9630a5e8c603dad4ddb2b6e80`
  - `docs/rb-03-lifecycle-and-file-completion-handoff.md`: `5d166c80c6a9a8ab4d3085a9d4ff872cc98efc7b6469ffe0583d48eedb6ff222`
  - `evidence/rebuild/parity-matrix.toml`: `88af89d940f22db9e8a939f2c93e3c34aac864257e6fbf73f3a3c0cb33df6384`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/checkpoint.md`: `e7b4cfbe204a37bf4d878ec097b159d5e9e89a66a2464b79216b541db25964cf`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/requirement-test-map.md`: `e03765801daa74cc500f5948c15065de0e013e7398491c2f18f325cc71c875da`
  - `crates/modeler3d-core/src/project.rs`: `cbac8d8d250d1532891fc6e170a34e4cd519fcbe2c8d58e8ae66a25a43eddff7`
  - `crates/modeler3d-core/tests/project_golden.rs`: `28577b83dbe38767c3bd2722bc13a74932335d25b593ef5a1b4cd90b22752dea`
  - `crates/modeler3d-app/src/commands.rs`: `f095c3fd5f92c2858341fdff111cdde83f544c7cd808fb86556b2511f07aab9f`
  - `crates/modeler3d-app/src/dialogs.rs`: `bd019a0d41dacc068c4a6b103c327e92fa37d6660f89b014728a22770289eac6`
  - `crates/modeler3d-app/src/shell.rs`: `9b272d4c76385b45ce24220f540dbbbfff9134867b729e9b28ef1819f419e350`

The receipt postdates the immutable snapshot. Only WORK_LOG.md and the RB-04 checkpoint are expected to differ afterward.

Post-receipt comparison: 1953 included paths in each tree; no missing or extra paths; exactly WORK_LOG.md and the RB-04 checkpoint differ from their post-copy receipts; no other size/SHA-256 differences; reparse entries 0 in each.

RB-04 Light selection subtask verified:9 native cases, exact unchanged mesh/IDs/poses/history and captured row selections. Explicit contiguous Shift range differs from source endpoint toggle; recorded, not accepted. C5 source/build gates unchanged. Native Rust29512 remains open/saved, Python closed. Continue clipboard/group/numeric/modal cases and Dark before completion; no RB-05/staging/commit.

Planned additive RB-04 progress recovery snapshot (slice incomplete): `G:\Builder3D-Rust_snapshot_20261031-224017_RB-04-native-light-selection`. Root target/.cache and reparse entries excluded; exact receipt follows after verification.


### RB-04 Light selection verified additive snapshot

- Immutable snapshot: `G:\Builder3D-Rust_snapshot_20261031-224017_RB-04-native-light-selection`. Verified 2026-10-31T22:40:24-06:00; Robocopy exit 1.
- Source and snapshot: 1,982 files / 253,458,359 bytes each; relative-path differences 0; size/SHA-256 differences 0; reparse entries 0 in each tree. Root target/, .cache/ and reparse entries excluded. Source, Git metadata, tests, documentation and evidence included.
- This is an interim RB-04 recovery point, not slice completion. Native transforms and all74 expanded reference scene results are verified; corresponding Rust scene replay is next. Windows is unlocked. Continue the remaining full RB-04 native transform/scene/focus evidence before RB-05.
- Pre-receipt recovery-critical SHA-256 values:
  - `crates/modeler3d-core/src/placement.rs`: `87c767a0e97adf72e422a340f92cb00c4a6194361128261395b4377e1218b8a6`
  - `crates/modeler3d-app/src/transform_gesture.rs`: `128d0a8f5033ce4df1875d5c1dc84c63731650ae8f1d5b902b2d3d54f510e3bc`
  - `crates/modeler3d-app/src/gesture_tests.rs`: `fd4f0e70b65dcc9c1c0aaa45e8479e7d3372b9738366395d7172544237e2dc93`
  - `crates/modeler3d-app/src/recent_preview.rs`: `5d4afde3bc674011a77f50ff95119de000563bf3cb1ab0a7d22b8c0223166051`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native-settle-verification.json`: `6aeaf2e2c80392ccfd60869b54696db8940a1d34cf49f4dd945998a7b0005a2a`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/python-settle-results.json`: `b4d32acb5cf8af18bb458db7dd52e0865b02301deb3b342b64df59715c8c3e96`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native_desktop_guard.py`: `ab64c435ba857d1c415dd3eb56292f0600b26a66cd8c99fc91d23eaff8dd1ad0`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native-rust-workflow.ps1`: `4b60949583751b0b4ab0499e25fe249b4b20962444864739fc28fdec3d30526a`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/rb03-preserved-status-check.json`: `6d35b75b8a9f952b12c07a80e7e0604cd685614b38d441e35c37357e4f3bd62a`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/numeric-selection.md`: `e7a1f4bd2b897ebd72f92a0d627fd9df91597ef838966de34bc78bc80f2585ad`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/numeric-selection-oracle.json`: `13f72c05c129a3f8bf4005c38f70f10c617a17009ac4c60739219abe5e114852`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/placement-oracle.json`: `4da2aa0e86abff0610caa45171f952d4d95ac44b11e869341a25e52ba368ad66`
  - `crates/modeler3d-viewport/src/scene.rs`: `68cd984b002793be27e19ad62596ae8c25761fcda6dc362ca0a46b430d7eac0d`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/gesture-oracle.json`: `a093e6129c0ea8db88e167a0fafd8629c92ba63544ded89574e47355b57fd843`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/viewport-transforms.md`: `f34082f746b68512e9393889e807292638d848063af0d317e3d174bc1f90b3d6`
  - `scripts/Initialize-BuildClang.ps1`: `73856cc160105f1a9c00bf845b14e29a3002dfdf6ba3efc5aed2742ed43a3d3f`
  - `docs/building.md`: `750a226f595466b2e5fc15f4fb453fe284c4c7f199dd9632e3e90233ec44495a`
  - `packaging/licenses/QHULL-COPYING.txt`: `106d55c931fd6a84822e5345d900273d059f1c27310d02567ccb313c5d18c55d`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/settle-fix.md`: `63a903bbb24d71770f971142e33fa8d063a3ad5155520c91fd442bd4a0c51fc1`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/report.md`: `42089fb0f16d830925835315515505e899c029a91d864329fc183f5e7380e509`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/checkpoint.md`: `b23852a5ed2484f345dd8ed1afdbbaf27c0c5b7de60654007a598b139f0fcd5a`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/verification.md`: `1f8cec516f1655e3bbe9f869e9e4ce1196412bc43c5e698e1f10ca1e308405b7`
  - `crates/modeler3d-app/src/rotation.rs`: `dc9c80002d7aa507e88d48fab024158b4cb3eff0638c6f179f6fcc450aba5484`
  - `crates/modeler3d-app/src/transforms.rs`: `dcbe4eebf97af53ba391c9a5d9070713a7737a9047b5d5f7089876de4a721861`
  - `AGENTS.md`: `986ba68a9e458b5c5bc9a7c9122e4a741d3dfc5f35228b431a2bc974c7f45a1b`
  - `Cargo.toml`: `d51c5bfb69d2e0cdf88fc252d58baafc54c99a926bef7e84074a44b8154e98ba`
  - `Cargo.lock`: `9cb2d94bb7ceb84f614228b7e79b060e08e46a10253573ca424f70dbf7459f2a`
  - `WORK_LOG.md`: `89777a162c6241f49decf446f619e54fabf711f841dda5d97c9ef75fda1c067d`
  - `.git/HEAD`: `28d25bf82af4c0e2b72f50959b2beb859e3e60b9630a5e8c603dad4ddb2b6e80`
  - `docs/rb-03-lifecycle-and-file-completion-handoff.md`: `5d166c80c6a9a8ab4d3085a9d4ff872cc98efc7b6469ffe0583d48eedb6ff222`
  - `evidence/rebuild/parity-matrix.toml`: `88af89d940f22db9e8a939f2c93e3c34aac864257e6fbf73f3a3c0cb33df6384`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/checkpoint.md`: `e7b4cfbe204a37bf4d878ec097b159d5e9e89a66a2464b79216b541db25964cf`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/requirement-test-map.md`: `e03765801daa74cc500f5948c15065de0e013e7398491c2f18f325cc71c875da`
  - `crates/modeler3d-core/src/project.rs`: `cbac8d8d250d1532891fc6e170a34e4cd519fcbe2c8d58e8ae66a25a43eddff7`
  - `crates/modeler3d-core/tests/project_golden.rs`: `28577b83dbe38767c3bd2722bc13a74932335d25b593ef5a1b4cd90b22752dea`
  - `crates/modeler3d-app/src/commands.rs`: `f095c3fd5f92c2858341fdff111cdde83f544c7cd808fb86556b2511f07aab9f`
  - `crates/modeler3d-app/src/dialogs.rs`: `bd019a0d41dacc068c4a6b103c327e92fa37d6660f89b014728a22770289eac6`
  - `crates/modeler3d-app/src/shell.rs`: `9b272d4c76385b45ce24220f540dbbbfff9134867b729e9b28ef1819f419e350`

The receipt postdates the immutable snapshot. Only WORK_LOG.md and the RB-04 checkpoint are expected to differ afterward.

Post-receipt comparison: 1982 included paths in each tree; no missing or extra paths; exactly WORK_LOG.md and the RB-04 checkpoint differ from their post-copy receipts; no other size/SHA-256 differences; reparse entries 0 in each.

RB-04 native clipboard defect found: C5 CtrlV was filtered by egui-winit before scene handler. Added native key delivery plus focused TextEdit clipboard forwarding, enabled pinned eframe OS clipboard feature. New regression passed; final-feature full workspace checks/build running. C5 release/gates are superseded for current source; no acceptance claim. Source and Rust GUIs closed. Continue fixed native replay and remaining full scene matrix, no RB-05/staging/commit.

RB-04 native clipboard repair verified on current f091 debug: real Copy/change-selection/Paste, exact geometry/IDs/history/undo/redo.260 tests/fmt/check/strict Clippy/deny/metadata/build pass. Pinned clipboard feature and Boost notices reviewed/staged; current release/size refresh still required. Rust7592 open/saved, Python closed; continue all remaining Light/Dark scene/native cases. No RB-05/staging/commit/review acceptance.

Planned additive RB-04 progress recovery snapshot (slice incomplete): `G:\Builder3D-Rust_snapshot_20261031-225406_RB-04-native-clipboard-fix`. Root target/.cache and reparse entries excluded; exact receipt follows after verification.


### RB-04 native clipboard fix verified additive snapshot

- Immutable snapshot: `G:\Builder3D-Rust_snapshot_20261031-225406_RB-04-native-clipboard-fix`. Verified 2026-10-31T22:54:13-06:00; Robocopy exit 1.
- Source and snapshot: 2,030 files / 256,937,298 bytes each; relative-path differences 0; size/SHA-256 differences 0; reparse entries 0 in each tree. Root target/, .cache/ and reparse entries excluded. Source, Git metadata, tests, documentation and evidence included.
- This is an interim RB-04 recovery point, not slice completion. Native transforms and all74 expanded reference scene results are verified; corresponding Rust scene replay is next. Windows is unlocked. Continue the remaining full RB-04 native transform/scene/focus evidence before RB-05.
- Pre-receipt recovery-critical SHA-256 values:
  - `crates/modeler3d-app/src/lib.rs`: `c2befb366c9464a279ffa4c35cddef7b9679fc84b3dcd7b2c61460f0d00e48b9`
  - `crates/modeler3d-app/src/packaging.rs`: `8b92ce7fe3fb6964ec95a95ef17f3ed6c8d58435452f7b3960d1cfdb85750768`
  - `packaging/licenses/BOOST-LICENSE-1.0.txt`: `c9bff75738922193e67fa726fa225535870d2aa1059f91452c411736284ad566`
  - `packaging/licenses/CLIPBOARD-NOTICE.txt`: `3ff9c3d1a7b57637e92d9442a032953bc9b0d17357fb892ee6b85b2ffcabf84b`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/clipboard-native-fix.md`: `72ebb969905bb73675ad9d8f253b6038b6afabcaa4ce2f36bf7b764487e990ff`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/clipboard-complete-gate-results.json`: `fdca31492a10e7ae2bd633f226c8dd395b01ca9828cdfa8dcf8159b7b10361e1`
  - `crates/modeler3d-core/src/placement.rs`: `87c767a0e97adf72e422a340f92cb00c4a6194361128261395b4377e1218b8a6`
  - `crates/modeler3d-app/src/transform_gesture.rs`: `128d0a8f5033ce4df1875d5c1dc84c63731650ae8f1d5b902b2d3d54f510e3bc`
  - `crates/modeler3d-app/src/gesture_tests.rs`: `fd4f0e70b65dcc9c1c0aaa45e8479e7d3372b9738366395d7172544237e2dc93`
  - `crates/modeler3d-app/src/recent_preview.rs`: `5d4afde3bc674011a77f50ff95119de000563bf3cb1ab0a7d22b8c0223166051`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native-settle-verification.json`: `6aeaf2e2c80392ccfd60869b54696db8940a1d34cf49f4dd945998a7b0005a2a`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/python-settle-results.json`: `b4d32acb5cf8af18bb458db7dd52e0865b02301deb3b342b64df59715c8c3e96`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native_desktop_guard.py`: `ab64c435ba857d1c415dd3eb56292f0600b26a66cd8c99fc91d23eaff8dd1ad0`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native-rust-workflow.ps1`: `4b60949583751b0b4ab0499e25fe249b4b20962444864739fc28fdec3d30526a`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/rb03-preserved-status-check.json`: `6d35b75b8a9f952b12c07a80e7e0604cd685614b38d441e35c37357e4f3bd62a`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/numeric-selection.md`: `e7a1f4bd2b897ebd72f92a0d627fd9df91597ef838966de34bc78bc80f2585ad`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/numeric-selection-oracle.json`: `13f72c05c129a3f8bf4005c38f70f10c617a17009ac4c60739219abe5e114852`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/placement-oracle.json`: `4da2aa0e86abff0610caa45171f952d4d95ac44b11e869341a25e52ba368ad66`
  - `crates/modeler3d-viewport/src/scene.rs`: `68cd984b002793be27e19ad62596ae8c25761fcda6dc362ca0a46b430d7eac0d`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/gesture-oracle.json`: `a093e6129c0ea8db88e167a0fafd8629c92ba63544ded89574e47355b57fd843`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/viewport-transforms.md`: `f34082f746b68512e9393889e807292638d848063af0d317e3d174bc1f90b3d6`
  - `scripts/Initialize-BuildClang.ps1`: `73856cc160105f1a9c00bf845b14e29a3002dfdf6ba3efc5aed2742ed43a3d3f`
  - `docs/building.md`: `750a226f595466b2e5fc15f4fb453fe284c4c7f199dd9632e3e90233ec44495a`
  - `packaging/licenses/QHULL-COPYING.txt`: `106d55c931fd6a84822e5345d900273d059f1c27310d02567ccb313c5d18c55d`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/settle-fix.md`: `63a903bbb24d71770f971142e33fa8d063a3ad5155520c91fd442bd4a0c51fc1`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/report.md`: `42089fb0f16d830925835315515505e899c029a91d864329fc183f5e7380e509`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/checkpoint.md`: `2104bb6a030b9f725c2ef4a0d5a81f9701a6078a52bb000c9cc759ed61d36a1e`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/verification.md`: `1f8cec516f1655e3bbe9f869e9e4ce1196412bc43c5e698e1f10ca1e308405b7`
  - `crates/modeler3d-app/src/rotation.rs`: `dc9c80002d7aa507e88d48fab024158b4cb3eff0638c6f179f6fcc450aba5484`
  - `crates/modeler3d-app/src/transforms.rs`: `dcbe4eebf97af53ba391c9a5d9070713a7737a9047b5d5f7089876de4a721861`
  - `AGENTS.md`: `986ba68a9e458b5c5bc9a7c9122e4a741d3dfc5f35228b431a2bc974c7f45a1b`
  - `Cargo.toml`: `ee87e4c1e28a4b4a17af12d5cb352235d590d621eb01bdff2c4b0a02226b2c65`
  - `Cargo.lock`: `fc3b23fa69d04bfc9e349413d0d24bcc919e4a3706cab72a02940a9ce43f96fe`
  - `WORK_LOG.md`: `30646b91b2688948c9d23604f2e9a95939780a853694887fef824f81ef56bc57`
  - `.git/HEAD`: `28d25bf82af4c0e2b72f50959b2beb859e3e60b9630a5e8c603dad4ddb2b6e80`
  - `docs/rb-03-lifecycle-and-file-completion-handoff.md`: `5d166c80c6a9a8ab4d3085a9d4ff872cc98efc7b6469ffe0583d48eedb6ff222`
  - `evidence/rebuild/parity-matrix.toml`: `88af89d940f22db9e8a939f2c93e3c34aac864257e6fbf73f3a3c0cb33df6384`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/checkpoint.md`: `e7b4cfbe204a37bf4d878ec097b159d5e9e89a66a2464b79216b541db25964cf`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/requirement-test-map.md`: `e03765801daa74cc500f5948c15065de0e013e7398491c2f18f325cc71c875da`
  - `crates/modeler3d-core/src/project.rs`: `cbac8d8d250d1532891fc6e170a34e4cd519fcbe2c8d58e8ae66a25a43eddff7`
  - `crates/modeler3d-core/tests/project_golden.rs`: `28577b83dbe38767c3bd2722bc13a74932335d25b593ef5a1b4cd90b22752dea`
  - `crates/modeler3d-app/src/commands.rs`: `f095c3fd5f92c2858341fdff111cdde83f544c7cd808fb86556b2511f07aab9f`
  - `crates/modeler3d-app/src/dialogs.rs`: `bd019a0d41dacc068c4a6b103c327e92fa37d6660f89b014728a22770289eac6`
  - `crates/modeler3d-app/src/shell.rs`: `0313536ffad1613a84a1ed16a2b068ca525f2a5f27e8155e1fddcbd41936390f`

The receipt postdates the immutable snapshot. Only WORK_LOG.md and the RB-04 checkpoint are expected to differ afterward.

Post-receipt comparison: 2030 included paths in each tree; no missing or extra paths; exactly WORK_LOG.md and the RB-04 checkpoint differ from their post-copy receipts; no other size/SHA-256 differences; reparse entries 0 in each.

RB-04 all28 Light generic scene cases verified on f091/C5 applicable paths. Native Settings pointer probe confirmed background Duplicate mutates3->4 objects (revision102->103); keyboard guards alone insufficient. Retained evidence; stopped owned old-source release runner before final bloat, closed Rust. Next is scoped pointer-modal containment/Escape repair and native regression, then remaining46 scene cases and final gates/snapshot. No RB-05/staging/commit.

Planned additive RB-04 progress recovery snapshot (slice incomplete): `G:\Builder3D-Rust_snapshot_20261031-230517_RB-04-native-light-models`. Root target/.cache and reparse entries excluded; exact receipt follows after verification.


### RB-04 Light generic models verified additive snapshot

- Immutable snapshot: `G:\Builder3D-Rust_snapshot_20261031-230517_RB-04-native-light-models`. Verified 2026-10-31T23:05:24-06:00; Robocopy exit 1.
- Source and snapshot: 2,142 files / 274,240,850 bytes each; relative-path differences 0; size/SHA-256 differences 0; reparse entries 0 in each tree. Root target/, .cache/ and reparse entries excluded. Source, Git metadata, tests, documentation and evidence included.
- This is an interim RB-04 recovery point, not slice completion. Native transforms and all74 expanded reference scene results are verified; corresponding Rust scene replay is next. Windows is unlocked. Continue the remaining full RB-04 native transform/scene/focus evidence before RB-05.
- Pre-receipt recovery-critical SHA-256 values:
  - `crates/modeler3d-app/src/lib.rs`: `c2befb366c9464a279ffa4c35cddef7b9679fc84b3dcd7b2c61460f0d00e48b9`
  - `crates/modeler3d-app/src/packaging.rs`: `8b92ce7fe3fb6964ec95a95ef17f3ed6c8d58435452f7b3960d1cfdb85750768`
  - `packaging/licenses/BOOST-LICENSE-1.0.txt`: `c9bff75738922193e67fa726fa225535870d2aa1059f91452c411736284ad566`
  - `packaging/licenses/CLIPBOARD-NOTICE.txt`: `3ff9c3d1a7b57637e92d9442a032953bc9b0d17357fb892ee6b85b2ffcabf84b`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/clipboard-native-fix.md`: `72ebb969905bb73675ad9d8f253b6038b6afabcaa4ce2f36bf7b764487e990ff`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/clipboard-complete-gate-results.json`: `fdca31492a10e7ae2bd633f226c8dd395b01ca9828cdfa8dcf8159b7b10361e1`
  - `crates/modeler3d-core/src/placement.rs`: `87c767a0e97adf72e422a340f92cb00c4a6194361128261395b4377e1218b8a6`
  - `crates/modeler3d-app/src/transform_gesture.rs`: `128d0a8f5033ce4df1875d5c1dc84c63731650ae8f1d5b902b2d3d54f510e3bc`
  - `crates/modeler3d-app/src/gesture_tests.rs`: `fd4f0e70b65dcc9c1c0aaa45e8479e7d3372b9738366395d7172544237e2dc93`
  - `crates/modeler3d-app/src/recent_preview.rs`: `5d4afde3bc674011a77f50ff95119de000563bf3cb1ab0a7d22b8c0223166051`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native-settle-verification.json`: `6aeaf2e2c80392ccfd60869b54696db8940a1d34cf49f4dd945998a7b0005a2a`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/python-settle-results.json`: `b4d32acb5cf8af18bb458db7dd52e0865b02301deb3b342b64df59715c8c3e96`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native_desktop_guard.py`: `ab64c435ba857d1c415dd3eb56292f0600b26a66cd8c99fc91d23eaff8dd1ad0`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native-rust-workflow.ps1`: `4b60949583751b0b4ab0499e25fe249b4b20962444864739fc28fdec3d30526a`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/rb03-preserved-status-check.json`: `6d35b75b8a9f952b12c07a80e7e0604cd685614b38d441e35c37357e4f3bd62a`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/numeric-selection.md`: `e7a1f4bd2b897ebd72f92a0d627fd9df91597ef838966de34bc78bc80f2585ad`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/numeric-selection-oracle.json`: `13f72c05c129a3f8bf4005c38f70f10c617a17009ac4c60739219abe5e114852`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/placement-oracle.json`: `4da2aa0e86abff0610caa45171f952d4d95ac44b11e869341a25e52ba368ad66`
  - `crates/modeler3d-viewport/src/scene.rs`: `68cd984b002793be27e19ad62596ae8c25761fcda6dc362ca0a46b430d7eac0d`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/gesture-oracle.json`: `a093e6129c0ea8db88e167a0fafd8629c92ba63544ded89574e47355b57fd843`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/viewport-transforms.md`: `f34082f746b68512e9393889e807292638d848063af0d317e3d174bc1f90b3d6`
  - `scripts/Initialize-BuildClang.ps1`: `73856cc160105f1a9c00bf845b14e29a3002dfdf6ba3efc5aed2742ed43a3d3f`
  - `docs/building.md`: `750a226f595466b2e5fc15f4fb453fe284c4c7f199dd9632e3e90233ec44495a`
  - `packaging/licenses/QHULL-COPYING.txt`: `106d55c931fd6a84822e5345d900273d059f1c27310d02567ccb313c5d18c55d`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/settle-fix.md`: `63a903bbb24d71770f971142e33fa8d063a3ad5155520c91fd442bd4a0c51fc1`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/report.md`: `42089fb0f16d830925835315515505e899c029a91d864329fc183f5e7380e509`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/checkpoint.md`: `0a794a8cd27d8603a5218e23727ffc40ba6d8e45c0e7ad8c999c3b7eec47ebd2`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/verification.md`: `1f8cec516f1655e3bbe9f869e9e4ce1196412bc43c5e698e1f10ca1e308405b7`
  - `crates/modeler3d-app/src/rotation.rs`: `dc9c80002d7aa507e88d48fab024158b4cb3eff0638c6f179f6fcc450aba5484`
  - `crates/modeler3d-app/src/transforms.rs`: `dcbe4eebf97af53ba391c9a5d9070713a7737a9047b5d5f7089876de4a721861`
  - `AGENTS.md`: `986ba68a9e458b5c5bc9a7c9122e4a741d3dfc5f35228b431a2bc974c7f45a1b`
  - `Cargo.toml`: `ee87e4c1e28a4b4a17af12d5cb352235d590d621eb01bdff2c4b0a02226b2c65`
  - `Cargo.lock`: `fc3b23fa69d04bfc9e349413d0d24bcc919e4a3706cab72a02940a9ce43f96fe`
  - `WORK_LOG.md`: `dc251019874e90b45881be7228eb2fa2ad9f056ded4b4c8fe272b5b91957552c`
  - `.git/HEAD`: `28d25bf82af4c0e2b72f50959b2beb859e3e60b9630a5e8c603dad4ddb2b6e80`
  - `docs/rb-03-lifecycle-and-file-completion-handoff.md`: `5d166c80c6a9a8ab4d3085a9d4ff872cc98efc7b6469ffe0583d48eedb6ff222`
  - `evidence/rebuild/parity-matrix.toml`: `88af89d940f22db9e8a939f2c93e3c34aac864257e6fbf73f3a3c0cb33df6384`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/checkpoint.md`: `e7b4cfbe204a37bf4d878ec097b159d5e9e89a66a2464b79216b541db25964cf`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/requirement-test-map.md`: `e03765801daa74cc500f5948c15065de0e013e7398491c2f18f325cc71c875da`
  - `crates/modeler3d-core/src/project.rs`: `cbac8d8d250d1532891fc6e170a34e4cd519fcbe2c8d58e8ae66a25a43eddff7`
  - `crates/modeler3d-core/tests/project_golden.rs`: `28577b83dbe38767c3bd2722bc13a74932335d25b593ef5a1b4cd90b22752dea`
  - `crates/modeler3d-app/src/commands.rs`: `f095c3fd5f92c2858341fdff111cdde83f544c7cd808fb86556b2511f07aab9f`
  - `crates/modeler3d-app/src/dialogs.rs`: `bd019a0d41dacc068c4a6b103c327e92fa37d6660f89b014728a22770289eac6`
  - `crates/modeler3d-app/src/shell.rs`: `0313536ffad1613a84a1ed16a2b068ca525f2a5f27e8155e1fddcbd41936390f`

The receipt postdates the immutable snapshot. Only WORK_LOG.md and the RB-04 checkpoint are expected to differ afterward.

Post-receipt comparison: 2142 included paths in each tree; no missing or extra paths; exactly WORK_LOG.md and the RB-04 checkpoint differ from their post-copy receipts; no other size/SHA-256 differences; reparse entries 0 in each.

RB-04 modal pointer containment/Escape repair verified:261 tests and strict gates0; native exact unchanged scene under Settings, positive Duplicate after close. Current debug82fd..., no core transform math change. All28 Light generic cases verified;9 Light manual +37 Dark remain. Current release refresh pending; continuefullnative/evidence/final additive snapshot before RB-05.

Planned additive RB-04 progress recovery snapshot (slice incomplete): `G:\Builder3D-Rust_snapshot_20261031-231526_RB-04-native-modal-fix`. Root target/.cache and reparse entries excluded; exact receipt follows after verification.


### RB-04 modal pointer fix verified additive snapshot

- Immutable snapshot: `G:\Builder3D-Rust_snapshot_20261031-231526_RB-04-native-modal-fix`. Verified 2026-10-31T23:15:33-06:00; Robocopy exit 1.
- Source and snapshot: 2,158 files / 275,488,838 bytes each; relative-path differences 0; size/SHA-256 differences 0; reparse entries 0 in each tree. Root target/, .cache/ and reparse entries excluded. Source, Git metadata, tests, documentation and evidence included.
- This is an interim RB-04 recovery point, not slice completion. Native transforms and all74 expanded reference scene results are verified; corresponding Rust scene replay is next. Windows is unlocked. Continue the remaining full RB-04 native transform/scene/focus evidence before RB-05.
- Pre-receipt recovery-critical SHA-256 values:
  - `crates/modeler3d-app/src/lib.rs`: `c2befb366c9464a279ffa4c35cddef7b9679fc84b3dcd7b2c61460f0d00e48b9`
  - `crates/modeler3d-app/src/packaging.rs`: `8b92ce7fe3fb6964ec95a95ef17f3ed6c8d58435452f7b3960d1cfdb85750768`
  - `packaging/licenses/BOOST-LICENSE-1.0.txt`: `c9bff75738922193e67fa726fa225535870d2aa1059f91452c411736284ad566`
  - `packaging/licenses/CLIPBOARD-NOTICE.txt`: `3ff9c3d1a7b57637e92d9442a032953bc9b0d17357fb892ee6b85b2ffcabf84b`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/clipboard-native-fix.md`: `72ebb969905bb73675ad9d8f253b6038b6afabcaa4ce2f36bf7b764487e990ff`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/clipboard-complete-gate-results.json`: `fdca31492a10e7ae2bd633f226c8dd395b01ca9828cdfa8dcf8159b7b10361e1`
  - `crates/modeler3d-core/src/placement.rs`: `87c767a0e97adf72e422a340f92cb00c4a6194361128261395b4377e1218b8a6`
  - `crates/modeler3d-app/src/transform_gesture.rs`: `128d0a8f5033ce4df1875d5c1dc84c63731650ae8f1d5b902b2d3d54f510e3bc`
  - `crates/modeler3d-app/src/gesture_tests.rs`: `fd4f0e70b65dcc9c1c0aaa45e8479e7d3372b9738366395d7172544237e2dc93`
  - `crates/modeler3d-app/src/recent_preview.rs`: `5d4afde3bc674011a77f50ff95119de000563bf3cb1ab0a7d22b8c0223166051`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native-settle-verification.json`: `6aeaf2e2c80392ccfd60869b54696db8940a1d34cf49f4dd945998a7b0005a2a`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/python-settle-results.json`: `b4d32acb5cf8af18bb458db7dd52e0865b02301deb3b342b64df59715c8c3e96`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native_desktop_guard.py`: `ab64c435ba857d1c415dd3eb56292f0600b26a66cd8c99fc91d23eaff8dd1ad0`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native-rust-workflow.ps1`: `73145ceceebf934e9704bd9bee45d37ae6ea7f4a68c43003d9857d6ac34a0408`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/rb03-preserved-status-check.json`: `6d35b75b8a9f952b12c07a80e7e0604cd685614b38d441e35c37357e4f3bd62a`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/numeric-selection.md`: `e7a1f4bd2b897ebd72f92a0d627fd9df91597ef838966de34bc78bc80f2585ad`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/numeric-selection-oracle.json`: `13f72c05c129a3f8bf4005c38f70f10c617a17009ac4c60739219abe5e114852`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/placement-oracle.json`: `4da2aa0e86abff0610caa45171f952d4d95ac44b11e869341a25e52ba368ad66`
  - `crates/modeler3d-viewport/src/scene.rs`: `68cd984b002793be27e19ad62596ae8c25761fcda6dc362ca0a46b430d7eac0d`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/gesture-oracle.json`: `a093e6129c0ea8db88e167a0fafd8629c92ba63544ded89574e47355b57fd843`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/viewport-transforms.md`: `f34082f746b68512e9393889e807292638d848063af0d317e3d174bc1f90b3d6`
  - `scripts/Initialize-BuildClang.ps1`: `73856cc160105f1a9c00bf845b14e29a3002dfdf6ba3efc5aed2742ed43a3d3f`
  - `docs/building.md`: `750a226f595466b2e5fc15f4fb453fe284c4c7f199dd9632e3e90233ec44495a`
  - `packaging/licenses/QHULL-COPYING.txt`: `106d55c931fd6a84822e5345d900273d059f1c27310d02567ccb313c5d18c55d`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/settle-fix.md`: `63a903bbb24d71770f971142e33fa8d063a3ad5155520c91fd442bd4a0c51fc1`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/report.md`: `42089fb0f16d830925835315515505e899c029a91d864329fc183f5e7380e509`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/checkpoint.md`: `0c48b46eaef8154a802744478efe123eceea5161e32d73702b587600bed35a5b`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/verification.md`: `1f8cec516f1655e3bbe9f869e9e4ce1196412bc43c5e698e1f10ca1e308405b7`
  - `crates/modeler3d-app/src/rotation.rs`: `dc9c80002d7aa507e88d48fab024158b4cb3eff0638c6f179f6fcc450aba5484`
  - `crates/modeler3d-app/src/transforms.rs`: `dcbe4eebf97af53ba391c9a5d9070713a7737a9047b5d5f7089876de4a721861`
  - `AGENTS.md`: `986ba68a9e458b5c5bc9a7c9122e4a741d3dfc5f35228b431a2bc974c7f45a1b`
  - `Cargo.toml`: `ee87e4c1e28a4b4a17af12d5cb352235d590d621eb01bdff2c4b0a02226b2c65`
  - `Cargo.lock`: `fc3b23fa69d04bfc9e349413d0d24bcc919e4a3706cab72a02940a9ce43f96fe`
  - `WORK_LOG.md`: `8a2842e723ec1d19ca9c8accade2009bb0b203a15ba6c44ea16652a0e1e6ea2b`
  - `.git/HEAD`: `28d25bf82af4c0e2b72f50959b2beb859e3e60b9630a5e8c603dad4ddb2b6e80`
  - `docs/rb-03-lifecycle-and-file-completion-handoff.md`: `5d166c80c6a9a8ab4d3085a9d4ff872cc98efc7b6469ffe0583d48eedb6ff222`
  - `evidence/rebuild/parity-matrix.toml`: `88af89d940f22db9e8a939f2c93e3c34aac864257e6fbf73f3a3c0cb33df6384`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/checkpoint.md`: `e7b4cfbe204a37bf4d878ec097b159d5e9e89a66a2464b79216b541db25964cf`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/requirement-test-map.md`: `e03765801daa74cc500f5948c15065de0e013e7398491c2f18f325cc71c875da`
  - `crates/modeler3d-core/src/project.rs`: `cbac8d8d250d1532891fc6e170a34e4cd519fcbe2c8d58e8ae66a25a43eddff7`
  - `crates/modeler3d-core/tests/project_golden.rs`: `28577b83dbe38767c3bd2722bc13a74932335d25b593ef5a1b4cd90b22752dea`
  - `crates/modeler3d-app/src/commands.rs`: `f095c3fd5f92c2858341fdff111cdde83f544c7cd808fb86556b2511f07aab9f`
  - `crates/modeler3d-app/src/dialogs.rs`: `b6768aa9fe967a138bb1595fd4eb6413b8dfca867c2377dc19f2d070c18575f4`
  - `crates/modeler3d-app/src/shell.rs`: `db3dab8be4efd8dfda71b616bbc146e30b90f4ed98b38ba916b997188db9652c`

The receipt postdates the immutable snapshot. Only WORK_LOG.md and the RB-04 checkpoint are expected to differ afterward.

Post-receipt comparison: 2158 included paths in each tree; no missing or extra paths; exactly WORK_LOG.md and the RB-04 checkpoint differ from their post-copy receipts; no other size/SHA-256 differences; reparse entries 0 in each.

Planned additive RB-04 progress recovery snapshot (slice incomplete): `G:\Builder3D-Rust_snapshot_20261031-233512_RB-04-native-light-complete`. Root target/.cache and reparse entries excluded; exact receipt follows after verification.


### RB-04 modal pointer fix verified additive snapshot

- Immutable snapshot: `G:\Builder3D-Rust_snapshot_20261031-233512_RB-04-native-light-complete`. Verified 2026-10-31T23:35:19-06:00; Robocopy exit 1.
- Source and snapshot: 2,217 files / 285,213,539 bytes each; relative-path differences 0; size/SHA-256 differences 0; reparse entries 0 in each tree. Root target/, .cache/ and reparse entries excluded. Source, Git metadata, tests, documentation and evidence included.
- This is an interim RB-04 recovery point, not slice completion. Native transforms and all74 expanded reference scene results are verified; corresponding Rust scene replay is next. Windows is unlocked. Continue the remaining full RB-04 native transform/scene/focus evidence before RB-05.
- Pre-receipt recovery-critical SHA-256 values:
  - `crates/modeler3d-app/src/lib.rs`: `c2befb366c9464a279ffa4c35cddef7b9679fc84b3dcd7b2c61460f0d00e48b9`
  - `crates/modeler3d-app/src/packaging.rs`: `8b92ce7fe3fb6964ec95a95ef17f3ed6c8d58435452f7b3960d1cfdb85750768`
  - `packaging/licenses/BOOST-LICENSE-1.0.txt`: `c9bff75738922193e67fa726fa225535870d2aa1059f91452c411736284ad566`
  - `packaging/licenses/CLIPBOARD-NOTICE.txt`: `3ff9c3d1a7b57637e92d9442a032953bc9b0d17357fb892ee6b85b2ffcabf84b`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/clipboard-native-fix.md`: `72ebb969905bb73675ad9d8f253b6038b6afabcaa4ce2f36bf7b764487e990ff`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/clipboard-complete-gate-results.json`: `fdca31492a10e7ae2bd633f226c8dd395b01ca9828cdfa8dcf8159b7b10361e1`
  - `crates/modeler3d-core/src/placement.rs`: `87c767a0e97adf72e422a340f92cb00c4a6194361128261395b4377e1218b8a6`
  - `crates/modeler3d-app/src/transform_gesture.rs`: `128d0a8f5033ce4df1875d5c1dc84c63731650ae8f1d5b902b2d3d54f510e3bc`
  - `crates/modeler3d-app/src/gesture_tests.rs`: `fd4f0e70b65dcc9c1c0aaa45e8479e7d3372b9738366395d7172544237e2dc93`
  - `crates/modeler3d-app/src/recent_preview.rs`: `5d4afde3bc674011a77f50ff95119de000563bf3cb1ab0a7d22b8c0223166051`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native-settle-verification.json`: `6aeaf2e2c80392ccfd60869b54696db8940a1d34cf49f4dd945998a7b0005a2a`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/python-settle-results.json`: `b4d32acb5cf8af18bb458db7dd52e0865b02301deb3b342b64df59715c8c3e96`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native_desktop_guard.py`: `ab64c435ba857d1c415dd3eb56292f0600b26a66cd8c99fc91d23eaff8dd1ad0`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native-rust-workflow.ps1`: `dbb0b2e19a33d553bec18ca6bdead187dc60b3105925181c62a5508552efd570`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/rb03-preserved-status-check.json`: `6d35b75b8a9f952b12c07a80e7e0604cd685614b38d441e35c37357e4f3bd62a`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/numeric-selection.md`: `e7a1f4bd2b897ebd72f92a0d627fd9df91597ef838966de34bc78bc80f2585ad`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/numeric-selection-oracle.json`: `13f72c05c129a3f8bf4005c38f70f10c617a17009ac4c60739219abe5e114852`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/placement-oracle.json`: `4da2aa0e86abff0610caa45171f952d4d95ac44b11e869341a25e52ba368ad66`
  - `crates/modeler3d-viewport/src/scene.rs`: `68cd984b002793be27e19ad62596ae8c25761fcda6dc362ca0a46b430d7eac0d`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/gesture-oracle.json`: `a093e6129c0ea8db88e167a0fafd8629c92ba63544ded89574e47355b57fd843`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/viewport-transforms.md`: `f34082f746b68512e9393889e807292638d848063af0d317e3d174bc1f90b3d6`
  - `scripts/Initialize-BuildClang.ps1`: `73856cc160105f1a9c00bf845b14e29a3002dfdf6ba3efc5aed2742ed43a3d3f`
  - `docs/building.md`: `750a226f595466b2e5fc15f4fb453fe284c4c7f199dd9632e3e90233ec44495a`
  - `packaging/licenses/QHULL-COPYING.txt`: `106d55c931fd6a84822e5345d900273d059f1c27310d02567ccb313c5d18c55d`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/settle-fix.md`: `63a903bbb24d71770f971142e33fa8d063a3ad5155520c91fd442bd4a0c51fc1`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/report.md`: `42089fb0f16d830925835315515505e899c029a91d864329fc183f5e7380e509`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/checkpoint.md`: `23e66da95aa6d88d7421df65c6d8e135afa7c7626655b8b4458650e2e07b8b5e`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/verification.md`: `1f8cec516f1655e3bbe9f869e9e4ce1196412bc43c5e698e1f10ca1e308405b7`
  - `crates/modeler3d-app/src/rotation.rs`: `dc9c80002d7aa507e88d48fab024158b4cb3eff0638c6f179f6fcc450aba5484`
  - `crates/modeler3d-app/src/transforms.rs`: `dcbe4eebf97af53ba391c9a5d9070713a7737a9047b5d5f7089876de4a721861`
  - `AGENTS.md`: `986ba68a9e458b5c5bc9a7c9122e4a741d3dfc5f35228b431a2bc974c7f45a1b`
  - `Cargo.toml`: `ee87e4c1e28a4b4a17af12d5cb352235d590d621eb01bdff2c4b0a02226b2c65`
  - `Cargo.lock`: `fc3b23fa69d04bfc9e349413d0d24bcc919e4a3706cab72a02940a9ce43f96fe`
  - `WORK_LOG.md`: `0ed685109c0e4af5e34ca482cf57a6b81ea707850026bd38338ad0e04ff870c1`
  - `.git/HEAD`: `28d25bf82af4c0e2b72f50959b2beb859e3e60b9630a5e8c603dad4ddb2b6e80`
  - `docs/rb-03-lifecycle-and-file-completion-handoff.md`: `5d166c80c6a9a8ab4d3085a9d4ff872cc98efc7b6469ffe0583d48eedb6ff222`
  - `evidence/rebuild/parity-matrix.toml`: `88af89d940f22db9e8a939f2c93e3c34aac864257e6fbf73f3a3c0cb33df6384`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/checkpoint.md`: `e7b4cfbe204a37bf4d878ec097b159d5e9e89a66a2464b79216b541db25964cf`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/requirement-test-map.md`: `e03765801daa74cc500f5948c15065de0e013e7398491c2f18f325cc71c875da`
  - `crates/modeler3d-core/src/project.rs`: `cbac8d8d250d1532891fc6e170a34e4cd519fcbe2c8d58e8ae66a25a43eddff7`
  - `crates/modeler3d-core/tests/project_golden.rs`: `28577b83dbe38767c3bd2722bc13a74932335d25b593ef5a1b4cd90b22752dea`
  - `crates/modeler3d-app/src/commands.rs`: `f095c3fd5f92c2858341fdff111cdde83f544c7cd808fb86556b2511f07aab9f`
  - `crates/modeler3d-app/src/dialogs.rs`: `b6768aa9fe967a138bb1595fd4eb6413b8dfca867c2377dc19f2d070c18575f4`
  - `crates/modeler3d-app/src/shell.rs`: `db3dab8be4efd8dfda71b616bbc146e30b90f4ed98b38ba916b997188db9652c`

The receipt postdates the immutable snapshot. Only WORK_LOG.md and the RB-04 checkpoint are expected to differ afterward.

Post-receipt comparison: 2217 included paths in each tree; no missing or extra paths; exactly WORK_LOG.md and the RB-04 checkpoint differ from their post-copy receipts; no other size/SHA-256 differences; reparse entries 0 in each.

Planned additive RB-04 progress recovery snapshot (slice incomplete): `G:\Builder3D-Rust_snapshot_20261031-235040_RB-04-native-dark-generic`. Root target/.cache and reparse entries excluded; exact receipt follows after verification.


### RB-04 Dark generic replay verified additive snapshot

- Immutable snapshot: `G:\Builder3D-Rust_snapshot_20261031-235040_RB-04-native-dark-generic`. Verified 2026-10-31T23:50:48-06:00; Robocopy exit 1.
- Source and snapshot: 2,390 files / 319,276,341 bytes each; relative-path differences 0; size/SHA-256 differences 0; reparse entries 0 in each tree. Root target/, .cache/ and reparse entries excluded. Source, Git metadata, tests, documentation and evidence included.
- This is an interim RB-04 recovery point, not slice completion. Native transforms and all74 expanded reference scene results are verified; Rust Light37 and Dark generic28 cases pass; Dark manual and pointer-batch investigation are next. Windows is unlocked. Continue the remaining full RB-04 native transform/scene/focus evidence before RB-05.
- Pre-receipt recovery-critical SHA-256 values:
  - `crates/modeler3d-app/src/lib.rs`: `c2befb366c9464a279ffa4c35cddef7b9679fc84b3dcd7b2c61460f0d00e48b9`
  - `crates/modeler3d-app/src/packaging.rs`: `8b92ce7fe3fb6964ec95a95ef17f3ed6c8d58435452f7b3960d1cfdb85750768`
  - `packaging/licenses/BOOST-LICENSE-1.0.txt`: `c9bff75738922193e67fa726fa225535870d2aa1059f91452c411736284ad566`
  - `packaging/licenses/CLIPBOARD-NOTICE.txt`: `3ff9c3d1a7b57637e92d9442a032953bc9b0d17357fb892ee6b85b2ffcabf84b`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/clipboard-native-fix.md`: `72ebb969905bb73675ad9d8f253b6038b6afabcaa4ce2f36bf7b764487e990ff`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/clipboard-complete-gate-results.json`: `fdca31492a10e7ae2bd633f226c8dd395b01ca9828cdfa8dcf8159b7b10361e1`
  - `crates/modeler3d-core/src/placement.rs`: `87c767a0e97adf72e422a340f92cb00c4a6194361128261395b4377e1218b8a6`
  - `crates/modeler3d-app/src/transform_gesture.rs`: `128d0a8f5033ce4df1875d5c1dc84c63731650ae8f1d5b902b2d3d54f510e3bc`
  - `crates/modeler3d-app/src/gesture_tests.rs`: `fd4f0e70b65dcc9c1c0aaa45e8479e7d3372b9738366395d7172544237e2dc93`
  - `crates/modeler3d-app/src/recent_preview.rs`: `5d4afde3bc674011a77f50ff95119de000563bf3cb1ab0a7d22b8c0223166051`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native-settle-verification.json`: `6aeaf2e2c80392ccfd60869b54696db8940a1d34cf49f4dd945998a7b0005a2a`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/python-settle-results.json`: `b4d32acb5cf8af18bb458db7dd52e0865b02301deb3b342b64df59715c8c3e96`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native_desktop_guard.py`: `ab64c435ba857d1c415dd3eb56292f0600b26a66cd8c99fc91d23eaff8dd1ad0`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native-rust-workflow.ps1`: `976a88454ffc11d9bf12ee6eb894a8deff6e10eb09f72960cdde081a5ca67021`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/rb03-preserved-status-check.json`: `6d35b75b8a9f952b12c07a80e7e0604cd685614b38d441e35c37357e4f3bd62a`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/numeric-selection.md`: `e7a1f4bd2b897ebd72f92a0d627fd9df91597ef838966de34bc78bc80f2585ad`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/numeric-selection-oracle.json`: `13f72c05c129a3f8bf4005c38f70f10c617a17009ac4c60739219abe5e114852`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/placement-oracle.json`: `4da2aa0e86abff0610caa45171f952d4d95ac44b11e869341a25e52ba368ad66`
  - `crates/modeler3d-viewport/src/scene.rs`: `68cd984b002793be27e19ad62596ae8c25761fcda6dc362ca0a46b430d7eac0d`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/gesture-oracle.json`: `a093e6129c0ea8db88e167a0fafd8629c92ba63544ded89574e47355b57fd843`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/viewport-transforms.md`: `f34082f746b68512e9393889e807292638d848063af0d317e3d174bc1f90b3d6`
  - `scripts/Initialize-BuildClang.ps1`: `73856cc160105f1a9c00bf845b14e29a3002dfdf6ba3efc5aed2742ed43a3d3f`
  - `docs/building.md`: `750a226f595466b2e5fc15f4fb453fe284c4c7f199dd9632e3e90233ec44495a`
  - `packaging/licenses/QHULL-COPYING.txt`: `106d55c931fd6a84822e5345d900273d059f1c27310d02567ccb313c5d18c55d`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/settle-fix.md`: `63a903bbb24d71770f971142e33fa8d063a3ad5155520c91fd442bd4a0c51fc1`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/report.md`: `42089fb0f16d830925835315515505e899c029a91d864329fc183f5e7380e509`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/checkpoint.md`: `323155afe4ba498bd824a5c966125cb5d2bf0f0636f8146abd38bc8c8057d951`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/verification.md`: `1f8cec516f1655e3bbe9f869e9e4ce1196412bc43c5e698e1f10ca1e308405b7`
  - `crates/modeler3d-app/src/rotation.rs`: `dc9c80002d7aa507e88d48fab024158b4cb3eff0638c6f179f6fcc450aba5484`
  - `crates/modeler3d-app/src/transforms.rs`: `dcbe4eebf97af53ba391c9a5d9070713a7737a9047b5d5f7089876de4a721861`
  - `AGENTS.md`: `986ba68a9e458b5c5bc9a7c9122e4a741d3dfc5f35228b431a2bc974c7f45a1b`
  - `Cargo.toml`: `ee87e4c1e28a4b4a17af12d5cb352235d590d621eb01bdff2c4b0a02226b2c65`
  - `Cargo.lock`: `fc3b23fa69d04bfc9e349413d0d24bcc919e4a3706cab72a02940a9ce43f96fe`
  - `WORK_LOG.md`: `7ec01f23765d7b4a119407c58c298f91cfea26eca14ea30a738134baed839701`
  - `.git/HEAD`: `28d25bf82af4c0e2b72f50959b2beb859e3e60b9630a5e8c603dad4ddb2b6e80`
  - `docs/rb-03-lifecycle-and-file-completion-handoff.md`: `5d166c80c6a9a8ab4d3085a9d4ff872cc98efc7b6469ffe0583d48eedb6ff222`
  - `evidence/rebuild/parity-matrix.toml`: `88af89d940f22db9e8a939f2c93e3c34aac864257e6fbf73f3a3c0cb33df6384`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/checkpoint.md`: `e7b4cfbe204a37bf4d878ec097b159d5e9e89a66a2464b79216b541db25964cf`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/requirement-test-map.md`: `e03765801daa74cc500f5948c15065de0e013e7398491c2f18f325cc71c875da`
  - `crates/modeler3d-core/src/project.rs`: `cbac8d8d250d1532891fc6e170a34e4cd519fcbe2c8d58e8ae66a25a43eddff7`
  - `crates/modeler3d-core/tests/project_golden.rs`: `28577b83dbe38767c3bd2722bc13a74932335d25b593ef5a1b4cd90b22752dea`
  - `crates/modeler3d-app/src/commands.rs`: `f095c3fd5f92c2858341fdff111cdde83f544c7cd808fb86556b2511f07aab9f`
  - `crates/modeler3d-app/src/dialogs.rs`: `b6768aa9fe967a138bb1595fd4eb6413b8dfca867c2377dc19f2d070c18575f4`
  - `crates/modeler3d-app/src/shell.rs`: `db3dab8be4efd8dfda71b616bbc146e30b90f4ed98b38ba916b997188db9652c`

The receipt postdates the immutable snapshot. Only WORK_LOG.md and the RB-04 checkpoint are expected to differ afterward.

Post-receipt comparison: 2390 included paths in each tree; no missing or extra paths; exactly WORK_LOG.md and the RB-04 checkpoint differ from their post-copy receipts; no other size/SHA-256 differences; reparse entries 0 in each.

Planned additive RB-04 progress recovery snapshot (slice incomplete): `G:\Builder3D-Rust_snapshot_20261101-000852_RB-04-native-evidence-verified`. Root target/.cache and reparse entries excluded; exact receipt follows after verification.


### RB-04 full native evidence verified additive snapshot

- Immutable snapshot: `G:\Builder3D-Rust_snapshot_20261101-000852_RB-04-native-evidence-verified`. Verified 2026-11-01T00:09:00-06:00; Robocopy exit 1.
- Source and snapshot: 2,492 files / 350,193,766 bytes each; relative-path differences 0; size/SHA-256 differences 0; reparse entries 0 in each tree. Root target/, .cache/ and reparse entries excluded. Source, Git metadata, tests, documentation and evidence included.
- This is an interim RB-04 recovery point, not slice completion. Native transforms and all74 expanded reference scene results are verified; all74 Rust scene/22viewport/4Settle/8pick states now verified; final visual inspection and completed-slice receipt are next. Windows is unlocked. Continue the remaining full RB-04 native transform/scene/focus evidence before RB-05.
- Pre-receipt recovery-critical SHA-256 values:
  - `crates/modeler3d-app/src/lib.rs`: `c2befb366c9464a279ffa4c35cddef7b9679fc84b3dcd7b2c61460f0d00e48b9`
  - `crates/modeler3d-app/src/packaging.rs`: `8b92ce7fe3fb6964ec95a95ef17f3ed6c8d58435452f7b3960d1cfdb85750768`
  - `packaging/licenses/BOOST-LICENSE-1.0.txt`: `c9bff75738922193e67fa726fa225535870d2aa1059f91452c411736284ad566`
  - `packaging/licenses/CLIPBOARD-NOTICE.txt`: `3ff9c3d1a7b57637e92d9442a032953bc9b0d17357fb892ee6b85b2ffcabf84b`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/clipboard-native-fix.md`: `72ebb969905bb73675ad9d8f253b6038b6afabcaa4ce2f36bf7b764487e990ff`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/clipboard-complete-gate-results.json`: `fdca31492a10e7ae2bd633f226c8dd395b01ca9828cdfa8dcf8159b7b10361e1`
  - `crates/modeler3d-core/src/placement.rs`: `87c767a0e97adf72e422a340f92cb00c4a6194361128261395b4377e1218b8a6`
  - `crates/modeler3d-app/src/transform_gesture.rs`: `128d0a8f5033ce4df1875d5c1dc84c63731650ae8f1d5b902b2d3d54f510e3bc`
  - `crates/modeler3d-app/src/gesture_tests.rs`: `fd4f0e70b65dcc9c1c0aaa45e8479e7d3372b9738366395d7172544237e2dc93`
  - `crates/modeler3d-app/src/recent_preview.rs`: `5d4afde3bc674011a77f50ff95119de000563bf3cb1ab0a7d22b8c0223166051`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native-settle-verification.json`: `6aeaf2e2c80392ccfd60869b54696db8940a1d34cf49f4dd945998a7b0005a2a`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/python-settle-results.json`: `b4d32acb5cf8af18bb458db7dd52e0865b02301deb3b342b64df59715c8c3e96`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native_desktop_guard.py`: `ab64c435ba857d1c415dd3eb56292f0600b26a66cd8c99fc91d23eaff8dd1ad0`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native-rust-workflow.ps1`: `886321d7e2f411e26933070d52b0f6ec2c44763909c7c6dbc81ef5457b23b61e`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/rb03-preserved-status-check.json`: `6d35b75b8a9f952b12c07a80e7e0604cd685614b38d441e35c37357e4f3bd62a`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/numeric-selection.md`: `393e44655f81755bf26eaf7949b6a7e3c6fa3b35836f5ae8b9b090c934dbe99b`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/numeric-selection-oracle.json`: `13f72c05c129a3f8bf4005c38f70f10c617a17009ac4c60739219abe5e114852`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/placement-oracle.json`: `4da2aa0e86abff0610caa45171f952d4d95ac44b11e869341a25e52ba368ad66`
  - `crates/modeler3d-viewport/src/scene.rs`: `68cd984b002793be27e19ad62596ae8c25761fcda6dc362ca0a46b430d7eac0d`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/gesture-oracle.json`: `a093e6129c0ea8db88e167a0fafd8629c92ba63544ded89574e47355b57fd843`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/viewport-transforms.md`: `0e2e7f1dd232a320153c4e51027d97ff99c3a36fa27b8df00b4e37a3e35cb0d0`
  - `scripts/Initialize-BuildClang.ps1`: `73856cc160105f1a9c00bf845b14e29a3002dfdf6ba3efc5aed2742ed43a3d3f`
  - `docs/building.md`: `750a226f595466b2e5fc15f4fb453fe284c4c7f199dd9632e3e90233ec44495a`
  - `packaging/licenses/QHULL-COPYING.txt`: `106d55c931fd6a84822e5345d900273d059f1c27310d02567ccb313c5d18c55d`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/settle-fix.md`: `6675af8640ca82626dbdfac8e664def2d4d4442edb803aa2199b7525af3ee2d4`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/report.md`: `8df99a70fc5eabbfa8084cf88a98875dc4f22895d02a2699ba03de90bac4ac6c`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/checkpoint.md`: `be0dd211868f4c227a9e8008de26dca4e7a87a6adf87c7a41e3934c794844cd1`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/verification.md`: `088711b3cdb2b7ae7d952bf61e565d14af599e545d3992baedffd0b5ccca3e90`
  - `crates/modeler3d-app/src/rotation.rs`: `dc9c80002d7aa507e88d48fab024158b4cb3eff0638c6f179f6fcc450aba5484`
  - `crates/modeler3d-app/src/transforms.rs`: `dcbe4eebf97af53ba391c9a5d9070713a7737a9047b5d5f7089876de4a721861`
  - `AGENTS.md`: `2253e20aed7056e2cae9af43c5cc5791fdd5235d2ba202ab7e12b9f6d441f763`
  - `Cargo.toml`: `ee87e4c1e28a4b4a17af12d5cb352235d590d621eb01bdff2c4b0a02226b2c65`
  - `Cargo.lock`: `fc3b23fa69d04bfc9e349413d0d24bcc919e4a3706cab72a02940a9ce43f96fe`
  - `WORK_LOG.md`: `e1cef30c51336e82344ed05a5e79058b5e37c7d71fd0814857d7a87348fd95f4`
  - `.git/HEAD`: `28d25bf82af4c0e2b72f50959b2beb859e3e60b9630a5e8c603dad4ddb2b6e80`
  - `docs/rb-03-lifecycle-and-file-completion-handoff.md`: `5d166c80c6a9a8ab4d3085a9d4ff872cc98efc7b6469ffe0583d48eedb6ff222`
  - `evidence/rebuild/parity-matrix.toml`: `df6a92058c77cf9480cd749fdd63aa2491d2f5d795004846dcbf5104fcc20a16`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/checkpoint.md`: `e7b4cfbe204a37bf4d878ec097b159d5e9e89a66a2464b79216b541db25964cf`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/requirement-test-map.md`: `e03765801daa74cc500f5948c15065de0e013e7398491c2f18f325cc71c875da`
  - `crates/modeler3d-core/src/project.rs`: `cbac8d8d250d1532891fc6e170a34e4cd519fcbe2c8d58e8ae66a25a43eddff7`
  - `crates/modeler3d-core/tests/project_golden.rs`: `28577b83dbe38767c3bd2722bc13a74932335d25b593ef5a1b4cd90b22752dea`
  - `crates/modeler3d-app/src/commands.rs`: `f095c3fd5f92c2858341fdff111cdde83f544c7cd808fb86556b2511f07aab9f`
  - `crates/modeler3d-app/src/dialogs.rs`: `b6768aa9fe967a138bb1595fd4eb6413b8dfca867c2377dc19f2d070c18575f4`
  - `crates/modeler3d-app/src/shell.rs`: `db3dab8be4efd8dfda71b616bbc146e30b90f4ed98b38ba916b997188db9652c`

The receipt postdates the immutable snapshot. Only WORK_LOG.md and the RB-04 checkpoint are expected to differ afterward.

Post-receipt comparison: 2492 included paths in each tree; no missing or extra paths; exactly WORK_LOG.md and the RB-04 checkpoint differ from their post-copy receipts; no other size/SHA-256 differences; reparse entries 0 in each.

## RB-04 completed implementation/native evidence boundary (2026-11-01)

Completed Items/thumbnails/selection/sticky/visibility/naming, independent duplicate/clipboard/delete/group identity/history, exact numeric and viewport Move/Rotate/Resize, Settle/Mirror/Measure/Align/Arrange. Native replay found and fixed untouched-axis rounding, eframe clipboard routing and modal background pointer containment; paired evidence retains genuine build identities. All29 RB-04 implementation/automated/native rows pass; visual/owner review remains pending. Broader RB-05 Move style/placement remains partial, with no RB-05 implementation.

Current gates:261 workspace tests,0failed/ignored; fmt/check/strict all-target all-feature workspace Clippy/deny/metadata/debug+release smoke/release/bloat pass; cargo audit unavailable. Read-only reference tests pass (52gizmo,25scene,11services,7controls,4adapters per recorded runs), and205 real-source oracle cases pass. Native74scene/22viewport/4Settle/8physical-pick states plus modal controls verified.386paired states/812PNG SHAs,108assets,142feature IDs,33catalog IDs and32linked rows validate. Python status116lines and32RB-03 review rows unchanged. Ordinary release14,972,928bytes below40MB executable target; installed package/performance/host acceptance remains later.

Paired agent inspection and visible differences are recorded in evidence/rebuild/rb-04-scene-and-precise-transforms/visual-inspection.md/report.md/visual-comparison.html. Source Shift, Escape/focus, naming/Eye and hidden lock differences remain explicit. Repair fan point and compact report variance remain RB-06; no exact mesh/visual/owner acceptance claimed. A speculative pointer-batch test passed unchanged code and was removed by scoped reversal; original source hash matches current gate artifacts. Rejected native automation attempts are superseded, preserved and not counted.

Both owned apps closed. No staging/commit; existing Rust tree preserved and Python unchanged. RB-03 pending saved-history branch replay, filesystem-failure matrix, extra Python per-format export-picker and visual/owner review are preserved. STOP before RB-05, whose next scope is full viewport gestures/presets/gizmo style/placement/snapping/floor/grid/lighting/colors/wireframe/X-Ray/remaining Resize. RB-06 through RB-11 remain in order. Final verified additive recovery receipt follows.

Planned additive completed RB-04 implementation and evidence recovery snapshot (visual/owner acceptance pending): `G:\Builder3D-Rust_snapshot_20261101-001137_RB-04-completed`. Root target/.cache and reparse entries excluded; exact receipt follows after verification.


The first completed-copy snapshot `G:\Builder3D-Rust_snapshot_20261101-001137_RB-04-completed` verified 2,501 files / 355,031,676 bytes with zero path/size/SHA differences (Robocopy 1), but receipt append failed on an incorrectly encoded UTF-8 BOM in the helper. No receipt was falsely recorded. The copy remains immutable. The helper encoding is corrected; a fresh final copy and receipt will follow. Application source/builds are unchanged.

Planned additive completed RB-04 implementation and evidence recovery snapshot (visual/owner acceptance pending): `G:\Builder3D-Rust_snapshot_20261101-001205_RB-04-completed`. Root target/.cache and reparse entries excluded; exact receipt follows after verification.


### RB-04 completed implementation/native evidence: verified additive snapshot

- Immutable snapshot: `G:\Builder3D-Rust_snapshot_20261101-001205_RB-04-completed`. Verified 2026-11-01T00:12:12-06:00; Robocopy exit 1.
- Source and snapshot: 2,501 files / 355,033,106 bytes each; relative-path differences 0; size/SHA-256 differences 0; reparse entries 0 in each tree. Root target/, .cache/ and reparse entries excluded. Source, Git metadata, tests, documentation and evidence included.
- RB-04 implementation, automated gates and sequential native Light/Dark evidence are complete:261 workspace tests,74 scene cases,22 viewport cases,4 Settle cases,8 paired physical-pick states and modal controls verified. Visual/owner acceptance remains pending. Python is unchanged; all32 RB-03 statuses are preserved. Both owned target apps are closed. STOP before RB-05; no staging or commit. Next slice is full viewport gestures/presets, gizmo style/placement, snapping, floor/grid, lighting/colors, wireframe/X-Ray and remaining resize interaction. See report.md for visible differences and Repair tessellation variance retained for RB-06.
- Pre-receipt recovery-critical SHA-256 values:
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/boundary-evidence-validation.json`: `ebc55a3bd5ee11584b49999a2d8f38b317b47aa0ed0efadfc7e2bde78d19fd35`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native-scene-verification.json`: `c3fe36c306336663235bcefb4151948609c16757b440858110189ba789711d89`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native-viewport-verification.json`: `77c8c01ee769d5112e1a9fe7ef73cd776f7f914f967fbb3de7fd1383c41d9ad5`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native-pick-verification.json`: `3e0bb0d3b07f76159df6a0de3154b670e2b93311a7845c43b6473cb9443923b9`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/comparison-pairs.json`: `4bffb314ecf9194b141cb40bd4adba44570e72ae5cf373c4cc4ee9159f89fac7`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/capture-metadata.json`: `6b4259e635daaa6ca631828f8f758da7b483f391372160fa8b8bd388da9a700f`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/visual-inspection.md`: `cebf5c387a9a641fde37ef144d8eb0570f85a78785aad1aa658016e01c409a4e`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/final-native-build-artifacts.json`: `c31625b1a815bab878bca98a3721dbaec78caccbdb34706838950f76630fc1d6`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/final-native-gate-results.json`: `4353e8fe241675263356884182d68639c326cd82d1add99c6e5b447e64f41ff0`
  - `crates/modeler3d-app/src/lib.rs`: `c2befb366c9464a279ffa4c35cddef7b9679fc84b3dcd7b2c61460f0d00e48b9`
  - `crates/modeler3d-app/src/packaging.rs`: `8b92ce7fe3fb6964ec95a95ef17f3ed6c8d58435452f7b3960d1cfdb85750768`
  - `packaging/licenses/BOOST-LICENSE-1.0.txt`: `c9bff75738922193e67fa726fa225535870d2aa1059f91452c411736284ad566`
  - `packaging/licenses/CLIPBOARD-NOTICE.txt`: `3ff9c3d1a7b57637e92d9442a032953bc9b0d17357fb892ee6b85b2ffcabf84b`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/clipboard-native-fix.md`: `72ebb969905bb73675ad9d8f253b6038b6afabcaa4ce2f36bf7b764487e990ff`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/clipboard-complete-gate-results.json`: `fdca31492a10e7ae2bd633f226c8dd395b01ca9828cdfa8dcf8159b7b10361e1`
  - `crates/modeler3d-core/src/placement.rs`: `87c767a0e97adf72e422a340f92cb00c4a6194361128261395b4377e1218b8a6`
  - `crates/modeler3d-app/src/transform_gesture.rs`: `128d0a8f5033ce4df1875d5c1dc84c63731650ae8f1d5b902b2d3d54f510e3bc`
  - `crates/modeler3d-app/src/gesture_tests.rs`: `fd4f0e70b65dcc9c1c0aaa45e8479e7d3372b9738366395d7172544237e2dc93`
  - `crates/modeler3d-app/src/recent_preview.rs`: `5d4afde3bc674011a77f50ff95119de000563bf3cb1ab0a7d22b8c0223166051`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native-settle-verification.json`: `6aeaf2e2c80392ccfd60869b54696db8940a1d34cf49f4dd945998a7b0005a2a`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/python-settle-results.json`: `b4d32acb5cf8af18bb458db7dd52e0865b02301deb3b342b64df59715c8c3e96`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native_desktop_guard.py`: `ab64c435ba857d1c415dd3eb56292f0600b26a66cd8c99fc91d23eaff8dd1ad0`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/native-rust-workflow.ps1`: `886321d7e2f411e26933070d52b0f6ec2c44763909c7c6dbc81ef5457b23b61e`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/rb03-preserved-status-check.json`: `6d35b75b8a9f952b12c07a80e7e0604cd685614b38d441e35c37357e4f3bd62a`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/numeric-selection.md`: `393e44655f81755bf26eaf7949b6a7e3c6fa3b35836f5ae8b9b090c934dbe99b`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/numeric-selection-oracle.json`: `13f72c05c129a3f8bf4005c38f70f10c617a17009ac4c60739219abe5e114852`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/placement-oracle.json`: `4da2aa0e86abff0610caa45171f952d4d95ac44b11e869341a25e52ba368ad66`
  - `crates/modeler3d-viewport/src/scene.rs`: `68cd984b002793be27e19ad62596ae8c25761fcda6dc362ca0a46b430d7eac0d`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/gesture-oracle.json`: `a093e6129c0ea8db88e167a0fafd8629c92ba63544ded89574e47355b57fd843`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/viewport-transforms.md`: `0e2e7f1dd232a320153c4e51027d97ff99c3a36fa27b8df00b4e37a3e35cb0d0`
  - `scripts/Initialize-BuildClang.ps1`: `73856cc160105f1a9c00bf845b14e29a3002dfdf6ba3efc5aed2742ed43a3d3f`
  - `docs/building.md`: `750a226f595466b2e5fc15f4fb453fe284c4c7f199dd9632e3e90233ec44495a`
  - `packaging/licenses/QHULL-COPYING.txt`: `106d55c931fd6a84822e5345d900273d059f1c27310d02567ccb313c5d18c55d`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/settle-fix.md`: `6675af8640ca82626dbdfac8e664def2d4d4442edb803aa2199b7525af3ee2d4`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/report.md`: `916aa1cdc908e20ff1469422c32ad8c21b450d3ef680d27a14c8bbb8035aecdc`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/checkpoint.md`: `bfdcd9b7fd3772f25dca78cee95c7284eb6aecbb4be2feef3375c10038316740`
  - `evidence/rebuild/rb-04-scene-and-precise-transforms/verification.md`: `088711b3cdb2b7ae7d952bf61e565d14af599e545d3992baedffd0b5ccca3e90`
  - `crates/modeler3d-app/src/rotation.rs`: `dc9c80002d7aa507e88d48fab024158b4cb3eff0638c6f179f6fcc450aba5484`
  - `crates/modeler3d-app/src/transforms.rs`: `dcbe4eebf97af53ba391c9a5d9070713a7737a9047b5d5f7089876de4a721861`
  - `AGENTS.md`: `2253e20aed7056e2cae9af43c5cc5791fdd5235d2ba202ab7e12b9f6d441f763`
  - `Cargo.toml`: `ee87e4c1e28a4b4a17af12d5cb352235d590d621eb01bdff2c4b0a02226b2c65`
  - `Cargo.lock`: `fc3b23fa69d04bfc9e349413d0d24bcc919e4a3706cab72a02940a9ce43f96fe`
  - `WORK_LOG.md`: `f67b6f21519f673d847bc664ee187f228e1341c824aba6a1a6d4bf9a15ab097a`
  - `.git/HEAD`: `28d25bf82af4c0e2b72f50959b2beb859e3e60b9630a5e8c603dad4ddb2b6e80`
  - `docs/rb-03-lifecycle-and-file-completion-handoff.md`: `5d166c80c6a9a8ab4d3085a9d4ff872cc98efc7b6469ffe0583d48eedb6ff222`
  - `evidence/rebuild/parity-matrix.toml`: `df6a92058c77cf9480cd749fdd63aa2491d2f5d795004846dcbf5104fcc20a16`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/checkpoint.md`: `e7b4cfbe204a37bf4d878ec097b159d5e9e89a66a2464b79216b541db25964cf`
  - `evidence/rebuild/rb-03-lifecycle-and-file-completion/requirement-test-map.md`: `e03765801daa74cc500f5948c15065de0e013e7398491c2f18f325cc71c875da`
  - `crates/modeler3d-core/src/project.rs`: `cbac8d8d250d1532891fc6e170a34e4cd519fcbe2c8d58e8ae66a25a43eddff7`
  - `crates/modeler3d-core/tests/project_golden.rs`: `28577b83dbe38767c3bd2722bc13a74932335d25b593ef5a1b4cd90b22752dea`
  - `crates/modeler3d-app/src/commands.rs`: `f095c3fd5f92c2858341fdff111cdde83f544c7cd808fb86556b2511f07aab9f`
  - `crates/modeler3d-app/src/dialogs.rs`: `b6768aa9fe967a138bb1595fd4eb6413b8dfca867c2377dc19f2d070c18575f4`
  - `crates/modeler3d-app/src/shell.rs`: `db3dab8be4efd8dfda71b616bbc146e30b90f4ed98b38ba916b997188db9652c`

The receipt postdates the immutable snapshot. Only WORK_LOG.md and the RB-04 checkpoint are expected to differ afterward.

Post-receipt comparison: 2501 included paths in each tree; no missing or extra paths; exactly WORK_LOG.md and the RB-04 checkpoint differ from their post-copy receipts; no other size/SHA-256 differences; reparse entries 0 in each.

## Full RB-05 next-session handoff (2026-11-01)

Created docs/rb-05-viewport-and-input-completion-handoff.md and updated current continuation links. The next session must complete the entire RB-05 viewport/input slice, its focused/integrated checks, sequential paired Python/Rust native evidence, ledger/docs and verified additive G snapshot, then stop before RB-06. The owner explicitly deferred unrelated button troubleshooting; no audit or RB-04 reopening was started. RB-03/RB-04 review statuses remain pending. No implementation, application interaction, Python edits, staging or commits occurred in this documentation task. Prior261-test/application-build results are cited as historical verified baseline, not rerun results. Handoff link/scope/source-preservation checks and documentation recovery receipt follow.

Planned additive RB-05 handoff documentation recovery snapshot (RB-05 implementation has not started): `G:\Builder3D-Rust_snapshot_20261101-051329_RB-05-handoff`. Root target/.cache and reparse entries excluded; exact receipt follows after verification.


## Verified handoff documentation snapshot

- Snapshot: `G:\Builder3D-Rust_snapshot_20261101-051329_RB-05-handoff`; verified 2026-11-01T05:13:37-07:00; Robocopy exit 1.
- Source/copy: 2,502 files / 355,076,335 bytes each; zero missing/extra paths and zero size/SHA-256 differences; zero reparse entries. Root target/.cache/reparse entries excluded.
- This is a documentation-only continuation snapshot. RB-05 implementation has not started. Links and reference paths validate; application source hashes and the entire parity ledger are unchanged. No implementation tests or native walkthrough were rerun for this documentation task. Full RB-05 work is the next-session assignment, followed by a stop before RB-06; unrelated button troubleshooting is deferred. No staging/commit or Python edits.
- The receipt postdates the immutable copy; only WORK_LOG.md and this RB-05 handoff are expected to differ afterward.

Post-receipt comparison: 2502 included paths in each tree; no missing or extra paths; exactly WORK_LOG.md and the RB-05 handoff differ from their post-copy receipts; no other size/SHA-256 differences; reparse entries 0 in each.

## RB-05 implementation checkpoint: camera/input/snap/gizmo foundations (2026-11-01)

Started RB-05 after preserving the mandatory baseline snapshot
`G:\Builder3D-Rust_snapshot_20261101-051800_RB-05-baseline` (2,502 files,
355,078,639 bytes, zero relative-path or SHA-256 differences). Added persisted
Snap state and ribbon command handling, hidden-object-safe framing, pinch input,
exact double-click frame selection, Move preview snapping with Shift bypass, and
shared bounds-aware camera-facing gizmo geometry/hit testing. The read-only
Python viewport/gizmo/snapping focus suite remains 145 passed; the Rust viewport
package is 30 passed and workspace check passes with the pinned libclang setup.
See `evidence/rebuild/rb-05-viewport-and-input-completion/checkpoint.md`.

Planned additive RB-05 implementation checkpoint snapshot:
`G:\Builder3D-Rust_snapshot_20261101-053120_RB-05-camera-input-gizmo`.
Continue with renderer display options and floor/grid/lighting/X-Ray before the
paired native evidence; stop before RB-06.

### Verified RB-05 camera/input/gizmo checkpoint snapshot

- Immutable snapshot: `G:\Builder3D-Rust_snapshot_20261101-053120_RB-05-camera-input-gizmo`;
  Robocopy exit 1 and SHA-256 verification completed 2026-11-01 05:31:25.
- Source and snapshot: 2,503 files / 355,102,864 bytes each; zero missing,
  extra, or differing relative-path SHA-256 entries. Root `target/`, `.cache/`,
  and reparse entries were excluded.
- This is an additive implementation checkpoint, not RB-05 completion. The
  remaining renderer presentation, native paired evidence, full verification,
  and final receipt are still required. RB-06 has not started.

## RB-05 completion checkpoint before final receipt (2026-11-01)

RB-05 implementation, focused and full tests, read-only Python-first paired
native evidence, documentation, parity/component ledger updates, and accepted
capture verification are complete. Accepted evidence is 30 Python records, 52
Rust records, and 82 PNGs at 1280 x 840 / 96 DPI. Implementation, automated,
and native statuses are pass; visual review and owner acceptance remain
pending. Direct Rust Grid-off supplemental captures are retained as rejected
because the frame was identical to rest. RB-06 has not started.

Planned additive final snapshots: `G:\Builder3D-Rust_snapshot_20261101-064700_RB-05-pre-receipt`
and `G:\Builder3D-Rust_snapshot_20261101-064700_RB-05-complete`. Root
`target/`, `.cache/`, and reparse entries are excluded; the final receipt is
written before the complete copy and no changes follow that copy.

## Verified RB-05 completed snapshot receipt (2026-11-01)

- Staging snapshot: `G:\Builder3D-Rust_snapshot_20261101-064700_RB-05-pre-receipt`;
  Robocopy exit 1; 2,639 included files / 379,185,008 bytes; zero relative
  path or size/SHA-256 differences; zero reparse entries.
- Final immutable snapshot: `G:\Builder3D-Rust_snapshot_20261101-064700_RB-05-complete`;
  Robocopy exit 1; 2,639 included files / **379187260 bytes**; zero relative
  path or size/SHA-256 differences; zero reparse entries. Repository-root
  `target/`, `.cache/`, and reparse entries were excluded.
- The final complete copy includes the receipt, all RB-05 source/docs/tests,
  native journals, accepted/rejected evidence, and capture metadata. No source
  changes follow this receipt or final copy. RB-06 has not started.

## RB-06 modelling-tools and brush completion (2026-11-01)

Started RB-06 only after preserving the mandatory baseline snapshot
`G:\Builder3D-Rust_snapshot_20261101-082535_RB-06-baseline` and the additive
inventory snapshot `G:\Builder3D-Rust_snapshot_20261101-090000_RB-06-inventory`.
The Python reference at `L:\Builder3D` remained read-only. Implemented the
missing native interaction seams: bounded editable Torus metadata and preview,
typed non-parametric Torus rejection, typed repair/error paths, one-history mesh
operations, persistent Grab/Crease/Smooth Brush/Blob/Normalize strokes, Boolean
Union plus exact-two Intersect/Subtract pickers, and typed Emboss/Hollow preview
dialogs. Audited visible Object, Paint, View, Items, and dialog buttons and
recorded the pre/post ledger in the RB-06 evidence directory.

The read-only Python test suite passed 272 tests with two existing trimesh
warnings. Locked Rust gates passed: 271 workspace tests, format, check,
all-feature clippy, deny, metadata, release build, cargo-bloat, and debug/release
smoke. Python native baseline evidence ran first with six records; Rust native
evidence then ran sequentially with actual OS input in Light and Dark, 36
accepted records per theme and one debug executable identity
`82284b6fd85221df3a804790cafb1171c469c5cb614460cb73199068ac551d14`. The final
accepted set contains 78 PNGs. Earlier stale-build, three-selection, empty
Subtract-fixture, missing-libclang, and test-assertion attempts are rejected and
documented, not used for parity claims.

Implementation, automated, and native statuses are pass. Visual review and
owner acceptance remain pending; RB-03/RB-04/RB-05 historical pending statuses
are preserved. The final additive snapshot is
`G:\Builder3D-Rust_snapshot_20261101-095500_RB-06-complete`. Its staging
comparison measured 2,532 included files / 396,942,394 bytes on both sides with
zero missing, extra, size, SHA-256, or reparse differences. The finalized
receipt is synchronized once more and verified before stopping before RB-07.


## RB-07 appearance, jobs, and print handoff (2026-11-01)

Completed the current Rust appearance/material/image, native job lifecycle, print-preparation, staging, and slicer-handoff slice from the current worktree. Reused the existing image-color, operation registry, print-prep, print-handoff, and platform slicer implementations and repaired the demonstrated native shell integration gaps. The Python reference at L:\Builder3D stayed read-only; its 116 pre-existing Git status records and ten tracked reference hashes are unchanged. No changes were staged or committed.

Rust workspace tests passed: 301; focused tests passed: app 150, core 88, platform 15, viewport 34. Python focused reference tests passed: 103. The full Python reference suite reported 785 passed, one existing K:\builder3D.png machine-path assertion failure, and two existing trimesh warnings. All ten documented Rust gates passed; cargo audit was not installed. Exact logs are in evidence/rebuild/rb-07-appearance-jobs-and-print/.

Accepted paired/native evidence contains 117 PNGs: Python 34 (17 per theme), Rust 3D Modeler 82 (40 Light / 42 Dark), and one real Bambu Studio frame. Rust Dark and Python Light/Dark native Cancel terminals are confirmed; Rust Light Cancel was attempted but not confirmed, and is documented without claiming success. The actual installed Bambu Studio opened the validated 684-byte, 12-triangle Rust staged STL. Debug executable SHA-256 is 0BF7753FE460A6A854906457F86363DDC905B455FA790760B6D9122F6F9CB0D6; release SHA-256 is E26C37DE652B6434727AE7F6396012CDF4AA911299857B621D336AFB4DA38448.

Additive RB-07 baseline: G:\Builder3D-Rust_snapshot_20261101-132912_RB-07-baseline. Final verified snapshot: G:\Builder3D-Rust_snapshot_20261101-162515_RB-07-complete; see the RB-07 snapshot receipt. Visual review and owner acceptance remain pending. **Next slice: RB-08; implementation has not started.**

## RB-08 continuation handoff (2026-11-02)

Prepared `docs/rb-08-preferences-and-complete-shell-handoff.md` with the RB-08 scope, verified RB-07 starting state, current implementation seams, test/native evidence plan, completion snapshot requirements, and a fresh-context prompt. Refreshed the current-slice pointers in `AGENTS.md` and `docs/product-rebuild-plan.md` from the stale RB-05 continuation to RB-08. No RB-08 implementation or tests were run, and no RB-08 snapshot was created; implementation has not started.

## RB-09 shared AI and regression handoff (2026-11-02)

The owner reported that the interface and floor are still missing, and directed the work to continue to RB-09. Added `docs/rb-09-shared-ai-and-regression-handoff.md`, updated the current continuation pointers in `AGENTS.md`, `docs/product-rebuild-plan.md`, and the legacy RB-04-through-RB-11 handoff, and marked the RB-08 handoff/report/checkpoint as historical and still open. RB-08 automated checks remain recorded, while its Rust native matrix and completion snapshot are pending; the owner direction permits RB-09 without retroactively accepting RB-08. Source inspection confirmed the key RB-09 gap: `R4Server::new` creates an independent MCP `Scene`, while `modeler3d-app` owns the live `CommandState` and does not currently depend on `modeler3d-mcp`. The new handoff captures the shared-document, history, revision, replay, file, cancellation, optional-service, real-client, native evidence, and snapshot requirements. Read `E:\MCP_DESIGN_GUIDE.md` and checked the linked current official MCP 2026-07-28 and Rust SDK references. This was documentation and source inspection only: no implementation or tests were run; no G: snapshot was created; `L:\Builder3D` was not modified; nothing was staged or committed.

## RB-09 shared-document implementation and native-gate handoff (2026-11-02)

Implemented the RB-09 Rust desktop/MCP integration in the existing `E:\3D-Modeler` worktree. `Modeler3dApp` owns the authenticated loopback bridge and dispatches synchronous R4 calls on the UI owner thread using the exact live scene/history. Long calls use bounded isolated scene copies and one checked core commit against project identity, generation, revision, and path baselines. The stdio executable now exposes the existing fixed 60-tool MCP catalog through `R4Gateway`, validates the desktop runtime descriptor and status identity, and returns a structured unavailable error when the desktop is absent. Added dirty project guards, serialized idempotency/revision dispatch, bridge request limits/authentication/cleanup, and scene snapshot history tests. Rust source was unchanged after the Release app and MCP binaries were built.

Focused results: 19 MCP library tests, 5 app bridge/shared-document tests, and 3 core snapshot commit/history tests passed. The locked Rust workspace passed 327 tests. Twelve focused Python catalog/bridge/gateway/SDK tests passed using the existing `L:\Builder3D` Python 3.12.10 venv; all scratch output was directed to the RB-09 evidence folder on E:. The format, workspace check/test, all-target/all-feature Clippy, cargo-deny, metadata, app smoke, Release app/MCP builds, and both cargo-bloat commands exited 0. The TOML/catalog/evidence-reference/JSON/NDJSON validator passed. `cargo deny check` emitted 28 duplicate-crate warnings and no advisory warnings. `cargo bloat --release --workspace` warned that `--workspace` is unused, so it is not counted as full workspace size coverage; the explicit app report passed with 12.7 MiB `.text` and 15.4 MiB reported file size. `cargo audit` is unavailable. Exact outputs are in `evidence/rebuild/rb-09-shared-ai-and-regression/verification.md` and `logs/`.

The Release app is 16,152,576 bytes with SHA-256 `98ff94d0ef5938817d10cbe8c0c7ec18058ad0487ad403a2e674612f62d21d5f`; Release `r4_mcp.exe` is 1,649,664 bytes with SHA-256 `331a69b9f4661f0f3687b0e4d1588a230e9106384a5b0190a7421cd9da9c1557`. The official Python MCP SDK 2.2.0 initialized the Release stdio gateway and listed all 60 tools in order; with the descriptor intentionally absent, `get_status` returned the bounded retryable `desktop_unavailable` error. This verifies the real client/gateway closed-desktop path only, not live desktop forwarding.

`L:\Builder3D` remained read-only: its command-scoped-safe Git status still has the same 116 records documented before RB-09, and `src/main_window.py` retains SHA-256 `3fc2b1c25a53f6f7d040d3321d331a22d7b8a462f8cb4fcd18a9631b960a7c41`. The git safety override was command-local; no global Git configuration was changed. No file was staged or committed.

At final process inspection Windows `LockApp.exe` PID 9808 still owned the interactive session; no Rust desktop or Rust MCP process was running. Therefore the paired live-client/native matrix, viewport/panel observation, shared undo/redo, save/reopen, file-path, long-operation cancellation, runtime shutdown, and screenshot evidence remain explicitly unverified. Do not close RB-09 on headless tests. The owner-reported interface and floor gaps remain open; earlier visual/owner statuses remain pending. RB-08's Rust native matrix and completion snapshot remain separately pending. Preserve this endpoint in the additive `G:\Builder3D-Rust_snapshot_20261102-152935_RB-09-handoff-staging` and `G:\Builder3D-Rust_snapshot_20261102-152935_RB-09-handoff` copies, then stop before RB-10. The latter is a handoff snapshot, not an RB-09 completion receipt.

## Active Rust path and backup policy update (2026-11-03)

The owner confirmed that future 3D Modeler Rust work, builds, and launches should use `E:\3D-Modeler`. `G:\` is the primary additive recovery-backup destination; removable USB copies are occasional one-offs unless the owner requests otherwise. The USB copy at `L:\Builder3D-Rust_snapshot_20261103-103227_RB-09-paused` remains a one-time recovery copy, not a working tree. Its copy and SHA-256 verification are recorded in `evidence/rebuild/rb-09-shared-ai-and-regression/usb-backup-receipt.md`. `L:\Builder3D` remains the separate read-only Python reference/shipping build.

Updated the root and Rust project instructions, build guide, current product plan, RB-09 handoff/checkpoint, and source-preservation/report notes to make the E: working path and backup policy explicit while retaining historical G: receipts and Python-reference paths. The paused RB-09 native gate remains pending; RB-10 has not started. The follow-up primary G: snapshot is `G:\Builder3D-Rust_snapshot_20261103-111729_RB-09-paused-e-primary`; see `primary-backup-receipt.md` for its exact verification.

## Review notes - E: workspace and backup policy task (2026-11-04)

Scope: close the active-path and backup-policy documentation task. This does
not close RB-09. The product plan's mandatory live-client/native gate remains
paused and unverified.

### Check commands and results

- `cargo fmt --all -- --check`: pass, exit 0.
- `cargo check --workspace --locked`: pass, exit 0.
- `cargo test --workspace --locked`: pass, exit 0. The quiet confirmation
  printed `test result: ok. 327 passed; 0 failed; 0 ignored` across the workspace.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`:
  pass, exit 0.
- `cargo deny check`: pass, exit 0; 27 duplicate-crate warning headers;
  final summary `advisories ok, bans ok, licenses ok, sources ok`.
- `cargo metadata --locked --format-version 1 --no-deps`: pass, exit 0.
- `cargo run -p modeler3d-app --bin modeler3d-app -- --smoke`: pass, exit 0;
  output `3D Modeler shell smoke: objects=2 selected=1 tabs=5`.
- `cargo build --release --locked -p modeler3d-app --bin modeler3d-app` and
  `cargo build --release --locked -p modeler3d-mcp --bin r4_mcp`: both pass,
  exit 0. Current files are 16,152,576 and 1,649,664 bytes; their hashes are
  respectively `C6BF15935F66DDE3047DDA886724E935ED781BADDB31DA8D6742EB4973C507C7`
  and `A1DBA8751C5A15CAFBF6E26FAC8DB0B8A96F8464BF750AC62336F573B729545B`.
- `cargo bloat --release --workspace`: the first attempt failed with exit 1
  because I omitted the documented libclang setup. With
  `Initialize-BuildClang.ps1` in the same PowerShell session it exited 0, but
  cargo-bloat warned that `--workspace` is unused and analyzed
  `target\release\r5_viewport.exe`; it is not a workspace-size result.
- `cargo bloat --release -p modeler3d-app --bin modeler3d-app`: pass, exit 0;
  12.7 MiB `.text`, 15.4 MiB reported executable size.
- `cargo audit --locked` was not run: `cargo-audit` is not installed.
- `python evidence/rebuild/rb-09-shared-ai-and-regression/validate_rb09_evidence.py`:
  pass; TOML, catalog IDs, evidence references, JSON, and NDJSON validation
  succeeded.
- The official Python MCP SDK 2.2.0 probe exited 0, initialized stdio, listed
  60 tools in order, and received the expected retryable `desktop_unavailable`
  response with the desktop closed. This is not live desktop evidence.

### 3. Acceptance criteria, commands, and results

- Criterion: Rust source/build/run root and Release artifacts are on E:.
  Command: `git rev-parse --show-toplevel` plus `Test-Path` for the Release app
  and MCP executables. Result: pass; root is `E:/3D-Modeler` and both
  executables exist under `target\release`.
- Criterion: active guidance names E: as the Rust worktree, G: as primary
  backup, USB as occasional, and L: as the read-only Python reference.
  Command: `rg -n -i` over `E:\AGENTS.md`, project `AGENTS.md`, the build guide,
  product plan, RB-09 handoff, checkpoint, source-preservation note, and primary
  backup receipt. Result: pass; those active pointers are present.
- Criterion: the paused-state G: backup matched the E: tree at verification
  time, excluding
  root `target`, `.cache`, and `logs`. Command:
  `verify_rb09_snapshot.ps1 -SourceRoot E:\3D-Modeler -SnapshotRoot G:\Builder3D-Rust_snapshot_20261103-111729_RB-09-paused-e-primary`.
  Result: pass; 3,291 files / 584,916,627 bytes on both sides, zero missing,
  extra, size/hash-different, or reparse entries. This check preceded the
  review entry and commit; the post-review snapshot receipt is recorded below.
- Criterion: USB remains a one-off copy and the Python reference stayed intact.
  Commands: `Test-Path` for the USB snapshot and `L:\Builder3D`; read-only
  `git -c safe.directory=L:/Builder3D -C L:\Builder3D status --short`; and
  `Get-FileHash L:\Builder3D\src\main_window.py -Algorithm SHA256`.
  Result: pass; both paths exist, the L: status still reports 116 records, and
  the source hash remains
  `3FC2B1C25A53F6F7D040D3321D331A22D7B8A462F8CB4FCD18A9631B960A7C41`.
- Criterion (RB-09 plan): a real client and native desktop edit one live
  document. Command: search the current plan, report, and checkpoint for the
  live-client/native state. Result: not met; the records explicitly say
  unverified, and no GUI or live edit was run during this paused wrap-up.
- Criterion (RB-09 plan): cross-adapter queries, edits, history, save/reopen,
  conflicts, cancellation, and failure atomicity pass automated and live checks.
  Commands: `cargo test --workspace --locked --quiet` and checkpoint/report
  search. Result: automated tests pass (327/327); live cross-adapter flows stay
  unverified, so the criterion is not met.
- Criterion (RB-09 plan): the fixed 60-tool catalog remains compatible.
  Commands: `cargo test -p modeler3d-mcp --lib --locked --quiet`, the evidence
  validator, and the official-client probe. Result: pass for the catalog and
  closed-desktop protocol path; the focused MCP tests pass 19/19 and the client
  lists 60 tools. This does not prove live forwarding.
- Criterion (RB-09 plan): MCP may be unavailable without blocking normal
  desktop use. Commands: official-client closed-desktop probe and app `--smoke`.
  Result: pass for the bounded unavailable response and semantic smoke path;
  interactive GUI usability was not exercised.
- Criterion (RB-09 plan): evidence, all applicable gates, and an RB-09
  completion snapshot are complete. Commands: evidence validator, current G:
  snapshot verifier, and `Get-ChildItem -LiteralPath 'G:\' -Directory -Filter 'Builder3D-Rust_snapshot_*_RB-09-complete'`. Result: evidence validation and
  the paused-state snapshot pass; no RB-09 completion snapshot exists, so this
  plan criterion is not met.

### 4. Shortcuts, warnings, and untested work

- No application stub, TODO marker, ignored test, or lint suppression was added
  for this documentation task. Source search found zero `todo!`,
  `unimplemented!`, `dbg!`, TODO/FIXME/HACK markers, and zero `#[ignore]` tests.
- Existing source contains 17 `#[allow(clippy::too_many_arguments)]` attributes
  and 2 scoped `#[allow(unsafe_code)]` attributes; the strict Clippy command
  passed. These were not changed here.
- Existing replay and operation-retention bounds are constants: 600 seconds / 256
  replay records and 3,600 seconds / 128 operation records. The RB-09 handoff
  says to verify them against the Python reference before changing them; that
  check remains open.
- `cargo deny check` reports 27 duplicate-crate warnings. It reports no
  advisory, ban, license, or source-policy failures. `cargo-audit` is absent.
- `git add` reported that six Markdown files with LF line endings will be
  normalized to CRLF by the current Git settings. No Rust source was staged.
- The workspace `cargo bloat` invocation does not measure the workspace because
  this cargo-bloat version ignores `--workspace`; the explicit app-target
  report is the available size result.
- The rebuilt Release hashes above differ from the hashes in the historical
  2026-11-02 build manifest, although the file sizes match. I did not rewrite
  that historical evidence; verify build identity before promoting a binary.
- RB-09 live desktop/MCP edits, paired viewport/panel updates, GUI/MCP
  undo/redo, save/reopen, path policy, revision races, long-operation
  cancellation, and runtime shutdown remain untested. The interface and floor
  gaps and the RB-08 native matrix/completion snapshot remain open. RB-10 has
  not started.

### 5. Files created, changed, or deleted

- `E:\AGENTS.md` - changed the workspace rule so Rust work uses E: and G: is
  primary, with removable USB copies occasional.
- `AGENTS.md` - recorded the active E: worktree, backup policy, and paused
  RB-09 continuation.
- `docs/building.md` - added the E: build/run location and backup/reference roles.
- `docs/product-rebuild-plan.md` - recorded current slice status and E:/G:/USB
  path policy.
- `docs/rb-09-shared-ai-and-regression-handoff.md` - recorded E: as active,
  G: as primary, USB as occasional, and the native pause.
- `evidence/rebuild/rb-09-shared-ai-and-regression/checkpoint.md` - recorded
  the E: worktree, backup roles, and pending gates.
- `evidence/rebuild/rb-09-shared-ai-and-regression/source-preservation.md` -
  recorded owner path and backup direction.
- `evidence/rebuild/rb-09-shared-ai-and-regression/report.md` - added the
  current path and paused-gate status.
- `evidence/rebuild/rb-09-shared-ai-and-regression/primary-backup-receipt.md` -
  created the receipt for the verified G: snapshot.
- `WORK_LOG.md` - recorded the path-policy task and this review entry.
- `G:\Builder3D-Rust_snapshot_20261103-111729_RB-09-paused-e-primary` -
  created the additive primary recovery snapshot; its exact verification is in
  `primary-backup-receipt.md`.
- `target\` - Cargo checks generated or refreshed ignored tool, test, and
  Release outputs, including `review-cargo-deny.log`; these are not source files.
- Deleted files: none.

### 6. Unplanned dependency, config, or design changes

None. No Cargo manifest/lockfile, dependency, runtime configuration, or
architecture change was made. The documented build helper used its pinned
build-only libclang under ignored `target\` and set `LIBCLANG_PATH` only in
the test process.

The owner explicitly requested a task-scoped documentation/evidence commit in
this wrap-up, overriding the earlier no-commit direction for these files only.
The commit does not close RB-09 or grant general commit authorization.

### Post-review G: snapshot receipt (2026-11-04)

After commit `8e2178e`, created the additive snapshot
`G:\Builder3D-Rust_snapshot_20261104-073919_RB-09-review` from the E: worktree.
Robocopy exited 1 (files copied, no failures). The independent
`verify_rb09_snapshot.ps1` check passed: source and snapshot each contain 3,312
included files / 585,155,060 bytes; missing, extra, size/hash-different, and
reparse entries are all zero. Root `target`, `.cache`, and `logs` were excluded;
the hidden Git metadata was included. This receipt was appended after copying,
so the snapshot predates this paragraph. The snapshot is a paused/review copy,
not an RB-09 completion snapshot.


## RB-09 live desktop/MCP verification (2026-11-04)

Completed the paired Python/Rust live desktop and MCP sequence from E:/3D-Modeler. The host helper confirmed the Default desktop was active and interactive before each input and capture; LockApp did not own the session. Edge?s 24-hour YouTube video stayed in the background as a keep-awake aid only. No Windows power or lock settings changed.

The initial Rust live run found a scoped Welcome-to-Scene presentation defect: MCP opened/edited the document but the GUI stayed on Welcome. Updated shell presentation synchronization after MCP installs/commits and project replacement, then added two app regression tests. The repaired live run showed the Scene and updated viewport/Items immediately.

The official Python MCP SDK 2.2.0 exercised the real Python and Rust app bridges. Project e5d81c5226e64f58b838d55d9395afa9 and stable Cube/Sphere IDs matched. GUI/MCP history, no-op behavior, Save/close/reopen/New/Open, dirty protection, denied/invalid files, failure atomicity, idempotency replay, expected-revision conflicts, competing clients, long-operation progress/cancel/failure/stale results, optional startup, closed-desktop responses, and shutdown cleanup passed. The Python initial capture is revision 4 with the Cube blue; the shared fixture was later saved at revision 23 with the Cube gold after a Python MCP edit. This difference is explicit in the paired board.

Post-repair gates passed: cargo fmt --all -- --check; cargo test -p modeler3d-app mcp_ (7/7); cargo test --workspace --locked (329/329); cargo clippy --all-targets -- -D warnings; locked Release app and gateway builds. Full logs are in evidence/rebuild/rb-09-shared-ai-and-regression/logs/postfix-*. The official SDK post-run probe also passed against the current gateway with all 60 tools and structured desktop_unavailable while closed. The evidence validator passed with 50 capture hashes/dimensions, two clean Light/Dark pairs, ten fixture hashes, 96 Rust sources, and valid TOML/JSON/NDJSON references.

Restored L:/Builder3D external QSettings exactly to object_snapping_enabled=true, ribbon_favorites=[], theme_mode=dark and removed the test-created recent_files key. The Python reference source tree remained untouched. The Rust live app/gateway and optional-failure app were closed; no runtime listener or descriptor remained. The uncommitted E: worktree remains intact.

The additive completion recovery copy is G:/Builder3D-Rust_snapshot_20261104-182021_RB-09-complete. Its independent path/size/SHA-256/reparse verification is recorded in evidence/rebuild/rb-09-shared-ai-and-regression/snapshot-receipt.md. At the end of the live-run phase, no commit or staging had been performed; the later wrap-up request authorized the scoped RB-09 commit.

The owner?s interface/floor gaps and visual review remain open. No owner visual approval or overall acceptance is claimed. RB-08 native matrix/completion snapshot remains open. RB-10 has not started.

## RB-09 wrap-up review (2026-11-04)

Re-read the RB-09 acceptance row in `docs/product-rebuild-plan.md`, the five
completion checks in `docs/rb-09-shared-ai-and-regression-handoff.md`, and the
project hard rules. No Rust source changed during this wrap-up.

Reran from the E: project root after sourcing `scripts/Initialize-BuildClang.ps1`:
`cargo fmt --all -- --check`, `cargo check --workspace --locked`,
`cargo test --workspace --locked` (329 passed, 0 failed, 0 ignored across 28
test-result records), `cargo clippy --all-targets -- -D warnings`,
`cargo deny check` (28 duplicate-version warnings, no policy errors), locked
metadata, focused MCP tests (19 passed), and app smoke. Every command exited 0;
smoke output was `3D Modeler shell smoke: objects=2 selected=1 tabs=5`.

The evidence-record audit passed criteria 1-4: matching live project/object IDs
and accepted interactive capture; cross-adapter edits/history/lifecycle,
conflicts, cancellation and stale-result outcomes; matching 60-tool client
catalogs and 19 focused MCP tests; optional-startup availability, bounded
closed-desktop errors and bridge cleanup. The gate-time evidence validator and
the prior G: snapshot check had passed. A fresh validator rerun now exits 1:
`release binary hash mismatch: live Rust app`. The tested app hash in the
manifest is `d680cf0ba8e4c8bf7d5ccd1536bf7d5720987d676a1a0946c6c3505b613b0cb9`;
the current target app is `0bc4f1a0fd0327e51ecd8acdb75a01120b3c53325b11738a7b3fb6ce198cc75a`.
The current gateway hash is also different from the recorded live and post-run
hashes. No application code was changed to mask this identity discrepancy.

The `cargo bloat --release --workspace` invocation warned that `--workspace`
was unused and analyzed `r5_viewport.exe`, so it did not measure the workspace.
The explicit app-target bloat run reported 12.7 MiB `.text` / 15.4 MiB file;
this is not a total installed-size result. `cargo audit` was unavailable. The
60-minute/256-entry replay and 3,600-second/128-entry operation-retention
limits remain existing constants; comparison with the Python reference is
still open. A 600k import completed too quickly to cancel; cancellation was
verified on Rust 50k normalize progress 90 to 95. Owner visual approval,
interface/floor parity, and the RB-08 native matrix remain open. RB-10 has not
started.

The plan's Review Notes records all five criteria, command/results, shortcuts,
file changes and dependency/config/design scope. The new additive G: review
snapshot and independent verification are recorded in the RB-09 snapshot
receipt. The plan's Recovery snapshot subsection and the snapshot/log receipt
updates were appended after the exact-tree verifier pass; the copy preserves
the staged recovery state from before commit. The owner explicitly authorized
the task-scoped commit in the wrap-up request; this does not alter the standing
rule for other work.

## RB-09 live-artifact identity and validator follow-up (2026-11-04)

Scope: resolve only the RB-09 live-artifact identity/evidence-validator issue.
No RB-10 work, application-source change, MCP-interface change, or new commit.
The task-scoped HEAD remains
53e60d6421ede93cb12f24e1f3c4502c5aec5468. The tracked tree was clean at
resumption. The original 2,807 untracked files were preserved. The identity
check added its screenshot/client log, two copied rebuild logs, one identity
record, and three validator result files; the final untracked count before the
recovery snapshot is 2,815.

The historical paired gate used app hash
d680cf0ba8e4c8bf7d5ccd1536bf7d5720987d676a1a0946c6c3505b613b0cb9 and live
gateway hash a1dba8751c5a15cafbf6e26fac8db0b8a96f8464bf750ac62336f573b729545b.
The historical post-run gateway hash was
9b8817015ab5c8195013ee788fb095e9393848be74c7da0221ebd33342fc99d3. The
gate-time validator passed with exit 0 and its original log remains in
evidence/rebuild/rb-09-shared-ai-and-regression/logs/. A later validator run
failed exactly with release binary hash mismatch: live Rust app because its
historical live role was compared to the current target path.

The ignored wrap-up logs show the cargo bloat --release --workspace
invocation warned that --workspace was unused while rebuilding
modeler3d-mcp and modeler3d-app; the app restore log records another completed
Release build. Copies and hashes are retained under
evidence/rebuild/rb-09-shared-ai-and-regression/logs/identity-recheck-*.
Current target hashes are app
0bc4f1a0fd0327e51ecd8acdb75a01120b3c53325b11738a7b3fb6ce198cc75a and gateway
aff0af548a3f8055e2ebe1d218135a774cc9b88f210a94f19627a9f9b22e1c1e.

With the host helper confirming the active Default desktop and 3D Modeler
foreground, the current Release app ran as PID 8516 and was captured at
1296x879. The official Python MCP SDK 2.2.0 listed 60 tools; its only tool
call, read-only get_status, succeeded with app_state=ready, profile=full,
dirty=false, scene_revision=0, and project_id
4774cfc03391404f8e53775aae37fa6b. Hashes stayed constant across launch and
shutdown. PID 8516, the gateway process, and the runtime descriptor were
absent after close. The screenshot is identity-only and is not part of the
original 50-capture paired matrix. No owner visual review or approval was
performed.

Updated validate_rb09_evidence.py to validate the historical app/gateway hashes
against capture-metadata.json, and the current target hashes against
artifact-identity-recheck.json, including copied rebuild logs, the screenshot,
and the official-client log. The validator passed: 50 historical capture PNG
hashes/dimensions and two clean pairs; 10 fixture hashes; 96-file Rust source
manifest; current app/gateway identities; current 60-tool/get_status result;
TOML, JSON, NDJSON, references, and cleanup evidence. The source manifest
remains 9846a4a6dee484186283e169a9b7cdbbf74007bdca772efa5981f438ce78b457.
The prior 329 workspace tests, 19 focused MCP tests, fmt/check/clippy/deny,
metadata, and smoke results were reused because Rust source did not change.

The exact identity record, client event log, screenshot, copied build logs, and
validator outputs are linked from the RB-09 report, verification, walkthrough,
requirement map, and parity row. Created the additive review snapshot
G:/Builder3D-Rust_snapshot_20261104-215904_RB-09-artifact-identity-review.
Robocopy exited 1 with no failures. The first direct-path verifier attempt
could not resolve one long nested shader-cache path; the same independent
verifier passed through temporary drive Q:, which was removed afterward.
Source and snapshot each contain 3,904 files / 701,101,053 bytes, with zero
missing, extra, size/hash-different, or reparse entries. Root target, .cache,
and logs are excluded; nested RB-09 evidence and Git metadata are included.
This is a review snapshot, not a completion snapshot, and no prior G: copy was
modified. The snapshot predates this receipt and the final WorkLog path/result
update.

Visual review, owner acceptance, overall product acceptance, the owner-reported
interface/floor gaps, and the RB-08 native matrix/completion snapshot remain
open. RB-10 has not started.

## Independent incremental review (2026-11-05)

Verdict: PASS WITH FIXES. Reviewed 53e60d6421ede93cb12f24e1f3c4502c5aec5468..f3b2d929d5d8ba0e30c447a6736c620b15ac4372 (20 files); no review tags existed, so the preceding recorded RB-09 wrap-up review was used as baseline. No Rust application source changed in this range. Existing untracked work was preserved.

SHOULD FIX R1: validate_rb09_evidence.py:308-311 checks only 60 unique tool names. An in-memory substitution of operation_status with nonexistent_review_probe still satisfies these predicates. Compare the complete ordered list against the established catalog or recorded paired-gate list, and add a negative renamed-tool check. Queued at the top of plan section 12. No application or validator implementation was changed by this review.

Fresh checks: cargo test --workspace --locked (329 passed, zero failed/ignored), cargo clippy --all-targets -- -D warnings, cargo fmt --check, cargo deny check, and the RB-09 evidence validator all passed. Logs are under ignored target/independent-review-*. The first shell attempt stopped on PowerShell native-stderr handling; the direct rerun with Continue completed successfully. Dependency duplicate warnings remain; no policy errors. Current Release app launched as PID 21848, with title 3D Modeler - Untitled and nonzero native window handle; left open for the owner. This is launch verification, not an interactive workflow acceptance test.

Review notes accurately distinguish the historical paired workflow from the current identity-only probe, and disclose the pending completion snapshot. Not independently repeated: paired desktop/MCP mutations, visual/owner acceptance, real Linux/macOS or alternate GPU checks, recovery snapshot hash verification, audit, size/performance gates. Review scope does not close RB-08/RB-09 acceptance gaps or authorize RB-10. No commits created. Review marker reviewed-1 will identify the reviewed HEAD; plan/log review additions remain uncommitted.

## 2026-11-05 - reviewed-1 ordered MCP tool-name verification

- Scope: fix the SHOULD FIX finding in the RB-09 evidence validator only; no Rust application code changed and RB-10 was not started.
- The validator now extracts the ordered 60-name `TOOL_CATALOG` from `crates/modeler3d-mcp/src/r4/catalog.rs` and compares the identity probe's complete list against it. This verifies names and order only; it does not verify tool schemas.
- Focused regression: `python evidence/rebuild/rb-09-shared-ai-and-regression/validate_rb09_evidence.py --tool-name-self-test` passed; the established list was accepted and a renamed tool was rejected.
- Evidence validation: `python evidence/rebuild/rb-09-shared-ai-and-regression/validate_rb09_evidence.py` passed, including the 50 accepted capture hashes/dimensions, 2 paired scene records, 10 fixture hashes, and 96-file Rust source manifest.
- Post-change launch: no 3D Modeler instance was running before launch. Started `target/release/modeler3d-app.exe` from this workspace as PID 40272; `3D Modeler - Untitled` had a nonzero window handle and responded. Left the app open. This is a launch check, not an interactive acceptance test.
- RB-08's native matrix and completion snapshot remain pending. The recorded paired RB-09 workflow was not repeated; the identity-only probe does not close RB-09 acceptance, the interface/floor parity gaps, visual review, owner acceptance, or overall product acceptance. No RB-10 work started.
- Changed files: `evidence/rebuild/rb-09-shared-ai-and-regression/validate_rb09_evidence.py`, `docs/product-rebuild-plan.md`, and `WORK_LOG.md`.
- Recovery snapshot: `G:\Builder3D-Rust_snapshot_20261105-143636_RB-09-R1`, created 2026-11-05 14:36:36 local time. Robocopy exit 1; 3,933 included files and 701,519,434 bytes. Independent verification found identical relative-path sets and byte counts, zero path/size mismatches, and matching SHA-256 for 12 recovery-critical files. The copied HEAD and `reviewed-1` tag both match `f3b2d929d5d8ba0e30c447a6736c620b15ac4372`. Excluded `target/`; no other cache directories or reparse points were present. This receipt was appended after copying, so the snapshot predates this receipt.
- An earlier long-name copy attempt, `G:\Builder3D-Rust_snapshot_20261105-143410_RB-09-reviewed-1-tool-name-validator`, is incomplete because a destination path exceeded the Win32 path limit. It was left untouched and is not a recovery point; the shorter snapshot above is the verified copy.

## 2026-11-06 - current Rust Release RB-09 recheck

Scope: continue RB-09 on the current Rust Release build, update only records supported by this evidence, and stop before RB-10. The historical Python/Rust paired workflow remains tied to its earlier recorded build. This continuation did not launch the Python 3D Modeler product or modify Rust application source.

The current Rust Release app and gateway hashes were 0bc4f1a0fd0327e51ecd8acdb75a01120b3c53325b11738a7b3fb6ce198cc75a and aff0af548a3f8055e2ebe1d218135a774cc9b88f210a94f19627a9f9b22e1c1e. The official Python MCP SDK 2.2.0 was used only as a stdio client to the Rust gateway. All four profile counts and ordered names matched the saved Python catalogs (7/52/59/60).

The current Rust GUI/MCP workflow opened, edited, saved, and reopened the shared project. The saved output is 82,178 bytes, SHA-256 e6046a8d3ed7ea949b00a13928311b2fc88e476bda6d4d859af6d866246d9262, project e5d81c5226e64f58b838d55d9395afa9, clean at revision 25 with the stable Cube/Sphere IDs. A separate optional-MCP startup failure left the GUI usable for New Scene, Cube creation, and Undo; closed-client reads returned retryable desktop_unavailable. The normal and failure-run app/client/gateway processes exited, and owned runtime state was absent after shutdown.

The schema comparison against saved Python catalog snapshots found zero exact input-schema and output-schema matches in each profile. It records return_detail on 19 tools without server-side use, orbit_view required/default differences, additionalProperties and bound/nullability differences, one untyped 104-property Rust output-schema superset, two annotation differences, and top-level required-field differences in eight successful tools. This comparison did not launch or refresh the Python product and is not full nested output-schema validation. No owner-approved deviations are recorded, so schema compatibility and RB-09 acceptance remain pending.

The current evidence is under evidence/rebuild/rb-09-shared-ai-and-regression/currpair-20261106-072716/. The focused validator self-check (py -3.13 evidence/rebuild/rb-09-shared-ai-and-regression/validate_rb09_evidence.py --tool-name-self-test) passed with exit 0. The full evidence validator passed with exit 0, validating the historical 50 captures and the nine current-run capture records while explicitly reporting schema parity pending. The current-run manifest writer, schema comparator, and git diff --check also exited 0; diff-check emitted only line-ending normalization warnings. No Rust tests were rerun because no Rust source changed; the previously recorded 329 workspace and 19 focused MCP results remain historical.

Created the new short-name additive snapshot G:\B3D_R09C_20261106. Initial Robocopy exit was 1 with no failures; final synchronization Robocopy exit was 1 with no failed files. The final synchronized source/snapshot manifests each contain 3991 files / 756841099 bytes, with zero missing, extra, differing, or reparse entries. The independent verifier passed after the final synchronization, and critical SHA-256 values match. The receipt records the exclusions and hashes. The verified read-only recovery point G:\Builder3D-Rust_snapshot_20261105-143636_RB-09-R1 and incomplete 143410 attempt were left untouched.

RB-08 native matrix/completion snapshot, the interface/floor gaps, visual review, owner acceptance, overall product acceptance, and RB-09 schema disposition remain open. No RB-10 work started.

## 2026-11-06 - RB-09 wrap-up checks

Re-read the workspace and project hard rules, the RB-09 plan row and section 12,
and handoff sections 5, 7, and 9. The current Release app/gateway hashes remain
`0bc4f1a0fd0327e51ecd8acdb75a01120b3c53325b11738a7b3fb6ce198cc75a` and
`aff0af548a3f8055e2ebe1d218135a774cc9b88f210a94f19627a9f9b22e1c1e`. No
3D Modeler app/gateway process was running after the checks; PID 40272 was absent.
No app launch or UI interaction was performed during this wrap-up.

`cargo fmt --check`, `cargo check --workspace --locked`,
`cargo test --workspace --locked --quiet` (329 passed, 0 failed/ignored),
`cargo test -p modeler3d-mcp --locked` (19 passed, 0 failed/ignored),
`cargo clippy --all-targets -- -D warnings`, and `cargo deny check` all exited 0.
Cargo Deny reported 28 duplicate-version warnings and `advisories ok, bans ok,
licenses ok, sources ok`. The evidence-validator name self-test, full validator,
current manifest writer, and schema comparator exited 0. The comparator reported
matching profile counts/names but zero exact schema matches in all profiles.
The full validator continues to report schema parity pending.

The prior snapshot `G:\B3D_R09C_20261106` remains untouched. Its verifier now
reports exactly two differing JSON evidence files because the current-run
manifest and schema comparison were regenerated during this wrap-up. A new
short-path recovery snapshot and its independent verification result are
recorded in the receipt appended after the copy; that receipt postdates the
snapshot. All older snapshots, including the verified read-only R1 recovery
point and incomplete 143410 attempt, remain untouched.

The acceptance table, shortcuts/open tests, file list, and dependency/config/
design findings are in the Review notes entry at the end of
`docs/product-rebuild-plan.md`. RB-09 schema disposition, the full live
cross-adapter matrix, RB-08 native evidence, interface/floor gaps, visual review,
owner acceptance, and overall product acceptance remain open. RB-10 has not
started.

### Wrap-up recovery snapshot receipt (appended after copy)

Created new additive review/recovery snapshot `G:\B3D_R09W_20261106` on
2026-11-06 at 11:35:42 local time; its absolute path length is 20 characters.
Robocopy exited 1 (3991 files copied, zero failed). The independent verifier
passed with 3991 files / 756853562 bytes on both sides; missing/extra/differing
paths 0/0/0; source/snapshot reparse entries 0/0; and all 8 critical SHA-256
comparisons matched. Root `target`, `.cache`, and `logs` were excluded; nested
RB-09 evidence logs and `.git` were copied. Detailed hashes are in
`evidence/rebuild/rb-09-shared-ai-and-regression/snapshot-receipt.md`.

This log entry and the final plan/receipt updates postdate the snapshot, as
recorded in the receipt. It is a recovery copy, not an RB-09 completion snapshot.
The previously verified read-only R1 snapshot and incomplete `143410` attempt
remain untouched; `G:\B3D_R09C_20261106` was also left untouched.
## 2026-11-07 - RB-09 schema-parity repair

Continued only the schema-parity repair in E:\3D-Modeler and stopped before RB-10. Inspected Git status and 3D Modeler processes first; retained the existing tracked changes and untracked files. No stage, commit, push, reset, restore, or older-snapshot modification occurred.

Confirmed the designated L:\Builder3D reference read-only at Git HEAD 3a0160a95c55f68ce9f1e89d17c792816ac001df, Python 3.12.10, and MCP SDK 2.2.0. Captured current tools/list catalogs with the official SDK; did not launch the Python product or invoke handlers. A post-capture check retained the same 223-entry Git status/hash and 21 behavior-critical source hashes. The Python runtime accepts and drops unknown root input fields and rejects an unknown nested apply_actions.args field; Rust now matches both.

The pre-edit Rust return_detail count is 20, not 19. Rust now exposes the five current reference fields in model/files/full and none in observe. Current Rust Release schemas and official MCP annotations match recursively in all four profiles (7/52/59/60), with zero recursive differences. The pinned jsonschema 0.60.2 validator checks full nested Draft 2020-12 schemas across all four profiles. The official Tools specification and Rust SDK assumptions are recorded in schema-parity-run-manifest.json.

Source changes added the typed contract and generator, full input/output validation, result-shape alignment, Python-compatible root-extra handling, and corrected annotations. jsonschema 0.60.2 is pinned in the MCP crate/lockfile; deny.toml has a version-specific MIT-0 exception for borrow-or-share 0.2.4.

Verification: focused MCP tests passed 26/26; the focused Release-output schema test passed 1/1 on 25 successful structured results. Workspace tests passed 336/336; current-reference MCP tests passed 9/9. cargo fmt --all -- --check, cargo check --workspace --locked, cargo clippy --workspace --all-targets --all-features --locked -- -D warnings, cargo metadata, cargo deny check, and explicit app Release bloat all passed. The app text section measured 16.0 MiB and its executable was 19.6 MiB. cargo-audit was unavailable because the tool is not installed. The full RB-09 evidence validator and ordered-tool-name self-test passed.

The rebuilt Release GUI/app SHA-256 is 0e874b6fb06d9b6cb5e70517ca35856d3a8304975e2a0842cd971f20df2c45ba; gateway SHA-256 is 636171b58800915de87fc8ad26814498279919ea80341076a344c208757da370. The official SDK received 25 successful structured outputs, including new_project and completed operation_status; the nested-extra apply_actions probe returned invalid_input. All 25 successful results passed the full output-schema test. The GUI exited normally and removed its runtime descriptor.

The pre-edit G: snapshot and additional G:\B3D_R09S_20261107 recovery copy are recorded in evidence/rebuild/rb-09-shared-ai-and-regression/snapshot-receipt.md. The pre-edit copy was verified before source edits at 3,743 files / 755,179,156 bytes with 17 critical SHA-256 matches. The additional copy's initial check passed at 4,078 files / 766,744,485 bytes with 23 critical hash matches. After synchronizing its evidence updates, final verification passed at 4,079 files / 766,761,893 bytes on both sides, with zero missing/extra/different paths or reparse entries and no SHA-256 differences across included files. The receipt and summary of this final check postdate the snapshot. G:\B3D_R09W_20261106 and all older snapshots were left untouched.

The schema criterion is complete. One custom emboss_text _meta.com.modeler3d/longRunning difference remains disclosed without owner approval. RB-08 native evidence, the interface/floor gaps, visual review, owner acceptance, overall product acceptance, and full RB-09 acceptance remain pending. RB-10 has not started.

## 2026-11-09 - Owner visual feedback follow-up

Continued the owner's visual follow-up in `E:\3D-Modeler` and stopped before RB-10. Compared the supplied 3D Builder floor screenshot and saved Python shell captures. The Python reference was not launched or modified.

Before the camera source edit, created the additive recovery snapshot `G:\B3D_RV_20261109_floorfix_pre`. It contains 4,096 files / 771,406,618 bytes (root `target` excluded to match the recovery-copy policy); source and snapshot counts and bytes match, and all 10 critical SHA-256 comparisons match. The receipt is `evidence/rebuild/rb-08-preferences-and-complete-shell/owner-feedback-followup-20261109/recovery-snapshot.json`. The existing `G:\B3D_RV_20261109` and older snapshots remain untouched.

The live empty-scene floor had been clipped between the camera and the scene center because the near plane could sit almost at the target. The camera now caps that plane at 2% of the camera distance while respecting the scene-derived limit. A regression test projects a near-side floor point for the empty-scene view. The existing visual changes retain the compact ribbon, labeled/briefly described shortcut buttons with reorder and pin/unpin controls, useful tooltips, the three-line dropdown menu, and the reference selection actions in the side panel.

The Release window was rebuilt and left open on the Side view. The floor fills the viewport, uses 50 mm checker cells and metric labels, and fades toward the far side. Top and Side captures both show floor coverage. The hamburger opens the dropdown. Captures and checks are in `evidence/rebuild/rb-08-preferences-and-complete-shell/owner-feedback-followup-20261109/`.

Verification: the focused near-floor camera test passed; `cargo test --workspace --locked` passed on retry (the first full run had one loopback connection reset, and its isolated rerun passed); `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo fmt --all -- --check`, `cargo deny check`, and the Release app build all exited 0. Cargo Deny reported existing duplicate-version warnings and a yanked locked crate while advisories, bans, licenses, and sources passed.

This records implementation and current GUI evidence only. It does not claim owner visual acceptance, close the RB-08 native matrix or overall product acceptance, or start RB-10.

The owner then clarified that only the splash screen should stay in front of
other windows. The main Release window had only been temporarily set to stay in
front during review. It was reopened normally, verified visible and not
topmost, and left open for the next review session. No application source
change was needed; splash-screen behavior was not changed.

## 2026-11-09 - Metric floor labels

The supplied 3D Builder screenshots show 10 mm checker squares, with 50 mm and
100 mm labels every five and ten squares. Rust used 50 mm checker cells and
put the labels on one camera-relative diagonal, so the values did not track the
two visible floor axes. Set the checker size to 10 mm, anchored `0 mm` at the
axis intersection, placed labels along both axes in 50 mm steps, and changed
the opening/reset yaw to 0 degrees to align the axes with the reference. The
product plan summary now records the updated floor treatment.

Verification: `cargo fmt --all -- --check`,
`cargo check --locked -p modeler3d-viewport -p modeler3d-app`, and
`cargo build --release --locked -p modeler3d-app` passed. No tests or GUI run
were performed.

Recovery snapshot: `G:\B3D_RV_20261109-102620_floorlabels`. Robocopy exited 1
with no failures; the source and snapshot each contain 4,102 files and
773,047,551 bytes. Root `target`, `.cache`, and `logs` directories were
excluded. Existing snapshots were left untouched.

## 2026-11-09 - New primitives rest on the build floor

New primitives were inserted at the identity transform, so centered geometry
extended below the Y=0 floor. `Scene::add_primitive` now centers the primitive's
X/Z bounds and translates its lowest Y vertex to the floor before adding it in
one history command. This applies to desktop, core-command, and MCP primitive
creation. Model-file imports already center their combined bounds onto the
floor and were left unchanged. Existing position expectations were updated for
the new primitive placement.

Verification: `cargo fmt --all -- --check`, `git diff --check`,
`cargo check --workspace --locked`, and
`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
passed. Tests and GUI review were not run.

Recovery snapshot: `G:\Builder3D-Rust_snapshot_20261109-112236_primitive-floor`.
Robocopy exited 1 with no failed files. Before this receipt was appended, source
and snapshot each contained 4,102 files / 773,048,580 bytes; relative-path
differences: 0; nine critical SHA-256 comparisons matched. The appended receipt
is the expected post-copy difference. Root `target`, `.cache`, and `logs`
directories were excluded. Existing snapshots were left untouched. No RB-10
work started; RB-09 and owner-acceptance statuses remain unchanged.

## 2026-11-09 - Refresh floor cues after viewport moves

A transform can commit during viewport input handling after the initial render
preview is built. That let the renderer pair the new scene revision with old
object bounds and cache the old floor contact cue under the new revision.
Refresh the preview and gizmo after routing input and before rendering, keeping
the cue cache's scene revision and bounds in sync.

Verification: `cargo fmt --all -- --check`, `git diff --check`,
`cargo check --workspace --locked`,
`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`,
and `cargo build --release --locked -p modeler3d-app` passed. The Release app
opened as PID 22264 with a responsive `3D Modeler - Untitled` window. Tests were
not run.

Recovery snapshot: `G:\Builder3D-Rust_snapshot_20261109-131628_contact-cue-refresh`.
Robocopy exited 1 with no failures; source and snapshot each contain 4,102
files / 773,051,025 bytes, with 0 relative-path differences and matching SHA-256
hashes for the changed shell source and work log. Root `target`, `.cache`, and
`logs` directories were excluded. The receipt appended after the copy is the
expected post-copy difference. Existing snapshots were left untouched.

## 2026-11-09 - Screen-aligned viewport lighting

Reviewed `erros/3Dviewer.png` and `erros/Shadows on objects.png`. Mesh shading
used a fixed light direction without transforming normals with the object and
camera, while the CPU fallback drew meshes unlit. Added a screen-aligned key
light by default, inverse-transpose normal transforms, diffuse shading,
Blinn-Phong highlights, and a subtle rim light in both renderer paths. The
Settings panel now persists controls for light alignment, direction, strength,
ambient fill, highlights, and floor contact shadows, with a one-click restore
for the reference-style defaults.

Verification: formatted the changed Rust sources with `rustfmt --edition 2024`.
Tests, build, and live visual review were not run. No G: snapshot was created;
workspace instructions require explicit authorization before adding to G:.
This records implementation only and does not claim viewport or product visual
acceptance.

## 2026-11-10 - Lighting build and runtime check

The first Cargo run found that `viewport_lighting` had been routed through the
legacy scalar preference parser. It is now loaded directly as a JSON object,
with an unsupported-value warning and the normal default fallback.

Verification: the Release app build passed and is open as PID 27100 with a
responsive `3D Modeler - Untitled` window. All 36 `modeler3d-viewport` tests and
the app preference round-trip test passed. The full workspace run reached the
app tests: 162 passed and four failed in existing placement-height and camera
preset expectations; none exercised lighting. Those unrelated expectations
were left unchanged. This runtime check is not owner visual acceptance.

## 2026-11-10 - Match 3D Builder shading and dock settings

Used `erros/Shading.png` to adjust the viewport defaults. Mesh normals now split
at sharp edges while remaining smooth across shallow curved surfaces, so cube
faces keep even lighting without making spheres faceted. The default light stays
screen-aligned and diffuse-only; removed the added rim glow. Floor shadows now
project from bounded object extents opposite the current key light, use layered
translucent polygons for softer edges, and remain visible during camera orbit
once their geometry is cached.

Settings now docks on the left between the measured ribbon and lower controls,
scrolls within that space, and leaves viewport orbit enabled outside the panel.
Pointer input over the panel is kept out of the viewport gesture handler. The
restore button now applies the 3D Builder-style defaults.

Verification: `rustfmt --edition 2024 --check` passed; all 39
`modeler3d-viewport` tests and 8 Settings/dialog tests passed; the Release build
in `target/lighting-review` passed; `modeler3d-app.exe --smoke` passed; and
`git diff --check` passed. The review window is open as PID 23560 and responding.
No full workspace suite or screenshot-based visual review was run. No G: snapshot
was created because workspace instructions require explicit authorization for G:
writes. Owner visual review remains pending.

## 2026-11-10 - Add bounded UI and viewport diagnostics

Added a standard-library logger at the platform log path
(`%LOCALAPPDATA%\Modeler3D\logs\modeler3d.log` on Windows). It records startup,
viewport, scene-preview, and optional MCP errors, CPU fallback transitions,
slow viewport/UI frames, and unhandled panic backtraces. The current log is
bounded at 2 MiB and retains one rotated file. Normal frames do not write files;
slow-frame reports are limited to one per context every 10 seconds. A background
watchdog records a warning after 15 seconds without a completed UI frame and a
recovery entry when frames resume. Egui schedules one idle heartbeat repaint per
10 seconds so an idle window is not mistaken for a hang.

The recent mesh-normal preparation remains tied to scene rebuilds, and floor
shadow overlays are bounded. No freeze was reproduced during the idle launch;
the watchdog is intended to capture timing if the problem recurs during use.

Verification: `cargo fmt --all -- --check`, `cargo check --locked -p
modeler3d-app`, `cargo clippy --locked -p modeler3d-app --all-targets -- -D
warnings`, the two `diagnostics::tests`, and all 39 `modeler3d-viewport` tests
passed. The Release app build took 3m39s, `modeler3d-app.exe --smoke` passed,
and the visible Release window (PID 41788) remained responsive through 20 seconds
of idle. The initial startup log recorded the Quadro RTX 4000 on Vulkan.
No interactive reproduction of the owner's freeze or full workspace suite was
run. This does not establish that the freeze is fixed.

Recovery snapshot: `G:\Builder3D-Rust_snapshot_20261110-093445_ui-diagnostics`.
Robocopy exited 1 with zero failed files. Source and snapshot each contain
4,104 files / 773,274,097 bytes after excluding root `target`, `.cache`, and
`logs`; SHA-256 hashes matched for the four changed Rust sources and this work
log. The snapshot receipt above was appended after the copy.

## 2026-11-10 - Restore mouse orbit in the viewport

Mouse orbit was blocked by global egui keyboard-focus and pointer-over-UI
checks. Viewport gesture routing now uses the viewport widget's own pointer
hit-test, while allowing an active drag to continue when it crosses UI panels.
Modal and other interaction blocking still applies. Added a regression check
showing that a focused text field does not prevent a mouse gesture over the
viewport.

Verification: the five `viewport_` app tests, `cargo fmt --all -- --check`,
Clippy with warnings denied, the optimized app build, and `modeler3d-app.exe
--smoke` passed. In the visible Release app (PID 26192), native left-button
drags in both empty viewport space and on the model visibly orbited the camera;
the model's transform fields stayed at zero during the surface drag. Added a
Cube from the ribbon and left the responsive window open for owner testing.
Startup diagnostics recorded the Quadro RTX 4000 on Vulkan with no errors.
This interactive check does not replace owner review.

Recovery snapshot: `G:\Builder3D-Rust_snapshot_20261110-101321_viewport-orbit-fix`.
Robocopy exited 1 with zero failures. Source and snapshot each contain 4,104
files / 773,276,725 bytes, with zero missing, extra, size-different, or reparse
entries. SHA-256 hashes for `shell.rs`, `diagnostics.rs`, `lib.rs`, and this
work log matched. This snapshot receipt was appended after the copy.

## 2026-11-10 - Keep floor scale labels attached during pan

Fixed floor scale-label placement in `build_floor_scale_labels`. Labels now
use fixed world-axis coordinates instead of offsets from `camera.center()`,
so right-button pan moves them across the view with the floor grid. Added the
`floor_scale_labels_remain_world_anchored_when_camera_pans` regression.

Verification: all 40 `modeler3d-viewport` tests passed; formatting and focused
Clippy passed; the optimized app build and shell smoke passed. A visible
Release right-drag moved the floor grid and its labels together, and the app
remained responsive. Before/after captures and SHA-256 values are recorded in
`evidence/rebuild/rb-05-viewport-and-input-completion/verification.md`. The
window is left open for owner testing.

Recovery snapshot: `G:\Builder3D-Rust_snapshot_20261110-113031_floor-label-pan-fix`.
Robocopy exited 1 with zero failed files (4,107 copied). An independent
full-tree SHA-256 verification found 4,107 files / 773,776,992 bytes on each
side, zero missing/extra/different paths, and zero reparse entries. Root
`target`, `.cache`, and `logs` were excluded. Critical hashes matched for the
floor-label source, product plan, RB-05 map and verification, before/after
captures, and `WORK_LOG.md` as it existed at copy time. This receipt was
appended after the snapshot; the snapshot remains untouched.

## 2026-11-10 - Refine move gizmo arrows

Move handles now use one camera-facing arrow per world axis. Each shaft starts just outside the selected object's near face and ends in a compact 12-facet head with a subtle graphite outline. X/Y/Z colors stay unchanged, and drawing and hit-testing share the same axis segment geometry. The focused geometry assertion now covers outward-facing handles.

Verification: `cargo fmt --check -p modeler3d-viewport`, `cargo check --locked -p modeler3d-app`, and `cargo clippy --locked -p modeler3d-viewport --all-targets -- -D warnings` passed. Automated tests were not run.

The existing Release window (PID 20188) still has an unsaved `Untitled *` scene and locks `target/release/modeler3d-app.exe`. I left it open; the Release rebuild could not replace the executable (`Access is denied`). The updated source is not live in that process. The pre-change window capture is `target/gizmo-arrow-open-app.png` and is not visual acceptance evidence. No G: snapshot was created because workspace-root instructions require explicit authorization for backup writes.

### Reload follow-up

The standard Release build now succeeds (`cargo build --release --locked -p modeler3d-app`, 3m42s), and the refreshed app was launched as PID 12228. It starts as `3D Modeler - Untitled`. The Windows foreground is currently `Windows Default Lock Screen`, so no further input or screenshot was sent. The app remains open for review; select a model after unlocking to display its gizmo.

### Arrow-tip clipping follow-up

The egui gizmo overlay now clips line segments and arrowhead triangles against
the camera frustum before perspective division. This prevents geometry that
crosses the near plane from stretching into viewport-wide streaks.

Verification: `cargo fmt --check -p modeler3d-app`,
`cargo check --locked -p modeler3d-app`, and
`cargo clippy --locked -p modeler3d-app -- -D warnings` passed. The optimized
Release build succeeded, and the refreshed app is open as PID 19108. It starts
as `3D Modeler - Untitled`; select a model to see the gizmo. Automated tests
were not run.

## 2026-11-11 - Bound Move gizmos in screen space

A capture of the running Release window confirmed that frustum clipping alone
did not remove the long strokes. Move mode now draws each shaft at a fixed
screen length and uses a compact outlined screen-space triangle for its head.
The projected world-axis segment supplies direction and placement; world-space
hit testing is unchanged. Rotate and Resize keep their clipped 3D geometry.

Visual check: an isolated debug preview with a temporary sphere showed all
three arrows without viewport streaks in
`target/gizmo-preview-after-fixed-v3.png`. The preview-only startup hook was
removed before the final build.

Verification: `cargo fmt --check -p modeler3d-app`,
`cargo check --locked -p modeler3d-app`,
`cargo clippy --locked -p modeler3d-app -- -D warnings`, the Debug build, and
the optimized Release build passed. Automated tests were not run. The previous
Release process had exited before the refreshed standard Release app was
launched as PID 42268 (`3D Modeler - Untitled`).

## 2026-11-11 - Enlarge zoom-stable Move arrows

Move arrows now use a 3x size scale in screen-space drawing and their matching
world-space geometry and hit testing. The shaft, outline, and arrowhead grow
together, while camera-scaled world lengths keep the target the same size as
the view zoom changes. Rotate and Resize handles keep their existing size.

Verification: `cargo fmt --check -p modeler3d-app -p modeler3d-viewport`,
`cargo check --locked -p modeler3d-app`,
`cargo clippy --locked -p modeler3d-app -- -D warnings`, and the optimized
Release build passed. The pinned `Initialize-BuildClang.ps1` script supplied
libclang for Cargo. The refreshed Release window is open as PID 27132
(`3D Modeler - Untitled`); add or select a model to view the arrows. Automated
tests were not run.

Follow-up: shortened the Move shaft to one arrowhead length and adjusted its
world-space tip and hit geometry to match the new visible endpoint.

Verification: formatting, `cargo check --locked -p modeler3d-app`,
`cargo clippy --locked -p modeler3d-app -- -D warnings`, and the optimized
Release build passed. Automated tests were not run. The refreshed app is open
as PID 36688 (`3D Modeler - Untitled`); add or select a model to view the arrows.

## 2026-11-11 - Add arrow grips to Rotate and Resize

Rotate mode retains its full rings and adds a curved highlighted section ending
in the shared outlined arrowhead. Resize face grips now use outward arrows with
two consecutive arrowheads for a barbed look; corner grips remain. Picking
covers the visible arrow length, and resize starts preserve the pointer offset
so grabbing an arrow away from its face does not jump the model.

Verification: formatting, `cargo check --locked -p modeler3d-app`, and
`cargo clippy --locked -p modeler3d-app -- -D warnings` passed. The optimized
Release build passed to the isolated output
`target/gizmo-arrow-review/release/modeler3d-app.exe`. The standard Release
window (PID 36688) has an unsaved `Untitled *` scene and was left open. No
refreshed window was launched. Automated tests were not run.

A subsequent check confirmed PID 36688 was still using
`target/release/modeler3d-app.exe`, so the standard Release executable remains
untouched until that unsaved window closes. The isolated review executable is
the updated build for now.

## 2026-11-11 - Refine transform gizmo appearance and picking

Move, Resize, and Rotate handles now share outlined shafts and faceted arrowheads, with axis-color, hover, and active-drag emphasis. Move and Resize picking follows the painted screen-space geometry with pixels-per-point scaling. Resize face and corner drags preserve the initial pointer offset.

Visual review captures are in `target/gizmo-review-final-move.png`, `target/gizmo-review-final-resize.png`, `target/gizmo-review-final-rotate.png`, and `target/gizmo-review-hover-resize.png`. The updated review app is running as PID 40424 from `target/release/modeler3d-app.exe`, with an unsaved Untitled cube, left open for owner review.

Verification: `cargo fmt --all -- --check`, `git diff --check`, `cargo check --locked -p modeler3d-app`, and `cargo clippy --locked -p modeler3d-app --lib -- -D warnings` passed. The default fat-LTO Release link failed with LLVM out-of-memory; the review build succeeded with LTO disabled and 16 codegen units. `cargo clippy --all-targets` remains blocked by pre-existing test references to the removed `Modeler3dApp::project_handle`. Automated tests were not run. Product acceptance status is unchanged.

## 2026-11-16 - Rename the app to 3D Modeler

Owner decision: the app is renamed from Builder3D to 3D Modeler. Display name "3D Modeler"; program file, crates, settings folder, environment variables and MCP server use `Modeler3D` / `modeler3d` (as Microsoft did with 3D Builder / `Builder3D.exe`). Settings start fresh; old records were renamed too. The places that keep the old name are listed in the "Name" section of `AGENTS.md`.

Pre-rename snapshot `G:\Builder3D-Rust_snapshot_20261116-094924_pre-rename-3D-Modeler`: 8,624 files and 1,216,896,280 bytes matched the source exactly (`target` excluded).

Done by script over 844 text files and 23 file/folder names, then hand fixes where a program-file name was needed. Recorded hashes and byte counts inside older evidence receipts describe the files as they were before the rename. Two old logs with mixed encoding (`evidence/rebuild/rb-03-lifecycle-and-file-completion/logs/cargo-clippy-workspace.log`, `cargo-test-workspace.log`) and binary captures were left as they were.

Folder renamed to `E:\3D-Modeler` and `E:\3D-Modeler-RC` the same day (an Explorer window on `target\release` had held the old folder). Debug build and `--smoke` pass from the new path. Full gate passed before the commit: fmt, clippy, 483 tests, deny, audit, MCP parity, smoke, capture board. Snapshot `G:\3D-Modeler_snapshot_20261116-104006_rename-complete`: 9,647 files and 1,251,402,190 bytes matched the source exactly (`target` excluded).

## 2026-11-16 - OF-65 transform strip clean-up

The transform strip no longer shows the "N selected / mm" note; each Move / Rotate / Resize label now sits centred over its X / Y / Z box (owner follow-up the same day); Rotate shows "0.000 deg", not "-0.000 deg" (owner yes). Test `transform_strip_has_no_count_and_each_tab_is_centred_over_its_box`. Pack `evidence/rebuild/captures/2026-11-16-of65-transform-strip/`. Also re-tracked `packaging/launchers/Modeler3D.bat`, untracked by mistake in `0156931` (`0297911`). Full gate passed; release exe rebuilt (4m08s). Snapshot `G:\3D-Modeler_snapshot_20261116-121325_OF65-transform-strip`: 9,683 files and 1,275,301,662 bytes matched the source exactly (`target`, `logs` excluded). Owner review pending.

OF-65 follow-ups the same day (owner): tabs centred over their boxes (`e6e03bf`), Rotate shows "0.000" not "-0.000" (`02dba59`). Full gate passed after each; release exe rebuilt (4m06s). Snapshot `G:D-Modeler_snapshot_20261116-133446_OF65-followups`: 10,009 files and 1,338,273,933 bytes matched the source exactly (`target`, `logs` excluded).

## 2026-11-16 - Linux RC2 package and Mac kit on the USB card

Linux build in WSL Ubuntu 24.04 (`~/3D-Modeler`). Fixes, one commit each: the `windows` crate became a Windows-only dependency of `modeler3d-platform` (`de5a2e3`); the Windows runtime-path test joins with the host separator (`3816e3f`); the registry decoder may go unused off Windows (`0651688`); the desktop `frame_view` answer stays inside its output schema, which every gateway call failed before (`20edded`); CPU viewport frames are copied into the displayed texture, which was grey on software-only adapters (`8ab250c`, test `cpu_frames_reach_the_displayed_colour_target`, runs on the Windows GPU, skips in WSL). Linux: 487 tests, clippy, fmt pass. Package `E:\3D-Modeler-RC\3D-Modeler-0.1.2-linux-x86_64.tar.gz` (sha256 `20de8a55...d8ee`), built by `packaging/linux/build-tarball.sh`; the unpacked package, with the source tree moved away, ran under WSLg and was driven through `r4_mcp`: insert, save, reopen, capture. AppImage not built: the `appimagetool` download was blocked and needs the owner's OK. Full Windows gate passed after the fixes (16:38). USB card `L:\3D-Modeler-packaging\`: `linux\` (same sha256), `3D-Modeler\` (clone of `rebuild/work` at `44791b0`), `mac\` (empty). Mac steps: `docs/packaging-linux-mac.md`, "Mac package from the USB"; `packaging/macos/build-dmg.sh` not yet run on a Mac. Snapshot `G:\3D-Modeler_snapshot_20261116-164334_linux-rc2-mac-usb`: 10,413 files and 1,401,131,129 bytes matched the source exactly (`target`, `logs` excluded).

## 2026-11-18 - Rotate ring occlusion

Rotate ring strokes and curved grips are clipped against the live oriented
selection frame in the UI painter, preventing linework from crossing the
object silhouette. The renderer-neutral gizmo also sizes rings from selected
bounds and culls rear segments. Picking, rotation math, history and MCP
behavior remain unchanged. Added the projected-segment regression in
`shell::rotate_controls`.

Fast gate passed for `modeler3d-app` and `modeler3d-viewport` (269 + 56 tests).
Release build and `--smoke` passed; light and dark Release captures are in
`evidence/rebuild/captures/2026-11-18-rotate-occlusion/`. Owner review pending.
Recovery snapshot `G:\3D-Modeler_snapshot_20261118-rotate-occlusion-final`:
13,104 files and 1,882,744,593 bytes matched the source, excluding `target`
and `logs`.

## 2026-11-19 - STATUS.md entries moved out
F5 and F6 closed 2026-11-16 (ledger parity test passed after F6). Full gate passed 2026-11-16 after OF-65 (fmt, clippy, tests, deny, audit, MCP parity, smoke, capture board). Before that, full gate passed 2026-11-16 after Phase 6 OF-63/OF-64 (fmt, clippy, tests, deny, audit, MCP parity, smoke, capture board). Earlier: full gate passed 2026-11-15 after Phase 5 OF-58 (fmt, clippy, tests, deny, audit, MCP parity with no exemptions, smoke, capture board); a last small fix then passed the Fast gate. Release exe rebuilt from that source (3m49s) and driven live through the real MCP gateway for the Phase 5 review pack.


**Resize reference toggle (owner trial, 2026-11-18):** icon-only
floor/object switch above Apply, with tooltips. Floor uses fixed floor axes;
Object follows imported local axes as the object turns. Multiple objects use
the first object's frame in model order. Handles, live dimensions and numeric
Apply use the chosen frame; switching does not dirty the document or add Undo.
Full gate passed (`logs/gate/20261118-112226-full`), including 274 app and 57
viewport tests, MCP parity and 96 captures. Release built and smoke passed
(23,402,496-byte executable). Pack: `evidence/rebuild/captures/2026-11-18-resize-basis/`.
Task: [`resize-basis-toggle.md`](tasks/resize-basis-toggle.md).
Built for the owner to try. Recovery: `G:\3D-Modeler_snapshot_20261118-112934_resize-basis-toggle`.

**Rotate grips from owner reference (2026-11-18):** `erros/rotate.png`
sets the grip style: filled double-ended tips joined by a narrower solid
stem. Grips follow short curved arcs; rings stay thin. Mesh depth testing
hides rear strokes and filled grips on GPU and CPU. Placement avoids folded
edge-on grips and favors space between axes. Rotation picking, preview and
history pass. Full gate passed (`logs/gate/20261118-104915-full`), with
271 app and 57 viewport tests and 92 captures. Release built and smoke passed
(23,393,792-byte executable). Both-theme comparisons and Release captures:
`evidence/rebuild/captures/2026-11-18-rotate-reference/`.
Task: [`rotate-reference.md`](tasks/rotate-reference.md). Owner-approved 2026-11-18 ("awesome!"). Done.
Recovery: `G:\3D-Modeler_snapshot_20261118-105552_rotate-reference`.

**Rotation ring finish (owner, 2026-11-18):** continuous shaded tube paths
with soft depth edge and specular glint; curved grips receive matching edge
definition; rings use 96 segments for smoother arcs. Picking and transform
behavior are unchanged. Fast gate passed (269 app + 56 viewport tests); Release
build and smoke passed. Both-theme before/after pack:
`evidence/rebuild/captures/2026-11-18-rotate-finish/`.
**Owner rejected the visual result, 2026-11-18:** "it still look awful (like a child did it)".
This earlier treatment is superseded by the owner-approved reference grips
above. The rotation finish task is closed.
Task: [`rotation-finish.md`](tasks/rotation-finish.md). Recovery:
`G:\3D-Modeler_snapshot_20261118-085850_rotation-finish`.

**Rotate ring occlusion (owner, 2026-11-18):** rear ring strokes and curved
grips clip at the rotated selection silhouette, so no lines cross the object
faces. Picking, visible grips and transform behavior are unchanged. Fast gate
passed for the app and viewport crates (269 + 56 tests); Release build and
smoke passed. Both-theme captures:
`evidence/rebuild/captures/2026-11-18-rotate-occlusion/`.
Task: [`rotation-occlusion.md`](tasks/rotation-occlusion.md).
Superseded by the owner-approved reference grips above. Recovery:
`G:\3D-Modeler_snapshot_20261118-rotate-occlusion-final` (13,104 files,
1,882,744,593 bytes verified).

**Rotation controls follow the object (owner, 2026-11-18):** local rings and
arrow grips follow committed orientation and the live drag. Painting, picking
and drag axes share the same frame; multiple selected objects follow the first
selected object's orientation. Regression covers single/multiple selection,
local-axis picking, preview, cancellation, commit, undo and redo. Independent
diff review found no remaining blockers. Full gate passed (`logs/gate/20261118-071412-full`), including 46 states
in both themes. Release build, smoke and both-theme captures passed. Pack: `evidence/rebuild/captures/2026-11-18-rotate-follow/`.
Task: [`rotate-follow.md`](tasks/rotate-follow.md). Included in the owner-approved Rotate result above.
Recovery: `G:\3D-Modeler_snapshot_20261118-072319_rotate-follow-checkpoint`.


**OF-69 Move arrows (2026-11-17):** flat axis-colored stems and solid
triangular heads consistent with Resize. Axis colors, directions, dimensions,
picking, hover/drag emphasis and gesture/history paths retained. MCP translation
semantics unchanged. Independent code and both-theme capture review found no
blockers (requested reviewer models unavailable; inherited-model review used).
Fast and Full gates passed (`logs/gate/20261117-183431-fast`,
`logs/gate/20261117-183452-full`), including the 44-state, both-theme board.
Release rebuilt (4m14s, 23,369,216 bytes); Release smoke and captures passed.
Pack: `evidence/rebuild/captures/2026-11-17-of69-move/`.
Task: [`move-arrows.md`](tasks/move-arrows.md).
**Done, awaiting owner review. OF-70 remains queued; Rotate approved 2026-11-18.**
Baseline: `G:\3D-Modeler_snapshot_20261117-183334_OF69-baseline`
(12,195 files, 1,770,275,521 bytes verified).
Recovery: `G:\3D-Modeler_snapshot_20261117-184100_OF69`
(additive copy; file and byte counts verified).

**OF-68 rotation arrows (2026-11-17):** original curved grips with solid flat
triangular heads pointing both ways, inspired by the owner reference and Resize.
Axis colors, rings, hover emphasis and existing picking retained; MCP rotation
semantics unchanged. Independent diff review found no remaining blockers after
viewport-edge fixes (requested reviewer models unavailable; inherited-model
independent review used). Fast and Full gates passed, including the 44-state,
both-theme capture board (`logs/gate/20261117-175642-full`); final clipping
changes also passed Fast (`logs/gate/20261117-175825-fast`).
Pack: `evidence/rebuild/captures/2026-11-17-of68-rotation/`.
Task: [`rotation-arrows.md`](tasks/rotation-arrows.md).
Final Release rebuilt; Release smoke and both-theme captures passed.
Recovery: `G:\3D-Modeler_snapshot_20261117-180620_OF68`
(additive copy, file and byte counts verified).
**Owner-approved through the reference Rotate grips, 2026-11-18. OF-69 and OF-70 remain queued.**
Baseline: `G:\3D-Modeler_snapshot_20261117-175442_OF68-baseline`
(11,995 files, 1,734,687,424 bytes verified against the pre-edit tree).


**Resize OF-66 / OF-67 (owner, 2026-11-17):** removed the size label above
Resize; the existing XYZ boxes follow the uncommitted mesh preview in display
units, including all axes affected by corner scaling. Escape, focus loss and
blocked input restore scene sizes. Numeric entry and gesture/history math
remain unchanged. Fast and Full gates passed: 508 workspace tests, fmt,
Clippy, deny, audit, MCP parity, smoke, 44 states x 2 themes
(`logs/gate/20261117-155921-full`). Independent code review found no blockers.
Release rebuilt (4m12s, 23,370,240 bytes); Release smoke passed. Both-theme
before/after and Release captures:
`evidence/rebuild/captures/2026-11-17-of66-67-resize/`.
**Done, awaiting owner review. OF-68 follows below; OF-69 and OF-70 remain queued.**
Feedback: [`owner-feedback-2026-11-17.md`](owner-feedback-2026-11-17.md).
Task: [`resize-live-sizes.md`](tasks/resize-live-sizes.md).
Recovery: `G:\3D-Modeler_snapshot_20261117-160600_OF66-67`
(copy and file/byte counts verified).
Baseline: `G:\3D-Modeler_snapshot_20261117-155214_OF66-67-baseline`
(11,778 files, 1,697,810,506 bytes; counts verified).

**Edit tools independent of selection (owner, 2026-11-17):** all 12 Edit
buttons can be chosen, switched and dismissed with no selection, including an
empty scene, from both the ribbon and favorites. Switching closes previous
controls. Smooth and Boolean tools chosen without a target offer nonmodal
Apply controls; invocation with an existing selection retains its behavior.
Mesh application still validates its target; tool changes add no undo entry.
Full gate passed: 507 tests, fmt, Clippy, deny, audit, MCP parity, smoke and
44 states x 2 themes (`logs/gate/20261117-130618-full`). Tests cover actual
egui clicks on both button surfaces, all tool pairs, clicking off the model,
selection resumption and one-step undo/redo. Both-theme before/after and Release
captures: `evidence/rebuild/captures/2026-11-17-edit-tools/`. Release rebuilt
(4m16s, 23,368,704 bytes); Release smoke passed. **Owner review pending.**
Task: [`edit-tools-selection.md`](tasks/edit-tools-selection.md).
Recovery: `G:\3D-Modeler_snapshot_20261117-131300_edit-tools`.

**Import (owner, 2026-11-17):** menu Import sits above New Scene and opens
an all-files native picker. First use resolves Documents (including Windows
Known Folder redirection); later uses remember the selected folder across
restarts. All eight export formats import: STL, OBJ, PLY, 3MF, glTF, GLB,
WRL and 3DP. Desktop and MCP share one atomic import/placement service.
WRL supports static triangulated IndexedFaceSet scenes, including app exports;
other VRML nodes return an explicit error. Project imports preserve groups,
metadata, textures and visibility with fresh IDs. Final checks passed:
502 tests, fmt, Clippy, deny, audit, MCP parity and smoke
(`logs/gate/20261117-123551-full`); all 40 states x 2 themes captured
separately in `gate-20261117-123113`, including the new menu.
Release rebuilt (4m15s, 23,362,048 bytes), smoke and both menu captures passed.
Live Release checks through the stdio MCP gateway passed all eight formats
(export/import/undo/redo) and failed-import scene/revision preservation.
Task: [`import.md`](tasks/import.md). **Native picker hands-on and owner
visual review pending.** Recovery: `G:\3D-Modeler_snapshot_20261117-124653_import`.


**Bottom bar and interface text (owner, 2026-11-17):** removed the permanent
status bar; Move/Rotate/Resize sits 12 px above the bottom. Active operations
retain a temporary progress/Cancel window. Home slogan is "Imagine, Design,
Play"; corrupted brush punctuation, face-preview arrow and shortcut-order
arrows fixed. Workspace checks passed (`logs/gate/20261117-111144-full`):
498 tests, fmt, Clippy, deny, audit, parity and smoke. The 40-state x 2-theme
capture board passed separately (`gate-20261117-110823`); its initial Clippy
failure was corrected before the final checks. Final slogan/layout captured
again from Release in both themes. Release rebuilt (4m06s, 23,335,936 bytes),
release smoke passed. Task and before/after evidence: [`bottom-text.md`](tasks/bottom-text.md).
**Owner visual review pending.** Recovery: `G:\3D-Modeler_snapshot_20261117-111816_bottom-text`.

**Caption and modifier toggles (owner, 2026-11-17):** Windows title bar follows
the menu theme. Clicking an active brush or modifier again closes its controls,
including after selection is lost. Full gate passed
(`logs/gate/20261117-104840-full`): 497 tests, fmt, Clippy, deny, audit, parity,
smoke, 40 states x 2 themes. Task/captures: [`toolbar-toggle.md`](tasks/toolbar-toggle.md).
Release rebuilt (5m58s, 23,341,056-byte exe) and launched from the active workspace.
Native caption/text setters passed in both themes; native colour readback is
unsupported. **Owner visual review pending.** Recovery:
`G:\3D-Modeler_snapshot_20261117-105524_toolbar-toggle` (robocopy 1;
11,119 files / 1,590,685,157 bytes matched).

**Startup updates (owner, 2026-11-17):** mirror the prompt inside the individual
Craft apps (confirmed in installed CADCraft), checking GitHub at launch and
asking before downloading/installing. Implementation and release preparation:
[`updates.md`](updates.md), task [`startup-updates.md`](tasks/startup-updates.md).
Feed remains unconfigured until the public repository is chosen; configure and
rebuild every platform before the first public build. **Full gate passed**
2026-11-17 (`logs/gate/20261117-101438-full`): 494 tests, fmt, Clippy, deny,
audit (informational warnings), parity, smoke, 38 states × 2 themes. Linux
helper success/invalid-payload/rollback passed in WSL; real stdio client against
an isolated desktop passed (76 tools, unavailable install refused, normal close).
Review pack: `evidence/rebuild/captures/2026-11-17-startup-updates/`, including
the actual CADCraft reference. **Owner visual review pending.** Mac helper and
live Windows/Mac/Linux installed upgrades are not yet validated; do those with
the real feed before public launch. MCP catalog is 76 tools (`2026-11-17.1`).
Recovery snapshot: `G:\3D-Modeler_snapshot_20261117-101915_startup-updates`
(robocopy plus matching source/destination file and byte counts).
Baseline snapshot: `G:\3D-Modeler_snapshot_20261117-094734_pre-updater`
(robocopy 1, 10,114 files / 1,396,721,684 bytes matched). USB Mac commits merged
as `ff8e43e`; DMG copied to `E:\3D-Modeler-RC\3D-Modeler-0.1.2.dmg`, SHA-256
`c90ba253986d89b1a0eba842b367b1db1b6eaf94f847620f3e56bca65d7b600d` matches USB.

**GitHub, 2026-11-16 (owner):** goal is a public repository named "3D Modeler", free to use, donations on; "almost" ready, not uploaded. Commit email chosen; licence (Apache 2.0 suggested) and artwork origin still to confirm. Checklist and open questions: [`github-upload.md`](github-upload.md).

**Bug sweep 2026-11-16 (owner request before the upload):** Full gate passed (all steps, capture board 37 states × 2 themes). New test `crates/modeler3d-core/tests/malformed_inputs.rs`: about 5,000 cut-short or corrupted copies of every importable format and of `.3dp` projects; every one is refused with an error, none crashes (debug and release). Last two `expect()` calls removed from library code (`legacy_settings.rs`). No new bugs found. Still open from before: the faint pale dotted rim at the floor (F9 note) and Emboss (deferred).

**Renamed 2026-11-16:** the app is now **3D Modeler** (was Builder3D); hidden names use `Modeler3D` / `modeler3d`. Where the old name stays, and why, is in the "Name" section of [`AGENTS.md`](../AGENTS.md). Pre-rename backup: `G:\Builder3D-Rust_snapshot_20261116-094924_pre-rename-3D-Modeler`.

**Shortcut bar 2026-11-16 (owner request):** a first start now gets Cube, Sphere, Settle, Mirror, Merge, Subtract. After that the bar is the person's own: every change is saved and kept, and an emptied bar stays empty (before, an empty bar got the defaults back at the next start). Existing settings keep their bar; Customize ribbon → Reset defaults gives the new six. Test `shortcut_bar_starts_with_the_owner_set_and_then_keeps_what_the_person_chose`. An empty bar shows "Drag shortcuts here" in small dim text after the View tab; **owner 2026-11-16: approved ("I love it. shade, size, location, all of it")**. Capture state `no-shortcuts`. **Drag on the bar, 2026-11-16 (owner request):** while a shortcut is dragged, an accent-coloured bar shows the gap it will land in and its icon follows the pointer (no bar where the drop would change nothing); a ribbon button can be dragged up onto the bar and is pinned at that gap (a click still runs it). On a full bar a new shortcut dropped at the end lands before the last visible one, not hidden in "More". `place_favorite_at` (`ui_state.rs`) with tests; `favorites_drop` (`shell.rs`). **Owner 2026-11-16: approved ("great job")** in the release build.

**Owner batch 7** ([`owner-feedback-2026-11-14.md`](owner-feedback-2026-11-14.md),
OF-28 to OF-58, five phases; Emboss OF-56/57 deferred by the owner).
Phase 1 done (OF-28 to OF-45), pack
`evidence/rebuild/captures/2026-11-14-review-phase1/`. Phase 2: OF-44 Paint
and OF-47 X-ray approved by the owner 2026-11-15 (pack
`evidence/rebuild/captures/2026-11-14-review-phase2/`). Owner answers
2026-11-15: OF-35 keep shade A as built; OF-43 confirmed (10 mm gap beside the
original); OF-44 keep the "Done" label; OF-46 store the texture image inside
the project file (`.3dp` format change approved), test textures in
`C:\Users\USER\Pictures\textures\` (owner-supplied; do not commit them).
OF-46 done 2026-11-15 (`654872a`) and approved by the owner 2026-11-15 ("it
works, good enough for now"); the texture dialog is to be revisited later
(last item below). Phase 2 is closed.

Phase 3 (OF-48 to OF-54) built 2026-11-15, pack
`evidence/rebuild/captures/2026-11-14-review-phase3/`. **Owner review
2026-11-15:** Grab, Crease, Smooth Brush, Simplify, Subdivide approved;
Hollow works at a 2 mm wall; Normalize to be removed; the whole-object Smooth
button stays as it is. Follow-ups OF-59 to OF-61 (OF-62 garbled text done).

Follow-ups OF-59 to OF-61 built 2026-11-15 (`57671fb`, `fbbfff8`,
`0238ccf`), pack `evidence/rebuild/captures/2026-11-15-review-phase3-followups/`.
**Owner review 2026-11-15: all three approved.** MCP `normalize_face_density`
stays (owner: keep it in AI for possible later use). Phase 3 is closed.

Phase 4 (OF-55 Measure ruler) built 2026-11-15 (`cfc96a7`), pack
`evidence/rebuild/captures/2026-11-15-review-phase4-measure/`. Measure is now
a click-to-click ruler with snapping; new MCP tool `measure` (catalog 66).
**Owner 2026-11-15: approved ("better than I imagined"; "call it all done").**
Owner's one request, Esc closes Measure in one press, done the same day.
Phase 4 is closed.

Phase 5 (OF-58, the AI does everything) built 2026-11-15, pack
`evidence/rebuild/captures/2026-11-15-review-phase5-mcp/`. Eight new MCP
tools (catalog 74): `app_settings`, `set_app_settings`, `customize_ribbon`,
`show_screen`, `recent_files`, `open_recent`, `capture_window`, `exit_app`,
carried out by the desktop shell with its own code. No "visual only" ledger
rows remain. **Owner 2026-11-16: approved ("it tests great … call it done").**
Phase 5 is closed.

Phase 6 (OF-63 Items double-click centres, OF-64 settings note) built
2026-11-16, pack `evidence/rebuild/captures/2026-11-16-review-phase6/`.
MCP `frame_view` in the live app now frames with the desktop camera (same
camera as a double-click); `app_settings` gains `settings_notice`,
`set_app_settings` gains `dismiss_settings_notice`. OF-65 (transform strip: the
"N selected / mm" note removed; each Move / Rotate / Resize label centred over
its X / Y / Z box (owner follow-up)) built 2026-11-16, pack
`evidence/rebuild/captures/2026-11-16-of65-transform-strip/`, test
`transform_strip_has_no_count_and_each_tab_is_centred_over_its_box`. Rotate now
shows "0.000 deg" instead of "-0.000 deg" (owner yes). **Owner 2026-11-16:
OF-63, OF-64, OF-65 approved ("love it all call it all done").** Phase 6 is
closed.


## Gates on the working tree (2026-11-12, debug build)

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass |
| `cargo test --workspace --locked --no-fail-fast` | Pass (2026-11-12, after the shadow follow-up and the MCP Merge color change) |
| `cargo deny check` | Pass 2026-11-12 (advisories, bans, licenses, sources ok; warning: `async-executor 1.14.0` yanked, Linux accessibility path only) |
| `cargo audit` | Pass 2026-11-12 — 0 vulnerabilities in 493 crates; 5 allowed warnings (4 unmaintained already accepted in `deny.toml`, 1 yanked as above). Tool installed 2026-11-12 |
| MCP catalog parity check | Pass 2026-11-15 (`cargo test -p modeler3d-mcp --test ledger_parity`; all 148 ledger rows map to catalog tools; exemptions now fail, D10) |
| Release (fat LTO) build | `cargo build --release -p modeler3d-app` passed twice on 2026-11-12 (4m50s, 3m43s); failed with LLVM out-of-memory on 2026-11-11 |
| `review` profile build | Pass (3m56s clean build; capture verified) |

## Last review pack

`evidence/rebuild/captures/2026-11-16-of65-transform-strip/` (OF-65) and `evidence/rebuild/captures/2026-11-16-review-phase6/` (OF-63, OF-64), approved by the owner 2026-11-16. `evidence/rebuild/captures/2026-11-15-review-phase5-mcp/` (OF-58), approved
by the owner 2026-11-16. `evidence/rebuild/captures/2026-11-15-review-phase4-measure/` (OF-55),
approved by the owner 2026-11-15. `evidence/rebuild/captures/2026-11-15-review-phase3-followups/` (OF-59 to
OF-61), approved by the owner 2026-11-15. `evidence/rebuild/captures/2026-11-14-review-phase3/` (Phase 3: OF-48 to
OF-54), reviewed by the owner 2026-11-15. `evidence/rebuild/captures/2026-11-14-review-phase2/` (Phase 2: OF-44 and
OF-47 approved 2026-11-15; OF-46 added and approved 2026-11-15). `evidence/rebuild/captures/2026-11-14-review-phase1/`
(Phase 1), not yet reviewed. Before that:

`evidence/rebuild/captures/2026-11-13-review-of1-9-f9/`, reviewed by the
owner 2026-11-14. Approved: floor look, F9, resize view, OF-2 shadow, OF-6
wireframe (for now), OF-9 Merge color, Smooth / Smooth Brush / Blob,
Intersect, Subtract. Follow-ups became OF-28 to OF-58. OF-4 hover: owner did
not know what it meant; explain it in the next pack.

## Fix-first list (plan section 4)

| # | Item | State |
|---|---|---|
| F1 | App tests compile | **Done** 2026-11-12 |
| F2 | Whole suite green | **Done** 2026-11-12 (`f1b581d`; re-verified after OF-1) |
| F3 | Clippy on all targets | **Done** 2026-11-12 |
| F4 | Baseline commit of 220+ uncommitted paths | **Done** 2026-11-12 on `rebuild/work` |
| F5 | Release link reliability | **Done** 2026-11-16 — two consecutive forced fat-LTO release links, both clean (3m23s, 3m18s; 64 GB host), exe 21.9 MB. Plan exit met; `lto = "fat"` kept. Earlier: three clean links 2026-11-12; one LLVM out-of-memory 2026-11-11 |
| F6 | Reconcile the 148-row feature ledger with the app | **Done** 2026-11-16 — 69 stale rows (mostly the October RB-00 audit: "no Ctrl+N", "no pan gesture", "text menu button" and so on) checked against the code and named tests and rewritten; owner approvals from this file copied in with the owner's words (`owner_quote`). Code and tests only, nothing re-driven live. Counts in the Ledger section |
| F7 | Clear ad hoc `target/` directories (`E:` had 40 GB free) | **Done** 2026-11-12 — archived to `G:\Builder3D-Rust_target-adhoc-archive_20261112`, `target/` 91 → ~55 GB |
| F8 | `scripts/Invoke-Gate.ps1` | **Done** 2026-11-13 — `-Tier fast\|full\|ship`; ship does not script package staging or the performance run |
| F9 | Depth fight where meshes meet the floor | **Owner-approved 2026-11-14** ("works exactly as I asked"); done 2026-11-13 — depth bias on floor surface, shadows and text; captures `evidence/rebuild/captures/2026-11-13-f9-floor-depth/` (before/after, both themes). Dark serration gone; a faint pale dotted fringe on the rim remains and does not change with the bias or the reflection (cause not found). CPU fallback not touched |
| F10 | `emboss_text` long-running metadata | Decided (D5): keep the flag; progress indicator is OF-57, deferred by the owner 2026-11-14 |
| F11 | Status narrative out of `AGENTS.md` | **Done** 2026-11-12 |
| F12 | MCP 100% desktop parity (plan 3.7) | **Done** 2026-11-13 — Merge color; `rename_object` and `set_visibility` tools (catalog now 62, each with a desktop-vs-MCP test); `launch_slicer` falls back to the saved slicer; ledger `mcp_tool` / `mcp_exempt` column and parity check. Owner 2026-11-14 (D10): no "visual only" exemptions — MCP gets tools for all of them (OF-58) |
| F13 | Run `cargo deny` and `cargo audit` | **Done** 2026-11-12 — both pass; now in the Full gate |

## Ledger (`evidence/rebuild/parity-matrix.toml`, 148 features, reconciled 2026-11-16)

Implementation: pass 142 · partial 5 · not applicable 1. Automated: pass 139 ·
partial 4 · pending 5. Used live: pass 124 · pending 20. Owner-accepted 29
(visual pass on the same 29); 119 rows wait for the owner's
walkthroughs (S2 owner review session 2). Still partial: the five RB-01
look rows (theme foundation, welcome, Items panel, dark palette, artwork
conflicts) — they belong to the S1 finish, not to code gaps. Gaps found:
no test dispatches Center View, Frame or the Snap toggle directly, and none
covers the window-icon load.


## Owner feedback queue

- 2026-11-02: interface and floor "still missing" — floor addressed
  2026-11-09/10; interface finish is stage S1.
- 2026-11-11: move / resize / rotate arrows "feel cheap" — handled as one
  reviewed set in S1 task 7, not iteratively.
- 2026-11-12: the app "looks kind of home made" — first finish pass landed
  (captures in `evidence/rebuild/captures/2026-11-12-finish-pass/`); remaining
  work is S1 tasks 1–9. **Not yet reviewed by the owner.**
- 2026-11-12 (second batch): nine items OF-1 to OF-9 in
  [`owner-feedback-2026-11-12.md`](owner-feedback-2026-11-12.md), worked in
  the owner's order (OF-1 floor, OF-1 resize + OF-3, OF-4, OF-6, OF-9, OF-2,
  OF-7/8; OF-5 waits for shots), one commit each, **none owner-reviewed
  yet**. All committed: OF-1 floor numbers `6fbb5b7`; OF-1 resize view and
  OF-3 `53a45f1`; OF-4 hover `8797405`; OF-6 wireframe `fc70679`; OF-9 Merge
  color `38d6437`; OF-2 shadow and reflection `f7e19aa`; OF-7/OF-8
  `89c1e6f`. Cause of OF-7/OF-8: the Paint tab and the sculpt tools were
  disabled whenever nothing was selected, and every slider was invisible.
  Details, known limits and lessons are in the batch summary at the end of
  the feedback file. Open: OF-5 (waits for shots), face/region painting, a
  comparison with Blender's brushes, the shadow and reflection on the CPU
  fallback, and Merge of more than two objects. X-ray and Paint reference shots received 2026-11-14 (OF-47, OF-44); Merge
  needs none now (owner approved it).
- 2026-11-12 (shadow follow-up, committed `43f0fd0`): floor
  shadow now follows the object's convex outline instead of its bounding
  box; shadows and floor text hide with the floor when the camera is below
  it; a dragged object's shadow hides during the gesture and returns once
  it lands (owner: "way better"; ideal would be a shadow that follows the
  object). Details and open points at the end of
  [`owner-feedback-2026-11-12.md`](owner-feedback-2026-11-12.md). Full
  workspace gate passed before commit.
- 2026-11-12: owner wants MCP at 100% (F12, D6, D7); `cargo deny` and
  `cargo audit` now run (F13). Not stressed about module size, lint
  enforcement or more comments; those are not work items.
- 2026-11-12 (snap re-capture, approved deviation from Python): owner
  reported that after missing a snap point while dragging, nothing snapped
  again for the rest of the drag. Cause: the released snap point stayed
  excluded until the pointer crossed back past it (Python behaviour). Now the
  exclusion lifts as soon as the pointer leaves the point's capture band, so
  coming back near it snaps again from either side. Desktop drag only; the
  MCP `snap_translate` tool is one-shot and already snaps whenever near.
  Test `missed_snap_recaptures_when_the_pointer_comes_back_near_it`.
  Commit `2c7b319`. **Owner-approved** in the release build 2026-11-12
  ("fantastic").
- 2026-11-12 (third batch): OF-10 to OF-14 **owner-approved** in the
  release build ("you did amazing. i say yes to it all") — icon-only
  shortcuts, Slicer button with remembered slicer (no dialog), labelled
  Save / Save As, light/dark toggle (`0f3dd09`); Shift+drag box select,
  additive, touching counts (`907f9b0`). Full workspace gate passed.
  Captures `evidence/rebuild/captures/2026-11-12-of10-14-topbar/`. Table in
  [`owner-feedback-2026-11-12.md`](owner-feedback-2026-11-12.md).
- 2026-11-12 (fourth batch): OF-15 to OF-19 **owner-approved** in the
  release build 2026-11-12 ("You did great. i approve of it all") — darker light-mode top bar, Settings follows the top-bar theme
  toggle, a press outside Settings closes it, one Close button (Done
  removed), menu icon centred with the tab text. Full workspace gate passed.
  Captures `evidence/rebuild/captures/2026-11-12-of15-19-settings-topbar/`.
  Table in [`owner-feedback-2026-11-12.md`](owner-feedback-2026-11-12.md).
- 2026-11-13 (sixth batch): OF-26 menu rows highlight under the pointer
  (also the Slicer chooser); OF-27 the active tab is joined to the open
  ribbon like a folder tab. **Owner-approved** in the release build
  2026-11-13 ("dang, that looks good. well done!"). Fast gate on
  `modeler3d-app` passed. Captures
  `evidence/rebuild/captures/2026-11-13-of26-27-menu-hover-tab/`. Table in
  [`owner-feedback-2026-11-12.md`](owner-feedback-2026-11-12.md).
- 2026-11-13 (test release RC1; how-to in [`packaging.md`](packaging.md)):
  `packaging/msi/Build-Msi.ps1 -Version 0.1.1 -Name Builder3D-RC1 -Build
  -Verify` builds a per-user WiX 5 MSI (no admin; installs to
  `%LOCALAPPDATA%\Programs\Modeler3D`; Start-menu + desktop shortcuts; VC++
  runtime app-local; bump the version's third field per candidate).
  Output `E:\3D-Modeler-RC\Builder3D-RC1.msi` (50 MB). Silent install,
  launch, smoke and clean uninstall verified on the dev PC; not yet tried on
  a clean PC. Fixed `--package` artwork staging (arguments swapped; never
  worked) with a regression test. The script stages directly because the
  package is 64 MB against the 40 MB S3 budget: the action icons are
  1254 px PNGs (~32 MB). Shrinking them is an S3 owner decision.
- Owner merge report 2026-11-13: Merge of a sphere and a cylinder that only
  touched gave one object with 2 disconnected shells (no warning); smoothing
  then melted the sparse stock cylinder (two rings only). Decided 2026-11-13:
  D8 and D9 (plan section 8). Not built yet.
- **Paused by the owner 2026-11-12** after batch 3. On resume, suggested
  next: MCP `launch_slicer` falls back to the saved slicer when no
  `slicer_key` is given (owner said yes; F12 parity), then F12 rename and
  show/hide tools, then F9.
- Review harness: `MODELER3D_REVIEW_CAMERA=yaw,pitch,distance` poses the
  camera after auto-framing; state `resize-drag` holds the +X handle mid-drag;
  `MODELER3D_REVIEW_POINTER=x,y` rests the pointer to show hover states;
  `MODELER3D_REVIEW_VIEW=wireframe,xray` turns view toggles on; state
  `merge` merges a red and a blue cube; `unselected`, `material` states;
  `MODELER3D_REVIEW_INTERACTIVE=1` stays open with isolated settings for
  hands-on checks. States `texture`, `texture-dialog` take pictures from
  `MODELER3D_REVIEW_TEXTURES` (`;`-separated paths).

- 2026-11-15 (found during the OF-58 live run): the settings-migration
  notice drawn over the ribbon labels — now OF-64.

## Last recovery point

Git: branch `rebuild/work`, see `git log` (F5/F6: `f0a6cb0`; OF-65: `02fc0ce`, centred tabs `e6e03bf`, 0.000 angles `02dba59`; Phase 6: `1ac2583` OF-63/OF-64; Phase 5: `2f6aaf8` OF-58; Phase 4: `cfc96a7` OF-55,
pack `ff8b28f`, one-press Esc `229f0d2`; Phase 3 follow-ups: `57671fb`
OF-59, `fbbfff8` OF-60, `0238ccf` OF-61; Phase 3: `e95dbd3` OF-48,
`e9aa73d` OF-49, `7248d91` OF-50, `7432d0b` OF-51, `54b6f2a` OF-52,
`e283f15` OF-53 findings, `114a676` OF-54; Phase 2: `654872a` (OF-46), `344e88e` (OF-44),
`82f4a58` (OF-47), pack `06699d2`; Phase 1: `566ba48` to `3288a55`). Tree
snapshot: `G:\3D-Modeler_snapshot_20261116-145640_F5-F6` (10,026
files, 1,338,525,537 bytes, counts verified; excludes `target\` and `logs\`).
Previous: `G:\3D-Modeler_snapshot_20261116-133446_OF65-followups`.

## Hand cursor polish � 2026-11-20
Original curved hand silhouettes in grab_cursor.rs; task card and STATUS updated. 539 workspace tests, fmt, workspace Clippy, deny, audit, parity, smoke passed; 106 full-board captures generated. Initial gate test linker lock resolved by a full workspace rerun. Both hand poses reviewed in both themes; owner approval pending. Release current. Baseline G:\3D-Modeler_snapshot_20261120-070750_hand-polish-baseline verified 15,658 files / 2,331,370,571 bytes. Recovery G:\3D-Modeler_snapshot_20261120-071700_hand-polish verified robocopy success and matching counts: 15873 files / 2369865347 bytes. Snapshot precedes this receipt and final commit. Next: owner review; no further implementation scheduled.

## 2026-11-20 Transform boxes fill on insert/import

Snapshot `G:\3D-Modeler_snapshot_20261120-075551_transform-boxes-fill`, taken 2026-11-20 07:55 after commit `4d15ced`. Robocopy result 1 (copied); source and copy both 16,821 files / 2,430,060,677 bytes (target\ excluded).

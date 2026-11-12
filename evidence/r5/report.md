# R5 final — viewport

R5 is complete in the authoritative order from section R of
`L:\Builder3D\TODO.md`:

1. wgpu renderer
2. camera
3. gizmo
4. picking
5. floor
6. contact cues
7. adaptive interaction tiers
8. central resource ledger
9. CPU/GPU/hybrid workload planner

## Delivered

- `builder3d-viewport` now has a real indexed wgpu renderer with color/depth
  attachments, WGSL mesh lighting, floor/gizmo/contact overlays, bounded
  staging/readback, DX12/Vulkan adapter matching, and a deterministic CPU
  rasterizer fallback.
- Camera and matrix code is renderer-neutral and validates finite input,
  singular matrices, clip ranges, screen rays, framing, and view-scaled
  tolerances.
- Gizmo geometry and hit testing are renderer-neutral for move, rotate, and
  resize modes. Axis colors match the Python reference (#FF3333, #33CC33,
  #3333FF) and the handle target is 96 pixels.
- Picking streams the retained full geometry with AABB culling and exact
  triangle intersections. It returns world-space hit details and never uses a
  display proxy.
- The floor is camera-relative, bounded, adaptively spaced, and hides below
  the build plane. Contact footprints are revision-keyed derived data: they
  are skipped during camera motion and restored from cache at rest.
- Full geometry remains authoritative. The active interaction target follows
  the reference 30,000,000 / 1,000,000 / 4,000,000 / 12% thresholds. The
  30,655,824-face probe target is 3,678,698 faces.
- The platform ledger accounts authoritative/target/cache/staging allocations
  and exposes live pressure. The shared planner supports CPU, GPU, and
  batch-level hybrid routes, with explicit integrated/unified and software
  fallbacks.

## Measured R5 run

Evidence was produced by the optimized `r5_viewport` binary on 2026-10-29.
The real renderer and the controls used the NVIDIA Quadro RTX 4000, DX12,
driver `32.0.15.8142`; the machine has an 8 GiB Quadro budget.

| Workload | Result |
| --- | ---: |
| Retained scene | 81,932 faces; 1,966,464 derived bytes |
| Resting viewport | 7.106 ms; 81,932 triangles; 564 overlay vertices |
| Active viewport | 0.543 ms; 81,932 triangles; 546 overlay vertices |
| 30,000,000-face control | 30.599 ms p50 / 32.062 ms p95 and max |
| 3,600,000-face interaction control | 3.492 ms p50 / 3.505 ms p95 and max |
| CPU reference loop | 19.314 ms |
| Release executable | 4,953,088 bytes; 4.0 MiB `.text` |

Captures: `viewport-resting.ppm`, `30m-control.ppm`, and
`interaction-control.ppm`. The full JSON and markdown measurement are in
`evidence/r5/report.json` and this file's sibling run output.

## Verification

- `cargo fmt --all -- --check` passed.
- `cargo check --workspace` passed.
- `cargo test --workspace` passed, including 18 viewport tests and the full
  existing workspace suite.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  passed.
- `cargo deny check` passed. It retains the pre-existing unmatched BSD-3-Clause
  allowance and duplicate transitive `hashbrown`/`syn` warnings.
- `cargo audit` could not run because this Cargo installation has no audit
  subcommand.
- `cargo bloat --release --package builder3d-app --bin r5_viewport -n 20`
  passed: 4.0 MiB `.text`, 4.7 MiB executable.
- Python reference checks through `L:\Builder3D\venv` ran with
  `PYTHONDONTWRITEBYTECODE=1` and `-p no:cacheprovider`: 11 gizmo/math tests,
  2 adaptive scene tests, and 7 camera/floor/contact tests passed.
- The production forbidden-marker scan found no production `unwrap(`,
  `expect(`, `panic!`, `todo!`, `unimplemented!`, or `dbg!` paths; existing
  matches are in test modules only.

## Limitations

- R5 validates the renderer offscreen; a native window/surface shell and text
  floor labels are later shell/UI work.
- The 30M and 3.6M timing controls use procedural instancing. The real
  viewport frame uses retained indexed geometry and measures its actual
  buffers separately.
- Exact picking is intentionally CPU-authoritative and bounded; it is not a
  GPU proxy pick.

## Final backup

The additive snapshots for each ordered R5 checkpoint and the final backup are
on `G:\Builder3D-Rust_snapshot_20261029-*`. The final snapshot is:

`G:\Builder3D-Rust_snapshot_20261029-191500_R5-final`

Every snapshot excludes `target\`. Exact source/snapshot file counts, byte
totals, relative-path comparison, and per-file SHA-256 results are in
`verification.md`.

No commit was created. Existing `L:\Builder3D` changes and prior `G:\`
snapshots were preserved.

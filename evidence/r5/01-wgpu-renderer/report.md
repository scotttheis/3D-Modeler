# R5.1 wgpu renderer evidence

Status: complete.

`crates/builder3d-viewport/src/renderer.rs` implements an indexed wgpu mesh
pipeline, depth attachment, real scene uniforms, floor/gizmo overlay passes,
staging/readback capture, adapter/backend matching, and a bounded CPU fallback.
The R5 executable exercised the DX12 Quadro RTX 4000 path and wrote
`viewport-resting.ppm`, `30m-control.ppm`, and `interaction-control.ppm`.

Focused Rust renderer tests passed; the release run is summarized in the
parent `evidence/r5/report.md`.

# Builder3D Rust R5 viewport

- Scene faces: 81932
- Scene derived bytes: authoritative 1966464 / interaction 1966464
- 30,655,824-face adaptive probe target: 3678698 faces
- Real viewport resting: 47.672 ms (CPU fallback: false)
- Real viewport active: 1.416 ms (CPU fallback: false)
- 30M control p50/p95/max: 32.753/33.919/33.919 ms
- Interaction control p50/p95/max: 2.453/2.459/2.459 ms
- CPU reference: 289.726 ms
- Binary size: 17249280 bytes

## Routing matrix

- **dedicated-discrete**: render `Gpu { adapter_index: 0 }` / fallback `None`; memory-bound `Cpu` / fallback `Some(CalibrationNotBeneficial)`.
- **intel-integrated**: render `Gpu { adapter_index: 0 }` / fallback `None`; memory-bound `Cpu` / fallback `Some(CalibrationNotBeneficial)`.
- **amd-unified**: render `Gpu { adapter_index: 0 }` / fallback `None`; memory-bound `Cpu` / fallback `Some(CalibrationNotBeneficial)`.
- **software-fallback**: render `Cpu` / fallback `Some(NoUsableAdapter)`; memory-bound `Cpu` / fallback `Some(NoUsableAdapter)`.

## Limitations

- The 30M and 3.6M controls use procedural instancing, while the viewport frame uses retained indexed scene geometry.
- R5 does not yet provide text labels or a native window surface; offscreen captures validate the renderer and overlay paths.
- Exact picking remains CPU-authoritative and is intentionally separate from display LOD.

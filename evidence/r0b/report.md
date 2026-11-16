# 3D Modeler Rust R0b spike

This is measured spike evidence, not a production-readiness claim.

- Adapter: NVIDIA Quadro RTX 4000 / Dx12 / Discrete / budget=7578058752 / usage=7950336 / available=7570108416
- CPU reference: 17.993 ms
- 30M control first frame: 34.097 ms
- Steady p50/p95/max: 31.240/32.543/32.543 ms
- Binary size: 4698624 bytes
- Capture: `evidence/r0b\30m-control.ppm`
- UI baseline: Use egui/eframe as the R0/R6 native shell baseline; keep the G1 ribbon visual-acceptance gate open.

## Routing evidence

- **actual calibrated route**: route `Cpu`, batches 300 (0 GPU, 300 CPU), fallback `Some(CalibrationNotBeneficial)`, transfer 0 bytes, adapter available 7569059840 bytes.
- **calibrated GPU-win planner route**: route `Gpu { adapter_index: 2 }`, batches 300 (300 GPU, 0 CPU), fallback `None`, transfer 960000000 bytes, adapter available 7569059840 bytes.
- **explicit CPU reference route**: route `Cpu`, batches 300 (0 GPU, 300 CPU), fallback `Some(CpuOverride)`, transfer 0 bytes, adapter available 0 bytes.
- **one-megabyte budget-pressure route**: route `Hybrid { adapter_index: 2 }`, batches 301 (1 GPU, 300 CPU), fallback `Some(BudgetPressure)`, transfer 943712 bytes, adapter available 1048576 bytes.
- **no-accelerator CPU fallback route**: route `Cpu`, batches 300 (0 GPU, 300 CPU), fallback `Some(NoUsableAdapter)`, transfer 0 bytes, adapter available 0 bytes.

The renderer control uses procedural instancing to submit exactly 30,000,000 triangle instances without replacing authoritative mesh data with a proxy. The planner separately accounts estimated authoritative geometry, transfer residency, live budgets, and batch-level fallback.

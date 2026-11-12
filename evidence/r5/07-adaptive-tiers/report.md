# R5.7 adaptive interaction-tier evidence

Status: complete.

`scene.rs` retains full geometry and creates deterministic interaction geometry
only for display; `adaptive.rs` selects resting full, active LOD, hybrid, or
CPU fallback tiers. The thresholds match the reference: 30,000,000 full-res
limit, 1,000,000 minimum, 4,000,000 maximum, and 12 percent proportional
target. The 30,655,824-face probe targets 3,678,698 faces.

Rust adaptive/scene tests, Python adaptive-LOD tests, and the release control
measurements passed.

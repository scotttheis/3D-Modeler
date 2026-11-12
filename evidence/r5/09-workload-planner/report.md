# R5.9 CPU/GPU/hybrid planner evidence

Status: complete.

The existing platform planner now has explicit uncalibrated integrated-memory
policy: CPU-parallel and memory-bound work stays on CPU, GPU render/compute
remain device candidates, measured GPU wins override the default, and live
budget pressure splits work at bounded batch boundaries. R5 pins the shared
planner to the adapter actually opened by the renderer.

The release report includes dedicated, Intel integrated, AMD unified, and
software fallback matrix entries. Platform routing tests cover GPU, CPU,
hybrid, transfer, and pressure decisions.

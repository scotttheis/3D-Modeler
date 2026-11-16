# R7-12 — platform runtime coverage

Component: `modeler3d.platform.runtime`

## Implemented contract

- Added injected-environment runtime path selection for Windows, macOS, and
  XDG-based Linux locations, including the MCP descriptor override and owned
  temporary directory.
- Added a current-host wrapper that performs no directory creation, keeping
  path selection separate from startup side effects.
- Added a read-only process-liveness query through the existing `sysinfo`
  dependency, with safe handling for zero, missing, and inaccessible IDs.
- Added the Python OpenGL/offscreen environment guard as a typed platform
  diagnostic seam. The Rust shell does not load Qt, so the guard is not wired
  into eframe startup.
- Added focused injected-environment, descriptor, process, and guard tests.

No Python source, R1–R6 evidence, or existing snapshot was modified.

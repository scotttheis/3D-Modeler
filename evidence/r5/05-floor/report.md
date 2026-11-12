# R5.5 floor evidence

Status: complete.

`floor.rs` builds a camera-relative y=0 floor, adaptive minor/major grid
spacing targeting 80 pixels, bounded line counts, and a fixed +Z front
orientation cue. The floor hides when the camera crosses below the build
plane.

Rust floor tests and the Python floor visibility/grid selectors passed.

# R5.2 camera evidence

Status: complete.

`camera.rs` and `math.rs` provide the Python-compatible orbit, pan, zoom,
reset, framing, perspective/view matrices, inverse screen rays, and
view-scaled tolerances. Camera values are validated and singular/non-finite
inputs return typed `ViewportError` values.

Camera and matrix Rust tests passed. The Python camera reference selectors
also passed without creating bytecode or pytest cache files.

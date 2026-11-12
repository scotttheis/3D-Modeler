# RB-05 paired comparison board

Python is the reference column. Rust is the rewrite column. These are observed paired states, not visual acceptance.

| Contract | Python reference | Rust rewrite |
|---|---|---|
| Empty/rest | [python-light-empty-rest.png](captures/python-light-empty-rest.png) / [python-dark-empty-rest.png](captures/python-dark-empty-rest.png) | [rust-light-empty-rest.png](captures/rust-light-empty-rest.png) / [rust-dark-empty-rest.png](captures/rust-dark-empty-rest.png) |
| Single/rest | [python-light-single-rest.png](captures/python-light-single-rest.png) | [rust-light-single-rest.png](captures/rust-light-single-rest.png) |
| Exact double-click frame | [python-light-double-click-frame.png](captures/python-light-double-click-frame.png) | [rust-light-double-click-frame.png](captures/rust-light-double-click-frame.png) |
| Orbit | [python-dark-orbit-after.png](captures/python-dark-orbit-after.png) | [rust-dark-orbit-after.png](captures/rust-dark-orbit-after.png) |
| Middle/right pan | [python-light-pan-middle-after.png](captures/python-light-pan-middle-after.png) / [python-light-pan-right-after.png](captures/python-light-pan-right-after.png) | [rust-light-pan-middle-after.png](captures/rust-light-pan-middle-after.png) / [rust-light-pan-right-after.png](captures/rust-light-pan-right-after.png) |
| Wheel zoom | [python-dark-wheel-after.png](captures/python-dark-wheel-after.png) | [rust-dark-wheel-after.png](captures/rust-dark-wheel-after.png) |
| Move preview/release | [python-light-move-rest.png](captures/python-light-move-rest.png) / [python-light-move-interaction.png](captures/python-light-move-interaction.png) | [rust-light-move-single-before.png](captures/rust-light-move-single-before.png) / [rust-light-move-single-after.png](captures/rust-light-move-single-after.png) |
| Multiple/grouped/hidden | [python-dark-multiple-rest.png](captures/python-dark-multiple-rest.png) / [python-dark-grouped-rest.png](captures/python-dark-grouped-rest.png) / [python-dark-hidden-rest.png](captures/python-dark-hidden-rest.png) | [rust-dark-move-group-after.png](captures/rust-dark-move-group-after.png) / [rust-dark-move-hidden-after.png](captures/rust-dark-move-hidden-after.png) |
| Resize presentation | Python source/test coverage; no separate accepted Python resize frame in this replay | [rust-light-resize-edge-after.png](captures/rust-light-resize-edge-after.png) / [rust-light-resize-corner-after.png](captures/rust-light-resize-corner-after.png) |
| Rotate presentation | Python source/test coverage; no separate accepted Python rotate frame in this replay | [rust-dark-rotate-single-after.png](captures/rust-dark-rotate-single-after.png) |
| Grid/floor | [python-light-grid-off.png](captures/python-light-grid-off.png) and restored state in `wireframe-xray` | [rust-light-wire-grid-rest.png](captures/rust-light-wire-grid-rest.png) |
| Wireframe/X-Ray | [python-light-wireframe-xray.png](captures/python-light-wireframe-xray.png) | Renderer/automated route plus [rust-dark-wire-grid-rest.png](captures/rust-dark-wire-grid-rest.png); the supplemental direct Grid-toggle journal is rejected, not a parity claim |

Theme, dimensions, device scale, action route, executable identity, and per-frame hashes are recorded in [capture-metadata.json](capture-metadata.json) and the two native journals.

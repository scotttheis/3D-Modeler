# RB-06 paired comparison board

Python is the reference column and Rust is the rewrite column. These are
observed paired states, not visual acceptance.

| Contract | Python reference | Rust rewrite |
|---|---|---|
| Empty/rest, Light | [python-light-empty-rest.png](captures/python-light-empty-rest.png) | [rust-light-empty-rest.png](captures/rust-light-empty-rest.png) |
| Empty/rest, Dark | [python-dark-empty-rest.png](captures/python-dark-empty-rest.png) | [rust-dark-empty-rest.png](captures/rust-dark-empty-rest.png) |
| Primitive Cube, Light | [python-light-primitive-cube.png](captures/python-light-primitive-cube.png) | [rust-light-torus-applied.png](captures/rust-light-torus-applied.png) plus primitive/core tests |
| Primitive Cube, Dark | [python-dark-primitive-cube.png](captures/python-dark-primitive-cube.png) | [rust-dark-torus-applied.png](captures/rust-dark-torus-applied.png) plus primitive/core tests |
| Modelling fixture, Light | [python-light-modelling-fixture.png](captures/python-light-modelling-fixture.png) | [rust-light-simplify-result.png](captures/rust-light-simplify-result.png) |
| Modelling fixture, Dark | [python-dark-modelling-fixture.png](captures/python-dark-modelling-fixture.png) | [rust-dark-smooth-result.png](captures/rust-dark-smooth-result.png) |
| Torus parameter preview | Python source/tests; no accepted modal reference capture | [rust-light-torus-dialog.png](captures/rust-light-torus-dialog.png) / [rust-light-torus-hole-preview.png](captures/rust-light-torus-hole-preview.png) |
| Persistent brush stroke | Python automated mesh-operation contract | [rust-light-grab-stroke-after.png](captures/rust-light-grab-stroke-after.png) and the corresponding Crease, Smooth Brush, Blob, and Normalize frames |
| Boolean Union and picker cancel | Python automated Boolean contract | [rust-light-merge-picker.png](captures/rust-light-merge-picker.png) / [rust-light-merge-complete.png](captures/rust-light-merge-complete.png) |
| Boolean Intersect/Subtract | Python automated Boolean contract | [rust-dark-intersect-complete.png](captures/rust-dark-intersect-complete.png) / [rust-dark-subtract-complete.png](captures/rust-dark-subtract-complete.png) |
| Repair/Paint/View/Items buttons | Python automated desktop/mesh contracts | [rust-light-repair-dialog.png](captures/rust-light-repair-dialog.png), [rust-light-material-dialog.png](captures/rust-light-material-dialog.png), [rust-light-view-toggles.png](captures/rust-light-view-toggles.png), [rust-light-items-actions.png](captures/rust-light-items-actions.png) |

The six Python baseline records are intentionally smaller than the Rust
operation replay: the reference modal runner did not provide a reliable
continuation route for the full typed operation sequence. Python's 272-test
reference suite is the operation contract; Rust's native run is the repaired
button/input evidence.

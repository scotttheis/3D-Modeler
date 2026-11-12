# Settle parity repair — RB-04 engineering checkpoint

The retained single/group matrix oracle is now enabled and passes unchanged.
The original failure occurred in the **single-object** case before the group
case ran; the earlier attribution to group triangulation was incomplete.
Python `lay_flat` obtains a world-space support normal, then calls its object's
post-multiplied `rotate`. Its multi-object service instead pre-multiplies one
world placement. Rust now preserves that distinction and scene-order input.

The kernel uses the reference's QHull `QbB Pp Qt` options and double-precision
world points. Both support-normal fixtures and both exact final-transform
fixtures match. Input memory/count/nonfinite bounds return typed errors before
mutation. The native wrapper is contained at the core boundary; the point-ID
API handles QbB's scaled coordinate buffer. No temporary-file capture is used.

The Windows probe compiled and ran the native library using the installed
MSVC toolchain and a verified, project-local build-only libclang DLL. The
released wrapper printed diagnostic pointers and asserted on its `index`
helper with scaled input, so the workspace pins reviewed upstream revision
`62e810ccf93b458470f03cae6ef4df26023d2260` (0.4.1), which removes that print.
Probe results are retained in the logs. See [building](../../../docs/building.md).

Upstream references: [qhull-rs](https://github.com/LucaCiucci/qhull-rs),
[QHull](https://www.qhull.org/), and [Qhull license](https://spdx.org/licenses/Qhull.html).
The native notice and MIT wrapper license were reviewed and copied unchanged
under `packaging/licenses/`; package staging now includes all three notice
files. Cargo's declared wrapper MIT license is checked by `cargo deny` and the
bundled native notice is tracked separately. No system app or Python package
was installed; libclang lives under ignored `target/` and is not a product
runtime dependency.

Focused and workspace results are in `settle-gate-results.json` and the
`logs/settle-*.txt` files. `logs/qhull-exact-parity-test.txt` demonstrates that
the previously ignored oracle now passes. The scene-order regression also
covers Measure ordering independently of Align/Arrange's user anchor order.

The initial locked-desktop attempt accepted no image or input. After the desktop became available, all single/two-selected-object before/after/undo/redo states were captured sequentially in both themes. Native saved matrices match Python within 9.32e-9, with stable IDs and one action per Settle. See native-settle-verification.json and the matched settle-1/2 board entries. Genuine capture build identities remain in the journal; later clipboard/modal changes did not alter Settle mathematics. Broader grouped-root transforms also have native viewport and independent oracle coverage. Visual/owner acceptance remains pending; RB-05 has not started.

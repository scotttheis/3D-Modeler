# RB-04 scene and precise transforms

RB-04 implementation, focused and integrated verification, and the required sequential Light/Dark native walkthrough are complete. Visual review and owner acceptance remain pending. RB-05 has not started; Python remains the shipping fallback. No staging or commit occurred.

## Result

Items now provide real shaded mesh thumbnails, durable selection anchors, sticky selection, health-to-Repair reachability, undoable naming and the existing visibility control. Duplicate and clipboard actions preserve metadata, create independent durable IDs, use the reference offsets, and commit once. Grouped roots render and pick their combined mesh once while retaining children for ungrouping.

Numeric Move/Rotate/Resize use the semantic command path, preserve exact untouched axes, convert mm/in only for display, and support degrees, Grid/Object resize bases and the edited-axis proportional ratio. Viewport previews share immutable geometry across CPU/GPU render routes, cancel safely, and commit all selected targets once on release. Resize presents the reference's six faces and eight corners; internal edge mathematics remains separately tested.

Settle uses bounded QHull support-face placement; Mirror follows the reference local-column/world-translation rule; Measure lists selected per-object Width X / Depth Z / Height Y in mm; Align and Arrange retain reference axis, landmark and anchor-order semantics. Hidden selected objects participate. Invalid values, canceled dialogs, no-motion drags, text focus and modal background input preserve the appropriate scene/history state.

Real native replay uncovered and fixed three issues: inch edits rounding untouched axes, eframe intercepting scene clipboard keys, and background pointer input remaining active under Settings. Dedicated regressions and native saved-state controls verify the fixes. See [clipboard](clipboard-native-fix.md), [modal input](modal-input-fix.md), and [verification](verification.md). A targeted reference replay waits for Qt to repaint chosen dialog values; earlier stale default-choice frames were superseded while the already-correct saved results were reverified.

## Native and reference evidence

Both themes include 37 scene cases, 11 viewport cases and two Settle cases: **100 case/theme combinations**, plus physical viewport-pick checks in both themes. Scene mutation cases include saved after/undo/redo states; selection/no-op cases preserve revision and geometry. The source visible Qt widgets/handlers are distinguished from physical input; both applications' viewport drags use guarded OS pointer/key events. Rust scene routes use guarded physical controls. Only one target UI was open at a time, at 1280 x 840 client pixels, 96 DPI/100%. Captures record actual origin, size, theme, fixture and build identity.

Rejected Dark dialog automation attempts lost selection or toggled the ribbon closed. The replay now checks painted tab/ribbon state and parks its idle cursor on blank navigation. Valid reruns pass saved-state/selection checks. A proposed queued-event product defect did not reproduce in real egui events; its experimental test was removed and the original source hash verified. No speculative product fix is claimed. See [investigation](queued-pointer-investigation.md).

[Comparison board](visual-comparison.html): 386 paired action states / 812 capture PNGs, Python left. Historical intermediate captures remain labelled and do not replace the matched native cases. Capture journals retain the genuine executable hash for each frame: later clipboard/modal changes did not alter the previously verified transform mathematics. Saved IDs, meshes, matrices, unaffected objects and history transitions are verified independently of screenshots. Failed/superseded attempts are retained with versioned outputs.

## Visible differences and contract exceptions

Python has tall actions, large wireframe tiles, stronger blue selection outlines, flatter selected surfaces, near-face arrows, white round Resize grips and a compact two-decimal numeric strip. Rust has wrapped actions, small shaded rows, softer green surfaces, a finer grid, centered arrows, square grips, a side inspector and larger three-decimal fields with Apply. Camera framing also differs. These observations remain available for review; they are not visual acceptance. Broader camera/gizmo/display work belongs to RB-05.

Python uses native axis/landmark/gap dialogs, often in two steps, and a native Dimensions message box. Rust combines placement choices in an in-app modal and dims/disables background controls. Python clamps negative arrangement gaps at zero; Rust rejects the invalid value and can cancel unchanged. Python numeric spin boxes validate while typing; Rust validates raw text on Apply. Both retain internal mm values.

The explicit naming/visibility scope retains Rust Name/Rename and Eye controls. Python has no observed interactive equivalents, so its comparison states were prepared through object data. Its hidden multi-selection proportional-lock widget was explicitly set by the reference harness. No source UI interaction is invented.

Observed source Shift-click selects endpoints [0,2]; Rust implements the handoff's contiguous range [0,1,2]. Source Escape retains a moved pose without a history entry and clears selection; source focus loss completes the drag. Rust cancels its preview and preserves the starting scene/history. These deliberate safety/requirement differences are recorded, not called exact source parity or owner-approved changes.

Repair reachability is complete: the open-triangle case becomes four faces in one undoable command with equal bounds and total surface area. **Tessellation differs:** Python's fan point is [5,5,0], Rust's is approximately [6.6666665,6.6666665,0]. Exact native mesh undo/redo is verified. Detailed Repair mesh-operation parity remains an RB-06 item; no exact tessellation claim is made. Python shows remaining issues, geometry counts and named stages in a large report; Rust shows a compact stage-count result in its Repair form. Both still flag the flat zero-volume fixture. Rust viewport picks update the highlighted row/count while the prior status message can remain; Python names the picked object. These visible differences remain recorded for review.

## Verification, limits and recovery

**261 workspace tests pass, zero failed/ignored.** Format, check, strict workspace/all-target/all-feature Clippy, deny, metadata, debug/release smoke, release and explicit app bloat pass. Read-only focused Python runs pass 52 gizmo, 25 scene/selection, 11 service, seven control and four adapter tests. Independent real-source oracles cover 122 gesture, 32 multi-numeric, 42 placement and nine Euler/polar cases. Cargo audit is unavailable, not passed.

The ordinary release is 14,972,928 bytes, below the 40 MB executable target; installed-package and representative performance acceptance remain later gates. Automatic health inspection is bounded to one million faces; Repair remains reachable for larger objects. Settle limits point/temporary-memory allocation and grouping retains the project depth limit. [Boundary verification](boundary-verification.md) records identities and limitations. The final verified additive G: snapshot receipt is in [checkpoint.md](checkpoint.md) and WORK_LOG.md.

All 32 RB-03 status rows are preserved: implementation/automated pass; native 30 pass/two partial; visual/owner pending. Remaining RB-03 work: saved-checkpoint undo/redo/branch native sequence, full filesystem-failure native matrix, extra Python per-format export-picker replay, paired visual review and owner acceptance. These did not block RB-04 and were not promoted.

## Stop boundary

Stop before **RB-05**: full viewport gestures/presets, gizmo placement/style, snapping, floor/grid, lighting/colors, wireframe/X-Ray and remaining resize interaction. Then RB-06 mesh/primitive/brush workflows (including Repair tessellation review), RB-07 appearance/jobs/print/slicer, RB-08 preferences/shell/accessibility, RB-09 live desktop/MCP consistency, RB-10 resources/performance/package/host coverage, and RB-11 final parity/live/visual/owner acceptance. RB-04 does not establish product acceptance.

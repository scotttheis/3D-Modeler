# RB-05 viewport and input completion

RB-05 is implemented and verified through focused Rust tests, the read-only Python reference suite, and sequential native Windows replays. The slice stops before RB-06. Implementation, automated verification, and native interaction are **pass**. Visual review and owner acceptance remain **pending**; no acceptance is inferred from the captures.

## Implemented contract

- Camera navigation now wires Python-matched logical-pixel orbit, middle/right camera-plane pan, wheel zoom, and trackpad-pinch zoom. Camera-only actions leave scene geometry, durable IDs, and history unchanged. Reset, Center, Frame, Top, and Side use the view-state controller. Framing ignores hidden objects and safely handles empty or hidden-only selections. A double-click performs an exact frontmost pick, selects the durable object, frames its world bounds, and creates no scene/history mutation.
- Gizmos use immutable selected bounds for both paint and hit testing. Move arrows are displaced to the camera-nearest face; Rotate uses the same projected extent for its ring and hit target; Resize exposes six bidirectional faces and eight corners, with no edge-grip substitute. Preview, release, cancel, focus loss, no-op, invalid, stale, and modal paths preserve the one-commit/safe-cancel contract.
- Move snapping captures a drag-start scene snapshot, excludes moving and hidden targets, considers lower/upper/center landmarks and the floor, uses view-scaled tolerance and nearest-candidate selection, and records feedback. The persisted Snap preference defaults on, parses boolean/string values safely, and Shift bypasses snapping without introducing fixed increments.
- The renderer emits an actual bounded build-plane surface and camera-relative metric grid. Major spacing is 100 mm; minor spacing adapts by camera distance while preserving major lines. Light/Dark floor, grid, selection tint, authored mesh color, depth-tested opaque black wireframe, and independent translucent X-Ray options reach both GPU and CPU fallback routes. Geometry is full mesh data, not a bounding-box substitute, and the cache identity includes the underlying geometry allocation.

The main implementation seams are [shell.rs](../../../crates/modeler3d-app/src/shell.rs), [commands.rs](../../../crates/modeler3d-app/src/commands.rs), [transform_gesture.rs](../../../crates/modeler3d-app/src/transform_gesture.rs), [gizmo.rs](../../../crates/modeler3d-viewport/src/gizmo.rs), [floor.rs](../../../crates/modeler3d-viewport/src/floor.rs), [renderer.rs](../../../crates/modeler3d-viewport/src/renderer.rs), and [cpu.rs](../../../crates/modeler3d-viewport/src/cpu.rs).

## Native evidence

The read-only Python replay ran first at 1280 x 840 logical/physical client pixels, 96 DPI / 100% scale, with real Win32 pointer, wheel, and ribbon input. It produced 30 accepted records: 15 Light and 15 Dark. It covers empty, single, exact double-click frame, orbit, both pan buttons, wheel, Grid off/on, Wireframe/X-Ray, Snap-off feedback, Move rest/interaction, multiple, grouped, and hidden scenes.

The final Rust replay then ran one visible target at a time against debug executable SHA-256 `4a42ec94338baf986806740e7a59c7fc7db2faa9bdf2027af4d8f6160ac69232`. It produced 52 accepted records: 26 Light and 26 Dark. Every record is 1280 x 840 at 96 DPI. It covers the same camera actions, exact double-click frame, Move/Rotate/Resize edge/corner, no-op, Escape cancel, focus loss, grouped, hidden, and actual floor/metric-grid resting render. The Rust target was closed after replay.

The accepted journals are [python-native-results.json](python-native-results.json), [rust-native-results.json](rust-native-results.json), and the per-frame PNGs under [captures](captures). The Rust display-controls experiment is deliberately not counted as accepted Grid-toggle evidence: its Grid-off frame did not differ from the resting frame and its journals are preserved under `rust-display-rejected-*.json`. The accepted native record still shows the actual Rust floor/grid render, while command/render-state behavior is covered by automated tests. This keeps the evidence honest and leaves visual review pending.

## Status boundary

| Dimension | Status | Meaning |
|---|---|---|
| Implementation | pass | RB-05 code paths are present and integrated. |
| Automated | pass | Focused and full locked workspace gates pass. |
| Native interaction | pass | Sequential paired Python/Rust OS-input replays pass their capture guards. |
| Visual review | pending | Captures are inspection material, not approval. |
| Owner acceptance | pending | No owner acceptance was requested or inferred. |
| RB-03 / RB-04 | preserved pending | Earlier review and acceptance statuses remain unchanged. |
| Next slice | RB-06 | Not started; work stops at this boundary. |

See [verification.md](verification.md), [requirement-test-map.md](requirement-test-map.md), [walkthrough.md](walkthrough.md), [paired-comparison-board.md](paired-comparison-board.md), and [rejected-attempts.md](rejected-attempts.md).

The final verified additive snapshot is `G:\Builder3D-Rust_snapshot_20261101-064700_RB-05-complete`; its exact receipt is in [WORK_LOG.md](../../../WORK_LOG.md) and this slice's [checkpoint.md](checkpoint.md).

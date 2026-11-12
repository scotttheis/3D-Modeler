# RB-04 viewport transforms

The native shell now routes Move, Rotate and Resize to renderer-neutral gesture math. Motion changes a small transform snapshot; the authoritative scene and history remain unchanged until release. Release resolves durable root IDs, rejects a stale/replaced document, and commits all targets together. Escape, loss of focus, a modal or changed scene cancels the gesture. A click without motion does not change dimensions or history.

The reference renders each visible top-level object's combined mesh. Children survive for ungrouping, including their visibility. Rust previously rendered the combined group and its children, creating duplicate surfaces and picks with internal child IDs. It now draws/picks the combined mesh once. Preview matrices share immutable full and interaction mesh buffers and feed the same scene to GPU and CPU paths.

Resize exposes six face and eight corner handles. Twelve edge cases remain internal mathematics and are not visible native grips. Faces retain the opposite side; edges scale their axis around the center; corners scale uniformly around the center. Grid axes are used for multiple targets; a single target can use the proper polar object basis. Numeric dimensions remain world dimensions, as in Python. Rotation accumulates signed steps across the angle boundary; invalid parallel/central rays leave the preview unchanged. Gizmo bounds stream exact vertices and cache by revision/selection/basis/mode; they do not clone meshes on each frame.

`generate-gesture-oracle.py` invokes the real read-only Python `GizmoManager` without a window. `gesture-oracle.json` contains 122 start/update ray and before/after matrix cases: all axes, every resize handle, both single-object bases, a grouped pair and a hidden second selection. Rust compares the original outputs within 0.003 per matrix entry, verifies no scene/history mutation during preview, and verifies one undo restores the full starting state. Additional tests cover stale/replaced documents, selection changes, no-ops and invalid rays. The Python gizmo test file passes all 52 tests.

Native pointer evidence is now complete in both themes; see native-viewport-verification.json and walkthrough.md. Visual/owner acceptance remains pending. RB-05 owns broader camera/display/style/snapping work; movement oracles disable source landmark snapping to isolate transform mathematics.

Grouping now rejects nesting beyond the existing 32-level project limit before recursive cloning or mutation. Rejection preserves identity and history.

Same-revision project replacement/new scene clears Items and gizmo caches. A changed selected set cancels the UI drag; the semantic snapshot still validates captured durable identities independently. The final identity regression brings the workspace total to 258 passing tests.

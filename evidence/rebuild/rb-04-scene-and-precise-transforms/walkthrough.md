# RB-04 native walkthrough

The Python reference and Rust application ran sequentially, one target UI at a time, in Light and Dark at 1280 x 840 client pixels, 96 DPI/100%. Dedicated profiles, bytecode/cache suppression and E: output paths kept L:/Builder3D read-only. Every physical action checks the input desktop and exact process window. A locked or wrong foreground window rejects capture/input; no corporate policy or sleep setting was changed.

## Scene replay: 37 cases per theme

`native-scene-case-specs.json` defines shared fixtures and ordered selections. `python-scene-walkthrough.py` drives the actual visible Qt widgets/handlers and records model/history plus 238 validated frames across both themes. It distinguishes prepared naming/visibility data and the hidden multi-lock widget from visible input. `native-rust-scene.ps1` and `native-rust-modal-workflow.ps1` use OS pointer/key input. Strong selection preconditions inspect highlighted rows before dispatch. File-dialog guards check actual surfaces before typing; Save As uses unique outputs and waits for the expected title/file.

- Row toggle, Ctrl toggle, Shift range, All/None/Invert, sticky and ordinary/Ctrl empty clicks.
- Duplicate, Copy, Paste after changing selection, Cut, Delete, Group/Ungroup; IDs, offsets, child metadata, hidden state and one-command undo/redo.
- Single/multi Move, Rotate and Resize; mm/in/degrees, proportional lock, exact untouched axes, invalid input, text Delete and display-only units.
- Requested Name/Rename and Eye control; source states are prepared equivalents, not invented source UI interactions.
- Mirror Y and cancel; Align Z/max with reversed anchor order; Arrange Z/5 mm and invalid/cancel; per-object Measure including hidden selection.
- Unhealthy Items warning, real Repair dialog/run/report, four-face result and exact native mesh undo/redo.

`verify-native-scenes.py` verifies all 74 cases against real-source results and scene specifications. It checks original IDs, independent new IDs, exact unaffected meshes/metadata/poses, expected revisions and mutation undo/redo. Selection-only results add no geometry history. Source Shift selects [0,2]; Rust implements the explicitly requested [0,1,2]. Repair bounds/area match with the documented interior-point variance. Latest routes are in `rust-scene-output-routes.ndjson`; superseded attempts stay on disk.

## Viewport replay: 11 cases per theme

Real OS pointer/key gestures exercise Move single/hidden/group, Rotate single/group, Grid-face/Object-face/uniform-corner Resize, no-motion, Escape and focus loss. Preview does not mutate the authoritative scene; release commits once. Saved after/undo/redo states verify exact IDs/matrices and unaffected objects, opposite-face anchoring and uniform center-based corner scaling. The source and Rust camera/gizmo projections yield different deltas for equal screen travel; the check uses each route's input geometry. Source Escape/focus behavior differs explicitly; Rust discards the preview. `native-viewport-verification.json` verifies 22 cases.

## Physical viewport picking

A separate real OS click in each application selects the frontmost visible object, preserves it on Ctrl-empty and clears on ordinary empty. Source picks an interior point using its exact face ray query; Rust selection is read from highlighted Items rows and saved unchanged scene metadata. All four states per theme are paired; no model history is added. `python-pick-verification.json` and `native-pick-verification.json` record the result.

## Settle and modal controls

Single and two-independent-selected-box Settle run before/after/undo/redo in both themes, with 16 frames per app. Saved matrices match within 9.32e-9. `native-settle-verification.json` verifies four cases and one action per operation. The broader grouped-root transform paths are covered by viewport/numeric fixtures and independent oracles.

The final modal regression repeats Settings with background Duplicate and numeric Apply, proves the three-object input/revision is unchanged, then proves Duplicate works after closing Settings. Escape closes the dialog. The final clipboard regression verifies physical Ctrl+C/Ctrl+V with changed selection independently of OS text clipboard, plus real TextEdit focus behavior.

## Comparison and review

The HTML board places Python left and Rust right with action-specific observations. Full-size captures and contact sheets support review of Items, highlighted rows, naming/hidden results, numeric strips, dialogs, grouped/hidden transforms, Repair, and undo/redo. Actual build identity is kept per frame; later clipboard/modal changes leave prior transform math unchanged. The board labels all historical intermediate states.

Python's large wireframe tiles/flat faces/strong outlines/near-face arrows/round grips/compact fields differ visibly from Rust shaded rows/soft green surfaces/fine grid/center arrows/square grips/side inspector/large Apply strip. Native Qt dialogs differ from Rust in-app modals. Naming/Eye extensions, Shift endpoints/range, source Escape/focus and Repair fan-point differences remain explicit in report.md. Agent inspection is recorded separately from pending owner/visual acceptance. No RB-03 status is promoted and no RB-05 implementation is started.

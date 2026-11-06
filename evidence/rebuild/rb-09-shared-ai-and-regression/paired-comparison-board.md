# RB-09 Paired Comparison Board

## Shared project identity

| Property | Python initial live scene | Rust final-build paired scene |
|---|---|---|
| Project ID | e5d81c5226e64f58b838d55d9395afa9 | e5d81c5226e64f58b838d55d9395afa9 |
| Revision | 4 | 23 |
| Cube | ID 3f449b78621c467b818e8c69925e2b6c; 20x20x20 mm; position [-15, 0, 10] | Same ID and geometry |
| Sphere | ID dff9cc124c6d4d27b4ea9b07d6e4ae6b; 14x14x14 mm; position [15, 0, 7] | Same ID and geometry |
| Cube color | [0.25, 0.55, 0.95, 1] in initial Python captures | [0.95, 0.8, 0.1, 1] in saved revision-23 fixture after later Python MCP edit |
| Sphere color | [0.65, 0.35, 0.85, 1] | Same |

The project and geometry match. The two Light/Dark capture pairs show the same stable objects and geometry, but the Rust final build opened the later saved file. The cube-color difference is recorded above and is not claimed as a matched pixel state.

## Live comparison

| Scenario | Python live app / client | Rust live app / client | Result |
|---|---|---|---|
| Initial Light and Dark scenes | 1296x859 px at 96 DPI; project e5…fa9 revision 4 | 1296x879 px at 96 DPI; project e5…fa9 revision 23 | Interactive captures pass; 20 px outer-height difference and later Cube color are disclosed |
| Alternate size | Light capture, 1600x900 px | Dark capture, 1600x900 px | Both captured; themes differ, so this is a size record, not a same-theme visual pair |
| MCP edits in GUI | GUI edits and MCP race edit visible; MCP reads revisions/IDs | MCP Sphere color edit at revision 34 visible immediately in viewport and Items | Pass; final Rust visibility check is post-repair |
| Shared history/no-op | No-op stayed revision 4; undo/redo shared across adapters | ID 42b03db2996a4e79bdac9fba29af4033 survived GUI and MCP undo/redo; revisions 35–40 | Pass |
| Save, New/Open, dirty protection | Save/Save As, New revision 13, Open back to e5 revision 14; dirty guard cancellation preserved state | Saved revision 34, closed/reopened same e5 project and four IDs; New e6 revision 0, Open restored e5 revision 34; dirty guard cancellation preserved state | Pass |
| Failed files | Denied/malformed/unsupported/overwrite failures preserved e5 revision 15 | Same failure classes preserved e5 revision 34; sentinel hash unchanged | Pass |
| Replay/races | Same key replayed same result; competing client lost with conflict | Same key replayed same object; stale expected revision and competitor lost with conflict | Pass |
| Long operations | 600k import canceled at 85; 50k import completed; stale result discarded at revision 22 | 50k normalize canceled at 90→95; resource-limit failure and stale-result rejection did not commit | Pass |
| Optional startup and cleanup | Startup failure left GUI usable; closed client returned unavailable; shutdown clean | Regular-file runtime-parent blocker left GUI usable; closed client returned unavailable; normal/failure shutdown removed descriptor/listener | Pass |

## Capture references

Accepted clean scene pairs:
- captures/python-light-initial.png and captures/rust-final-paired-light-initial.png.
- captures/python-dark-initial.png and captures/rust-final-paired-dark-scene.png.

The clean Dark Rust capture is rust-final-paired-dark-scene.png. rust-final-paired-dark-initial.png shows a Settings dialog and is not used as a clean scene. Alternate-size captures are python-light-alt-1600x900.png and rust-final-paired-dark-alt-1600x900.png. All PNG sizes and SHA-256 values are recorded in capture-metadata.json.

The host UI was interactive and unlocked for every input/capture. Edge’s background YouTube video was only a keep-awake aid. The owner-reported interface/floor gaps remain open; visual review and owner acceptance remain pending. RB-08’s native matrix/completion snapshot remains open.

## Current Rust Release recheck (2026-11-06)

The current Rust app/gateway workflow reopened project `e5d81c5226e64f58b838d55d9395afa9` at clean revision 25. The viewport and Items panel showed the same saved Cube and Sphere; the final capture is in `currpair-20261106-072716/captures/` and the complete run record is `currpair-20261106-072716/current-run-manifest.json`.

This is a Rust GUI/MCP check. The historical Python/Rust visual pairs above remain from the earlier recorded build; the Python product was not launched for this current run. The saved Python catalog snapshots were used only for a separate name/order and schema comparison.

All profile names/counts matched (7/52/59/60), but exact input/output schema matches were zero in every profile. The current schema findings are recorded in `currpair-20261106-072716/schema-comparison.json`; no schema deviation has owner approval. The current capture is not a visual acceptance review.

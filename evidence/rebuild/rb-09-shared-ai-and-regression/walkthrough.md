# RB-09 Client and Desktop Walkthrough

## Host preflight and build identity

Before any test input or screenshot, evidence/rebuild/rb-09-shared-ai-and-regression/host_desktop.ps1 reported the current host session active and interactive on the Default desktop. LockApp did not own the session. The owner’s YouTube video remained in Edge in the background only to keep the session awake; it was not captured or used as evidence. Power and lock settings were not changed.

The official Python MCP SDK 2.2.0 ran separate stdio clients against the Python reference and Rust gateway. The Python reference was L:/Builder3D, Python 3.12.10, Git HEAD 3a0160a95c55f68ce9f1e89d17c792816ac001df. The Rust final app was target/release/builder3d-app.exe, SHA-256 d680cf0ba8e4c8bf7d5ccd1536bf7d5720987d676a1a0946c6c3505b613b0cb9. The paired Rust live client used r4_mcp.exe SHA-256 a1dba8751c5a15cafbf6e26fac8db0b8a96f8464bf750ac62336f573b729545b.

## Python live sequence

1. The Python client initialized the 60-tool catalog and read project e5d81c5226e64f58b838d55d9395afa9 at revision 0. It created RB09 Shared Cube, ID 3f449b78621c467b818e8c69925e2b6c, 20x20x20 mm, then RB09 Shared Sphere, ID dff9cc124c6d4d27b4ea9b07d6e4ae6b, 14x14x14 mm. Transforms and framing advanced the scene to revision 4.
2. It saved fixtures/approved/RB09-Paired-Live.3dp and verified a same-position transform as a no-op at revision 4. Light and Dark scene captures were taken at 1296x859 px; an alternate Light capture was taken at 1600x900 px. All host captures were 96 DPI on the 2560x1440 desktop.
3. A repeated idempotency key returned the original Cube ID; the same key with conflicting arguments failed. A stale expected revision failed. In a two-client color race, one request committed and the other received revision_conflict. GUI edits and GUI undo/redo were reflected by MCP revisions and stable IDs.
4. MCP Save, New, and Open acted on the desktop project. New produced project 1e4a8f118734489888a97590d2f91f9c at revision 13; Open restored project e5…fa9 at revision 14. GUI dirty-document confirmation was canceled and preserved state.
5. Denied open/save, malformed project open, unsupported import/export, and overwrite refusal failed at revision 15. The sentinel file hash did not change. Project path, scene, dirty state, and history remained unchanged.
6. A 600k-face STL import reported progress and canceled at 85; the document stayed at revision 15. A 50k-face import completed and was undoable. A later stale import result was discarded with revision_conflict at revision 22; undo restored revision 23. Python viewport and GUI-edit captures are listed in capture-metadata.json.
7. Optional MCP startup failure left the Python GUI usable. The live client received a structured unavailable result; normal close cleaned up the app and bridge.

## Rust live sequence and repair

1. The first live Rust run exposed a defect: MCP opened and edited the project while the GUI remained on Welcome. This initial screenshot/result is retained as rejected evidence for final visibility claims.
2. The scoped repair synchronized presentation after MCP document installation and scene commits and reset project replacement to Scene. Two focused regression tests were added.
3. A real MCP client then opened RB09-Rust-Live.3dp from Welcome. The app showed Scene immediately. Project ID e5…fa9 was at revision 34 with four objects: Cube 3f449b78621c467b818e8c69925e2b6c, Sphere dff9cc124c6d4d27b4ea9b07d6e4ae6b, MCP Cube cff795a29bb0454cbd01932d20e562bf, and Object1 4312fa6a32c64c9f8181f21f3233a203.
4. MCP changed the Sphere color at revision 34; viewport and Items updated immediately. The client saved the approved fixture at revision 34. GUI New prompted about the dirty document; Cancel preserved project/revision/dirty state. MCP New created project e6b3ff7558cf465198ddb2c4d410ebb0 at revision 0; MCP Open restored project e5…fa9 at revision 34.
5. The final Rust build then closed and reopened the saved project through the GUI. Project ID e5…fa9, revision 34, the four original IDs, and clean state were restored. GUI and MCP shared history: MCP add created ID 42b03db2996a4e79bdac9fba29af4033 at revision 35; GUI undo/redo reached 36/37; MCP undo/redo reached 38/39; final undo restored the original four-object clean baseline at revision 40. A no-op transform did not advance revision.
6. Denied open/save, malformed open, unsupported import/export, and overwrite refusal failed while preserving scene/path/revision 34/dirty/history. Overwrite sentinel SHA-256 stayed b14dc62457f04cec9b732d6b2bab2bfd3a5827b4fed640f70c61516e1443f72f.
7. Identical replay of rb09-rust-post-fix-replay returned the same object ID at revision 35; conflicting arguments returned idempotency_conflict. A stale expected revision and competing-client race returned revision_conflict; one race request committed at revision 36.
8. A 600k STL import completed too quickly to cancel and was undone; that timing is recorded rather than counted as a cancellation. A 600k normalize request and 250k normalize request failed bounded-resource checks and made no commit. A 50k STL imported at revision 45; normalize reported running at progress 90, accepted cancellation, and terminated canceled at 95 with revision unchanged. A second normalize operation ran at revision 45 while a GUI Cube was inserted at revision 46; its stale result failed with revision_conflict and was discarded. MCP undo restored the original four objects clean at revision 48.
9. A separate optional-startup run used fixtures/approved/RB09-runtime-parent-blocker.txt as an invalid regular-file parent path. MCP initialization failure was displayed in the still-usable GUI. An official client listed 60 tools and received retryable desktop_unavailable. After Alt+F4, app/gateway/client processes, runtime descriptor, and listener were absent. The normal shutdown path also removed its descriptor/listener and returned desktop_unavailable to a closed client.
10. The final-build paired scene was opened from RB09-Paired-Live.3dp at revision 23. It has the same project ID, stable Cube/Sphere IDs, geometry, sizes, and positions as Python’s revision-4 initial scene. Python later changed the saved Cube color from [0.25, 0.55, 0.95, 1] to [0.95, 0.8, 0.1, 1]. The Rust final capture therefore shows the gold saved color; do not describe the image colors as identical.

## Capture and protocol references

The accepted clean scene pairs are python-light-initial.png with rust-final-paired-light-initial.png and python-dark-initial.png with rust-final-paired-dark-scene.png. Rust’s rust-final-paired-dark-initial.png contains a Settings dialog and is excluded from clean scene-pair claims. Alternate-size captures are python-light-alt-1600x900.png and rust-final-paired-dark-alt-1600x900.png; both are 1600x900, but themes differ. Capture hashes and full metadata are in capture-metadata.json.

Client request/result logs are rb09-python-client.jsonl, rb09-python-client-2.jsonl, rb09-python-optional-client.jsonl, rb09-rust-client.jsonl, rb09-rust-client-2.jsonl, rb09-rust-client-final-pair.jsonl, and rb09-rust-optional-client.jsonl under logs/. The initial live Rust visibility defect is retained in the early client log and rejected-journal.ndjson. Exact gates are in verification.md. No credential or bridge secret is recorded.

## Boundary

The YouTube video was only a keep-awake aid. The owner-reported interface/floor gaps remain open; no visual approval or overall acceptance is claimed. RB-08 native matrix/completion snapshot remains open. RB-10 was not started.

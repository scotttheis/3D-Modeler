# Review pack: owner batch 7, Phase 6 — fixes before F5 (2026-11-16)

Every image here was taken from one `review`-profile build
(`target\review\builder3d-app.exe`) with the review harness, in both themes
(`-light.png`, `-dark.png`). Images show what was painted; they are not
approval and do not test the mouse. Open the release exe to try it by hand.

| Item | Files | What to look at |
|---|---|---|
| OF-63 Double-click an Items row to centre on it | `of63-before` (state `multi`), `of63-after` (state `centre`) | Before: four objects, framed as a whole. After: a double-click on "Object 3" (the sphere) in Items — the camera keeps its angle and centres on the sphere, the row is the only selection, the status bar says "Centred on Object 3". No Undo entry. A group row centres on the whole group (its tooltip says so). A hidden object's row is selected but the camera stays put, with the message "… is hidden; show it to centre on it" (framing skips hidden objects everywhere, as in the Python build). The 3D-view double-click and AI `frame_view` now use the very same framing, so all three give the identical camera (test `items_and_viewport_double_click_frame_like_mcp_frame_view`). |
| OF-64 Settings note no longer covers the ribbon | `of64-after` (state `settings-note`); before: the first-run shot in `../2026-11-15-review-phase5-mcp/` | The "Imported supported Python settings…" message is in its own strip under the ribbon, pushing the workspace down instead of drawing over the button labels, with a **Dismiss** button. The state runs the real first-start import with no Rust settings file and a stand-in Python registry value. The AI sees the note in `app_settings` (`settings_notice`) and closes it with `set_app_settings` `dismiss_settings_notice` (test `settings_note_is_read_and_dismissed_like_its_button`). |

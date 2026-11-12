# RB-03 visual comparison record

**Status:** Captures inspected; owner visual acceptance pending. The complete side-by-side operation board is [visual-comparison.html](visual-comparison.html). Each pair links to the original 1280 × 840 client capture. The Python reference capture is always on the left.

## What the old app showed while performing RB-03 actions

- **Lifecycle:** New displayed a native `Save Changes` dialog with Save, Discard, and Cancel over the existing viewport. Save As opened the Windows `Save Project` picker with `.3dp` selected. Exit used the same native dirty guard.
- **Model input:** Add/Open opened the Windows `Add 3D Model` picker. The unitless STL mm and inch captures show `2 × 3 × 4 mm` and `50.8 × 76.2 × 101.6 mm`. The old UI places these values in the lower Resize strip.
- **Structured imports:** the old app displayed separate `First part` / `Second part` objects after 3MF, glTF, and GLB imports. Cancelling the picker preserved the existing scene. A malformed STL left the scene in place and set the status text to `Import contained no objects`.
- **Export:** the old app opened the native `Export Mesh` picker with `3MF Model (*.3mf)` selected. A successful no-selection 3MF export updated the status line. Existing-file confirmation used a `Confirm Save As` Yes/No prompt; choosing No left the target hash unchanged.
- **Recents:** the old welcome surface showed large wireframe thumbnails in two columns and needed scrolling. Opening a model recent used the additive import path. Missing files showed a `File Not Found` dialog and the row was removed after dismissal; corrupt `.3dp` and unsupported WRL entries showed native error dialogs while preserving the current scene.

## Observed Python/Rust differences

- Python uses Windows-native Open/Save dialogs and message boxes in these workflows. Rust uses in-app forms for import, project Save As, export, and overwrite prompts.
- Python recent cards use two columns and large wireframe thumbnails. Rust uses four compact columns with a status and Open/Remove action. Rust thumbnails are small shaded mesh previews when decoding succeeds; Python previews are wireframe drawings.
- Python reports malformed import as a status-line message; Rust shows the parser reason in the import form. Python presents missing/corrupt/unsupported recent errors in modal dialogs; Rust leaves a status on the welcome surface and exposes a Remove action for missing files.
- Scene counts and recent ordering differ in some pairs because the actual sessions recorded different successful operations. The board marks these cases and does not label them as same-state pixel matches.

The desktop locked during an additional attempt to capture Python native exports for each format, so no per-format Python picker replay is claimed. The existing Python native export capture shows the shared Export Mesh picker and successful 3MF export. All seven formats were exported and independently verified in Rust; see [format-unit-results.md](format-unit-results.md) and [verification.md](verification.md).

# RB-03 native walkthrough record

**Status:** The Rust lifecycle/import/export/recent workflows and required output verification were exercised in the visible app. Python-reference operations were captured during each major action and paired with Rust in [visual-comparison.html](visual-comparison.html). Owner visual acceptance remains pending; no visual pass is claimed.

## Capture environment and build identity

- Windows 11 Enterprise; primary display 2560 × 1440 at 100% scale. Python and Rust client areas were 1280 × 840 physical/logical pixels at 96 DPI.
- Python reference: `L:\Builder3D\venv\Scripts\pythonw.exe -m src`, Python 3.12.10. It ran with an isolated `LOCALAPPDATA` and temp directory under `E:\3D-Modeler\target\rb03-python-profile`.
- The Python reference source remained read-only. Its post-walkthrough Git status exactly matches the saved pre-capture baseline (116 lines). Original Qt settings were restored: `theme_mode=light`; `recent_files` absent; `default_import_unit` absent.
- Rust native capture executable: `target\rb03-recents-build\debug\modeler3d-app.exe`, SHA-256 `dec750d4a6ef65a1a038fd5221f5b941b9799789ec21ec31600becd18d1cd8b1`. Release executable: `target\release\modeler3d-app.exe`, SHA-256 `de547662b3fca993d631b7ef931aaabd30c28f4f17dbfa629db4adc19949a999`.
- The two applications were captured sequentially; no 3D Modeler Python and Rust windows overlapped. `capture-metadata.toml` records themes, dimensions, paths, and the paired image list.

## Visible workflow results

| Workflow | Rust native result | Python-reference evidence |
|---|---|---|
| New, Save, Save As, Open, Exit | Dirty New/Open/Exit Save/Discard/Cancel, Save As cancel, overwrite cancel, successful save/reopen, and corrupt-open preservation were exercised. | Native Save Project dialog, Save Changes prompt, New Cancel/Discard, project saved/reopened, and dirty Exit prompt were captured. |
| Unitless STL | Imported in mm and inches. Displayed dimensions were 2 × 3 × 4 mm and 50.8 × 76.2 × 101.6 mm. | Both dimensions and the reference default-unit setting were captured. |
| OBJ and PLY | Added to a saved project; names, geometry, color, transforms, and additive scene state were checked. | The Python scene with the same project plus OBJ and PLY was captured. |
| 3MF, glTF, GLB | Multi-object fixtures imported with names, supported colors, transforms, authoritative units, placement, centering, and framing checked. | The old app's resulting two-part scenes were captured after each import. |
| Import cancel and errors | Picker Cancel and malformed STL preserved scene state; Rust showed the parser reason in its import form. | The old picker and preserved scene were captured; malformed STL reported `Import contained no objects` in the status line. |
| Seven exports | STL, OBJ, PLY, 3MF, glTF, GLB, and WRL were exported from the native Rust app and independently inspected. Selection/visibility cases and no-visible-object failure were exercised. | The old `Export Mesh` picker, successful no-selection 3MF export, overwrite prompt, and overwrite cancellation were captured. The Python GUI was not replayed separately for every output format. |
| Recents | Restart persistence, project/model dispatch, previews/fallbacks, and missing/corrupt/WRL behavior were exercised. | The old two-column recent cards, model open, missing warning/removal, corrupt-project error, WRL error, and Light/Dark welcome states were captured. |

## Observed old-app appearance

The Python app used Windows-native Open/Save dialogs and message boxes. New and Exit placed a native `Save Changes` prompt over the 3D viewport; Add/Open and Export opened Windows file pickers. Recent cards used two columns with large wireframe thumbnails and a scrolling viewport. The Rust app uses in-app import/export/project forms and four compact recent columns with status/actions and small shaded previews. Failure presentation also differs: Python often uses a native modal plus status text, while Rust reports recent errors in the Welcome status area and import parser errors in the form.

The paired board includes exact filenames and calls out state differences where the recent ordering or prior scene operations were not identical. Images were inspected and differences recorded; the owner still needs to judge appearance. See [visual-comparison.md](visual-comparison.md).

## Evidence boundary and remaining review

- All seven Rust export routes and selected/visible behavior passed independent verification; see [format-unit-results.md](format-unit-results.md) and [verification.md](verification.md).
- The old Python GUI export captures cover the shared Export Mesh picker and a successful 3MF export. A second per-format Python picker replay was attempted after the app had been closed, but Windows returned no foreground window (`GetForegroundWindow` = 0). No input or capture was claimed from that locked desktop. The extra Python process was stopped and original settings were restored.
- Native Rust showed Saved/Unsaved after save/edit and captured imported-scene undo. The full saved-checkpoint undo/redo/new-branch matrix is covered by the focused automated regression; the native live sequence is partial.
- Owner visual review is pending. No RB-04 implementation has started.

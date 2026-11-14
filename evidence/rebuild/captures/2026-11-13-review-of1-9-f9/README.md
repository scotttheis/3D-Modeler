# Review pack: F9 and OF-1 to OF-9 (2026-11-13)

One build, one capture set. Every image was taken from the same release exe
(`target\release\builder3d-app.exe`, built 2026-11-13 15:22 from the source
at `21f13ee`; `cargo build --release` reports it current) using the review
harness (plan section 3.5). Each file exists as `-light.png` and `-dark.png`.

Captures show what was painted. They are not approval, and they do not test
pointer behaviour. For the hands-on check, open the same exe.

| Item | Files | What to look at |
|---|---|---|
| F9 floor depth fight | `f9-framed`, `f9-close-rim`, `f9-rim-closeup` | Where the cube meets the floor: no dark serration. The faint pale dotted fringe on the rim is still there (cause not found). |
| OF-1 floor numbers | `of1-floor-empty`, `of1-floor-far`, `of1-floor-top` | Numbers painted flat on the floor every 50 mm, shrinking with distance. |
| OF-1 resize view | `of1-resize` | White box with corner dots, double chevrons, one size label. |
| OF-3 resize handles | `of3-resize-drag` | +X handle held mid-drag: pale drag band. |
| OF-2 shadow and reflection | `of2-shadow-multi`, `of2-shadow-low` | Soft grey contact shadow plus cast shadow; faint reflection. GPU only. |
| OF-4 top-bar hover | `of4-hover-tab`, `of4-hover-button`, `of4-hover-ribbon` | Tile behind "Object" tab, "Save" button, "Cube" ribbon button. |
| OF-6 wireframe | `of6-wireframe-multi`, `of6-wireframe-close` | Clean triangle edges over the shaded model. |
| OF-7 Paint / Material | `of7-paint-unselected`, `of7-material-dialog` | Paint tab enabled with nothing selected; color dialog with swatches and picker. |
| OF-8 Edit sculpt tools | `of8-edit-unselected` | Grab, Crease, Smooth Brush, Blob, Normalize enabled with nothing selected. |
| OF-9 Merge color | `of9-merge` | Merged red and blue cubes come out red (top of the Items list). |

OF-5 (X-ray) is not in the pack: it waits for the owner's shots.

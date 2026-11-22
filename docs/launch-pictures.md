# Pictures for the GitHub page

Pictures come from the current Release build with the review harness
(`MODELER3D_REVIEW_CAPTURE`, see `crates/modeler3d-app/src/review_capture.rs`),
never from photographing the desktop. The owner approves anything shown publicly.

## In the repo now (placeholders from the stock shapes)

| File | Used in | Shows |
|---|---|---|
| `docs/images/main-window-dark.png` | README top | Four shapes, Items list (state `multi`, dark) |
| `docs/images/main-window-light.png` | README | Same, light theme |
| `docs/images/combine.png` | README table | Merge result (state `merge`) |
| `docs/images/material.png` | README table | Material dialog (state `material`) |
| `docs/images/repair.png` | README table | Repair dialog (state `repair`) |
| `docs/images/home.png` | spare | Home screen (state `welcome`) |

Recapture example (Windows, from the repo root):

```
$env:MODELER3D_REVIEW_CAPTURE="docs\images\main-window-dark.png"
$env:MODELER3D_REVIEW_STATE="multi"; $env:MODELER3D_REVIEW_THEME="dark"
target\release\modeler3d-app.exe
```

## Still needed (needs better models and the owner's approval)

| Id | Picture | Size | Notes |
|---|---|---|---|
| B1 | Hero GIF: build a part | 1280 x 720, 25-40 s | Shapes, Subtract, emboss text, export 3MF, open in slicer. Replaces the top picture. |
| B2 | Full window, dark and light | 1920 x 1080 | A nice finished model, not a cube. |
| C1 | Insert gallery | 1600 x 1000 | The shapes. |
| C2 | Subtract with one object selected | 1600 x 1000 | Before, after, result. |
| C3 | Print check warning | 1600 x 1000 | Finding shown beside the model. |
| C4 | AI driving the app | 1600 x 1000 | Chat plus the live model. |
| C6 | Photo of a printed part | any | Phone photo, plain background. |
| D1 | Diagram: one engine, two front ends | SVG | Desktop and MCP over one command layer. |

The social preview image is `docs/images/social-preview.png` (1280 x 640).
GitHub has no API for it: upload it by hand under Settings, General, Social preview.

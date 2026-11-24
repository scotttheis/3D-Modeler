# Pictures for the GitHub page

Pictures come from a Release build, captured by the app itself (the MCP
`capture_window` tool, or the review harness `MODELER3D_REVIEW_CAPTURE`, see
`crates/modeler3d-app/src/review_capture.rs`), never from photographing the
desktop. The owner approves anything shown publicly.

## Used in the README

| File | Shows | How it was made |
|---|---|---|
| `docs/media/fidget-spinner.gif` / `.mp4` | B1 hero: building a fidget spinner, then the real print spinning | 1.0.4, MCP-driven shoot; 1280 x 720 GIF (12 fps, 6.8 MB) linking to the 1920 x 1080 MP4 |
| `docs/images/designed-printed.jpg` | C6: the model next to the printed part | 1.0.5 top view (grid off) beside the last frame of the owner's phone clip |
| `docs/images/window-dark.png` / `window-light.png` | B2: full window with the spinner | 1.0.5, maximized 2560 x 1377, UI size Big, scaled to 1600 wide; README swaps them with the visitor's GitHub theme |
| `docs/images/subtract-before.png` / `subtract-after.png` | C2: cutters in X-ray, then one Subtract | 1.0.5, same settings, 1280 wide |

Lighting for these shots ("soft sheen"): ambient 36, azimuth 40, contact
shadow 40, elevation 45, highlight sharpness 55, intensity 100, screen
aligned, specular 25. Camera for the window shots: yaw -58, pitch 30.

UI size Big on a 2560-wide window gives the same text size as a 1280-wide
window at Normal, but twice as sharp, so the UI stays readable at README width.

## Older placeholders (stock shapes, no longer in the README)

`main-window-dark.png`, `main-window-light.png`, `combine.png`, `material.png`,
`repair.png`, `home.png`. Recapture example (Windows, from the repo root):

```
$env:MODELER3D_REVIEW_CAPTURE="docs\images\main-window-dark.png"
$env:MODELER3D_REVIEW_STATE="multi"; $env:MODELER3D_REVIEW_THEME="dark"
target\release\modeler3d-app.exe
```

## Still needed

| Id | Picture | Size | Notes |
|---|---|---|---|
| C1 | Insert gallery | 1600 x 1000 | The shapes. |
| C3 | Print check warning | 1600 x 1000 | Finding shown beside the model. |
| C4 | AI driving the app | 1600 x 1000 | Chat plus the live model. |

D1 (one engine, two front ends) is a Mermaid diagram in the README.

The social preview image is `docs/images/social-preview.png` (1280 x 640).
GitHub has no API for it: upload it by hand under Settings, General, Social preview.

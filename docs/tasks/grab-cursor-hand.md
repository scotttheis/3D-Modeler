# Grab cursor: Claw / Hand choice

Status: **owner approved 2026-11-20** ("approved"). Owner wants to **modify
the hand drawing** next, in a fresh session. Keep everything else as it is.

## What exists (commits `2bc8aab`, `587bf62`)

- Settings > **Grab cursor** (directly below Viewport lighting): Claw | Hand.
  Claw is the default. Saved as `"grab_cursor": "claw" | "hand"`.
- MCP parity: `set_app_settings` takes `grab_cursor`; `app_settings` reports
  it (`crates/modeler3d-mcp/src/r4/schema_contract.json`). Parity row
  `modeler3d.ui.settings.grab_cursor` in `evidence/rebuild/parity-matrix.toml`.

## Where the hand is drawn

The original hand was redrawn on 2026-11-20. `OPEN_HAND` and `CLOSED_HAND`
in `crates/modeler3d-app/src/grab_cursor.rs` are original quadratic contours:
each segment is `[control_x, control_y, end_x, end_y]`, in points relative to
the palm-centre hotspot. Both start/end at `(-5, 7)`.

`hand_geometry` samples and caches each silhouette once, filling concave finger
gaps with trapezoids (egui's convex fill would bridge them). `paint_hand` adds
a light outer keyline, dark contour, and thumb/finger creases. Overall scale
is 1.05. Keep the claw and cursor selection behavior unchanged.

## How to see a change

Build the review profile, then capture the open and held hand in both themes
(plan section 3.5; `MODELER3D_REVIEW_GRAB_CURSOR` selects the look):

```powershell
. .\scripts\Initialize-BuildClang.ps1
cargo build --profile review -p modeler3d-app
$dir = "evidence\rebuild\captures\<date>-hand-redraw"; New-Item -ItemType Directory -Force $dir
foreach ($theme in 'dark','light') { foreach ($state in 'grab-ready','grab-held') {
  $env:MODELER3D_REVIEW_CAPTURE = "$dir\$state-hand-$theme.png"
  $env:MODELER3D_REVIEW_STATE = $state; $env:MODELER3D_REVIEW_THEME = $theme
  $env:MODELER3D_REVIEW_GRAB_CURSOR = 'hand'
  Start-Process -Wait .\target\review\modeler3d-app.exe } }
```

Use `Start-Process -Wait` (a plain `&` call returns early and can leave an
empty PNG). The cursor is about 25 px: crop and enlarge the region around
x 712-762, y 455-505 (1280x840 capture) with System.Drawing to inspect it.
Previous pack for before/after: `evidence/rebuild/captures/2026-11-20-grab-cursor-hand/`.

## Checks before handing back

- `cargo test -p modeler3d-app grab_cursor` (the test only requires the hand
  and claw to paint a different number of shapes; keep that true).
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`.
- `.\scripts\Update-CurrentBuild.ps1 -Launch` so the owner sees it in Release.
- Commit on `rebuild/work`, G: snapshot, update `docs/STATUS.md`.

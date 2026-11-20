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

All of it is `paint_hand` in `crates/modeler3d-app/src/grab_cursor.rs`, with
these constants just above it (units are points before `HAND_SCALE`; y is
down; (0,0) is the hotspot at the palm centre):

| Constant | Controls |
|---|---|
| `FINGERS` | four finger x positions and open fingertip heights |
| `KNUCKLE_TOP` | knuckle height above the palm when closed (fist) |
| `FINGER_WIDTH`, `HAND_OUTLINE` | finger thickness and dark outline width |
| `PALM` | palm rectangle (left, top, right, bottom) |
| `HAND_SCALE` | overall size (1.2 sits beside the claw) |

Open thumb: `capsule(at(-4.5, 5.), at(-10., -0.5))`. Closed thumb across the
fist: `capsule(at(-6., 2.), at(1.5, 1.))`. Draw order matters: palm outline,
open thumb, fingers (outline then fill each), palm fill (hides finger roots),
then the closed thumb. Colours match the claw (dark outline, light fill).
Do not change the claw (`paint`) unless the owner asks.

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

# RB-01 Comparison Board

**Review state:** Four matched live Light/Dark client captures are prepared. Owner visual review and explicit approval are pending.  
**Comparison target:** 1280 × 840 logical client pixels at 100% display scaling.  
**Captured state:** Empty Welcome, no recent projects, Insert tab active. This does not cover a selected scene, model list, transform strip, or interactive New/Cube flow.

## Dark pair

![Python 3D Modeler on the left and Rust eframe on the right, Dark theme, 1:1 client pixels](captures/comparison-dark-python-left-rust-right.png)

- [Python 3D Modeler Dark capture](captures/python-dark-client-1280x840.png)
- [Rust eframe Dark capture](captures/rust-dark-client-1280x840.png)
- [1:1 side-by-side comparison](captures/comparison-dark-python-left-rust-right.png)

## Light pair

![Python 3D Modeler on the left and Rust eframe on the right, Light theme, 1:1 client pixels](captures/comparison-light-python-left-rust-right.png)

- [Python 3D Modeler Light capture](captures/python-light-client-1280x840.png)
- [Rust eframe Light capture](captures/rust-light-client-1280x840.png)
- [1:1 side-by-side comparison](captures/comparison-light-python-left-rust-right.png)

## Capture measurements

| Capture | Client logical px | PNG physical px | Window DPI | Display scale | Client origin on screen | Window |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| Python Dark | 1280 × 840 | 1280 × 840 | 96 | 100% | 88, 71 | Normal/restored |
| Python Light | 1280 × 840 | 1280 × 840 | 96 | 100% | 88, 71 | Normal/restored |
| Rust Dark | 1280 × 840 | 1280 × 840 | 96 | 100% | 88, 71 | Normal/restored |
| Rust Light | 1280 × 840 | 1280 × 840 | 96 | 100% | 88, 71 | Normal/restored |

The host primary display is 2560 × 1440 physical pixels with a 2560 × 1392 work area. System and per-window DPI were 96 (100%). Each app window used the same 1280 × 840 client area. The outer window was placed at screen position 80, 40, measured 1296 × 879, and was temporarily topmost during direct client-area capture. The saved screenshots were not resized or cropped. Capture times, window handles, PIDs, byte counts, SHA-256 values, executable identity, and source hashes are in [capture-metadata.toml](capture-metadata.toml).

## Observed differences in this state

- Python places the 3D Modeler title and action cards toward the left and shows its right-side selection/action and Items panels.
- Rust centers its brand and New Scene/Open Project cards, then shows a full-width empty Recent area. The matching right-side panel is not visible in this Rust Welcome state.
- The Python hamburger uses three horizontal lines. The Rust capture shows a small outlined square in that position.
- Both apps show the Insert ribbon and the same empty Welcome intent. Their ribbon spacing, proportions, title placement, panel presentation, and empty Recent treatment differ. Regional tolerances and owner acceptance are not assigned yet.

These are observed visual differences, not approval decisions. No object was created or imported, and no selection or transform state was captured. The Items-panel image only compares what each app presents on the empty Welcome screen; it does not verify matching object rows.

## Serial capture and isolation

Python Dark, Python Light, Rust Dark, and Rust Light were captured in that order with no target windows overlapping. Each app was closed before the next app was opened. The background MCP gateway remained running and was not stopped; it had no target UI window and did not prevent the serial captures.

Python ran with isolated QSettings and runtime paths so the Light/Dark capture setup did not change the Python user's saved settings or runtime files. Rust used isolated app-data paths; Light was selected in its Settings dialog. Theme persistence across an app restart was not tested. All four images were opened and visually inspected.

The 2026-09-19 Python previews and 2026-10-20 Insert ribbon crops remain available as historical references in [reference](reference) and are not part of these matched captures.

## Review gate

Owner acceptance remains pending. Review the two 1:1 comparison images above. RB-02 has not started.
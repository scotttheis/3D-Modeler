# RB-01 Visual Foundations

**Date:** 2026-10-31  
**Status:** Visual foundations are implemented; four matched live captures are prepared. Owner visual review and explicit approval remain pending.

## Scope and result

This slice updates the existing eframe shell's visual foundations using the read-only Python Builder3D source, its style guide, and the RB-00 asset inventory. The app remains on eframe 0.37.0. The changes cover the original Light and Dark palettes, platform UI font setup, ribbon geometry and artwork, Welcome and Items presentation, and the contextual Move/Rotate/Resize strip.

Serial live captures of the read-only Python Builder3D reference and the Rust eframe app were completed in Light and Dark at 1280 x 840 logical client pixels. The background MCP gateway remained active but had no target UI window; it was not stopped. All four client sizes and per-window DPI were measured, and the 1:1 comparison images were inspected. The captured state is the empty Welcome screen with Insert active. Owner review remains pending; no visual acceptance is claimed and RB-02 has not started.

## Visual decisions implemented

- The first-run theme is now Dark, matching Python's `DEFAULT_THEME_MODE`. Theme choice and the collapsed/pinned command-row preference remain persisted through the existing preferences path.
- `ThemePalette` now uses the source Light and Dark tokens, including hover, pressed, disabled, border, selection, navigation, viewport, and grid colors. The Light viewport uses the source `COLOR_BG` value (`#F0F0F0`); the Dark viewport color is converted from the source `COLOR_BG` tuple.
- On Windows the shell loads Segoe UI from the system font directory when present. Platform fallbacks are provided for macOS and Linux. The eframe sizes map the source's 9.5 pt general control text to 12.7 logical points, with the source's 13 pt ribbon tabs represented at 17 logical points.
- The command row follows the source's 91 px height, 46 px icons, 3 px group dividers, 8 px button radius, and 5 px icon/text lift. Command button widths use measured labels with the source's 66-104 px bounds.
- The top row keeps the hamburger at the left, the five ribbon tabs, compact favorites with width-aware overflow, 3D Print, and icon-only Save, Save As, Undo, and Redo controls in the source order. Utilities use 36 px hit boxes. The existing tab toggle and persisted Pin behavior remain in place.
- The Welcome surface uses the original Builder3D brandmark and original New Scene/Open artwork. Recent entries use responsive cards and source icons; generated mesh thumbnails are still not implemented.
- The Items panel uses source primitive artwork for matching shapes and keeps the action card and list in the existing panel. The panel's current Rust layout uses a 236 px default width within its existing 210-250 px range; live proportion comparison is pending.
- The bottom contextual strip presents Move, Rotate, and Resize selection states and uses the existing numeric-transform commands. This is a shell presentation cue; it does not integrate the 3D renderer or add viewport gizmos.
- Supplied transparent artwork is decoded as RGBA without changing its stored bytes. The selected theme chooses the matching action and Insert directories. The Rust shell and staged native package resolve assets inside the Rust product tree; they do not load from `L:\Builder3D`.

The command mapping preserves the source's `ExRay.png`, `Subtract.png`, and `Simplify.png` mapping for both Simplify and Subdivide. Open Project and Add/Open use `Folder and Open.png`; Pin uses its theme-specific action image. Undo/Redo are a source/contract conflict: Python's utility buttons call `make_icon`, which resolves `Undo.png` and `Redo.png` from `src/icon_assets.py`, while the current `docs/ui-style-guide.md` says those controls use generated filled curved arrows. The Rust shell follows the current Python icon-loading path, but live visual evidence is needed to resolve the discrepancy. Tetrahedron has no supplied Insert image in the eight-shape source set; the existing vector fallback remains for that unmapped Rust-only case. The root `pin.jpg` and `snap.png` are included unchanged in the packaged asset tree, while themed Pin/Snap images shadow those fallbacks in the source loader.

## Artwork provenance and package-size conflict

The RB-00 manifest describes 108 files: 44 action images per theme, eight Insert images per theme, and four shared branding/legacy files. All 108 were copied unchanged into `crates/builder3d-app/assets/original`. The RB-01 manifest retains the source path, original facts, theme, logical action, dimensions, format, and alpha treatment, and adds the Rust/package destinations and source/package hashes.

Those source assets total **43,340,965 bytes**. The existing installed-package limit is **41,943,040 bytes (40 MiB)**, so the original artwork alone exceeds it by **1,397,925 bytes** before the executable, launchers, or installers are counted. The 15 MiB archive limit is also unverified. I have not resized, recompressed, removed, or substituted source art, and I did not run package staging. This conflict must be resolved through a later packaging decision while retaining the original files and provenance.

## Capture dimensions and availability

The comparison target is 1280 x 840 logical client pixels. Python source startup requests a 1280 x 820 outer window; the capture session placed and measured its client at the comparison target. Rust requests a 1280 x 840 inner client and was measured at the same size. All four captures were 1280 x 840 logical and physical pixels.

The host primary display measured 2560 x 1440 physical pixels with a 2560 x 1392 work area. System DPI and GetDpiForWindow for both applications were 96 DPI (100% scale). The windows were normal/restored at screen origin 80, 40 with an outer size of 1296 x 879; the client origin was 88, 71. They were temporarily topmost during direct client capture. The saved captures were not resized or cropped.

Capture order was Python Dark, Python Light, Rust Dark, Rust Light. Each application was closed before opening the next, with no target windows overlapping. Python used isolated QSettings and runtime paths; its Light capture profile was prepared without modifying the user's saved settings. Rust used isolated app-data paths and Light was selected through Settings. Theme persistence across restart was not checked. The active background MCP gateway was left untouched.

The two comparison images place Python on the left and Rust on the right at 1:1 pixels. The captures show the empty Welcome state with no recent projects and Insert active. Python places its Welcome content left and shows the right selection/action and Items panels; Rust centers its Welcome content, shows a full-width empty Recent area, and has no matching right panel in this state. The Rust hamburger appears as a small outlined square. These are observed differences for this captured state; regional tolerances and owner acceptance remain pending. See the [comparison board](capture-board.md) and per-image hashes in [capture-metadata.toml](capture-metadata.toml).

The captures do not show a selected scene, populated object list, transform strip, native import dialog, or New/Cube workflow. They provide matched empty-Welcome and Insert-ribbon evidence only. Historical Python previews and ribbon crops remain identified as historical; they are not substituted for the current captures.

## Verification and limitations

The current build command, cargo build --package builder3d-app --locked, exited 0. Earlier RB-01 records document successful formatting, workspace check, and library Clippy checks; those commands were not rerun in this capture session. No automated tests were run. All four live captures and both side-by-side images were opened and visually inspected. Their measured dimensions and hashes are recorded in capture-metadata.toml.

No New, Cube creation, native import, shaded viewport, exact pick, gizmo move, undo, or save/reopen interaction was exercised. The Rust Settings dialog was used to select Light; persistence across restart was not checked. No package staging was attempted. L:\Builder3D remained read-only. Both target apps were closed after capture; the background MCP gateway remained running. Temporary isolated profiles and dialog diagnostics remain under the ignored project target directory and are excluded from the recovery snapshot.

RB-01 visual acceptance is still pending. The empty-state comparison does not establish populated Items-panel or transform-strip parity, viewport behavior, DPI behavior at other scales, or owner acceptance.

## Review and exact next work

Review both 1:1 theme comparisons in [capture-board.md](capture-board.md), including the recorded Welcome layout, right-panel, hamburger, and ribbon differences. Explicit owner approval is still required before RB-02. Do not start RB-02 until that approval is given.
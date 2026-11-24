<p align="center">
  <img src="crates/modeler3d-app/assets/original/Modeler3D.png" alt="3D Modeler logo" width="128">
</p>

<h1 align="center">3D Modeler</h1>

<p align="center">
  A simple, free 3D modeler for 3D printing. Windows, macOS and Linux.<br>
  Inspired by Microsoft 3D Builder, with an AI (MCP) server built in.
</p>

<p align="center">
  <a href="https://github.com/scotttheis/3D-Modeler/releases/latest"><img src="https://img.shields.io/github/v/release/scotttheis/3D-Modeler?label=release&color=2f81f7" alt="Latest release"></a>
  <a href="https://github.com/scotttheis/3D-Modeler/actions/workflows/ci.yml"><img src="https://github.com/scotttheis/3D-Modeler/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue" alt="Apache-2.0"></a>
  <img src="https://img.shields.io/badge/made%20with-Rust-dea584?logo=rust&logoColor=white" alt="Made with Rust">
  <a href="docs/mcp.md"><img src="https://img.shields.io/badge/AI%20ready-MCP-8a63d2" alt="AI ready: MCP"></a>
</p>

<p align="center">
  <a href="https://github.com/scotttheis/3D-Modeler/releases/latest"><img src="https://img.shields.io/badge/Download-Windows-0078D4?style=for-the-badge&logo=windows&logoColor=white" alt="Download for Windows"></a>
  <a href="https://github.com/scotttheis/3D-Modeler/releases/latest"><img src="https://img.shields.io/badge/Download-macOS-111111?style=for-the-badge&logo=apple&logoColor=white" alt="Download for macOS"></a>
  <a href="https://github.com/scotttheis/3D-Modeler/releases/latest"><img src="https://img.shields.io/badge/Download-Linux-E95420?style=for-the-badge&logo=linux&logoColor=white" alt="Download for Linux"></a>
</p>

<p align="center">
  <a href="docs/media/fidget-spinner.mp4"><img src="docs/media/fidget-spinner.gif" alt="Building a fidget spinner in 3D Modeler around a bearing and finger caps, then the real 3D-printed spinner spinning in a hand" width="860"></a><br>
  <sub>Designed in 3D Modeler, printed, and spun. <a href="docs/media/fidget-spinner.mp4">Watch in full quality (MP4)</a>.</sub>
</p>

## From screen to hand

<p align="center">
  <img src="docs/images/designed-printed.jpg" alt="Left: the fidget spinner in 3D Modeler. Right: the same spinner 3D-printed in gold, held in a hand" width="860">
</p>

The spinner body is just cylinders: a hub and three lobes merged into one
piece, then four more cylinders as cutters for the bearing pocket and the
weight holes. The bearing and finger caps were imported as 3MF to check the
fit. One Subtract, one 3MF export, and it went to the printer.

<table align="center"><tr><td align="center"><img src="crates/modeler3d-app/assets/original/insert_icons/Cube.png" width="96" alt="Cube"><br><sub>Cube</sub></td><td align="center"><img src="crates/modeler3d-app/assets/original/insert_icons/Cylinder.png" width="96" alt="Cylinder"><br><sub>Cylinder</sub></td><td align="center"><img src="crates/modeler3d-app/assets/original/insert_icons/Pyramid.png" width="96" alt="Pyramid"><br><sub>Pyramid</sub></td><td align="center"><img src="crates/modeler3d-app/assets/original/insert_icons/Cone.png" width="96" alt="Cone"><br><sub>Cone</sub></td><td align="center"><img src="crates/modeler3d-app/assets/original/insert_icons/Sphere.png" width="96" alt="Sphere"><br><sub>Sphere</sub></td><td align="center"><img src="crates/modeler3d-app/assets/original/insert_icons/Torus.png" width="96" alt="Torus"><br><sub>Torus</sub></td><td align="center"><img src="crates/modeler3d-app/assets/original/insert_icons/Hexagon.png" width="96" alt="Hexagon"><br><sub>Hexagon</sub></td><td align="center"><img src="crates/modeler3d-app/assets/original/insert_icons/Wedge.png" width="96" alt="Wedge"><br><sub>Wedge</sub></td></tr></table>

<table align="center"><tr><td align="center"><img src="crates/modeler3d-app/assets/original/action_icons/Merge.png" width="96" alt="Merge"><br><sub>Merge</sub></td><td align="center"><img src="crates/modeler3d-app/assets/original/action_icons/Subtract.png" width="96" alt="Subtract"><br><sub>Subtract</sub></td><td align="center"><img src="crates/modeler3d-app/assets/original/action_icons/Intersect.png" width="96" alt="Intersect"><br><sub>Intersect</sub></td><td align="center"><img src="crates/modeler3d-app/assets/original/action_icons/Hollow.png" width="96" alt="Hollow"><br><sub>Hollow</sub></td><td align="center"><img src="crates/modeler3d-app/assets/original/action_icons/Repair.png" width="96" alt="Repair"><br><sub>Repair</sub></td><td align="center"><img src="crates/modeler3d-app/assets/original/action_icons/Material.png" width="96" alt="Material"><br><sub>Material</sub></td></tr></table>

## Why

3D Builder was the easiest way to make something printable. It is gone from
most computers and never ran outside Windows. 3D Modeler keeps the same idea:
add shapes, move them, combine them, check them, print them. Nothing has to be
learned first. It is a small desktop app written in Rust, with no account and
no tracking.

## What it does

- **Shapes:** cube, cylinder, pyramid, cone, sphere, torus, hexagon and wedge.
- **Move, rotate and resize** with on-screen arrows or typed numbers in millimetres.
- **Combine:** Merge, Subtract and Intersect. Select one object and click
  Subtract to cut away everything it touches.
- **Edit meshes:** Grab, Smooth, Crease, Blob, Simplify, Subdivide, Hollow, Mirror, Align.
- **Print check:** finds holes and other problems and can repair them.
- **Look:** colours, textures, paint, X-ray and wireframe, light and dark themes.
- **Files:** opens and saves `.3dp` projects; imports and exports STL, OBJ, PLY,
  3MF, glTF/GLB and VRML (WRL).
- **Send to your slicer** from the toolbar.
- **AI built in:** an MCP server lets an assistant such as Claude use the same
  tools you do. See [`docs/mcp.md`](docs/mcp.md).

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: light)" srcset="docs/images/window-light.png">
    <img src="docs/images/window-dark.png" alt="The 3D Modeler window with the fidget spinner, the Insert ribbon and the Items list" width="860">
  </picture><br>
  <sub>Light and dark themes. This picture follows your GitHub theme.</sub>
</p>

### Cut with one click

Place cutter shapes where you want holes, then **Subtract**. X-ray shows what
will be cut before you commit.

| Before: cutters in X-ray | After: one Subtract |
|---|---|
| ![Four turquoise cutter cylinders inside the gold spinner body, seen in X-ray, with the Edit ribbon open](docs/images/subtract-before.png) | ![The finished spinner body with the bearing pocket and three weight holes cut out](docs/images/subtract-after.png) |

## Download

Get the installer for your system from the
[Releases page](https://github.com/scotttheis/3D-Modeler/releases/latest).

| System | File | Notes |
|---|---|---|
| Windows | `3D-Modeler-<version>-windows-x86_64.msi` | Not code-signed, so Windows SmartScreen shows "More info", then "Run anyway". |
| macOS | `3D-Modeler-<version>-macos-universal.dmg` | Not notarized. Right-click the app, choose Open, then Open again. |
| Linux | `3D-Modeler-<version>-linux-x86_64.tar.gz` | Unpack it and run the installer script or launcher inside. |

The app checks GitHub once at startup for a newer version and asks before
downloading anything. Releases also carry `3D-Modeler-SHA256SUMS.txt` so you
can verify a download.

## Use it with an AI assistant

The AI (MCP) gateway is part of the app: every installer includes
`modeler3d-mcp` right beside 3D Modeler. Point your AI client at it while the
app is running and the assistant can use the same tools you click, live, in
the window you are looking at. The fidget spinner video at the top was built
this way.

```mermaid
flowchart LR
    you(["You: mouse and keyboard"]) --> cmd["One command layer"]
    ai(["AI assistant: MCP"]) --> gw["modeler3d-mcp gateway"] --> cmd
    cmd --> scene[("Your model, live in the window")]
```

There are four safety profiles (observe, model, files, full), and file access
is limited to folders you approve. Setup and the full tool list are in
[`docs/mcp.md`](docs/mcp.md).

## Build from source

<details>
<summary>Rust toolchain, C toolchain and libclang</summary>

You need the Rust toolchain pinned in `rust-toolchain.toml`, a C toolchain and
libclang. See [`docs/building.md`](docs/building.md).

```
cargo build --release
target/release/modeler3d-app
```

On Windows, run `scripts/Initialize-BuildClang.ps1` in the same PowerShell
session before Cargo. Run the tests with `cargo test --workspace`.

</details>

## Status

Version 1.0 is out. Known limits:

- **Repair works best on Windows.** On Windows, Repair uses the advanced
  3D-printing repair engine built into Windows itself. Mac and Linux don't
  have that engine, so 3D Modeler uses its own repair there. It fixes common
  problems such as holes and flipped faces well, but on badly damaged models
  it may not be as thorough as the Windows engine.
- Very dense models (hundreds of thousands of faces) can still make the window
  pause for a few seconds during a boolean operation.

Current work is tracked in [`docs/STATUS.md`](docs/STATUS.md) and changes in
[`CHANGELOG.md`](CHANGELOG.md).

## Contributing

Bug reports are very welcome. Because the product stands on extreme
simplicity, new features and look changes are considered one at a time. Read
[`CONTRIBUTING.md`](CONTRIBUTING.md) first. Security problems: see
[`SECURITY.md`](SECURITY.md).

## Support

3D Modeler is free and always will be. If it's useful to you, you can help
keep the updates and new features coming by
[supporting it on Ko-fi](https://ko-fi.com/scotttheis). Thank you!

<a href="https://ko-fi.com/scotttheis"><img src="https://img.shields.io/badge/Support%20on-Ko--fi-FF5E5B?style=for-the-badge&logo=ko-fi&logoColor=white" alt="Support on Ko-fi"></a>

If you like it, a ⭐ on GitHub helps other people find it.

## Licence

Licensed under the Apache License, Version 2.0. See [`LICENSE`](LICENSE) and
[`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md).

## Disclaimer

3D Modeler is an independent project. It is not affiliated with, endorsed by
or sponsored by Microsoft. 3D Builder is a trademark of Microsoft Corporation.

# Linux and Mac test packages

Owner request (2026-11-16): two packages, one for the owner's Linux box and
one for the owner's Mac, to install and try before the public GitHub release
([`github-upload.md`](github-upload.md)). The app has only ever run on
Windows, so expect to fix build and runtime problems along the way. The
Windows MSI is separate: [`packaging.md`](packaging.md).

A session asked to "package for Linux and Mac" does the whole job itself.
Do not hand the owner commands to run. Stop and ask only at the points marked
**Ask**.

## What to make

| | Linux | Mac |
|---|---|---|
| File | `3D-Modeler-<version>-x86_64.AppImage` (one file, runs on most distributions; no install needed) | `3D-Modeler-<version>.dmg` holding `3D Modeler.app` (drag to Applications) |
| CPU | x86_64 | Universal (Apple Silicon and Intel) via `lipo` |
| Built on | WSL Ubuntu 24.04 on this PC (`wsl -d Ubuntu-24.04`) | A Mac: GitHub's macOS build machines, or the owner's Mac |
| Icon | `crates/modeler3d-app/assets/original/Modeler3D.png` | Same picture as `.icns` |
| Output | `E:\3D-Modeler-RC\` next to the MSI | same |

Both use the version in `Cargo.toml` and the next RC number from the "Test
releases" line in `docs/STATUS.md`. Include the licence notices staged for
Windows (`packaging/licenses/`). Name shown to people: **3D Modeler**;
program file `Modeler3D` (see the "Name" section of `AGENTS.md`).

## Steps

1. **Linux build.** In WSL Ubuntu, install the build needs (Rust toolchain
   from `rust-toolchain.toml`, `clang`/`libclang-dev`, `cmake`, `pkg-config`,
   and the window/GPU libraries egui and wgpu need: X11/Wayland, xkbcommon,
   Vulkan loader). Build from a copy of the tree inside the WSL home folder,
   not from `/mnt/e` (slow, and keeps Linux build output off `E:`).
   `cargo test --workspace --locked`, then a release build. Fix Windows-only
   assumptions in the code behind the platform adapters (`modeler3d-platform`),
   one commit per fix, with tests.
2. **Linux run check.** Start it under WSLg, run `--smoke`, open a project,
   insert a cube, save, reopen; check settings land in `~/.config` or
   `~/.local/share` (not a Windows path) and the MCP gateway starts. Note
   anything WSL cannot show (GPU driver, real desktop menus) as not checked.
3. **Linux package.** Build the AppImage (`linuxdeploy` or `appimagetool`)
   with a `.desktop` file and the icon. Run it once from the AppImage.
4. **Mac build.** A Mac is required. **Ask** the owner which route:
   - **GitHub (recommended):** a workflow on GitHub's macOS machines builds
     both CPU types, joins them, makes the `.app` and `.dmg`, and keeps the
     file as a download. Needs the project on GitHub first; private is fine
     for this, and pushing needs the owner's yes ([`github-upload.md`](github-upload.md)).
   - **The owner's Mac:** a Claude Code session on the Mac runs the same
     steps there.
   The app is not signed (an Apple developer account costs $99 a year;
   owner has not decided). Tell the owner the first open shows a warning and
   is allowed once in System Settings → Privacy & Security → Open Anyway.
5. **Gate.** Full gate on Windows still passes after any code fix.
6. **Hand-off.** Update the "Test releases" line in `docs/STATUS.md`, commit,
   make a `G:\` snapshot, and give the owner a short plain message: where
   each file is, how to copy it to the Linux box and the Mac, how to open it,
   and what to try (open, insert, move, save, reopen, MCP if they use it).

## Known risks

- Never built on Linux or macOS. Build-time QHull/bindgen, font loading,
  file dialogs, the slicer finder and the MCP bridge paths are the likely
  breaks.
- Resource routing (AGENTS.md) has only been measured on the Windows Quadro
  host; WSL and Mac use different GPU paths. Record what was seen; do not
  claim a pass.
- macOS wants `~/Library/Application Support` for settings and does not let
  an unsigned app open without the owner's one-time OK.

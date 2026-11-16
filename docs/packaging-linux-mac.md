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
| File | `3D-Modeler-<version>-linux-x86_64.tar.gz` (unpack, run `Modeler3D`; needs no downloaded tools) and, once the owner allows the `appimagetool` download, `3D-Modeler-<version>-x86_64.AppImage` (one file) | `3D-Modeler-<version>.dmg` holding `3D Modeler.app` (drag to Applications) |
| CPU | x86_64 | Universal (Apple Silicon and Intel) via `lipo` |
| Built on | WSL Ubuntu 24.04 on this PC (`wsl -d Ubuntu-24.04`) | The owner's Mac, from the USB card (owner choice 2026-11-16; see "Mac package from the USB") |
| Script | `packaging/linux/build-tarball.sh`, `packaging/linux/build-appimage.sh` | `packaging/macos/build-dmg.sh` |
| Runs on | glibc 2.39 or newer (Ubuntu 24.04, Fedora 40, Debian 13), X11 or Wayland | macOS 11 or newer |
| Icon | `crates/modeler3d-app/assets/original/Modeler3D.png` | Same picture as `.icns` |
| Output | `E:\3D-Modeler-RC\` next to the MSI, and the USB card | the USB card, then `E:\3D-Modeler-RC\` |

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
   WSL has no hardware graphics adapter, so the viewport runs on the CPU
   route (llvmpipe): grainy floor, floor numbers drawn over objects, about
   300 ms a frame. That is the software route, not the Linux build.
3. **Linux package.** `packaging/linux/build-tarball.sh <version> ~/out`,
   then unpack it under `/tmp`, move `~/3D-Modeler` aside (so the program
   cannot fall back to the source tree's pictures) and repeat step 2 from the
   unpacked folder. The AppImage (`build-appimage.sh`) needs `appimagetool`
   downloaded from GitHub; **Ask** the owner before downloading it.
4. **Mac build.** A Mac is required. The owner chose the USB route
   (2026-11-16): see "Mac package from the USB" below. The two options were:
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

## Mac package from the USB

The owner carries the micro-SD card (`L:` on this PC, label `16GBMICROSD`,
FAT32) to the Mac and asks a Claude Code session there to package it.
Everything that session needs is in `3D-Modeler-packaging/` on the card:

| On the card | What it is |
|---|---|
| `3D-Modeler/` | Git clone of `rebuild/work`: source, docs, this file |
| `linux/` | The Linux package, for the owner to copy to the Linux box |
| `mac/` | Empty until the `.dmg` is built into it |

On the Mac the card mounts at `/Volumes/16GBMICROSD` (check `ls /Volumes`).

1. Do not build on the card (FAT32: slow, no executable bits). Clone it:
   `git clone -b rebuild/work /Volumes/16GBMICROSD/3D-Modeler-packaging/3D-Modeler ~/3D-Modeler`
   and work there; read its `AGENTS.md` and `docs/STATUS.md`.
2. Tools: Xcode Command Line Tools (`xcode-select -p`; if missing,
   `xcode-select --install` opens a window the owner clicks through:
   **Ask**) and rustup from rustup.rs (the toolchain in
   `rust-toolchain.toml` installs itself). QHull's bindgen needs libclang;
   if it is not found,
   `export LIBCLANG_PATH=/Library/Developer/CommandLineTools/usr/lib`.
3. `cargo test --workspace --locked`, `cargo clippy --all-targets --locked
   -- -D warnings`, `cargo fmt --all -- --check`. Fix Mac-only breaks behind
   the platform adapters, one commit per fix, with tests.
4. Run check as in Linux step 2: start `target/release/modeler3d-app`,
   insert, save, reopen; settings must land in
   `~/Library/Application Support/Modeler3D`. Drive it through
   `target/release/r4_mcp` with `MODELER3D_AI_APPROVED_ROOTS` set for both the
   app and the gateway; saves and opens are queued, so wait on
   `operation_status`.
5. `packaging/macos/build-dmg.sh <version> /Volumes/16GBMICROSD/3D-Modeler-packaging/mac`
   with the version of the Linux package of the same RC (the "Test releases"
   line in `docs/STATUS.md`). The script was written on Windows and has not
   run on a Mac; fixing it is part of the job. Open the `.dmg`, drag the app
   to Applications, start it once.
6. Bring the work back: update the "Test releases" line in
   `docs/STATUS.md`, commit, then
   `git push /Volumes/16GBMICROSD/3D-Modeler-packaging/3D-Modeler HEAD:mac/work`.
   A later Windows session fetches `mac/work` from the card, fast-forwards
   `rebuild/work`, runs the full gate, copies the `.dmg` to
   `E:\3D-Modeler-RC\` and makes the `G:\` snapshot.
7. Tell the owner in a short plain message: the app is in Applications; the
   first open shows a warning because it is not signed, allowed once in
   System Settings, Privacy & Security, Open Anyway.

## Known risks

- Never built on Linux or macOS. Build-time QHull/bindgen, font loading,
  file dialogs, the slicer finder and the MCP bridge paths are the likely
  breaks.
- Resource routing (AGENTS.md) has only been measured on the Windows Quadro
  host; WSL and Mac use different GPU paths. Record what was seen; do not
  claim a pass.
- macOS wants `~/Library/Application Support` for settings and does not let
  an unsigned app open without the owner's one-time OK.

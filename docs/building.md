# Native build prerequisites

Follow `E:\RustDeveloper.md`, `AGENTS.md`, and the pinned Rust toolchain.
QHull placement uses a static C dependency and bindgen at build time. Windows
requires the existing MSVC/Windows SDK plus libclang; Linux/macOS require the
host C toolchain and libclang. There is no Python or libclang runtime dependency.

## Active workspace

The active Rust source/build/run copy is `E:\3D-Modeler`. Run build and
launch commands from that workspace; the Release desktop is
`E:\3D-Modeler\target\release\modeler3d-app.exe`. `G:\` is the primary
backup destination. G: snapshots and occasional USB copies are recovery copies,
not working trees. `L:\Builder3D` is the separate Python reference/shipping
build.

On this Windows machine, initialize the verified project-local build tool in
the same PowerShell session before Cargo:

```powershell
.\scripts\Initialize-BuildClang.ps1
cargo test --workspace --locked
```

The helper verifies a pinned 26.4 MB Windows libclang 18.1.1 wheel and DLL,
extracts only the DLL under ignored `target/build-tools/`, and sets
`LIBCLANG_PATH` for the current process. It installs nothing globally and does
not require Python. An existing downloaded wheel can be supplied with
`-WheelPath`. A host LLVM installation can instead supply `LIBCLANG_PATH`
directly; other hosts should use their normal LLVM installation.

QHull and its wrapper are pinned to upstream revision
`62e810ccf93b458470f03cae6ef4df26023d2260`. This revision removes the released
wrapper's diagnostic pointer print. Placement uses the safe point-ID API,
bounded three-dimensional input, and no temporary-file capture. The native
kernel's own license and wrapper MIT license are included in staged packages
under `licenses/`; see `packaging/licenses/QHULL-NOTICE.txt`.

To package a Windows installer (MSI) for testers, see
[`packaging.md`](packaging.md).

Compatibility and parity probes ran on Windows x64/MSVC. Real Linux/macOS
builds remain part of the existing final host gate; no host pass is inferred.

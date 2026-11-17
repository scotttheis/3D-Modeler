# Packaging a Windows test release (MSI)

How to turn the current source into an installable `3D-Modeler-RCn.msi` that
someone else can run on their PC. The owner only asks for "a release
candidate" (or similar); the session does every step below itself and reports
the MSI path. The last release number is on the "Test releases" line in
[`STATUS.md`](STATUS.md).

## Steps for the session

1. Read the "Test releases" line in `STATUS.md`; the new candidate is the
   next number (after RC1 / 0.1.1 comes RC2 / 0.1.2).
2. Run the one command below. Fix and rerun on any failure.
3. Update the "Test releases" line in `STATUS.md` (number, version, date,
   size, verify result), commit on `rebuild/work`.
4. Give the owner the MSI path and the "person installing it" section below.

## One command

From PowerShell (any directory):

```powershell
E:\3D-Modeler\packaging\msi\Build-Msi.ps1 -Version 0.1.2 -Name 3D-Modeler-RC2 -Build -Verify
```

- `-Build` stops a running 3D Modeler (it locks the release exe; the owner
  allows this), runs `scripts\Initialize-BuildClang.ps1`, then
  `cargo build --release -p modeler3d-app --locked` (~4 min, fat LTO).
- Then it stages the app, runs `--smoke` on the staged copy and builds the MSI
  into `E:\3D-Modeler-RC\` (outside the repo, so `E:` and `target\` stay clean).
- `-Verify` silently installs the MSI on this PC, checks files and shortcuts,
  runs `--smoke` and opens the real window for 8 s, then silently uninstalls
  and checks nothing is left behind. It fails loudly at the first problem.

Leave out `-Build` to package the release exe that is already built.

## Versions

The script embeds this version into the executable and also emits
`3D-Modeler-<version>-windows-x86_64.msi` for the update feed. Before the first
public build, configure the feed and add `-PublicRelease`; see
[`updates.md`](updates.md). Keep the existing UpgradeCode unchanged.

Release candidate *n* is version `0.1.n` (RC1 = 0.1.1, RC2 = 0.1.2, ...).
**Bump the third number every time.** Windows Installer ignores a fourth
field, so an MSI with the same three numbers will not replace the old one.
A higher version upgrades the previous install in place (`MajorUpgrade`);
a lower one is refused.

Never change the `UpgradeCode` in `packaging/msi/Modeler3D.wxs`. It is what
ties every candidate together as "the same product".

## What the MSI does

| Item | Choice |
|---|---|
| Scope | Per-user. No administrator rights, no UAC prompt |
| Install folder | `%LOCALAPPDATA%\Programs\Modeler3D` |
| App settings | `%LOCALAPPDATA%\Modeler3D` (written by the app, kept on uninstall). Do not install into this folder |
| Shortcuts | Start menu and desktop, both "3D Modeler", with the app icon |
| Wizard | WixUI_Minimal: one screen with the notice from `packaging/msi/License.rtf`, then progress, then a "Launch 3D Modeler" checkbox |
| Contents | `Modeler3D.exe`, `assets\original\` (artwork the app loads at startup), `licenses\`, and the VC++ runtime DLLs `msvcp140.dll`, `vcruntime140.dll`, `vcruntime140_1.dll` copied app-local so a PC without the VC++ redistributable still runs it |
| Uninstall | Settings > Apps > 3D Modeler > Uninstall, or `msiexec /x 3D-Modeler-RCn.msi` |

The exe needs the `assets\original` folder beside it. On the dev PC it falls
back to the source tree, which hides a missing folder; on any other PC the
app would fail at startup. The staged `--smoke` and `-Verify` steps check the
packaged copy for this reason.

If the exe's DLL imports change (for example a new C++ dependency), recheck
them and add any new runtime DLL to the `.wxs`:

```powershell
& "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\*\bin\Hostx64\x64\dumpbin.exe" /dependents E:\3D-Modeler\target\release\modeler3d-app.exe
```

## One-time setup on a new build machine

1. Visual Studio 2022 with the C++ workload (provides MSVC, the Windows SDK
   and the `VC\Redist\MSVC\*\x64\Microsoft.VC143.CRT` runtime folder).
2. .NET SDK (for the `wix` tool).
3. WiX **5.0.2** and its two extensions:

   ```powershell
   dotnet tool install --global wix --version 5.0.2
   wix extension add -g WixToolset.UI.wixext/5.0.2
   wix extension add -g WixToolset.Util.wixext/5.0.2
   ```

   Use WiX 5, not 6 or 7. WiX 7 refuses to build until you accept the Open
   Source Maintenance Fee EULA. WiX 5 is plain MS-RL. The script checks for
   5.x and both extensions.

## The person installing it

Send them the `.msi` only. Tell them:

1. Double-click it. Windows shows "Windows protected your PC" because the
   file is not code-signed: click **More info**, then **Run anyway**.
2. Accept the notice, click Install, leave "Launch 3D Modeler" ticked.
3. Requirements: 64-bit Windows 10 or 11. No GPU is needed; the app falls
   back to its CPU renderer.

## Lessons already paid for

- **libclang:** the release build fails with "Unable to find libclang"
  unless `scripts\Initialize-BuildClang.ps1` was dot-sourced in the same
  PowerShell session. `-Build` does this.
- **Locked exe:** a running 3D Modeler stops Cargo replacing the release exe.
  `-Build` stops it.
- **Size budget:** `modeler3d-app.exe --package <dir>` enforces the S3 budget
  (40 MB installed, 15 MB archive) and currently fails at ~64 MB, mostly the
  1254 px action-icon PNGs (~32 MB). The MSI script therefore stages the same
  layout itself. When the icons are reduced (an owner decision in S3),
  `--package` will pass again; the MSI is ~50 MB until then.
- **`--package` artwork bug:** before commit `4c99d5c` it could not copy the
  artwork at all (swapped arguments). Covered by the test
  `staged_package_artwork_validates_beside_the_executable`.
- **No console window (OF-28):** the release exe is a windowed program on
  Windows. PowerShell does not wait for a windowed exe unless its output is
  piped, so script calls such as `--smoke` end in `| Write-Host` to get a real
  `$LASTEXITCODE`. Diagnostics always go to
  `%LOCALAPPDATA%\Modeler3D\logs\modeler3d.log`; `Modeler3D.exe --console`
  also shows them live in a console window.
- **WiX source rules:** no `--` inside XML comments; keep `License.rtf`
  plain ASCII (code page 1252); one versioned file per `<Component>`; do not
  set `ARPNOMODIFY` (WixUI_Minimal already does); `wix build` has no `-sice`
  switch in WiX 5.
- **Per-user validation warnings:** WiX 5 `wix build` does not run ICE
  validation, so the usual per-user ICE38/ICE64 complaints do not appear.
  Shortcut components carry HKCU registry key paths so they install and
  uninstall cleanly.

## Not covered yet

- Code signing (owner aware; SmartScreen warning is accepted for testing).
- A clean-PC test. `-Verify` runs on the dev PC, which already has the VC++
  runtime and the source tree. The first friend install is the real check.
- Linux and macOS packages: see [`packaging-linux-mac.md`](packaging-linux-mac.md).
- The ship-tier gate (`scripts\Invoke-Gate.ps1 -Tier ship`) still reports
  package staging as "not scripted"; it does not call this script.

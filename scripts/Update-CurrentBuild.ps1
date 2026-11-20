<#
One current build (owner rule, 2026-11-19).

The only build the owner ever runs is target\release\modeler3d-app.exe, and it
must always match the source. This script makes that mechanical:

  -Check   report CURRENT or STALE, change nothing (exit 0 current, 1 stale)
  (none)   if STALE: stop a running 3D Modeler, move the old exe into
           G:\3D-Modeler-old-builds\<timestamp>_OLD-BUILD\, rebuild Release,
           then confirm CURRENT. If already CURRENT: do nothing.
  -Launch  after the above, start the current build.

Stale = the exe is older than the newest file under crates\, Cargo.toml or
Cargo.lock. Run from E:\3D-Modeler.
#>
param([switch]$Check, [switch]$Launch)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$exe = Join-Path $root 'target\release\modeler3d-app.exe'
$backups = 'G:\3D-Modeler-old-builds'

function Get-NewestSource {
    $items = @(Get-ChildItem (Join-Path $root 'crates') -Recurse -File |
        Where-Object { $_.FullName -notlike '*\target\*' })
    $items += Get-Item (Join-Path $root 'Cargo.toml'), (Join-Path $root 'Cargo.lock')
    $items | Sort-Object LastWriteTime -Descending | Select-Object -First 1
}
function Test-Current {
    if (-not (Test-Path $exe)) { return $false }
    (Get-Item $exe).LastWriteTime -ge (Get-NewestSource).LastWriteTime
}

if ($Check) {
    if (Test-Current) { 'CURRENT'; exit 0 } else { 'STALE'; exit 1 }
}

if (-not (Test-Current)) {
    Get-Process modeler3d-app -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep 1
    if (Test-Path $exe) {
        $stamp = (Get-Item $exe).LastWriteTime.ToString('yyyyMMdd-HHmmss')
        $dest = Join-Path $backups "${stamp}_OLD-BUILD"
        New-Item -ItemType Directory -Force $dest | Out-Null
        Move-Item $exe (Join-Path $dest 'modeler3d-app.OLD-BUILD.exe')
        "Old build moved to $dest"
    }
    & (Join-Path $PSScriptRoot 'Initialize-BuildClang.ps1')
    Push-Location $root
    try { cargo build --release -p modeler3d-app --locked } finally { Pop-Location }
    if (-not (Test-Current)) { throw 'Build finished but the exe is still older than the source.' }
}
'CURRENT: ' + $exe
if ($Launch) { Start-Process $exe -WorkingDirectory $root }

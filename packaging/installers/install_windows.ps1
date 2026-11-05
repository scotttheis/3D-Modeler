param(
    [string] $InstallRoot = (Join-Path $env:LOCALAPPDATA 'Builder3D')
)
$ErrorActionPreference = 'Stop'

$source_root = Split-Path -Parent $PSScriptRoot
$app_exe = Join-Path $source_root 'Builder3D.exe'
if (-not (Test-Path -LiteralPath $app_exe -PathType Leaf)) {
    throw 'Builder3D.exe is missing from the package root.'
}
if ([string]::IsNullOrWhiteSpace($InstallRoot) -or $InstallRoot.Length -gt 4096) {
    throw 'InstallRoot is empty or exceeds the bounded path length.'
}
New-Item -ItemType Directory -Path $InstallRoot -Force | Out-Null
Get-ChildItem -LiteralPath $source_root -Force |
    Where-Object { $_.Name -ne 'installers' } |
    ForEach-Object {
        Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $InstallRoot $_.Name) -Recurse -Force
    }
Write-Output "Builder3D installed to $InstallRoot"

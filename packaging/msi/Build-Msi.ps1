param(
    [Parameter(Mandatory = $true)] [string] $Version,   # e.g. 0.1.1 for RC1, 0.1.2 for RC2
    [string] $Name = "Builder3D-RC",
    [string] $OutDir = "E:\Builder3D-RC"
)
# Builds a per-user Builder3D MSI from the current release exe.
# Needs: `cargo build --release -p builder3d-app` done first, and WiX 5
# (`dotnet tool install --global wix --version 5.0.2` plus the
# WixToolset.UI.wixext and WixToolset.Util.wixext 5.0.2 extensions).
$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path
$exe = Join-Path $projectRoot 'target\release\builder3d-app.exe'
if (-not (Test-Path -LiteralPath $exe)) { throw "Release exe not found: $exe" }

$crt = Get-ChildItem 'C:\Program Files\Microsoft Visual Studio\*\*\VC\Redist\MSVC\*\x64\Microsoft.VC143.CRT' -Directory |
    Sort-Object FullName -Descending | Select-Object -First 1
if (-not $crt) { throw 'MSVC x64 redistributable runtime folder not found.' }

$stage = Join-Path $OutDir "stage-$Version"
if (Test-Path -LiteralPath $stage) { Remove-Item -LiteralPath $stage -Recurse -Force }
# Same layout as `builder3d-app.exe --package`, staged directly so a test
# candidate can be built before the S3 40 MB installed-size budget is met
# (the full-size action icons alone are ~32 MB).
New-Item -ItemType Directory -Path $stage -Force | Out-Null
Copy-Item -LiteralPath $exe -Destination (Join-Path $stage 'Builder3D.exe')
New-Item -ItemType Directory -Path (Join-Path $stage 'assets') -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $projectRoot 'crates\builder3d-app\assets\original') `
    -Destination (Join-Path $stage 'assets\original') -Recurse
Copy-Item -LiteralPath (Join-Path $projectRoot 'packaging\licenses') `
    -Destination (Join-Path $stage 'licenses') -Recurse
& (Join-Path $stage 'Builder3D.exe') --smoke
if ($LASTEXITCODE -ne 0) { throw "Staged app failed its smoke check ($LASTEXITCODE)." }

$msi = Join-Path $OutDir "$Name.msi"
& wix build (Join-Path $PSScriptRoot 'Builder3D.wxs') -arch x64 `
    -ext WixToolset.UI.wixext -ext WixToolset.Util.wixext `
    -d "Version=$Version" -d "StageDir=$stage" -d "CrtDir=$($crt.FullName)" `
    -o $msi
if ($LASTEXITCODE -ne 0) { throw "wix build failed ($LASTEXITCODE)." }
Remove-Item -LiteralPath $stage -Recurse -Force
Remove-Item -LiteralPath (Join-Path $OutDir "$Name.wixpdb") -ErrorAction SilentlyContinue
Get-Item -LiteralPath $msi | Select-Object FullName, Length

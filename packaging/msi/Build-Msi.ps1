param(
    [Parameter(Mandatory = $true)] [string] $Version,   # 0.1.1 = RC1, 0.1.2 = RC2, ...
    [string] $Name = "3D-Modeler-RC",
    [string] $OutDir = "E:\3D-Modeler-RC",
    [switch] $Build,    # stop a running 3D Modeler, set up libclang, cargo build --release
    [switch] $Verify,    # silent install, smoke + launch check, silent uninstall
    [switch] $PublicRelease # refuse an unconfigured updater
)
# Builds a per-user 3D Modeler MSI from the release exe. See docs/packaging.md.
# One-shot:  .\packaging\msi\Build-Msi.ps1 -Version 0.1.2 -Name 3D-Modeler-RC2 -Build -Verify
$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..')).Path
$exe = Join-Path $projectRoot 'target\release\modeler3d-app.exe'
$mcpExe = Join-Path $projectRoot 'target\release\modeler3d-mcp.exe'

if ($Version -notmatch '^\d+\.\d+\.\d+$') { throw 'Version must be major.minor.patch, e.g. 0.1.2.' }
if (-not (Get-Command wix -ErrorAction SilentlyContinue)) {
    throw 'WiX not found. Install: dotnet tool install --global wix --version 5.0.2 (see docs/packaging.md).'
}
$wixVersion = (& wix --version)
if ($wixVersion -notmatch '^5\.') { throw "WiX 5.x required, found $wixVersion (see docs/packaging.md)." }
$extensions = (& wix extension list -g) -join "`n"
foreach ($ext in 'WixToolset.UI.wixext', 'WixToolset.Util.wixext') {
    if ($extensions -notmatch [regex]::Escape($ext)) {
        throw "Missing WiX extension $ext. Install: wix extension add -g $ext/5.0.2"
    }
}

if ($Build) {
    # The running app locks target\release\modeler3d-app.exe; the owner allows stopping it.
    Get-Process modeler3d-app, Modeler3D, builder3d-app, Builder3D -ErrorAction SilentlyContinue | Stop-Process -Force
    Push-Location $projectRoot
    try {
        $previousReleaseVersion = $env:MODELER3D_RELEASE_VERSION
        $env:MODELER3D_RELEASE_VERSION = $Version
        . (Join-Path $projectRoot 'scripts\Initialize-BuildClang.ps1') | Out-Null
        cmd /c "cargo build --release -p modeler3d-app -p modeler3d-mcp --locked 2>&1"
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed ($LASTEXITCODE)." }
    } finally { $env:MODELER3D_RELEASE_VERSION = $previousReleaseVersion; Pop-Location }
}
if (-not (Test-Path -LiteralPath $exe)) { throw "Release exe not found: $exe (use -Build)." }
$builtVersion = (& $exe --version | Out-String).Trim()
if ($builtVersion -ne "3D Modeler $Version") { throw "Binary version does not match $Version; rebuild with -Build." }
if ($PublicRelease) {
    & $exe --require-update-feed | Write-Host
    if ($LASTEXITCODE -ne 0) { throw 'Configure packaging/update-feed.json, then rebuild.' }
}

$crt = Get-ChildItem 'C:\Program Files\Microsoft Visual Studio\*\*\VC\Redist\MSVC\*\x64\Microsoft.VC143.CRT' -Directory |
    Sort-Object FullName -Descending | Select-Object -First 1
if (-not $crt) { throw 'MSVC x64 redistributable runtime folder not found.' }

New-Item -ItemType Directory -Path $OutDir -Force | Out-Null
$stage = Join-Path $OutDir "stage-$Version"
if (Test-Path -LiteralPath $stage) { Remove-Item -LiteralPath $stage -Recurse -Force }
# Same layout as `modeler3d-app.exe --package`, staged directly so a test
# candidate can be built before the S3 40 MB installed-size budget is met
# (the full-size action icons alone are ~32 MB).
New-Item -ItemType Directory -Path $stage -Force | Out-Null
Copy-Item -LiteralPath $exe -Destination (Join-Path $stage 'Modeler3D.exe')
if (-not (Test-Path -LiteralPath $mcpExe)) { throw "MCP gateway not found: $mcpExe (build -p modeler3d-mcp)." }
Copy-Item -LiteralPath $mcpExe -Destination (Join-Path $stage 'modeler3d-mcp.exe')
New-Item -ItemType Directory -Path (Join-Path $stage 'assets') -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $projectRoot 'crates\modeler3d-app\assets\original') `
    -Destination (Join-Path $stage 'assets\original') -Recurse
# Release package icons are 128 px; the full-size art stays in the source tree.
& (Join-Path $stage 'Modeler3D.exe') --shrink-assets (Join-Path $stage 'assets\original') | Write-Host
if ($LASTEXITCODE -ne 0) { throw "Icon shrinking failed ($LASTEXITCODE)." }
Copy-Item -LiteralPath (Join-Path $projectRoot 'packaging\licenses') `
    -Destination (Join-Path $stage 'licenses') -Recurse
# Piped so PowerShell waits for the windowed exe and sets $LASTEXITCODE (OF-28).
& (Join-Path $stage 'Modeler3D.exe') --smoke | Write-Host
if ($LASTEXITCODE -ne 0) { throw "Staged app failed its smoke check ($LASTEXITCODE)." }

$msi = Join-Path $OutDir "$Name.msi"
& wix build (Join-Path $PSScriptRoot 'Modeler3D.wxs') -arch x64 `
    -ext WixToolset.UI.wixext -ext WixToolset.Util.wixext `
    -d "Version=$Version" -d "StageDir=$stage" -d "CrtDir=$($crt.FullName)" `
    -o $msi
if ($LASTEXITCODE -ne 0) { throw "wix build failed ($LASTEXITCODE)." }
$updateMsi = Join-Path $OutDir "3D-Modeler-$Version-windows-x86_64.msi"
if ([IO.Path]::GetFullPath($msi) -ne [IO.Path]::GetFullPath($updateMsi)) {
    Copy-Item -LiteralPath $msi -Destination $updateMsi -Force
}
Remove-Item -LiteralPath $stage -Recurse -Force
Remove-Item -LiteralPath (Join-Path $OutDir "$Name.wixpdb") -ErrorAction SilentlyContinue
Get-Item -LiteralPath $msi | Select-Object FullName, Length

if ($Verify) {
    $installDir = Join-Path $env:LOCALAPPDATA 'Programs\Modeler3D'
    $startMenu = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\3D Modeler.lnk'
    $desktop = Join-Path ([Environment]::GetFolderPath('Desktop')) '3D Modeler.lnk'
    $log = Join-Path $OutDir "$Name-install.log"
    $p = Start-Process msiexec -ArgumentList "/i `"$msi`" /qn /l*v `"$log`"" -Wait -PassThru
    if ($p.ExitCode -ne 0) { throw "Silent install failed ($($p.ExitCode)); see $log." }
    try {
        foreach ($path in "$installDir\Modeler3D.exe", "$installDir\vcruntime140.dll",
                          "$installDir\assets\original\Modeler3D.ico", $startMenu, $desktop) {
            if (-not (Test-Path -LiteralPath $path)) { throw "Installed item missing: $path" }
        }
        & "$installDir\Modeler3D.exe" --smoke | Write-Host
        if ($LASTEXITCODE -ne 0) { throw "Installed app failed its smoke check ($LASTEXITCODE)." }
        $app = Start-Process "$installDir\Modeler3D.exe" -PassThru
        Start-Sleep -Seconds 8
        $app.Refresh()
        if ($app.HasExited) { throw "Installed app exited during launch ($($app.ExitCode))." }
        Stop-Process -Id $app.Id -Force
        Write-Output "Verify: install, shortcuts, smoke and launch OK."
    } finally {
        $u = Start-Process msiexec -ArgumentList "/x `"$msi`" /qn" -Wait -PassThru
        if ($u.ExitCode -ne 0) { throw "Silent uninstall failed ($($u.ExitCode))." }
    }
    foreach ($path in $installDir, $startMenu, $desktop) {
        if (Test-Path -LiteralPath $path) { throw "Uninstall left behind: $path" }
    }
    Remove-Item -LiteralPath $log -ErrorAction SilentlyContinue
    Write-Output "Verify: uninstall clean."
}

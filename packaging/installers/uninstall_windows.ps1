param(
    [string] $InstallRoot = (Join-Path $env:LOCALAPPDATA 'Builder3D')
)
$ErrorActionPreference = 'Stop'

$expected_root = [IO.Path]::GetFullPath((Join-Path $env:LOCALAPPDATA 'Builder3D'))
$actual_root = [IO.Path]::GetFullPath($InstallRoot)
if ($actual_root -ne $expected_root) {
    throw 'Refusing to remove a path other than the private Builder3D install root.'
}
if (Test-Path -LiteralPath $actual_root) {
    Remove-Item -LiteralPath $actual_root -Recurse -Force
}
Write-Output "Builder3D removed from $actual_root"

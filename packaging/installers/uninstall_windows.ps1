param(
    [string] $InstallRoot = (Join-Path $env:LOCALAPPDATA 'Modeler3D')
)
$ErrorActionPreference = 'Stop'

$expected_root = [IO.Path]::GetFullPath((Join-Path $env:LOCALAPPDATA 'Modeler3D'))
$actual_root = [IO.Path]::GetFullPath($InstallRoot)
if ($actual_root -ne $expected_root) {
    throw 'Refusing to remove a path other than the private 3D Modeler install root.'
}
if (Test-Path -LiteralPath $actual_root) {
    Remove-Item -LiteralPath $actual_root -Recurse -Force
}
Write-Output "3D Modeler removed from $actual_root"

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
# Remove the file types install_windows.ps1 registered, and nothing else.
$classes = 'HKCU:\Software\Classes'
foreach ($key in @("$classes\Modeler3D.Project", "$classes\Modeler3D.Model",
        "$classes\Applications\Modeler3D.exe", 'HKCU:\Software\Modeler3D\Capabilities')) {
    if (Test-Path -LiteralPath $key) { Remove-Item -LiteralPath $key -Recurse -Force }
}
if ((Get-ItemProperty -LiteralPath "$classes\.3dp" -ErrorAction SilentlyContinue).'(default)' -eq 'Modeler3D.Project') {
    Set-Item -LiteralPath "$classes\.3dp" -Value ''
}
foreach ($extension in @('.3dp', '.stl', '.obj', '.ply', '.3mf', '.gltf', '.glb', '.wrl')) {
    Remove-ItemProperty -LiteralPath "$classes\$extension\OpenWithProgids" -Name 'Modeler3D.Project', 'Modeler3D.Model' -ErrorAction SilentlyContinue
}
Remove-ItemProperty -LiteralPath 'HKCU:\Software\RegisteredApplications' -Name '3D Modeler' -ErrorAction SilentlyContinue

Write-Output "3D Modeler removed from $actual_root"

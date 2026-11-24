param(
    [string] $InstallRoot = (Join-Path $env:LOCALAPPDATA 'Modeler3D')
)
$ErrorActionPreference = 'Stop'

$source_root = Split-Path -Parent $PSScriptRoot
$app_exe = Join-Path $source_root 'Modeler3D.exe'
if (-not (Test-Path -LiteralPath $app_exe -PathType Leaf)) {
    throw 'Modeler3D.exe is missing from the package root.'
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
# Windows file types, per user. Keep in step with packaging/msi/Modeler3D.wxs.
# .3dp opens in 3D Modeler; model formats are offered in "Open with" and
# Settings > Default apps, where the user chooses the default.
$installed_exe = Join-Path $InstallRoot 'Modeler3D.exe'
$open_command = '"' + $installed_exe + '" "%1"'
$icon = '"' + $installed_exe + '",0'
$classes = 'HKCU:\Software\Classes'
$model_types = @('.stl', '.obj', '.ply', '.3mf', '.gltf', '.glb', '.wrl')
# New-Item -Force would empty an existing key (other apps' entries), so a key
# is only created when it is missing.
function New-RegistryKeyIfMissing([string] $Key) {
    if (-not (Test-Path -LiteralPath $Key)) { New-Item -Path $Key -Force | Out-Null }
}
function Set-RegistryDefault([string] $Key, [string] $Value) {
    New-RegistryKeyIfMissing $Key
    Set-Item -LiteralPath $Key -Value $Value
}
function Set-RegistryName([string] $Key, [string] $Name, [string] $Value) {
    New-RegistryKeyIfMissing $Key
    New-ItemProperty -LiteralPath $Key -Name $Name -Value $Value -PropertyType String -Force | Out-Null
}
foreach ($prog_id in @(@('Modeler3D.Project', '3D Modeler Project'), @('Modeler3D.Model', '3D Model'))) {
    Set-RegistryDefault "$classes\$($prog_id[0])" $prog_id[1]
    Set-RegistryDefault "$classes\$($prog_id[0])\DefaultIcon" $icon
    Set-RegistryDefault "$classes\$($prog_id[0])\shell\open\command" $open_command
}
Set-RegistryDefault "$classes\.3dp" 'Modeler3D.Project'
Set-RegistryName "$classes\.3dp\OpenWithProgids" 'Modeler3D.Project' ''
foreach ($extension in $model_types) {
    Set-RegistryName "$classes\$extension\OpenWithProgids" 'Modeler3D.Model' ''
}
$application = "$classes\Applications\Modeler3D.exe"
Set-RegistryName $application 'FriendlyAppName' '3D Modeler'
Set-RegistryDefault "$application\shell\open\command" $open_command
$capabilities = 'HKCU:\Software\Modeler3D\Capabilities'
Set-RegistryName $capabilities 'ApplicationName' '3D Modeler'
Set-RegistryName $capabilities 'ApplicationDescription' 'Build, repair and prepare 3D models for printing.'
Set-RegistryName $capabilities 'ApplicationIcon' $icon
Set-RegistryName "$capabilities\FileAssociations" '.3dp' 'Modeler3D.Project'
Set-RegistryName "$application\SupportedTypes" '.3dp' ''
foreach ($extension in $model_types) {
    Set-RegistryName "$capabilities\FileAssociations" $extension 'Modeler3D.Model'
    Set-RegistryName "$application\SupportedTypes" $extension ''
}
Set-RegistryName 'HKCU:\Software\RegisteredApplications' '3D Modeler' 'Software\Modeler3D\Capabilities'

Write-Output "3D Modeler installed to $InstallRoot"

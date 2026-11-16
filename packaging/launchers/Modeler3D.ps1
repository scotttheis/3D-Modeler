$ErrorActionPreference = 'Stop'
$app_exe = Join-Path -Path $PSScriptRoot -ChildPath 'Modeler3D.exe'
if (-not (Test-Path -LiteralPath $app_exe -PathType Leaf)) {
    throw 'Modeler3D.exe was not found beside this launcher.'
}
& $app_exe @args
exit $LASTEXITCODE

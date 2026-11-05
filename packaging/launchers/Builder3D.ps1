$ErrorActionPreference = 'Stop'
$app_exe = Join-Path -Path $PSScriptRoot -ChildPath 'Builder3D.exe'
if (-not (Test-Path -LiteralPath $app_exe -PathType Leaf)) {
    throw 'Builder3D.exe was not found beside this launcher.'
}
& $app_exe @args
exit $LASTEXITCODE

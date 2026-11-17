param([int]$TaskPid, [string]$Installer, [string]$Executable, [string]$Work)
$ErrorActionPreference = 'Stop'
$resultFile = Join-Path (Split-Path -Parent $Work) 'last-result.txt'
try {
    $running = Get-Process -Id $TaskPid -ErrorAction SilentlyContinue
    if ($running -and -not $running.WaitForExit(120000)) { throw 'Application did not close; update cancelled.' }
    $log = Join-Path $Work 'installer.log'
    $arguments = '/i "' + $Installer + '" /passive /norestart /l*v "' + $log + '"'
    $install = Start-Process -FilePath "$env:SystemRoot\System32\msiexec.exe" -ArgumentList $arguments -PassThru -Wait -WindowStyle Hidden
    if ($install.ExitCode -notin @(0,3010)) { throw "Installer returned $($install.ExitCode). Details: $log" }
    Set-Content -LiteralPath $resultFile -Encoding UTF8 -Value '3D Modeler was updated successfully.'
} catch {
    Set-Content -LiteralPath $resultFile -Encoding UTF8 -Value ('Update failed: ' + $_.Exception.Message)
}
if (Test-Path -LiteralPath $Executable) { Start-Process -FilePath $Executable -WindowStyle Normal }

$ErrorActionPreference = 'Stop'
$projectRoot = 'E:\3D-Modeler'
$evidenceRoot = Join-Path $projectRoot 'evidence\rebuild\rb-08-preferences-and-complete-shell\native'
. (Join-Path $projectRoot 'evidence\rebuild\rb-04-scene-and-precise-transforms\native-ui-helper.ps1')
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$runRoot = Join-Path $evidenceRoot ("python-live-baseline-{0}" -f $stamp)
if (Test-Path -LiteralPath $runRoot) { throw "Refusing to overwrite existing run: $runRoot" }
$settingsRoot = Join-Path $runRoot 'settings'
$runtimeRoot = Join-Path $runRoot 'runtime'
$capturesRoot = Join-Path $runRoot 'captures'
New-Item -ItemType Directory -Force -Path $settingsRoot,$runtimeRoot,$capturesRoot | Out-Null
$launcher = Join-Path $evidenceRoot 'python-reference-launcher.py'
$stdoutPath = Join-Path $runRoot 'stdout.txt'
$stderrPath = Join-Path $runRoot 'stderr.txt'
$oldLocalAppData = $env:LOCALAPPDATA
$oldSettings = $env:RB08_REFERENCE_SETTINGS_DIR
$oldBytecode = $env:PYTHONDONTWRITEBYTECODE
$env:LOCALAPPDATA = $runtimeRoot
$env:RB08_REFERENCE_SETTINGS_DIR = $settingsRoot
$env:PYTHONDONTWRITEBYTECODE = '1'
try {
    $process = Start-Process -FilePath 'L:\Builder3D\venv\Scripts\python.exe' -ArgumentList @('-B','-u',$launcher) -WorkingDirectory 'L:\Builder3D' -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath -PassThru
    $deadline = [DateTime]::UtcNow.AddSeconds(30)
    $ownerPid = 0
    $handle = [IntPtr]::Zero
    do {
        Start-Sleep -Milliseconds 250
        $matching = @(Get-CimInstance Win32_Process | Where-Object {
            $_.Name -eq 'python.exe' -and $_.CommandLine -like "*$launcher*"
        })
        foreach ($candidate in $matching) {
            $candidateHandle = [Rb03UiNative]::Find([uint32]$candidate.ProcessId)
            if ($candidateHandle -ne [IntPtr]::Zero) {
                $ownerPid = [int]$candidate.ProcessId
                $handle = $candidateHandle
                break
            }
        }
        if ($process.HasExited -and $ownerPid -eq 0) { throw "Python reference launcher exited with code $($process.ExitCode)" }
    } while ($ownerPid -eq 0 -and [DateTime]::UtcNow -lt $deadline)
    if ($ownerPid -eq 0) { throw 'Python reference target window did not become ready' }
    $script:rb03AppPid = $ownerPid
    $info = Get-Rb03WindowInfo $handle
    if ($info.Title -notlike '3D Modeler - *') { throw "Unexpected Python window title: $($info.Title)" }
    if ($info.ClientWidth -ne 1280 -or $info.ClientHeight -ne 840 -or $info.Dpi -ne 96) {
        throw "Unexpected reference baseline dimensions: $($info.ClientWidth)x$($info.ClientHeight), DPI $($info.Dpi)"
    }
    $relative = "evidence\rebuild\rb-08-preferences-and-complete-shell\native\python-live-baseline-$stamp\captures\python-dark-welcome-empty.png"
    $capture = Capture-Rb03App $relative
    $metadata = [pscustomobject]@{
        slice = 'RB-08 baseline'
        target = 'Python reference MainWindow'
        process_id = $ownerPid
        launcher_process_id = $process.Id
        title = $capture.Title
        theme = 'dark; first-run default'
        client_size = @($capture.ClientWidth,$capture.ClientHeight)
        dpi = $capture.Dpi
        settings_backend = 'isolated QSettings IniFormat via the production settings adapter'
        settings_path = $settingsRoot
        local_app_data = $runtimeRoot
        image = $relative.Replace('\','/')
        image_sha256 = $capture.Sha256
        source_sha256 = (Get-FileHash -LiteralPath 'L:\Builder3D\src\main_window.py' -Algorithm SHA256).Hash.ToLowerInvariant()
        captured_at = [DateTimeOffset]::Now.ToString('o')
    }
    $metadata | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $runRoot 'window.json') -Encoding UTF8
    $metadata | Format-List
    Write-Output "RUN_ROOT=$runRoot"
    Write-Output "WINDOW_PID=$ownerPid"
} finally {
    $env:LOCALAPPDATA = $oldLocalAppData
    $env:RB08_REFERENCE_SETTINGS_DIR = $oldSettings
    $env:PYTHONDONTWRITEBYTECODE = $oldBytecode
}

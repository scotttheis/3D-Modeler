param()

$ErrorActionPreference = 'Stop'
$evidenceRoot = (Resolve-Path $PSScriptRoot).Path
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
. (Join-Path $projectRoot 'evidence\rebuild\rb-04-scene-and-precise-transforms\native-ui-helper.ps1')

$fixture = Join-Path $evidenceRoot 'fixtures\slow-job-closed-cube-heavy.ply'
$executable = Join-Path $projectRoot 'target\debug\modeler3d-app.exe'
$journalPath = Join-Path $evidenceRoot 'rust-subdivide-cancel-attempt3-captures.ndjson'
$resultsPath = Join-Path $evidenceRoot 'rust-subdivide-cancel-attempt3-results.json'
$captureRows = [System.Collections.Generic.List[object]]::new()
$buildHash = (Get-FileHash -LiteralPath $executable -Algorithm SHA256).Hash.ToLowerInvariant()
$fixtureHash = (Get-FileHash -LiteralPath $fixture -Algorithm SHA256).Hash.ToLowerInvariant()
$script:rb03AppPid = 0
$localAppDataRoot = Join-Path $evidenceRoot 'runtime\subdivide-cancel-localappdata-light'
$oldLocalAppData = $env:LOCALAPPDATA

function Capture-Rb07Subdivision {
    param([string]$Action, [string]$FixtureLabel, [string]$InputRoute)

    $handle = Activate-Rb03App
    $info = Get-Rb03WindowInfo $handle
    if ($info.ClientWidth -ne 1280 -or $info.ClientHeight -ne 840 -or $info.Dpi -ne 96) {
        throw "Rejected native capture dimensions or DPI for $Action"
    }
    $relative = "evidence\rebuild\rb-07-appearance-jobs-and-print\captures\rust-rb07-light-job-subdivide-attempt2-$Action.png"
    $captured = Capture-Rb03App $relative
    $record = [pscustomobject]@{
        slice = 'RB-07'
        app = 'Rust native desktop'
        process_id = $script:rb03AppPid
        theme = 'light'
        action = $Action
        fixture = $FixtureLabel
        input_route = $InputRoute
        window_title = $captured.Title
        logical_client_size = @($captured.ClientWidth, $captured.ClientHeight)
        captured_size = @($captured.ClientWidth, $captured.ClientHeight)
        dpi = $captured.Dpi
        build_sha256 = $buildHash
        image = $relative.Replace('\', '/')
        image_sha256 = $captured.Sha256
        captured_at = [DateTimeOffset]::Now.ToString('o')
    }
    $captureRows.Add($record)
    ConvertTo-Json -InputObject $record -Depth 8 -Compress | Add-Content -LiteralPath $journalPath -Encoding UTF8
}

function Invoke-Rb07ClickFast {
    param([int]$X, [int]$Y)

    $handle = Activate-Rb03App
    $info = Get-Rb03WindowInfo $handle
    if ($X -lt 0 -or $X -ge $info.ClientWidth -or $Y -lt 0 -or $Y -ge $info.ClientHeight) {
        throw "Fast native input is outside the target client: $X,$Y"
    }
    $point = New-Object Rb03UiNative+POINT
    $point.X = $X
    $point.Y = $Y
    [void][Rb03UiNative]::ClientToScreen($handle, [ref]$point)
    [void][Rb03UiNative]::SetCursorPos($point.X, $point.Y)
    Start-Sleep -Milliseconds 40
    [Rb03UiNative]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 30
    [Rb03UiNative]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)
}

function Set-Rb07LightTheme {
    Invoke-Rb03Click 30 35
    Invoke-Rb03Click 688 92
    Invoke-Rb03Click 482 334
    Invoke-Rb03Keys '{ESC}'
    Start-Sleep -Milliseconds 500
    $handle = Activate-Rb03App
    $info = Get-Rb03WindowInfo $handle
    $bitmap = [Drawing.Bitmap]::new($info.ClientWidth, $info.ClientHeight)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen($info.ScreenX, $info.ScreenY, 0, 0, [Drawing.Size]::new($info.ClientWidth, $info.ClientHeight))
    $pixel = $bitmap.GetPixel(640, 10)
    $graphics.Dispose()
    $bitmap.Dispose()
    if ($pixel.B -gt ($pixel.R * 1.3)) {
        throw 'Rust native Light theme did not apply before the operation capture'
    }
}

function Close-Rb07SubdivisionApp {
    param([System.Diagnostics.Process]$Process)

    if ($null -eq $Process) { return }
    $Process.Refresh()
    if ($Process.HasExited -or $null -eq (Get-Process -Id $Process.Id -ErrorAction SilentlyContinue)) { return }
    $script:rb03AppPid = $Process.Id
    Invoke-Rb03Keys '{ESC}'
    Invoke-Rb03Keys '%{F4}'
    Start-Sleep -Milliseconds 500
    if ($null -ne (Get-Process -Id $Process.Id -ErrorAction SilentlyContinue)) {
        Invoke-Rb03Click 554 452
        Start-Sleep -Milliseconds 250
        if ($null -ne (Get-Process -Id $Process.Id -ErrorAction SilentlyContinue)) {
            Invoke-Rb03Keys '{ENTER}'
        }
    }
    $deadline = [DateTime]::UtcNow.AddSeconds(8)
    do {
        Start-Sleep -Milliseconds 150
        $stillRunning = $null -ne (Get-Process -Id $Process.Id -ErrorAction SilentlyContinue)
    } while ($stillRunning -and [DateTime]::UtcNow -lt $deadline)
    if ($stillRunning) { throw "Rust native job-capture process $($Process.Id) did not close cleanly" }
}

try {
    if (Get-Process -Name modeler3d-app -ErrorAction SilentlyContinue) {
        throw 'A Rust 3D Modeler app is already open; refusing overlapping native input'
    }
    if ((Test-Path -LiteralPath $journalPath) -or (Test-Path -LiteralPath $resultsPath)) {
        throw 'Refusing to overwrite existing Subdivide cancellation evidence'
    }
    New-Item -ItemType Directory -Force -Path $localAppDataRoot | Out-Null
    Push-Location $projectRoot
    $env:LOCALAPPDATA = $localAppDataRoot
    $process = Start-Process -FilePath $executable -PassThru
    $script:rb03AppPid = $process.Id
    try {
        $deadline = [DateTime]::UtcNow.AddSeconds(25)
        do {
            Start-Sleep -Milliseconds 250
            try { $null = Get-Rb03AppHandle; $ready = $true } catch { $ready = $false }
        } while (-not $ready -and [DateTime]::UtcNow -lt $deadline)
        if (-not $ready -or $process.HasExited) { throw 'Rust native app failed to become ready' }

        Set-Rb07LightTheme
        Capture-Rb07Subdivision 'welcome-theme-ready' 'empty' 'native Settings Light theme; 1280x840 client at 96 DPI'
        Invoke-Rb03Click 400 380
        Invoke-Rb03Click 41 130
        Invoke-Rb03Click 580 381
        Invoke-Rb03Keys '^a'
        $sent = [Rb03UiNative]::Text($fixture)
        if ($sent -ne ($fixture.Length * 2)) { throw "Incomplete fixture path input: $sent UTF-16 events" }
        Capture-Rb07Subdivision 'import-fixture-path' 'slow-job-closed-cube-heavy.ply' 'native Insert/Import Model path'
        Invoke-Rb03Click 486 487
        Start-Sleep -Seconds 12
        Capture-Rb07Subdivision 'fixture-imported' '960002 vertices / 1920000 faces' 'native Import action; one selected dense mesh'

        Invoke-Rb03Click 210 35
        Start-Sleep -Milliseconds 900
        Capture-Rb07Subdivision 'edit-tab-ready' '960002 vertices / 1920000 faces' 'native Edit tab selected before Subdivide'
        Invoke-Rb07ClickFast 110 130
        Start-Sleep -Milliseconds 120
        Invoke-Rb07ClickFast 660 815
        Start-Sleep -Milliseconds 150
        Capture-Rb07Subdivision 'subdivide-cancel-requested' '1920000 input faces' 'native Subdivide start followed immediately by its Cancel control'
        Start-Sleep -Seconds 30
        Capture-Rb07Subdivision 'subdivide-cancelled' 'original 1920000-face scene and history retained' 'native terminal cancellation'

        Invoke-Rb03Click 110 130
        Start-Sleep -Seconds 30
        Capture-Rb07Subdivision 'subdivide-retry-complete' '1920000 input faces; 3840000 output faces' 'native Subdivide retry after cancellation'
        Invoke-Rb03Click 110 130
        Start-Sleep -Milliseconds 900
        Capture-Rb07Subdivision 'subdivide-limit-failure' '3840000 input faces; 7680000-face result exceeds 4000000-face limit' 'native second Subdivide; controlled limit error'
        Invoke-Rb03Click 181 130
        Start-Sleep -Seconds 20
        Capture-Rb07Subdivision 'smooth-retry-after-failure' '3840000-face scene retained after controlled failure' 'native Edit/Smooth retry after failure'
    } finally {
        try { Close-Rb07SubdivisionApp $process } finally { $script:rb03AppPid = 0 }
    }
    $result = [pscustomobject]@{
        schema_version = 1
        slice = 'RB-07'
        app = 'Rust native desktop'
        build_sha256 = $buildHash
        fixture_sha256 = $fixtureHash
        records = $captureRows
        failures = @()
        all_test_processes_closed = $true
    }
    $result | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $resultsPath -Encoding UTF8
} finally {
    $env:LOCALAPPDATA = $oldLocalAppData
    Pop-Location
}

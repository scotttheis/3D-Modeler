param()

$ErrorActionPreference = 'Stop'
$evidenceRoot = (Resolve-Path $PSScriptRoot).Path
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
. (Join-Path $projectRoot 'evidence\rebuild\rb-04-scene-and-precise-transforms\native-ui-helper.ps1')

$script:rb03AppPid = 0
$script:rb07EvidenceRoot = $evidenceRoot
$script:rb07ProjectRoot = $projectRoot
$script:rb07Executable = Join-Path $projectRoot 'target\debug\modeler3d-app.exe'
$script:rb07Fixture = Join-Path $evidenceRoot 'fixtures\slow-job-closed-cube-heavy.ply'
$script:rb07BuildSha256 = (Get-FileHash -LiteralPath $script:rb07Executable -Algorithm SHA256).Hash.ToLowerInvariant()
$script:rb07Records = [System.Collections.Generic.List[object]]::new()
$script:rb07Journal = Join-Path $evidenceRoot 'rust-job-heavy-native-captures.ndjson'
$script:rb07Results = Join-Path $evidenceRoot 'rust-job-heavy-native-results.json'
$script:oldLocalAppData = $env:LOCALAPPDATA

function Wait-Rb07Window {
    param([System.Diagnostics.Process]$Process)

    $deadline = [DateTime]::UtcNow.AddSeconds(25)
    do {
        Start-Sleep -Milliseconds 250
        try {
            $null = Get-Rb03AppHandle
            $ready = $true
        } catch {
            $ready = $false
        }
    } while (-not $ready -and [DateTime]::UtcNow -lt $deadline)
    if (-not $ready) {
        throw 'RB-07 Rust native target window did not become ready'
    }
    if ($Process.HasExited) {
        throw 'RB-07 Rust native target exited during startup'
    }
}

function Invoke-Rb07Click {
    param([int]$X, [int]$Y)

    $handle = Activate-Rb03App
    $info = Get-Rb03WindowInfo $handle
    if ($X -lt 0 -or $X -ge $info.ClientWidth -or $Y -lt 0 -or $Y -ge $info.ClientHeight) {
        throw "Native input coordinate is outside the target client: $X,$Y"
    }
    $point = New-Object Rb03UiNative+POINT
    $point.X = $X
    $point.Y = $Y
    [void][Rb03UiNative]::ClientToScreen($handle, [ref]$point)
    [void][Rb03UiNative]::SetCursorPos($point.X, $point.Y)
    Start-Sleep -Milliseconds 50
    [Rb03UiNative]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 45
    [Rb03UiNative]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 90
}

function Set-Rb07Theme {
    param([ValidateSet('light', 'dark')][string]$Theme)

    $handle = Activate-Rb03App
    $info = Get-Rb03WindowInfo $handle
    $bitmap = [Drawing.Bitmap]::new($info.ClientWidth, $info.ClientHeight)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen($info.ScreenX, $info.ScreenY, 0, 0, [Drawing.Size]::new($info.ClientWidth, $info.ClientHeight))
    $navigation = $bitmap.GetPixel(640, 10)
    $graphics.Dispose()
    $bitmap.Dispose()
    $looksDark = $navigation.B -gt ($navigation.R * 1.3)
    if (($Theme -eq 'dark' -and $looksDark) -or ($Theme -eq 'light' -and -not $looksDark)) {
        return
    }

    Invoke-Rb07Click 30 35
    Invoke-Rb07Click 688 92
    if ($Theme -eq 'light') {
        Invoke-Rb07Click 482 334
    } else {
        Invoke-Rb07Click 531 334
    }
    Invoke-Rb03Keys '{ESC}'
    Start-Sleep -Milliseconds 350

    $handle = Activate-Rb03App
    $info = Get-Rb03WindowInfo $handle
    $bitmap = [Drawing.Bitmap]::new($info.ClientWidth, $info.ClientHeight)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen($info.ScreenX, $info.ScreenY, 0, 0, [Drawing.Size]::new($info.ClientWidth, $info.ClientHeight))
    $graphics.Dispose()
    $navigation = $bitmap.GetPixel(640, 10)
    $bitmap.Dispose()
    $looksDark = $navigation.B -gt ($navigation.R * 1.3)
    if (($Theme -eq 'light' -and $looksDark) -or ($Theme -eq 'dark' -and -not $looksDark)) {
        throw "Native theme did not settle to $Theme"
    }
}

function Capture-Rb07Job {
    param(
        [ValidateSet('light', 'dark')][string]$Theme,
        [string]$Action,
        [string]$Fixture,
        [string]$InputRoute
    )

    $handle = Activate-Rb03App
    $info = Get-Rb03WindowInfo $handle
    if ($info.Title -notlike '3D Modeler - *' -or $info.ClientWidth -ne 1280 -or $info.ClientHeight -ne 840 -or $info.Dpi -ne 96) {
        throw "Rejected RB-07 capture target or dimensions for $Action"
    }

    $relative = "evidence\rebuild\rb-07-appearance-jobs-and-print\captures\rust-rb07-$Theme-job-heavy-$Action.png"
    $captured = Capture-Rb03App $relative
    $bitmap = [Drawing.Bitmap]::new($captured.Path)
    $navigation = $bitmap.GetPixel(640, 10)
    $bitmap.Dispose()
    $looksDark = $navigation.B -gt ($navigation.R * 1.3)
    if (($Theme -eq 'light' -and $looksDark) -or ($Theme -eq 'dark' -and -not $looksDark)) {
        throw "Rejected RB-07 capture with unexpected theme for $Action ($Theme)"
    }

    $record = [pscustomobject]@{
        slice = 'RB-07'
        app = 'Rust native desktop'
        process_id = $script:rb03AppPid
        theme = $Theme
        action = $Action
        fixture = $Fixture
        input_route = $InputRoute
        window_title = $captured.Title
        logical_client_size = @($captured.ClientWidth, $captured.ClientHeight)
        captured_size = @($captured.ClientWidth, $captured.ClientHeight)
        dpi = $captured.Dpi
        build_sha256 = $script:rb07BuildSha256
        image = $relative.Replace('\', '/')
        image_sha256 = $captured.Sha256
        captured_at = [DateTimeOffset]::Now.ToString('o')
    }
    $script:rb07Records.Add($record)
    ConvertTo-Json -InputObject $record -Depth 8 -Compress | Add-Content -LiteralPath $script:rb07Journal -Encoding UTF8
    return $record
}

function Close-Rb07App {
    param([System.Diagnostics.Process]$Process)

    if ($null -eq $Process) {
        return
    }
    $Process.Refresh()
    if ($Process.HasExited -or $null -eq (Get-Process -Id $Process.Id -ErrorAction SilentlyContinue)) {
        return
    }
    $script:rb03AppPid = $Process.Id
    [void](Activate-Rb03App)
    [System.Windows.Forms.SendKeys]::SendWait('{ESC}')
    Start-Sleep -Milliseconds 150
    [System.Windows.Forms.SendKeys]::SendWait('%{F4}')
    Start-Sleep -Milliseconds 500
    if ($null -ne (Get-Process -Id $Process.Id -ErrorAction SilentlyContinue)) {
        Invoke-Rb07Click 554 452
        Start-Sleep -Milliseconds 250
        [System.Windows.Forms.SendKeys]::SendWait('{ENTER}')
    }
    $deadline = [DateTime]::UtcNow.AddSeconds(5)
    do {
        Start-Sleep -Milliseconds 150
        $stillRunning = $null -ne (Get-Process -Id $Process.Id -ErrorAction SilentlyContinue)
    } while ($stillRunning -and [DateTime]::UtcNow -lt $deadline)
    if ($stillRunning) {
        throw "Native Rust test process $($Process.Id) did not close after its Discard action"
    }
}

function Invoke-Rb07JobRun {
    param([ValidateSet('light', 'dark')][string]$Theme)

    $themeLocalAppData = Join-Path $evidenceRoot "runtime\job-heavy-localappdata-$Theme"
    New-Item -ItemType Directory -Force -Path $themeLocalAppData | Out-Null
    $env:LOCALAPPDATA = $themeLocalAppData
    $process = Start-Process -FilePath $script:rb07Executable -PassThru
    $script:rb03AppPid = $process.Id
    try {
        Wait-Rb07Window $process
        Set-Rb07Theme $Theme
        $null = Capture-Rb07Job $Theme 'welcome-theme-ready' 'empty' 'native Settings theme choice; 1280x840 client, 96 DPI'
        Invoke-Rb07Click 400 380
        Start-Sleep -Milliseconds 450
        # New Scene retains the expanded Insert ribbon; clicking its active
        # tab again would collapse the Import Model button.
        Invoke-Rb07Click 41 130
        Start-Sleep -Milliseconds 250
        Invoke-Rb03Click 580 381
        Invoke-Rb03Keys '^a'
        $sent = [Rb03UiNative]::Text($script:rb07Fixture)
        if ($sent -ne ($script:rb07Fixture.Length * 2)) {
            throw "Import fixture path input was incomplete ($sent input events)"
        }
        $null = Capture-Rb07Job $Theme 'import-fixture-path' 'slow-job-closed-cube-heavy.ply' 'native Insert/Import Model; bounded Unicode SendInput path'
        Invoke-Rb07Click 486 487
        Start-Sleep -Seconds 12
        $null = Capture-Rb07Job $Theme 'fixture-imported' 'slow-job-closed-cube-heavy.ply; 960002 vertices / 1920000 faces' 'native Import button; one selected mesh'

        Invoke-Rb07Click 210 35
        Invoke-Rb07Click 181 130
        Start-Sleep -Milliseconds 800
        $null = Capture-Rb07Job $Theme 'smooth-progress' 'slow-job-closed-cube-heavy.ply; 1920000 faces' 'native Edit/Smooth click; visible running status and Cancel control'

        Invoke-Rb07Click 181 130
        Start-Sleep -Milliseconds 100
        $null = Capture-Rb07Job $Theme 'smooth-duplicate-attempt' 'slow-job-closed-cube-heavy.ply; 1920000 faces' 'native second Edit/Smooth click while first job is expected to run'

        Invoke-Rb07Click 637 815
        Start-Sleep -Milliseconds 100
        $null = Capture-Rb07Job $Theme 'smooth-cancel-requested' 'slow-job-closed-cube-heavy.ply; 1920000 faces' 'native progress Cancel click'
        Start-Sleep -Seconds 5
        $null = Capture-Rb07Job $Theme 'smooth-cancelled' 'slow-job-closed-cube-heavy.ply; scene/history retained' 'native terminal status after cancellation'

        Invoke-Rb07Click 181 130
        Start-Sleep -Seconds 20
        $null = Capture-Rb07Job $Theme 'smooth-retry-complete' 'slow-job-closed-cube-heavy.ply; 1920000 faces' 'native Edit/Smooth click after cancellation'

        Invoke-Rb07Click 110 130
        Start-Sleep -Milliseconds 750
        $null = Capture-Rb07Job $Theme 'subdivide-limit-failure' 'slow-job-closed-cube-heavy.ply; 1920000 input faces exceed the 4000000-face result limit' 'native Edit/Subdivide click; controlled resource-limit failure'
        Invoke-Rb07Click 181 130
        Start-Sleep -Seconds 20
        $null = Capture-Rb07Job $Theme 'smooth-retry-after-failure' 'slow-job-closed-cube-heavy.ply; unchanged source after failed Subdivide' 'native Edit/Smooth click after failure'
    } finally {
        try {
            Close-Rb07App $process
        } finally {
            $script:rb03AppPid = 0
        }
    }
}

try {
    if (Get-Process -Name modeler3d-app -ErrorAction SilentlyContinue) {
        throw 'A 3D Modeler Rust app process is already running; refusing ambiguous native input'
    }
    foreach ($path in @($script:rb07Journal, $script:rb07Results)) {
        if (Test-Path -LiteralPath $path) {
            throw "Refusing to overwrite existing RB-07 job evidence: $path"
        }
    }
    foreach ($theme in @('light', 'dark')) {
        Invoke-Rb07JobRun $theme
    }
    $result = [pscustomobject]@{
        schema_version = 1
        slice = 'RB-07'
        app = 'Rust native desktop'
        build_sha256 = $script:rb07BuildSha256
        fixture_sha256 = (Get-FileHash -LiteralPath $script:rb07Fixture -Algorithm SHA256).Hash.ToLowerInvariant()
        records = $script:rb07Records
        failures = @()
        all_test_processes_closed = $true
    }
    $result | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $script:rb07Results -Encoding UTF8
} finally {
    $env:LOCALAPPDATA = $script:oldLocalAppData
}

param(
    [string[]]$Themes = @('dark', 'light')
)

$ErrorActionPreference = 'Stop'
. "$(Join-Path $PSScriptRoot '..\rb-04-scene-and-precise-transforms\native-ui-helper.ps1')"

$root = (Resolve-Path $PSScriptRoot).Path
$fixture = (Resolve-Path (Join-Path $PSScriptRoot '..\rb-04-scene-and-precise-transforms\native-viewport-fixtures\viewport-move-single.3dp')).Path
$buildHash = (Get-FileHash -LiteralPath 'target\debug\modeler3d-app.exe' -Algorithm SHA256).Hash.ToLowerInvariant()
$resultsPath = Join-Path $root 'rust-display-results.json'
$records = [System.Collections.Generic.List[object]]::new()

function Wait-DisplayFrame {
    $null = Activate-Rb03App
    Start-Sleep -Milliseconds 350
}

function Send-DisplayText([string]$Value) {
    $null = Activate-Rb03App
    foreach ($key in @(16, 17, 18)) {
        if (([Rb03UiNative]::GetAsyncKeyState($key) -band 0x8000) -ne 0) {
            throw 'Modifier held; no display text sent'
        }
    }
    $sent = [Rb03UiNative]::Text($Value)
    if ($sent -ne $Value.Length * 2) {
        throw "Native display text input was incomplete: $sent events"
    }
    Start-Sleep -Milliseconds 300
}

function Set-DisplayTheme([string]$Theme) {
    Invoke-Rb03Click 30 35
    Invoke-Rb03Click 688 92
    Start-Sleep -Milliseconds 400
    if ($Theme -eq 'light') {
        Invoke-Rb03Click 482 334
    } else {
        Invoke-Rb03Click 531 334
    }
    Invoke-Rb03Click 486 535
    Wait-DisplayFrame
}

function Open-DisplayFixture {
    try {
        $currentTitle = (Get-Rb03WindowInfo (Activate-Rb03App)).Title
        if ($currentTitle.EndsWith(' *')) {
            Invoke-Rb03Keys '^z'
        }
    } catch {
        # The initial Welcome surface has no project title to inspect.
    }
    Invoke-Rb03Keys '{ESC}'
    Invoke-Rb03Click 30 35
    Invoke-Rb03Click 235 92
    Start-Sleep -Milliseconds 500
    $info = Get-Rb03WindowInfo (Activate-Rb03App)
    if ($info.ClientWidth -ne 1280 -or $info.ClientHeight -ne 840) {
        throw 'Native display file-dialog precondition has the wrong target size'
    }
    Invoke-Rb03Click 585 405
    Invoke-Rb03Keys '^a'
    Send-DisplayText $fixture
    Invoke-Rb03Click 484 460
    $expected = [IO.Path]::GetFileName($fixture)
    $deadline = [DateTime]::UtcNow.AddSeconds(12)
    do {
        Start-Sleep -Milliseconds 150
        try { $title = (Get-Rb03WindowInfo (Activate-Rb03App)).Title } catch { $title = '' }
    } while ($title -ne "3D Modeler - $expected" -and [DateTime]::UtcNow -lt $deadline)
    if ($title -ne "3D Modeler - $expected") {
        throw "Native display fixture did not open: $expected"
    }
    Wait-DisplayFrame
}

function Capture-Display([string]$Theme, [string]$Action, [string]$Mode, [string]$Route) {
    Wait-DisplayFrame
    $relative = "evidence/rebuild/rb-05-viewport-and-input-completion/captures/rust-$Theme-$Action.png"
    $record = Capture-Rb03App $relative
    if ($record.ClientWidth -ne 1280 -or $record.ClientHeight -ne 840 -or $record.Dpi -ne 96) {
        throw "Rejected display capture dimensions or DPI: $Action"
    }
    $bitmap = [Drawing.Bitmap]::new($record.Path)
    $navigation = $bitmap.GetPixel(640, 10)
    $bitmap.Dispose()
    $looksDark = $navigation.B -gt $navigation.R * 1.3
    if (($Theme -eq 'light' -and $looksDark) -or ($Theme -eq 'dark' -and -not $looksDark)) {
        throw "Rejected display capture theme: $Action ($Theme)"
    }
    $record | Add-Member Theme $Theme
    $record | Add-Member Action $Action
    $record | Add-Member Mode $Mode
    $record | Add-Member InputRoute $Route
    $record | Add-Member Fixture $fixture
    $record | Add-Member BuildSha256 $buildHash
    $record | Add-Member CapturedAt ([DateTimeOffset]::Now.ToString('o'))
    $records.Add($record)
}

$process = $null
try {
    $process = Start-Process -FilePath (Join-Path (Get-Location) 'target\debug\modeler3d-app.exe') -PassThru
    $script:rb03AppPid = $process.Id
    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    do {
        Start-Sleep -Milliseconds 250
        try { $null = Get-Rb03AppHandle; $ready = $true } catch { $ready = $false }
    } while (-not $ready -and [DateTime]::UtcNow -lt $deadline)
    if (-not $ready) { throw 'Rust display target window did not become ready' }

    foreach ($theme in $Themes) {
        Set-DisplayTheme $theme
        Open-DisplayFixture
        Invoke-Rb03Click 337 37
        Capture-Display $theme 'view-rest' 'rest' 'actual OS View tab click'
        Invoke-Rb03Click 422 100
        Capture-Display $theme 'grid-off' 'interaction' 'actual OS Grid button click'
        Invoke-Rb03Click 422 100
        Invoke-Rb03Click 496 100
        Invoke-Rb03Click 568 100
        Capture-Display $theme 'wireframe-xray' 'interaction' 'actual OS Wireframe and X-Ray button clicks'
        Invoke-Rb03Click 638 100
        Capture-Display $theme 'snap-off' 'interaction' 'actual OS Snap button click'
    }
    $records | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $resultsPath -Encoding UTF8
}
finally {
    if ($process -and -not $process.HasExited) {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    }
}

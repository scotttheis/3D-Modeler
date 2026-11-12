param(
    [string[]]$Themes = @('dark', 'light')
)

$ErrorActionPreference = 'Stop'
. "$(Join-Path $PSScriptRoot '..\rb-04-scene-and-precise-transforms\native-ui-helper.ps1')"

$script:rb05Root = (Resolve-Path (Join-Path $PSScriptRoot '')).Path
$script:rb05FixtureRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\rb-04-scene-and-precise-transforms\native-viewport-fixtures')).Path
$script:rb05Geometry = (Resolve-Path (Join-Path $PSScriptRoot '..\rb-04-scene-and-precise-transforms\native_rust_geometry.py')).Path
$script:rb05Records = [System.Collections.Generic.List[object]]::new()
$script:rb05BuildHash = (Get-FileHash -LiteralPath 'target\debug\builder3d-app.exe' -Algorithm SHA256).Hash.ToLowerInvariant()
$script:rb05Journal = Join-Path $script:rb05Root 'rust-native-captures-final.ndjson'
$script:rb05Results = Join-Path $script:rb05Root 'rust-native-results.json'
$script:rb05Error = Join-Path $script:rb05Root 'rust-native-error.txt'

function Wait-Rb05Frame {
    $null = Activate-Rb03App
    $point = New-Object Rb03UiNative+POINT
    $point.X = 1000
    $point.Y = 10
    $handle = Get-Rb03AppHandle
    [void][Rb03UiNative]::ClientToScreen($handle, [ref]$point)
    [void][Rb03UiNative]::SetCursorPos($point.X, $point.Y)
    Start-Sleep -Milliseconds 350
}

function Invoke-Rb05Text([string]$Value) {
    $null = Activate-Rb03App
    foreach ($key in @(16, 17, 18)) {
        if (([Rb03UiNative]::GetAsyncKeyState($key) -band 0x8000) -ne 0) {
            throw 'Modifier held; no text sent'
        }
    }
    $sent = [Rb03UiNative]::Text($Value)
    if ($sent -ne $Value.Length * 2) {
        throw "Native text input was incomplete: $sent events"
    }
    Start-Sleep -Milliseconds 300
}

function Assert-Rb05FileDialog {
    Start-Sleep -Milliseconds 500
    $handle = Activate-Rb03App
    $info = Get-Rb03WindowInfo $handle
    if ($info.ClientWidth -ne 1280 -or $info.ClientHeight -ne 840) {
        throw 'Native file-dialog precondition has the wrong target window size'
    }
}

function Open-Rb05Project([string]$Path) {
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
    Assert-Rb05FileDialog
    Invoke-Rb03Click 585 405
    Invoke-Rb03Keys '^a'
    Invoke-Rb05Text $Path
    Invoke-Rb03Click 484 460
    $expected = [IO.Path]::GetFileName($Path)
    $deadline = [DateTime]::UtcNow.AddSeconds(12)
    do {
        Start-Sleep -Milliseconds 150
        try {
            $title = (Get-Rb03WindowInfo (Activate-Rb03App)).Title
        } catch {
            $title = ''
        }
    } while ($title -ne "Builder3D - $expected" -and [DateTime]::UtcNow -lt $deadline)
    if ($title -ne "Builder3D - $expected") {
        throw "Native project did not open: $expected"
    }
    Wait-Rb05Frame
}

function Capture-Rb05State(
    [string]$Theme,
    [string]$Action,
    [string]$Fixture,
    [string]$Mode = 'rest',
    [string]$InputRoute = 'native window capture'
) {
    Wait-Rb05Frame
    $relative = "evidence/rebuild/rb-05-viewport-and-input-completion/captures/rust-$Theme-$Action.png"
    $record = Capture-Rb03App $relative
    if ($record.ClientWidth -ne 1280 -or $record.ClientHeight -ne 840 -or $record.Dpi -ne 96) {
        throw "Rejected native capture dimensions or DPI for $Action"
    }
    $bitmap = [Drawing.Bitmap]::new($record.Path)
    $navigation = $bitmap.GetPixel(640, 10)
    $bitmap.Dispose()
    $looksDark = $navigation.B -gt $navigation.R * 1.3
    if (($Theme -eq 'light' -and $looksDark) -or ($Theme -eq 'dark' -and -not $looksDark)) {
        throw "Rejected native capture theme for $Action ($Theme)"
    }
    $record | Add-Member Theme $Theme
    $record | Add-Member Action $Action
    $record | Add-Member Fixture $Fixture
    $record | Add-Member Mode $Mode
    $record | Add-Member InputRoute $InputRoute
    $record | Add-Member CapturedAt ([DateTimeOffset]::Now.ToString('o'))
    $record | Add-Member BuildSha256 $script:rb05BuildHash
    $script:rb05Records.Add($record)
    ConvertTo-Json -InputObject $record -Depth 8 -Compress | Add-Content -LiteralPath $script:rb05Journal -Encoding UTF8
    return $record
}

function Set-Rb05Theme([string]$Theme) {
    Invoke-Rb03Click 30 35
    Invoke-Rb03Click 688 92
    Start-Sleep -Milliseconds 400
    if ($Theme -eq 'light') {
        Invoke-Rb03Click 482 334
    } else {
        Invoke-Rb03Click 531 334
    }
    Invoke-Rb03Click 486 535
    Wait-Rb05Frame
}

function Set-Rb05Pointer([int]$X, [int]$Y) {
    $handle = Activate-Rb03App
    $info = Get-Rb03WindowInfo $handle
    if ($X -lt 0 -or $X -ge $info.ClientWidth -or $Y -lt 0 -or $Y -ge $info.ClientHeight) {
        throw 'Pointer is outside the native target client'
    }
    $point = New-Object Rb03UiNative+POINT
    $point.X = $X
    $point.Y = $Y
    [void][Rb03UiNative]::ClientToScreen($handle, [ref]$point)
    [void][Rb03UiNative]::SetCursorPos($point.X, $point.Y)
    Start-Sleep -Milliseconds 80
}

function Invoke-Rb05Drag([int[]]$Start, [int[]]$Finish, [uint32]$Down, [uint32]$Up, [string]$Termination = 'commit') {
    Set-Rb05Pointer $Start[0] $Start[1]
    [Rb03UiNative]::mouse_event($Down, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 120
    try {
        if ($Termination -ne 'noop') {
            foreach ($step in 1..6) {
                $fraction = $step / 6.0
                Set-Rb05Pointer ([int]($Start[0] + ($Finish[0] - $Start[0]) * $fraction)) ([int]($Start[1] + ($Finish[1] - $Start[1]) * $fraction))
            }
        }
        if ($Termination -eq 'escape') {
            Invoke-Rb03Keys '{ESC}'
        }
        if ($Termination -eq 'focus') {
            $handle = Activate-Rb03App
            [void][Rb03UiNative]::ShowWindowAsync($handle, 6)
            Start-Sleep -Milliseconds 350
            if (-not [Rb03UiNative]::Available()) {
                throw 'Desktop became unavailable during focus-loss case'
            }
            [void][Rb03UiNative]::ShowWindowAsync($handle, 9)
            Start-Sleep -Milliseconds 300
            $null = Activate-Rb03App
        }
    } finally {
        if ([Rb03UiNative]::Available()) {
            if ($Termination -eq 'noop') {
                Set-Rb05Pointer $Start[0] $Start[1]
            } else {
                Set-Rb05Pointer $Finish[0] $Finish[1]
            }
            [Rb03UiNative]::mouse_event($Up, 0, 0, 0, [UIntPtr]::Zero)
            Start-Sleep -Milliseconds 350
        }
    }
}

function Invoke-Rb05Wheel([int]$X, [int]$Y, [int]$Delta) {
    Set-Rb05Pointer $X $Y
    [Rb03UiNative]::mouse_event(0x0800, 0, 0, $Delta, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 350
}

function Select-Rb05FirstObject {
    Invoke-Rb03Click 1144 462
    Wait-Rb05Frame
}

function Get-Rb05Geometry([string]$Fixture, [string]$Mode, [string]$Handle, [string]$Selection = '0') {
    $json = & 'L:\Builder3D\venv\Scripts\python.exe' -B $script:rb05Geometry $Fixture --mode $Mode --handle $Handle --selection $Selection | Out-String
    return ($json | ConvertFrom-Json)
}

function Invoke-Rb05GizmoCase([string]$Theme, [string]$Case, [string]$FixtureName, [string]$Mode, [string]$Handle, [string]$Selection = '0', [string]$Termination = 'commit') {
    $fixture = Join-Path $script:rb05FixtureRoot $FixtureName
    Open-Rb05Project $fixture
    Select-Rb05FirstObject
    $modeX = @{ move = 492; rotate = 580; resize = 666 }[$Mode]
    Invoke-Rb03Click $modeX 738
    Wait-Rb05Frame
    $geometry = Get-Rb05Geometry $fixture $Mode $Handle $Selection
    Capture-Rb05State $Theme "$Case-before" $fixture 'rest' 'actual OS pointer/button precondition'
    Invoke-Rb05Drag $geometry.start $geometry.end 2 4 $Termination
    Capture-Rb05State $Theme "$Case-after" $fixture $(if ($Termination -eq 'commit') { 'release' } else { $Termination }) 'actual OS primary pointer drag'
}

$process = $null
try {
    $process = Start-Process -FilePath (Join-Path (Get-Location) 'target\debug\builder3d-app.exe') -PassThru
    $script:rb03AppPid = $process.Id
    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    do {
        Start-Sleep -Milliseconds 250
        try { $null = Get-Rb03AppHandle; $ready = $true } catch { $ready = $false }
    } while (-not $ready -and [DateTime]::UtcNow -lt $deadline)
    if (-not $ready) { throw 'Rust native target window did not become ready' }

    foreach ($theme in $Themes) {
        Set-Rb05Theme $theme
        Invoke-Rb03Click 30 35
        Invoke-Rb03Click 145 92
        Wait-Rb05Frame
        Capture-Rb05State $theme 'empty-rest' 'empty' 'rest' 'native New Scene command through application menu'

        $single = Join-Path $script:rb05FixtureRoot 'viewport-move-single.3dp'
        Open-Rb05Project $single
        Capture-Rb05State $theme 'single-rest' $single 'rest'
        Invoke-Rb05Drag @(700, 300) @(752, 325) 2 4 'commit'
        Capture-Rb05State $theme 'orbit-after' $single 'release' 'actual OS primary pointer drag'
        Invoke-Rb05Drag @(700, 300) @(748, 326) 32 64 'commit'
        Capture-Rb05State $theme 'pan-middle-after' $single 'release' 'actual OS middle pointer drag'
        Invoke-Rb05Drag @(700, 300) @(660, 318) 8 16 'commit'
        Capture-Rb05State $theme 'pan-right-after' $single 'release' 'actual OS secondary pointer drag'
        Invoke-Rb05Wheel 700 300 120
        Capture-Rb05State $theme 'wheel-after' $single 'release' 'actual OS wheel'
        Invoke-Rb03Click 500 450
        Invoke-Rb03Click 500 450
        Capture-Rb05State $theme 'double-click-frame' $single 'release' 'actual OS double primary click'

        Invoke-Rb05GizmoCase $theme 'move-single' 'viewport-move-single.3dp' 'move' 'x'
        Invoke-Rb05GizmoCase $theme 'rotate-single' 'viewport-rotate-single.3dp' 'rotate' 'z'
        Invoke-Rb05GizmoCase $theme 'resize-edge' 'viewport-resize-edge.3dp' 'resize' 'face_0'
        Invoke-Rb05GizmoCase $theme 'resize-corner' 'viewport-resize-corner.3dp' 'resize' 'corner_6'
        Invoke-Rb05GizmoCase $theme 'move-noop' 'viewport-move-noop.3dp' 'move' 'x' '0' 'noop'
        Invoke-Rb05GizmoCase $theme 'move-cancel' 'viewport-move-cancel.3dp' 'move' 'x' '0' 'escape'
        Invoke-Rb05GizmoCase $theme 'move-focus' 'viewport-move-focus.3dp' 'move' 'x' '0' 'focus'
        Invoke-Rb05GizmoCase $theme 'move-group' 'viewport-move-group.3dp' 'move' 'x' '0'
        Invoke-Rb05GizmoCase $theme 'move-hidden' 'viewport-move-hidden.3dp' 'move' 'x' '0,1'

        Open-Rb05Project $single
        Capture-Rb05State $theme 'wire-grid-rest' $single 'rest' 'native viewport render with floor and metric grid'
    }
    $script:rb05Records | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $script:rb05Results -Encoding UTF8
}
catch {
    $_ | Out-String -Width 240 | Set-Content -LiteralPath $script:rb05Error -Encoding UTF8
    throw
}
finally {
    if ($process -and -not $process.HasExited) {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    }
}

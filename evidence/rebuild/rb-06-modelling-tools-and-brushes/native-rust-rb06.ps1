param(
    [string[]]$Themes = @('light', 'dark')
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot '..\rb-04-scene-and-precise-transforms\native-ui-helper.ps1')

$script:rb06EvidenceRoot = (Resolve-Path $PSScriptRoot).Path
$script:rb06ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
$script:rb06Executable = Join-Path $script:rb06ProjectRoot 'target\debug\modeler3d-app.exe'
$script:rb06Records = [System.Collections.Generic.List[object]]::new()
$script:rb06BuildSha256 = (Get-FileHash -LiteralPath $script:rb06Executable -Algorithm SHA256).Hash.ToLowerInvariant()
$script:rb06Journal = Join-Path $script:rb06EvidenceRoot 'rust-native-captures.ndjson'
$script:rb06Results = Join-Path $script:rb06EvidenceRoot 'rust-native-results.json'
$script:rb06Error = Join-Path $script:rb06EvidenceRoot 'rust-native-error.txt'
foreach ($generated in @($script:rb06Journal, $script:rb06Results, $script:rb06Error)) {
    if (Test-Path -LiteralPath $generated) {
        Clear-Content -LiteralPath $generated
    }
}

function Wait-Rb06Ready {
    $deadline = [DateTime]::UtcNow.AddSeconds(20)
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
        throw 'Rust native RB-06 target window did not become ready'
    }
}

function Set-Rb06Theme([string]$Theme) {
    Invoke-Rb03Click 30 35
    Invoke-Rb03Click 688 92
    Start-Sleep -Milliseconds 350
    if ($Theme -eq 'light') {
        Invoke-Rb03Click 482 334
        Invoke-Rb03Click 482 334
    } else {
        Invoke-Rb03Click 531 334
        Invoke-Rb03Click 531 334
    }
    Start-Sleep -Milliseconds 450
    Invoke-Rb03Click 486 535
    Invoke-Rb03Click 486 518
    # Theme selection reports inline and shifts the Done button.  Escape is
    # the stable native close route for both settings layouts.
    Invoke-Rb06Escape
    Start-Sleep -Milliseconds 350
}

function Capture-Rb06State(
    [string]$Theme,
    [string]$Action,
    [string]$Fixture,
    [string]$InputRoute,
    [string]$Mode = 'rest'
) {
    $handle = Activate-Rb03App
    $info = Get-Rb03WindowInfo $handle
    if ($info.ClientWidth -ne 1280 -or $info.ClientHeight -ne 840 -or $info.Dpi -ne 96) {
        throw "Rejected native capture dimensions or DPI for $Action"
    }
    $relative = "captures\rust-$Theme-$Action.png"
    $path = Join-Path $script:rb06EvidenceRoot $relative
    $bitmap = [Drawing.Bitmap]::new($info.ClientWidth, $info.ClientHeight)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen($info.ScreenX, $info.ScreenY, 0, 0, [Drawing.Size]::new($info.ClientWidth, $info.ClientHeight))
    $graphics.Dispose()
    $bitmap.Save($path, [Drawing.Imaging.ImageFormat]::Png)
    $bitmap.Dispose()
    $check = [Drawing.Bitmap]::new($path)
    $navigation = $check.GetPixel(640, 10)
    $check.Dispose()
    $looksDark = $navigation.B -gt ($navigation.R * 1.3)
    if (($Theme -eq 'light' -and $looksDark) -or ($Theme -eq 'dark' -and -not $looksDark)) {
        throw "Rejected native capture theme for $Action ($Theme)"
    }
    $record = [pscustomobject]@{
        Action = $Action
        Theme = $Theme
        Fixture = $Fixture
        Mode = $Mode
        InputRoute = $InputRoute
        ClientWidth = $info.ClientWidth
        ClientHeight = $info.ClientHeight
        Dpi = $info.Dpi
        ScreenX = $info.ScreenX
        ScreenY = $info.ScreenY
        Path = $relative.Replace('\', '/')
        Sha256 = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
        BuildSha256 = $script:rb06BuildSha256
        CapturedAt = [DateTimeOffset]::Now.ToString('o')
    }
    $script:rb06Records.Add($record)
    ConvertTo-Json -InputObject $record -Depth 8 -Compress | Add-Content -LiteralPath $script:rb06Journal -Encoding UTF8
    return $record
}

function Set-Rb06Pointer([int]$X, [int]$Y) {
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
    Start-Sleep -Milliseconds 90
}

function Get-Rb06SurfaceState {
    $handle = Activate-Rb03App
    $info = Get-Rb03WindowInfo $handle
    $bitmap = [Drawing.Bitmap]::new($info.ClientWidth, $info.ClientHeight)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen($info.ScreenX, $info.ScreenY, 0, 0, [Drawing.Size]::new($info.ClientWidth, $info.ClientHeight))
    $graphics.Dispose()
    $xs = @(80, 145, 210, 273, 336)
    $colors = @()
    foreach ($x in $xs) {
        $pixel = $bitmap.GetPixel($x, 50)
        $colors += "$($pixel.R),$($pixel.G),$($pixel.B)"
    }
    $active = -1
    for ($index = 0; $index -lt $colors.Count; $index++) {
        $other = @($colors | Where-Object { $_ -ne $colors[$index] })
        if ($other.Count -ge 3) {
            $active = $index
            break
        }
    }
    $top = $bitmap.GetPixel(5, 130)
    $viewport = $bitmap.GetPixel(5, 200)
    $expanded = [Math]::Abs($top.R - $viewport.R) -gt 5
    $bitmap.Dispose()
    return [pscustomobject]@{ Active = $active; Expanded = $expanded }
}

function Select-Rb06Tab([int]$Index, [int]$X) {
    for ($attempt = 0; $attempt -lt 6; $attempt++) {
        $state = Get-Rb06SurfaceState
        if ($state.Active -eq $Index -and $state.Expanded) {
            return
        }
        Invoke-Rb03Click $X 35
        Start-Sleep -Milliseconds 250
    }
    $state = Get-Rb06SurfaceState
    if ($state.Active -ne $Index -or -not $state.Expanded) {
        throw "Native tab did not settle (wanted $Index, active $($state.Active), expanded $($state.Expanded))"
    }
}

function Invoke-Rb06ShiftDrag([int]$StartX, [int]$StartY, [int]$EndX, [int]$EndY) {
    Set-Rb06Pointer $StartX $StartY
    [Rb03UiNative]::keybd_event(0x10, 0, 0, [UIntPtr]::Zero)
    try {
        [Rb03UiNative]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero)
        Start-Sleep -Milliseconds 120
        foreach ($step in 1..6) {
            $fraction = $step / 6.0
            Set-Rb06Pointer ([int]($StartX + ($EndX - $StartX) * $fraction)) ([int]($StartY + ($EndY - $StartY) * $fraction))
        }
        [Rb03UiNative]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)
    } finally {
        [Rb03UiNative]::keybd_event(0x10, 0, 2, [UIntPtr]::Zero)
    }
    Start-Sleep -Milliseconds 450
}

function Invoke-Rb06Escape {
    Invoke-Rb03Keys '{ESC}'
}

function Invoke-Rb06Brush([int]$X, [string]$Name, [string]$Theme, [string]$Fixture) {
    Invoke-Rb03Click $X 130
    Capture-Rb06State $Theme "brush-$($Name.ToLowerInvariant().Replace(' ', '-'))-active" $Fixture 'actual OS Edit ribbon brush click' 'active'
    Invoke-Rb06Escape
}

function Invoke-Rb06BrushStroke([int]$X, [string]$Name, [string]$Theme, [string]$Fixture) {
    $slug = $Name.ToLowerInvariant().Replace(' ', '-')
    Invoke-Rb03Click $X 130
    Capture-Rb06State $Theme "brush-$slug-active" $Fixture 'actual OS Edit ribbon brush click' 'active'
    Capture-Rb06State $Theme "$slug-stroke-before" $Fixture 'actual OS brush activation' 'stroke'
    Invoke-Rb06ShiftDrag 520 480 575 495
    Capture-Rb06State $Theme "$slug-stroke-after" $Fixture 'actual OS Shift-primary brush stroke' 'commit'
    Invoke-Rb06Escape
}

$process = $null
try {
    foreach ($theme in $Themes) {
        $process = Start-Process -FilePath $script:rb06Executable -PassThru
        $script:rb03AppPid = $process.Id
        Wait-Rb06Ready
        Set-Rb06Theme $theme

        # Welcome -> Scene is a native click, preserving the same empty-state
        # and 1280x840 capture contract as the paired reference run.
        Invoke-Rb03Click 400 380
        Capture-Rb06State $theme 'empty-rest' 'empty' 'actual OS New Scene card click'

        # Torus insertion and its live editor.  Preview changes are made in
        # the dialog before the single Apply commit.
        Select-Rb06Tab 0 80
        Invoke-Rb03Click 468 130
        Capture-Rb06State $theme 'torus-dialog' 'primitive-torus' 'actual OS Insert/Torus click' 'dialog'
        Invoke-Rb03Click 572 350
        Capture-Rb06State $theme 'torus-hole-preview' 'primitive-torus' 'actual OS Torus Hole button click' 'preview'
        Invoke-Rb03Click 500 520
        Capture-Rb06State $theme 'torus-applied' 'primitive-torus' 'actual OS Apply torus click' 'commit'

        # Use a low-face sphere for the persistent brush family and primitive
        # modelling commands.  Every brush is activated and receives a real
        # OS Shift-primary path before it is cancelled.
        Select-Rb06Tab 0 80
        Invoke-Rb03Click 398 130
        $fixture = 'native-sphere-brush-fixture'
        Select-Rb06Tab 2 210
        Invoke-Rb06BrushStroke 250 'Grab' $theme $fixture
        Invoke-Rb06BrushStroke 318 'Crease' $theme $fixture
        Invoke-Rb06BrushStroke 402 'Smooth Brush' $theme $fixture
        Invoke-Rb06BrushStroke 486 'Blob' $theme $fixture
        Invoke-Rb06BrushStroke 558 'Normalize' $theme $fixture

        Select-Rb06Tab 2 210
        Invoke-Rb03Click 38 130
        Start-Sleep -Milliseconds 1000
        Capture-Rb06State $theme 'simplify-result' $fixture 'actual OS Edit/Simplify click' 'commit'
        Invoke-Rb03Click 108 130
        Start-Sleep -Milliseconds 1200
        Capture-Rb06State $theme 'subdivide-result' $fixture 'actual OS Edit/Subdivide click' 'commit'
        Invoke-Rb03Click 180 130
        Start-Sleep -Milliseconds 1200
        Capture-Rb06State $theme 'smooth-result' $fixture 'actual OS Edit/Smooth click' 'commit'

        # One selected object enters the Boolean picker; Escape is a native
        # cancellation.  A second primitive then supplies the exact-two path.
        Invoke-Rb03Click 638 130
        Capture-Rb06State $theme 'merge-picker' $fixture 'actual OS Edit/Merge click with one selection' 'picker'
        Invoke-Rb06Escape
        Capture-Rb06State $theme 'merge-picker-cancel' $fixture 'actual OS Escape' 'cancel'
        Select-Rb06Tab 0 80
        Invoke-Rb03Click 122 130
        Invoke-Rb03Click 1080 284
        Invoke-Rb03Click 1150 531
        Capture-Rb06State $theme 'boolean-two-selection' $fixture 'actual OS Items/All plus third-row deselect' 'precondition'
        Select-Rb06Tab 2 210
        Invoke-Rb03Click 638 130
        Start-Sleep -Milliseconds 3500
        Capture-Rb06State $theme 'merge-complete' $fixture 'actual OS two-selection Merge click' 'commit'

        # Emboss and Hollow retain their typed parameter dialogs at the
        # RB-06 boundary.  Reach and cancel both native editors here; the
        # bounded successful geometry paths are covered by core/app tests.
        Select-Rb06Tab 2 210
        Invoke-Rb03Click 705 130
        Capture-Rb06State $theme 'emboss-dialog' $fixture 'actual OS Edit/Emboss click' 'dialog'
        Invoke-Rb06Escape
        Invoke-Rb03Click 774 130
        Capture-Rb06State $theme 'hollow-dialog' $fixture 'actual OS Edit/Hollow click' 'dialog'
        Invoke-Rb06Escape

        # Exercise the other two boolean operators through the same native
        # exact-two selection route after the union has completed.
        Select-Rb06Tab 0 80
        Invoke-Rb03Click 122 130
        Invoke-Rb03Click 1080 284
        Invoke-Rb03Click 1150 531
        Select-Rb06Tab 2 210
        Invoke-Rb03Click 844 130
        Start-Sleep -Milliseconds 3500
        Capture-Rb06State $theme 'intersect-complete' $fixture 'actual OS two-selection Intersect click' 'commit'
        Select-Rb06Tab 0 80
        Invoke-Rb03Click 122 130
        Invoke-Rb03Click 1080 284
        Invoke-Rb03Click 1150 498
        Select-Rb06Tab 2 210
        Invoke-Rb03Click 913 130
        Start-Sleep -Milliseconds 3500
        Capture-Rb06State $theme 'subtract-complete' $fixture 'actual OS two-selection Subtract click' 'commit'

        # Object and Paint command surfaces, including dialog reachability.
        Select-Rb06Tab 1 145
        Capture-Rb06State $theme 'object-tab' $fixture 'actual OS Object tab click'
        Invoke-Rb03Click 110 130
        Capture-Rb06State $theme 'repair-dialog' $fixture 'actual OS Object/Repair click' 'dialog'
        Invoke-Rb06Escape
        Select-Rb06Tab 3 273
        Capture-Rb06State $theme 'paint-tab' $fixture 'actual OS Paint tab click'
        Invoke-Rb03Click 40 130
        Capture-Rb06State $theme 'material-dialog' $fixture 'actual OS Paint/Material click' 'dialog'
        Invoke-Rb06Escape

        # View toggles and the Items panel action buttons.
        Select-Rb06Tab 4 336
        Invoke-Rb03Click 423 130
        Invoke-Rb03Click 496 130
        Invoke-Rb03Click 568 130
        Invoke-Rb03Click 638 130
        Capture-Rb06State $theme 'view-toggles' $fixture 'actual OS View ribbon clicks'
        Invoke-Rb03Click 1128 284
        Invoke-Rb03Click 1080 284
        Invoke-Rb03Click 1180 284
        Capture-Rb06State $theme 'items-actions' $fixture 'actual OS Items panel All/None/Invert clicks'

        if (-not $process.HasExited) {
            Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
        }
        $process = $null
        $script:rb03AppPid = 0
        Start-Sleep -Milliseconds 500
    }
    $script:rb06Records | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $script:rb06Results -Encoding UTF8
}
catch {
    $_ | Out-String -Width 240 | Set-Content -LiteralPath $script:rb06Error -Encoding UTF8
    throw
}
finally {
    if ($process -and -not $process.HasExited) {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    }
}

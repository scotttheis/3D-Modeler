Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
. "$PSScriptRoot\rb08-ui-driver.ps1"

function Get-Rb08AutomationRoot {
    if ($script:rb08TargetPid -le 0) { throw 'Set the RB-08 target PID first.' }
    $main = @(Get-Rb08TargetWindows | Where-Object { $_.Title -like 'Builder3D - *' })
    if ($main.Count -ne 1) { throw "Expected one Builder3D main window, found $($main.Count)." }
    return [System.Windows.Automation.AutomationElement]::FromHandle([IntPtr]$main[0].Handle)
}

function Get-Rb08AutomationElements {
    param([string]$Name)
    $root = Get-Rb08AutomationRoot
    if ([string]::IsNullOrWhiteSpace($Name)) {
        return $root.FindAll(
            [System.Windows.Automation.TreeScope]::Descendants,
            [System.Windows.Automation.Condition]::TrueCondition)
    }
    $condition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::NameProperty,
        $Name)
    return $root.FindAll([System.Windows.Automation.TreeScope]::Descendants,$condition)
}

function Invoke-Rb08NamedControlClick {
    param(
        [Parameter(Mandatory)][string]$Name,
        [int]$Index = 0
    )
    $matches = @(Get-Rb08AutomationElements -Name $Name)
    if ($matches.Count -le $Index) { throw "Accessible control '$Name' index $Index was not found." }
    $element = $matches[$Index]
    if (-not $element.Current.IsEnabled) { throw "Accessible control '$Name' is disabled." }
    $bounds = $element.Current.BoundingRectangle
    if ($bounds.IsEmpty -or $bounds.Width -le 0 -or $bounds.Height -le 0) {
        throw "Accessible control '$Name' has no visible bounds."
    }
    $main = @(Get-Rb08TargetWindows | Where-Object { $_.Title -like 'Builder3D - *' })
    if ($main.Count -ne 1) { throw "Expected one Builder3D main window, found $($main.Count)." }
    $x = [int][Math]::Round($bounds.X + ($bounds.Width / 2) - $main[0].Left)
    $y = [int][Math]::Round($bounds.Y + ($bounds.Height / 2) - $main[0].Top)
    Invoke-Rb08ForegroundClick -X $x -Y $y
    return [pscustomobject]@{Name=$Name;Index=$Index;ScreenX=[int][Math]::Round($bounds.X+($bounds.Width/2));ScreenY=[int][Math]::Round($bounds.Y+($bounds.Height/2));RelativeX=$x;RelativeY=$y}
}

function Get-Rb08FocusedAutomationElement {
    $element = [System.Windows.Automation.AutomationElement]::FocusedElement
    if ($null -eq $element) { return $null }
    return [pscustomobject]@{
        Name = $element.Current.Name
        ControlType = $element.Current.ControlType.ProgrammaticName
        KeyboardFocusable = $element.Current.IsKeyboardFocusable
        HasKeyboardFocus = $element.Current.HasKeyboardFocus
        Bounds = $element.Current.BoundingRectangle
    }
}

function Save-Rb08AccessibilityTree {
    param([Parameter(Mandatory)][string]$RelativePath)
    $root = Get-Rb08AutomationRoot
    $all = $root.FindAll(
        [System.Windows.Automation.TreeScope]::Descendants,
        [System.Windows.Automation.Condition]::TrueCondition)
    $elements = @()
    for ($i=0; $i -lt $all.Count; $i++) {
        $element = $all.Item($i)
        try {
            $bounds = $element.Current.BoundingRectangle
            $elements += [pscustomobject]@{
                name = $element.Current.Name
                control_type = $element.Current.ControlType.ProgrammaticName
                automation_id = $element.Current.AutomationId
                enabled = $element.Current.IsEnabled
                keyboard_focusable = $element.Current.IsKeyboardFocusable
                has_keyboard_focus = $element.Current.HasKeyboardFocus
                bounds = @([Math]::Round($bounds.X,1),[Math]::Round($bounds.Y,1),[Math]::Round($bounds.Width,1),[Math]::Round($bounds.Height,1))
            }
        } catch {}
    }
    $doc = [ordered]@{
        captured_utc = [DateTime]::UtcNow.ToString('o')
        process_id = $script:rb08TargetPid
        window_title = $root.Current.Name
        descendant_count = $all.Count
        named_count = @($elements | Where-Object { $_.name }).Count
        elements = $elements
    }
    $path = Join-Path (Get-Location) $RelativePath
    $json = $doc | ConvertTo-Json -Depth 7
    [System.IO.File]::WriteAllText($path,$json,[System.Text.UTF8Encoding]::new($false))
    return [pscustomobject]@{Path=$path;Descendants=$all.Count;Named=$doc.named_count;Sha256=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()}
}

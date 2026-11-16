Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class Rb08NativeUi {
 [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
 [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X,Y; }
 public delegate bool EnumProc(IntPtr h,IntPtr p);
 [DllImport("user32.dll")] public static extern IntPtr OpenInputDesktop(uint flags,bool inherit,uint access);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern bool GetUserObjectInformation(IntPtr h,int index,StringBuilder name,uint length,out uint needed);
 [DllImport("user32.dll")] public static extern bool CloseDesktop(IntPtr h);
 [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc callback,IntPtr data);
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
 [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h,StringBuilder text,int count);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h,out uint processId);
 [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h,out RECT rect);
 [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h,out RECT rect);
 [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h,ref POINT point);
 [DllImport("user32.dll",SetLastError=true)] public static extern bool SetWindowPos(IntPtr h,IntPtr after,int x,int y,int width,int height,uint flags);
 [DllImport("user32.dll")] public static extern bool ShowWindowAsync(IntPtr h,int command);
 [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
 [DllImport("user32.dll")] public static extern bool SetCursorPos(int x,int y);
 [DllImport("user32.dll")] public static extern void mouse_event(uint flags,uint x,uint y,uint data,UIntPtr extra);
 public static bool IsDefaultInputDesktop(){IntPtr h=OpenInputDesktop(0,false,1);if(h==IntPtr.Zero)return false;try{var n=new StringBuilder(256);uint need;return GetUserObjectInformation(h,2,n,512,out need)&&n.ToString().Equals("Default",StringComparison.OrdinalIgnoreCase);}finally{CloseDesktop(h);}}
}
"@

$script:rb08TargetPid = 0

function Set-Rb08TargetProcess([int]$ProcessId) {
    $null = Get-Process -Id $ProcessId -ErrorAction Stop
    $script:rb08TargetPid = $ProcessId
}

function Get-Rb08TargetWindows {
    if ($script:rb08TargetPid -le 0) { throw 'Set the RB-08 target PID first.' }
    $rows = [System.Collections.Generic.List[object]]::new()
    [void][Rb08NativeUi]::EnumWindows({
        param($h,$unused)
        [uint32]$owner = 0
        [void][Rb08NativeUi]::GetWindowThreadProcessId($h,[ref]$owner)
        if ($owner -eq $script:rb08TargetPid -and [Rb08NativeUi]::IsWindowVisible($h)) {
            $title = [Text.StringBuilder]::new(512)
            [void][Rb08NativeUi]::GetWindowText($h,$title,$title.Capacity)
            $rect = New-Object Rb08NativeUi+RECT
            [void][Rb08NativeUi]::GetWindowRect($h,[ref]$rect)
            $rows.Add([pscustomobject]@{Handle=$h;Title=$title.ToString();Left=$rect.L;Top=$rect.T;Width=$rect.R-$rect.L;Height=$rect.B-$rect.T})
        }
        return $true
    },[IntPtr]::Zero)
    return $rows.ToArray()
}

function Set-Rb08MainClientSize([int]$ClientWidth,[int]$ClientHeight) {
    if ($ClientWidth -le 0 -or $ClientHeight -le 0) { throw 'Client dimensions must be positive.' }
    $matches = @(Get-Rb08TargetWindows | Where-Object { $_.Title -like '3D Modeler - *' })
    if ($matches.Count -ne 1) { throw "Expected one 3D Modeler main window, found $($matches.Count)." }
    $h = [IntPtr]$matches[0].Handle
    $outer = New-Object Rb08NativeUi+RECT
    $client = New-Object Rb08NativeUi+RECT
    [void][Rb08NativeUi]::GetWindowRect($h,[ref]$outer)
    [void][Rb08NativeUi]::GetClientRect($h,[ref]$client)
    $frameWidth = ($outer.R-$outer.L)-($client.R-$client.L)
    $frameHeight = ($outer.B-$outer.T)-($client.B-$client.T)
    $ok = [Rb08NativeUi]::SetWindowPos($h,[IntPtr]::Zero,$outer.L,$outer.T,$ClientWidth+$frameWidth,$ClientHeight+$frameHeight,0x0004)
    if (-not $ok) { throw "SetWindowPos failed with Win32 error $([Runtime.InteropServices.Marshal]::GetLastWin32Error())." }
    Start-Sleep -Milliseconds 350
    $actual = New-Object Rb08NativeUi+RECT
    [void][Rb08NativeUi]::GetClientRect($h,[ref]$actual)
    $actualWidth = $actual.R-$actual.L
    $actualHeight = $actual.B-$actual.T
    if ($actualWidth -ne $ClientWidth -or $actualHeight -ne $ClientHeight) {
        throw "Requested ${ClientWidth}x${ClientHeight} client; received ${actualWidth}x${actualHeight}."
    }
    return [pscustomobject]@{Width=$actualWidth;Height=$actualHeight;Dpi=[Rb08NativeUi]::GetDpiForWindow($h)}
}

function Set-Rb08MainMaximized {
    $matches = @(Get-Rb08TargetWindows | Where-Object { $_.Title -like '3D Modeler - *' })
    if ($matches.Count -ne 1) { throw "Expected one 3D Modeler main window, found $($matches.Count)." }
    $h = [IntPtr]$matches[0].Handle
    [void][Rb08NativeUi]::ShowWindowAsync($h,3)
    Start-Sleep -Milliseconds 500
    $client = New-Object Rb08NativeUi+RECT
    [void][Rb08NativeUi]::GetClientRect($h,[ref]$client)
    return [pscustomobject]@{Width=$client.R-$client.L;Height=$client.B-$client.T;Dpi=[Rb08NativeUi]::GetDpiForWindow($h)}
}

function Get-Rb08VerifiedForeground {
    if ($script:rb08TargetPid -le 0) { throw 'Set the RB-08 target PID first.' }
    if (-not [Rb08NativeUi]::IsDefaultInputDesktop()) { throw 'Input desktop unavailable; no native input or capture sent.' }
    $h = [Rb08NativeUi]::GetForegroundWindow()
    if ($h -eq [IntPtr]::Zero) { throw 'No foreground window.' }
    [uint32]$owner = 0
    [void][Rb08NativeUi]::GetWindowThreadProcessId($h,[ref]$owner)
    $foregroundProcess = Get-Process -Id $owner -ErrorAction SilentlyContinue
    if ($foregroundProcess -and $foregroundProcess.ProcessName -in @('LockApp','LogonUI')) {
        throw 'Windows lock screen is foreground; no native input or capture sent.'
    }
    if ($owner -ne $script:rb08TargetPid) { throw "Foreground PID $owner does not match RB-08 target PID $script:rb08TargetPid." }
    return $h
}

function Invoke-Rb08ForegroundClick([int]$X,[int]$Y) {
    $h = Get-Rb08VerifiedForeground
    $rect = New-Object Rb08NativeUi+RECT
    [void][Rb08NativeUi]::GetWindowRect($h,[ref]$rect)
    $screenX = $rect.L + $X
    $screenY = $rect.T + $Y
    if ($X -lt 0 -or $Y -lt 0 -or $screenX -ge $rect.R -or $screenY -ge $rect.B) { throw 'Click lies outside the verified foreground window.' }
    [void][Rb08NativeUi]::SetCursorPos($screenX,$screenY)
    Start-Sleep -Milliseconds 80
    [Rb08NativeUi]::mouse_event(2,0,0,0,[UIntPtr]::Zero)
    Start-Sleep -Milliseconds 65
    [Rb08NativeUi]::mouse_event(4,0,0,0,[UIntPtr]::Zero)
    Start-Sleep -Milliseconds 400
}

function Invoke-Rb08ForegroundKeys([string]$Keys) {
    $null = Get-Rb08VerifiedForeground
    [System.Windows.Forms.SendKeys]::SendWait($Keys)
    Start-Sleep -Milliseconds 300
}

function Capture-Rb08Window([IntPtr]$Handle,[string]$RelativePath) {
    if ($script:rb08TargetPid -le 0) { throw 'Set the RB-08 target PID first.' }
    $foreground = Get-Rb08VerifiedForeground
    if ($Handle -ne $foreground) { throw 'Capture handle is not the verified foreground window.' }
    [uint32]$owner = 0
    [void][Rb08NativeUi]::GetWindowThreadProcessId($Handle,[ref]$owner)
    if ($owner -ne $script:rb08TargetPid) { throw "Capture target PID $owner does not match RB-08 target PID $script:rb08TargetPid." }
    if (-not [Rb08NativeUi]::IsDefaultInputDesktop()) { throw 'Input desktop unavailable; no capture made.' }
    $rect = New-Object Rb08NativeUi+RECT
    [void][Rb08NativeUi]::GetWindowRect($Handle,[ref]$rect)
    $width = $rect.R-$rect.L
    $height = $rect.B-$rect.T
    if ($width -le 0 -or $height -le 0) { throw 'Capture target has an empty rectangle.' }
    $path = Join-Path (Get-Location) $RelativePath
    $bitmap = [Drawing.Bitmap]::new($width,$height)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen($rect.L,$rect.T,0,0,$bitmap.Size)
    $bitmap.Save($path,[Drawing.Imaging.ImageFormat]::Png)
    $graphics.Dispose()
    $bitmap.Dispose()
    $title = [Text.StringBuilder]::new(512)
    [void][Rb08NativeUi]::GetWindowText($Handle,$title,$title.Capacity)
    return [pscustomobject]@{Handle=$Handle;Title=$title.ToString();Width=$width;Height=$height;Path=$path;Sha256=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()}
}

function Capture-Rb08Foreground([string]$RelativePath) {
    $h = Get-Rb08VerifiedForeground
    return Capture-Rb08Window $h $RelativePath
}

function Capture-Rb08MainClient([string]$RelativePath) {
    if ($script:rb08TargetPid -le 0) { throw 'Set the RB-08 target PID first.' }
    $null = Get-Rb08VerifiedForeground
    $matches = @(Get-Rb08TargetWindows | Where-Object { $_.Title -like '3D Modeler - *' })
    if ($matches.Count -ne 1) { throw "Expected one 3D Modeler main window, found $($matches.Count)." }
    $h = [IntPtr]$matches[0].Handle
    if ($h -ne [Rb08NativeUi]::GetForegroundWindow()) { throw 'Main window is not the verified foreground window.' }
    [uint32]$owner = 0
    [void][Rb08NativeUi]::GetWindowThreadProcessId($h,[ref]$owner)
    if ($owner -ne $script:rb08TargetPid -or -not [Rb08NativeUi]::IsDefaultInputDesktop()) { throw 'Main-window capture target is not verified.' }
    $rect = New-Object Rb08NativeUi+RECT
    [void][Rb08NativeUi]::GetClientRect($h,[ref]$rect)
    $point = New-Object Rb08NativeUi+POINT
    [void][Rb08NativeUi]::ClientToScreen($h,[ref]$point)
    $width = $rect.R-$rect.L
    $height = $rect.B-$rect.T
    $path = Join-Path (Get-Location) $RelativePath
    $bitmap = [Drawing.Bitmap]::new($width,$height)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen($point.X,$point.Y,0,0,$bitmap.Size)
    $bitmap.Save($path,[Drawing.Imaging.ImageFormat]::Png)
    $graphics.Dispose()
    $bitmap.Dispose()
    return [pscustomobject]@{Handle=$h;Title=$matches[0].Title;Width=$width;Height=$height;Dpi=[Rb08NativeUi]::GetDpiForWindow($h);Path=$path;Sha256=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()}
}

function Capture-Rb08ClientWindow([IntPtr]$Handle,[string]$RelativePath) {
    if ($script:rb08TargetPid -le 0) { throw 'Set the RB-08 target PID first.' }
    [uint32]$owner = 0
    [void][Rb08NativeUi]::GetWindowThreadProcessId($Handle,[ref]$owner)
    if ($owner -ne $script:rb08TargetPid -or -not [Rb08NativeUi]::IsDefaultInputDesktop()) { throw 'Client capture target is not verified.' }
    if ($Handle -ne (Get-Rb08VerifiedForeground)) { throw 'Client capture handle is not the verified foreground window.' }
    $rect = New-Object Rb08NativeUi+RECT
    [void][Rb08NativeUi]::GetClientRect($Handle,[ref]$rect)
    $point = New-Object Rb08NativeUi+POINT
    [void][Rb08NativeUi]::ClientToScreen($Handle,[ref]$point)
    $width = $rect.R-$rect.L
    $height = $rect.B-$rect.T
    $path = Join-Path (Get-Location) $RelativePath
    $bitmap = [Drawing.Bitmap]::new($width,$height)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen($point.X,$point.Y,0,0,$bitmap.Size)
    $bitmap.Save($path,[Drawing.Imaging.ImageFormat]::Png)
    $graphics.Dispose()
    $bitmap.Dispose()
    $title = [Text.StringBuilder]::new(512)
    [void][Rb08NativeUi]::GetWindowText($Handle,$title,$title.Capacity)
    return [pscustomobject]@{Handle=$Handle;Title=$title.ToString();Width=$width;Height=$height;Dpi=[Rb08NativeUi]::GetDpiForWindow($Handle);Path=$path;Sha256=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()}
}

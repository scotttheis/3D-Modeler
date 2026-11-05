param(
    [Parameter(Mandatory = $true)][ValidateSet('probe', 'activate', 'resize', 'key', 'desktop-key', 'click', 'type', 'capture')][string]$Action,
    [Parameter(Mandatory = $false)][int]$ProcessId,
    [Parameter(Mandatory = $false)][int]$X,
    [Parameter(Mandatory = $false)][int]$Y,
    [Parameter(Mandatory = $false)][int]$Width,
    [Parameter(Mandatory = $false)][int]$Height,
    [Parameter(Mandatory = $false)][string]$Text,
    [Parameter(Mandatory = $false)][byte[]]$Keys,
    [Parameter(Mandatory = $false)][string]$OutputPath
)

$ErrorActionPreference = 'Stop'

Add-Type -TypeDefinition @'
using System;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;

public static class HostDesktopProbe {
    [StructLayout(LayoutKind.Sequential)]
    public struct Rect { public int Left; public int Top; public int Right; public int Bottom; }

    [DllImport("user32.dll", SetLastError = true)]
    private static extern IntPtr OpenInputDesktop(uint flags, bool inherit, uint access);
    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern bool GetUserObjectInformation(IntPtr hObj, int index, StringBuilder info, int length, out uint needed);
    [DllImport("user32.dll", SetLastError = true)]
    private static extern bool CloseDesktop(IntPtr hDesktop);
    [DllImport("user32.dll")]
    public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetWindowText(IntPtr hwnd, StringBuilder text, int max);
    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
    [DllImport("user32.dll", SetLastError = true)]
    public static extern bool GetWindowRect(IntPtr hwnd, out Rect rect);
    [DllImport("user32.dll", SetLastError = true)]
    public static extern bool GetClientRect(IntPtr hwnd, out Rect rect);
    [DllImport("user32.dll", SetLastError = true)]
    public static extern bool ClientToScreen(IntPtr hwnd, ref NativePoint point);
    [DllImport("user32.dll", SetLastError = true)]
    private static extern bool SetWindowPos(IntPtr hwnd, IntPtr insertAfter, int x, int y, int cx, int cy, uint flags);
    [DllImport("user32.dll")]
    private static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")]
    private static extern bool BringWindowToTop(IntPtr hwnd);
    [DllImport("user32.dll")]
    private static extern IntPtr SetActiveWindow(IntPtr hwnd);
    [DllImport("user32.dll")]
    private static extern bool AttachThreadInput(uint from, uint to, bool attach);
    [DllImport("kernel32.dll")]
    private static extern uint GetCurrentThreadId();
    [DllImport("user32.dll")]
    private static extern bool ShowWindow(IntPtr hwnd, int command);
    [DllImport("user32.dll")]
    private static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")]
    private static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extraInfo);
    [DllImport("user32.dll")]
    private static extern void keybd_event(byte virtualKey, byte scanCode, uint flags, UIntPtr extraInfo);
    [DllImport("user32.dll")]
    private static extern int GetSystemMetrics(int index);
    [DllImport("user32.dll")]
    private static extern uint GetDpiForWindow(IntPtr hwnd);
    [DllImport("wtsapi32.dll", SetLastError = true)]
    private static extern bool WTSQuerySessionInformation(IntPtr server, int sessionId, int infoClass, out IntPtr buffer, out int bytesReturned);
    [DllImport("wtsapi32.dll")]
    private static extern void WTSFreeMemory(IntPtr buffer);

    [StructLayout(LayoutKind.Sequential)]
    public struct NativePoint { public int X; public int Y; }

    private const uint KEYEVENTF_KEYUP = 0x0002;
    private const uint MOUSEEVENTF_LEFTDOWN = 0x0002;
    private const uint MOUSEEVENTF_LEFTUP = 0x0004;
    private const uint SWP_NOZORDER = 0x0004;
    private const uint SWP_NOACTIVATE = 0x0010;

    public static string InputDesktopName() {
        IntPtr desktop = OpenInputDesktop(0, false, 1);
        if (desktop == IntPtr.Zero) return "OPEN_FAILED:" + Marshal.GetLastWin32Error();
        try {
            var name = new StringBuilder(256);
            uint needed;
            if (!GetUserObjectInformation(desktop, 2, name, name.Capacity, out needed))
                return "READ_FAILED:" + Marshal.GetLastWin32Error();
            return name.ToString();
        } finally { CloseDesktop(desktop); }
    }

    public static int CurrentSessionState() {
        int sessionId = Process.GetCurrentProcess().SessionId;
        IntPtr buffer;
        int bytes;
        if (!WTSQuerySessionInformation(IntPtr.Zero, sessionId, 8, out buffer, out bytes))
            return -Marshal.GetLastWin32Error();
        try { return Marshal.ReadInt32(buffer); }
        finally { WTSFreeMemory(buffer); }
    }

    public static int ForegroundProcessId() {
        uint pid;
        GetWindowThreadProcessId(GetForegroundWindow(), out pid);
        return (int)pid;
    }

    public static string ForegroundProcessName() {
        int pid = ForegroundProcessId();
        if (pid <= 0) return "";
        try { return Process.GetProcessById(pid).ProcessName; }
        catch { return "<exited>"; }
    }

    public static string ForegroundTitle() {
        var title = new StringBuilder(512);
        GetWindowText(GetForegroundWindow(), title, title.Capacity);
        return title.ToString();
    }

    public static IntPtr WindowForProcess(int processId) {
        try { return Process.GetProcessById(processId).MainWindowHandle; }
        catch { return IntPtr.Zero; }
    }

    public static bool IsForegroundProcess(int processId) { return ForegroundProcessId() == processId; }

    public static bool Activate(int processId) {
        IntPtr hwnd = WindowForProcess(processId);
        if (hwnd == IntPtr.Zero) return false;
        ShowWindow(hwnd, 9);
        uint targetPid;
        uint foregroundPid;
        uint targetThread = GetWindowThreadProcessId(hwnd, out targetPid);
        uint foregroundThread = GetWindowThreadProcessId(GetForegroundWindow(), out foregroundPid);
        uint currentThread = GetCurrentThreadId();
        bool attachedForeground = foregroundThread != currentThread && AttachThreadInput(currentThread, foregroundThread, true);
        bool attachedTarget = targetThread != currentThread && AttachThreadInput(currentThread, targetThread, true);
        BringWindowToTop(hwnd);
        SetActiveWindow(hwnd);
        bool activated = SetForegroundWindow(hwnd);
        if (attachedTarget) AttachThreadInput(currentThread, targetThread, false);
        if (attachedForeground) AttachThreadInput(currentThread, foregroundThread, false);
        return activated || IsForegroundProcess(processId);
    }

    public static bool Resize(int processId, int x, int y, int width, int height) {
        IntPtr hwnd = WindowForProcess(processId);
        if (hwnd == IntPtr.Zero) return false;
        return SetWindowPos(hwnd, IntPtr.Zero, x, y, width, height, SWP_NOZORDER | SWP_NOACTIVATE);
    }

    public static void Click(int x, int y) {
        if (!SetCursorPos(x, y)) throw new InvalidOperationException("SetCursorPos failed: " + Marshal.GetLastWin32Error());
        Thread.Sleep(70);
        mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, UIntPtr.Zero);
        Thread.Sleep(60);
        mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, UIntPtr.Zero);
    }

    public static void Press(byte[] keys) {
        foreach (byte key in keys) keybd_event(key, 0, 0, UIntPtr.Zero);
        Thread.Sleep(50);
        for (int i = keys.Length - 1; i >= 0; i--) keybd_event(keys[i], 0, KEYEVENTF_KEYUP, UIntPtr.Zero);
    }

    public static void TypeUnicode(string text) {
        foreach (char character in text) {
            keybd_event(0, (byte)character, 0x0004, UIntPtr.Zero);
            keybd_event(0, (byte)character, 0x0004 | KEYEVENTF_KEYUP, UIntPtr.Zero);
        }
    }

    public static uint DpiForProcess(int processId) {
        IntPtr hwnd = WindowForProcess(processId);
        return hwnd == IntPtr.Zero ? 0 : GetDpiForWindow(hwnd);
    }

    public static string WindowMetrics(int processId) {
        IntPtr hwnd = WindowForProcess(processId);
        if (hwnd == IntPtr.Zero) return "no_main_window";
        Rect outer, client;
        if (!GetWindowRect(hwnd, out outer) || !GetClientRect(hwnd, out client))
            return "window_metrics_failed:" + Marshal.GetLastWin32Error();
        var topLeft = new NativePoint { X = 0, Y = 0 };
        ClientToScreen(hwnd, ref topLeft);
        int clientWidth = client.Right - client.Left;
        int clientHeight = client.Bottom - client.Top;
        return string.Format("{{\"left\":{0},\"top\":{1},\"outer_width\":{2},\"outer_height\":{3},\"client_left\":{4},\"client_top\":{5},\"client_width\":{6},\"client_height\":{7},\"dpi\":{8},\"screen_width\":{9},\"screen_height\":{10}}}",
            outer.Left, outer.Top, outer.Right - outer.Left, outer.Bottom - outer.Top,
            topLeft.X, topLeft.Y, clientWidth, clientHeight, GetDpiForWindow(hwnd),
            GetSystemMetrics(0), GetSystemMetrics(1));
    }
}
'@

function Get-HostState {
    $desktop = [HostDesktopProbe]::InputDesktopName()
    $sessionState = [HostDesktopProbe]::CurrentSessionState()
    [pscustomobject]@{
        input_desktop = $desktop
        session_state = $sessionState
        foreground_pid = [HostDesktopProbe]::ForegroundProcessId()
        foreground_process = [HostDesktopProbe]::ForegroundProcessName()
        foreground_title = [HostDesktopProbe]::ForegroundTitle()
        current_session_id = [System.Diagnostics.Process]::GetCurrentProcess().SessionId
        observed_at = [DateTimeOffset]::Now.ToString('o')
    }
}

function Assert-HostInteractive {
    $state = Get-HostState
    if ($state.input_desktop -ne 'Default') {
        throw "Input desktop is '$($state.input_desktop)'; host is not confirmed interactive."
    }
    if ($state.session_state -ne 0) {
        throw "WTS session state is $($state.session_state), not WTSActive (0)."
    }
    if ($state.foreground_process -ieq 'LockApp' -or $state.foreground_process -ieq 'LogonUI') {
        throw "Foreground desktop is owned by $($state.foreground_process); refusing input/capture."
    }
    return $state
}

function Assert-TargetForeground([int]$TargetPid) {
    $state = Assert-HostInteractive
    if (-not [HostDesktopProbe]::IsForegroundProcess($TargetPid)) {
        throw "Target process $TargetPid is not foreground; actual foreground is $($state.foreground_process) PID $($state.foreground_pid)."
    }
    return $state
}

$state = Assert-HostInteractive
switch ($Action) {
    'probe' {
        $state | ConvertTo-Json -Compress
    }
    'activate' {
        if (-not $ProcessId) { throw 'ProcessId is required.' }
        if (-not [HostDesktopProbe]::Activate($ProcessId)) { throw "Could not activate process $ProcessId." }
        Start-Sleep -Milliseconds 350
        $state = Assert-TargetForeground $ProcessId
        [pscustomobject]@{ action = $Action; state = $state; window = ([HostDesktopProbe]::WindowMetrics($ProcessId) | ConvertFrom-Json) } | ConvertTo-Json -Compress -Depth 4
    }
    'resize' {
        if (-not $ProcessId -or -not $Width -or -not $Height) { throw 'ProcessId, Width, and Height are required.' }
        $state = Assert-TargetForeground $ProcessId
        if (-not [HostDesktopProbe]::Resize($ProcessId, $X, $Y, $Width, $Height)) { throw "SetWindowPos failed for process $ProcessId." }
        Start-Sleep -Milliseconds 400
        $state = Assert-TargetForeground $ProcessId
        [pscustomobject]@{ action = $Action; state = $state; window = ([HostDesktopProbe]::WindowMetrics($ProcessId) | ConvertFrom-Json) } | ConvertTo-Json -Compress -Depth 4
    }
    'key' {
        if (-not $ProcessId -or -not $Keys -or $Keys.Length -eq 0) { throw 'ProcessId and Keys are required.' }
        $state = Assert-TargetForeground $ProcessId
        [HostDesktopProbe]::Press($Keys)
        [pscustomobject]@{ action = $Action; state = $state; keys = @($Keys) } | ConvertTo-Json -Compress -Depth 4
    }
    'desktop-key' {
        if (-not $Keys -or $Keys.Length -eq 0) { throw 'Keys are required.' }
        [HostDesktopProbe]::Press($Keys)
        Start-Sleep -Milliseconds 300
        $state = Assert-HostInteractive
        [pscustomobject]@{ action = $Action; state = $state; keys = @($Keys) } | ConvertTo-Json -Compress -Depth 4
    }
    'click' {
        if (-not $ProcessId) { throw 'ProcessId is required.' }
        $state = Assert-TargetForeground $ProcessId
        [HostDesktopProbe]::Click($X, $Y)
        [pscustomobject]@{ action = $Action; state = $state; x = $X; y = $Y } | ConvertTo-Json -Compress -Depth 4
    }
    'type' {
        if (-not $ProcessId) { throw 'ProcessId is required.' }
        $state = Assert-TargetForeground $ProcessId
        [HostDesktopProbe]::TypeUnicode($Text)
        [pscustomobject]@{ action = $Action; state = $state; text_length = $Text.Length } | ConvertTo-Json -Compress -Depth 4
    }
    'capture' {
        if (-not $ProcessId -or -not $OutputPath) { throw 'ProcessId and OutputPath are required.' }
        $state = Assert-TargetForeground $ProcessId
        Add-Type -AssemblyName System.Drawing
        $hwnd = [HostDesktopProbe]::WindowForProcess($ProcessId)
        $rect = New-Object HostDesktopProbe+Rect
        if (-not [HostDesktopProbe]::GetWindowRect($hwnd, [ref]$rect)) { throw 'GetWindowRect failed.' }
        $width = $rect.Right - $rect.Left
        $height = $rect.Bottom - $rect.Top
        if ($width -le 0 -or $height -le 0) { throw 'Window has invalid bounds.' }
        $path = [System.IO.Path]::GetFullPath($OutputPath)
        [System.IO.Directory]::CreateDirectory([System.IO.Path]::GetDirectoryName($path)) | Out-Null
        $bitmap = New-Object System.Drawing.Bitmap($width, $height)
        try {
            $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
            try {
                $graphics.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bitmap.Size, [System.Drawing.CopyPixelOperation]::SourceCopy)
            } finally { $graphics.Dispose() }
            $bitmap.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
        } finally { $bitmap.Dispose() }
        $state = Assert-TargetForeground $ProcessId
        $metrics = [HostDesktopProbe]::WindowMetrics($ProcessId) | ConvertFrom-Json
        [pscustomobject]@{
            action = $Action
            capture = $path
            sha256 = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
            bytes = (Get-Item -LiteralPath $path).Length
            captured_at = [DateTimeOffset]::Now.ToString('o')
            theme = $env:RB09_CAPTURE_THEME
            state = $state
            window = $metrics
        } | ConvertTo-Json -Compress -Depth 5
    }
}

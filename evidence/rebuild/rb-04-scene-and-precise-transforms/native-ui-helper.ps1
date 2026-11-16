Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;using System.Runtime.InteropServices;using System.Text;
public static class Rb03UiNative {
 [DllImport("user32.dll")] public static extern IntPtr OpenInputDesktop(uint flags,bool inherit,uint access);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern bool GetUserObjectInformation(IntPtr h,int index,StringBuilder name,uint length,out uint needed);
 [DllImport("user32.dll")] public static extern bool CloseDesktop(IntPtr h);
 public static bool Available(){var h=OpenInputDesktop(0,false,1);if(h==IntPtr.Zero)return false;try{var name=new StringBuilder(256);uint n;return GetUserObjectInformation(h,2,name,512,out n)&&name.ToString().Equals("Default",StringComparison.OrdinalIgnoreCase);}finally{CloseDesktop(h);}}
 public delegate bool CB(IntPtr h,IntPtr p);
 [DllImport("user32.dll")] public static extern bool EnumWindows(CB c,IntPtr p);
 public static IntPtr Find(uint pid){IntPtr found=IntPtr.Zero;EnumWindows((h,p)=>{uint id;GetWindowThreadProcessId(h,out id);var t=new StringBuilder(512);GetWindowText(h,t,512);if(id==pid && t.ToString().StartsWith("3D Modeler - ")){found=h;return false;}return true;},IntPtr.Zero);return found;}
 [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h,StringBuilder s,int n);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h,out uint p);
 [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
 [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint a,uint b,bool attach);
 [DllImport("user32.dll")] public static extern bool ShowWindowAsync(IntPtr h,int cmd);
 [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr h);
 [DllImport("user32.dll")] public static extern IntPtr SetActiveWindow(IntPtr h);
 [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
 [DllImport("user32.dll")] public static extern bool SetCursorPos(int x,int y);
 [DllImport("user32.dll")] public static extern void mouse_event(uint flags,uint dx,uint dy,uint data,UIntPtr extra);
 [DllImport("user32.dll")] public static extern void keybd_event(byte key,byte scan,uint flags,UIntPtr extra);
 [DllImport("user32.dll")] public static extern short GetAsyncKeyState(int key);
 [StructLayout(LayoutKind.Sequential)] public struct KEYBDINPUT {public ushort vk,scan;public uint flags,time;public UIntPtr extra;}
 [StructLayout(LayoutKind.Sequential)] public struct MOUSEINPUT {public int x,y;public uint data,flags,time;public UIntPtr extra;}
 [StructLayout(LayoutKind.Explicit)] public struct INPUTUNION {[FieldOffset(0)] public KEYBDINPUT key;[FieldOffset(0)] public MOUSEINPUT mouse;}
 [StructLayout(LayoutKind.Sequential)] public struct INPUT {public uint type;public INPUTUNION data;}
 [DllImport("user32.dll",SetLastError=true)] public static extern uint SendInput(uint count,INPUT[] inputs,int size);
 public static uint Text(string value){
  if(value.Length>2048)throw new ArgumentException("Evidence input exceeds the bounded text limit");
  var inputs=new INPUT[value.Length*2];
  for(int i=0;i<value.Length;i++){
   inputs[i*2].type=1;inputs[i*2].data.key=new KEYBDINPUT{scan=(ushort)value[i],flags=4};
   inputs[i*2+1].type=1;inputs[i*2+1].data.key=new KEYBDINPUT{scan=(ushort)value[i],flags=6};
  }
  return SendInput((uint)inputs.Length,inputs,Marshal.SizeOf(typeof(INPUT)));
 }
 [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h,out RECT r);
 [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h,ref POINT p);
 [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
 [StructLayout(LayoutKind.Sequential)] public struct RECT{public int L,T,R,B;}
 [StructLayout(LayoutKind.Sequential)] public struct POINT{public int X,Y;}
}
"@
$script:rb03AppPid=0
function Get-Rb03AppHandle { if($script:rb03AppPid -gt 0){$p=Get-Process -Id $script:rb03AppPid -ErrorAction Stop}else{$p=@(Get-Process -Name modeler3d-app -ErrorAction Stop);if($p.Count -ne 1){throw 'Exactly one native target process is required'}}; $h=[Rb03UiNative]::Find($p.Id);if($h -eq [IntPtr]::Zero){throw 'Native target window not ready'};return $h }
function Activate-Rb03App {
 if(-not [Rb03UiNative]::Available()){throw 'Input desktop is locked or unavailable; no input or capture sent'}
 $target=Get-Rb03AppHandle; $front=[Rb03UiNative]::GetForegroundWindow(); if($front -eq [IntPtr]::Zero){throw "Desktop unavailable"}
 if($front -ne $target){[uint32]$p=0;$ft=[Rb03UiNative]::GetWindowThreadProcessId($front,[ref]$p);$ct=[Rb03UiNative]::GetCurrentThreadId();$attached=[Rb03UiNative]::AttachThreadInput($ft,$ct,$true);[void][Rb03UiNative]::ShowWindowAsync($target,9);[void][Rb03UiNative]::BringWindowToTop($target);[void][Rb03UiNative]::SetActiveWindow($target);[void][Rb03UiNative]::SetForegroundWindow($target);[void][Rb03UiNative]::AttachThreadInput($ft,$ct,$false);Start-Sleep -Milliseconds 250 }
 if([Rb03UiNative]::GetForegroundWindow() -ne $target){throw '3D Modeler did not become foreground; no input was sent.'}
 return $target
}
function Invoke-Rb03Click([int]$x,[int]$y){$h=Activate-Rb03App;$pt=New-Object Rb03UiNative+POINT;$pt.X=$x;$pt.Y=$y;[void][Rb03UiNative]::ClientToScreen($h,[ref]$pt);[void][Rb03UiNative]::SetCursorPos($pt.X,$pt.Y);Start-Sleep -Milliseconds 80;[Rb03UiNative]::mouse_event(2,0,0,0,[UIntPtr]::Zero);Start-Sleep -Milliseconds 65;[Rb03UiNative]::mouse_event(4,0,0,0,[UIntPtr]::Zero);Start-Sleep -Milliseconds 400}
function Invoke-Rb03Keys([string]$keys){[void](Activate-Rb03App);[System.Windows.Forms.SendKeys]::SendWait($keys);Start-Sleep -Milliseconds 250}
function Get-Rb03WindowInfo([IntPtr]$h){$t=[Text.StringBuilder]::new(512);[void][Rb03UiNative]::GetWindowText($h,$t,$t.Capacity);$r=New-Object Rb03UiNative+RECT;[void][Rb03UiNative]::GetClientRect($h,[ref]$r);$pt=New-Object Rb03UiNative+POINT;$pt.X=0;$pt.Y=0;[void][Rb03UiNative]::ClientToScreen($h,[ref]$pt);return [pscustomobject]@{Handle=$h;Title=$t.ToString();ClientWidth=$r.R-$r.L;ClientHeight=$r.B-$r.T;ScreenX=$pt.X;ScreenY=$pt.Y;Dpi=[Rb03UiNative]::GetDpiForWindow($h)}}
function Capture-Rb03App([string]$relativePath){$h=Activate-Rb03App;$info=Get-Rb03WindowInfo $h;if($info.Title -notlike '3D Modeler - *'){throw "Unexpected capture title: $($info.Title)"};$path=Join-Path (Get-Location) $relativePath;$bmp=[Drawing.Bitmap]::new($info.ClientWidth,$info.ClientHeight);$g=[Drawing.Graphics]::FromImage($bmp);$g.CopyFromScreen($info.ScreenX,$info.ScreenY,0,0,[Drawing.Size]::new($info.ClientWidth,$info.ClientHeight));$bmp.Save($path,[Drawing.Imaging.ImageFormat]::Png);$g.Dispose();$bmp.Dispose();$info|Add-Member -NotePropertyName Path -NotePropertyValue $path;$info|Add-Member -NotePropertyName Sha256 -NotePropertyValue (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant();return $info}





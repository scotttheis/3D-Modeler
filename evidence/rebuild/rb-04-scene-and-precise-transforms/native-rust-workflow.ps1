# Run from the Rust project root against only the current evidence process.
$ErrorActionPreference='Stop'
. "$PSScriptRoot/native-ui-helper.ps1"
$script:rb03AppPid=[int](Get-Content -LiteralPath 'target/rb04-current-native-pid.txt')
$script:rb04EvidenceRoot='evidence/rebuild/rb-04-scene-and-precise-transforms'
function Invoke-Rb04Text([string]$Value){
 [void](Activate-Rb03App)
 foreach($key in @(16,17,18)){if(([Rb03UiNative]::GetAsyncKeyState($key) -band 0x8000) -ne 0){throw 'Modifier held; no text sent'}}
 $sent=[Rb03UiNative]::Text($Value)
 if($sent -ne $Value.Length*2){throw "Native text input was incomplete: $sent events"}
 Start-Sleep -Milliseconds 300
}
function Wait-Rb04Frame {
 $h=Activate-Rb03App;$point=New-Object Rb03UiNative+POINT;$point.X=1000;$point.Y=10
 [void][Rb03UiNative]::ClientToScreen($h,[ref]$point);[void][Rb03UiNative]::SetCursorPos($point.X,$point.Y);Start-Sleep -Milliseconds 400
}
function Capture-Rb04State([string]$Theme,[string]$Action,[string]$Fixture) {
 Wait-Rb04Frame
 $record=Capture-Rb03App "$script:rb04EvidenceRoot/captures/rust-$Theme-$Action.png"
 $bitmap=[Drawing.Bitmap]::new($record.Path)
 $navigation=$bitmap.GetPixel(640,10);$bitmap.Dispose()
 # Modal backdrops and disabled panels lower luminance; the navy channel
 # ratio still separates Dark from the near-neutral Light navigation band.
 $looksDark=$navigation.B -gt $navigation.R*1.3
 if(($Theme -eq 'light' -and $looksDark) -or ($Theme -eq 'dark' -and -not $looksDark)){throw 'Capture theme does not match the requested theme; frame is not accepted'}
 $record|Add-Member Theme $Theme;$record|Add-Member Action $Action;$record|Add-Member Fixture $Fixture
 $record|Add-Member CapturedAt ([DateTimeOffset]::Now.ToString('o'))
 $record|Add-Member BuildSha256 ((Get-FileHash -LiteralPath 'target/debug/modeler3d-app.exe' -Algorithm SHA256).Hash.ToLowerInvariant())
 $journal="$script:rb04EvidenceRoot/rust-current-native-captures.ndjson"
 ConvertTo-Json -InputObject $record -Depth 6 -Compress | Add-Content -LiteralPath $journal -Encoding UTF8
 return $record
}
function Open-Rb04Project([string]$Path) {
 Invoke-Rb03Keys '{ESC}';Wait-Rb04Frame
 Invoke-Rb03Click 30 35;Wait-Rb04Frame
 Invoke-Rb03Click 235 92;Wait-Rb04Frame
 Assert-Rb04FileDialog
 Invoke-Rb03Click 585 405;Invoke-Rb03Keys '^a';Invoke-Rb04Text $Path;Invoke-Rb03Click 484 460
 $expected=[IO.Path]::GetFileName($Path)
 $deadline=[DateTime]::UtcNow.AddSeconds(12)
 while((Get-Rb03WindowInfo (Activate-Rb03App)).Title -ne "3D Modeler - $expected" -or -not (Test-Rb04RootReady)){
  if([DateTime]::UtcNow -ge $deadline){throw "Open did not complete for $expected; no scene input sent"}
  Start-Sleep -Milliseconds 150
 }
 Wait-Rb04Frame
}
function Test-Rb04RootReady {
 $h=Activate-Rb03App;$info=Get-Rb03WindowInfo $h
 $bitmap=[Drawing.Bitmap]::new(1,1);$graphics=[Drawing.Graphics]::FromImage($bitmap)
 $graphics.CopyFromScreen($info.ScreenX+640,$info.ScreenY+10,0,0,[Drawing.Size]::new(1,1))
 $color=$bitmap.GetPixel(0,0);$graphics.Dispose();$bitmap.Dispose()
 return (($color.R -eq 232 -and $color.G -eq 234 -and $color.B -eq 245) -or ($color.R -eq 21 -and $color.G -eq 49 -and $color.B -eq 85))
}
function Select-Rb04SceneIndices([int[]]$Indices,[string]$Kind,[int]$Count){
 foreach($attempt in 1..3){
  Invoke-Rb03Click 1126 283;Wait-Rb04Frame
  $valid=$true;$prefix=@()
  foreach($index in $Indices){
   $x=if($Kind -eq 'unhealthy' -or ($Kind -eq 'group' -and $index -eq 1)){1175}else{1144}
   Invoke-Rb03Click $x (462+35*$index);Wait-Rb04Frame
   Capture-Rb03App 'target/rb04-selection-precondition.png'|Out-Null
   $actual=& 'L:\Builder3D\venv\Scripts\python.exe' -B "$PSScriptRoot/native-selection-state.py" 'target/rb04-selection-precondition.png' $Count|ConvertFrom-Json
   $prefix+=,$index
   if((($actual|Sort-Object)-join ',') -ne (($prefix|Sort-Object)-join ',')){$valid=$false;break}
  }
  if($valid){return}
 }
 throw 'Ordered Items selection did not visibly complete; no model action sent'
}
function Save-Rb04State([string]$Name) {
 $path="E:/3D-Modeler/$script:rb04EvidenceRoot/native-scene-states/$Name.3dp"
 if(Test-Path -LiteralPath $path){throw "Native evidence destination exists: $path"}
 Invoke-Rb03Click 1174 35;Wait-Rb04Frame;Assert-Rb04FileDialog
 Invoke-Rb03Click 585 405;Invoke-Rb03Keys '^a';Invoke-Rb04Text $path;Invoke-Rb03Click 483 460
 Wait-Rb04Frame
 $deadline=[DateTime]::UtcNow.AddSeconds(10)
 while(((-not(Test-Path -LiteralPath $path)) -or (Get-Rb03WindowInfo (Activate-Rb03App)).Title -ne "3D Modeler - $Name.3dp") -and [DateTime]::UtcNow -lt $deadline){
  if(-not [Rb03UiNative]::Available()){throw 'Desktop unavailable while waiting for native Save As; no input sent'}
  Start-Sleep -Milliseconds 200
 }
 if((-not(Test-Path -LiteralPath $path)) -or (Get-Rb03WindowInfo (Activate-Rb03App)).Title -ne "3D Modeler - $Name.3dp"){throw "Native Save As did not finish $path"}
 return $path
}
function Assert-Rb04FileDialog {
 $deadline=[DateTime]::UtcNow.AddSeconds(4)
 do{
  $h=Activate-Rb03App;$info=Get-Rb03WindowInfo $h
  $bitmap=[Drawing.Bitmap]::new(826,501);$graphics=[Drawing.Graphics]::FromImage($bitmap)
  $graphics.CopyFromScreen($info.ScreenX,$info.ScreenY,0,0,[Drawing.Size]::new(826,501))
  $left=$bitmap.GetPixel(455,400);$right=$bitmap.GetPixel(825,400);$header=$bitmap.GetPixel(800,340)
  $panel=($left.R -eq 255 -and $left.G -eq 255 -and $left.B -eq 255) -or ($left.R -eq 13 -and $left.G -eq 29 -and $left.B -eq 58)
  $headerShade=($header.R -ge 44 -and $header.R -le 48) -or ($header.R -ge 219 -and $header.R -le 223)
  $ready=$panel -and $left.ToArgb() -eq $right.ToArgb() -and $header.R -eq $header.G -and $header.G -eq $header.B -and $headerShade
  $graphics.Dispose();$bitmap.Dispose()
  if(-not $ready){Start-Sleep -Milliseconds 150}
 }while(-not $ready -and [DateTime]::UtcNow -lt $deadline)
 if(-not $ready){throw 'Native path dialog is not visibly ready; no path text sent'}
}
function Invoke-Rb04SettleCase([string]$Theme,[int]$Count) {
 $fixture="E:/3D-Modeler/$script:rb04EvidenceRoot/settle-$Count-before.3dp"
 Open-Rb04Project $fixture;Invoke-Rb03Keys '^a';Wait-Rb04Frame
 Capture-Rb04State $Theme "settle-$Count-before" $fixture
 Invoke-Rb03Click 465 128;Capture-Rb04State $Theme "settle-$Count-after" $fixture
 Invoke-Rb03Keys '^z';Capture-Rb04State $Theme "settle-$Count-undo" $fixture
 Invoke-Rb03Keys '^y';Capture-Rb04State $Theme "settle-$Count-redo" $fixture
 $name="rust-$Theme-settle-$Count-redo";$version=2
 while(Test-Path -LiteralPath "$PSScriptRoot/native-scene-states/$name.3dp"){$name="rust-$Theme-settle-$Count-redo-v$version";$version++}
 $saved=Save-Rb04State $name
 @{theme=$Theme;count=$Count;saved=$saved;build_sha256=(Get-FileHash target/debug/modeler3d-app.exe).Hash.ToLowerInvariant()}|ConvertTo-Json -Compress|Add-Content -LiteralPath "$PSScriptRoot/rust-settle-output-routes.ndjson" -Encoding UTF8
}
function Set-Rb04Pointer([int]$X,[int]$Y) {
 $h=Activate-Rb03App;$info=Get-Rb03WindowInfo $h
 if($X -lt 0 -or $X -ge $info.ClientWidth -or $Y -lt 0 -or $Y -ge $info.ClientHeight){throw 'Pointer is outside the native target client'}
 $point=New-Object Rb03UiNative+POINT;$point.X=$X;$point.Y=$Y
 [void][Rb03UiNative]::ClientToScreen($h,[ref]$point);[void][Rb03UiNative]::SetCursorPos($point.X,$point.Y);Start-Sleep -Milliseconds 80
}
function Invoke-Rb04Drag([int[]]$Start,[int[]]$Finish,[string]$Termination='commit') {
 Set-Rb04Pointer $Start[0] $Start[1];[Rb03UiNative]::mouse_event(2,0,0,0,[UIntPtr]::Zero);Start-Sleep -Milliseconds 120
 try {
  if($Termination -ne 'noop'){
   foreach($step in 1..5){Set-Rb04Pointer ([int]($Start[0]+($Finish[0]-$Start[0])*$step/5)) ([int]($Start[1]+($Finish[1]-$Start[1])*$step/5))}
  }
  if($Termination -eq 'escape'){Invoke-Rb03Keys '{ESC}'}
  if($Termination -eq 'focus'){
   $h=Activate-Rb03App;[void][Rb03UiNative]::ShowWindowAsync($h,6);Start-Sleep -Milliseconds 350
   if(-not [Rb03UiNative]::Available()){throw 'Desktop became unavailable; no further input sent'}
   [void][Rb03UiNative]::ShowWindowAsync($h,9);Start-Sleep -Milliseconds 300;[void](Activate-Rb03App)
  }
 } finally {
  if([Rb03UiNative]::Available()){
   if($Termination -eq 'noop'){Set-Rb04Pointer $Start[0] $Start[1]}else{Set-Rb04Pointer $Finish[0] $Finish[1]}
   [Rb03UiNative]::mouse_event(4,0,0,0,[UIntPtr]::Zero);Start-Sleep -Milliseconds 300
  }
 }
}
function Select-Rb04Targets([string]$Kind) {
 Invoke-Rb03Click 1126 283
 Wait-Rb04Frame
 if($Kind -eq 'hidden'){Invoke-Rb03Keys '^a'}else{Invoke-Rb03Click 1144 462}
 Wait-Rb04Frame
 $deadline=[DateTime]::UtcNow.AddSeconds(4)
 do{
  $h=Activate-Rb03App;$info=Get-Rb03WindowInfo $h
  $bitmap=[Drawing.Bitmap]::new(100,1);$graphics=[Drawing.Graphics]::FromImage($bitmap)
  $graphics.CopyFromScreen($info.ScreenX+1120,$info.ScreenY+453,0,0,[Drawing.Size]::new(100,1))
  $selected=$false
  foreach($x in 0..99){
   $color=$bitmap.GetPixel($x,0)
   if(($color.R -eq 225 -and $color.G -eq 230 -and $color.B -eq 255) -or ($color.R -eq 26 -and $color.G -eq 75 -and $color.B -eq 130)){$selected=$true;break}
  }
  $graphics.Dispose();$bitmap.Dispose()
  if(-not $selected){Start-Sleep -Milliseconds 150}
 }while(-not $selected -and [DateTime]::UtcNow -lt $deadline)
 if(-not $selected){throw 'First Items row did not visibly select; no gesture sent'}
}

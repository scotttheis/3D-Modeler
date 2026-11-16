param([string]$Theme='light',[string[]]$Cases=@('move-single','move-hidden','move-group','rotate-single','rotate-group','resize-grid-face','resize-corner','move-noop','move-cancel','move-focus'))
$ErrorActionPreference='Stop'
. "$PSScriptRoot/native-rust-workflow.ps1"
foreach($case in $Cases){
 $name="viewport-$case"
 $fixture="E:/3D-Modeler/$script:rb04EvidenceRoot/native-viewport-fixtures/$name.3dp"
 $mode=($case -split '-')[0]
 $kind=if($case -match 'hidden'){'hidden'}elseif($case -match 'group'){'group'}else{'single'}
 $handle=if($mode -eq 'rotate'){'z'}elseif($mode -eq 'resize'){if($case -match 'corner'){'corner_6'}else{'face_0'}}else{'x'}
 $selected=if($kind -eq 'hidden'){'0,1'}else{'0'}
 Open-Rb04Project $fixture
 Select-Rb04Targets $kind
 $modeX=@{move=492;rotate=580;resize=666}[$mode]
 Invoke-Rb03Click $modeX 738
 Wait-Rb04Frame
 if($mode -eq 'resize'){
  Invoke-Rb03Click 1097 664
  Wait-Rb04Frame
  if($case -match 'object-face'){Invoke-Rb03Click 1220 758}else{Invoke-Rb03Click 1168 758}
  Invoke-Rb03Click 1097 664
  Wait-Rb04Frame
 }
 $arguments=@('-B',"$PSScriptRoot/native_rust_geometry.py",$fixture,'--mode',$mode,'--handle',$handle,'--selection',$selected)
 if($case -match 'object-face'){$arguments+='--basis'}
 $geometry=(& 'L:\Builder3D\venv\Scripts\python.exe' @arguments | Out-String | ConvertFrom-Json)
 Capture-Rb04State $Theme "$name-before" $fixture
 $termination=if($case -match 'noop'){'noop'}elseif($case -match 'cancel'){'escape'}elseif($case -match 'focus'){'focus'}else{'commit'}
 Invoke-Rb04Drag $geometry.start $geometry.end $termination
 Capture-Rb04State $Theme "$name-after" $fixture
 $prefix="rust-$Theme-$name"
 if(Test-Path -LiteralPath "$PSScriptRoot/native-scene-states/$prefix-after.3dp"){$prefix+='-v2'}
 Save-Rb04State "$prefix-after"
 & 'L:\Builder3D\venv\Scripts\python.exe' -B "$PSScriptRoot/verify-native-step.py" $fixture "$PSScriptRoot/native-scene-states/$prefix-after.3dp" $termination
 if($LASTEXITCODE -ne 0){throw "Native result failed for $Theme $case; no undo/redo input sent"}
 if($termination -eq 'commit'){
  Invoke-Rb03Keys '^z'
  Capture-Rb04State $Theme "$name-undo" $fixture
  Save-Rb04State "$prefix-undo"
  Invoke-Rb03Keys '^y'
  Capture-Rb04State $Theme "$name-redo" $fixture
  Save-Rb04State "$prefix-redo"
 }
 $geometry|Add-Member Theme $Theme
 $geometry|Add-Member Case $case
 $geometry|Add-Member Termination $termination
 $geometry|Add-Member OutputPrefix $prefix
 $geometry|ConvertTo-Json -Depth 8 -Compress | Add-Content -LiteralPath "$PSScriptRoot/rust-viewport-pointer-routes.ndjson" -Encoding UTF8
 Write-Output "COMPLETED $Theme $case"
}

param([string]$Theme='dark')
$ErrorActionPreference='Stop'
. "$PSScriptRoot/native-rust-modal-workflow.ps1"
$fixture="E:/Builder3D-Rust/$script:rb04EvidenceRoot/native-scene-fixtures/row-toggle.3dp"
Open-Rb04Project $fixture;Set-Rb04InsertTab
Select-Rb04SceneIndices @() 'base' 3
Invoke-Rb03Click 492 738;Wait-Rb04Frame
Capture-Rb04State $Theme 'pick-before' $fixture|Out-Null
Invoke-Rb03Click 500 450;Wait-Rb04Frame
Capture-Rb04State $Theme 'pick-after' $fixture|Out-Null
$selection=& 'L:\Builder3D\venv\Scripts\python.exe' -B "$PSScriptRoot/native-selection-state.py" "$PSScriptRoot/captures/rust-$Theme-pick-after.png" 3|ConvertFrom-Json
if(($selection -join ',') -ne '0'){throw 'Viewport pick did not select object0; no accepted saved result'}
Save-Rb04State "rust-$Theme-pick-after"
[void](Activate-Rb03App);[Rb03UiNative]::keybd_event(17,0,0,[UIntPtr]::Zero)
try{Invoke-Rb03Click 12 203}finally{if([Rb03UiNative]::Available()){[Rb03UiNative]::keybd_event(17,0,2,[UIntPtr]::Zero)}}
Capture-Rb04State $Theme 'pick-ctrl-empty' $fixture|Out-Null
Invoke-Rb03Click 12 203
Capture-Rb04State $Theme 'pick-clear' $fixture|Out-Null
Save-Rb04State "rust-$Theme-pick-clear"

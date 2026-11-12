param([string]$Theme='light',[string[]]$Cases=@())
$ErrorActionPreference='Stop'
. "$PSScriptRoot/native-rust-workflow.ps1"
$specs=Get-Content -LiteralPath "$PSScriptRoot/native-scene-case-specs.json" -Raw | ConvertFrom-Json
$keepProportions=$false
$sticky=$false
function Click-ModifiedRow([int]$Index,[byte]$Key){
 [void](Activate-Rb03App)
 [Rb03UiNative]::keybd_event($Key,0,0,[UIntPtr]::Zero)
 try{Invoke-Rb03Click 1144 (462+35*$Index)}finally{if([Rb03UiNative]::Available()){[Rb03UiNative]::keybd_event($Key,0,2,[UIntPtr]::Zero)}}
}
foreach($spec in $specs){
 $name=$spec.name
 if($Cases.Count -gt 0 -and $name -notin $Cases){continue}
 if($name -in @('mirror-y','align-z-max','arrange-z-gap5','mirror-cancel','arrange-invalid-cancel','measure','health-warning','health-repair','units-only')){continue}
 $fixture="E:/Builder3D-Rust/$script:rb04EvidenceRoot/$($spec.fixture)"
 Open-Rb04Project $fixture
 if($sticky){Invoke-Rb03Click 1065 435;$sticky=$false}
 $count=if($spec.kind -eq 'unhealthy'){1}elseif($spec.kind -eq 'group'){2}else{3}
 Select-Rb04SceneIndices $spec.selection $spec.kind $count
 Invoke-Rb03Click 492 738
 Capture-Rb04State $Theme "scene-$name-before" $fixture
 switch($name){
  'row-toggle' {Invoke-Rb03Click 1144 497}
  'ctrl-toggle' {Click-ModifiedRow 0 17}
  'shift-range' {Click-ModifiedRow 2 16}
  'select-all' {Invoke-Rb03Click 1082 283}
  'deselect-all' {Invoke-Rb03Click 1126 283}
  'invert' {Invoke-Rb03Click 1180 283}
  'sticky-empty' {Invoke-Rb03Click 1065 435;Invoke-Rb03Click 12 203;$sticky=$true}
  'empty-clear' {Invoke-Rb03Click 12 203}
  'ctrl-empty' {
   [void](Activate-Rb03App);[Rb03UiNative]::keybd_event(17,0,0,[UIntPtr]::Zero)
   try{Invoke-Rb03Click 12 203}finally{if([Rb03UiNative]::Available()){[Rb03UiNative]::keybd_event(17,0,2,[UIntPtr]::Zero)}}
  }
  'duplicate' {Invoke-Rb03Keys '^d'}
  'copy' {Invoke-Rb03Keys '^c'}
  'paste-other-selection' {Invoke-Rb03Keys '^c';Invoke-Rb03Click 1126 283;Invoke-Rb03Click 1144 532;Invoke-Rb03Keys '^v'}
  'cut' {Invoke-Rb03Keys '^x'}
  'delete' {Invoke-Rb03Keys '{DELETE}'}
  'group' {Invoke-Rb03Click 1089 317}
  'ungroup' {Invoke-Rb03Click 1154 317}
  'delete-text-focus' {Invoke-Rb03Click 507 773;Invoke-Rb03Keys '{DELETE}'}
  'numeric-invalid' {Invoke-Rb03Click 507 773;Invoke-Rb03Keys '^a';Invoke-Rb03Keys 'NaN';Invoke-Rb03Click 803 773}
  'rename-reference' {Invoke-Rb03Click 1120 630;Invoke-Rb03Keys '^a';Invoke-Rb03Keys 'Precision part';Invoke-Rb03Click 1225 630}
  'visibility-reference' {Invoke-Rb03Click 1103 497}
  default {
   $mode=if($name -match 'rotate'){'rotate'}elseif($name -match 'resize'){'resize'}else{'move'}
   Invoke-Rb03Click (@{move=492;rotate=580;resize=666}[$mode]) 738
   if($mode -eq 'resize'){
    $desired=$name -match 'locked'
    if($desired -ne $keepProportions){
     $header=if($spec.selection.Count -gt 1){630}else{664}
     Invoke-Rb03Click 1097 $header;Wait-Rb04Frame
     Invoke-Rb03Click 1074 ($header+65)
     Invoke-Rb03Click 1097 $header
     $keepProportions=$desired
    }
   }
   $axis=if($mode -eq 'rotate'){2}elseif($name -in @('resize-locked-inch','multi-move-inch','multi-resize-locked')){1}else{0}
   $value=switch($name){
    'move-mm' {'12 mm'}
    'move-inch' {'1 in'}
    'rotate-degree' {'30 deg'}
    'resize-inch' {'1 in'}
    'resize-locked-inch' {'2 in'}
    'multi-move-inch' {'2 in'}
    'multi-rotate' {'30 deg'}
    'multi-resize-locked' {'60 mm'}
    default {throw "Unknown native case $name"}
   }
   Invoke-Rb03Click (@(507,635,737)[$axis]) 773
   Invoke-Rb03Keys '^a';Invoke-Rb03Keys $value;Invoke-Rb03Click 803 773
  }
 }
 Capture-Rb04State $Theme "scene-$name-after" $fixture
 $prefix="rust-$Theme-scene-$name";$version=2
 while(Test-Path -LiteralPath "$PSScriptRoot/native-scene-states/$prefix-after.3dp"){$prefix="rust-$Theme-scene-$name-v$version";$version++}
 Save-Rb04State "$prefix-after"
 & 'L:\Builder3D\venv\Scripts\python.exe' -B "$PSScriptRoot/verify-native-scenes.py" --step $Theme $name --prefix $prefix
 if($LASTEXITCODE -ne 0){throw "Native scene result failed for $Theme $name; no further input sent"}
 if($name -in @('duplicate','cut','delete','group','ungroup','paste-other-selection','move-mm','move-inch','rotate-degree','resize-inch','resize-locked-inch','multi-move-inch','multi-rotate','multi-resize-locked','rename-reference','visibility-reference')){
  Invoke-Rb03Keys '^z';Capture-Rb04State $Theme "scene-$name-undo" $fixture;Save-Rb04State "$prefix-undo"
  Invoke-Rb03Keys '^y';Capture-Rb04State $Theme "scene-$name-redo" $fixture;Save-Rb04State "$prefix-redo"
 }
 @{theme=$Theme;case=$name;prefix=$prefix;fixture=$fixture;build_sha256=(Get-FileHash target/debug/builder3d-app.exe).Hash.ToLowerInvariant()}|ConvertTo-Json -Compress|Add-Content -LiteralPath "$PSScriptRoot/rust-scene-output-routes.ndjson" -Encoding UTF8
 Write-Output "COMPLETED $Theme scene-$name"
}

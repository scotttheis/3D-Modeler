param([string]$Theme='dark',[string[]]$Cases=@())
$ErrorActionPreference='Stop'
. "$PSScriptRoot/native-rust-modal-workflow.ps1"
foreach($name in @('mirror-y','align-z-max','arrange-z-gap5','mirror-cancel','arrange-invalid-cancel','measure','health-warning','health-repair')){
 if($Cases.Count -gt 0 -and $name -notin $Cases){continue}
 $case=New-Rb04SceneCase $Theme $name
 $case|ConvertTo-Json|Set-Content -LiteralPath 'target/rb04-current-modal-case.json'
 Set-Rb04Tab 'Object'
 $mutation=$name -in @('mirror-y','align-z-max','arrange-z-gap5','health-repair')
 switch($name){
  'mirror-y' {
   Invoke-Rb03Click 535 128;Wait-Rb04Frame
   Invoke-Rb03Click 656 420
   Capture-Rb04State $Theme "scene-$name-dialog-0" $case.Fixture|Out-Null
   Invoke-Rb03Click 602 455
  }
  'mirror-cancel' {
   Invoke-Rb03Click 535 128;Wait-Rb04Frame
   Capture-Rb04State $Theme "scene-$name-dialog-0" $case.Fixture|Out-Null
   Invoke-Rb03Keys '{ESC}'
  }
  'align-z-max' {
   Invoke-Rb03Click 671 128;Wait-Rb04Frame
   Invoke-Rb03Click 646 403
   Capture-Rb04State $Theme "scene-$name-dialog-0" $case.Fixture|Out-Null
   Invoke-Rb03Click 714 437
   Capture-Rb04State $Theme "scene-$name-dialog-1" $case.Fixture|Out-Null
   Invoke-Rb03Click 563 471
  }
  'arrange-z-gap5' {
   Invoke-Rb03Click 742 128;Wait-Rb04Frame
   Invoke-Rb03Click 646 391
   Capture-Rb04State $Theme "scene-$name-dialog-0" $case.Fixture|Out-Null
   Invoke-Rb03Click 620 448;Invoke-Rb03Keys '^a';Invoke-Rb04Text '5 mm'
   Capture-Rb04State $Theme "scene-$name-dialog-1" $case.Fixture|Out-Null
   Invoke-Rb03Click 563 482
  }
  'arrange-invalid-cancel' {
   Invoke-Rb03Click 742 128;Wait-Rb04Frame
   Invoke-Rb03Click 646 391
   Capture-Rb04State $Theme "scene-$name-dialog-0" $case.Fixture|Out-Null
   Invoke-Rb03Click 620 448;Invoke-Rb03Keys '^a';Invoke-Rb04Text '-1 mm'
   Invoke-Rb03Click 563 482;Wait-Rb04Frame
   Capture-Rb04State $Theme "scene-$name-dialog-1" $case.Fixture|Out-Null
   Capture-Rb04State $Theme "scene-$name-rejected-negative" $case.Fixture|Out-Null
   Invoke-Rb03Keys '{ESC}'
  }
  'measure' {
   Invoke-Rb03Click 605 128;Wait-Rb04Frame
   Capture-Rb04State $Theme "scene-$name-dialog-0" $case.Fixture|Out-Null
   Invoke-Rb03Keys '{ESC}'
  }
  'health-warning' { }
  'health-repair' {
   Invoke-Rb03Click 1100 462;Wait-Rb04Frame
   Capture-Rb04State $Theme "scene-$name-dialog-0" $case.Fixture|Out-Null
   Invoke-Rb03Click 495 443;Wait-Rb04Frame
   Capture-Rb04State $Theme "scene-$name-report" $case.Fixture|Out-Null
   Invoke-Rb03Keys '{ESC}'
  }
 }
 Complete-Rb04SceneCase $case $mutation
}

. "$PSScriptRoot/native-rust-workflow.ps1"
function Set-Rb04Tab([string]$Name) {
 # Inspect the painted state: clicking an already active tab can collapse it.
 $x=switch($Name){'Insert'{80};'Object'{145};default{throw 'Unsupported evidence tab'}}
 foreach($attempt in 1..5){
  Wait-Rb04Frame
  $info=Get-Rb03WindowInfo (Activate-Rb03App)
  $bitmap=[Drawing.Bitmap]::new(180,140);$graphics=[Drawing.Graphics]::FromImage($bitmap)
  $graphics.CopyFromScreen($info.ScreenX,$info.ScreenY,0,0,[Drawing.Size]::new(180,140))
  $tab=$bitmap.GetPixel($x,20);$body=$bitmap.GetPixel(10,120)
  $graphics.Dispose();$bitmap.Dispose()
  $selected=($tab.R -eq 255 -and $tab.G -eq 255 -and $tab.B -eq 255) -or ($tab.R -eq 13 -and $tab.G -eq 29 -and $tab.B -eq 58)
  $expanded=($body.R -eq 255 -and $body.G -eq 255 -and $body.B -eq 255) -or ($body.R -eq 13 -and $body.G -eq 29 -and $body.B -eq 58)
  if($selected -and $expanded){return}
  Invoke-Rb03Click $x 35;Wait-Rb04Frame
 }
 throw "Visible $Name ribbon did not become ready; no action sent"
}
function Set-Rb04InsertTab {
 Set-Rb04Tab 'Insert'
}
function New-Rb04SceneCase([string]$Theme,[string]$Name,[bool]$CaptureBefore=$true){
 $specs=Get-Content -LiteralPath "$PSScriptRoot/native-scene-case-specs.json" -Raw|ConvertFrom-Json
 $matching=@($specs|Where-Object {$_.name -eq $Name})
 if($matching.Count -ne 1){throw "Expected one scene case for $Name, found $($matching.Count)"}
 $spec=$matching[0]
 $fixture="E:/3D-Modeler/$script:rb04EvidenceRoot/$($spec.fixture)"
 Open-Rb04Project $fixture
 Set-Rb04InsertTab
 $count=if($spec.kind -eq 'unhealthy'){1}elseif($spec.kind -eq 'group'){2}else{3}
 Select-Rb04SceneIndices $spec.selection $spec.kind $count
 Invoke-Rb03Click 492 738;Wait-Rb04Frame
 $prefix="rust-$Theme-scene-$Name";$version=2
 while(Test-Path -LiteralPath "$PSScriptRoot/native-scene-states/$prefix-after.3dp"){$prefix="rust-$Theme-scene-$Name-v$version";$version++}
 $case=[pscustomobject]@{Theme=$Theme;Name=$Name;Fixture=$fixture;Prefix=$prefix}
 if($CaptureBefore){Capture-Rb04State $Theme "scene-$Name-before" $fixture|Out-Null}
 return $case
}
function Complete-Rb04SceneCase([object]$Case,[bool]$Mutation=$false){
 if($Case.Name -notmatch '^health-'){Set-Rb04InsertTab}
 Capture-Rb04State $Case.Theme "scene-$($Case.Name)-after" $Case.Fixture
 Save-Rb04State "$($Case.Prefix)-after"
 & 'L:\Builder3D\venv\Scripts\python.exe' -B "$PSScriptRoot/verify-native-scenes.py" --step $Case.Theme $Case.Name --prefix $Case.Prefix
 if($LASTEXITCODE -ne 0){throw "Native modal result failed: $($Case.Name)"}
 if($Mutation){
  Invoke-Rb03Keys '^z';Capture-Rb04State $Case.Theme "scene-$($Case.Name)-undo" $Case.Fixture;Save-Rb04State "$($Case.Prefix)-undo"
  Invoke-Rb03Keys '^y';Capture-Rb04State $Case.Theme "scene-$($Case.Name)-redo" $Case.Fixture;Save-Rb04State "$($Case.Prefix)-redo"
 }
 @{theme=$Case.Theme;case=$Case.Name;prefix=$Case.Prefix;fixture=$Case.Fixture;build_sha256=(Get-FileHash target/debug/modeler3d-app.exe).Hash.ToLowerInvariant()}|ConvertTo-Json -Compress|Add-Content -LiteralPath "$PSScriptRoot/rust-scene-output-routes.ndjson" -Encoding UTF8
 Write-Output "COMPLETED $($Case.Theme) modal-$($Case.Name)"
}

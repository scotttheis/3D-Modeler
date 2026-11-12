$ErrorActionPreference = 'Stop'
$sourceRoot = (Resolve-Path -LiteralPath '.').Path.TrimEnd('\')
$timestamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$snapshotRoot = "G:\Builder3D-Rust_snapshot_${timestamp}_RB-04-completed"
if (Test-Path -LiteralPath $snapshotRoot) { throw "Snapshot destination already exists: $snapshotRoot" }
$excludedTarget = Join-Path $sourceRoot 'target'
$excludedCache = Join-Path $sourceRoot '.cache'
$planned = "`r`nPlanned additive completed RB-04 implementation and evidence recovery snapshot (visual/owner acceptance pending): ``$snapshotRoot``. Root target/.cache and reparse entries excluded; exact receipt follows after verification.`r`n"
Add-Content -LiteralPath (Join-Path $sourceRoot 'WORK_LOG.md') -Value $planned -Encoding UTF8
Add-Content -LiteralPath (Join-Path $sourceRoot 'evidence/rebuild/rb-04-scene-and-precise-transforms/checkpoint.md') -Value $planned -Encoding UTF8
$copyArguments = @(
  $sourceRoot,
  $snapshotRoot,
  '/E',
  '/COPY:DAT',
  '/DCOPY:DAT',
  '/R:1',
  '/W:1',
  '/XJ',
  '/NFL',
  '/NDL',
  '/NP',
  '/XD',
  $excludedTarget,
  $excludedCache
)
& robocopy.exe @copyArguments
$copyExitCode = $LASTEXITCODE
if ($copyExitCode -ge 8) { throw "Robocopy failed with exit code $copyExitCode" }

function Get-VerifiedManifest([string]$RootPath) {
  $resolvedRoot = (Resolve-Path -LiteralPath $RootPath).Path.TrimEnd('\')
  $manifest = [System.Collections.Generic.Dictionary[string,string]]::new([StringComparer]::Ordinal)
  $directoryStack = [System.Collections.Generic.Stack[string]]::new()
  $directoryStack.Push($resolvedRoot)
  $totalBytes = [long]0
  $reparseEntries = 0
  $trimChars = [char[]]@(92, 47)
  while ($directoryStack.Count -gt 0) {
    $currentDirectory = $directoryStack.Pop()
    foreach ($entry in [System.IO.Directory]::EnumerateFileSystemEntries($currentDirectory)) {
      $attributes = [System.IO.File]::GetAttributes($entry)
      if (($attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        $reparseEntries++
        continue
      }
      $relativePath = $entry.Substring($resolvedRoot.Length).TrimStart($trimChars).Replace('\', '/')
      if ($relativePath -eq 'target' -or $relativePath.StartsWith('target/', [StringComparison]::Ordinal) -or
          $relativePath -eq '.cache' -or $relativePath.StartsWith('.cache/', [StringComparison]::Ordinal)) {
        continue
      }
      if (($attributes -band [System.IO.FileAttributes]::Directory) -ne 0) {
        $directoryStack.Push($entry)
        continue
      }
      $fileInfo = [System.IO.FileInfo]::new($entry)
      $hashProvider = [System.Security.Cryptography.SHA256]::Create()
      try {
        $stream = [System.IO.File]::OpenRead($entry)
        try { $digest = $hashProvider.ComputeHash($stream) } finally { $stream.Dispose() }
      } finally {
        $hashProvider.Dispose()
      }
      $hashText = [BitConverter]::ToString($digest).Replace('-', '').ToLowerInvariant()
      $manifest[$relativePath] = ('{0}|{1}' -f $fileInfo.Length, $hashText)
      $totalBytes += $fileInfo.Length
    }
  }
  return [pscustomobject]@{ Manifest = $manifest; FileCount = $manifest.Count; Bytes = $totalBytes; ReparseEntries = $reparseEntries }
}

$sourceManifest = Get-VerifiedManifest $sourceRoot
$snapshotManifest = Get-VerifiedManifest $snapshotRoot
$pathDifferences = 0
$sizeHashDifferences = 0
foreach ($relativePath in $sourceManifest.Manifest.Keys) {
  if (-not $snapshotManifest.Manifest.ContainsKey($relativePath)) {
    $pathDifferences++
  } elseif ($sourceManifest.Manifest[$relativePath] -cne $snapshotManifest.Manifest[$relativePath]) {
    $sizeHashDifferences++
  }
}
foreach ($relativePath in $snapshotManifest.Manifest.Keys) {
  if (-not $sourceManifest.Manifest.ContainsKey($relativePath)) { $pathDifferences++ }
}
if (Test-Path -LiteralPath (Join-Path $snapshotRoot 'target')) { throw 'Repository-root target was copied unexpectedly.' }
if (Test-Path -LiteralPath (Join-Path $snapshotRoot '.cache')) { throw 'Repository-root .cache was copied unexpectedly.' }
if ($sourceManifest.FileCount -ne $snapshotManifest.FileCount -or
    $sourceManifest.Bytes -ne $snapshotManifest.Bytes -or
    $pathDifferences -ne 0 -or $sizeHashDifferences -ne 0 -or
    $sourceManifest.ReparseEntries -ne 0 -or $snapshotManifest.ReparseEntries -ne 0) {
  throw 'Snapshot path, byte, SHA-256, or reparse verification failed.'
}

$criticalPaths = @(
 'evidence/rebuild/rb-04-scene-and-precise-transforms/boundary-evidence-validation.json',
 'evidence/rebuild/rb-04-scene-and-precise-transforms/native-scene-verification.json',
 'evidence/rebuild/rb-04-scene-and-precise-transforms/native-viewport-verification.json',
 'evidence/rebuild/rb-04-scene-and-precise-transforms/native-pick-verification.json',
 'evidence/rebuild/rb-04-scene-and-precise-transforms/comparison-pairs.json',
 'evidence/rebuild/rb-04-scene-and-precise-transforms/capture-metadata.json',
 'evidence/rebuild/rb-04-scene-and-precise-transforms/visual-inspection.md',
 'evidence/rebuild/rb-04-scene-and-precise-transforms/final-native-build-artifacts.json',
 'evidence/rebuild/rb-04-scene-and-precise-transforms/final-native-gate-results.json',
 'crates/builder3d-app/src/lib.rs',
 'crates/builder3d-app/src/packaging.rs',
 'packaging/licenses/BOOST-LICENSE-1.0.txt',
 'packaging/licenses/CLIPBOARD-NOTICE.txt',
 'evidence/rebuild/rb-04-scene-and-precise-transforms/clipboard-native-fix.md',
 'evidence/rebuild/rb-04-scene-and-precise-transforms/clipboard-complete-gate-results.json',
 'crates/builder3d-core/src/placement.rs',
 'crates/builder3d-app/src/transform_gesture.rs',
 'crates/builder3d-app/src/gesture_tests.rs',
 'crates/builder3d-app/src/recent_preview.rs',
 'evidence/rebuild/rb-04-scene-and-precise-transforms/native-settle-verification.json',
 'evidence/rebuild/rb-04-scene-and-precise-transforms/python-settle-results.json',
 'evidence/rebuild/rb-04-scene-and-precise-transforms/native_desktop_guard.py',
 'evidence/rebuild/rb-04-scene-and-precise-transforms/native-rust-workflow.ps1',
 'evidence/rebuild/rb-04-scene-and-precise-transforms/rb03-preserved-status-check.json',
 'evidence/rebuild/rb-04-scene-and-precise-transforms/numeric-selection.md',
 'evidence/rebuild/rb-04-scene-and-precise-transforms/numeric-selection-oracle.json',
 'evidence/rebuild/rb-04-scene-and-precise-transforms/placement-oracle.json',
 'crates/builder3d-viewport/src/scene.rs',
 'evidence/rebuild/rb-04-scene-and-precise-transforms/gesture-oracle.json',
 'evidence/rebuild/rb-04-scene-and-precise-transforms/viewport-transforms.md',
 'scripts/Initialize-BuildClang.ps1',
 'docs/building.md',
 'packaging/licenses/QHULL-COPYING.txt',
 'evidence/rebuild/rb-04-scene-and-precise-transforms/settle-fix.md',
 'evidence/rebuild/rb-04-scene-and-precise-transforms/report.md',
 'evidence/rebuild/rb-04-scene-and-precise-transforms/checkpoint.md',
 'evidence/rebuild/rb-04-scene-and-precise-transforms/verification.md',
 'crates/builder3d-app/src/rotation.rs',
 'crates/builder3d-app/src/transforms.rs',
  'AGENTS.md', 'Cargo.toml', 'Cargo.lock', 'WORK_LOG.md', '.git/HEAD',
  'docs/rb-03-lifecycle-and-file-completion-handoff.md',
  'evidence/rebuild/parity-matrix.toml',
  'evidence/rebuild/rb-03-lifecycle-and-file-completion/checkpoint.md',
  'evidence/rebuild/rb-03-lifecycle-and-file-completion/requirement-test-map.md',
  'crates/builder3d-core/src/project.rs',
  'crates/builder3d-core/tests/project_golden.rs',
  'crates/builder3d-app/src/commands.rs',
  'crates/builder3d-app/src/dialogs.rs',
  'crates/builder3d-app/src/shell.rs'
)
$criticalHashes = [ordered]@{}
foreach ($relativePath in $criticalPaths) {
  if (-not $sourceManifest.Manifest.ContainsKey($relativePath)) { throw "Critical file missing from source manifest: $relativePath" }
  $criticalHashes[$relativePath] = ($sourceManifest.Manifest[$relativePath] -split '\|')[1]
}
$receipt = [pscustomobject]@{
  SnapshotPath = $snapshotRoot
  VerifiedAt = [DateTimeOffset]::Now.ToString('yyyy-MM-ddTHH:mm:sszzz')
  RobocopyExitCode = $copyExitCode
  SourceFileCount = $sourceManifest.FileCount
  SnapshotFileCount = $snapshotManifest.FileCount
  SourceBytes = $sourceManifest.Bytes
  SnapshotBytes = $snapshotManifest.Bytes
  RelativePathDifferences = $pathDifferences
  SizeOrSha256Differences = $sizeHashDifferences
  SourceReparseEntries = $sourceManifest.ReparseEntries
  SnapshotReparseEntries = $snapshotManifest.ReparseEntries
  Exclusions = @('repository-root target/', 'repository-root .cache/', 'reparse-point entries')
  RecoveryCriticalSha256 = $criticalHashes
}
$receipt | ConvertTo-Json -Depth 6



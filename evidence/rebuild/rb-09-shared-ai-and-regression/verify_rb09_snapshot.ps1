param(
    [Parameter(Mandatory = $true)][string]$SourceRoot,
    [Parameter(Mandatory = $false)][string]$SnapshotRoot
)

$ErrorActionPreference = 'Stop'
$sourcePath = (Resolve-Path -LiteralPath $SourceRoot).Path.TrimEnd('\')
$sourceExclusions = @(
    (Join-Path $sourcePath 'target').TrimEnd('\'),
    (Join-Path $sourcePath '.cache').TrimEnd('\'),
    (Join-Path $sourcePath 'logs').TrimEnd('\')
)

function Get-IncludedTreeManifest([string]$Root, [string[]]$ExcludedPaths) {
    $files = [System.Collections.Generic.Dictionary[string, object]]::new([System.StringComparer]::Ordinal)
    $reparseCount = 0
    $pendingDirectories = [System.Collections.Generic.Stack[string]]::new()
    $pendingDirectories.Push($Root)

    while ($pendingDirectories.Count -gt 0) {
        $directory = $pendingDirectories.Pop()
        foreach ($item in Get-ChildItem -LiteralPath $directory -Force -ErrorAction Stop) {
            if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
                $reparseCount++
                continue
            }
            if ($item.PSIsContainer) {
                $fullPath = $item.FullName.TrimEnd('\')
                if ($ExcludedPaths -contains $fullPath) {
                    continue
                }
                $pendingDirectories.Push($item.FullName)
                continue
            }

            $relativePath = $item.FullName.Substring($Root.Length).TrimStart('\')
            $hash = (Get-FileHash -LiteralPath $item.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
            $files.Add($relativePath, [pscustomobject]@{
                bytes = [int64]$item.Length
                sha256 = $hash
            })
        }
    }

    $totalBytes = [int64](($files.Values | Measure-Object -Property bytes -Sum).Sum)
    [pscustomobject]@{
        files = $files
        file_count = $files.Count
        total_bytes = $totalBytes
        reparse_entries = $reparseCount
    }
}

$source = Get-IncludedTreeManifest -Root $sourcePath -ExcludedPaths $sourceExclusions
$sourceOnly = [string]::IsNullOrWhiteSpace($SnapshotRoot)
if ($sourceOnly) {
    [pscustomobject]@{
        source = $sourcePath
        excluded_source_directories = $sourceExclusions
        source_files = $source.file_count
        source_bytes = $source.total_bytes
        source_reparse_entries = $source.reparse_entries
        manifest_only = $true
    } | ConvertTo-Json -Depth 4
    return
}

$snapshotPath = (Resolve-Path -LiteralPath $SnapshotRoot).Path.TrimEnd('\')
$snapshot = Get-IncludedTreeManifest -Root $snapshotPath -ExcludedPaths @()

$missing = @($source.files.Keys | Where-Object { -not $snapshot.files.ContainsKey($_) } | Sort-Object)
$extra = @($snapshot.files.Keys | Where-Object { -not $source.files.ContainsKey($_) } | Sort-Object)
$different = [System.Collections.Generic.List[string]]::new()
foreach ($relativePath in $source.files.Keys) {
    if (-not $snapshot.files.ContainsKey($relativePath)) {
        continue
    }
    $sourceFile = $source.files[$relativePath]
    $snapshotFile = $snapshot.files[$relativePath]
    if ($sourceFile.bytes -ne $snapshotFile.bytes -or $sourceFile.sha256 -ne $snapshotFile.sha256) {
        $different.Add($relativePath)
    }
}

$report = [pscustomobject]@{
    source = $sourcePath
    snapshot = $snapshotPath
    excluded_source_directories = $sourceExclusions
    source_files = $source.file_count
    source_bytes = $source.total_bytes
    snapshot_files = $snapshot.file_count
    snapshot_bytes = $snapshot.total_bytes
    missing_count = $missing.Count
    extra_count = $extra.Count
    differing_count = $different.Count
    source_reparse_entries = $source.reparse_entries
    snapshot_reparse_entries = $snapshot.reparse_entries
    verification_pass = ($missing.Count -eq 0 -and $extra.Count -eq 0 -and $different.Count -eq 0 -and $source.reparse_entries -eq 0 -and $snapshot.reparse_entries -eq 0)
    missing_paths = $missing
    extra_paths = $extra
    differing_paths = @($different)
}

$report | ConvertTo-Json -Depth 5
if (-not $report.verification_pass) {
    exit 1
}

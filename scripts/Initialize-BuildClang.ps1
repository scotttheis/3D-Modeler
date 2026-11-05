param([string]$WheelPath = '')
$ErrorActionPreference = 'Stop'
if ($env:OS -ne 'Windows_NT') { throw 'Use the host LLVM/libclang installation on Linux or macOS.' }
$projectRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$toolRoot = Join-Path $projectRoot 'target\build-tools\clang\18.1.1'
$dllPath = Join-Path $toolRoot 'libclang.dll'
$dllHash = 'b13ec96d9a036d6c6c36dd09e00415ead7acc7fcfd40e721bf33ea86b3490f9d'
$wheelHash = '4dd2d3b82fab35e2bf9ca717d7b63ac990a3519c7e312f19fa8e86dcc712f7fb'
if (-not (Test-Path -LiteralPath $dllPath) -or (Get-FileHash -LiteralPath $dllPath -Algorithm SHA256).Hash.ToLowerInvariant() -ne $dllHash) {
    if (-not $WheelPath) {
        $downloadRoot = Join-Path $projectRoot 'target\build-tools\downloads'
        [void](New-Item -ItemType Directory -Force -Path $downloadRoot)
        $WheelPath = Join-Path $downloadRoot 'libclang-18.1.1-py2.py3-none-win_amd64.whl'
        if (-not (Test-Path -LiteralPath $WheelPath)) {
            $metadata = Invoke-RestMethod -Uri 'https://pypi.org/pypi/libclang/18.1.1/json'
            $release = @($metadata.urls | Where-Object { $_.filename -eq 'libclang-18.1.1-py2.py3-none-win_amd64.whl' })
            if ($release.Count -ne 1 -or $release[0].digests.sha256 -ne $wheelHash) { throw 'The pinned libclang release hash does not match.' }
            Invoke-WebRequest -Uri $release[0].url -OutFile $WheelPath -UseBasicParsing
        }
    }
    if ((Get-FileHash -LiteralPath $WheelPath -Algorithm SHA256).Hash.ToLowerInvariant() -ne $wheelHash) { throw 'libclang wheel SHA-256 mismatch.' }
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = [IO.Compression.ZipFile]::OpenRead((Resolve-Path -LiteralPath $WheelPath).Path)
    try {
        $entries = @($archive.Entries | Where-Object { $_.FullName.EndsWith('clang/native/libclang.dll', [StringComparison]::Ordinal) })
        if ($entries.Count -ne 1) { throw 'Expected one libclang.dll in the pinned wheel.' }
        [void](New-Item -ItemType Directory -Force -Path $toolRoot)
        [IO.Compression.ZipFileExtensions]::ExtractToFile($entries[0], $dllPath, $true)
    } finally { $archive.Dispose() }
    if ((Get-FileHash -LiteralPath $dllPath -Algorithm SHA256).Hash.ToLowerInvariant() -ne $dllHash) { throw 'Extracted libclang DLL SHA-256 mismatch.' }
}
$env:LIBCLANG_PATH = $toolRoot
Write-Output "Build-only libclang ready: $toolRoot"

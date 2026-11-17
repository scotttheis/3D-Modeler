param([Parameter(Mandatory = $true)][string]$Repository)
$ErrorActionPreference = 'Stop'
if ($Repository -notmatch '^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$' -or @($Repository.Split('/') | Where-Object { $_ -in @('.', '..') }).Count) {
    throw 'Repository must be owner/repository, without https:// or .git.'
}
$path = Join-Path $PSScriptRoot 'update-feed.json'
$content = @{ repository = $Repository } | ConvertTo-Json
[IO.File]::WriteAllText($path, $content + [Environment]::NewLine, [Text.UTF8Encoding]::new($false))
Write-Host "Configured https://github.com/$Repository/releases. Rebuild every platform before publishing."

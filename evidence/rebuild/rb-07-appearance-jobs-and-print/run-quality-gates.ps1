Set-Location 'E:\3D-Modeler'
. .\scripts\Initialize-BuildClang.ps1
$ErrorActionPreference = 'Continue'

$outputDirectory = Join-Path $PSScriptRoot 'quality-gates'
New-Item -ItemType Directory -Force -Path $outputDirectory | Out-Null
$resultsPath = Join-Path $outputDirectory 'results.tsv'
"gate`tstatus`tlog" | Set-Content -LiteralPath $resultsPath -Encoding utf8

function Invoke-Rb07Gate([string]$Name, [scriptblock]$Command) {
    $logPath = Join-Path $outputDirectory ($Name + '.txt')
    & $Command *> $logPath
    $exitCode = $LASTEXITCODE
    if ($null -eq $exitCode) { $exitCode = 0 }
    $status = if ($exitCode -eq 0) { 'PASS' } else { "FAIL($exitCode)" }
    "$Name`t$status`t$([IO.Path]::GetFileName($logPath))" | Add-Content -LiteralPath $resultsPath -Encoding utf8
    Write-Host "$Name : $status"
}

Invoke-Rb07Gate 'cargo-fmt' { cargo fmt --all -- --check }
Invoke-Rb07Gate 'cargo-check-workspace' { cargo check --workspace --locked }
Invoke-Rb07Gate 'cargo-test-workspace' { cargo test --workspace --locked }
Invoke-Rb07Gate 'cargo-clippy' { cargo clippy --workspace --all-targets --all-features --locked -- -D warnings }
Invoke-Rb07Gate 'cargo-deny' { cargo deny check }
Invoke-Rb07Gate 'cargo-metadata' { cargo metadata --locked --format-version 1 --no-deps }
Invoke-Rb07Gate 'cargo-smoke' { cargo run -p modeler3d-app --bin modeler3d-app -- --smoke }
Invoke-Rb07Gate 'cargo-build-release' { cargo build --release --locked -p modeler3d-app --bin modeler3d-app }
Invoke-Rb07Gate 'cargo-bloat-workspace' { cargo bloat --release --workspace }
Invoke-Rb07Gate 'cargo-bloat-app' { cargo bloat --release -p modeler3d-app --bin modeler3d-app }

Write-Host "Results: $resultsPath"

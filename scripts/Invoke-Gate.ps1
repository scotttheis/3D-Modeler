<#
.SYNOPSIS
    Runs a Builder3D Rust gate tier (plan section 6) with one command.

.DESCRIPTION
    Tiers
      fast  fmt; clippy and tests on the changed crates; cargo deny and
            cargo audit when Cargo.toml or Cargo.lock changed.
      full  fmt; clippy and tests across the workspace; cargo deny; cargo
            audit; the MCP catalog parity check; --smoke; the capture board
            (every review state in both themes).
      ship  full, then a locked release build and cargo bloat. Package staging
            and the performance run are not scripted and are reported as
            "Not run".

    The script loads the build-only libclang first (scripts\Initialize-BuildClang.ps1),
    runs every step even after a failure (steps that depend on a failed build
    are reported "Not run"), prints a result table, writes each step's output
    to logs\gate\<timestamp>\ (ignored by Git), and exits non-zero if any step
    failed. A step that could not run is reported "Not run", never as a pass.

.PARAMETER Tier
    fast, full or ship.

.PARAMETER Crates
    Fast tier only: package names to check. Default: crates changed in the
    working tree against HEAD, or in HEAD against HEAD~1 when the tree is clean.

.PARAMETER SkipCaptures
    Full and ship tiers: skip the capture board (reported "Not run").

.EXAMPLE
    .\scripts\Invoke-Gate.ps1 -Tier fast
    .\scripts\Invoke-Gate.ps1 -Tier full
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('fast', 'full', 'ship')]
    [string]$Tier,

    [string[]]$Crates = @(),

    [switch]$SkipCaptures
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$logDir = Join-Path $root "logs\gate\$stamp-$Tier"
New-Item -ItemType Directory -Force $logDir | Out-Null

$results = New-Object System.Collections.Generic.List[object]

function Add-Result([string]$Name, [string]$State, [double]$Seconds, [string]$Note) {
    $results.Add([pscustomobject]@{
            Step    = $Name
            Result  = $State
            Seconds = [math]::Round($Seconds, 0)
            Note    = $Note
        })
}

# Runs a command line through cmd.exe so that stderr is merged without
# PowerShell wrapping it in error records. Returns $true on exit code 0.
function Invoke-Step([string]$Name, [string]$CommandLine) {
    $log = Join-Path $logDir (($Name -replace '[^A-Za-z0-9]+', '-').Trim('-') + '.log')
    Write-Host ""
    Write-Host "=== $Name" -ForegroundColor Cyan
    Write-Host "    $CommandLine"
    $started = Get-Date
    & cmd.exe /c "$CommandLine 2>&1" | Tee-Object -FilePath $log | ForEach-Object { Write-Host $_ }
    $code = $LASTEXITCODE
    $seconds = ((Get-Date) - $started).TotalSeconds
    if ($code -eq 0) {
        Add-Result $Name 'Pass' $seconds ''
        return $true
    }
    Add-Result $Name 'FAIL' $seconds "exit $code, see $log"
    return $false
}

function Skip-Step([string]$Name, [string]$Reason) {
    Write-Host ""
    Write-Host "=== $Name -- not run: $Reason" -ForegroundColor Yellow
    Add-Result $Name 'Not run' 0 $Reason
}

function Test-CargoTool([string]$Subcommand) {
    & cmd.exe /c "cargo $Subcommand --version >nul 2>&1"
    return ($LASTEXITCODE -eq 0)
}

# libclang for the QHull build dependency, in this session.
. (Join-Path $PSScriptRoot 'Initialize-BuildClang.ps1') | Out-Null

# --- changed crates (fast tier) ---------------------------------------------
$lockChanged = $false
if ($Tier -eq 'fast' -and $Crates.Count -eq 0) {
    $paths = @(& git -c core.safecrlf=false diff --name-only HEAD) + @(& git ls-files --others --exclude-standard)
    $paths = @($paths | Where-Object { $_ })
    $source = 'working tree against HEAD'
    if ($paths.Count -eq 0) {
        $paths = @(& git -c core.safecrlf=false diff --name-only HEAD~1 HEAD)
        $source = 'HEAD against HEAD~1'
    }
    $found = @{}
    foreach ($path in $paths) {
        if ($path -match '^crates/([^/]+)/') { $found[$Matches[1]] = $true }
        if ($path -in @('Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', 'deny.toml')) {
            $lockChanged = $true
            foreach ($dir in Get-ChildItem (Join-Path $root 'crates') -Directory) { $found[$dir.Name] = $true }
        }
        if ($path -match '(^|/)Cargo\.(toml|lock)$') { $lockChanged = $true }
    }
    $Crates = @($found.Keys | Sort-Object)
    Write-Host "Changed crates ($source): $($Crates -join ', ')"
}

$fullScope = ($Tier -ne 'fast')

# --- formatting -------------------------------------------------------------
[void](Invoke-Step 'fmt' 'cargo fmt --all -- --check')

# --- clippy and tests -------------------------------------------------------
if ($fullScope) {
    $clippyOk = Invoke-Step 'clippy (workspace)' 'cargo clippy --workspace --all-targets --locked -- -D warnings'
    $testOk = Invoke-Step 'test (workspace)' 'cargo test --workspace --locked --no-fail-fast'
} elseif ($Crates.Count -gt 0) {
    $packages = ($Crates | ForEach-Object { "-p $_" }) -join ' '
    $clippyOk = Invoke-Step 'clippy (changed crates)' "cargo clippy $packages --all-targets --locked -- -D warnings"
    $testOk = Invoke-Step 'test (changed crates)' "cargo test $packages --locked --no-fail-fast"
} else {
    Skip-Step 'clippy (changed crates)' 'no crate changed'
    Skip-Step 'test (changed crates)' 'no crate changed'
}

# --- supply chain -----------------------------------------------------------
if ($fullScope -or $lockChanged) {
    if (Test-CargoTool 'deny') {
        [void](Invoke-Step 'cargo deny' 'cargo deny check')
    } else {
        Skip-Step 'cargo deny' 'cargo-deny is not installed'
    }
    if (Test-CargoTool 'audit') {
        [void](Invoke-Step 'cargo audit' 'cargo audit')
    } else {
        Skip-Step 'cargo audit' 'cargo-audit is not installed'
    }
}

# --- full-tier extras -------------------------------------------------------
if ($fullScope) {
    [void](Invoke-Step 'MCP catalog parity' 'cargo test -p builder3d-mcp --test ledger_parity --locked')
    [void](Invoke-Step 'smoke' 'cargo run -p builder3d-app --bin builder3d-app --locked -- --smoke')

    if ($SkipCaptures) {
        Skip-Step 'capture board' '-SkipCaptures'
    } else {
        $built = Invoke-Step 'build review profile' 'cargo build --profile review -p builder3d-app --locked'
        if (-not $built) {
            Skip-Step 'capture board' 'review build failed'
        } else {
            $exe = Join-Path $root 'target\review\builder3d-app.exe'
            $boardDir = Join-Path $root "evidence\rebuild\captures\gate-$stamp"
            New-Item -ItemType Directory -Force $boardDir | Out-Null
            $states = @('welcome', 'empty', 'move', 'rotate', 'resize', 'resize-drag', 'multi', 'merge', 'unselected', 'material', 'settings', 'save', 'export', 'menu', 'colors', 'mirror', 'merge-apart', 'repair', 'paste', 'align', 'paint', 'paint-picker', 'xray', 'texture', 'texture-dialog', 'grab', 'crease', 'smooth-brush', 'simplify', 'subdivide', 'hollow')
            $started = Get-Date
            $missing = @()
            Write-Host ""
            Write-Host "=== capture board ($($states.Count) states x 2 themes)" -ForegroundColor Cyan
            foreach ($theme in @('light', 'dark')) {
                foreach ($state in $states) {
                    $png = Join-Path $boardDir "$state-$theme.png"
                    $env:BUILDER3D_REVIEW_CAPTURE = $png
                    $env:BUILDER3D_REVIEW_STATE = $state
                    $env:BUILDER3D_REVIEW_THEME = $theme
                    & $exe | Out-Null
                    if (-not (Test-Path $png)) { $missing += "$state-$theme" }
                }
            }
            Remove-Item Env:BUILDER3D_REVIEW_CAPTURE, Env:BUILDER3D_REVIEW_STATE, Env:BUILDER3D_REVIEW_THEME -ErrorAction SilentlyContinue
            $seconds = ((Get-Date) - $started).TotalSeconds
            if ($missing.Count -eq 0) {
                Add-Result 'capture board' 'Pass' $seconds "images only (not owner review): $boardDir"
            } else {
                Add-Result 'capture board' 'FAIL' $seconds ("missing: " + ($missing -join ', '))
            }
        }
    }
}

# --- ship-tier extras -------------------------------------------------------
if ($Tier -eq 'ship') {
    [void](Invoke-Step 'release build' 'cargo build --release --locked -p builder3d-app')
    if (Test-CargoTool 'bloat') {
        [void](Invoke-Step 'cargo bloat' 'cargo bloat --release -p builder3d-app --crates')
    } else {
        Skip-Step 'cargo bloat' 'cargo-bloat is not installed'
    }
    Skip-Step 'package staging and measurement' 'not scripted'
    Skip-Step 'performance run' 'not scripted'
}

# --- summary ----------------------------------------------------------------
Write-Host ""
Write-Host "Gate '$Tier' on $(Get-Date -Format 'yyyy-MM-dd HH:mm')" -ForegroundColor Cyan
$results | Format-Table Step, Result, Seconds, Note -AutoSize -Wrap | Out-String | Write-Host
Write-Host "Logs: $logDir"

$failed = @($results | Where-Object { $_.Result -eq 'FAIL' })
if ($failed.Count -gt 0) {
    Write-Host "GATE FAILED: $($failed.Count) step(s)" -ForegroundColor Red
    exit 1
}
$notRun = @($results | Where-Object { $_.Result -eq 'Not run' })
if ($notRun.Count -gt 0) {
    Write-Host "Gate passed; $($notRun.Count) step(s) NOT RUN (above)." -ForegroundColor Yellow
} else {
    Write-Host "GATE PASSED" -ForegroundColor Green
}
exit 0

#Requires -Version 7
<#
.SYNOPSIS
    Checks, tests and builds Spectacle, then compiles its installer.

.DESCRIPTION
    Runs rustfmt and Clippy, the unit tests, an optimized release build and
    Inno Setup. The installer is written to dist\Spectacle-<version>-Setup.exe.

.PARAMETER SkipChecks
    Skips formatting, linting and tests, for a quicker installer rebuild.
#>
param([switch]$SkipChecks)

$ErrorActionPreference = 'Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)

function Invoke-Step([string]$Name, [scriptblock]$Command) {
    Write-Host "==> $Name" -ForegroundColor Cyan
    & $Command
    if ($LASTEXITCODE -ne 0) { throw "$Name failed with exit code $LASTEXITCODE" }
}

$cargo = (Get-Command cargo -ErrorAction SilentlyContinue)?.Source ?? (Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe')
if (-not (Test-Path $cargo)) { throw 'Rust is not installed; see "Building" in README.md.' }

if (-not $SkipChecks) {
    Invoke-Step 'Checking formatting' { & $cargo fmt --check }
    Invoke-Step 'Linting' { & $cargo clippy --all-targets --release -- -D warnings }
    Invoke-Step 'Running tests' { & $cargo test }
}
Invoke-Step 'Building the release executable' { & $cargo build --release }

$iscc = @(
    (Get-Command ISCC.exe -ErrorAction SilentlyContinue)?.Source,
    (Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 7\ISCC.exe'),
    (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 7\ISCC.exe'),
    (Join-Path $env:ProgramFiles 'Inno Setup 7\ISCC.exe')
) | Where-Object { $_ -and (Test-Path $_) } | Select-Object -First 1
if (-not $iscc) { throw 'Inno Setup 7 is not installed; see "Building" in README.md.' }
Invoke-Step 'Compiling the installer' { & $iscc /Q installer\Spectacle.iss }

Get-ChildItem dist\*.exe | ForEach-Object { Write-Host "Installer: $($_.FullName)" -ForegroundColor Green }

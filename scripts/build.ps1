#Requires -Version 7
<#
.SYNOPSIS
    Checks, tests and builds Spectacle, then compiles its installer.

.DESCRIPTION
    Runs rustfmt and Clippy, the unit tests, an optimized release build and
    Inno Setup. The installer is written to dist\Spectacle-<version>-Setup.exe.

    When a code signing certificate is available, Spectacle.exe, the setup
    program and its uninstaller are signed with it and timestamped, so that
    Windows shows the publisher instead of "Unknown publisher". By default the
    newest valid code signing certificate issued to the publisher in the
    current user's certificate store is used.

.PARAMETER SkipChecks
    Skips formatting, linting and tests, for a quicker installer rebuild.

.PARAMETER CertificateThumbprint
    The thumbprint of the code signing certificate to sign with, in the
    current user's certificate store. Defaults to the
    SPECTACLE_SIGNING_CERTIFICATE environment variable, then to the newest
    certificate issued to -Publisher.

.PARAMETER Publisher
    The subject name of the certificate to look for when no thumbprint is
    given.

.PARAMETER TimestampServer
    The RFC 3161 time stamping service that dates the signatures, which keeps
    them valid after the certificate expires.

.PARAMETER Unsigned
    Builds without signing even if a certificate is available.
#>
param(
    [switch]$SkipChecks,
    [string]$CertificateThumbprint = $env:SPECTACLE_SIGNING_CERTIFICATE,
    [string]$Publisher = 'Kianoush Akbari',
    [string]$TimestampServer = 'http://timestamp.digicert.com',
    [switch]$Unsigned
)

$ErrorActionPreference = 'Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)

function Invoke-Step([string]$Name, [scriptblock]$Command) {
    Write-Host "==> $Name" -ForegroundColor Cyan
    & $Command
    if ($LASTEXITCODE -ne 0) { throw "$Name failed with exit code $LASTEXITCODE" }
}

$cargo = (Get-Command cargo -ErrorAction SilentlyContinue)?.Source ?? (Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe')
if (-not (Test-Path $cargo)) { throw 'Rust is not installed; see "Building" in README.md.' }

$iscc = @(
    (Get-Command ISCC.exe -ErrorAction SilentlyContinue)?.Source,
    (Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 7\ISCC.exe'),
    (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 7\ISCC.exe'),
    (Join-Path $env:ProgramFiles 'Inno Setup 7\ISCC.exe')
) | Where-Object { $_ -and (Test-Path $_) } | Select-Object -First 1
if (-not $iscc) { throw 'Inno Setup 7 is not installed; see "Building" in README.md.' }

# Find the certificate and the Windows SDK's signing tool.
$certificate = $null
if (-not $Unsigned) {
    $codeSigning = '1.3.6.1.5.5.7.3.3'
    $candidates = Get-ChildItem Cert:\CurrentUser\My | Where-Object {
        $_.HasPrivateKey -and $_.NotAfter -gt (Get-Date) -and $_.EnhancedKeyUsageList.ObjectId -contains $codeSigning
    }
    $certificate = if ($CertificateThumbprint) {
        $candidates | Where-Object Thumbprint -eq $CertificateThumbprint.Replace(' ', '').ToUpperInvariant()
    } else {
        $candidates | Where-Object Subject -eq "CN=$Publisher" | Sort-Object NotAfter -Descending | Select-Object -First 1
    }
    if ($CertificateThumbprint -and -not $certificate) {
        throw "No valid code signing certificate with thumbprint $CertificateThumbprint was found in the current user's store."
    }
}
$signtool = $null
if ($certificate) {
    $signtool = Get-ChildItem (Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin\*\x64\signtool.exe') -ErrorAction SilentlyContinue |
        Sort-Object { [version]$_.Directory.Parent.Name } -Descending | Select-Object -First 1 -ExpandProperty FullName
    if (-not $signtool) { throw 'signtool.exe was not found; install the Windows SDK, see "Building" in README.md.' }
    Write-Host "Signing as $($certificate.Subject) ($($certificate.Thumbprint))" -ForegroundColor Cyan
} else {
    Write-Host 'No code signing certificate found; building unsigned.' -ForegroundColor Yellow
}

if (-not $SkipChecks) {
    Invoke-Step 'Checking formatting' { & $cargo fmt --check }
    Invoke-Step 'Linting' { & $cargo clippy --all-targets --release -- -D warnings }
    Invoke-Step 'Running tests' { & $cargo test }
}
Invoke-Step 'Building the release executable' { & $cargo build --release }

$executable = 'target\x86_64-pc-windows-msvc\release\Spectacle.exe'
if ($certificate) {
    $signArguments = @('sign', '/sha1', $certificate.Thumbprint, '/fd', 'sha256', '/tr', $TimestampServer, '/td', 'sha256', '/d', 'Spectacle')
    Invoke-Step 'Signing Spectacle.exe' { & $signtool @signArguments $executable }
}

$innoArguments = @('/Q')
if ($certificate) {
    # Inno Setup signs the setup program and the uninstaller it contains;
    # $f is the file to sign and $q a quotation mark, which keeps the command
    # a single command-line argument.
    $command = "`$q$signtool`$q sign /sha1 $($certificate.Thumbprint) /fd sha256 /tr $TimestampServer /td sha256 /d `$qSpectacle`$q `$f"
    $innoArguments += @('/DSignToolName=spectacle', "/Sspectacle=$command")
}
Invoke-Step 'Compiling the installer' { & $iscc @innoArguments installer\Spectacle.iss }

$setup = Get-ChildItem dist\*.exe | Sort-Object LastWriteTime -Descending | Select-Object -First 1
if ($certificate) {
    foreach ($file in @($executable, $setup.FullName)) {
        $signature = Get-AuthenticodeSignature $file
        if ($signature.SignerCertificate.Thumbprint -ne $certificate.Thumbprint) {
            throw "$file is not signed with the expected certificate."
        }
        Write-Host "Signed: $file ($($signature.Status))" -ForegroundColor Green
    }
}
Write-Host "Installer: $($setup.FullName)" -ForegroundColor Green

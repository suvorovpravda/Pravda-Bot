#Requires -Version 7.0
<#
.SYNOPSIS
  Windows hosted-runner MSI clean-install and uninstall acceptance test.
.DESCRIPTION
  Never executes the trading application, contacts an exchange, or modifies
  existing installations. Run only in a disposable Windows test environment.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$MsiPath
)

$ErrorActionPreference = 'Stop'
$msi = (Resolve-Path -LiteralPath $MsiPath -ErrorAction Stop).Path
if ([IO.Path]::GetExtension($msi) -ne '.msi') {
    throw 'An MSI installer is required'
}
$msiexec = Join-Path $env:SystemRoot 'System32/msiexec.exe'
$log = Join-Path $env:RUNNER_TEMP 'pravda-msi-install.log'
$uninstallPaths = @(
    'Registry::HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*',
    'Registry::HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*',
    'Registry::HKEY_CURRENT_USER\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*'
)
function Get-PravdaInstall {
    $matches = @(
        Get-ItemProperty -Path $uninstallPaths -ErrorAction SilentlyContinue |
            Where-Object { $_.DisplayName -eq 'Pravda-Bot' }
    )
    return $matches
}

if (@(Get-PravdaInstall).Count -ne 0) {
    throw 'Clean-install smoke test requires a pristine Windows runner'
}

$installedCode = $null
$testSucceeded = $false
$canary = Join-Path $env:APPDATA 'com.pravda.bot/ci-retention-canary.txt'
$canaryContents = 'Pravda MSI retention canary - do not remove user data on uninstall'
if (Test-Path -LiteralPath $canary) { throw 'Cannot overwrite pre-existing test canary' }
try {
    $process = Start-Process -FilePath $msiexec -ArgumentList @(
        '/i', ('"' + $msi + '"'), '/qn', '/norestart',
        '/L*v', ('"' + $log + '"')
    ) -PassThru -Wait
    if ($process.ExitCode -ne 0) {
        throw "MSI silent install returned exit code $($process.ExitCode)"
    }
    $registration = @(Get-PravdaInstall)
    if ($registration.Count -ne 1) {
        throw "Expected exactly one installed Pravda-Bot registration, got $($registration.Count)"
    }
    $installedCode = [string]$registration[0].PSChildName
    if ($installedCode -notmatch '^\{[A-Fa-f0-9-]{36}\}$') {
        throw 'MSI installation registration lacks a product GUID'
    }
    $roots = @(
        [string]$registration[0].InstallLocation,
        (Join-Path $env:ProgramFiles 'Pravda-Bot'),
        (Join-Path $env:LOCALAPPDATA 'Programs/Pravda-Bot')
    ) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Select-Object -Unique
    $found = $false
    foreach ($folder in $roots) {
        if (-not (Test-Path -LiteralPath $folder -PathType Container)) { continue }
        $executables = @(Get-ChildItem -LiteralPath $folder -Filter '*.exe' -File -Recurse -ErrorAction Stop)
        if (@($executables | Where-Object {
            $_.Name -in @('pravda-app.exe', 'Pravda-Bot.exe')
        }).Count -gt 0) {
            $found = $true
            break
        }
    }
    if (-not $found) {
        throw 'MSI registration exists but its application executable was not found'
    }
    # This sentinel represents an existing per-user file; the installer must not
    # delete the Tauri app data directory on uninstall. It does not replace a
    # full GUI/database persistence acceptance test.
    New-Item -Path (Split-Path -Parent $canary) -ItemType Directory -Force | Out-Null
    Set-Content -LiteralPath $canary -Value $canaryContents -Encoding utf8
    Write-Host 'PASS: silent MSI install, registered product and Windows application binary'
    $testSucceeded = $true
}
finally {
    if (-not $installedCode) {
        $candidates = @(Get-PravdaInstall)
        if ($candidates.Count -eq 1) { $installedCode = [string]$candidates[0].PSChildName }
    }
    if ($installedCode -and $installedCode -match '^\{[A-Fa-f0-9-]{36}\}$') {
        $remove = Start-Process -FilePath $msiexec -ArgumentList @(
            '/x', $installedCode, '/qn', '/norestart'
        ) -Wait -PassThru
        if ($remove.ExitCode -ne 0) {
            throw "MSI uninstall failed with exit code $($remove.ExitCode)"
        }
        if (@(Get-PravdaInstall).Count -ne 0) {
            throw 'MSI product remains registered after uninstall'
        }
        Write-Host 'PASS: clean MSI uninstall and registry removal'
        if ($testSucceeded) {
            if (-not (Test-Path -LiteralPath $canary -PathType Leaf) -or
                (Get-Content -LiteralPath $canary -Raw).TrimEnd() -cne $canaryContents) {
                throw 'MSI uninstall removed or altered existing per-user app data'
            }
            Write-Host 'PASS: existing per-user app data survives MSI uninstall'
        }
    }
    if (Test-Path -LiteralPath $canary -PathType Leaf) {
        Remove-Item -LiteralPath $canary -Force -ErrorAction SilentlyContinue
    }
    # Retain verbose MSI log on ANY smoke failure for the CI failure artifact.
    # The Windows hosted runner has no broker or code-signing secrets.
    if ($testSucceeded) {
        Remove-Item -LiteralPath $log -Force -ErrorAction SilentlyContinue
    } else {
        Write-Warning 'MSI smoke failed; retaining verbose log for protected CI artifact'
    }
}

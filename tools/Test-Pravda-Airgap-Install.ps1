#Requires -Version 7.0
<#
.SYNOPSIS
  Qualify one Windows MSI or NSIS on a disposable offline VM with WebView2 absent.
.DESCRIPTION
  Acceptance gate only, not an installer for normal users.
  Restore a clean VM snapshot separately for each MSI and NSIS test.
  The hypervisor must disable all network access BEFORE starting this script.
  This script never changes firewall, UAC, SmartScreen or network configuration.
.EXAMPLE
  ./tools/Test-Pravda-Airgap-Install.ps1 -InstallerPath ./Pravda-Bot_x64.msi -Kind MSI -ExpectedSha256 HASH_FROM_TRUSTED_MANIFEST -DisposableVm
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$InstallerPath,
    [Parameter(Mandatory = $true)]
    [ValidateSet('MSI', 'NSIS')]
    [string]$Kind,
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[a-fA-F0-9]{64}$')]
    [string]$ExpectedSha256,
    [Parameter(Mandatory = $true)]
    [switch]$DisposableVm
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if (-not $DisposableVm) {
    throw 'REFUSED: disposable-VM acknowledgement required'
}
if ($env:OS -ne 'Windows_NT') {
    throw 'REFUSED: Windows VM required'
}
if (-not (Get-Command Get-NetAdapter -ErrorAction SilentlyContinue)) {
    throw 'REFUSED: cannot verify disconnected network adapters'
}
$principal = [Security.Principal.WindowsPrincipal]::new(
    [Security.Principal.WindowsIdentity]::GetCurrent()
)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'REFUSED: elevate inside the disposable VM using normal UAC consent'
}
# Conservative additional check: actual VM network isolation belongs to the hypervisor.
if (@(Get-NetAdapter -ErrorAction Stop | Where-Object { $_.Status -eq 'Up' }).Count -ne 0) {
    throw 'REFUSED: network adapter still connected'
}
if (@(Get-NetRoute -ErrorAction Stop | Where-Object {
    $_.DestinationPrefix -in @('0.0.0.0/0', '::/0')
}).Count -ne 0) {
    throw 'REFUSED: a default network route is present'
}

function Get-WebView2EvergreenVersion {
    $guid = '{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
    foreach ($key in @(
        "HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\$guid",
        "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\$guid",
        "HKCU:\SOFTWARE\Microsoft\EdgeUpdate\Clients\$guid",
        "HKCU:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\$guid"
    )) {
        $value = Get-ItemProperty -LiteralPath $key -ErrorAction SilentlyContinue
        if ($null -eq $value) { continue }
        $pv = if ($value.PSObject.Properties['pv']) { [string]$value.pv } else { '' }
        if ($pv -match '^\d+\.\d+\.\d+\.\d+$' -and $pv -notmatch '^0+\.0+\.0+\.0+$') {
            return $pv
        }
    }
    return $null
}

if (Get-WebView2EvergreenVersion) {
    throw 'REFUSED: WebView2 was already installed, restore the clean VM snapshot'
}

$installer = (Resolve-Path -LiteralPath $InstallerPath -ErrorAction Stop).Path
if (($Kind -eq 'MSI' -and [IO.Path]::GetExtension($installer) -ine '.msi') -or
    ($Kind -eq 'NSIS' -and $installer -notmatch '(?i)-setup\.exe$')) {
    throw 'REFUSED: unexpected file extension or installer type'
}
$actual = (Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actual -cne $ExpectedSha256.ToLowerInvariant()) {
    throw 'REFUSED: installer hash differs from trusted release manifest'
}

$uninstallKeys = @(
    'Registry::HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*',
    'Registry::HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*',
    'Registry::HKEY_CURRENT_USER\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*'
)
$existing = @(Get-ItemProperty -Path $uninstallKeys -ErrorAction SilentlyContinue |
    Where-Object { $_.PSObject.Properties['DisplayName'] -and $_.DisplayName -eq 'Pravda-Bot' })
if ($existing.Count -gt 0) {
    throw 'REFUSED: Pravda-Bot already installed'
}

$root = Join-Path $env:TEMP ('pravda-offline-acceptance-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root -Force | Out-Null
$oldTemp = [Environment]::GetEnvironmentVariable('RUNNER_TEMP', 'Process')
$success = $false
try {
    $env:RUNNER_TEMP = $root
    # Run the EXACT existing Windows CI smoke, with synthetic local SQLite,
    # auth and market feeds disabled. It installs and uninstalls the app.
    if ($Kind -eq 'MSI') {
        & (Join-Path $PSScriptRoot 'Test-Pravda-Msi-Install.ps1') -MsiPath $installer
    } else {
        & (Join-Path $PSScriptRoot 'Test-Pravda-Nsis-Install.ps1') -NsisPath $installer
    }
    if (-not $?) {
        throw 'FAIL: installed desktop acceptance smoke returned failure'
    }
    $after = Get-WebView2EvergreenVersion
    if ([string]::IsNullOrWhiteSpace($after)) {
        throw 'FAIL: offline installer did not register WebView2 Evergreen'
    }
    if (@(Get-NetAdapter -ErrorAction Stop | Where-Object { $_.Status -eq 'Up' }).Count -ne 0) {
        throw 'FAIL: network interface was enabled during the test'
    }
    $success = $true
    Write-Host ('PASS: offline ' + $Kind + ' install/start/uninstall on WebView2-free VM')
    Write-Host ('PASS: SHA256=' + $actual)
    Write-Host ('PASS: WebView2 Evergreen=' + $after)
}
finally {
    [Environment]::SetEnvironmentVariable('RUNNER_TEMP', $oldTemp, 'Process')
    if ($success -and (Test-Path -LiteralPath $root)) {
        Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
    }
    if (-not $success) {
        Write-Warning 'NO-GO: keep VM isolated; review redacted installation evidence'
    }
}

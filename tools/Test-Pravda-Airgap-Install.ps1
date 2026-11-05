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
  ./tools/Test-Pravda-Airgap-Install.ps1 -InstallerPath ./Pravda-Bot_x64.msi -Kind MSI -ExpectedSha256 HASH_FROM_TRUSTED_MANIFEST -ExpectedCommitSha 40_CHAR_SHA -DisposableVm
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
    [ValidatePattern('^[a-fA-F0-9]{40}$')]
    [string]$ExpectedCommitSha,
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
# Refuse likely physical machines, even if the explicit switch was supplied.
# Heuristics can false-negative: they are only an additional guard, not proof
# of disposable hypervisor isolation or authorization to touch user files.
$computer = Get-CimInstance -ClassName Win32_ComputerSystem -ErrorAction Stop
$machineModel = ([string]$computer.Manufacturer + ' ' + [string]$computer.Model)
if ($machineModel -notmatch '(?i)Virtual|VMware|QEMU|KVM|Xen|Parallels|Hyper-V|HVM domU|OpenStack|Amazon EC2') {
    throw 'REFUSED: system does not identify as a virtual machine'
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

# Test-only toolchain must be staged before the hypervisor disconnects
# the disposable VM. No download, installation or user-data change here.
foreach ($binary in @('node.exe', 'tauri-driver.exe', 'msedgedriver.exe')) {
    if (-not (Get-Command -Name $binary -CommandType Application -ErrorAction SilentlyContinue)) {
        throw "REFUSED: offline GUI test tool missing: $binary"
    }
}
foreach ($required in @('Test-Pravda-Msi-Install.ps1', 'Test-Pravda-Nsis-Install.ps1',
                       'Initialize-Pravda-Airgap-Gui.ps1', 'test-installed-onboarding.mjs')) {
    if (-not (Test-Path -LiteralPath (Join-Path $PSScriptRoot $required) -PathType Leaf)) {
        throw "REFUSED: offline GUI acceptance script missing: $required"
    }
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
$oldEnvironment = @{}
foreach ($name in @('GITHUB_SHA', 'PRAVDA_AIRGAP_ACCEPTANCE',
                    'PRAVDA_E2E_WEBVIEW2_VERSION', 'PRAVDA_E2E_WINDOWS_VERSION')) {
    $oldEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
}
$success = $false
try {
    $env:RUNNER_TEMP = $root
    # The operator supplies the trusted source SHA, never inferred from VM.
    $env:GITHUB_SHA = $ExpectedCommitSha.ToLowerInvariant()
    $env:PRAVDA_AIRGAP_ACCEPTANCE = 'true'
    Remove-Item Env:PRAVDA_E2E_WEBVIEW2_VERSION -ErrorAction SilentlyContinue
    Remove-Item Env:PRAVDA_E2E_WINDOWS_VERSION -ErrorAction SilentlyContinue
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
    # The GUI JSON lives in RUNNER_TEMP and would otherwise be destroyed by
    # routine cleanup. Retain a redacted, hashed proof separately on this VM.
    # The VM operator exports it only after the offline tests are complete.
    $reportName = if ($Kind -eq 'MSI') {
        'Pravda-msi-installed-onboarding.json'
    } else {
        'Pravda-nsis-installed-onboarding.json'
    }
    $reportPath = Join-Path $root $reportName
    if (-not (Test-Path -LiteralPath $reportPath -PathType Leaf)) {
        throw 'FAIL: installed GUI acceptance proof is missing'
    }
    $gui = Get-Content -LiteralPath $reportPath -Raw -ErrorAction Stop | ConvertFrom-Json
    if ($gui.version -ne 1 -or $gui.passed -ne $true -or
        $gui.kind -ine $Kind -or $gui.sha -cne $ExpectedCommitSha.ToLowerInvariant() -or
        @($gui.tests).Count -lt 14 -or
        @($gui.tests | Where-Object { $_.passed -ne $true }).Count -ne 0) {
        throw 'FAIL: installed GUI acceptance proof is inconsistent'
    }
    $proofName = 'pravda-airgap-evidence-' + [guid]::NewGuid().ToString('N')
    $proofRoot = Join-Path $env:TEMP $proofName
    New-Item -ItemType Directory -Path $proofRoot -ErrorAction Stop | Out-Null
    $proofCopy = Join-Path $proofRoot 'installed-gui-report.json'
    Copy-Item -LiteralPath $reportPath -Destination $proofCopy -ErrorAction Stop
    $proofMeta = [ordered]@{
        version = 1
        kind = $Kind
        source_sha = $ExpectedCommitSha.ToLowerInvariant()
        installer_sha256 = $actual
        installed_gui_report_sha256 = (Get-FileHash -LiteralPath $proofCopy -Algorithm SHA256).Hash.ToLowerInvariant()
        webview2_version = $after
        passed = $true
        scope = 'Disposable VM with disconnected hypervisor and no initial WebView2'
        limitation = 'Operator must independently attest the hypervisor snapshot and network isolation'
    }
    $proofMeta | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $proofRoot 'airgap-evidence.json') -Encoding utf8 -ErrorAction Stop
    Write-Host ('PASS: retained offline acceptance evidence in TEMP/' + $proofName)
    $success = $true
    Write-Host ('PASS: offline ' + $Kind + ' install/start/uninstall on WebView2-free VM')
    Write-Host ('PASS: SHA256=' + $actual)
    Write-Host ('PASS: WebView2 Evergreen=' + $after)
}
finally {
    [Environment]::SetEnvironmentVariable('RUNNER_TEMP', $oldTemp, 'Process')
    foreach ($name in $oldEnvironment.Keys) {
        [Environment]::SetEnvironmentVariable($name, $oldEnvironment[$name], 'Process')
    }
    if ($success -and (Test-Path -LiteralPath $root)) {
        Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
    }
    if (-not $success) {
        Write-Warning 'NO-GO: keep VM isolated; review redacted installation evidence'
    }
}

#Requires -Version 7.0
<#
.SYNOPSIS
  Validate offline-VM GUI test prerequisites after installing MSI/NSIS.
.DESCRIPTION
  Only runs in a disposable disconnected VM. Node, tauri-driver and the
  matching EdgeDriver must be staged BEFORE disconnecting the hypervisor.
  This helper does not fetch tools, install software, or enable networking.
#>
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ($env:PRAVDA_AIRGAP_ACCEPTANCE -cne 'true' -or
    $env:GITHUB_SHA -notmatch '^[a-fA-F0-9]{40}$') {
    throw 'REFUSED: offline VM acknowledgement and exact release SHA required'
}
if (@(Get-NetAdapter -ErrorAction Stop | Where-Object { $_.Status -eq 'Up' }).Count -ne 0) {
    throw 'REFUSED: network adapter active during offline GUI verification'
}
if (@(Get-NetRoute -ErrorAction Stop | Where-Object {
    $_.DestinationPrefix -in @('0.0.0.0/0', '::/0')
}).Count -ne 0) {
    throw 'REFUSED: default route appeared during offline GUI verification'
}
foreach ($binary in @('node.exe', 'tauri-driver.exe', 'msedgedriver.exe')) {
    if (-not (Get-Command -Name $binary -CommandType Application -ErrorAction SilentlyContinue)) {
        throw "REFUSED: offline GUI test tool missing: $binary"
    }
}
$nodeVersion = (& node.exe --version | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or $nodeVersion -notmatch '^v([0-9]+)\.') {
    throw 'REFUSED: cannot determine offline Node version'
}
$nodeMajor = [int]$Matches[1]
if ($nodeMajor -lt 22) { throw 'REFUSED: Node 22 or newer required' }

$driverOutput = (& msedgedriver.exe --version | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or $driverOutput -notmatch '\b([0-9]+\.[0-9]+\.[0-9]+\.[0-9]+)\b') {
    throw 'REFUSED: cannot determine staged Microsoft EdgeDriver version'
}
$driverVersion = $Matches[1]
$webViewVersion = $null
$guid = '{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
foreach ($key in @(
    "HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\$guid",
    "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\$guid",
    "HKCU:\SOFTWARE\Microsoft\EdgeUpdate\Clients\$guid",
    "HKCU:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\$guid"
)) {
    $entry = Get-ItemProperty -LiteralPath $key -ErrorAction SilentlyContinue
    if ($null -eq $entry) { continue }
    $pv = if ($entry.PSObject.Properties['pv']) { [string]$entry.pv } else { '' }
    if ($pv -match '^\d+\.\d+\.\d+\.\d+$' -and $pv -notmatch '^0+\.0+\.0+\.0+$') {
        $webViewVersion = $pv
        break
    }
}
if (-not $webViewVersion) {
    throw 'FAIL: WebView2 runtime missing after offline installer'
}
if ($driverVersion.Split('.')[0] -cne $webViewVersion.Split('.')[0]) {
    throw 'REFUSED: staged EdgeDriver major does not match installed WebView2'
}
$env:PRAVDA_E2E_WEBVIEW2_VERSION = $webViewVersion
$env:PRAVDA_E2E_WINDOWS_VERSION = [System.Environment]::OSVersion.VersionString
Write-Host 'PASS: offline GUI preflight — local tools, matching runtime major and release SHA'

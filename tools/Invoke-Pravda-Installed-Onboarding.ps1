[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('MSI', 'NSIS')]
    [string]$Kind
)
$ErrorActionPreference = 'Stop'

# Shared acceptance path for the two real installed Windows bundles.
# All inputs originate from an isolated test profile, never user data.
if ($env:PRAVDA_INSTALL_KIND -cne $Kind) {
    throw 'Installed GUI kind does not match the selected installer'
}
if ($env:PRAVDA_SOURCE_SHA -notmatch '^[0-9a-fA-F]{40}$') {
    throw 'Installed GUI acceptance requires the exact source SHA'
}
if ($env:PRAVDA_AIRGAP_ACCEPTANCE -ceq 'true') {
    & (Join-Path $PSScriptRoot 'Initialize-Pravda-Airgap-Gui.ps1')
}
node (Join-Path $PSScriptRoot 'test-installed-onboarding.mjs')
if ($LASTEXITCODE -ne 0) {
    throw "$Kind installed production GUI onboarding acceptance failed"
}
$uiEvidence = Get-Content -LiteralPath $env:PRAVDA_ONBOARDING_REPORT -Raw -ErrorAction Stop | ConvertFrom-Json
if ($uiEvidence.version -ne 1 -or $uiEvidence.passed -ne $true -or
    $uiEvidence.kind -cne $Kind -or $uiEvidence.sha -cne $env:PRAVDA_SOURCE_SHA -or
    @($uiEvidence.tests).Count -lt 14 -or
    @($uiEvidence.tests | Where-Object { $_.passed -ne $true }).Count -ne 0) {
    throw "$Kind native GUI onboarding evidence is missing or invalid"
}
Write-Host "PASS: $Kind installed production WebView2 first-use setup, restart, login and logout"

#Requires -Version 7.0
<#
.SYNOPSIS
  Verify Pravda-Bot Windows release candidate archive after extraction.
.DESCRIPTION
  SHA256SUMS paths are relative to the common target/release directory
  retained by actions/upload-artifact. Signature validation is mandatory
  only when the signed Enterprise flag is explicitly supplied.
  This tool never imports a certificate and handles no private key.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$ReleaseRoot,
    [switch]$RequireSigned,
    [switch]$RequireCompanions,
    [string]$ExpectedThumbprint = ''
)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath $ReleaseRoot -ErrorAction Stop).Path
$manifestPath = Join-Path $root 'bundle/SHA256SUMS.txt'
if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
    throw 'Release checksum manifest is missing'
}
if ($RequireSigned -and $ExpectedThumbprint -notmatch '^[0-9A-Fa-f]{40}$') {
    throw 'Signed release requires a trusted certificate thumbprint'
}

$lines = @(Get-Content -LiteralPath $manifestPath -Encoding ascii)
if ($lines.Count -lt 3) {
    throw 'Release manifest must contain EXE, MSI and NSIS'
}
$paths = [System.Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
$kinds = [System.Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
$pattern = '^([0-9a-f]{64})  (pravda-app\.exe|pravda-doctor\.exe|pravda-harness\.exe|pravda-test-center\.exe|pravda-setup\.exe|bundle/msi/[^/\\]+\.msi|bundle/nsis/[^/\\]+-setup\.exe)$'
foreach ($line in $lines) {
    if ($line -cnotmatch $pattern) {
        throw 'Malformed or unsafe checksum manifest entry'
    }
    $expectedHash = $Matches[1]
    $relative = $Matches[2]
    if (-not $paths.Add($relative)) {
        throw 'Duplicate manifest entry'
    }
    $kind = if ($relative -eq 'pravda-app.exe') {
        'exe'
    } elseif ($relative -eq 'pravda-doctor.exe') {
        'doctor'
    } elseif ($relative -eq 'pravda-harness.exe') {
        'harness'
    } elseif ($relative -eq 'pravda-test-center.exe') {
        'test_center'
    } elseif ($relative -eq 'pravda-setup.exe') {
        'setup'
    } elseif ($relative.StartsWith('bundle/msi/')) {
        'msi'
    } else {
        'nsis'
    }
    [void]$kinds.Add($kind)
    $filepath = Join-Path $root ($relative.Replace('/', [IO.Path]::DirectorySeparatorChar))
    $file = Get-Item -LiteralPath $filepath -ErrorAction Stop
    if ($file.PSIsContainer -or ($file.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        throw 'Manifest points to a directory or a reparse point'
    }
    $actualHash = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($expectedHash -cne $actualHash) {
        throw "SHA-256 mismatch for $relative"
    }
    if ($RequireSigned) {
        $signature = Get-AuthenticodeSignature -LiteralPath $file.FullName
        if ($signature.Status -ne 'Valid' -or
            -not $signature.SignerCertificate -or
            $signature.SignerCertificate.Thumbprint -ne $ExpectedThumbprint -or
            -not $signature.TimeStamperCertificate) {
            throw "Authenticode or timestamp verification failed for $relative"
        }
    }
}
$requiredKinds = @('exe', 'msi', 'nsis')
if ($RequireCompanions) { $requiredKinds += @('doctor', 'harness', 'test_center', 'setup') }
foreach ($expectedKind in $requiredKinds) {
    if (-not $kinds.Contains($expectedKind)) {
        throw "Release manifest is missing the $expectedKind candidate"
    }
}
$actualCandidates = @(
    Get-Item -LiteralPath (Join-Path $root 'pravda-app.exe') -ErrorAction Stop
    Get-ChildItem -LiteralPath $root -Filter 'pravda-doctor.exe' -File -ErrorAction Stop
    Get-ChildItem -LiteralPath $root -Filter 'pravda-harness.exe' -File -ErrorAction Stop
    Get-ChildItem -LiteralPath $root -Filter 'pravda-test-center.exe' -File -ErrorAction Stop
    Get-ChildItem -LiteralPath $root -Filter 'pravda-setup.exe' -File -ErrorAction Stop
    Get-ChildItem -LiteralPath (Join-Path $root 'bundle/msi') -Filter '*.msi' -File -ErrorAction Stop
    Get-ChildItem -LiteralPath (Join-Path $root 'bundle/nsis') -Filter '*-setup.exe' -File -ErrorAction Stop
)
if ($actualCandidates.Count -ne $paths.Count) {
    throw 'Unlisted release candidate files exist or manifest is incomplete'
}
if ($RequireSigned) {
    Write-Host 'Validated Enterprise candidate: SHA256 + Authenticode + timestamp.'
} else {
    Write-Warning 'Validated unsigned candidate hashes only; NOT an Enterprise release.'
}

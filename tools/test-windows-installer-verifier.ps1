#Requires -Version 7.0
# Synthetic Windows-only contract test: no signing certificate or real
# installer is required; production Authenticode checks remain independent.
$ErrorActionPreference = 'Stop'
$root = Join-Path $env:RUNNER_TEMP ("pravda-installer-test-" + [guid]::NewGuid().ToString('N'))
$bundle = Join-Path $root 'bundle'
$msiDir = Join-Path $bundle 'msi'
$nsisDir = Join-Path $bundle 'nsis'
New-Item -ItemType Directory -Force -Path $msiDir, $nsisDir | Out-Null

$targets = @(
    @{ Path = 'pravda-app.exe'; Data = 'synthetic executable' },
    @{ Path = 'bundle/msi/pravda-test.msi'; Data = 'synthetic MSI' },
    @{ Path = 'bundle/nsis/pravda-test-setup.exe'; Data = 'synthetic NSIS' }
)
try {
    $manifest = foreach ($target in $targets) {
        $file = Join-Path $root $target.Path
        [IO.File]::WriteAllText($file, $target.Data)
        $hash = (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash.ToLowerInvariant()
        "$hash  $($target.Path)"
    }
    $manifestPath = Join-Path $bundle 'SHA256SUMS.txt'
    $manifest | Set-Content -LiteralPath $manifestPath -Encoding ascii
    & "$PSScriptRoot/Verify-Pravda-Installer.ps1" -ReleaseRoot $root

    # SEC: Missing certificate must never permit signed Enterprise artifacts.
    $signedRejected = $false
    try {
        & "$PSScriptRoot/Verify-Pravda-Installer.ps1" -ReleaseRoot $root -RequireSigned
    } catch {
        $signedRejected = $true
    }
    if (-not $signedRejected) { throw 'Unsigned artifacts were accepted as Enterprise' }
    # SEC: An arbitrary thumbprint does not certify unsigned binaries.
    $signedRejected = $false
    try {
        & "$PSScriptRoot/Verify-Pravda-Installer.ps1" -ReleaseRoot $root -RequireSigned -ExpectedThumbprint ('A' * 40)
    } catch {
        $signedRejected = $true
    }
    if (-not $signedRejected) { throw 'Untrusted certificate thumbprint was accepted' }
    # Corruption must be rejected even when names/counts remain unchanged.
    [IO.File]::AppendAllText((Join-Path $root $targets[1].Path), 'tampered')
    $rejected = $false
    try {
        & "$PSScriptRoot/Verify-Pravda-Installer.ps1" -ReleaseRoot $root
    } catch {
        $rejected = $true
    }
    if (-not $rejected) { throw 'Tampered MSI passed checksum validation' }

    # Restore a valid manifest, then reject traversal in a manifest path.
    $targets[1].Data = 'synthetic MSI'
    [IO.File]::WriteAllText((Join-Path $root $targets[1].Path), $targets[1].Data)
    $badManifest = @($manifest)
    $badManifest[1] = $badManifest[1].Replace('bundle/msi/pravda-test.msi', '../outside.msi')
    $badManifest | Set-Content -LiteralPath $manifestPath -Encoding ascii
    $rejected = $false
    try {
        & "$PSScriptRoot/Verify-Pravda-Installer.ps1" -ReleaseRoot $root
    } catch {
        $rejected = $true
    }
    if (-not $rejected) { throw 'Unsafe manifest path was accepted' }
    # Companion contract: a complete release must also authenticate Doctor
    # and Harness, but historic 3-artifact test candidates remain compatible.
    foreach ($item in @(
        @{ Path = 'pravda-doctor.exe'; Data = 'synthetic doctor' },
        @{ Path = 'pravda-harness.exe'; Data = 'synthetic harness' ,
        @{ Path = 'pravda-test-center.exe'; Data = 'synthetic integrated test center' }
    )) {
        $file = Join-Path $root $item.Path
        [IO.File]::WriteAllText($file, $item.Data)
    }
    $fullManifest = @($manifest)
    foreach ($name in @('pravda-doctor.exe', 'pravda-harness.exe', 'pravda-test-center.exe')) {
        $file = Join-Path $root $name
        $sum = (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash.ToLowerInvariant()
        $fullManifest += "$sum  $name"
    }
    $fullManifest | Set-Content -LiteralPath $manifestPath -Encoding ascii
    & "$PSScriptRoot/Verify-Pravda-Installer.ps1" -ReleaseRoot $root -RequireCompanions

    # Missing companion must fail closed rather than publishing partial release.
    Remove-Item -LiteralPath (Join-Path $root 'pravda-harness.exe') -Force
    $rejected = $false
    try {
        & "$PSScriptRoot/Verify-Pravda-Installer.ps1" -ReleaseRoot $root -RequireCompanions
    } catch {
        $rejected = $true
    }
    if (-not $rejected) { throw 'Missing Harness passed companion verification' }
    # The integrated EXE is mandatory for complete P0 companion releases.
    [IO.File]::WriteAllText((Join-Path $root 'pravda-harness.exe'), 'synthetic harness')
    Remove-Item -LiteralPath (Join-Path $root 'pravda-test-center.exe') -Force
    $rejected = $false
    try {
        & "$PSScriptRoot/Verify-Pravda-Installer.ps1" -ReleaseRoot $root -RequireCompanions
    } catch { $rejected = $true }
    if (-not $rejected) { throw 'Missing test-center passed companion verification' }
    Write-Host 'Installer manifest, companions and tampering tests passed.'
} finally {
    Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
}

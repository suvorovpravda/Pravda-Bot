# Synchronise a clean local checkout with the validated main branch.
# No reset, clean, deletion or overwrite of uncommitted work is performed.
[CmdletBinding()]
param(
    [string]$Destination = (Join-Path $env:USERPROFILE "Pravda-Bot"),
    [switch]$Launch
)
$ErrorActionPreference = "Stop"
$repository = "https://github.com/suvorovpravda/Pravda-Bot.git"

function Invoke-Checked {
    param([string]$Program, [string[]]$Arguments)
    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Program failed with exit code $LASTEXITCODE"
    }
}

if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    throw "Git is required. Install Git for Windows first."
}
if (-not (Test-Path -LiteralPath $Destination)) {
    Invoke-Checked git @("clone", "--branch", "main", $repository, $Destination)
} elseif (-not (Test-Path -LiteralPath (Join-Path $Destination ".git"))) {
    throw "Destination exists but is not a Git checkout: $Destination. No files were modified."
}

$remote = (& git -C $Destination remote get-url origin).Trim()
if ($LASTEXITCODE -ne 0 -or $remote -notmatch "github\.com[:/]suvorovpravda/Pravda-Bot(?:\.git)?$") {
    throw "Unexpected Git origin. No changes were made: $remote"
}
$dirty = & git -C $Destination status --porcelain --untracked-files=normal
if ($LASTEXITCODE -ne 0) { throw "Unable to check repository state." }
if ($dirty) {
    throw "Local files are modified or untracked. Back them up or commit them first; refusing to overwrite anything."
}
$branch = (& git -C $Destination branch --show-current).Trim()
if ($branch -ne "main") {
    throw "Checkout is on '$branch', not main. Switch branches manually after saving local changes."
}
Invoke-Checked git @("-C", $Destination, "fetch", "origin", "main")
Invoke-Checked git @("-C", $Destination, "pull", "--ff-only", "origin", "main")
$localSha = (& git -C $Destination rev-parse HEAD).Trim()
$remoteSha = (& git -C $Destination rev-parse origin/main).Trim()
if ($LASTEXITCODE -ne 0 -or $localSha -ne $remoteSha) {
    throw "Local and remote main are not identical."
}
Write-Host "Git checkout synchronized at commit $localSha"

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "Rust/Cargo is required. Install it using rustup, then rerun this script."
}
Push-Location $Destination
try {
    Invoke-Checked cargo @("fmt", "--all", "--", "--check")
    Invoke-Checked cargo @("fmt", "--manifest-path", "crates/pravda-app/Cargo.toml", "--", "--check")
    Invoke-Checked cargo @("check", "--locked", "--workspace", "--all-targets")
    Invoke-Checked cargo @("test", "--locked", "--workspace")
    Invoke-Checked cargo @("test", "--locked", "--manifest-path", "crates/pravda-app/Cargo.toml")
    Invoke-Checked cargo @("build", "--locked", "--manifest-path", "crates/pravda-app/Cargo.toml")
    $app = Join-Path $Destination "crates/pravda-app/target/debug/pravda-app.exe"
    if (-not (Test-Path -LiteralPath $app)) {
        throw "Desktop executable not found after build: $app"
    }
    Write-Host "Windows desktop executable built: $app"
    if ($Launch) {
        Start-Process -FilePath $app -WorkingDirectory $Destination
        Write-Host "Launched Pravda-Bot. Sign in or set a local password at first run."
    }
} finally {
    Pop-Location
}

#Requires -Version 7.0
<#
.SYNOPSIS
  Windows hosted-runner MSI clean-install and uninstall acceptance test.
.DESCRIPTION
  Launches the installed desktop once with isolated test-only storage and
  disabled market feeds. Never sends orders or modifies existing installations.
  Run only in a disposable Windows test environment.
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
            Where-Object { $_.PSObject.Properties['DisplayName'] -and $_.DisplayName -eq 'Pravda-Bot' }
    )
    return $matches
}

if (@(Get-PravdaInstall).Count -ne 0) {
    throw 'Clean-install smoke test requires a pristine Windows runner'
}

$installedCode = $null
$testSucceeded = $false
$canaries = @(
    (Join-Path $env:APPDATA 'com.pravda.bot/ci-retention-canary.txt'),
    (Join-Path $env:LOCALAPPDATA 'Pravda-Bot/ci-retention-canary.txt')
)
$canaryContents = 'Pravda MSI retention canary - do not remove user data on uninstall'
foreach ($canary in $canaries) {
    if (Test-Path -LiteralPath $canary) { throw "Cannot overwrite pre-existing test canary: $canary" }
}
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
    $installedExe = $null
    foreach ($folder in $roots) {
        if (-not (Test-Path -LiteralPath $folder -PathType Container)) { continue }
        $executables = @(Get-ChildItem -LiteralPath $folder -Filter '*.exe' -File -Recurse -ErrorAction Stop)
        $applications = @($executables | Where-Object {
            $_.Name -in @('pravda-app.exe', 'Pravda-Bot.exe')
        })
        if ($applications.Count -eq 1) {
            $found = $true
            $installedExe = $applications[0].FullName
            break
        }
    }
    if (-not $found) {
        throw 'MSI registration exists but its application executable was not found'
    }
    # Exercise the *installed* desktop binary, not the build-tree EXE.
    # A disposable database/auth path prevents touching normal user data.
    $smokeDir = Join-Path $env:RUNNER_TEMP ('pravda-installed-smoke-' + [guid]::NewGuid().ToString('N'))
    New-Item -Path $smokeDir -ItemType Directory -Force | Out-Null
    $dbPath = Join-Path $smokeDir 'data.db'
    $authPath = Join-Path $smokeDir 'auth.json'
    $keys = @('PRAVDA_DB_PATH', 'PRAVDA_AUTH_PATH', 'PRAVDA_MARKET_FEEDS_ENABLED')
    $previous = @{}
    foreach ($key in $keys) {
        $previous[$key] = [Environment]::GetEnvironmentVariable($key, 'Process')
    }
    $app = $null
    try {
        $env:PRAVDA_DB_PATH = $dbPath
        $env:PRAVDA_AUTH_PATH = $authPath
        $env:PRAVDA_MARKET_FEEDS_ENABLED = 'false'
        $app = Start-Process -FilePath $installedExe -PassThru
        $initialized = $false
        for ($attempt = 0; $attempt -lt 60; $attempt++) {
            $app.Refresh()
            if ($app.HasExited) { throw "Installed desktop exited prematurely: $($app.ExitCode)" }
            if (Test-Path -LiteralPath $dbPath -PathType Leaf) {
                $initialized = $true
                break
            }
            Start-Sleep -Milliseconds 500
        }
        if (-not $initialized) { throw 'Installed desktop did not initialize SQLite' }
        Start-Sleep -Seconds 2
        $app.Refresh()
        if ($app.HasExited) { throw 'Installed desktop exited after SQLite initialization' }
        $listeners = @(Get-NetTCPConnection -State Listen -ErrorAction SilentlyContinue |
            Where-Object { $_.OwningProcess -eq $app.Id })
        if ($listeners.Count -ne 0) { throw 'Installed desktop opened an unexpected TCP listener' }
        if (Test-Path -LiteralPath $authPath) {
            throw 'Installed desktop created password verifier before onboarding'
        }
        Write-Host 'PASS: installed release desktop starts, initializes isolated SQLite and has no HTTP listener'
        # Restart the actual installed release executable against the same
        # durable SQLite path, not a new test database. No password is enrolled.
        Stop-Process -Id $app.Id -Force -ErrorAction Stop
        if (-not $app.WaitForExit(10000)) {
            throw 'Installed MSI desktop did not exit before restart'
        }
        $header = [System.IO.File]::ReadAllBytes($dbPath)
        if ($header.Length -lt 16 -or
            [System.Text.Encoding]::ASCII.GetString($header, 0, 16) -cne ("SQLite format 3" + [char]0)) {
            throw 'MSI first launch did not preserve an intact SQLite file'
        }
        $app = Start-Process -FilePath $installedExe -PassThru
        Start-Sleep -Seconds 3
        $app.Refresh()
        if ($app.HasExited) {
            throw "Installed MSI desktop exited on second launch: $($app.ExitCode)"
        }
        if (-not (Test-Path -LiteralPath $dbPath -PathType Leaf) -or
            (Get-Item -LiteralPath $dbPath).Length -lt 16) {
            throw 'MSI second launch failed to retain the original SQLite file'
        }
        if (Test-Path -LiteralPath $authPath) {
            throw 'MSI desktop created a password verifier without onboarding after restart'
        }
        $secondListeners = @(Get-NetTCPConnection -State Listen -ErrorAction SilentlyContinue |
            Where-Object { $_.OwningProcess -eq $app.Id })
        if ($secondListeners.Count -ne 0) {
            throw 'MSI installed desktop opened a TCP listener after restart'
        }
        Write-Host 'PASS: MSI installed release desktop restarts with its existing SQLite and no implicit auth'
    }
    finally {
        if ($null -ne $app) {
            $app.Refresh()
            if (-not $app.HasExited) { Stop-Process -Id $app.Id -Force -ErrorAction Stop }
        }
        foreach ($key in $keys) {
            [Environment]::SetEnvironmentVariable($key, $previous[$key], 'Process')
        }
        Remove-Item -LiteralPath $smokeDir -Recurse -Force -ErrorAction SilentlyContinue
    }
    # This sentinel represents an existing per-user file; the installer must not
    # delete the Tauri app data directory on uninstall. It does not replace a
    # full GUI/database persistence acceptance test.
    foreach ($canary in $canaries) {
        New-Item -Path (Split-Path -Parent $canary) -ItemType Directory -Force | Out-Null
        Set-Content -LiteralPath $canary -Value $canaryContents -Encoding utf8
    }
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
            foreach ($canary in $canaries) {
                if (-not (Test-Path -LiteralPath $canary -PathType Leaf) -or
                    (Get-Content -LiteralPath $canary -Raw).TrimEnd() -cne $canaryContents) {
                    throw "MSI uninstall removed or altered existing per-user app data: $canary"
                }
            }
            Write-Host 'PASS: existing per-user app data survives MSI uninstall'
            # Reinstall the same verified MSI and prove user files persist.
            # Keep the product cleanup inside a finally, even if acceptance fails.
            try {
                $again = Start-Process -FilePath $msiexec -ArgumentList @(
                    '/i', ('"' + $msi + '"'), '/qn', '/norestart'
                ) -PassThru -Wait
                if ($again.ExitCode -ne 0) {
                    throw "MSI reinstall failed with exit code $($again.ExitCode)"
                }
                $installedAgain = @(Get-PravdaInstall)
                if ($installedAgain.Count -ne 1 -or
                    [string]$installedAgain[0].PSChildName -cne $installedCode) {
                    throw 'MSI reinstall did not restore exactly the same product registration'
                }
                if (-not (Test-Path -LiteralPath $installedExe -PathType Leaf)) {
                    throw 'MSI reinstall did not restore the installed desktop executable'
                }
                foreach ($canary in $canaries) {
                    if (-not (Test-Path -LiteralPath $canary -PathType Leaf) -or
                        (Get-Content -LiteralPath $canary -Raw).TrimEnd() -cne $canaryContents) {
                        throw "MSI reinstall changed the existing user-data canary: $canary"
                    }
                }
                Write-Host 'PASS: MSI reinstall preserves both user-data canaries and desktop binary'
            }
            finally {
                $afterReinstall = @(Get-PravdaInstall)
                if ($afterReinstall.Count -eq 1) {
                    $guid = [string]$afterReinstall[0].PSChildName
                    if ($guid -notmatch '^[{][A-Fa-f0-9-]{36}[}]$' -or $guid -cne $installedCode) {
                        throw 'Unexpected product registration after reinstall; refusing unrelated uninstall'
                    }
                    $cleanup = Start-Process -FilePath $msiexec -ArgumentList @(
                        '/x', $guid, '/qn', '/norestart'
                    ) -PassThru -Wait
                    if ($cleanup.ExitCode -ne 0) {
                        throw "Post-reinstall cleanup failed with code $($cleanup.ExitCode)"
                    }
                }
                if (@(Get-PravdaInstall).Count -ne 0) {
                    throw 'MSI remains installed after the final uninstall'
                }
            }
        }
    }
    foreach ($canary in $canaries) {
        if (Test-Path -LiteralPath $canary -PathType Leaf) {
            Remove-Item -LiteralPath $canary -Force -ErrorAction SilentlyContinue
        }
    }
    # Retain verbose MSI log on ANY smoke failure for the CI failure artifact.
    # The Windows hosted runner has no broker or code-signing secrets.
    if ($testSucceeded) {
        Remove-Item -LiteralPath $log -Force -ErrorAction SilentlyContinue
    } else {
        Write-Warning 'MSI smoke failed; retaining verbose log for protected CI artifact'
    }
}

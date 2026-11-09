#Requires -Version 7.0
<#
.SYNOPSIS
  Tests the NSIS installation, installed desktop, and silent removal.
.DESCRIPTION
  Only for disposable Windows CI. All synthetic database/auth paths and
  installation files are kept under RUNNER_TEMP. Real user data is never used.
  No broker credentials, no trading orders, no downloaded scripts.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$NsisPath
)

$ErrorActionPreference = 'Stop'
$installer = (Resolve-Path -LiteralPath $NsisPath -ErrorAction Stop).Path
if ($installer -notmatch '(?i)-setup\.exe$') {
    throw 'A Tauri NSIS setup executable is required'
}
if ([string]::IsNullOrWhiteSpace($env:RUNNER_TEMP)) {
    throw 'Disposable Windows RUNNER_TEMP is required'
}
$root = Join-Path $env:RUNNER_TEMP ('pravda-nsis-smoke-' + [guid]::NewGuid().ToString('N'))
$destination = Join-Path $root 'installed'
$isolated = Join-Path $root 'storage'
$dbPath = Join-Path $isolated 'data.db'
$authPath = Join-Path $isolated 'auth.json'
$uninstallPaths = @(
    'Registry::HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*',
    'Registry::HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*',
    'Registry::HKEY_CURRENT_USER\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*'
)
function Get-PravdaNsisRegistration {
    return @(Get-ItemProperty -Path $uninstallPaths -ErrorAction SilentlyContinue |
        Where-Object { $_.PSObject.Properties['DisplayName'] -and $_.DisplayName -eq 'Pravda-Bot' })
}

if (@(Get-PravdaNsisRegistration).Count -ne 0) {
    throw 'NSIS smoke requires a pristine Windows runner without Pravda-Bot installed'
}

$canaries = @(
    (Join-Path $env:APPDATA 'com.pravda.bot/ci-nsis-canary.txt'),
    (Join-Path $env:LOCALAPPDATA 'Pravda-Bot/ci-nsis-canary.txt')
)
$canaryText = 'NSIS CI synthetic user profile retention sentinel'
foreach ($canary in $canaries) {
    if (Test-Path -LiteralPath $canary) {
        throw 'Refusing to overwrite pre-existing profile canary'
    }
}
$installed = $false
$uninstalled = $false
$testSucceeded = $false
New-Item -ItemType Directory -Path $destination, $isolated -Force | Out-Null
try {
    # Tauri NSIS follows the documented NSIS uppercase /S, with /D last.
    # The private installation root guarantees no unrelated install is touched.
    $process = Start-Process -FilePath $installer -ArgumentList @('/S', ('/D=' + $destination)) -Wait -PassThru
    if ($process.ExitCode -ne 0) {
        throw "NSIS silent setup failed with code $($process.ExitCode)"
    }
    $registration = @(Get-PravdaNsisRegistration)
    if ($registration.Count -ne 1) {
        throw "NSIS expected exactly one product registration, got $($registration.Count)"
    }
    if (-not (Test-Path -LiteralPath $destination -PathType Container)) {
        throw 'NSIS did not install in the expected isolated destination'
    }
    $applications = @(Get-ChildItem -LiteralPath $destination -Filter '*.exe' -File -Recurse -ErrorAction Stop |
        Where-Object { $_.Name -in @('pravda-app.exe', 'Pravda-Bot.exe') })
    if ($applications.Count -ne 1) {
        throw 'NSIS did not install exactly one expected desktop executable'
    }
    $installed = $true

    $keys = @('PRAVDA_DB_PATH', 'PRAVDA_AUTH_PATH', 'PRAVDA_MARKET_FEEDS_ENABLED', 'PRAVDA_INSTALLED_EXE', 'PRAVDA_INSTALL_KIND', 'PRAVDA_ONBOARDING_REPORT')
    $previous = @{}
    foreach ($key in $keys) {
        $previous[$key] = [Environment]::GetEnvironmentVariable($key, 'Process')
    }
    $app = $null
    try {
        $env:PRAVDA_DB_PATH = $dbPath
        $env:PRAVDA_AUTH_PATH = $authPath
        $env:PRAVDA_MARKET_FEEDS_ENABLED = 'false'
        $app = Start-Process -FilePath $applications[0].FullName -PassThru
        $ready = $false
        for ($attempt = 0; $attempt -lt 60; $attempt++) {
            $app.Refresh()
            if ($app.HasExited) {
                throw "Installed NSIS desktop exited prematurely: $($app.ExitCode)"
            }
            if (Test-Path -LiteralPath $dbPath -PathType Leaf) {
                $ready = $true
                break
            }
            Start-Sleep -Milliseconds 500
        }
        if (-not $ready) {
            throw 'NSIS installed desktop did not initialize isolated SQLite'
        }
        Start-Sleep -Seconds 2
        $app.Refresh()
        if ($app.HasExited) {
            throw 'NSIS installed desktop unexpectedly exited after startup'
        }
        $listeners = @(Get-NetTCPConnection -State Listen -ErrorAction SilentlyContinue |
            Where-Object { $_.OwningProcess -eq $app.Id })
        if ($listeners.Count -ne 0) {
            throw 'NSIS installed desktop opened an unexpected TCP listener'
        }
        if (Test-Path -LiteralPath $authPath) {
            throw 'NSIS desktop created password verifier before onboarding'
        }
        Write-Host 'PASS: silent NSIS install and real desktop startup with isolated SQLite'
        # Check the real installed executable after process restart; using the
        # same file prevents a false PASS from a fresh empty database.
        Stop-Process -Id $app.Id -Force -ErrorAction Stop
        if (-not $app.WaitForExit(10000)) {
            throw 'Installed NSIS desktop did not exit before restart'
        }
        $header = [System.IO.File]::ReadAllBytes($dbPath)
        if ($header.Length -lt 16 -or
            [System.Text.Encoding]::ASCII.GetString($header, 0, 16) -cne ("SQLite format 3" + [char]0)) {
            throw 'NSIS first launch did not preserve an intact SQLite file'
        }
        $app = Start-Process -FilePath $applications[0].FullName -PassThru
        Start-Sleep -Seconds 3
        $app.Refresh()
        if ($app.HasExited) {
            throw "Installed NSIS desktop exited on second launch: $($app.ExitCode)"
        }
        if (-not (Test-Path -LiteralPath $dbPath -PathType Leaf) -or
            (Get-Item -LiteralPath $dbPath).Length -lt 16) {
            throw 'NSIS second launch failed to retain the original SQLite file'
        }
        if (Test-Path -LiteralPath $authPath) {
            throw 'NSIS desktop created a password verifier without onboarding after restart'
        }
        $secondListeners = @(Get-NetTCPConnection -State Listen -ErrorAction SilentlyContinue |
            Where-Object { $_.OwningProcess -eq $app.Id })
        if ($secondListeners.Count -ne 0) {
            throw 'NSIS installed desktop opened a TCP listener after restart'
        }
        Write-Host 'PASS: NSIS installed release desktop restarts with its existing SQLite and no implicit auth'
        # Execute a REAL first-use graphical WebView2 flow on the installed
        # release EXE, after the ordinary two-launch smoke. Synthetic storage.
        Stop-Process -Id $app.Id -Force -ErrorAction Stop
        if (-not $app.WaitForExit(10000)) {
            throw 'NSIS installed desktop did not stop for graphical onboarding acceptance'
        }
        $env:PRAVDA_INSTALLED_EXE = $applications[0].FullName
        $env:PRAVDA_INSTALL_KIND = 'NSIS'
        $env:PRAVDA_ONBOARDING_REPORT = Join-Path $env:RUNNER_TEMP 'Pravda-nsis-installed-onboarding.json'
        if ($env:PRAVDA_AIRGAP_ACCEPTANCE -ceq 'true') {
            & (Join-Path $PSScriptRoot 'Initialize-Pravda-Airgap-Gui.ps1')
        }
        if ($env:PRAVDA_SOURCE_SHA -notmatch '^[0-9a-fA-F]{40}
        if ($LASTEXITCODE -ne 0) {
            throw 'NSIS installed production GUI onboarding acceptance failed'
        }
        $uiEvidence = Get-Content -LiteralPath $env:PRAVDA_ONBOARDING_REPORT -Raw -ErrorAction Stop | ConvertFrom-Json
        if ($uiEvidence.version -ne 1 -or $uiEvidence.passed -ne $true -or
            $uiEvidence.kind -cne 'NSIS' -or $uiEvidence.sha -cne $env:PRAVDA_SOURCE_SHA -or
            @($uiEvidence.tests).Count -lt 14 -or
            @($uiEvidence.tests | Where-Object { $_.passed -ne $true }).Count -ne 0) {
            throw 'NSIS native GUI onboarding evidence is missing or invalid'
        }
        Write-Host 'PASS: NSIS installed production WebView2 first-use setup, restart, login and logout'

    }
    finally {
        if ($null -ne $app) {
            $app.Refresh()
            if (-not $app.HasExited) {
                Stop-Process -Id $app.Id -Force -ErrorAction Stop
                $app.WaitForExit(10000) | Out-Null
            }
        }
        foreach ($key in $keys) {
            [Environment]::SetEnvironmentVariable($key, $previous[$key], 'Process')
        }
    }

    foreach ($canary in $canaries) {
        New-Item -ItemType Directory -Path (Split-Path -Parent $canary) -Force | Out-Null
        Set-Content -LiteralPath $canary -Value $canaryText -NoNewline -Encoding utf8
    }
    $uninstallers = @(Get-ChildItem -LiteralPath $destination -Filter '*.exe' -File -Recurse -ErrorAction Stop |
        Where-Object { $_.Name -match '^(?i:uninstall|unins[0-9]+)\.exe$' })
    if ($uninstallers.Count -ne 1) {
        throw "NSIS expected exactly one private uninstaller; got $($uninstallers.Count)"
    }
    $uninstaller = Start-Process -FilePath $uninstallers[0].FullName -ArgumentList '/S' -PassThru -Wait
    if ($uninstaller.ExitCode -ne 0) {
        throw "NSIS silent uninstall failed with code $($uninstaller.ExitCode)"
    }
    $uninstalled = $true
    if (@(Get-PravdaNsisRegistration).Count -ne 0) {
        throw 'NSIS product registration remains after silent uninstall'
    }
    foreach ($canary in $canaries) {
        if (-not (Test-Path -LiteralPath $canary -PathType Leaf) -or
            (Get-Content -LiteralPath $canary -Raw) -cne $canaryText) {
            throw 'NSIS uninstall removed or changed per-user profile data'
        }
    }
    $testSucceeded = $true
    Write-Host 'PASS: NSIS silent uninstall retains both synthetic per-user files'
}
finally {
    foreach ($canary in $canaries) {
        if ((Test-Path -LiteralPath $canary -PathType Leaf) -and
            (Get-Content -LiteralPath $canary -Raw) -ceq $canaryText) {
            Remove-Item -LiteralPath $canary -Force -ErrorAction SilentlyContinue
        }
    }
    # Only remove a directory created under the disposable runner root.
    if (Test-Path -LiteralPath $root -PathType Container) {
        Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
    }
    if (-not $testSucceeded) {
        Write-Warning "NSIS smoke failed (installed=$installed, uninstalled=$uninstalled)"
    }
}
) {
            throw 'Installed GUI acceptance requires the exact source SHA'
        }
        node (Join-Path $PSScriptRoot 'test-installed-onboarding.mjs')
        if ($LASTEXITCODE -ne 0) {
            throw 'NSIS installed production GUI onboarding acceptance failed'
        }
        $uiEvidence = Get-Content -LiteralPath $env:PRAVDA_ONBOARDING_REPORT -Raw -ErrorAction Stop | ConvertFrom-Json
        if ($uiEvidence.version -ne 1 -or $uiEvidence.passed -ne $true -or
            $uiEvidence.kind -cne 'NSIS' -or $uiEvidence.sha -cne $env:PRAVDA_SOURCE_SHA -or
            @($uiEvidence.tests).Count -lt 14 -or
            @($uiEvidence.tests | Where-Object { $_.passed -ne $true }).Count -ne 0) {
            throw 'NSIS native GUI onboarding evidence is missing or invalid'
        }
        Write-Host 'PASS: NSIS installed production WebView2 first-use setup, restart, login and logout'

    }
    finally {
        if ($null -ne $app) {
            $app.Refresh()
            if (-not $app.HasExited) {
                Stop-Process -Id $app.Id -Force -ErrorAction Stop
                $app.WaitForExit(10000) | Out-Null
            }
        }
        foreach ($key in $keys) {
            [Environment]::SetEnvironmentVariable($key, $previous[$key], 'Process')
        }
    }

    foreach ($canary in $canaries) {
        New-Item -ItemType Directory -Path (Split-Path -Parent $canary) -Force | Out-Null
        Set-Content -LiteralPath $canary -Value $canaryText -NoNewline -Encoding utf8
    }
    $uninstallers = @(Get-ChildItem -LiteralPath $destination -Filter '*.exe' -File -Recurse -ErrorAction Stop |
        Where-Object { $_.Name -match '^(?i:uninstall|unins[0-9]+)\.exe$' })
    if ($uninstallers.Count -ne 1) {
        throw "NSIS expected exactly one private uninstaller; got $($uninstallers.Count)"
    }
    $uninstaller = Start-Process -FilePath $uninstallers[0].FullName -ArgumentList '/S' -PassThru -Wait
    if ($uninstaller.ExitCode -ne 0) {
        throw "NSIS silent uninstall failed with code $($uninstaller.ExitCode)"
    }
    $uninstalled = $true
    if (@(Get-PravdaNsisRegistration).Count -ne 0) {
        throw 'NSIS product registration remains after silent uninstall'
    }
    foreach ($canary in $canaries) {
        if (-not (Test-Path -LiteralPath $canary -PathType Leaf) -or
            (Get-Content -LiteralPath $canary -Raw) -cne $canaryText) {
            throw 'NSIS uninstall removed or changed per-user profile data'
        }
    }
    $testSucceeded = $true
    Write-Host 'PASS: NSIS silent uninstall retains both synthetic per-user files'
}
finally {
    foreach ($canary in $canaries) {
        if ((Test-Path -LiteralPath $canary -PathType Leaf) -and
            (Get-Content -LiteralPath $canary -Raw) -ceq $canaryText) {
            Remove-Item -LiteralPath $canary -Force -ErrorAction SilentlyContinue
        }
    }
    # Only remove a directory created under the disposable runner root.
    if (Test-Path -LiteralPath $root -PathType Container) {
        Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
    }
    if (-not $testSucceeded) {
        Write-Warning "NSIS smoke failed (installed=$installed, uninstalled=$uninstalled)"
    }
}

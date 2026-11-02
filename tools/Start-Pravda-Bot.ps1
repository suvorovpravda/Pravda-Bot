# Browser fallback: uses the independent Rust server, not the Tauri WebView.
# Data is kept in LOCALAPPDATA and never deleted by this launcher.
[CmdletBinding()]
param(
    [switch]$NoBrowser,
    [string]$DataDirectory = "",
    [ValidateRange(1, 65535)]
    [int]$Port = 8080
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$serverExe = Join-Path $root "pravda-server.exe"
if (-not (Test-Path -LiteralPath $serverExe -PathType Leaf)) {
    throw "Serveur Pravda-Bot introuvable. Extraire l'archive ZIP complete."
}
if (-not $DataDirectory) {
    $localData = [Environment]::GetFolderPath("LocalApplicationData")
    if (-not $localData) { throw "LOCALAPPDATA indisponible" }
    $DataDirectory = Join-Path $localData "Pravda-Bot"
}
[System.IO.Directory]::CreateDirectory($DataDirectory) | Out-Null
$env:PRAVDA_DB_PATH = Join-Path $DataDirectory "pravda.db"
$env:PRAVDA_AUTH_PATH = Join-Path $DataDirectory "pravda_auth.json"
$env:DASHBOARD_PORT = "$Port"
$address = "http://127.0.0.1:$Port"
# Refuse all occupied ports, including a service that impersonates Pravda-Bot.
$socket = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, $Port)
try {
    $socket.Start()
} catch {
    throw "Le port $Port est utilise par un autre programme. Fermer les anciennes instances Pravda-Bot avant de relancer."
} finally {
    $socket.Stop()
}
$logs = Join-Path $DataDirectory "logs"
[System.IO.Directory]::CreateDirectory($logs) | Out-Null
$stdout = Join-Path $logs "browser-server.out.log"
$stderr = Join-Path $logs "browser-server.err.log"
# Start-Process intentionally detaches the server: closing the browser must NOT
# terminate it and lose the login session. Private logs remain under LOCALAPPDATA.
$child = Start-Process -FilePath $serverExe -WorkingDirectory $root -WindowStyle Hidden -PassThru -RedirectStandardOutput $stdout -RedirectStandardError $stderr
$ready = $false
try {
    for ($attempt = 0; $attempt -lt 100; $attempt++) {
        $child.Refresh()
        if ($child.HasExited) { throw "Le serveur s'est arrete au demarrage. Journaux : $logs" }
        try {
            $response = Invoke-WebRequest -Uri "$address/login" -TimeoutSec 2 -UseBasicParsing -MaximumRedirection 0
            if ($response.StatusCode -eq 200 -and $response.Headers["x-pravda-server"] -eq "1") {
                $ready = $true
                break
            }
        } catch { }
        Start-Sleep -Milliseconds 200
    }
    if (-not $ready) { throw "Le serveur local ne repond pas. Journaux : $logs" }
    if ($NoBrowser) {
        Write-Output "PRAVDA_READY:$($child.Id)"
    } else {
        Start-Process "$address/login"
        Write-Host "Pravda-Bot est ouvert dans votre navigateur. Donnees : $DataDirectory"
    }
} catch {
    if (-not $child.HasExited) { Stop-Process -Id $child.Id -Force -ErrorAction SilentlyContinue }
    throw
}

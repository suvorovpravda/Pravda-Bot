# End-to-end test: real Windows exe, real form submit and persistence after restart.
$ErrorActionPreference = "Stop"
$exe = Join-Path $PSScriptRoot "release-stage/pravda-server.exe"
$launcher = Join-Path $PSScriptRoot "release-stage/Start-Pravda-Bot.ps1"
if (-not (Test-Path $exe)) { throw "Server binary not packaged" }
$probe = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
$probe.Start()
$port = $probe.LocalEndpoint.Port
$probe.Stop()
$data = Join-Path $env:RUNNER_TEMP ("pravda-browser-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Force $data | Out-Null
$env:PRAVDA_MARKET_FEEDS_ENABLED = "false"
$childId = 0
try {
    $first = & $launcher -NoBrowser -DataDirectory $data -Port $port
    $ready = [string]($first | Where-Object { $_ -match '^PRAVDA_READY:\d+$' } | Select-Object -Last 1)
    if (-not $ready) { throw "Browser launcher did not return server process id" }
    $childId = [int]$ready.Split(':')[1]
    $base = "http://127.0.0.1:$port"
    $handler = [System.Net.Http.HttpClientHandler]::new()
    $handler.UseCookies = $true
    $handler.AllowAutoRedirect = $false
    $handler.CookieContainer = [System.Net.CookieContainer]::new()
    $http = [System.Net.Http.HttpClient]::new($handler)
    try {
        $login = $http.GetAsync("$base/login").GetAwaiter().GetResult()
        $loginPage = $login.Content.ReadAsStringAsync().GetAwaiter().GetResult()
        if (-not $loginPage.Contains("Nouveau mot de passe")) {
            throw "First launch did not expose initial setup form"
        }
        $password = [Convert]::ToHexString([System.Security.Cryptography.RandomNumberGenerator]::GetBytes(20))
        $form = [System.Collections.Generic.Dictionary[string,string]]::new()
        $form.Add("password", $password)
        $form.Add("confirm", $password)
        $setup = $http.PostAsync("$base/setup", [System.Net.Http.FormUrlEncodedContent]::new($form)).GetAwaiter().GetResult()
        if ([int]$setup.StatusCode -ne 303) {
            throw "Setup password was rejected"
        }
        $authFile = Join-Path $data "pravda_auth.json"
        if (-not (Test-Path $authFile)) { throw "Local password was not persisted" }
        $record = Get-Content -Raw $authFile
        if ($record.Contains($password)) { throw "Plaintext password present in storage" }
        $recordJson = $record | ConvertFrom-Json
        if ($recordJson.algorithm -ne "pbkdf2-sha256" -or -not $recordJson.hash) {
            throw "Invalid password hash file"
        }
        $dash = $http.GetAsync("$base/").GetAwaiter().GetResult()
        if ([int]$dash.StatusCode -ne 200) { throw "Dashboard failed after creating password" }
    } finally {
        $http.Dispose()
        $handler.Dispose()
    }
    Stop-Process -Id $childId -Force -ErrorAction Stop
    Wait-Process -Id $childId -ErrorAction SilentlyContinue
    $childId = 0
    # Ensure a new process, without PRAVDA_ADMIN_PASSWORD, reads the saved hash.
    Remove-Item Env:PRAVDA_ADMIN_PASSWORD -ErrorAction SilentlyContinue
    $second = & $launcher -NoBrowser -DataDirectory $data -Port $port
    $ready2 = [string]($second | Where-Object { $_ -match '^PRAVDA_READY:\d+$' } | Select-Object -Last 1)
    if (-not $ready2) { throw "Server did not restart with saved data" }
    $childId = [int]$ready2.Split(':')[1]
    $handler = [System.Net.Http.HttpClientHandler]::new()
    $handler.UseCookies = $true
    $handler.AllowAutoRedirect = $false
    $handler.CookieContainer = [System.Net.CookieContainer]::new()
    $http = [System.Net.Http.HttpClient]::new($handler)
    try {
        $loginPage = $http.GetAsync("$base/login").GetAwaiter().GetResult().Content.ReadAsStringAsync().GetAwaiter().GetResult()
        if (-not $loginPage.Contains("Entrez le mot de passe local.") -or $loginPage.Contains("Nouveau mot de passe")) {
            throw "Password setup was not remembered after restart"
        }
        $auth = [System.Collections.Generic.Dictionary[string,string]]::new()
        $auth.Add("password", $password)
        $res = $http.PostAsync("$base/login", [System.Net.Http.FormUrlEncodedContent]::new($auth)).GetAwaiter().GetResult()
        if ([int]$res.StatusCode -ne 303) { throw "Saved password cannot authenticate after restart" }
        $dashboard = $http.GetAsync("$base/").GetAwaiter().GetResult()
        if ([int]$dashboard.StatusCode -ne 200) { throw "Dashboard failed after re-login" }
    } finally {
        $http.Dispose()
        $handler.Dispose()
    }
    Write-Host "REAL WINDOWS TEST PASSED: create password, hash on disk, dashboard, restart, relogin."
} finally {
    if ($childId) { Stop-Process -Id $childId -Force -ErrorAction SilentlyContinue }
    Remove-Item -Recurse -Force -LiteralPath $data -ErrorAction SilentlyContinue
}

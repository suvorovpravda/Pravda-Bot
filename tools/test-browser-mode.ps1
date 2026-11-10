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
        # Reproduce the actual WebView2/Edge top-level navigation to /setup.
        $setupNavigation = [System.Net.Http.HttpRequestMessage]::new(
            [System.Net.Http.HttpMethod]::Get, "$base/setup")
        $setupNavigation.Headers.TryAddWithoutValidation("Sec-Fetch-Site", "cross-site") | Out-Null
        $setupNavigation.Headers.TryAddWithoutValidation("Sec-Fetch-Mode", "navigate") | Out-Null
        $setupNavigation.Headers.TryAddWithoutValidation("Sec-Fetch-Dest", "document") | Out-Null
        $setupPage = $http.SendAsync($setupNavigation).GetAwaiter().GetResult()
        if ([int]$setupPage.StatusCode -ne 200) {
            throw "Cross-site GET /setup must show the first-run form"
        }
        $form = [System.Collections.Generic.Dictionary[string,string]]::new()
        $form.Add("password", $password)
        $form.Add("confirm", $password)
        # An attacker-supplied Origin must remain denied, even with navigation metadata.
        $attack = [System.Net.Http.HttpRequestMessage]::new(
            [System.Net.Http.HttpMethod]::Post, "$base/setup")
        $attack.Headers.TryAddWithoutValidation("Origin", "https://attacker.invalid") | Out-Null
        $attack.Headers.TryAddWithoutValidation("Sec-Fetch-Site", "cross-site") | Out-Null
        $attack.Headers.TryAddWithoutValidation("Sec-Fetch-Mode", "navigate") | Out-Null
        $attack.Headers.TryAddWithoutValidation("Sec-Fetch-Dest", "document") | Out-Null
        $attack.Content = [System.Net.Http.FormUrlEncodedContent]::new($form)
        $blocked = $http.SendAsync($attack).GetAwaiter().GetResult()
        if ([int]$blocked.StatusCode -ne 403) {
            throw "Unsafe cross-site password setup was accepted"
        }
        if (Test-Path (Join-Path $data "pravda_auth.json")) {
            throw "Rejected attacker form managed to create an authentication file"
        }
        $navigation = [System.Net.Http.HttpRequestMessage]::new(
            [System.Net.Http.HttpMethod]::Post, "$base/setup")
        $navigation.Headers.TryAddWithoutValidation("Origin", "$base") | Out-Null
        $navigation.Headers.TryAddWithoutValidation("Sec-Fetch-Site", "cross-site") | Out-Null
        $navigation.Headers.TryAddWithoutValidation("Sec-Fetch-Mode", "navigate") | Out-Null
        $navigation.Headers.TryAddWithoutValidation("Sec-Fetch-Dest", "document") | Out-Null
        $navigation.Content = [System.Net.Http.FormUrlEncodedContent]::new($form)
        $setup = $http.SendAsync($navigation).GetAwaiter().GetResult()
        if ([int]$setup.StatusCode -ne 303) {
            throw "Browser-style password setup rejected HTTP $([int]$setup.StatusCode)"
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
        # The browser compatibility API must reuse Rust feed adapters while
        # refusing all network traffic under the explicit offline setting.
        # Exercise real authenticated HTTP routes, not only source assertions.
        foreach ($route in @('/api/fx', '/api/energy', '/api/eu/flows', '/api/news', '/api/geopolitics')) {
            $response = $http.GetAsync("$base$route").GetAwaiter().GetResult()
            if ([int]$response.StatusCode -ne 200) {
                throw "Public feed $route did not return a typed offline response"
            }
            $body = $response.Content.ReadAsStringAsync().GetAwaiter().GetResult() | ConvertFrom-Json
            if ($body.status -ne 'unavailable' -or $body.notice -ne 'PUBLIC_FEEDS_DISABLED') {
                throw "Public feed $route attempted an online fetch or forged a successful offline result"
            }
        }
    } finally {
        $http.Dispose()
        $handler.Dispose()
    }
    Write-Host "REAL WINDOWS TEST PASSED: create password, hash on disk, dashboard, restart, relogin."
} finally {
    if ($childId) { Stop-Process -Id $childId -Force -ErrorAction SilentlyContinue }
    Remove-Item -Recurse -Force -LiteralPath $data -ErrorAction SilentlyContinue
}

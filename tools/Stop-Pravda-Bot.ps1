# Stop only Pravda-Bot's own server executable in this distribution.
$ErrorActionPreference = "Stop"
$expected = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "pravda-server.exe"))
$stopped = 0
Get-CimInstance Win32_Process -Filter "Name = 'pravda-server.exe'" | ForEach-Object {
    if ($_.ExecutablePath -and [string]::Equals(
        [System.IO.Path]::GetFullPath($_.ExecutablePath),
        $expected,
        [System.StringComparison]::OrdinalIgnoreCase
    )) {
        Stop-Process -Id $_.ProcessId -Force
        $stopped++
    }
}
Write-Host "Pravda-Bot : $stopped processus arrete(s)."

@echo off
setlocal

rem Ensure port env var
if "%DASHBOARD_PORT%"=="" set DASHBOARD_PORT=8080

rem Start server in a new window
start "Pravda Server" cmd /c "cargo run -p pravda-server"

rem Wait for the server to start (adjust if needed)
timeout /t 2 >nul

rem Open default browser to dashboard
start "Dashboard" http://127.0.0.1:%DASHBOARD_PORT%/

endlocal
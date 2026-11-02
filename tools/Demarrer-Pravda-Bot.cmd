@echo off
setlocal
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0Start-Pravda-Bot.ps1"
if errorlevel 1 (
 echo.
 echo Echec du demarrage. Consultez %%LOCALAPPDATA%%\Pravda-Bot\logs
 pause
)

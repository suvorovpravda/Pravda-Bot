@echo off
setlocal
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0Stop-Pravda-Bot.ps1"
if errorlevel 1 pause

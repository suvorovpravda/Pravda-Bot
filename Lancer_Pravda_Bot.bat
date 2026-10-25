@echo off
setlocal
cd /d "%~dp0"
set "PYTHONUTF8=1"

set "PYTHON_EXE="
if exist "%~dp0.venv\Scripts\python.exe" set "PYTHON_EXE=%~dp0.venv\Scripts\python.exe"
if not defined PYTHON_EXE (
    for /f "delims=" %%P in ('py -3.12 -c "import sys; print(sys.executable)" 2^>nul') do set "PYTHON_EXE=%%P"
)

if not defined PYTHON_EXE (
    echo Python 3.12 est introuvable. Installe Python 3.12 ou cree un environnement .venv dans ce dossier.
    pause
    exit /b 1
)

"%PYTHON_EXE%" -c "import requests, websockets, dotenv, rich, psutil" >nul 2>nul
if errorlevel 1 (
    echo Des dependances Python sont manquantes.
    echo Execute une fois : "%PYTHON_EXE%" -m pip install -r requirements.txt
    pause
    exit /b 1
)

start "Pravda-Bot - Flux" /D "%~dp0" "%PYTHON_EXE%" main.py
start "Pravda-Bot - Dashboard" /D "%~dp0" "%PYTHON_EXE%" dashboard.py --port 8080
timeout /t 2 /nobreak >nul

if exist "%ProgramFiles%\Google\Chrome\Application\chrome.exe" (
    start "" "%ProgramFiles%\Google\Chrome\Application\chrome.exe" "http://127.0.0.1:8080"
) else if exist "%ProgramFiles(x86)%\Google\Chrome\Application\chrome.exe" (
    start "" "%ProgramFiles(x86)%\Google\Chrome\Application\chrome.exe" "http://127.0.0.1:8080"
) else (
    start "" "http://127.0.0.1:8080"
)

echo Pravda-Bot et le dashboard sont demarres.
echo Laisse les deux fenetres ouvertes pour voir les donnees en direct.
echo Pour arreter, ferme les fenetres du bot et du dashboard.
endlocal

@echo off
setlocal EnableExtensions DisableDelayedExpansion
cd /d "%~dp0"
if errorlevel 1 exit /b 1
set "PRAVDA_PY=%~dp0.venv\Scripts\python.exe"
set "PRAVDA_LOG=%~dp0installation-pravda.log"
set "PYTHONUTF8=1"
set "PYTHONIOENCODING=utf-8"
set "PIP_DISABLE_PIP_VERSION_CHECK=1"
set "PIP_NO_INPUT=1"
> "%PRAVDA_LOG%" echo Installation Pravda : Python et dependances verrouillees.
if not exist ".python-version" goto erreur
if not exist "%PRAVDA_PY%" (
    py -3.12 -m venv "%~dp0.venv" >> "%PRAVDA_LOG%" 2>&1
    if errorlevel 1 goto erreur
)
"%PRAVDA_PY%" -c "from pathlib import Path; import platform,sys; expected=Path('.python-version').read_text().strip(); actual=platform.python_version(); print('Python attendu : '+expected+' / actif : '+actual); sys.exit(0 if actual == expected else 1)" >> "%PRAVDA_LOG%" 2>&1
if errorlevel 1 goto erreur
"%PRAVDA_PY%" -m pip install --require-hashes --only-binary=:all: -r requirements-bootstrap.txt >> "%PRAVDA_LOG%" 2>&1
if errorlevel 1 goto erreur
"%PRAVDA_PY%" -m pip install --require-hashes --only-binary=:all: -r requirements.txt >> "%PRAVDA_LOG%" 2>&1
if errorlevel 1 goto erreur
"%PRAVDA_PY%" -m pip check >> "%PRAVDA_LOG%" 2>&1
if errorlevel 1 goto erreur
if not exist ".env" copy ".env.example" ".env" >nul
if errorlevel 1 goto erreur
set "NTFY_TOPIC_URL="
"%PRAVDA_PY%" pravda_launcher.py
if errorlevel 1 goto erreur
exit /b 0
:erreur
echo [ERREUR] Installation interrompue. Python doit correspondre a .python-version.
if exist "%PRAVDA_LOG%" type "%PRAVDA_LOG%"
echo [INFO] Journal : "%PRAVDA_LOG%"
pause
exit /b 1

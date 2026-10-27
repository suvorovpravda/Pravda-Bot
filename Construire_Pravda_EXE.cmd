@echo off
setlocal EnableExtensions DisableDelayedExpansion
cd /d "%~dp0"
if errorlevel 1 exit /b 1
set "PRAVDA_RUNTIME=%~dp0.venv\Scripts\python.exe"
set "PRAVDA_BUILD=%~dp0.build-venv\Scripts\python.exe"
set "PRAVDA_LOG=%~dp0construction-pravda.log"
set "PYTHONUTF8=1"
set "PYTHONIOENCODING=utf-8"
set "PIP_DISABLE_PIP_VERSION_CHECK=1"
set "PIP_NO_INPUT=1"
> "%PRAVDA_LOG%" echo Construction du lanceur dans un environnement separe.
if not exist "pravda_launcher.py" goto failed
if not exist "main.py" goto failed
if not exist "dashboard.py" goto failed
if not exist ".python-version" goto failed
if not exist "%PRAVDA_RUNTIME%" goto failed
"%PRAVDA_RUNTIME%" -c "from pathlib import Path; import platform,sys; expected=Path('.python-version').read_text().strip(); sys.exit(0 if platform.python_version() == expected else 'Version Python differente de .python-version')" >> "%PRAVDA_LOG%" 2>&1
if errorlevel 1 goto failed
"%PRAVDA_RUNTIME%" -m pip check >> "%PRAVDA_LOG%" 2>&1
if errorlevel 1 goto failed
if not exist "%PRAVDA_BUILD%" (
    "%PRAVDA_RUNTIME%" -m venv "%~dp0.build-venv" >> "%PRAVDA_LOG%" 2>&1
    if errorlevel 1 goto failed
)
"%PRAVDA_BUILD%" -c "from pathlib import Path; import platform,sys; expected=Path('.python-version').read_text().strip(); sys.exit(0 if platform.python_version() == expected else 'Version Python de construction incorrecte')" >> "%PRAVDA_LOG%" 2>&1
if errorlevel 1 goto failed
"%PRAVDA_BUILD%" -m pip install --require-hashes --only-binary=:all: -r requirements-bootstrap.txt >> "%PRAVDA_LOG%" 2>&1
if errorlevel 1 goto failed
"%PRAVDA_BUILD%" -m pip install --require-hashes --only-binary=:all: -r requirements-build.txt >> "%PRAVDA_LOG%" 2>&1
if errorlevel 1 goto failed
"%PRAVDA_BUILD%" -m pip check >> "%PRAVDA_LOG%" 2>&1
if errorlevel 1 goto failed
"%PRAVDA_RUNTIME%" -m unittest discover -s launcher_tests -v >> "%PRAVDA_LOG%" 2>&1
if errorlevel 1 goto failed
"%PRAVDA_BUILD%" -m PyInstaller --noconfirm --clean --onefile --console --noupx --name Pravda_Lanceur --distpath "." --workpath ".pravda-exe-build" --specpath ".pravda-exe-spec" pravda_launcher.py >> "%PRAVDA_LOG%" 2>&1
if errorlevel 1 goto failed
if not exist "Pravda_Lanceur.exe" goto failed
echo [OK] Pravda_Lanceur.exe cree. Journal : "%PRAVDA_LOG%"
pause
exit /b 0
:failed
echo [ERREUR] Construction interrompue. Installer le runtime avec Installer_et_lancer.cmd.
if exist "%PRAVDA_LOG%" type "%PRAVDA_LOG%"
pause
exit /b 1

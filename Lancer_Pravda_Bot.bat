@echo off
setlocal EnableExtensions DisableDelayedExpansion
cd /d "%~dp0"
if errorlevel 1 exit /b 1

set "PRAVDA_PYTHON=%~dp0.venv\Scripts\python.exe"
if not exist "%PRAVDA_PYTHON%" (
    echo [ERREUR] Environnement .venv absent. Voir le guide d'installation.
    pause
    exit /b 1
)
if not exist "main.py" goto missing_files
if not exist "dashboard.py" goto missing_files
if not exist "bot_config.py" goto missing_files

rem Settings for this launch only. Existing .env is not modified.
rem LM_MODEL_NAME is read from the environment or the existing .env.
set "LM_STUDIO_API_BASE=http://127.0.0.1:1234/v1"
set "TRADING_MODE=paper"
set "PAPER_MODE=portfolio"
set "PAPER_STARTING_BALANCE_USDT=1000"
set "PAPER_POSITION_PERCENT=10"
set "MIN_SPREAD_THRESHOLD=0.05"
set "MAX_ALLOWED_SPREAD_PERCENT=5"
set "ALERT_CONFIRMATION_SECONDS=3"
set "ALERT_REARM_HYSTERESIS_PERCENT=0.05"
set "MARKET_DATA_STALE_SECONDS=4"
set "MARKET_DATA_MAX_SKEW_SECONDS=3"
set "SPREAD_LOG_INTERVAL_SECONDS=2"
if not defined DASHBOARD_PORT set "DASHBOARD_PORT=8080"

if /i "%~1"=="--bot" goto run_bot
if /i "%~1"=="--dashboard" goto run_dashboard
if not "%~1"=="" (
    echo Usage: Lancer_Pravda_Bot.bat
    exit /b 1
)

echo [INFO] Validation de la configuration et des dependances...
"%PRAVDA_PYTHON%" -c "import os; from dotenv import load_dotenv; os.environ['NTFY_TOPIC_URL']=''; load_dotenv(); from bot_config import load_config; load_config(); import requests, websockets, rich, psutil"
if errorlevel 1 goto preflight_failed

echo [INFO] Verification du port local configure...
"%PRAVDA_PYTHON%" -c "import os, socket, sys; port=int(os.environ['DASHBOARD_PORT']); (1 <= port <= 65535) or sys.exit('DASHBOARD_PORT must be between 1 and 65535'); s=socket.socket(); s.bind(('127.0.0.1', port)); s.close()"
if errorlevel 1 (
    echo [ERREUR] Le port demande est occupe, inaccessible ou invalide. Aucun processus lance.
    echo [INFO] Autre port dans PowerShell : $env:DASHBOARD_PORT='8081'
    pause
    exit /b 1
)

echo [INFO] Execution de la suite de tests avant lancement...
"%PRAVDA_PYTHON%" -m unittest discover -s tests -t . -q
if errorlevel 1 (
    echo [ERREUR] Des tests ont echoue. Lancement interrompu.
    pause
    exit /b 1
)

start "Pravda - Dashboard local" "%ComSpec%" /D /K call Lancer_Pravda_Bot.bat --dashboard
if errorlevel 1 goto launch_failed
start "Pravda - Simulation papier" "%ComSpec%" /D /K call Lancer_Pravda_Bot.bat --bot
if errorlevel 1 goto launch_failed
echo [INFO] Deux consoles ouvertes. Dashboard : http://127.0.0.1:%DASHBOARD_PORT%
echo [INFO] Metriques : http://127.0.0.1:%DASHBOARD_PORT%/api/metrics
echo [INFO] Ctrl+C dans chaque console pour arreter les deux processus.
exit /b 0

:run_bot
title Pravda - Simulation papier
rem Set the empty topic inside Python so dotenv cannot restore a local topic.
"%PRAVDA_PYTHON%" -c "import os, runpy; os.environ['NTFY_TOPIC_URL']=''; runpy.run_path('main.py', run_name='__main__')"
exit /b %errorlevel%

:run_dashboard
title Pravda - Dashboard local
"%PRAVDA_PYTHON%" dashboard.py --csv spread_observations.csv --threshold 0.05 --stale-after 30
exit /b %errorlevel%

:missing_files
echo [ERREUR] Placer ce fichier dans le dossier de Pravda-Bot, avec main.py et dashboard.py.
pause
exit /b 1

:preflight_failed
echo [ERREUR] Verifier les dependances et la configuration avec le guide fourni.
pause
exit /b 1

:launch_failed
echo [ERREUR] Impossible d'ouvrir une console. Verifier les consoles deja ouvertes.
pause
exit /b 1

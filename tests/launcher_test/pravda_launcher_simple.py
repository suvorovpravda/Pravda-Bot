"""Pravda-Bot launcher simplifié - version réécrite à partir de zéro.
Objectif : démarrage fiable en mode frozen/non-frozen, sans subprocess complexe.
"""
from __future__ import annotations
import argparse
import json
import os
import sys
import threading
import time
from pathlib import Path
import subprocess

READY_PREFIX = "PRAVDA_DASHBOARD_READY "

PAPER_SETTINGS = {
    "LM_STUDIO_API_BASE": "http://127.0.0.1:1234/v1",
    "TRADING_MODE": "paper",
    "PAPER_MODE": "portfolio",
    "PAPER_STARTING_BALANCE_USDT": "1000",
    "PAPER_POSITION_PERCENT": "10",
    "MIN_SPREAD_THRESHOLD": "0.05",
    "MAX_ALLOWED_SPREAD_PERCENT": "5",
    "ALERT_CONFIRMATION_SECONDS": "3",
    "ALERT_REARM_HYSTERESIS_PERCENT": "0.05",
    "MARKET_DATA_STALE_SECONDS": "4",
    "MARKET_DATA_MAX_SKEW_SECONDS": "3",
    "SPREAD_LOG_INTERVAL_SECONDS": "2",
    "NTFY_TOPIC_URL": "",
    "PYTHONUTF8": "1",
    "PYTHONIOENCODING": "utf-8",
}

PREFLIGHT = """
from dotenv import load_dotenv
load_dotenv()
from bot_config import load_config
config = load_config()
import requests, websockets, rich, psutil
if config.ntfy_topic_url:
    raise RuntimeError('Les notifications doivent etre desactivees pour ce lancement.')
print('Configuration et dependances valides.', flush=True)
"""

DASHBOARD_CODE = """
import json, sys
from pathlib import Path
from dotenv import load_dotenv
load_dotenv()
from dashboard import DashboardHandler
from http.server import ThreadingHTTPServer
from functools import partial

handler = partial(
    DashboardHandler,
    csv_path=Path('spread_observations.csv'),
    lng_csv_path=Path('lng_observations.csv'),
    threshold=0.05,
    stale_after_seconds=30.0,
)
server = ThreadingHTTPServer(('127.0.0.1', 0), handler)
print('PRAVDA_DASHBOARD_READY ' + json.dumps({'port': server.server_address[1]}), flush=True)
server.serve_forever()
"""

BOT_CODE = """
import runpy
runpy.run_path('main.py', run_name='__main__')
"""

def project_root() -> Path:
    if getattr(sys, "frozen", False):
        return Path(sys.executable).resolve().parent
    return Path(__file__).resolve().parent

def venv_python(root: Path) -> Path:
    rel = "Scripts/python.exe" if os.name == "nt" else "bin/python"
    return root / ".venv" / rel

def env_for():
    e = os.environ.copy()
    e.pop("PYTHONHOME", None)
    e.pop("PYTHONPATH", None)
    e.update(PAPER_SETTINGS)
    return e

def run_subprocess(label, cmd, cwd):
    print(f"[{label}] {' '.join(cmd)}")
    proc = subprocess.Popen(cmd, cwd=str(cwd), env=env_for(),
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                            text=True, bufsize=1)
    out_lines = []
    for line in proc.stdout:
        line = line.rstrip()
        print(f"[{label}] {line}")
        out_lines.append(line)
    rc = proc.wait()
    return rc, out_lines

def main():
    root = project_root()
    py = venv_python(root)
    # validation basique
    for name in ("main.py","dashboard.py","bot_config.py","tests"):
        if not (root/name).exists():
            print(f"Fichier/dossier manquant : {name}")
            sys.exit(2)
    if not py.is_file():
        print("Environnement .venv absent")
        sys.exit(2)

    print("Verification de la configuration et des dependances...")
    rc, _ = run_subprocess("Configuration", [str(py), "-X", "utf8", "-u", "-c", PREFLIGHT], root)
    if rc != 0:
        print("Preflight echoue")
        sys.exit(1)

    print("Execution de la suite de tests du projet...")
    rc, _ = run_subprocess("Tests", [str(py), "-m", "unittest", "discover", "-s", "tests", "-t", ".", "-q"], root)
    if rc != 0:
        print("Tests echoue")
        sys.exit(1)

    # Démarrage dashboard dans un thread subprocess
    dashboard_ready = threading.Event()
    dashboard_url = None

    def dashboard_thread():
        nonlocal dashboard_url
        proc = subprocess.Popen([str(py), "-X", "utf8", "-u", "-c", DASHBOARD_CODE],
                                cwd=str(root), env=env_for(),
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1)
        for line in proc.stdout:
            line = line.rstrip()
            if line.startswith(READY_PREFIX):
                try:
                    data = json.loads(line[len(READY_PREFIX):])
                    port = data["port"]
                    dashboard_url = f"http://127.0.0.1:{port}"
                    print(f"Dashboard : {dashboard_url}")
                    dashboard_ready.set()
                    break
                except Exception:
                    pass
            print(f"[Dashboard] {line}")

    t_dash = threading.Thread(target=dashboard_thread, daemon=True)
    t_dash.start()
    if not dashboard_ready.wait(timeout=20):
        print("Dashboard n'a pas demarre")
        sys.exit(1)

    # Démarrage bot dans un thread subprocess
    def bot_thread():
        proc = subprocess.Popen([str(py), "-X", "utf8", "-u", "-c", BOT_CODE],
                                cwd=str(root), env=env_for(),
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1)
        for line in proc.stdout:
            print(f"[Bot] {line.rstrip()}")

    t_bot = threading.Thread(target=bot_thread, daemon=True)
    t_bot.start()

    print("Simulation papier lancee. Ctrl+C pour arreter.")
    try:
        while True:
            time.sleep(0.5)
    except KeyboardInterrupt:
        print("Arret demande.")
        sys.exit(0)

if __name__ == "__main__":
    sys.exit(main())

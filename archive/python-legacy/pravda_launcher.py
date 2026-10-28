"""Console launcher for an existing Pravda-Bot checkout and virtual environment.

Only this standard-library launcher is frozen into the Windows executable.
The bot and the dashboard run with the project's own Python and dependencies.
"""

from __future__ import annotations

import argparse
import ctypes
import hashlib
import json
import os
from pathlib import Path
import queue
import subprocess
import sys
import tempfile
import threading
import time
import webbrowser


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

BOT_WORKER = """
import _thread, runpy, sys, threading
def watch_parent():
    for line in sys.stdin:
        if line.strip() == 'STOP':
            _thread.interrupt_main()
            return
threading.Thread(target=watch_parent, daemon=True).start()
try:
    runpy.run_path('main.py', run_name='__main__')
except KeyboardInterrupt:
    pass
"""

DASHBOARD_WORKER = """
import json, socket, sys, threading
from functools import partial
from http.server import ThreadingHTTPServer
from pathlib import Path
from dotenv import load_dotenv
load_dotenv()
from dashboard import DashboardHandler
class LocalServer(ThreadingHTTPServer):
    allow_reuse_address = False
    def server_bind(self):
        if hasattr(socket, 'SO_EXCLUSIVEADDRUSE'):
            self.socket.setsockopt(socket.SOL_SOCKET, socket.SO_EXCLUSIVEADDRUSE, 1)
        super().server_bind()
handler = partial(
    DashboardHandler,
    csv_path=Path('spread_observations.csv'),
    lng_csv_path=Path('lng_observations.csv'),
    threshold=0.05,
    stale_after_seconds=30.0,
)
# This is the real listening socket: it stays open for the server's lifetime.
server = LocalServer(('127.0.0.1', 0), handler)
print('PRAVDA_DASHBOARD_READY ' + json.dumps({'port': server.server_port}), flush=True)
def watch_parent():
    for line in sys.stdin:
        if line.strip() == 'STOP':
            server.shutdown()
            return
threading.Thread(target=watch_parent, daemon=True).start()
try:
    server.serve_forever(poll_interval=0.2)
finally:
    server.server_close()
"""


class LaunchError(RuntimeError):
    pass


def project_directory() -> Path:
    path = sys.executable if getattr(sys, "frozen", False) else __file__
    return Path(path).resolve().parent


def project_python(root: Path) -> Path:
    relative = "Scripts/python.exe" if os.name == "nt" else "bin/python"
    return root / ".venv" / relative


def runtime_environment(source=None):
    env = dict(os.environ if source is None else source)
    # External Python must not load libraries from the frozen application's home.
    for name in ("PYTHONHOME", "PYTHONPATH"):
        env.pop(name, None)
    env.update(PAPER_SETTINGS)
    return env


def validate_project(root: Path, python: Path):
    for name in ("main.py", "dashboard.py", "bot_config.py", "tests"):
        if not (root / name).exists():
            raise LaunchError(f"Fichier ou dossier manquant : {name}. Placer le lanceur dans Pravda-Bot.")
    if not python.is_file():
        raise LaunchError("Environnement .venv absent : installer Python 3.12 et les dependances du projet.")


def dashboard_url(message: str) -> str:
    data = json.loads(message)
    port = data.get("port") if isinstance(data, dict) else None
    if type(port) is not int or not 1 <= port <= 65535:
        raise LaunchError("Le dashboard a fourni un port invalide.")
    return f"http://127.0.0.1:{port}"


def _windows_kernel():
    from ctypes import wintypes

    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    handle = wintypes.HANDLE
    kernel.CloseHandle.argtypes = [handle]
    kernel.CloseHandle.restype = wintypes.BOOL
    kernel.CreateMutexW.argtypes = [ctypes.c_void_p, wintypes.BOOL, wintypes.LPCWSTR]
    kernel.CreateMutexW.restype = handle
    kernel.CreateJobObjectW.argtypes = [ctypes.c_void_p, wintypes.LPCWSTR]
    kernel.CreateJobObjectW.restype = handle
    kernel.SetInformationJobObject.argtypes = [handle, ctypes.c_int, ctypes.c_void_p, wintypes.DWORD]
    kernel.SetInformationJobObject.restype = wintypes.BOOL
    kernel.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
    kernel.OpenProcess.restype = handle
    kernel.AssignProcessToJobObject.argtypes = [handle, handle]
    kernel.AssignProcessToJobObject.restype = wintypes.BOOL
    kernel.SetDllDirectoryW.argtypes = [wintypes.LPCWSTR]
    kernel.SetDllDirectoryW.restype = wintypes.BOOL
    return kernel


class InstanceLock:
    """Protect against starting this launcher twice for the same checkout."""

    def __init__(self, root):
        canonical = os.path.normcase(str(root.resolve()))
        self.key = hashlib.sha256(canonical.encode("utf-8")).hexdigest()[:32]
        self.handle = None
        self.file = None

    def __enter__(self):
        if os.name == "nt":
            self.kernel = _windows_kernel()
            ctypes.set_last_error(0)
            self.handle = self.kernel.CreateMutexW(None, False, "Local\\PravdaLauncher_" + self.key)
            error = ctypes.get_last_error()
            if not self.handle:
                raise ctypes.WinError(error)
            if error == 183:  # ERROR_ALREADY_EXISTS
                self.kernel.CloseHandle(self.handle)
                self.handle = None
                raise LaunchError("Un lanceur Pravda est deja ouvert pour ce projet.")
        else:
            import fcntl

            path = Path(tempfile.gettempdir()) / ("pravda-launcher-" + self.key + ".lock")
            flags = os.O_RDWR | os.O_CREAT | getattr(os, "O_NOFOLLOW", 0)
            self.file = os.fdopen(os.open(path, flags, 0o600), "r+b")
            try:
                fcntl.flock(self.file, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except OSError as error:
                self.file.close()
                self.file = None
                raise LaunchError("Un lanceur Pravda est deja ouvert pour ce projet.") from error
        return self

    def __exit__(self, *_):
        if self.handle:
            self.kernel.CloseHandle(self.handle)
            self.handle = None
        if self.file:
            self.file.close()
            self.file = None


class WindowsJob:
    """Own only our child processes; close them if this console disappears."""

    def __enter__(self):
        self.handle = None
        if os.name != "nt":
            return self

        class BasicLimits(ctypes.Structure):
            _fields_ = [
                ("process_time", ctypes.c_int64), ("job_time", ctypes.c_int64),
                ("flags", ctypes.c_uint32), ("minimum_ws", ctypes.c_size_t),
                ("maximum_ws", ctypes.c_size_t), ("active_processes", ctypes.c_uint32),
                ("affinity", ctypes.c_size_t), ("priority", ctypes.c_uint32),
                ("scheduling", ctypes.c_uint32),
            ]

        class ExtendedLimits(ctypes.Structure):
            _fields_ = [
                ("basic", BasicLimits), ("io", ctypes.c_uint64 * 6),
                ("process_memory", ctypes.c_size_t), ("job_memory", ctypes.c_size_t),
                ("peak_process_memory", ctypes.c_size_t), ("peak_job_memory", ctypes.c_size_t),
            ]

        self.kernel = _windows_kernel()
        self.handle = self.kernel.CreateJobObjectW(None, None)
        if not self.handle:
            raise ctypes.WinError(ctypes.get_last_error())
        limits = ExtendedLimits()
        limits.basic.flags = 0x2000  # JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        if not self.kernel.SetInformationJobObject(self.handle, 9, ctypes.byref(limits), ctypes.sizeof(limits)):
            error = ctypes.get_last_error()
            self.kernel.CloseHandle(self.handle)
            self.handle = None
            raise ctypes.WinError(error)
        return self

    def assign(self, process):
        if not self.handle:
            return
        handle = self.kernel.OpenProcess(0x0100 | 0x0001, False, process.pid)
        if not handle:
            if process.poll() is not None:
                return
            raise ctypes.WinError(ctypes.get_last_error())
        try:
            if not self.kernel.AssignProcessToJobObject(self.handle, handle):
                if process.poll() is None:
                    raise ctypes.WinError(ctypes.get_last_error())
        finally:
            self.kernel.CloseHandle(handle)

    def __exit__(self, *_):
        if self.handle:
            self.kernel.CloseHandle(self.handle)
            self.handle = None


class Child:
    def __init__(self, label, command, root, env, job, output=print):
        self.label = label
        self.output = output
        self.lines = queue.Queue(maxsize=1000)
        self.ready = threading.Event()
        self.url = None
        self.protocol_error = None
        kernel = _windows_kernel() if os.name == "nt" and getattr(sys, "frozen", False) else None
        try:
            if kernel and not kernel.SetDllDirectoryW(None):
                raise ctypes.WinError(ctypes.get_last_error())
            self.process = subprocess.Popen(
                command, cwd=root, env=env, stdin=subprocess.PIPE,
                stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                text=True, encoding="utf-8", errors="replace", bufsize=1,
                close_fds=True,
                creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0,
            )
        finally:
            if kernel:
                kernel.SetDllDirectoryW(sys._MEIPASS)
        try:
            job.assign(self.process)
        except BaseException:
            self.process.terminate()
            self.process.wait(timeout=5)
            self.process.stdin.close()
            self.process.stdout.close()
            raise
        self.reader = threading.Thread(target=self._read, daemon=True)
        self.reader.start()

    def _read(self):
        try:
            for line in self.process.stdout:
                line = line.rstrip("\r\n")
                if self.label == "Dashboard" and line.startswith(READY_PREFIX):
                    try:
                        self.url = dashboard_url(line[len(READY_PREFIX):])
                    except (ValueError, LaunchError) as error:
                        self.protocol_error = str(error)
                    self.ready.set()
                    continue
                try:
                    self.lines.put_nowait(line[:4000])
                except queue.Full:
                    # Bounded display queue; readiness messages have their own channel.
                    try:
                        self.lines.get_nowait()
                    except queue.Empty:
                        pass
                    try:
                        self.lines.put_nowait(line[:4000])
                    except queue.Full:
                        pass
        finally:
            self.process.stdout.close()

    def drain(self):
        while True:
            try:
                line = self.lines.get_nowait()
            except queue.Empty:
                return
            self.output(f"[{self.label}] {line}")

    def request_stop(self):
        if self.process.poll() is None:
            try:
                self.process.stdin.write("STOP\n")
                self.process.stdin.flush()
            except (BrokenPipeError, OSError, ValueError):
                pass


def stop_children(children, grace=15):
    for child in children:
        child.request_stop()
    deadline = time.monotonic() + grace
    while any(child.process.poll() is None for child in children) and time.monotonic() < deadline:
        for child in children:
            child.drain()
        time.sleep(0.1)
    for child in children:
        if child.process.poll() is None:
            child.output(f"[{child.label}] Delai d'arret depasse : fermeture de ce processus.")
            child.process.terminate()
    for child in children:
        try:
            child.process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            child.process.kill()
            child.process.wait(timeout=3)
        child.process.stdin.close()
        child.reader.join(timeout=2)
        child.drain()


def wait_checked(child, stop, timeout):
    started = time.monotonic()
    while child.process.poll() is None:
        child.drain()
        if stop.is_set():
            raise KeyboardInterrupt
        if time.monotonic() - started > timeout:
            raise LaunchError(f"{child.label} : delai depasse.")
        time.sleep(0.05)
    child.reader.join(timeout=2)
    child.drain()
    if child.process.returncode:
        raise LaunchError(f"{child.label} a echoue (code {child.process.returncode}). Aucun bot demarre.")


def launch(root, python=None, *, stop=None, open_browser=True, output=print):
    root = Path(root).resolve()
    python = Path(python) if python is not None else project_python(root)
    validate_project(root, python)
    stop = threading.Event() if stop is None else stop
    env = runtime_environment()
    children = []
    with InstanceLock(root), WindowsJob() as job:
        def start(label, args):
            child = Child(label, [str(python), "-X", "utf8", "-u", *args], root, env, job, output)
            children.append(child)
            return child

        try:
            output("Verification de la configuration et des dependances...")
            wait_checked(start("Configuration", ["-c", PREFLIGHT]), stop, timeout=30)
            output("Execution de la suite de tests du projet...")
            wait_checked(start("Tests", ["-m", "unittest", "discover", "-s", "tests", "-t", ".", "-q"]), stop, timeout=180)
            if stop.is_set():
                raise KeyboardInterrupt
            dashboard = start("Dashboard", ["-c", DASHBOARD_WORKER])
            deadline = time.monotonic() + 20
            while not dashboard.ready.wait(0.05):
                dashboard.drain()
                if stop.is_set():
                    raise KeyboardInterrupt
                if dashboard.process.poll() is not None or time.monotonic() > deadline:
                    dashboard.reader.join(timeout=0.2)
                    dashboard.drain()
                    raise LaunchError("Le dashboard n'a pas confirme son demarrage.")
            if dashboard.protocol_error:
                raise LaunchError(dashboard.protocol_error)
            output("Dashboard : " + dashboard.url)
            output("Metriques : " + dashboard.url + "/api/metrics")
            bot = start("Bot", ["-c", BOT_WORKER])
            if open_browser:
                try:
                    webbrowser.open(dashboard.url)
                except Exception as error:
                    output("Ouverture du navigateur indisponible (" + type(error).__name__ + "). Utiliser l'adresse affichee.")
            output("Simulation papier lancee ; ntfy desactive. Ctrl+C pour arreter les deux processus.")
            output("Le demarrage du serveur HTTP ne valide pas encore la collecte /api/metrics.")
            while not stop.wait(0.1):
                for child in (dashboard, bot):
                    child.drain()
                    if child.process.poll() is not None:
                        child.reader.join(timeout=0.2)
                        child.drain()
                        raise LaunchError(f"{child.label} s'est arrete (code {child.process.returncode}).")
        except KeyboardInterrupt:
            output("Arret demande.")
        finally:
            stop_children(children)


def main():
    parser = argparse.ArgumentParser(description="Lanceur local Pravda-Bot en simulation papier.")
    parser.add_argument("--no-browser", action="store_true")
    parser.add_argument("--no-pause", action="store_true")
    args = parser.parse_args()
    result = 0
    try:
        launch(project_directory(), open_browser=not args.no_browser)
    except (LaunchError, OSError) as error:
        print("[ERREUR] " + str(error))
        result = 1
    if not args.no_pause and sys.stdin is not None and sys.stdin.isatty():
        try:
            input("\nAppuyez sur Entree pour fermer...")
        except (EOFError, KeyboardInterrupt):
            pass
    return result


if __name__ == "__main__":
    raise SystemExit(main())

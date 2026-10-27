"""Validate built artifacts and smoke-test an installed wheel outside the repo."""
from pathlib import Path
import subprocess
import sys
import tempfile
import zipfile


root = Path(__file__).resolve().parents[1]
wheels = list((root / "dist").glob("*.whl"))
sources = list((root / "dist").glob("*.tar.gz"))
if len(wheels) != 1 or len(sources) != 1:
    raise SystemExit("Expected exactly one wheel and one source distribution")
with zipfile.ZipFile(wheels[0]) as archive:
    names = archive.namelist()
    for name in ("main.py", "dashboard.py", "market_history.py", "pravda_resources/dashboard.html"):
        if name not in names:
            raise SystemExit("Missing wheel component: " + name)
    for name in names:
        if Path(name).name == ".env" or name.endswith((".sqlite3", ".csv", ".exe", ".log")):
            raise SystemExit("Private/runtime file in wheel: " + name)
    if archive.read("pravda_resources/dashboard.html") != (root / "dashboard.html").read_bytes():
        raise SystemExit("The packaged HTML differs from the source")

with tempfile.TemporaryDirectory(prefix="pravda-wheel-") as directory:
    target = Path(directory) / "installed"
    subprocess.run([
        sys.executable, "-m", "pip", "install", "--no-index", "--no-deps",
        "--target", str(target), str(wheels[0]),
    ], check=True, cwd=directory)
    for module, function in (("pravda_cli", "main"), ("dashboard", "main")):
        code = (
            "import sys; sys.path.insert(0, sys.argv[1]); "
            f"from {module} import {function}; sys.argv=['{module}', '--help']; {function}()"
        )
        subprocess.run([sys.executable, "-I", "-c", code, str(target)], cwd=directory, check=True)
    smoke = '''
import sys
sys.path.insert(0, sys.argv[1])
import dashboard
from http.server import ThreadingHTTPServer
import threading
from urllib.request import build_opener, ProxyHandler
server = ThreadingHTTPServer(('127.0.0.1', 0), dashboard.DashboardHandler)
thread = threading.Thread(target=server.serve_forever, daemon=True)
thread.start()
try:
    with build_opener(ProxyHandler({})).open(f'http://127.0.0.1:{server.server_port}/', timeout=5) as response:
        page = response.read().decode('utf-8')
        if '<script>' not in page or '</script>' not in page or not dashboard.INLINE_SCRIPT_HASHES:
            raise SystemExit('Packaged dashboard script/CSP missing')
finally:
    server.shutdown()
    thread.join(5)
    server.server_close()
'''
    subprocess.run([sys.executable, "-I", "-c", smoke, str(target)], cwd=directory, check=True)
print("Wheel resources, installed commands and loopback HTTP page verified outside the checkout.")

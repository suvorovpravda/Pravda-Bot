import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import textwrap
import threading
import time
import unittest
from urllib.request import ProxyHandler, build_opener
from unittest.mock import Mock, call, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import pravda_launcher as launcher


class LauncherTests(unittest.TestCase):
    def test_existing_topic_is_disabled_without_losing_model_setting(self):
        source = {
            'NTFY_TOPIC_URL': 'https://example.invalid/test-topic',
            'LM_MODEL_NAME': 'custom-local-model',
            'LM_STUDIO_API_BASE': 'https://example.invalid/remote',
        }
        env = launcher.runtime_environment(source)
        self.assertEqual(env['NTFY_TOPIC_URL'], '')
        self.assertEqual(env['LM_MODEL_NAME'], 'custom-local-model')
        self.assertEqual(env['LM_STUDIO_API_BASE'], 'http://127.0.0.1:1234/v1')
        self.assertEqual(source['NTFY_TOPIC_URL'], 'https://example.invalid/test-topic')

    def test_external_python_does_not_inherit_frozen_python_home(self):
        env = launcher.runtime_environment({'PYTHONHOME': 'bundle', 'PYTHONPATH': 'bundle'})
        self.assertNotIn('PYTHONHOME', env)
        self.assertNotIn('PYTHONPATH', env)

    def test_frozen_location_uses_executable_parent(self):
        fake = str(Path(tempfile.gettempdir()) / 'a space & directory' / 'Pravda_Lanceur.exe')
        with patch.object(sys, 'frozen', True, create=True), patch.object(sys, 'executable', fake):
            self.assertEqual(launcher.project_directory(), Path(fake).resolve().parent)

    def test_readiness_cannot_redirect_browser_to_another_host(self):
        self.assertEqual(launcher.dashboard_url('{"port": 8101, "url":"https://example.invalid"}'), 'http://127.0.0.1:8101')
        for bad in (0, -1, 65536, True, 8080.0, '8080', None):
            with self.subTest(port=bad), self.assertRaises(launcher.LaunchError):
                launcher.dashboard_url(json.dumps({'port': bad}))
        for bad in ('[]', 'null', '{}', '{bad'):
            with self.subTest(payload=bad), self.assertRaises((ValueError, launcher.LaunchError)):
                launcher.dashboard_url(bad)

    def test_missing_checkout_is_rejected_before_any_process_starts(self):
        with tempfile.TemporaryDirectory() as folder, patch.object(subprocess, 'Popen') as popen:
            with self.assertRaises(launcher.LaunchError):
                launcher.launch(Path(folder), Path(sys.executable))
            popen.assert_not_called()

    def test_duplicate_launcher_is_blocked_and_lock_released(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            with launcher.InstanceLock(root):
                with self.assertRaises(launcher.LaunchError):
                    with launcher.InstanceLock(root):
                        self.fail('second instance acquired the lock')
            with launcher.InstanceLock(root):
                pass

    def test_invalid_ready_message_is_reported(self):
        with tempfile.TemporaryDirectory() as folder, launcher.WindowsJob() as job:
            child = launcher.Child('Dashboard', [sys.executable, '-u', '-c', "print('PRAVDA_DASHBOARD_READY {\"port\": 0}')"], Path(folder), launcher.runtime_environment(), job, lambda _: None)
            try:
                self.assertTrue(child.ready.wait(5))
                self.assertIsNotNone(child.protocol_error)
                self.assertIsNone(child.url)
            finally:
                launcher.stop_children([child], grace=0.2)

    def test_ready_message_survives_log_queue_overflow(self):
        code = "for n in range(2000): print(n)\nprint('PRAVDA_DASHBOARD_READY {\"port\": 8102}')"
        with tempfile.TemporaryDirectory() as folder, launcher.WindowsJob() as job:
            child = launcher.Child('Dashboard', [sys.executable, '-u', '-c', code], Path(folder), launcher.runtime_environment(), job, lambda _: None)
            try:
                self.assertTrue(child.ready.wait(5))
                self.assertEqual(child.url, 'http://127.0.0.1:8102')
                self.assertLessEqual(child.lines.qsize(), 1000)
            finally:
                launcher.stop_children([child], grace=0.2)

    @unittest.skipUnless(os.name == 'nt', 'Windows Job Objects require Windows')
    def test_windows_job_closes_only_assigned_children(self):
        code = 'import time; time.sleep(60)'
        unrelated = subprocess.Popen([sys.executable, '-c', code], stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        owned = None
        try:
            with launcher.WindowsJob() as job:
                owned = subprocess.Popen([sys.executable, '-c', code], stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                job.assign(owned)
            owned.wait(timeout=5)
            self.assertIsNone(unrelated.poll())
        finally:
            for process in (owned, unrelated):
                if process is not None:
                    if process.poll() is None:
                        process.terminate()
                    process.wait(timeout=5)

    @unittest.skipUnless(os.name == 'nt', 'Windows frozen subprocess handling requires Windows')
    def test_frozen_child_temporarily_restores_windows_dll_search(self):
        with tempfile.TemporaryDirectory() as folder, launcher.WindowsJob() as job:
            kernel = Mock()
            kernel.SetDllDirectoryW.return_value = True
            with patch.object(sys, 'frozen', True, create=True), patch.object(sys, '_MEIPASS', folder, create=True), patch.object(launcher, '_windows_kernel', return_value=kernel):
                child = launcher.Child('Control', [sys.executable, '-c', 'pass'], Path(folder), launcher.runtime_environment(), job, lambda _: None)
            try:
                self.assertEqual(kernel.SetDllDirectoryW.call_args_list, [call(None), call(folder)])
            finally:
                launcher.stop_children([child], grace=0.2)


class LauncherIntegrationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='pravda space & project ')
        self.addCleanup(self.temp.cleanup)
        env_patch = patch.dict(os.environ)
        env_patch.start()
        self.addCleanup(env_patch.stop)
        os.environ.pop('LM_MODEL_NAME', None)
        self.root = Path(self.temp.name)
        tests = self.root / 'tests'
        tests.mkdir()
        (tests / '__init__.py').touch()
        (tests / 'test_smoke.py').write_text('import unittest\nclass Smoke(unittest.TestCase):\n    def test_ok(self): self.assertTrue(True)\n')
        (self.root / '.env').write_text('NTFY_TOPIC_URL=https://example.invalid/test-topic\nLM_MODEL_NAME=local-test-model\n')
        (self.root / 'bot_config.py').write_text(textwrap.dedent('''
            import os
            from types import SimpleNamespace
            def load_config():
                return SimpleNamespace(ntfy_topic_url=os.environ.get('NTFY_TOPIC_URL', ''))
        '''))
        (self.root / 'dashboard.py').write_text(textwrap.dedent('''
            import json
            from http.server import BaseHTTPRequestHandler
            class DashboardHandler(BaseHTTPRequestHandler):
                def __init__(self, *args, **kwargs):
                    for name in ('csv_path', 'lng_csv_path', 'threshold', 'stale_after_seconds'):
                        kwargs.pop(name, None)
                    super().__init__(*args, **kwargs)
                def do_GET(self):
                    data = json.dumps({'status':'success', 'system':{'cpu_percent':0}}).encode()
                    self.send_response(200)
                    self.send_header('Content-Length', str(len(data)))
                    self.end_headers()
                    self.wfile.write(data)
                def log_message(self, *_): pass
        '''))
        (self.root / 'main.py').write_text(textwrap.dedent('''
            import json, os, time
            from pathlib import Path
            from dotenv import load_dotenv
            load_dotenv()
            capture = {name:os.environ.get(name) for name in ('NTFY_TOPIC_URL', 'LM_MODEL_NAME', 'TRADING_MODE', 'PAPER_MODE')}
            capture['cwd'] = str(Path.cwd())
            Path('bot-started.json').write_text(json.dumps(capture))
            try:
                while True: time.sleep(0.02)
            except KeyboardInterrupt:
                Path('bot-stopped').touch()
        '''))
        self.stop = threading.Event()
        self.output = []
        self.errors = []
        self.thread = None

    def start(self):
        def run():
            try:
                launcher.launch(self.root, Path(sys.executable), stop=self.stop, open_browser=False, output=self.output.append)
            except BaseException as error:
                self.errors.append(error)
        self.thread = threading.Thread(target=run)
        self.thread.start()
        self.addCleanup(self.finish)

    def finish(self):
        self.stop.set()
        if self.thread:
            self.thread.join(timeout=22)
            self.assertFalse(self.thread.is_alive(), self.output)

    def wait_for(self, predicate, timeout=12):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if predicate():
                return
            if self.errors:
                self.fail(str(self.errors[0]) + '\n' + '\n'.join(self.output))
            time.sleep(0.05)
        self.fail('Startup timeout\n' + '\n'.join(self.output))

    def url(self):
        return next(line[len('Dashboard : '):] for line in self.output if line.startswith('Dashboard : '))

    def test_live_start_with_occupied_requested_port_and_graceful_stop(self):
        with socket.socket() as occupied:
            occupied.bind(('127.0.0.1', 0))
            occupied.listen()
            occupied_port = occupied.getsockname()[1]
            with patch.dict(os.environ, {'DASHBOARD_PORT':str(occupied_port)}):
                self.start()
                self.wait_for(lambda: (self.root / 'bot-started.json').exists())
                url = self.url()
                actual_port = int(url.rsplit(':', 1)[1])
                self.assertNotEqual(actual_port, occupied_port)
                with socket.socket() as contender:
                    with self.assertRaises(OSError):
                        contender.bind(('127.0.0.1', actual_port))
                opener = build_opener(ProxyHandler({}))
                with opener.open(url + '/api/metrics', timeout=3) as response:
                    self.assertEqual(response.status, 200)
                state = json.loads((self.root / 'bot-started.json').read_text())
                self.assertEqual(state['NTFY_TOPIC_URL'], '')
                self.assertEqual(state['LM_MODEL_NAME'], 'local-test-model')
                self.assertEqual(state['TRADING_MODE'], 'paper')
                self.assertEqual(state['PAPER_MODE'], 'portfolio')
                self.assertTrue(
                    Path(state['cwd']).samefile(self.root),
                    f"Le processus utilise un autre dossier : {state['cwd']!r} / {str(self.root)!r}",
                )
                self.finish()
                self.assertFalse(self.errors, self.errors)
                self.assertTrue((self.root / 'bot-stopped').exists())
                with socket.socket() as connection:
                    connection.settimeout(1)
                    self.assertNotEqual(connection.connect_ex(('127.0.0.1', actual_port)), 0)

    def test_failed_project_tests_prevent_bot_and_dashboard_start(self):
        (self.root / 'tests/test_smoke.py').write_text('import unittest\nclass Smoke(unittest.TestCase):\n    def test_fail(self): self.fail("expected control failure")\n')
        with self.assertRaisesRegex(launcher.LaunchError, 'Tests a echoue'):
            launcher.launch(self.root, Path(sys.executable), open_browser=False, output=self.output.append)
        self.assertFalse((self.root / 'bot-started.json').exists())
        self.assertFalse(any(line.startswith('Dashboard : ') for line in self.output))

    def test_invalid_configuration_prevents_tests_and_start(self):
        (self.root / 'bot_config.py').write_text('def load_config(): raise ValueError("invalid configuration control")\n')
        with self.assertRaisesRegex(launcher.LaunchError, 'Configuration a echoue'):
            launcher.launch(self.root, Path(sys.executable), open_browser=False, output=self.output.append)
        self.assertFalse(any('Execution de la suite' in line for line in self.output))
        self.assertFalse((self.root / 'bot-started.json').exists())

    def test_bot_exit_also_closes_owned_dashboard(self):
        (self.root / 'main.py').write_text('raise SystemExit(7)\n')
        with self.assertRaisesRegex(launcher.LaunchError, 'Bot.*code 7'):
            launcher.launch(self.root, Path(sys.executable), open_browser=False, output=self.output.append)
        port = int(self.url().rsplit(':', 1)[1])
        with socket.socket() as connection:
            connection.settimeout(1)
            self.assertNotEqual(connection.connect_ex(('127.0.0.1', port)), 0)


if __name__ == '__main__':
    unittest.main()

"""Build hook: keep one editable dashboard.html, include it inside wheels."""
from pathlib import Path

from setuptools import setup
from setuptools.command.build_py import build_py


class BuildWithDashboard(build_py):
    def run(self):
        super().run()
        destination = Path(self.build_lib) / "pravda_resources"
        destination.mkdir(parents=True, exist_ok=True)
        self.copy_file("dashboard.html", str(destination / "dashboard.html"))


setup(cmdclass={"build_py": BuildWithDashboard})

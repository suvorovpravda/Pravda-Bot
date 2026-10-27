#!/usr/bin/env python3
"""
Build a single-file executable for Pravda-Bot.
Usage:
    python build_onefile.py
Builds a PyInstaller onefile for the current platform.
"""
import argparse
import subprocess
import sys
from pathlib import Path

def run(cmd):
    print("+", " ".join(cmd))
    subprocess.check_call(cmd)

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--name", default="Pravda_Lanceur")
    parser.add_argument("--entry", default="pravda_launcher.py")
    args = parser.parse_args()

    root = Path(__file__).resolve().parent
    entry = root / args.entry
    if not entry.exists():
        parser.error("entry point does not exist: " + str(entry))

    # Ensure requirements-build are installed
    # PyInstaller must be available
    cmd = [
        sys.executable, "-m", "PyInstaller",
        "--onefile",
        "--console",
        "--noupx",
        "--name", args.name,
        "--clean",
        "--noconfirm",
        "--distpath", str(root),
        "--workpath", str(root / ".pravda-exe-build"),
        "--specpath", str(root / ".pravda-exe-spec"),
        str(entry)
    ]
    run(cmd)
    print(f"Built launcher in {root}; keep it beside main.py and .venv.")

if __name__ == "__main__":
    main()

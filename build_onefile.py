#!/usr/bin/env python3
"""
Build a single-file executable for Pravda-Bot.
Usage:
    python build_onefile.py --name PravdaBot
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
    parser.add_argument("--name", default="PravdaBot")
    parser.add_argument("--entry", default="pravda_launcher.py")
    args = parser.parse_args()

    root = Path(__file__).parent
    entry = root / args.entry
    if not entry.exists():
        # fallback to main.py
        entry = root / "main.py"

    # Ensure requirements-build are installed
    # PyInstaller must be available
    cmd = [
        sys.executable, "-m", "PyInstaller",
        "--onefile",
        "--name", args.name,
        "--clean",
        "--noconfirm",
        "--distpath", str(root / "dist"),
        "--workpath", str(root / ".build"),
        "--specpath", str(root / ".build"),
        str(entry)
    ]
    run(cmd)
    print(f"Built {root / 'dist' / args.name}")

if __name__ == "__main__":
    main()

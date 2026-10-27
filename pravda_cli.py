"""Installed command line entry point without starting the bot on --help."""
import argparse
import runpy


def main():
    parser = argparse.ArgumentParser(description="Pravda-Bot: public monitoring and paper simulation only.")
    parser.parse_args()
    runpy.run_module("main", run_name="__main__")


if __name__ == "__main__":
    main()

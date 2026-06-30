"""Python launcher for the bundled dead-code-finder binary."""

from __future__ import annotations

import os
import subprocess
import sys
from importlib.resources import files


def _binary_name() -> str:
    if os.name == "nt":
        return "dead-code-finder.exe"
    return "dead-code-finder"


def binary_path() -> str:
    return str(files(__package__).joinpath("bin", _binary_name()))


def main() -> None:
    binary = binary_path()
    if not os.path.exists(binary):
        raise SystemExit(f"bundled binary not found: {binary}")

    completed = subprocess.run([binary, *sys.argv[1:]], check=False)
    raise SystemExit(completed.returncode)

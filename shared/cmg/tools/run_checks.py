#!/usr/bin/env python3
"""Run deterministic local gates for the standalone CMG core."""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
CMG = ROOT / "shared" / "cmg"
MAC_STATA = Path("/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp")


def _run(label: str, command: list[str], *, marker: str | None = None) -> None:
    print(f"\n== {label} ==", flush=True)
    print("+", " ".join(command), flush=True)
    completed = subprocess.run(
        command,
        cwd=ROOT,
        env=os.environ.copy(),
        stdin=subprocess.DEVNULL,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    print(completed.stdout, end="", flush=True)
    if completed.returncode != 0 or (marker is not None and marker not in completed.stdout):
        raise subprocess.CalledProcessError(completed.returncode or 1, command)


def _stata() -> str | None:
    configured = os.environ.get("CMG_STATA")
    if configured:
        return configured
    discovered = shutil.which("stata-mp")
    if discovered:
        return discovered
    return str(MAC_STATA) if MAC_STATA.is_file() else None


def main() -> int:
    _run(
        "deterministic generated-artifact check",
        [sys.executable, str(CMG / "tools" / "assemble.py"), "--all", "--check"],
    )
    _run(
        "independent Python and cross-language tests",
        [sys.executable, "-m", "pytest", str(CMG / "tests"), "-q"],
    )
    stata = _stata()
    if stata is None:
        if os.environ.get("CMG_REQUIRE_STATA") == "1":
            raise RuntimeError("CMG_REQUIRE_STATA=1 but Stata/MP is unavailable")
        print("\n== Mata gates ==\nSKIP: Stata/MP is unavailable.", flush=True)
        return 0
    _run(
        "Mata mathematical and runtime core",
        [
            stata,
            "-q",
            "do",
            str(CMG / "tests" / "stata" / "test_core.do"),
            str(ROOT),
        ],
        marker="CMG MATA CORE TEST PASS",
    )
    _run(
        "package namespace compile gate",
        [
            stata,
            "-q",
            "do",
            str(CMG / "tests" / "stata" / "test_namespaces.do"),
            str(ROOT),
        ],
        marker="CMG NAMESPACE COMPILE TEST PASS",
    )
    _run(
        "large multilevel hierarchy scale gate",
        [
            stata,
            "-q",
            "do",
            str(CMG / "tests" / "stata" / "test_hierarchy_scale.do"),
            str(ROOT),
        ],
        marker="CMG HIERARCHY SCALE TEST PASS",
    )
    print("\nCMG LOCAL CORE GATES PASS", flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())

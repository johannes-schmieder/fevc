#!/usr/bin/env python3
"""Run the local qualification gates for the sibling ``kss_bc`` package."""
from __future__ import annotations

import csv
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
PYTHON = ROOT / ".venv/bin/python"


def run(label: str, command: list[str], cwd: Path = ROOT) -> None:
    print(f"\n== {label} ==", flush=True)
    print("+", " ".join(command), flush=True)
    subprocess.run(command, cwd=cwd, check=True)


def stata_executable() -> str | None:
    configured = os.environ.get("KSS_BC_STATA")
    if configured:
        return configured
    discovered = shutil.which("stata-mp")
    if discovered:
        return discovered
    macos = Path("/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp")
    if macos.is_file():
        return str(macos)
    return None


def run_stata(
    label: str,
    stata: str,
    do_file: Path,
    arguments: list[str],
    pass_marker: str,
    cwd: Path = ROOT,
) -> None:
    """Require Stata's application marker as well as its process status."""

    command = [stata, "-q", "do", str(do_file), *arguments]
    print(f"\n== {label} ==", flush=True)
    print("+", " ".join(command), flush=True)
    completed = subprocess.run(
        command,
        cwd=cwd,
        env=os.environ.copy(),
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    print(completed.stdout, end="", flush=True)
    if completed.returncode != 0 or pass_marker not in completed.stdout:
        raise subprocess.CalledProcessError(completed.returncode or 1, command)


def validate_benchmark(output: Path) -> None:
    marker = output / "smoke.stata.pass"
    result = output / "smoke.csv"
    if not marker.is_file() or not result.is_file():
        raise RuntimeError("KSS benchmark smoke did not create its evidence files.")
    with result.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    if len(rows) != 1 or rows[0].get("status") != "KSS_POINT_ESTIMATES_ONLY":
        raise RuntimeError("KSS benchmark smoke returned an invalid structured status.")
    for field in (
        "graph_seconds",
        "fit_seconds",
        "preconditioner_seconds",
        "leverage_seconds",
        "target_seconds",
        "correction_seconds",
    ):
        value = float(rows[0][field])
        if not 0 <= value < 10_000:
            raise RuntimeError(f"KSS benchmark smoke returned invalid {field}.")
    if not 0 < float(rows[0]["deletion_rank_gap"]) <= 1:
        raise RuntimeError("KSS benchmark smoke returned an invalid rank certificate.")


def validate_separations_preparation(output: Path) -> None:
    prepared = output / "prepared.csv"
    metadata = output / "prepare.csv"
    if not prepared.is_file() or not metadata.is_file():
        raise RuntimeError("Separations preparation smoke omitted its outputs.")
    with prepared.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    if len(rows) != 8 or set(rows[0]) != {
        "worker", "firm", "period", "y_minus_xb"
    }:
        raise RuntimeError("Separations preparation changed its four-column contract.")
    keys = [(row["worker"], row["firm"], row["period"]) for row in rows]
    if len(set(keys)) == len(keys):
        raise RuntimeError("Separations preparation dropped valid aggregate duplicates.")
    with metadata.open(newline="", encoding="utf-8") as handle:
        metadata_rows = list(csv.DictReader(handle))
    if (
        len(metadata_rows) != 1
        or int(float(metadata_rows[0]["stored_rows"])) != 8
        or int(float(metadata_rows[0]["aggregate_duplicate_rows"])) != 8
    ):
        raise RuntimeError("Separations preparation metadata is inconsistent.")


def main() -> int:
    if not PYTHON.is_file():
        raise RuntimeError(f"repository interpreter is missing: {PYTHON}")
    run(
        "KSS independent Python oracles",
        [str(PYTHON), "-m", "pytest", "kss_bc/tests/python", "-q"],
    )

    stata = stata_executable()
    if stata is None:
        if os.environ.get("KSS_BC_REQUIRE_STATA") == "1":
            raise RuntimeError(
                "KSS_BC_REQUIRE_STATA=1 but no Stata/MP executable was found."
            )
        print(
            "\n== KSS Stata gates ==\n"
            "SKIP: no Stata/MP launcher found; set KSS_BC_STATA or "
            "KSS_BC_REQUIRE_STATA=1.",
            flush=True,
        )
        return 0

    suite = ROOT / "kss_bc/tests/stata/run_all.do"
    run_stata(
        "KSS Stata quick suite",
        stata,
        suite,
        ["quick"],
        "KSS_BC TEST SUITE PASS: quick",
    )
    run_stata(
        "KSS Stata full suite",
        stata,
        suite,
        ["full"],
        "KSS_BC TEST SUITE PASS: full",
    )
    with tempfile.TemporaryDirectory(prefix="kss-bc-install-") as temporary:
        install_root = Path(temporary) / "plus"
        install_root.mkdir()
        run_stata(
            "KSS clean-install smoke test",
            stata,
            ROOT / "kss_bc/tests/stata/test_install.do",
            [str(ROOT / "kss_bc"), str(install_root)],
            "KSS_BC INSTALL TEST PASS",
            Path(temporary),
        )
    with tempfile.TemporaryDirectory(prefix="kss-bc-benchmark-") as temporary:
        output = Path(temporary)
        run_stata(
            "KSS synthetic benchmark harness smoke test",
            stata,
            ROOT / "kss_bc/benchmarks/synthetic_benchmark.do",
            [
                "local-gate",
                "smoke",
                "100",
                "10",
                "10",
                "20260814",
                str(output),
                "0" * 40,
            ],
            "KSS_BC BENCHMARK PASS: smoke",
        )
        validate_benchmark(output)
    with tempfile.TemporaryDirectory(prefix="kss-bc-numopt-") as temporary:
        output = Path(temporary)
        commit = "0" * 40
        for route in ("b1", "cmg"):
            route_output = output / "numopt" / "moderate" / route
            route_output.mkdir(parents=True)
            run_stata(
                f"KSS end-to-end {route.upper()} benchmark smoke test",
                stata,
                ROOT / "kss_bc/benchmarks/estimator_cmg_benchmark.do",
                [
                    "local-gate",
                    route,
                    "moderate",
                    "1200",
                    "300",
                    "40",
                    "8675309",
                    "4",
                    "60",
                    "local_e2e",
                    str(route_output),
                    commit,
                ],
                f"KSS_BC NUMOPT BENCHMARK PASS: {route} moderate",
            )
        run(
            "KSS paired B1/forced-CMG benchmark validation",
            [
                str(PYTHON),
                "kss_bc/benchmarks/validate_numopt.py",
                "--run-dir",
                str(output),
                "--expected-commit",
                commit,
                "--scenarios",
                "moderate",
            ],
        )
    with tempfile.TemporaryDirectory(prefix="kss-bc-separations-") as temporary:
        output = Path(temporary)
        run_stata(
            "KSS Separations preparation duplicate-key smoke test",
            stata,
            ROOT / "kss_bc/tests/stata/test_separations_prepare.do",
            [str(ROOT / "kss_bc"), str(output)],
            "KSS_BC SEPARATIONS PREPARE PASS: fixture",
        )
        validate_separations_preparation(output)
    print("\nKSS_BC LOCAL QUALIFICATION PASS", flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())

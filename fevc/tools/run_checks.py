#!/usr/bin/env python3
"""Run the integrated local qualification gates for ``fevc``."""
from __future__ import annotations

import csv
import math
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PYTHON = ROOT / ".venv/bin/python"


def run(
    label: str,
    command: list[str],
    cwd: Path = ROOT,
    env: dict[str, str] | None = None,
) -> None:
    print(f"\n== {label} ==", flush=True)
    print("+", " ".join(command), flush=True)
    subprocess.run(command, cwd=cwd, env=env, check=True)


def stata_executable() -> str | None:
    configured = os.environ.get("FEVC_STATA")
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
        stdin=subprocess.DEVNULL,
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
    row = rows[0]

    def finite(
        field: str, *, positive: bool = False, nonnegative: bool = False
    ) -> float:
        try:
            value = float(row[field])
        except (KeyError, TypeError, ValueError) as error:
            raise RuntimeError(
                f"KSS benchmark smoke omitted required numeric field {field}."
            ) from error
        if (
            not math.isfinite(value)
            or (nonnegative and value < 0)
            or (positive and value <= 0)
        ):
            raise RuntimeError(f"KSS benchmark smoke returned invalid {field}.")
        return value

    def optional_finite(field: str) -> float | None:
        raw = row.get(field, "")
        if raw == "":
            return None
        try:
            value = float(raw)
        except ValueError as error:
            raise RuntimeError(
                f"KSS benchmark smoke returned nonnumeric {field}."
            ) from error
        if not math.isfinite(value) or value < 0:
            raise RuntimeError(f"KSS benchmark smoke returned invalid {field}.")
        return value

    command_seconds = finite("command_seconds", positive=True)
    total_seconds = finite("total_seconds", positive=True)
    if total_seconds + 1e-12 < command_seconds:
        raise RuntimeError("KSS benchmark smoke has inconsistent outer timings.")
    for field in (
        "N_stored",
        "N_physical",
        "N_retained",
        "worker_levels",
        "firm_levels",
        "deletion_units",
        "requested_probes",
        "probes",
    ):
        finite(field, positive=True)
    for prefix in ("plugin", "correction", "corrected", "mcse"):
        for target in ("worker", "firm", "covariance", "total"):
            value = finite(
                f"{prefix}_{target}", nonnegative=(prefix == "mcse")
            )
            if prefix == "mcse" and value < 0:
                raise RuntimeError("KSS benchmark smoke returned a negative MCSE.")

    backend = row.get("backend_selected")
    if backend not in {"mata", "rust"}:
        raise RuntimeError("KSS benchmark smoke omitted its selected backend.")
    if backend == "mata":
        for field in (
            "graph_seconds",
            "fit_seconds",
            "preconditioner_seconds",
            "leverage_seconds",
            "target_seconds",
            "correction_seconds",
        ):
            value = finite(field, nonnegative=True)
            if value >= 10_000:
                raise RuntimeError(f"KSS benchmark smoke returned invalid {field}.")
        rank_gap = finite("deletion_rank_gap", positive=True)
    else:
        # Native phase timing is not yet part of the public ABI. Keep those
        # columns empty rather than fabricating Mata-shaped measurements, and
        # require the native scientific/resource receipts instead.
        for field in (
            "graph_seconds",
            "fit_seconds",
            "preconditioner_seconds",
            "leverage_seconds",
            "target_seconds",
            "correction_seconds",
        ):
            optional_finite(field)
        rank_gap = finite("rust_deletion_rank_gap", positive=True)
        finite("complete_residual_max", nonnegative=True)
        finite("target_identity_residual", nonnegative=True)
        finite("rust_maker_relres", nonnegative=True)
        finite("rust_actual_accounting_residual", nonnegative=True)
        finite("rust_plan_solve_peak_bytes", positive=True)
        finite("rust_plan_nonbatched_peak_bytes", positive=True)
        if finite("rust_counter_plan_complete", nonnegative=True) != 1:
            raise RuntimeError("KSS benchmark smoke has an incomplete counter plan.")
        if (
            finite("rust_pre_rng_hi", nonnegative=True) != 0
            or finite("rust_pre_rng_lo", nonnegative=True) != 0
        ):
            raise RuntimeError("KSS benchmark smoke consumed pre-estimator RNG.")
    if not 0 < rank_gap <= 1:
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
        or metadata_rows[0]["sample_selection"] != "full_natural_graph"
    ):
        raise RuntimeError("Separations preparation metadata is inconsistent.")
    route_rows: dict[str, dict[str, str]] = {}
    for route in ("b1", "cmg"):
        path = output / route / f"separations_fixture_{route}.csv"
        with path.open(newline="", encoding="utf-8") as handle:
            result_rows = list(csv.DictReader(handle))
        if len(result_rows) != 1 or result_rows[0]["converged"] != "1":
            raise RuntimeError(f"Separations {route} duplicate-key fixture failed.")
        route_rows[route] = result_rows[0]
    fields = [
        f"{prefix}_{target}"
        for prefix in ("plugin", "correction", "corrected", "mcse")
        for target in ("worker", "firm", "covariance", "total")
    ]
    left = [float(route_rows["b1"][field]) for field in fields]
    right = [float(route_rows["cmg"][field]) for field in fields]
    if len(left) != len(right):
        raise RuntimeError("Separations route result shapes disagree.")
    scale = max((abs(value) for value in left + right), default=0.0)
    difference = max(
        (abs(a - b) for a, b in zip(left, right)), default=0.0
    )
    if difference / max(scale, 1e-300) > 2e-9:
        raise RuntimeError("Separations fixture changed the estimator matrix by route.")


def main() -> int:
    if not PYTHON.is_file():
        raise RuntimeError(f"repository interpreter is missing: {PYTHON}")
    run(
        "historical-name and immutable-evidence audit",
        [str(PYTHON), "fevc/tools/check_legacy_names.py"],
    )
    run(
        "FEVC public-identity audit",
        [str(PYTHON), "fevc/tools/check_fevc_public_identity.py"],
    )
    run(
        "FEVC preserved-history audit",
        [str(PYTHON), "fevc/tools/check_fevc_history.py"],
    )
    run(
        "FEVC license and provenance audit",
        [str(PYTHON), "fevc/tools/license_audit.py"],
    )
    run(
        "generated Rust/Mata parity check",
        [str(PYTHON), "fevc/tools/render_rust_mata_parity.py", "--check"],
    )
    run(
        "deterministic portable release-artifact check",
        [str(PYTHON), "fevc/tools/build_release_artifact.py", "--check"],
    )
    run(
        "deterministic CMG generated-artifact check",
        [
            str(PYTHON),
            "fevc/cmg/tools/assemble.py",
            "--all",
            "--check",
        ],
    )
    run("package, CMG, and MATLAB-harness Python tests", [str(PYTHON), "-m", "pytest", "-q"])

    stata = stata_executable()
    if stata is None:
        if os.environ.get("FEVC_REQUIRE_STATA") == "1":
            raise RuntimeError(
                "FEVC_REQUIRE_STATA=1 but no Stata/MP executable was found."
            )
        print(
            "\n== KSS Stata gates ==\n"
            "SKIP: no Stata/MP launcher found; set FEVC_STATA or "
            "FEVC_REQUIRE_STATA=1.",
            flush=True,
        )
        return 0

    component_environment = os.environ.copy()
    component_environment["CMG_STATA"] = stata
    component_environment["CMG_REQUIRE_STATA"] = "1"
    run(
        "CMG component qualification",
        [str(PYTHON), "fevc/cmg/tools/run_checks.py"],
        env=component_environment,
    )

    suite = ROOT / "fevc/tests/stata/run_all.do"
    run_stata(
        "KSS Stata quick suite",
        stata,
        suite,
        ["quick"],
        "FEVC TEST SUITE PASS: quick",
    )
    run_stata(
        "KSS Stata full suite",
        stata,
        suite,
        ["full"],
        "FEVC TEST SUITE PASS: full",
    )
    with tempfile.TemporaryDirectory(prefix="fevc-install-") as temporary:
        install_root = Path(temporary) / "plus"
        install_root.mkdir()
        run_stata(
            "KSS clean-install smoke test",
            stata,
            ROOT / "fevc/tests/stata/test_install.do",
            [str(ROOT / "fevc"), str(install_root)],
            "FEVC INSTALL TEST PASS",
            Path(temporary),
        )
    with tempfile.TemporaryDirectory(prefix="fevc-benchmark-") as temporary:
        output = Path(temporary)
        run_stata(
            "KSS synthetic benchmark harness smoke test",
            stata,
            ROOT / "fevc/benchmarks/synthetic_benchmark.do",
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
            "FEVC BENCHMARK PASS: smoke",
        )
        validate_benchmark(output)
    with tempfile.TemporaryDirectory(prefix="fevc-numopt-") as temporary:
        output = Path(temporary)
        commit = "0" * 40
        for route in ("b1", "cmg"):
            route_output = output / "numopt" / "moderate" / route
            route_output.mkdir(parents=True)
            run_stata(
                f"KSS end-to-end {route.upper()} benchmark smoke test",
                stata,
                ROOT / "fevc/benchmarks/estimator_cmg_benchmark.do",
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
                f"FEVC NUMOPT BENCHMARK PASS: {route} moderate",
            )
        run(
            "KSS paired B1/forced-CMG benchmark validation",
            [
                str(PYTHON),
                "fevc/benchmarks/validate_numopt.py",
                "--run-dir",
                str(output),
                "--expected-commit",
                commit,
                "--scenarios",
                "moderate",
            ],
        )
    with tempfile.TemporaryDirectory(prefix="fevc-separations-") as temporary:
        output = Path(temporary)
        run_stata(
            "KSS Separations preparation duplicate-key smoke test",
            stata,
            ROOT / "fevc/tests/stata/test_separations_prepare.do",
            [str(ROOT / "fevc"), str(output)],
            "FEVC SEPARATIONS PREPARE PASS: fixture",
        )
        validate_separations_preparation(output)
    with tempfile.TemporaryDirectory(
        prefix="fevc-matlab-sample-"
    ) as temporary:
        run_stata(
            "KSS MATLAB-retained sample and bridge-audit smoke test",
            stata,
            ROOT / "fevc/tests/stata/test_separations_matlab_sample.do",
            [str(ROOT / "fevc"), temporary],
            "PASS test_separations_matlab_sample.do",
        )
    print("\nFEVC LOCAL QUALIFICATION PASS", flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())

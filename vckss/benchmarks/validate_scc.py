#!/usr/bin/env python3
"""Validate collected SCC scheduler, application, and structured evidence."""
from __future__ import annotations

import argparse
import csv
import math
import re
from pathlib import Path

VALID_JOBS = {"portability", "oracle", "smoke", "medium", "large"}
TARGETS = (
    "worker_variance",
    "firm_variance",
    "worker_firm_covariance",
    "total_variance",
)
TARGET_FIELDS = ("worker", "firm", "covariance", "total")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def text(path: Path) -> str:
    require(path.is_file(), f"missing evidence file: {path}")
    return path.read_text(encoding="utf-8", errors="replace")


def validate_qacct(root: Path, job: str) -> None:
    receipt = text(root / "qacct" / f"{job}.txt")
    failed = re.search(r"(?m)^failed\s+(\d+)", receipt)
    exit_status = re.search(r"(?m)^exit_status\s+(\d+)", receipt)
    require(failed is not None, f"qacct {job}: missing failed field")
    require(exit_status is not None, f"qacct {job}: missing exit_status field")
    require(failed.group(1) == "0", f"qacct {job}: scheduler failure")
    require(exit_status.group(1) == "0", f"qacct {job}: nonzero exit status")


def validate_portability(root: Path, commit: str) -> None:
    validate_qacct(root, "portability")
    wrapper_log = text(root / "logs" / "portability.stdout.txt")
    require("VCKSS SCC PORTABILITY PASS" in wrapper_log, "portability wrapper marker absent")
    full_log = text(root / "portability" / "full_suite.application.txt")
    install_log = text(root / "portability" / "install.application.txt")
    require("VCKSS TEST SUITE PASS: full" in full_log, "full Stata suite marker absent")
    require("VCKSS INSTALL TEST PASS" in install_log, "install marker absent")
    marker = text(root / "portability" / "portability.pass")
    require(marker.strip() == f"VCKSS_PORTABILITY_PASS {commit}", "bad portability marker")
    for resource in ("full_suite.resources.txt", "install.resources.txt"):
        resource_path = root / "portability" / resource
        require("stata-mp" in text(resource_path), f"portability did not invoke stata-mp in {resource}")
        validate_resource(resource_path)


def read_rows(path: Path) -> list[dict[str, str]]:
    require(path.is_file(), f"missing CSV: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        return list(csv.DictReader(handle))


def finite(row: dict[str, str], name: str) -> float:
    try:
        value = float(row[name])
    except (KeyError, ValueError) as exc:
        raise ValueError(f"invalid numeric field {name}") from exc
    require(math.isfinite(value), f"nonfinite numeric field {name}")
    return value


def validate_oracle(root: Path, commit: str) -> None:
    validate_qacct(root, "oracle")
    wrapper_log = text(root / "logs" / "oracle.stdout.txt")
    matlab_log = text(root / "oracle" / "matlab.application.txt")
    stata_log = text(root / "oracle" / "stata.application.txt")
    require("VCKSS SCC ORACLE PASS" in wrapper_log, "oracle wrapper marker absent")
    require("VCKSS MATLAB ORACLE PASS" in matlab_log, "MATLAB oracle marker absent")
    require("VCKSS STATA/MATLAB ORACLE PASS" in stata_log, "paired oracle marker absent")
    require(
        text(root / "oracle" / "oracle.wrapper.pass").strip()
        == f"VCKSS_ORACLE_PASS {commit}",
        "bad oracle wrapper marker",
    )
    matlab = read_rows(root / "oracle" / "matlab_oracle.csv")
    stata = read_rows(root / "oracle" / "stata_oracle.csv")
    require(len(matlab) == len(stata) == 4, "oracle CSVs must each have four rows")
    for left, right, target in zip(matlab, stata, TARGETS, strict=True):
        require(left["source_commit"] == right["source_commit"] == commit, "oracle commit mismatch")
        require(left["target"] == right["target"] == target, "oracle target mismatch")
        for field in ("plugin", "correction", "corrected"):
            gap = abs(finite(left, field)-finite(right, field))
            require(gap <= 2e-9, f"oracle {target} {field} gap {gap:g}")
    validate_resource(root / "oracle" / "matlab.resources.txt")
    validate_resource(root / "oracle" / "stata.resources.txt")


def validate_resource(path: Path) -> None:
    report = text(path)
    maximum = re.search(r"Maximum resident set size \(kbytes\):\s*(\d+)", report)
    require(maximum is not None, f"peak RSS absent from {path}")
    require(int(maximum.group(1)) > 0, f"nonpositive peak RSS in {path}")


def validate_scale(root: Path, commit: str, scenario: str) -> None:
    validate_qacct(root, scenario)
    job_dir = root / "scale" / scenario
    wrapper_log = text(root / "logs" / f"{scenario}.stdout.txt")
    app_log = text(job_dir / f"{scenario}.application.txt")
    require(f"VCKSS SCC SCALE PASS: {scenario}" in wrapper_log, f"{scenario} wrapper marker absent")
    require(f"VCKSS BENCHMARK PASS: {scenario}" in app_log, f"{scenario} application marker absent")
    require(
        text(job_dir / f"{scenario}.wrapper.pass").strip()
        == f"VCKSS_SCALE_PASS {scenario} {commit}",
        f"bad {scenario} wrapper marker",
    )
    require(commit in text(job_dir / f"{scenario}.stata.pass"), f"bad {scenario} Stata marker")
    rows = read_rows(job_dir / f"{scenario}.csv")
    require(len(rows) == 1, f"{scenario} CSV must contain one row")
    row = rows[0]
    require(row["source_commit"] == commit, f"{scenario} commit mismatch")
    require(row["scenario"] == scenario, f"{scenario} label mismatch")
    require(row["status"] == "KSS_POINT_ESTIMATES_ONLY", f"{scenario} bad status")
    require(row["algorithm"] == "jla", f"{scenario} did not use JLA")
    require(row["stata_version"].startswith("19"), f"{scenario} did not use Stata 19")
    require(row["stata_flavor"] in {"IC", "MP"}, f"{scenario} has unexpected Stata flavor")
    workers = finite(row, "requested_workers")
    firms = finite(row, "requested_firms")
    probes = finite(row, "requested_probes")
    require(finite(row, "N_stored") == 4*workers, f"{scenario} stored-row mismatch")
    require(finite(row, "N_retained") == 4*workers, f"{scenario} graph dropped rows")
    require(finite(row, "worker_levels") == workers, f"{scenario} worker count mismatch")
    require(finite(row, "firm_levels") == firms, f"{scenario} firm count mismatch")
    expected_parameters = workers + firms + 1  # all workers, F-1 firms, two controls
    require(
        finite(row, "full_parameters") == expected_parameters,
        f"{scenario} full parameter count mismatch",
    )
    require(
        finite(row, "correction_parameters") == expected_parameters,
        f"{scenario} correction parameter count mismatch",
    )
    require(
        finite(row, "parameters") == finite(row, "correction_parameters"),
        f"{scenario} compatibility parameter count mismatch",
    )
    require(finite(row, "deletion_units") == 2*workers, f"{scenario} match count mismatch")
    require(finite(row, "probes") == probes, f"{scenario} realized probe mismatch")
    require(0 <= finite(row, "max_leverage") < 1, f"{scenario} invalid leverage")
    require(finite(row, "solver_max_residual") <= 1e-8, f"{scenario} residual gate failed")
    require(finite(row, "inverse_relres") <= 1e-8, f"{scenario} inverse residual gate failed")
    require(0 < finite(row, "preconditioner_ratio") <= 1, f"{scenario} invalid preconditioner ratio")
    require(
        0 < finite(row, "control_schur_rcond") <= 1,
        f"{scenario} invalid control Schur conditioning",
    )
    require(
        0 < finite(row, "deletion_rank_gap") <= 1,
        f"{scenario} invalid deletion-rank certificate",
    )
    for field in (
        "data_prep_seconds", "command_seconds", "total_seconds",
        "graph_seconds", "fit_seconds", "setup_seconds",
        "preconditioner_seconds", "schur_seconds",
        "preconditioner_apply_seconds", "pcg_seconds",
        "solver_backend_seconds", "leverage_seconds", "target_seconds",
        "correction_seconds",
    ):
        require(finite(row, field) >= 0, f"{scenario} negative timing {field}")
    for field in (
        "solver_iterations", "solver_schur_actions", "solver_schur_batches",
        "solver_precond_applications", "solver_precond_batches",
    ):
        value = finite(row, field)
        require(value >= 0 and value == math.floor(value), f"{scenario} invalid count {field}")
    require(
        finite(row, "solver_schur_actions") >= finite(row, "solver_schur_batches") > 0,
        f"{scenario} invalid Schur action accounting",
    )
    require(
        finite(row, "solver_precond_applications")
        >= finite(row, "solver_precond_batches") > 0,
        f"{scenario} invalid preconditioner accounting",
    )
    for prefix in ("plugin", "correction", "corrected", "mcse"):
        for target in TARGET_FIELDS:
            finite(row, f"{prefix}_{target}")
    for prefix in ("plugin", "correction", "corrected"):
        total = finite(row, f"{prefix}_total")
        identity = (
            finite(row, f"{prefix}_worker")
            + finite(row, f"{prefix}_firm")
            + 2*finite(row, f"{prefix}_covariance")
        )
        require(abs(total-identity) <= 1e-8*(1+abs(identity)), f"{scenario} accounting failure")

    rhs_rows = read_rows(job_dir / f"{scenario}.rhs.csv")
    require(len(rhs_rows) >= 3*int(probes), f"{scenario} incomplete RHS diagnostics")
    stage_counts = {stage: 0 for stage in range(1, 6)}
    for diagnostic in rhs_rows:
        require(diagnostic["source_commit"] == commit, f"{scenario} RHS commit mismatch")
        require(diagnostic["scenario"] == scenario, f"{scenario} RHS label mismatch")
        require(
            diagnostic["stata_version"].startswith("19")
            and diagnostic["stata_flavor"] in {"IC", "MP"},
            f"{scenario} RHS has unexpected Stata 19 metadata",
        )
        stage = finite(diagnostic, "stage")
        batch_start = finite(diagnostic, "batch_start")
        rhs = finite(diagnostic, "rhs")
        iterations = finite(diagnostic, "iterations")
        residual = finite(diagnostic, "relative_residual")
        converged = finite(diagnostic, "converged")
        require(stage in stage_counts, f"{scenario} invalid RHS stage")
        require(batch_start == math.floor(batch_start), f"{scenario} invalid RHS batch")
        if int(stage) in {4, 5}:
            require(batch_start >= 1, f"{scenario} invalid probe RHS batch")
        else:
            require(batch_start == 0, f"{scenario} invalid nonprobe RHS batch")
        require(rhs >= 1 and rhs == math.floor(rhs), f"{scenario} invalid RHS index")
        require(iterations >= 0 and iterations == math.floor(iterations), f"{scenario} invalid RHS iterations")
        require(residual <= 1e-7, f"{scenario} RHS complete residual failed")
        require(converged == 1, f"{scenario} unconverged RHS was accepted")
        stage_counts[int(stage)] += 1
    require(stage_counts[4] == int(probes), f"{scenario} leverage RHS count mismatch")
    require(stage_counts[5] == 2*int(probes), f"{scenario} target RHS count mismatch")
    resource_path = job_dir / "resources.txt"
    require("stata-mp" in text(resource_path), f"{scenario} did not invoke stata-mp")
    validate_resource(resource_path)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--run-dir", required=True, type=Path)
    parser.add_argument("--expected-commit", required=True)
    parser.add_argument("--jobs", nargs="+", required=True)
    args = parser.parse_args()
    require(re.fullmatch(r"[0-9a-f]{40}", args.expected_commit) is not None, "expected commit must be full SHA")
    require(set(args.jobs) <= VALID_JOBS, "unknown job label")
    recorded = text(args.run_dir / "source_commit.txt").strip()
    require(recorded == args.expected_commit, "run source_commit.txt mismatch")
    for job in args.jobs:
        if job == "portability":
            validate_portability(args.run_dir, args.expected_commit)
        elif job == "oracle":
            validate_oracle(args.run_dir, args.expected_commit)
        else:
            validate_scale(args.run_dir, args.expected_commit, job)
        print(f"PASS {job}")
    print("VCKSS SCC EVIDENCE PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

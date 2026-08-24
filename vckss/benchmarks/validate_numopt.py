#!/usr/bin/env python3
"""Validate paired end-to-end B1/forced-CMG benchmark evidence."""
from __future__ import annotations

import argparse
import csv
import math
import re
from pathlib import Path

TARGETS = ("worker", "firm", "covariance", "total")
TIMINGS = (
    "data_prep_seconds",
    "command_seconds",
    "total_seconds",
    "graph_seconds",
    "fit_seconds",
    "setup_seconds",
    "schur_seconds",
    "preconditioner_apply_seconds",
    "pcg_seconds",
    "leverage_seconds",
    "target_seconds",
    "correction_seconds",
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def read_text(path: Path) -> str:
    require(path.is_file(), f"missing evidence file: {path}")
    return path.read_text(encoding="utf-8", errors="replace")


def read_one(path: Path) -> dict[str, str]:
    require(path.is_file(), f"missing result CSV: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    require(len(rows) == 1, f"expected one result row in {path}")
    return rows[0]


def finite(row: dict[str, str], field: str) -> float:
    try:
        value = float(row[field])
    except (KeyError, ValueError) as exc:
        raise ValueError(f"invalid numeric field {field}") from exc
    require(math.isfinite(value), f"nonfinite numeric field {field}")
    return value


def parse_qacct(path: Path) -> None:
    receipt = read_text(path)
    failed = re.search(r"(?m)^failed\s+(\d+)", receipt)
    exit_status = re.search(r"(?m)^exit_status\s+(\d+)", receipt)
    require(failed is not None and failed.group(1) == "0", f"bad qacct failed field: {path}")
    require(
        exit_status is not None and exit_status.group(1) == "0",
        f"bad qacct exit_status field: {path}",
    )


def peak_rss(path: Path) -> int:
    report = read_text(path)
    match = re.search(r"Maximum resident set size \(kbytes\):\s*(\d+)", report)
    require(match is not None, f"peak RSS missing from {path}")
    value = int(match.group(1))
    require(0 < value < 60 * 1024 * 1024, f"peak RSS outside 60 GiB gate: {path}")
    return value


def matrix_relative_difference(
    left: list[float], right: list[float]
) -> float:
    scale = max((abs(value) for value in left + right), default=0.0)
    difference = max((abs(a - b) for a, b in zip(left, right, strict=True)), default=0.0)
    return difference / max(scale, 1e-300)


def validate_rhs(path: Path, row: dict[str, str], commit: str) -> None:
    with path.open(newline="", encoding="utf-8") as handle:
        rhs_rows = list(csv.DictReader(handle))
    probes = int(finite(row, "requested_probes"))
    require(len(rhs_rows) == 3 * probes + 1, f"unexpected RHS count in {path}")
    counts = {2: 0, 4: 0, 5: 0}
    for diagnostic in rhs_rows:
        require(diagnostic["source_commit"] == commit, f"RHS commit mismatch in {path}")
        stage = int(finite(diagnostic, "stage"))
        require(stage in counts, f"unexpected RHS stage {stage} in {path}")
        counts[stage] += 1
        require(finite(diagnostic, "converged") == 1, f"unchecked RHS success in {path}")
        require(
            finite(diagnostic, "relative_residual") <= 1e-9,
            f"complete residual exceeds registered gate in {path}",
        )
        require(finite(diagnostic, "iterations") >= 0, f"negative iteration count in {path}")
    require(counts == {2: 1, 4: probes, 5: 2 * probes}, f"bad RHS stage counts in {path}")


def result_path(run_dir: Path, scenario: str, route: str, suffix: str) -> Path:
    return run_dir / "numopt" / scenario / route / f"numopt_{scenario}_{route}{suffix}"


def validate_success(
    run_dir: Path,
    scenario: str,
    route: str,
    commit: str,
    require_scc: bool,
) -> tuple[dict[str, str], int | None]:
    row = read_one(result_path(run_dir, scenario, route, ".csv"))
    require(row["source_commit"] == commit, f"{scenario}/{route}: source mismatch")
    require(row["route"] == route and row["scenario"] == scenario, "route/scenario mismatch")
    require(row["estimator_status"] == "KSS_POINT_ESTIMATES_ONLY", "bad estimator status")
    require(finite(row, "converged") == 1 and finite(row, "command_rc") == 0, "route failed")
    require(0 < finite(row, "projected_seconds") <= 5400, "90-minute projection gate failed")
    require(finite(row, "tolerance") == 1e-10, "solver tolerance changed")
    require(finite(row, "seed") == 8675309, "benchmark seed changed")
    require(finite(row, "solver_max_residual") <= 1e-9, "complete residual gate failed")
    require(finite(row, "inverse_relres") <= 1e-9, "inverse residual gate failed")
    for timing in TIMINGS:
        require(finite(row, timing) >= 0, f"negative timing {timing}")
    for prefix in ("plugin", "correction", "corrected"):
        values = {target: finite(row, f"{prefix}_{target}") for target in TARGETS}
        identity = values["worker"] + values["firm"] + 2 * values["covariance"]
        require(abs(values["total"] - identity) <= 2e-10 * (1 + abs(identity)), "target identity failed")
    validate_rhs(result_path(run_dir, scenario, route, "_rhs.csv"), row, commit)
    marker = read_text(result_path(run_dir, scenario, route, ".stata.pass"))
    require(f"VCKSS_NUMOPT_CONVERGED {route} {scenario} {commit}" in marker, "bad marker")
    rss: int | None = None
    if require_scc:
        require(row["stata_version"].startswith("19"), "SCC result is not Stata 19")
        require(row["stata_flavor"] in {"IC", "MP"}, "unexpected Stata flavor")
        label = f"numopt_{scenario}_{route}"
        parse_qacct(run_dir / "qacct" / f"{label}.txt")
        rss = peak_rss(run_dir / "numopt" / scenario / route / "resources.txt")
    return row, rss


def validate_easy_cmg_failure(run_dir: Path, commit: str, require_scc: bool) -> None:
    path = result_path(run_dir, "easy", "cmg", ".csv")
    row = read_one(path)
    require(row["source_commit"] == commit, "easy/CMG source mismatch")
    require(finite(row, "converged") == 0, "easy/CMG unexpectedly converged")
    require(
        row["route_status"]
        in {"HIERARCHY_EDGE_LIMIT", "HIERARCHY_COMPLEXITY_LIMIT", "HIERARCHY_STALLED"},
        "easy/CMG did not preserve its typed hierarchy rejection",
    )
    require(0 < finite(row, "projected_seconds") <= 5400, "easy/CMG projection gate failed")
    marker = read_text(result_path(run_dir, "easy", "cmg", ".stata.pass"))
    require("VCKSS_NUMOPT_TYPED_FAILURE cmg easy" in marker, "bad easy/CMG marker")
    if require_scc:
        parse_qacct(run_dir / "qacct" / "numopt_easy_cmg.txt")
        peak_rss(run_dir / "numopt" / "easy" / "cmg" / "resources.txt")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--run-dir", required=True, type=Path)
    parser.add_argument("--expected-commit", required=True)
    parser.add_argument("--scenarios", nargs="+", default=["easy", "moderate", "weak"])
    parser.add_argument("--require-scc", action="store_true")
    args = parser.parse_args()
    require(re.fullmatch(r"[0-9a-f]{40}", args.expected_commit) is not None, "bad commit")
    require(set(args.scenarios) <= {"easy", "moderate", "weak"}, "bad scenario")
    if args.require_scc:
        require(
            read_text(args.run_dir / "source_commit.txt").strip() == args.expected_commit,
            "run source_commit.txt mismatch",
        )

    for scenario in args.scenarios:
        b1, b1_rss = validate_success(
            args.run_dir, scenario, "b1", args.expected_commit, args.require_scc
        )
        if scenario == "easy":
            validate_easy_cmg_failure(args.run_dir, args.expected_commit, args.require_scc)
            print("PASS easy: B1 converged; forced CMG failed closed")
            continue
        cmg, cmg_rss = validate_success(
            args.run_dir, scenario, "cmg", args.expected_commit, args.require_scc
        )
        for field in (
            "N_stored",
            "N_physical",
            "N_retained",
            "worker_levels",
            "firm_levels",
            "deletion_units",
            "target_weight_sum",
            "requested_probes",
            "seed",
            "tolerance",
        ):
            require(finite(b1, field) == finite(cmg, field), f"{scenario}: changed {field}")
        b1_estimates: list[float] = []
        cmg_estimates: list[float] = []
        for prefix in ("plugin", "correction", "corrected", "mcse"):
            for target in TARGETS:
                b1_estimates.append(finite(b1, f"{prefix}_{target}"))
                cmg_estimates.append(finite(cmg, f"{prefix}_{target}"))
        maximum_difference = matrix_relative_difference(b1_estimates, cmg_estimates)
        require(maximum_difference <= 2e-9, f"{scenario}: estimator mreldif gate failed")
        speedup = finite(b1, "command_seconds") / finite(cmg, "command_seconds")
        rss_text = ""
        if b1_rss is not None and cmg_rss is not None:
            rss_text = f", peak RSS B1/CMG={b1_rss}/{cmg_rss} KiB"
        print(f"PASS {scenario}: estimator mreldif={maximum_difference:.3g}, speedup={speedup:.3f}x{rss_text}")
    print("VCKSS NUMOPT EVIDENCE PASS; automatic routing remains disabled")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Validate privacy-safe SCC Separations wage benchmark evidence."""
from __future__ import annotations

import argparse
import csv
import math
import re
from pathlib import Path


TARGETS = ("worker", "firm", "covariance", "total")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def text(path: Path) -> str:
    require(path.is_file(), f"missing evidence: {path}")
    return path.read_text(encoding="utf-8", errors="replace")


def rows(path: Path) -> list[dict[str, str]]:
    require(path.is_file(), f"missing CSV: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        return list(csv.DictReader(handle))


def one(path: Path) -> dict[str, str]:
    result = rows(path)
    require(len(result) == 1, f"expected one row: {path}")
    return result[0]


def finite(row: dict[str, str], field: str) -> float:
    try:
        value = float(row[field])
    except (KeyError, ValueError) as exc:
        raise ValueError(f"invalid field {field}") from exc
    require(math.isfinite(value), f"nonfinite field {field}")
    return value


def qacct(root: Path, label: str) -> None:
    receipt = text(root / "qacct" / f"separations_{label}.txt")
    failed = re.search(r"(?m)^failed\s+(\d+)", receipt)
    status = re.search(r"(?m)^exit_status\s+(\d+)", receipt)
    require(failed is not None and failed.group(1) == "0", f"scheduler failure: {label}")
    require(status is not None and status.group(1) == "0", f"process failure: {label}")


def rss(path: Path) -> int:
    report = text(path)
    match = re.search(r"Maximum resident set size \(kbytes\):\s*(\d+)", report)
    require(match is not None, f"peak RSS missing: {path}")
    value = int(match.group(1))
    require(0 < value < 60 * 1024 * 1024, f"60 GiB RSS gate failed: {path}")
    return value


def matrix_relative_difference(left: list[float], right: list[float]) -> float:
    scale = max((abs(value) for value in left + right), default=0.0)
    difference = max((abs(a - b) for a, b in zip(left, right, strict=True)), default=0.0)
    return difference / max(scale, 1e-300)


def validate_rhs(path: Path, probes: int, commit: str) -> None:
    diagnostics = rows(path)
    require(len(diagnostics) == 3 * probes + 1, "incomplete RHS diagnostics")
    counts = {2: 0, 4: 0, 5: 0}
    for row in diagnostics:
        require(row["source_commit"] == commit, "RHS commit mismatch")
        stage = int(finite(row, "stage"))
        require(stage in counts, "invalid RHS stage")
        counts[stage] += 1
        require(finite(row, "converged") == 1, "unchecked RHS success")
        require(finite(row, "relative_residual") <= 1e-9, "complete residual failed")
    require(counts == {2: 1, 4: probes, 5: 2 * probes}, "RHS stage count mismatch")


def validate_stata_route(
    root: Path, label: str, route: str, commit: str
) -> tuple[dict[str, str], int]:
    directory = root / "separations" / label / route
    result = one(directory / f"separations_{label}_{route}.csv")
    require(result["source_commit"] == commit, "Stata source mismatch")
    require(result["label"] == label and result["route"] == route, "Stata label mismatch")
    require(result["stata_version"].startswith("19"), "real benchmark is not Stata 19")
    require(0 < finite(result, "projected_seconds") <= 5400, "90-minute projection failed")
    if route == "cmg":
        require(finite(result, "hybrid_vertices") > 0, "CMG hybrid vertex count missing")
        require(finite(result, "hybrid_edges") > 0, "CMG hybrid edge count missing")
    if finite(result, "converged") == 0:
        require(route == "cmg", "B1 failed")
        require(result["route_status"] not in {"", "INVALID_INPUT"}, "untyped CMG failure")
        qacct(root, f"{label}_{route}")
        return result, rss(directory / "resources.txt")
    require(result["estimator_status"] == "KSS_POINT_ESTIMATES_ONLY", "bad KSS status")
    require(finite(result, "seed") == 8675309, "seed mismatch")
    require(finite(result, "requested_probes") == 200, "probe mismatch")
    require(result["probe_order"] == "observation_key", "probe-order mismatch")
    require(finite(result, "tolerance") == 1e-10, "tolerance mismatch")
    require(finite(result, "solver_max_residual") <= 1e-9, "solver residual failed")
    require(finite(result, "inverse_relres") <= 1e-9, "inverse residual failed")
    for timing in (
        "command_seconds", "graph_seconds", "fit_seconds", "setup_seconds",
        "schur_seconds", "preconditioner_apply_seconds", "pcg_seconds",
        "leverage_seconds", "target_seconds", "correction_seconds",
    ):
        require(finite(result, timing) >= 0, f"negative {timing}")
    for prefix in ("plugin", "correction", "corrected"):
        values = {target: finite(result, f"{prefix}_{target}") for target in TARGETS}
        identity = values["worker"] + values["firm"] + 2 * values["covariance"]
        require(abs(values["total"] - identity) <= 2e-10 * (1 + abs(identity)), "identity failed")
    validate_rhs(directory / f"separations_{label}_{route}_rhs.csv", 200, commit)
    qacct(root, f"{label}_{route}")
    return result, rss(directory / "resources.txt")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--run-dir", required=True, type=Path)
    parser.add_argument("--expected-commit", required=True)
    parser.add_argument("--label", required=True)
    parser.add_argument("--expected-core-sha256", required=True)
    parser.add_argument("--expected-matlab-cmg-sha256", required=True)
    parser.add_argument("--expected-matlab-cmg-mex-sha256", required=True)
    parser.add_argument("--expected-matlab-cmg-solver-sha256", required=True)
    parser.add_argument("--matlab-only", action="store_true")
    args = parser.parse_args()
    require(re.fullmatch(r"[0-9a-f]{40}", args.expected_commit) is not None, "bad commit")
    require(re.fullmatch(r"[A-Za-z0-9._-]+", args.label) is not None, "bad label")
    require(re.fullmatch(r"[0-9a-f]{64}", args.expected_core_sha256) is not None, "bad core hash")
    require(re.fullmatch(r"[0-9a-f]{64}", args.expected_matlab_cmg_sha256) is not None, "bad CMG hash")
    require(re.fullmatch(r"[0-9a-f]{64}", args.expected_matlab_cmg_mex_sha256) is not None, "bad CMG MEX hash")
    require(re.fullmatch(r"[0-9a-f]{64}", args.expected_matlab_cmg_solver_sha256) is not None, "bad CMG solver hash")
    require(text(args.run_dir / "source_commit.txt").strip() == args.expected_commit, "run mismatch")
    base = args.run_dir / "separations" / args.label

    prepare = one(base / "prepare" / "prepare.csv")
    require(prepare["source_commit"] == args.expected_commit, "prepare source mismatch")
    require(prepare["stata_version"].startswith("19"), "prepare is not Stata 19")
    expected_selection = (
        "dense_mover_core"
        if prepare["sample_mode"] == "small"
        else "full_natural_graph"
    )
    require(prepare["sample_selection"] == expected_selection, "sample selection mismatch")
    qacct(args.run_dir, f"{args.label}_prepare")
    prepare_rss = rss(base / "prepare" / "resources.txt")

    b1: dict[str, str] | None = None
    cmg: dict[str, str] | None = None
    b1_rss = math.nan
    cmg_rss = math.nan
    cmg_converged = False
    estimator_difference = math.nan
    speedup = math.nan
    if not args.matlab_only:
        b1, b1_rss = validate_stata_route(
            args.run_dir, args.label, "b1", args.expected_commit
        )
        cmg, cmg_rss = validate_stata_route(
            args.run_dir, args.label, "cmg", args.expected_commit
        )
        cmg_converged = finite(cmg, "converged") == 1
        if cmg_converged:
            for field in (
                "prepared_sha256", "wage_input_sha256", "input_rows", "N_stored",
                "N_physical", "N_retained", "worker_levels", "firm_levels",
                "deletion_units", "requested_probes", "seed", "tolerance",
                "probe_order",
            ):
                if field.endswith("sha256") or field == "probe_order":
                    require(b1[field] == cmg[field], f"changed {field}")
                else:
                    require(finite(b1, field) == finite(cmg, field), f"changed {field}")
            b1_estimates = [
                finite(b1, f"{prefix}_{target}")
                for prefix in ("plugin", "correction", "corrected", "mcse")
                for target in TARGETS
            ]
            cmg_estimates = [
                finite(cmg, f"{prefix}_{target}")
                for prefix in ("plugin", "correction", "corrected", "mcse")
                for target in TARGETS
            ]
            estimator_difference = matrix_relative_difference(b1_estimates, cmg_estimates)
            require(estimator_difference <= 2e-9, "B1/CMG estimator equality failed")
            speedup = finite(b1, "command_seconds") / finite(cmg, "command_seconds")

    matlab = rows(base / "matlab" / "matlab.csv")
    require(len(matlab) == 4, "MATLAB must return four targets")
    by_target = {row["target"]: row for row in matlab}
    require(set(by_target) == set(TARGETS), "MATLAB target labels changed")
    for row in matlab:
        require(row["source_commit"] == args.expected_commit, "MATLAB source mismatch")
        require(row["kss_core_sha256"] == args.expected_core_sha256, "MATLAB core mismatch")
        require(row["matlab_cmg_sha256"] == args.expected_matlab_cmg_sha256, "MATLAB CMG mismatch")
        require(row["matlab_cmg_mex_sha256"] == args.expected_matlab_cmg_mex_sha256, "MATLAB CMG MEX mismatch")
        require(row["matlab_cmg_solver_sha256"] == args.expected_matlab_cmg_solver_sha256, "MATLAB CMG solver mismatch")
        require(finite(row, "seed") == 8675309 and finite(row, "probes") == 200, "MATLAB tuning mismatch")
        require(finite(row, "mex_setup_seconds") >= 0, "MATLAB MEX timing missing")
        require(finite(row, "command_seconds") > 0, "MATLAB timing missing")
    matlab_identity = (
        finite(by_target["worker"], "value") + finite(by_target["firm"], "value")
        + 2 * finite(by_target["covariance"], "value")
    )
    require(abs(finite(by_target["total"], "value") - matlab_identity) <= 1e-12 * (1 + abs(matlab_identity)), "MATLAB identity failed")
    projection = text(base / "matlab" / "projection.csv").strip().split(",", 1)
    require(len(projection) == 2 and 0 < float(projection[0]) <= 5400, "MATLAB projection failed")
    qacct(args.run_dir, f"{args.label}_matlab")
    matlab_rss = rss(base / "matlab" / "resources.txt")

    if args.matlab_only:
        print(
            f"PASS {args.label} MATLAB-only: "
            f"setup={finite(matlab[0], 'mex_setup_seconds'):.3f}s, "
            f"command={finite(matlab[0], 'command_seconds'):.3f}s, "
            f"RSS={matlab_rss} KiB, prepare RSS={prepare_rss} KiB"
        )
        print(
            "MATLAB output is descriptive: its selected sample, legacy "
            "finite-projection formula, and probe stream are not an API 17 "
            "equality oracle."
        )
        print("KSS_BC SEPARATIONS MATLAB-ONLY EVIDENCE PASS")
        return 0

    assert b1 is not None and cmg is not None

    overlap = one(base / "comparison" / "sample_overlap.csv")
    require(overlap["source_commit"] == args.expected_commit, "sample comparison source mismatch")
    if cmg_converged:
        require(finite(overlap, "b1_only_cmg") == 0 and finite(overlap, "cmg_only_b1") == 0, "B1/CMG retained samples differ")
    qacct(args.run_dir, f"{args.label}_compare")

    cmg_timing = (
        "failed closed"
        if not cmg_converged
        else f"{finite(cmg, 'command_seconds'):.3f}s"
    )
    print(
        f"PASS {args.label}: B1={finite(b1, 'command_seconds'):.3f}s, "
        f"CMG={cmg_timing}, MATLAB={finite(matlab[0], 'command_seconds'):.3f}s"
    )
    print(
        f"RSS KiB prepare/B1/CMG/MATLAB={prepare_rss}/{b1_rss}/{cmg_rss}/{matlab_rss}; "
        f"B1/CMG mreldif={estimator_difference}; speedup={speedup}"
    )
    print(
        "MATLAB comparison is descriptive: its legacy finite-projection formula and "
        "language-specific probe stream are not an equality oracle for API 17."
    )
    print("KSS_BC SEPARATIONS EVIDENCE PASS; automatic routing remains disabled")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

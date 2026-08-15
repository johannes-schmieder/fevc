#!/usr/bin/env python3
"""Validate SCC-only MATLAB-retained exact/B1/CMG benchmark evidence."""
from __future__ import annotations

import argparse
import math
import re
from pathlib import Path

from validate_separations import (
    TARGETS,
    finite,
    matrix_relative_difference,
    one,
    qacct,
    require,
    rss,
    text,
    validate_stata_route,
)


def validate_identity(row: dict[str, str], prefix: str) -> None:
    values = {target: finite(row, f"{prefix}_{target}") for target in TARGETS}
    identity = values["worker"] + values["firm"] + 2 * values["covariance"]
    require(
        abs(values["total"] - identity) <= 2e-10 * (1 + abs(identity)),
        f"{prefix} target identity failed",
    )


def validate_exact(
    run_dir: Path, label: str, commit: str, expected_rows: float
) -> tuple[dict[str, str], int]:
    directory = run_dir / "separations" / label / "exact"
    row = one(directory / f"separations_{label}_exact.csv")
    require(row["source_commit"] == commit, "exact source mismatch")
    require(row["label"] == label and row["route"] == "exact", "exact label mismatch")
    require(row["stata_version"].startswith("19"), "exact route is not Stata 19")
    require(finite(row, "converged") == 1, "exact route did not converge")
    require(row["estimator_status"] == "KSS_POINT_ESTIMATES_ONLY", "bad exact status")
    require(0 < finite(row, "projected_seconds") <= 5400, "exact projection failed")
    require(finite(row, "input_rows") == expected_rows, "exact input changed")
    require(finite(row, "N_stored") == expected_rows, "exact sample changed")
    require(finite(row, "N_retained") == expected_rows, "exact graph pruned rows")
    require(finite(row, "inverse_relres") <= 1e-9, "exact inverse residual failed")
    require(finite(row, "solver_max_residual") <= 1e-9, "exact solver residual failed")
    for prefix in ("plugin", "correction", "corrected"):
        validate_identity(row, prefix)
    qacct(run_dir, f"{label}_exact")
    return row, rss(directory / "resources.txt")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--run-dir", required=True, type=Path)
    parser.add_argument("--expected-commit", required=True)
    parser.add_argument("--label", required=True)
    parser.add_argument("--expected-matlab-commit", required=True)
    parser.add_argument("--expected-parent-prepared-sha256", required=True)
    parser.add_argument("--expected-matlab-detail-sha256", required=True)
    parser.add_argument("--expected-prepared-sha256", required=True)
    parser.add_argument("--expected-wage-sha256", required=True)
    parser.add_argument(
        "--omit-exact",
        action="store_true",
        help=(
            "validate a post-oracle scale step without the dense exact route; "
            "the default small-sample gate still requires exact"
        ),
    )
    parser.add_argument(
        "--oracle-run-dir",
        type=Path,
        help="source-bound run containing the prerequisite small exact oracle",
    )
    parser.add_argument(
        "--oracle-label",
        help="MATLAB-retained label containing the prerequisite small exact oracle",
    )
    args = parser.parse_args()
    require(re.fullmatch(r"[0-9a-f]{40}", args.expected_commit) is not None, "bad commit")
    require(re.fullmatch(r"[0-9a-f]{40}", args.expected_matlab_commit) is not None, "bad MATLAB commit")
    require(re.fullmatch(r"[A-Za-z0-9._-]+", args.label) is not None, "bad label")
    if args.omit_exact:
        require(args.oracle_run_dir is not None, "post-oracle step requires oracle run")
        require(args.oracle_label is not None, "post-oracle step requires oracle label")
        require(
            re.fullmatch(r"[A-Za-z0-9._-]+", args.oracle_label) is not None,
            "bad oracle label",
        )
    else:
        require(args.oracle_run_dir is None, "oracle run requires --omit-exact")
        require(args.oracle_label is None, "oracle label requires --omit-exact")
    for value in (
        args.expected_parent_prepared_sha256,
        args.expected_matlab_detail_sha256,
        args.expected_prepared_sha256,
        args.expected_wage_sha256,
    ):
        require(re.fullmatch(r"[0-9a-f]{64}", value) is not None, "bad SHA-256")
    require(text(args.run_dir / "source_commit.txt").strip() == args.expected_commit, "run mismatch")

    base = args.run_dir / "separations" / args.label
    prepare = one(base / "prepare" / "prepare.csv")
    require(prepare["source_commit"] == args.expected_commit, "prepare source mismatch")
    require(prepare["matlab_source_commit"] == args.expected_matlab_commit, "MATLAB source mismatch")
    require(prepare["sample_mode"] == "matlab", "prepare mode mismatch")
    require(prepare["sample_selection"] == "matlab_retained_bridge_core", "selection mismatch")
    require(prepare["parent_prepared_sha256"] == args.expected_parent_prepared_sha256, "parent sample mismatch")
    require(prepare["matlab_detail_sha256"] == args.expected_matlab_detail_sha256, "MATLAB detail mismatch")
    require(prepare["wage_input_sha256"] == args.expected_wage_sha256, "wage input mismatch")
    require(prepare["stata_version"].startswith("19"), "prepare is not Stata 19")
    matlab_rows = finite(prepare, "matlab_rows")
    stored_rows = finite(prepare, "stored_rows")
    removed_rows = finite(prepare, "graph_removed_rows") + finite(prepare, "bridge_removed_rows")
    require(matlab_rows == stored_rows + removed_rows, "sample row accounting failed")
    require(stored_rows > 0 and finite(prepare, "matches") > 0, "empty prepared sample")
    require(finite(prepare, "audit_iterations") >= 1, "sample audit missing")
    recorded_sha = text(base / "prepare" / "prepared.dta.sha256").strip()
    require(recorded_sha == args.expected_prepared_sha256, "derived sample hash mismatch")
    qacct(args.run_dir, f"{args.label}_matlab-sample")
    prepare_rss = rss(base / "prepare" / "resources.txt")

    exact: dict[str, str] | None = None
    exact_rss: int | None = None
    if not args.omit_exact:
        exact, exact_rss = validate_exact(
            args.run_dir, args.label, args.expected_commit, stored_rows
        )
    else:
        assert args.oracle_run_dir is not None and args.oracle_label is not None
        require(
            text(args.oracle_run_dir / "source_commit.txt").strip()
            == args.expected_commit,
            "oracle run source mismatch",
        )
        oracle_prepare = one(
            args.oracle_run_dir
            / "separations"
            / args.oracle_label
            / "prepare"
            / "prepare.csv"
        )
        require(
            oracle_prepare["sample_selection"] == "matlab_retained_bridge_core",
            "oracle is not a MATLAB-retained sample",
        )
        validate_exact(
            args.oracle_run_dir,
            args.oracle_label,
            args.expected_commit,
            finite(oracle_prepare, "stored_rows"),
        )
    b1, b1_rss = validate_stata_route(args.run_dir, args.label, "b1", args.expected_commit)
    cmg, cmg_rss = validate_stata_route(args.run_dir, args.label, "cmg", args.expected_commit)
    require(finite(b1, "converged") == 1, "B1 did not converge")
    require(finite(cmg, "converged") == 1, "CMG did not converge")
    for row, route in ((b1, "B1"), (cmg, "CMG")):
        require(finite(row, "input_rows") == stored_rows, f"{route} input changed")
        require(finite(row, "N_stored") == stored_rows, f"{route} sample changed")
        require(finite(row, "N_retained") == stored_rows, f"{route} graph pruned rows")
        require(row["prepared_sha256"] == args.expected_prepared_sha256, f"{route} sample hash mismatch")

    fixed_fields = (
        "prepared_sha256", "wage_input_sha256", "input_rows", "N_stored",
        "N_physical", "N_retained", "worker_levels", "firm_levels",
        "deletion_units", "requested_probes", "seed", "tolerance", "probe_order",
    )
    for field in fixed_fields:
        if field.endswith("sha256") or field == "probe_order":
            require(b1[field] == cmg[field], f"B1/CMG changed {field}")
        else:
            require(finite(b1, field) == finite(cmg, field), f"B1/CMG changed {field}")
    estimator_fields = [
        f"{prefix}_{target}"
        for prefix in ("plugin", "correction", "corrected", "mcse")
        for target in TARGETS
    ]
    difference = matrix_relative_difference(
        [finite(b1, field) for field in estimator_fields],
        [finite(cmg, field) for field in estimator_fields],
    )
    require(difference <= 2e-9, "B1/CMG estimator equality failed")
    plugin_difference = math.nan
    if exact is not None:
        plugin_difference = matrix_relative_difference(
            [finite(exact, f"plugin_{target}") for target in TARGETS],
            [finite(b1, f"plugin_{target}") for target in TARGETS],
        )
        require(plugin_difference <= 2e-9, "exact/B1 plug-in equality failed")

    overlap = one(base / "comparison" / "sample_overlap.csv")
    require(overlap["source_commit"] == args.expected_commit, "comparison source mismatch")
    require(finite(overlap, "b1_only_cmg") == 0, "B1-only retained matches")
    require(finite(overlap, "cmg_only_b1") == 0, "CMG-only retained matches")
    qacct(args.run_dir, f"{args.label}_compare")

    speedup = finite(b1, "command_seconds") / finite(cmg, "command_seconds")
    require(math.isfinite(speedup) and speedup > 0, "invalid CMG speedup")
    if exact is None:
        print(
            f"PASS {args.label} post-oracle scale step: "
            f"B1={finite(b1, 'command_seconds'):.3f}s, "
            f"CMG={finite(cmg, 'command_seconds'):.3f}s, speedup={speedup:.3f}x"
        )
        print(
            "RSS KiB prepare/B1/CMG="
            f"{prepare_rss}/{b1_rss}/{cmg_rss}; "
            f"B1/CMG mreldif={difference}; exact deliberately omitted"
        )
        print(
            f"prerequisite exact oracle={args.oracle_label} "
            f"in {args.oracle_run_dir}"
        )
    else:
        print(
            f"PASS {args.label}: exact={finite(exact, 'command_seconds'):.3f}s, "
            f"B1={finite(b1, 'command_seconds'):.3f}s, "
            f"CMG={finite(cmg, 'command_seconds'):.3f}s, speedup={speedup:.3f}x"
        )
        print(
            "RSS KiB prepare/exact/B1/CMG="
            f"{prepare_rss}/{exact_rss}/{b1_rss}/{cmg_rss}; "
            f"B1/CMG mreldif={difference}; "
            f"exact/B1 plugin mreldif={plugin_difference}"
        )
    print("KSS_BC MATLAB-RETAINED REAL-DATA EVIDENCE PASS; automatic routing remains disabled")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

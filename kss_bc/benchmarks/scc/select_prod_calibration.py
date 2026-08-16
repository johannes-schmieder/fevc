#!/usr/bin/env python3
"""Select a CZ18 CMG configuration from paired setup/slope calibrations."""
from __future__ import annotations

import argparse
import csv
import hashlib
import math
import re
from pathlib import Path


FULL_PROBES = 200
CALIBRATION_PROBES = (20, 40)
SAFETY_FACTOR = 1.5  # Retained for the separate larger-stress projection.
SETUP_SAFETY_FACTOR = 1.25
MARGINAL_SAFETY_FACTOR = 1.5
FIXED_HEADROOM_SECONDS = 120
MINIMUM_TIMEOUT_SECONDS = 300
MAXIMUM_TIMEOUT_SECONDS = 5400
REQUESTED_TOLERANCE = 1e-10
COMPLETE_RESIDUAL_GATE = max(1e-11, 10 * REQUESTED_TOLERANCE)
FORMULA = (
    "beta=max(0,(t40-t20)/20);alpha=max(0,t20-20*beta);"
    "ceil(max(300,1.25*alpha+1.5*200*beta+120))"
)

CALIBRATION_MEMORY_GIB = 56
BATCH_MEMORY_FRACTION = 0.35
BATCH_BASE_WIDTH = 8
BATCH_BASE_SCRATCH_BYTES = 7_447_296_000
FEASIBLE_EXPLICIT_BATCHES = (8, 16)
INFEASIBLE_BATCHES = (32, 64, 128)
CALIBRATION_ROUTES = ("auto", "cmg")
CALIBRATION_BATCH_REQUESTS = ("8", "16", "auto")
CALIBRATION_PATTERN = re.compile(
    r"cal_(auto|cmg)_b(8|16|auto)_p(20|40)_(cold|warm)"
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def read_one(path: Path) -> dict[str, str]:
    require(path.is_file(), f"missing calibration output: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    require(len(rows) == 1, f"calibration output must have one row: {path}")
    return rows[0]


def read_plan(path: Path) -> dict[str, dict[str, str]]:
    require(path.is_file(), f"missing experiment plan: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle, delimiter="\t"))
    return {row["experiment_id"]: row for row in rows}


def finite(row: dict[str, str], field: str) -> float:
    try:
        value = float(row[field])
    except (KeyError, ValueError) as exc:
        raise ValueError(f"invalid {field}") from exc
    require(math.isfinite(value), f"nonfinite {field}")
    return value


def identity(row: dict[str, str], prefix: str) -> None:
    total = finite(row, f"{prefix}_total")
    parts = finite(row, f"{prefix}_worker") + finite(row, f"{prefix}_firm")
    parts += 2 * finite(row, f"{prefix}_covariance")
    require(abs(total - parts) <= 1e-8 * (1 + abs(parts)), f"{prefix} identity failed")


def memory_bytes(value: str) -> float:
    match = re.fullmatch(r"([0-9]+(?:\.[0-9]+)?)([KMGTP]?)", value.strip(), re.I)
    require(match is not None, f"unparseable memory value: {value}")
    scale = {"": 1, "K": 1024, "M": 1024**2, "G": 1024**3,
             "T": 1024**4, "P": 1024**5}[match.group(2).upper()]
    return float(match.group(1)) * scale


def batch_memory_budget_bytes() -> int:
    return math.floor(BATCH_MEMORY_FRACTION * CALIBRATION_MEMORY_GIB * 1024**3)


def forecast_batch_bytes(width: int) -> int:
    require(width > 0 and width % BATCH_BASE_WIDTH == 0, "invalid forecast batch width")
    return BATCH_BASE_SCRATCH_BYTES * width // BATCH_BASE_WIDTH


def projection_from_pair(t20: float, t40: float) -> tuple[float, float, float, int]:
    require(math.isfinite(t20) and math.isfinite(t40) and t20 >= 0 and t40 >= 0,
            "invalid paired timing")
    beta = max(0.0, (t40 - t20) / 20.0)
    alpha = max(0.0, t20 - 20.0 * beta)
    projected = (SETUP_SAFETY_FACTOR * alpha +
                 MARGINAL_SAFETY_FACTOR * FULL_PROBES * beta +
                 FIXED_HEADROOM_SECONDS)
    timeout = math.ceil(max(MINIMUM_TIMEOUT_SECONDS, projected))
    return alpha, beta, projected, timeout


def candidate_sort_key(row: dict[str, object]) -> tuple[object, ...]:
    # Integer-second projections reflect the timeout decision's resolution.
    # At equal projected ceilings, prefer the public automatic route.
    return (
        row["timeout_seconds"],
        0 if row["preconditioner"] == "auto" else 1,
        row["projected_seconds"],
        row["batch"],
    )


def qacct_pass(path: Path, processors: int, expected_job_id: str) -> None:
    require(path.is_file(), f"missing calibration qacct: {path}")
    text = path.read_text(encoding="utf-8", errors="replace")
    def field(name: str) -> str:
        matches = re.findall(rf"(?m)^{name}\s+(\S+)", text)
        require(len(matches) == 1, f"qacct missing/duplicate {name}: {path}")
        return matches[0]
    require(field("jobnumber") == expected_job_id.split(".", 1)[0],
            f"qacct job number mismatch: {path}")
    require(field("failed") == "0" and field("exit_status") == "0", f"failed qacct: {path}")
    require(int(field("slots")) == processors, f"qacct slot mismatch: {path}")
    require(memory_bytes(field("maxvmem")) <= 60 * 1024**3,
            f"qacct memory exceeded policy: {path}")
    for name in ("ru_wallclock", "cpu", "hostname", "qname"):
        field(name)


def resource_pass(path: Path) -> None:
    require(path.is_file(), f"missing resource evidence: {path}")
    text = path.read_text(encoding="utf-8", errors="replace")
    match = re.search(r"Maximum resident set size \(kbytes\):\s*(\d+)", text)
    require(match is not None and 0 < int(match.group(1)) <= 60 * 1024**2,
            f"resource RSS exceeded policy or is absent: {path}")


def validate_rhs(path: Path, expected: int) -> None:
    require(path.is_file(), f"missing RHS evidence: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    require(len(rows) == expected, f"RHS evidence count mismatch: {path}")
    require(all(finite(row, "converged") == 1 for row in rows), f"unaccepted RHS: {path}")
    require(all(finite(row, "relative_residual") >= 0 for row in rows),
            f"negative RHS residual: {path}")
    residual = max(finite(row, "relative_residual") for row in rows)
    require(residual <= COMPLETE_RESIDUAL_GATE * (1 + 1e-10), f"RHS residual gate failed: {path}")


def evidence_digest(run_dir: Path, experiment_ids: list[str]) -> str:
    digest = hashlib.sha256()
    relative_paths: list[Path] = []
    for experiment in experiment_ids:
        root = Path("experiments") / experiment
        relative_paths.extend((
            root / f"prod_{experiment}.csv",
            root / "retained_matches.csv",
            root / "resources.txt",
            root / "wrapper.pass",
            root / "rhs_repetition_1.csv",
            Path("qacct") / f"{experiment}.txt",
        ))
    for relative in sorted(relative_paths, key=str):
        path = run_dir / relative
        require(path.is_file(), f"missing calibration evidence: {path}")
        digest.update(str(relative).encode() + b"\0")
        digest.update(path.read_bytes())
    return digest.hexdigest()


def validate_row(
    args: argparse.Namespace,
    plan: dict[str, dict[str, str]],
    experiment: str,
) -> dict[str, str]:
    spec = plan.get(experiment)
    require(spec is not None and spec["stage"] == "calibration", "unplanned calibration")
    root = args.run_dir / "experiments" / experiment
    row = read_one(root / f"prod_{experiment}.csv")
    require(row["bundle_sha256"] == args.bundle_sha, "bundle changed in calibration")
    require(row["source_commit"] == args.source_commit, "source changed in calibration")
    require(row["data_manifest_sha256"] == args.data_manifest_sha,
            "data manifest changed in calibration")
    require(row["prepared_sha256"] == args.prepared_sha, "prepared sample changed")
    require(row["estimator_input_sha256"] == args.prepared_sha, "estimator input changed")
    require(row["wage_input_sha256"] == args.wage_sha, "wage input changed")
    require(row["temperature"] == spec["temperature"], "calibration temperature changed")
    require(row["estimator_status"] == "KSS_POINT_ESTIMATES_ONLY", "calibration failed")
    require(row["algorithm_requested"] == row["algorithm_selected"] == "jla",
            "calibration did not exercise JLA")
    require(finite(row, "command_rc") == 0 and finite(row, "command_seconds") >= 0,
            "calibration command failed or has invalid timing")
    require(abs(finite(row, "tolerance") - REQUESTED_TOLERANCE) <= 1e-20,
            "calibration tolerance changed")
    require(0 <= finite(row, "solver_max_residual") <=
            COMPLETE_RESIDUAL_GATE * (1 + 1e-10), "calibration residual failed")
    require(int(finite(row, "requested_probes")) == int(spec["probes"]),
            "calibration probes changed")
    require(int(finite(row, "seed")) == 8675309, "calibration seed changed")
    require(int(finite(row, "declared_memory_gib")) == CALIBRATION_MEMORY_GIB,
            "calibration memory changed")
    require(int(finite(row, "requested_processors")) == int(spec["processors"]) and
            int(finite(row, "actual_processors")) == int(spec["processors"]),
            "calibration processor count was not bound exactly")
    require(row["batch_requested"] == spec["batch"], "calibration batch changed")
    selected_batch = int(finite(row, "selected_batch"))
    require(selected_batch in FEASIBLE_EXPLICIT_BATCHES, "calibration selected infeasible batch")
    if spec["batch"] != "auto":
        require(selected_batch == int(spec["batch"]), "explicit calibration batch changed")
    require(int(finite(row, "batch_scratch_forecast_bytes")) ==
            forecast_batch_bytes(selected_batch), "batch scratch forecast changed")
    require(finite(row, "batch_scratch_forecast_bytes") <= batch_memory_budget_bytes(),
            "calibration batch exceeds registered scratch budget")
    require(row["preconditioner_requested"] == spec["preconditioner"],
            "calibration route changed")
    require(finite(row, "rng_state_reproducible") == 1,
            "calibration terminal RNG state is not reproducible")
    require(finite(row, "route_planned_rhs") == 3 * finite(row, "requested_probes") + 1,
            "calibration RHS plan changed")
    identity(row, "plugin")
    identity(row, "corrected")
    expected_job_id = (args.run_dir / "submissions" /
                       f"{experiment}.job_id").read_text(encoding="utf-8").strip()
    qacct_pass(args.run_dir / "qacct" / f"{experiment}.txt",
               int(spec["processors"]), expected_job_id)
    resource_pass(root / "resources.txt")
    validate_rhs(root / "rhs_repetition_1.csv", int(finite(row, "route_planned_rhs")))
    wrapper = root / "wrapper.pass"
    expected = (f"KSS_PROD_WRAPPER_PASS {experiment} {args.bundle_sha} "
                f"{args.source_commit} {args.data_manifest_sha}\n")
    require(wrapper.read_text(encoding="utf-8") == expected, "calibration wrapper mismatch")
    row["_retained_bytes"] = (root / "retained_matches.csv").read_bytes()  # type: ignore[assignment]
    return row


def compare_cold_warm(cold: dict[str, str], warm: dict[str, str]) -> None:
    for field in ("plugin_worker", "plugin_firm", "plugin_covariance", "plugin_total",
                  "corrected_worker", "corrected_firm", "corrected_covariance", "corrected_total"):
        left, right = finite(cold, field), finite(warm, field)
        require(abs(left - right) <= 1e-10 * (1 + abs(left)), f"cold/warm {field} changed")
    require(cold["_retained_bytes"] == warm["_retained_bytes"],
            "cold/warm retained sample changed")
    for field in ("selected_batch", "N_retained", "worker_levels", "firm_levels",
                  "deletion_units", "route_hybrid_vertices", "route_hybrid_edges",
                  "route_hierarchy_levels", "route_planned_rhs"):
        require(finite(warm, field) == finite(cold, field), f"cold/warm {field} changed")
    require(warm["preconditioner_selected"] == cold["preconditioner_selected"],
            "cold/warm selected route changed")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--run-dir", required=True, type=Path)
    parser.add_argument("--plan", required=True, type=Path)
    parser.add_argument("--bundle-sha", required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--data-manifest-sha", required=True)
    parser.add_argument("--prepared-sha", required=True)
    parser.add_argument("--wage-sha", required=True)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    require(re.fullmatch(r"[0-9a-f]{64}", args.bundle_sha) is not None, "invalid bundle SHA")
    require(re.fullmatch(r"[0-9a-f]{40}", args.source_commit) is not None, "invalid source commit")
    for value in (args.data_manifest_sha, args.prepared_sha, args.wage_sha):
        require(re.fullmatch(r"[0-9a-f]{64}", value) is not None, "invalid input SHA")

    plan = read_plan(args.plan)
    calibration_ids = sorted(
        experiment for experiment, spec in plan.items() if spec["stage"] == "calibration"
    )
    require(len(calibration_ids) == 24 and
            all(CALIBRATION_PATTERN.fullmatch(experiment) for experiment in calibration_ids),
            "calibration plan is not the registered 24-cell paired matrix")
    evidence = {experiment: validate_row(args, plan, experiment)
                for experiment in calibration_ids}
    candidates: list[dict[str, object]] = []

    for route in CALIBRATION_ROUTES:
        for batch_request in CALIBRATION_BATCH_REQUESTS:
            paired: dict[int, dict[str, dict[str, str]]] = {}
            for probes in CALIBRATION_PROBES:
                cold_id = f"cal_{route}_b{batch_request}_p{probes}_cold"
                warm_id = f"cal_{route}_b{batch_request}_p{probes}_warm"
                cold, warm = evidence[cold_id], evidence[warm_id]
                compare_cold_warm(cold, warm)
                paired[probes] = {"cold": cold, "warm": warm}

            p20, p40 = paired[20], paired[40]
            for temperature in ("cold", "warm"):
                low, high = p20[temperature], p40[temperature]
                require(low["_retained_bytes"] == high["_retained_bytes"],
                        "P20/P40 retained sample changed")
                for field in ("plugin_worker", "plugin_firm", "plugin_covariance", "plugin_total"):
                    require(abs(finite(low, field) - finite(high, field)) <=
                            1e-10 * (1 + abs(finite(low, field))),
                            f"P20/P40 plug-in {field} changed")
                for field in ("selected_batch", "N_retained", "worker_levels", "firm_levels",
                              "deletion_units", "route_hybrid_vertices", "route_hybrid_edges",
                              "route_hierarchy_levels"):
                    require(finite(low, field) == finite(high, field),
                            f"P20/P40 {field} changed")
                require(low["preconditioner_selected"] == high["preconditioner_selected"],
                        "P20/P40 selected route changed")

            selected_batches = {int(finite(row, "selected_batch"))
                                for pair in paired.values() for row in pair.values()}
            selected_routes = {row["preconditioner_selected"]
                               for pair in paired.values() for row in pair.values()}
            t20 = max(finite(p20["cold"], "command_seconds"),
                      finite(p20["warm"], "command_seconds"))
            t40 = max(finite(p40["cold"], "command_seconds"),
                      finite(p40["warm"], "command_seconds"))
            alpha, beta, projected, timeout = projection_from_pair(t20, t40)
            if (len(selected_batches) != 1 or selected_routes != {"cmg"} or
                    timeout > MAXIMUM_TIMEOUT_SECONDS):
                continue
            selected_batch = selected_batches.pop()
            candidates.append({
                "p20_cold_experiment": f"cal_{route}_b{batch_request}_p20_cold",
                "p20_warm_experiment": f"cal_{route}_b{batch_request}_p20_warm",
                "p40_cold_experiment": f"cal_{route}_b{batch_request}_p40_cold",
                "p40_warm_experiment": f"cal_{route}_b{batch_request}_p40_warm",
                "preconditioner": route,
                "selected_preconditioner": "cmg",
                "batch_request": batch_request,
                "batch": selected_batch,
                "processors": 4,
                "actual_processors": 4,
                "memory_gib": CALIBRATION_MEMORY_GIB,
                "t20_seconds": t20,
                "t40_seconds": t40,
                "alpha_seconds": alpha,
                "beta_seconds_per_probe": beta,
                "projected_seconds": projected,
                "timeout_seconds": timeout,
            })

    require(candidates, "no paired CMG calibration projects below 5,400 seconds")
    selected = min(candidates, key=candidate_sort_key)
    budget = batch_memory_budget_bytes()
    selected.update({
        "bundle_sha256": args.bundle_sha,
        "source_commit": args.source_commit,
        "data_manifest_sha256": args.data_manifest_sha,
        "prepared_sha256": args.prepared_sha,
        "wage_input_sha256": args.wage_sha,
        "calibration_evidence_sha256": evidence_digest(args.run_dir, calibration_ids),
        "full_probes": FULL_PROBES,
        "formula": FORMULA,
        "candidate_count": len(candidates),
        "measured_candidate_count": 6,
        "batch_memory_fraction": BATCH_MEMORY_FRACTION,
        "batch_memory_budget_bytes": budget,
        "batch8_scratch_forecast_bytes": forecast_batch_bytes(8),
        "batch16_scratch_forecast_bytes": forecast_batch_bytes(16),
        "batch32_scratch_forecast_bytes": forecast_batch_bytes(32),
        "batch64_scratch_forecast_bytes": forecast_batch_bytes(64),
        "batch128_scratch_forecast_bytes": forecast_batch_bytes(128),
        "feasible_batch_widths": "8|16|auto",
        "infeasible_batch_widths": "32|64|128",
    })
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(selected))
        writer.writeheader()
        writer.writerow(selected)
    print("KSS_PROD CALIBRATION SELECTOR PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

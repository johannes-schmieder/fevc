#!/usr/bin/env python3
"""Select an admissible CZ18 production configuration from accepted evidence."""
from __future__ import annotations

import argparse
import csv
import hashlib
import math
import re
from pathlib import Path


FULL_PROBES = 200
SAFETY_FACTOR = 1.5
FIXED_HEADROOM_SECONDS = 120
MINIMUM_TIMEOUT_SECONDS = 300
MAXIMUM_TIMEOUT_SECONDS = 5400
REQUESTED_TOLERANCE = 1e-10
COMPLETE_RESIDUAL_GATE = max(1e-11, 10 * REQUESTED_TOLERANCE)
FORMULA = "ceil(max(300,max(cold,warm)*(200/calibration_probes)*1.5+120))"


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
    calibration_root = args.run_dir / "experiments"
    cold_paths = sorted(calibration_root.glob("cal_*_cold/prod_cal_*_cold.csv"))
    require(cold_paths, "no cold calibration outputs")
    candidates: list[dict[str, object]] = []
    evidence_ids: list[str] = []
    for cold_path in cold_paths:
        cold_id = cold_path.parent.name
        warm_id = cold_id.removesuffix("_cold") + "_warm"
        warm_path = calibration_root / warm_id / f"prod_{warm_id}.csv"
        cold = read_one(cold_path)
        warm = read_one(warm_path)
        evidence_ids.extend((cold_id, warm_id))
        for experiment, row, temperature in (
            (cold_id, cold, "cold"), (warm_id, warm, "warm")
        ):
            spec = plan.get(experiment)
            require(spec is not None and spec["stage"] == "calibration", "unplanned calibration")
            require(row["bundle_sha256"] == args.bundle_sha, "bundle changed in calibration")
            require(row["source_commit"] == args.source_commit, "source changed in calibration")
            require(row["data_manifest_sha256"] == args.data_manifest_sha,
                    "data manifest changed in calibration")
            require(row["prepared_sha256"] == args.prepared_sha, "prepared sample changed")
            require(row["estimator_input_sha256"] == args.prepared_sha, "estimator input changed")
            require(row["wage_input_sha256"] == args.wage_sha, "wage input changed")
            require(row["temperature"] == temperature, "cold/warm evidence mixed")
            require(row["estimator_status"] == "KSS_POINT_ESTIMATES_ONLY", "calibration failed")
            require(row["algorithm_requested"] == "jla" and row["algorithm_selected"] == "jla",
                    "calibration did not exercise JLA")
            require(finite(row, "command_rc") == 0, "calibration command failed")
            require(abs(finite(row, "tolerance") - REQUESTED_TOLERANCE) <= 1e-20,
                    "calibration tolerance changed")
            require(0 <= finite(row, "solver_max_residual") <=
                    COMPLETE_RESIDUAL_GATE * (1 + 1e-10),
                    "calibration residual failed")
            require(finite(row, "command_seconds") >= 0,
                    "calibration command time is negative")
            require(int(finite(row, "requested_probes")) == int(spec["probes"]),
                    "calibration probes changed")
            require(int(finite(row, "seed")) == 8675309, "calibration seed changed")
            require(int(finite(row, "declared_memory_gib")) == int(spec["memory_gib"]) <= 56,
                    "calibration memory changed")
            require(int(finite(row, "requested_processors")) == int(spec["processors"]),
                    "calibration processor request changed")
            require(int(finite(row, "actual_processors")) == int(spec["processors"]),
                    "calibration processor count was not bound exactly")
            require(row["batch_requested"] == spec["batch"], "calibration batch changed")
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
            resource_pass(calibration_root / experiment / "resources.txt")
            validate_rhs(calibration_root / experiment / "rhs_repetition_1.csv",
                         int(finite(row, "route_planned_rhs")))
            wrapper = calibration_root / experiment / "wrapper.pass"
            expected = (f"KSS_PROD_WRAPPER_PASS {experiment} {args.bundle_sha} "
                        f"{args.source_commit} {args.data_manifest_sha}\n")
            require(wrapper.read_text(encoding="utf-8") == expected, "calibration wrapper mismatch")
        for field in ("plugin_worker", "plugin_firm", "plugin_covariance", "plugin_total",
                      "corrected_worker", "corrected_firm", "corrected_covariance", "corrected_total"):
            left, right = finite(cold, field), finite(warm, field)
            require(abs(left - right) <= 1e-10 * (1 + abs(left)), f"cold/warm {field} changed")
        cold_retained = (cold_path.parent / "retained_matches.csv").read_bytes()
        warm_retained = (warm_path.parent / "retained_matches.csv").read_bytes()
        require(cold_retained == warm_retained, "cold/warm retained sample changed")
        calibration_probes = int(finite(cold, "requested_probes"))
        requested_processors = int(finite(cold, "requested_processors"))
        actual_processors = int(finite(cold, "actual_processors"))
        require(int(finite(warm, "requested_processors")) == requested_processors,
                "cold/warm processor request changed")
        require(int(finite(warm, "actual_processors")) == actual_processors,
                "cold/warm processor capacity changed")
        require(int(finite(warm, "selected_batch")) ==
                int(finite(cold, "selected_batch")),
                "cold/warm selected batch changed")
        require(warm["preconditioner_selected"] == cold["preconditioner_selected"],
                "cold/warm selected route changed")
        for field in ("N_retained", "worker_levels", "firm_levels",
                      "deletion_units", "route_hybrid_vertices",
                      "route_hybrid_edges", "route_hierarchy_levels",
                      "route_planned_rhs"):
            require(finite(warm, field) == finite(cold, field),
                    f"cold/warm {field} changed")
        observed = max(finite(cold, "command_seconds"), finite(warm, "command_seconds"))
        projected = observed * (FULL_PROBES / calibration_probes) * SAFETY_FACTOR
        timeout = math.ceil(max(MINIMUM_TIMEOUT_SECONDS, projected + FIXED_HEADROOM_SECONDS))
        # CZ18 is the large-hybrid CMG qualification. Diagonal calibrations
        # remain correctness/performance evidence but are not admissible as
        # the production selector's terminal route.
        if timeout > MAXIMUM_TIMEOUT_SECONDS or cold["preconditioner_selected"] != "cmg" or \
                warm["preconditioner_selected"] != "cmg":
            continue
        candidates.append(
            {
                "cold_experiment": cold_id,
                "warm_experiment": warm_id,
                "preconditioner": cold["preconditioner_requested"],
                "selected_preconditioner": cold["preconditioner_selected"],
                "batch": cold["selected_batch"],
                "processors": requested_processors,
                "actual_processors": actual_processors,
                "memory_gib": int(finite(cold, "declared_memory_gib")),
                "calibration_probes": calibration_probes,
                "cold_seconds": finite(cold, "command_seconds"),
                "warm_seconds": finite(warm, "command_seconds"),
                "projected_seconds": projected,
                "timeout_seconds": timeout,
            }
        )
    require(candidates, "no CMG calibration configuration projects below 5,400 seconds")
    selected = min(candidates, key=lambda row: (row["projected_seconds"], row["memory_gib"], row["processors"]))
    selected.update(
        {
            "bundle_sha256": args.bundle_sha,
            "source_commit": args.source_commit,
            "data_manifest_sha256": args.data_manifest_sha,
            "prepared_sha256": args.prepared_sha,
            "wage_input_sha256": args.wage_sha,
            "calibration_evidence_sha256": evidence_digest(args.run_dir, evidence_ids),
            "full_probes": FULL_PROBES,
            "formula": FORMULA,
            "candidate_count": len(candidates),
        }
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(selected))
        writer.writeheader()
        writer.writerow(selected)
    print("KSS_PROD CALIBRATION SELECTOR PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

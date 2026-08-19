#!/usr/bin/env python3
"""Fail-closed validation and analysis of paired PREP-BND-1 SCC jobs."""

from __future__ import annotations

import argparse
import csv
import json
import statistics
from datetime import UTC, datetime
from pathlib import Path

from common import (
    HEX40,
    HEX64,
    INPUT_CZ18_SHA256,
    MARKER_CZ18,
    MARKER_SCALE,
    PREP_BND_METRICS,
    SCIENTIFIC_ABS_REL_TOLERANCE,
    TIMING_FIELDS,
    compare_scientific_contract,
    parse_qacct,
    profile_map,
    read_key_value,
    read_profiles,
    read_rows,
    require,
    sha256,
    validate_causal_transition,
)

CASES = tuple(f"F{firms}-P256" for firms in (256, 1024, 4096, 8192)) + ("CZ18-P20",)
ORDERS = ("ab", "ba")
ROLES = ("baseline", "candidate")
LEDGER_FIELDS = (
    "job_id",
    "case",
    "order",
    "result_relpath",
    "scheduler_log",
    "expected_slots",
    "repetitions",
)


def bounded(root: Path, relative: str) -> Path:
    candidate = (root / relative).resolve()
    require(candidate == root.resolve() or root.resolve() in candidate.parents, "path escapes run root")
    return candidate


def read_ledger(path: Path) -> list[dict[str, str]]:
    require(path.is_file(), f"missing job ledger: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle, delimiter="\t")
        require(tuple(reader.fieldnames or ()) == LEDGER_FIELDS, "job ledger schema differs")
        rows = list(reader)
    require(len(rows) == len(CASES) * len(ORDERS), "job ledger must contain ten scalar jobs")
    require(len({row["job_id"] for row in rows}) == len(rows), "duplicate job ID")
    require(
        {(row["case"], row["order"]) for row in rows}
        == {(case, order) for case in CASES for order in ORDERS},
        "job ledger case/order matrix is incomplete",
    )
    return rows


def percent(candidate: float, baseline: float) -> float | None:
    return None if baseline == 0 else 100 * (candidate / baseline - 1)


def optional_median(values: list[float | None]) -> float | None:
    observed = [value for value in values if value is not None]
    return statistics.median(observed) if observed else None


def median_fields(rows: list[dict[str, str]], fields: tuple[str, ...]) -> dict[str, float]:
    return {field: statistics.median(float(row[field]) for row in rows) for field in fields}


def median_profile(
    rows: list[dict[str, str]], runs: tuple[int, ...]
) -> dict[str, float]:
    return {
        metric: statistics.median(
            profile_map(rows, "prep_boundary_profile", run)[metric] for run in runs
        )
        for metric in PREP_BND_METRICS
    }


def validate_source_receipt(
    receipt: dict[str, str],
    *,
    job: dict[str, str],
    baseline: str,
    candidate: str,
    baseline_bundle_sha256: str,
    candidate_bundle_sha256: str,
    driver_sha256: str,
    qacct: dict[str, str],
) -> None:
    expected = {
        "schema": "varcomp-kss-prep-bnd1-node-v1",
        "job_id": job["job_id"],
        "case": job["case"],
        "order": job["order"],
        "baseline": baseline,
        "candidate": candidate,
        "baseline_bundle_sha256": baseline_bundle_sha256,
        "candidate_bundle_sha256": candidate_bundle_sha256,
        "driver_sha256": driver_sha256,
        "requested_slots": job["expected_slots"],
        "stata_processors": "4",
        "repetitions": job["repetitions"],
    }
    for key, value in expected.items():
        require(receipt.get(key) == value, f"{job['case']}/{job['order']}: receipt mismatch {key}")
    expected_input = INPUT_CZ18_SHA256 if job["case"] == "CZ18-P20" else "GENERATED_DETERMINISTIC"
    require(receipt.get("input_sha256") == expected_input, "input hash binding failed")
    receipt_host = receipt.get("hostname", "").split(".", 1)[0]
    accounting_host = qacct["hostname"].split(".", 1)[0]
    require(receipt_host == accounting_host, "node receipt and qacct host differ")


def validate_job(
    *,
    run_root: Path,
    qacct_dir: Path,
    job: dict[str, str],
    baseline: str,
    candidate: str,
    baseline_bundle_sha256: str,
    candidate_bundle_sha256: str,
    driver_sha256: str,
) -> dict[str, object]:
    case = job["case"]
    order = job["order"]
    slots = int(job["expected_slots"])
    repetitions = int(job["repetitions"])
    require(slots == (14 if case == "CZ18-P20" else 4), f"{case}: resource policy differs")
    require(repetitions == 3, f"{case}: expected three repetitions")
    result_root = bounded(run_root, job["result_relpath"])
    scheduler_log = bounded(run_root, job["scheduler_log"])
    accounting_path = qacct_dir / f"{job['job_id']}.txt"
    qacct = parse_qacct(accounting_path, job["job_id"], slots)
    receipt = read_key_value(result_root / "node_receipt.tsv")
    validate_source_receipt(
        receipt,
        job=job,
        baseline=baseline,
        candidate=candidate,
        baseline_bundle_sha256=baseline_bundle_sha256,
        candidate_bundle_sha256=candidate_bundle_sha256,
        driver_sha256=driver_sha256,
        qacct=qacct,
    )
    wrapper_marker = f"PREP_BND1_PAIR_PASS {case} {order} baseline={baseline} candidate={candidate}"
    require((result_root / "pair.pass").read_text(encoding="utf-8").strip() == wrapper_marker, "pair marker differs")
    require(scheduler_log.is_file() and wrapper_marker in scheduler_log.read_text(encoding="utf-8"), "scheduler log lacks wrapper marker")

    role_rows: dict[str, list[dict[str, str]]] = {}
    role_profiles: dict[str, list[dict[str, str]]] = {}
    artifacts: dict[str, str] = {
        "node_receipt.tsv": sha256(result_root / "node_receipt.tsv"),
        "pair.pass": sha256(result_root / "pair.pass"),
        "scheduler_log": sha256(scheduler_log),
        "qacct": sha256(accounting_path),
    }
    if case == "CZ18-P20":
        for role, commit in (("baseline", baseline), ("candidate", candidate)):
            role_rows[role] = []
            role_profiles[role] = []
            for round_ in range(1, 4):
                directory = result_root / f"round{round_}-{role}"
                summary_path = directory / "summary.csv"
                profiles_path = directory / "profiles.csv"
                log_path = directory / "application.log"
                rows = read_rows(summary_path, role, commit, 1)
                require(rows[0]["pair_order"] == order, "CZ18 order binding failed")
                require(int(rows[0]["pair_round"]) == round_, "CZ18 round binding failed")
                profiles = read_profiles(profiles_path, role, commit, 1)
                marker = f"{MARKER_CZ18} {role} {commit}"
                require(marker in log_path.read_text(encoding="utf-8"), "CZ18 application marker missing")
                role_rows[role].extend(rows)
                for item in profiles:
                    adjusted = dict(item)
                    adjusted["run"] = str(round_)
                    role_profiles[role].append(adjusted)
                for item in (summary_path, profiles_path, log_path, directory / "resources.txt"):
                    artifacts[str(item.relative_to(result_root))] = sha256(item)
    else:
        firms, probes = case.removeprefix("F").split("-P", 1)
        for role, commit in (("baseline", baseline), ("candidate", candidate)):
            summary_path = result_root / f"{role}.csv"
            profiles_path = result_root / f"{role}_profiles.csv"
            log_path = result_root / f"{role}.log"
            role_rows[role] = read_rows(summary_path, role, commit, repetitions)
            role_profiles[role] = read_profiles(profiles_path, role, commit, repetitions)
            marker = f"{MARKER_SCALE} {role} {commit}"
            require(marker in log_path.read_text(encoding="utf-8"), "scale application marker missing")
            for item in (
                summary_path,
                profiles_path,
                log_path,
                result_root / f"{role}_resources.txt",
            ):
                artifacts[str(item.relative_to(result_root))] = sha256(item)

    compare_scientific_contract(
        role_rows["baseline"], role_rows["candidate"], f"{case}/{order}"
    )
    for role in ROLES:
        for run in range(1, repetitions + 1):
            profile = profile_map(role_profiles[role], "prep_boundary_profile", run)
            require(set(profile) == set(PREP_BND_METRICS), f"{case}/{order}/{role}: PREP-BND profile absent")
    causal_transitions = validate_causal_transition(
        role_profiles["baseline"], role_profiles["candidate"], role_rows["candidate"]
    )
    timed_rows = slice(1, None) if case != "CZ18-P20" else slice(None)
    timed_runs = (2, 3) if case != "CZ18-P20" else (1, 2, 3)
    baseline_times = median_fields(role_rows["baseline"][timed_rows], TIMING_FIELDS)
    candidate_times = median_fields(role_rows["candidate"][timed_rows], TIMING_FIELDS)
    baseline_prep = median_profile(role_profiles["baseline"], timed_runs)
    candidate_prep = median_profile(role_profiles["candidate"], timed_runs)
    return {
        "case": case,
        "order": order,
        "job_id": job["job_id"],
        "hostname": qacct["hostname"],
        "qname": qacct["qname"],
        "qacct_wall_seconds": float(qacct["ru_wallclock"]),
        "qacct_cpu_seconds": float(qacct["cpu"]),
        "qacct_maxvmem": qacct["maxvmem"],
        "requested_slots": slots,
        "stata_processors": 4,
        "temperature_policy": "warm_rows_2_3" if case != "CZ18-P20" else "fresh_process_rounds_1_3",
        "baseline_median_seconds": baseline_times,
        "candidate_median_seconds": candidate_times,
        "candidate_change_percent": {
            field: percent(candidate_times[field], baseline_times[field]) for field in TIMING_FIELDS
        },
        "baseline_prep_boundary_median_seconds": baseline_prep,
        "candidate_prep_boundary_median_seconds": candidate_prep,
        "candidate_prep_boundary_change_percent": {
            field: percent(candidate_prep[field], baseline_prep[field]) for field in PREP_BND_METRICS
        },
        "structural_sample_rng_comparison": "EXACT_PASS",
        "scientific_comparison": "TOLERANCE_PASS",
        "scientific_abs_rel_tolerance": SCIENTIFIC_ABS_REL_TOLERANCE,
        "causal_exposure_transition": causal_transitions,
        "artifact_sha256": artifacts,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--run-root", type=Path, required=True)
    parser.add_argument("--jobs-tsv", type=Path, required=True)
    parser.add_argument("--qacct-dir", type=Path, required=True)
    parser.add_argument("--baseline", required=True)
    parser.add_argument("--candidate", required=True)
    parser.add_argument("--baseline-bundle-sha256", required=True)
    parser.add_argument("--candidate-bundle-sha256", required=True)
    parser.add_argument("--driver-sha256", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(HEX40.fullmatch(args.baseline) is not None, "invalid baseline")
    require(HEX40.fullmatch(args.candidate) is not None, "invalid candidate")
    for value in (
        args.baseline_bundle_sha256,
        args.candidate_bundle_sha256,
        args.driver_sha256,
    ):
        require(HEX64.fullmatch(value) is not None, "invalid source SHA-256")
    ledger = read_ledger(args.jobs_tsv)
    jobs = [
        validate_job(
            run_root=args.run_root,
            qacct_dir=args.qacct_dir,
            job=job,
            baseline=args.baseline,
            candidate=args.candidate,
            baseline_bundle_sha256=args.baseline_bundle_sha256,
            candidate_bundle_sha256=args.candidate_bundle_sha256,
            driver_sha256=args.driver_sha256,
        )
        for job in ledger
    ]
    by_case = {
        case: {
            field: optional_median([
                item["candidate_change_percent"][field]
                for item in jobs if item["case"] == case
            ])
            for field in TIMING_FIELDS
        }
        for case in CASES
    }
    summary = {
        "schema": "varcomp-kss-prep-bnd1-scc-summary-v1",
        "status": "PASS",
        "generated_at_utc": datetime.now(UTC).isoformat(),
        "baseline": args.baseline,
        "candidate": args.candidate,
        "baseline_bundle_sha256": args.baseline_bundle_sha256,
        "candidate_bundle_sha256": args.candidate_bundle_sha256,
        "driver_sha256": args.driver_sha256,
        "jobs": jobs,
        "median_candidate_change_percent_by_case": by_case,
        "performance_thresholds_are_advisory": True,
        "structural_sample_rng_comparison": "EXACT_PASS",
        "scientific_comparison": "TOLERANCE_PASS",
        "scientific_abs_rel_tolerance": SCIENTIFIC_ABS_REL_TOLERANCE,
        "qacct_application_source_validation": "PASS",
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print("PREP_BND1_SCC_ANALYSIS_PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

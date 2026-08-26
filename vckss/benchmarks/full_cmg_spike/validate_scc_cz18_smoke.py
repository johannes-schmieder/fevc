#!/usr/bin/env python3
"""Validate one fixed-CZ18 P20/P200 private full-CMG SCC comparison."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import re
from pathlib import Path


RETAINED_SHA = "1748ca2a6a46f248e05c0329407e7e7708ec7628c1ffce5f0e06ee264bdf0575"
BASELINE_COMMIT = "4124b34f3ca216dcc3aae27e4b31bbac9e011f11"
CMG_COMMIT = "dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10"
LEGACY_P20_NODE_COMMIT = "c1d9b8eca7387d2d69448ffd32ab36d7d966a724"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def key_values(path: Path, separator: str = "=") -> dict[str, str]:
    result: dict[str, str] = {}
    for raw in path.read_text(encoding="utf-8").splitlines():
        if not raw:
            continue
        fields = raw.split(separator, 1)
        require(len(fields) == 2 and fields[0], f"invalid receipt row: {path}")
        require(fields[0] not in result, f"duplicate receipt key: {fields[0]}")
        result[fields[0]] = fields[1]
    return result


def one_csv(path: Path) -> dict[str, str]:
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    require(len(rows) == 1, f"expected one CSV row: {path}")
    return rows[0]


def finite(row: dict[str, str], name: str) -> float:
    value = float(row[name])
    require(math.isfinite(value), f"nonfinite {name}")
    return value


def qacct(path: Path) -> dict[str, str]:
    result: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        fields = line.strip().split(None, 1)
        if len(fields) == 2:
            result[fields[0]] = fields[1]
    required = {
        "jobnumber",
        "taskid",
        "project",
        "granted_pe",
        "slots",
        "failed",
        "exit_status",
        "ru_wallclock",
        "cpu",
        "maxvmem",
        "hostname",
    }
    require(required <= result.keys(), "incomplete qacct")
    return result


def max_rss_kib(path: Path) -> int:
    match = re.search(
        r"Maximum resident set size \(kbytes\):\s*([0-9]+)",
        path.read_text(encoding="utf-8"),
    )
    require(match is not None, f"missing GNU-time RSS: {path}")
    return int(match.group(1))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--run-dir", type=Path, required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--job-id", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(re.fullmatch(r"[0-9a-f]{40}", args.source_commit) is not None,
            "invalid source commit")
    require(re.fullmatch(r"[0-9]+", args.job_id) is not None, "invalid job ID")

    task_path = args.run_dir / "receipts/task.txt"
    task = key_values(task_path)
    node = key_values(args.run_dir / "receipts/node.txt")
    require(task["schema"] == "VCKSS_FULL_CMG_CZ18_SMOKE_TASK_V1",
            "task schema changed")
    require(node["schema"] == "VCKSS_FULL_CMG_CZ18_SCC_SMOKE_V1" and
            node["status"] == "PASS", "node receipt failed")
    require(task["source_commit"] == node["vckss_commit"] ==
            args.source_commit, "source binding changed")
    require(task["baseline_commit"] == node["baseline_commit"] ==
            BASELINE_COMMIT, "baseline binding changed")
    require(task["cmg_commit"] == node["cmg_commit"] == CMG_COMMIT,
            "CMG binding changed")
    require(task["input_sha256"] == node["input_sha256"] == RETAINED_SHA,
            "restricted input binding changed")
    probes = int(task["probes"])
    require(probes in (20, 200) and task["seed"] == "8675309",
            "CZ18 probe/seed contract changed")
    expected_experiment = (
        "cz18_p20_smoke" if probes == 20 else "cz18_p200_decision"
    )
    if "probes" in node or "experiment" in node:
        require(node.get("probes") == str(probes) and
                node.get("experiment") == expected_experiment,
                "CZ18 node experiment contract changed")
    else:
        require(probes == 20 and args.source_commit == LEGACY_P20_NODE_COMMIT,
                "missing CZ18 node experiment receipt")
    require(task["application_threads"] == node["application_threads"] == "4",
            "application thread contract changed")
    expected_inner_tolerance = "1e-8" if probes == 20 else "1e-9"
    require(task["candidate_probe_inner_tolerance"] ==
            node["candidate_probe_inner_tolerance"] == expected_inner_tolerance,
            "candidate inner tolerance contract changed")
    fast_preparation = int(task["candidate_fast_preparation"])
    require(fast_preparation in (0, 1) and
            node["candidate_fast_preparation"] == str(fast_preparation),
            "candidate fast-preparation contract changed")
    require(task["requested_slots"] == node["requested_slots"] == "14",
            "reservation contract changed")
    require(sha256(task_path) == node["task_sha256"] ==
            (args.run_dir / "receipts/task.sha256").read_text().strip(),
            "task hash changed")

    baseline = one_csv(args.run_dir / "artifacts/baseline/stata.csv")
    candidate = one_csv(args.run_dir / "artifacts/candidate/stata.csv")
    for role, row, commit in (
        ("baseline", baseline, BASELINE_COMMIT),
        ("candidate", candidate, args.source_commit),
    ):
        require(row["schema"] == "VCKSS-FULL-CMG-CZ18-STATA-V1" and
                row["role"] == role and row["application_status"] == "PASS",
                f"{role} application receipt failed")
        require(row["source_commit"] == commit and
                row["task_sha256"] == node["task_sha256"] and
                row["input_sha256"] == RETAINED_SHA,
                f"{role} identity changed")
        for field, expected in (
            ("rows", 8201888),
            ("workers", 117529),
            ("firms", 10603),
            ("coefficient_cells", 311730),
            ("probes", probes),
            ("processors", 4),
            ("sample_count", 8201888),
        ):
            require(int(finite(row, field)) == expected,
                    f"{role} dimension changed: {field}")
        for field in ("data_restored", "rng_restored", "sort_rng_restored"):
            require(int(finite(row, field)) == 1, f"{role} failed {field}")
        require(finite(row, "command_seconds") > 0,
                f"{role} command time failed")
        require(finite(row, "max_complete_residual") <=
                finite(row, "residual_acceptance"),
                f"{role} complete residual failed")
        require(abs(finite(row, "target_identity_residual")) <= 1e-12,
                f"{role} target identity failed")

    common_probe_gates: list[dict[str, float | int | bool]] = []
    for index in range(1, 5):
        left = finite(baseline, f"corrected{index}")
        right = finite(candidate, f"corrected{index}")
        left_mcse = finite(baseline, f"mcse{index}")
        right_mcse = finite(candidate, f"mcse{index}")
        scale = max(1.0, abs(left), abs(right))
        limit = max(1e-8 * scale, 0.1 * max(left_mcse, right_mcse))
        difference = abs(left - right)
        common_probe_gates.append({
            "target": index,
            "difference": difference,
            "limit": limit,
            "ratio": difference / limit if limit else 0.0,
            "pass": difference <= limit,
        })
    require(all(bool(gate["pass"]) for gate in common_probe_gates),
            "common-probe corrected-target gate failed")

    candidate_application = (
        args.run_dir / "artifacts/candidate/application.txt"
    ).read_text(encoding="utf-8")
    fast_diagnostics = re.findall(
        r"CMG_FULL_SPIKE_V1 SETUP .*?fast_preparation=([01])",
        candidate_application,
    )
    require(fast_diagnostics == [str(fast_preparation)],
            "candidate fast-preparation diagnostic changed")

    matlab = json.loads(
        (args.run_dir / "artifacts/matlab/aggregate.json").read_text(
            encoding="utf-8"
        )
    )
    source = json.loads(
        (args.run_dir / "artifacts/matlab/source_identity.json").read_text(
            encoding="utf-8"
        )
    )
    process_tree = json.loads(
        (args.run_dir / "artifacts/matlab/process_tree.json").read_text(
            encoding="utf-8"
        )
    )
    require(matlab["schema"] == "CMG-MATA-1-CZ18-MATLAB-AGGREGATE-V1" and
            matlab["status"] == source["status"] == "PASS",
            "MATLAB application or source identity failed")
    require(matlab["input_sha256"] == RETAINED_SHA and
            int(matlab["probes"]) == probes and int(matlab["pool_workers"]) == 4,
            "MATLAB input or worker contract changed")
    require(float(matlab["command_seconds"]) > 0 and
            float(matlab["target_identity_scaled_error"]) <= 1e-12,
            "MATLAB timing or identity failed")
    require(process_tree["status"] == "PASS" and
            process_tree["all_named_pids_observed_as_descendants"] is True,
            "MATLAB process-tree gate failed")

    accounting = qacct(args.run_dir / "qacct" / f"{args.job_id}.txt")
    require(accounting["jobnumber"] == node["job_id"] == args.job_id,
            "scheduler job identity changed")
    require(accounting["taskid"] == "undefined" and
            accounting["project"] == "welfgr" and
            accounting["failed"] == accounting["exit_status"] == "0",
            "scheduler completion failed")
    require(int(accounting["slots"]) == int(node["actual_slots"]) == 14,
            "scheduler slots changed")
    require((args.run_dir / "receipts/wrapper.pass").is_file() and
            not (args.run_dir / "receipts/wrapper.fail").exists(),
            "wrapper marker failed")

    baseline_seconds = finite(baseline, "command_seconds")
    candidate_seconds = finite(candidate, "command_seconds")
    matlab_seconds = float(matlab["command_seconds"])
    payload = {
        "schema": "VCKSS_FULL_CMG_CZ18_SCC_SMOKE_VALIDATION_V1",
        "status": "PASS",
        "promotion_status": (
            "P20_FAST_PREPARATION_SMOKE_ONLY"
            if probes == 20 and fast_preparation
            else "P20_SMOKE_ONLY"
            if probes == 20
            else "P200_FAST_PREPARATION_SINGLE_RUN_ONLY"
            if fast_preparation
            else "P200_SINGLE_RUN_DECISION_ONLY"
        ),
        "source_commit": args.source_commit,
        "baseline_commit": BASELINE_COMMIT,
        "cmg_commit": CMG_COMMIT,
        "job_id": args.job_id,
        "host": accounting["hostname"],
        "task_sha256": sha256(task_path),
        "input_sha256": RETAINED_SHA,
        "probes": probes,
        "candidate_probe_inner_tolerance": float(expected_inner_tolerance),
        "candidate_fast_preparation": bool(fast_preparation),
        "timing_seconds": {
            "baseline": baseline_seconds,
            "candidate": candidate_seconds,
            "matlab": matlab_seconds,
        },
        "ratios": {
            "candidate_over_baseline": candidate_seconds / baseline_seconds,
            "candidate_over_matlab": candidate_seconds / matlab_seconds,
            "matlab_speedup": matlab_seconds / candidate_seconds,
        },
        "common_probe_corrected_target_gates": common_probe_gates,
        "matlab_corrected_target_gate": (
            "DESCRIPTIVE_SINGLE_SEED_NO_REGISTERED_DISTRIBUTION"
        ),
        "max_rss_kib": {
            "baseline": max_rss_kib(
                args.run_dir / "artifacts/baseline/resources.txt"
            ),
            "candidate": max_rss_kib(
                args.run_dir / "artifacts/candidate/resources.txt"
            ),
            "matlab": max_rss_kib(
                args.run_dir / "artifacts/matlab/resources.txt"
            ),
        },
        "qacct": {
            "wall_seconds": float(accounting["ru_wallclock"]),
            "cpu": accounting["cpu"],
            "maxvmem": accounting["maxvmem"],
        },
        "evidence_sha256": {
            "task": sha256(task_path),
            "node": sha256(args.run_dir / "receipts/node.txt"),
            "baseline": sha256(args.run_dir / "artifacts/baseline/stata.csv"),
            "candidate": sha256(args.run_dir / "artifacts/candidate/stata.csv"),
            "candidate_application": sha256(
                args.run_dir / "artifacts/candidate/application.txt"
            ),
            "matlab": sha256(args.run_dir / "artifacts/matlab/aggregate.json"),
            "qacct": sha256(args.run_dir / "qacct" / f"{args.job_id}.txt"),
        },
    }
    require(not args.output.exists(), "validation output already exists")
    args.output.write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"VCKSS_FULL_CMG_CZ18_SCC_VALIDATION_PASS {args.job_id}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

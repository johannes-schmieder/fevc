#!/usr/bin/env python3
"""Validate the production fixed-CZ18 VCkss/MATLAB SCC matrix."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import re
import statistics
from pathlib import Path
from typing import Any

INPUT_SHA256 = "1748ca2a6a46f248e05c0329407e7e7708ec7628c1ffce5f0e06ee264bdf0575"
CMG_COMMIT = "dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10"
PRIVATE_WINNER_COMMIT = "3daa465aa56b48d65457c626381f9600c55afdbc"
PRIVATE_WINNER_SECONDS = 33.942
ROUNDS = (
    ("00-cold", "cold", ("vckss", "matlab")),
    ("01-warm", "warm", ("matlab", "vckss")),
    ("02-warm", "warm", ("vckss", "matlab")),
    ("03-warm", "warm", ("matlab", "vckss")),
    ("04-warm", "warm", ("vckss", "matlab")),
    ("05-warm", "warm", ("matlab", "vckss")),
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def key_values(path: Path) -> dict[str, str]:
    result: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        key, separator, value = line.partition("=")
        require(bool(separator) and key and key not in result, f"invalid receipt: {path}")
        result[key] = value
    return result


def one_csv(path: Path) -> dict[str, str]:
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    require(len(rows) == 1, f"expected one CSV row: {path}")
    return rows[0]


def finite(row: dict[str, Any], field: str) -> float:
    value = float(row[field])
    require(math.isfinite(value), f"nonfinite {field}")
    return value


def max_rss_bytes(path: Path) -> int:
    match = re.search(
        r"Maximum resident set size \(kbytes\):\s*([0-9]+)",
        path.read_text(encoding="utf-8"),
    )
    require(match is not None, f"missing GNU time RSS: {path}")
    return int(match.group(1)) * 1024


def qacct(path: Path) -> dict[str, str]:
    result: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        fields = line.strip().split(None, 1)
        if len(fields) == 2:
            result[fields[0]] = fields[1]
    required = {"jobnumber", "project", "slots", "failed", "exit_status", "maxvmem"}
    require(required <= result.keys(), "incomplete qacct receipt")
    return result


def expected_order() -> list[dict[str, str]]:
    rows: list[dict[str, str]] = []
    for round_name, temperature, roles in ROUNDS:
        for position, role in enumerate(roles, start=1):
            rows.append({
                "round": round_name,
                "temperature": temperature,
                "position": str(position),
                "role": role,
            })
    return rows


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-dir", type=Path, required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--job-id", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(re.fullmatch(r"[0-9a-f]{40}", args.source_commit) is not None,
            "invalid source commit")
    require(re.fullmatch(r"[0-9]+", args.job_id) is not None, "invalid job ID")

    task_path = args.run_dir / "receipts/task.txt"
    node_path = args.run_dir / "receipts/node.txt"
    order_path = args.run_dir / "receipts/order.tsv"
    qacct_path = args.run_dir / "qacct/qacct.txt"
    task = key_values(task_path)
    node = key_values(node_path)
    accounting = qacct(qacct_path)
    require(task["schema"] == "VCKSS_FULL_CMG_PRODUCTION_CZ18_TASK_V1",
            "task schema changed")
    require(node["schema"] == "VCKSS_FULL_CMG_PRODUCTION_CZ18_SCC_V1"
            and node["status"] == "PASS", "node receipt failed")
    require(task["source_commit"] == node["vckss_commit"] == args.source_commit,
            "source binding changed")
    require(task["cmg_commit"] == node["cmg_commit"] == CMG_COMMIT,
            "CMG binding changed")
    require(task["input_sha256"] == node["input_sha256"] == INPUT_SHA256,
            "input binding changed")
    require(task["rust_toolchain"] == "1.85.1"
            and node["rustc"].startswith("rustc 1.85.1 "), "Rust toolchain changed")
    require(task["matlab_module"] == node["matlab_module"] == "matlab/2024b",
            "MATLAB module changed")
    require(node["stata_module"] == "stata-mp/19", "Stata module changed")
    require(task["application_threads"] == node["application_threads"] == "4"
            and node["matlab_pool_workers"] == "4", "worker contract changed")
    require(task["requested_slots"] == node["requested_slots"] == node["actual_slots"] == "14",
            "slot contract changed")
    require(task["rounds"] == node["rounds"] == "6"
            and task["warm_rounds"] == node["warm_rounds"] == "5",
            "repetition contract changed")
    require(sha256(task_path) == node["task_sha256"]
            == (args.run_dir / "receipts/task.sha256").read_text().strip(),
            "task hash changed")
    with order_path.open(newline="", encoding="utf-8") as handle:
        order = list(csv.DictReader(handle, delimiter="\t"))
    require(order == expected_order() and sha256(order_path) == node["order_sha256"],
            "position-balanced order changed")
    require((args.run_dir / "receipts/wrapper.pass").is_file(), "wrapper PASS missing")
    require(accounting["jobnumber"] == args.job_id and accounting["project"] == "welfgr",
            "qacct identity changed")
    require(accounting["slots"] == "14" and accounting["failed"] == "0"
            and accounting["exit_status"] == "0", "SGE accounting failed")

    source_identity = json.loads(
        (args.run_dir / "artifacts/preparation/matlab_source_identity.json").read_text()
    )
    require(source_identity["status"] == "PASS", "MATLAB source identity failed")

    rows: dict[str, list[dict[str, Any]]] = {"vckss": [], "matlab": []}
    evidence: dict[str, str] = {
        "task": sha256(task_path),
        "node": sha256(node_path),
        "order": sha256(order_path),
        "qacct": sha256(qacct_path),
        "validator": sha256(Path(__file__)),
        "matlab_source_identity": sha256(
            args.run_dir / "artifacts/preparation/matlab_source_identity.json"
        ),
    }
    for round_name, temperature, roles in ROUNDS:
        round_root = args.run_dir / "artifacts/runs" / round_name
        stata_root = round_root / "vckss"
        stata = one_csv(stata_root / "stata.csv")
        require(stata["schema"] == "VCKSS-FULL-CMG-PRODUCTION-CZ18-STATA-V1"
                and stata["application_status"] == "PASS", f"{round_name} VCkss failed")
        require(stata["source_commit"] == args.source_commit
                and stata["task_sha256"] == node["task_sha256"]
                and stata["input_sha256"] == INPUT_SHA256, f"{round_name} VCkss binding failed")
        require(stata["cmg_backend"] == "CMG_FULL_V2"
                and stata["cmg_source_commit"] == CMG_COMMIT,
                f"{round_name} production backend changed")
        for field, expected in (
            ("rows", 8_201_888), ("workers", 117_529), ("firms", 10_603),
            ("coefficient_cells", 311_730), ("probes", 200),
            ("processors", 4), ("sample_count", 8_201_888),
            ("cmg_threads_requested", 4), ("cmg_threads_used", 4),
            ("cmg_rhs_count", 601),
        ):
            require(int(finite(stata, field)) == expected,
                    f"{round_name} dimension/receipt changed: {field}")
        require(finite(stata, "cmg_max_complete_residual")
                <= finite(stata, "residual_acceptance"), f"{round_name} residual failed")
        require(finite(stata, "cmg_admitted_peak_bytes")
                <= finite(stata, "cmg_pre_rng_forecast_bytes"),
                f"{round_name} memory receipt failed")
        for field in ("data_restored", "rng_restored", "sort_rng_restored"):
            require(int(finite(stata, field)) == 1, f"{round_name} failed {field}")
        stata.update({
            "round": round_name,
            "temperature": temperature,
            "position": roles.index("vckss") + 1,
            "process_peak_rss_bytes": max_rss_bytes(stata_root / "resources.txt"),
        })
        rows["vckss"].append(stata)

        matlab_root = round_root / "matlab"
        matlab = json.loads((matlab_root / "aggregate.json").read_text())
        process_tree = json.loads((matlab_root / "process_tree.json").read_text())
        require(matlab["status"] == "PASS" and matlab["input_sha256"] == INPUT_SHA256,
                f"{round_name} MATLAB binding failed")
        require(int(matlab["pool_workers"]) == 4 and int(matlab["probes"]) == 200,
                f"{round_name} MATLAB worker/probe count changed")
        require(process_tree["status"] == "PASS"
                and int(process_tree["expected_pool_workers"]) == 4,
                f"{round_name} MATLAB process tree failed")
        matlab.update({
            "round": round_name,
            "temperature": temperature,
            "position": roles.index("matlab") + 1,
            "process_peak_rss_bytes": int(process_tree["peak_rss_kib"]) * 1024,
        })
        rows["matlab"].append(matlab)
        for role in ("vckss", "matlab"):
            root = round_root / role
            for filename in ("application.txt", "resources.txt"):
                evidence[f"{round_name}_{role}_{filename}"] = sha256(root / filename)

    medians = {
        role: statistics.median(finite(row, "command_seconds") for row in values[1:])
        for role, values in rows.items()
    }
    rss_medians = {
        role: statistics.median(finite(row, "process_peak_rss_bytes") for row in values[1:])
        for role, values in rows.items()
    }
    vckss_reference = rows["vckss"][0]
    matlab_reference = rows["matlab"][0]
    target_fields = (
        ("worker", "corrected1", "corrected_worker"),
        ("firm", "corrected2", "corrected_firm"),
        ("covariance", "corrected3", "corrected_covariance"),
        ("total", "corrected4", "corrected_total"),
    )
    target_differences = []
    for name, left, right in target_fields:
        vckss_value = finite(vckss_reference, left)
        matlab_value = finite(matlab_reference, right)
        difference = abs(vckss_value - matlab_value)
        limit = 1.0e-3 * max(1.0, abs(vckss_value), abs(matlab_value))
        target_differences.append({
            "target": name,
            "vckss": vckss_value,
            "matlab": matlab_value,
            "absolute_difference": difference,
            "descriptive_scale_limit": limit,
            "descriptive_scale_check": "PASS" if difference <= limit else "FAIL",
        })
    require(all(item["descriptive_scale_check"] == "PASS" for item in target_differences),
            "corrected targets are not statistically comparable")
    production_over_matlab = medians["vckss"] / medians["matlab"]
    production_over_private = medians["vckss"] / PRIVATE_WINNER_SECONDS
    summary = {
        "schema": "VCKSS-FULL-CMG-PRODUCTION-CZ18-SCC-VALIDATION-V1",
        "status": "PASS",
        "source_commit": args.source_commit,
        "job_id": args.job_id,
        "host": node["hostname"],
        "input_sha256": INPUT_SHA256,
        "cmg_commit": CMG_COMMIT,
        "private_winner_commit": PRIVATE_WINNER_COMMIT,
        "private_winner_seconds": PRIVATE_WINNER_SECONDS,
        "warm_median_command_seconds": medians,
        "warm_median_process_peak_rss_bytes": rss_medians,
        "production_over_matlab": production_over_matlab,
        "production_over_private_winner": production_over_private,
        "faster_than_matlab_gate": production_over_matlab < 1.0,
        "within_five_percent_private_winner_gate": production_over_private <= 1.05,
        "peak_rss_no_greater_than_matlab_gate": rss_medians["vckss"] <= rss_medians["matlab"],
        "two_x_matlab_objective": production_over_matlab <= 0.5,
        "matlab_target_comparison": "DESCRIPTIVE_SCALE_CHECK_ONLY_INDEPENDENT_RNG_AND_SOLVER",
        "target_differences": target_differences,
        "rows": rows,
        "qacct": accounting,
        "evidence_sha256": evidence,
    }
    for gate in (
        "faster_than_matlab_gate",
        "within_five_percent_private_winner_gate",
        "peak_rss_no_greater_than_matlab_gate",
    ):
        require(bool(summary[gate]), f"promotion gate failed: {gate}")
    args.output.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")
    print(
        "VCKSS_FULL_CMG_PRODUCTION_CZ18_SCC_VALIDATION_PASS "
        f"production_over_matlab={production_over_matlab:.6f} "
        f"production_over_private={production_over_private:.6f}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

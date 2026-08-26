#!/usr/bin/env python3
"""Validate the six-round fixed-CZ18 P200 A/C/MATLAB SCC matrix."""

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


RETAINED_SHA = "1748ca2a6a46f248e05c0329407e7e7708ec7628c1ffce5f0e06ee264bdf0575"
BASELINE_COMMIT = "4124b34f3ca216dcc3aae27e4b31bbac9e011f11"
CMG_COMMIT = "dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10"
ROUNDS = (
    ("00-cold", "cold", ("baseline", "candidate", "matlab")),
    ("01-warm", "warm", ("candidate", "matlab", "baseline")),
    ("02-warm", "warm", ("matlab", "baseline", "candidate")),
    ("03-warm", "warm", ("baseline", "matlab", "candidate")),
    ("04-warm", "warm", ("candidate", "baseline", "matlab")),
    ("05-warm", "warm", ("matlab", "candidate", "baseline")),
)
STATA_PHASES = (
    "import_seconds",
    "command_seconds",
    "graph_seconds",
    "compression_seconds",
    "setup_seconds",
    "work_seconds",
    "fit_seconds",
    "leverage_seconds",
    "target_seconds",
    "correction_seconds",
    "rng_seconds",
    "schur_seconds",
    "pcg_seconds",
    "ingest_seconds",
    "canonical_seconds",
    "native_graph_seconds",
    "native_compression_seconds",
    "native_plan_seconds",
    "native_stayer_seconds",
    "native_solve_seconds",
    "native_total_seconds",
)
MATLAB_PHASES = (
    "input_validation_seconds",
    "pool_startup_seconds",
    "mex_setup_seconds",
    "command_seconds",
    "pool_teardown_seconds",
)


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


def finite(row: dict[str, Any], name: str) -> float:
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


def common_probe_gate(
    left: dict[str, str], right: dict[str, str], target: int
) -> dict[str, float | int | bool]:
    left_value = finite(left, f"corrected{target}")
    right_value = finite(right, f"corrected{target}")
    left_mcse = finite(left, f"mcse{target}")
    right_mcse = finite(right, f"mcse{target}")
    require(left_mcse >= 0 and right_mcse >= 0, "negative numerical MCSE")
    scale = max(1.0, abs(left_value), abs(right_value))
    limit = max(1e-8 * scale, 0.1 * max(left_mcse, right_mcse))
    difference = abs(left_value - right_value)
    return {
        "target": target,
        "difference": difference,
        "limit": limit,
        "ratio": difference / limit if limit else 0.0,
        "pass": difference <= limit,
    }


def phase_medians(rows: list[dict[str, Any]], fields: tuple[str, ...]) -> dict[str, float]:
    return {
        field: statistics.median(finite(row, field) for row in rows)
        for field in fields
    }


def expected_order_rows() -> list[dict[str, str]]:
    result: list[dict[str, str]] = []
    for round_name, temperature, order in ROUNDS:
        for position, role in enumerate(order, start=1):
            result.append(
                {
                    "round": round_name,
                    "temperature": temperature,
                    "position": str(position),
                    "role": role,
                }
            )
    return result


def read_order(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as handle:
        return list(csv.DictReader(handle, delimiter="\t"))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--run-dir", type=Path, required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--job-id", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(
        re.fullmatch(r"[0-9a-f]{40}", args.source_commit) is not None,
        "invalid source commit",
    )
    require(re.fullmatch(r"[0-9]+", args.job_id) is not None, "invalid job ID")

    task_path = args.run_dir / "receipts/task.txt"
    node_path = args.run_dir / "receipts/node.txt"
    order_path = args.run_dir / "receipts/order.tsv"
    task = key_values(task_path)
    node = key_values(node_path)
    require(
        task["schema"] == "VCKSS_FULL_CMG_CZ18_MATRIX_TASK_V1",
        "task schema changed",
    )
    require(
        node["schema"] == "VCKSS_FULL_CMG_CZ18_SCC_MATRIX_V1"
        and node["status"] == "PASS",
        "node receipt failed",
    )
    require(
        task["source_commit"] == node["vckss_commit"] == args.source_commit,
        "source binding changed",
    )
    require(
        task["baseline_commit"] == node["baseline_commit"] == BASELINE_COMMIT,
        "baseline binding changed",
    )
    require(
        task["cmg_commit"] == node["cmg_commit"] == CMG_COMMIT,
        "CMG binding changed",
    )
    require(
        task["input_sha256"] == node["input_sha256"] == RETAINED_SHA,
        "restricted input binding changed",
    )
    require(task["probes"] == node["probes"] == "200", "probe count changed")
    require(task["seed"] == "8675309", "seed changed")
    for field, expected in (
        ("rows", "8201888"),
        ("workers", "117529"),
        ("firms", "10603"),
        ("cells", "311730"),
    ):
        require(task[field] == expected, f"task dimension changed: {field}")
    require(
        task["application_threads"] == node["application_threads"] == "4"
        and node["matlab_pool_workers"] == "4",
        "application thread contract changed",
    )
    require(
        task["requested_slots"] == node["requested_slots"] == "14",
        "reservation contract changed",
    )
    require(
        task["candidate_probe_inner_tolerance"]
        == node["candidate_probe_inner_tolerance"]
        == "1e-9",
        "candidate inner tolerance changed",
    )
    require(
        task["candidate_fast_preparation"]
        == node["candidate_fast_preparation"]
        == "1"
        and task["candidate_raw_match"] == node["candidate_raw_match"] == "1",
        "candidate private route changed",
    )
    for field, expected in (
        ("rounds", "6"),
        ("cold_rounds", "1"),
        ("warm_rounds", "5"),
        ("order_schema", "POSITION_BALANCED_V1"),
    ):
        require(task[field] == node[field] == expected, f"matrix field changed: {field}")
    require(read_order(order_path) == expected_order_rows(), "matrix order changed")
    require(sha256(order_path) == node["order_sha256"], "order hash changed")
    require(
        sha256(task_path)
        == node["task_sha256"]
        == (args.run_dir / "receipts/task.sha256").read_text().strip(),
        "task hash changed",
    )

    source = json.loads(
        (args.run_dir / "artifacts/preparation/matlab_source_identity.json").read_text(
            encoding="utf-8"
        )
    )
    require(source["status"] == "PASS", "MATLAB source identity failed")

    all_rows: dict[str, list[dict[str, Any]]] = {
        "baseline": [],
        "candidate": [],
        "matlab": [],
    }
    warm_rows: dict[str, list[dict[str, Any]]] = {
        "baseline": [],
        "candidate": [],
        "matlab": [],
    }
    rss: dict[str, list[int]] = {"baseline": [], "candidate": [], "matlab": []}
    corrected_gates: list[dict[str, Any]] = []
    repeatability_gates: list[dict[str, Any]] = []
    reference_rows: dict[str, dict[str, str]] = {}
    evidence_hashes: dict[str, str] = {
        "task": sha256(task_path),
        "node": sha256(node_path),
        "order": sha256(order_path),
        "matlab_source_identity": sha256(
            args.run_dir / "artifacts/preparation/matlab_source_identity.json"
        ),
    }

    for round_name, temperature, order in ROUNDS:
        round_root = args.run_dir / "artifacts/runs" / round_name
        stata_rows: dict[str, dict[str, str]] = {}
        for role, commit in (
            ("baseline", BASELINE_COMMIT),
            ("candidate", args.source_commit),
        ):
            role_root = round_root / role
            row = one_csv(role_root / "stata.csv")
            stata_rows[role] = row
            require(
                row["schema"] == "VCKSS-FULL-CMG-CZ18-STATA-V1"
                and row["role"] == role
                and row["application_status"] == "PASS",
                f"{round_name} {role} application receipt failed",
            )
            require(
                row["source_commit"] == commit
                and row["task_sha256"] == node["task_sha256"]
                and row["input_sha256"] == RETAINED_SHA,
                f"{round_name} {role} identity changed",
            )
            for field, expected in (
                ("rows", 8201888),
                ("workers", 117529),
                ("firms", 10603),
                ("coefficient_cells", 311730),
                ("probes", 200),
                ("processors", 4),
                ("sample_count", 8201888),
            ):
                require(
                    int(finite(row, field)) == expected,
                    f"{round_name} {role} dimension changed: {field}",
                )
            for field in ("data_restored", "rng_restored", "sort_rng_restored"):
                require(
                    int(finite(row, field)) == 1,
                    f"{round_name} {role} failed {field}",
                )
            require(finite(row, "command_seconds") > 0, "invalid Stata timing")
            require(
                finite(row, "max_complete_residual")
                <= finite(row, "residual_acceptance"),
                f"{round_name} {role} complete residual failed",
            )
            require(
                abs(finite(row, "target_identity_residual")) <= 1e-12,
                f"{round_name} {role} target identity failed",
            )
            for field in STATA_PHASES:
                require(finite(row, field) >= 0, f"negative {field}")
            application_path = role_root / "application.txt"
            application = application_path.read_text(encoding="utf-8")
            if role == "candidate":
                fast = re.findall(
                    r"CMG_FULL_SPIKE_V1 SETUP .*?fast_preparation=([01])",
                    application,
                )
                raw = re.findall(
                    r"CMG_FULL_SPIKE_V1 SETUP .*?raw_match=([01])", application
                )
                require(
                    fast == ["1"] and raw == ["1"],
                    f"{round_name} candidate private-route diagnostic changed",
                )
            else:
                require(
                    "CMG_FULL_SPIKE_V1 SETUP" not in application,
                    f"{round_name} baseline entered private route",
                )
            if role not in reference_rows:
                reference_rows[role] = row
            else:
                gates = [
                    common_probe_gate(reference_rows[role], row, target)
                    for target in range(1, 5)
                ]
                repeatability_gates.append(
                    {"round": round_name, "role": role, "targets": gates}
                )
                require(
                    all(bool(gate["pass"]) for gate in gates),
                    f"{round_name} {role} repeatability failed",
                )
            all_rows[role].append(row)
            if temperature == "warm":
                warm_rows[role].append(row)
            rss[role].append(max_rss_kib(role_root / "resources.txt"))
            evidence_hashes[f"{round_name}_{role}_csv"] = sha256(
                role_root / "stata.csv"
            )
            evidence_hashes[f"{round_name}_{role}_application"] = sha256(
                application_path
            )

        gates = [
            common_probe_gate(stata_rows["baseline"], stata_rows["candidate"], target)
            for target in range(1, 5)
        ]
        corrected_gates.append({"round": round_name, "targets": gates})
        require(
            all(bool(gate["pass"]) for gate in gates),
            f"{round_name} common-probe corrected-target gate failed",
        )

        matlab_root = round_root / "matlab"
        matlab = json.loads(
            (matlab_root / "aggregate.json").read_text(encoding="utf-8")
        )
        process_tree = json.loads(
            (matlab_root / "process_tree.json").read_text(encoding="utf-8")
        )
        expected_experiment = f"cz18_p200_matrix_{round_name}_matlab"
        require(
            matlab["schema"] == "CMG-MATA-1-CZ18-MATLAB-AGGREGATE-V1"
            and matlab["status"] == "PASS"
            and matlab["experiment_id"] == expected_experiment,
            f"{round_name} MATLAB application failed",
        )
        require(
            matlab["comparison_source_commit"] == args.source_commit
            and matlab["task_sha256"] == node["task_sha256"]
            and matlab["input_sha256"] == RETAINED_SHA
            and matlab["prepared_input_sha256"] == node["prepared_input_sha256"],
            f"{round_name} MATLAB identity changed",
        )
        for field, expected in (
            ("stored_rows", 8201888),
            ("workers", 117529),
            ("firms", 10603),
            ("coefficient_cells", 311730),
            ("probes", 200),
            ("pool_workers", 4),
        ):
            require(int(finite(matlab, field)) == expected, f"MATLAB {field} changed")
        require(
            finite(matlab, "command_seconds") > 0
            and finite(matlab, "target_identity_scaled_error") <= 1e-12,
            f"{round_name} MATLAB timing or identity failed",
        )
        for field in MATLAB_PHASES:
            require(finite(matlab, field) >= 0, f"negative MATLAB {field}")
        require(
            process_tree["status"] == "PASS"
            and process_tree["all_named_pids_observed_as_descendants"] is True,
            f"{round_name} MATLAB process-tree gate failed",
        )
        all_rows["matlab"].append(matlab)
        if temperature == "warm":
            warm_rows["matlab"].append(matlab)
        rss["matlab"].append(max_rss_kib(matlab_root / "resources.txt"))
        evidence_hashes[f"{round_name}_matlab_aggregate"] = sha256(
            matlab_root / "aggregate.json"
        )
        evidence_hashes[f"{round_name}_matlab_process_tree"] = sha256(
            matlab_root / "process_tree.json"
        )

        positions = {role: position for position, role in enumerate(order, start=1)}
        for role in ("baseline", "candidate", "matlab"):
            all_rows[role][-1]["_matrix_round"] = round_name
            all_rows[role][-1]["_matrix_temperature"] = temperature
            all_rows[role][-1]["_matrix_position"] = positions[role]

    require(
        all(len(rows) == 6 for rows in all_rows.values())
        and all(len(rows) == 5 for rows in warm_rows.values()),
        "incomplete matrix",
    )

    accounting_path = args.run_dir / "qacct" / f"{args.job_id}.txt"
    accounting = qacct(accounting_path)
    require(
        accounting["jobnumber"] == node["job_id"] == args.job_id,
        "scheduler job identity changed",
    )
    require(
        accounting["taskid"] == "undefined"
        and accounting["project"] == "welfgr"
        and accounting["failed"] == accounting["exit_status"] == "0",
        "scheduler completion failed",
    )
    require(
        int(accounting["slots"]) == int(node["actual_slots"]) == 14,
        "scheduler slots changed",
    )
    require(
        (args.run_dir / "receipts/wrapper.pass").is_file()
        and not (args.run_dir / "receipts/wrapper.fail").exists(),
        "wrapper marker failed",
    )

    baseline_phases = phase_medians(warm_rows["baseline"], STATA_PHASES)
    candidate_phases = phase_medians(warm_rows["candidate"], STATA_PHASES)
    matlab_phases = phase_medians(warm_rows["matlab"], MATLAB_PHASES)
    baseline_seconds = baseline_phases["command_seconds"]
    candidate_seconds = candidate_phases["command_seconds"]
    matlab_seconds = matlab_phases["command_seconds"]
    candidate_over_matlab = candidate_seconds / matlab_seconds
    candidate_over_baseline = candidate_seconds / baseline_seconds
    rss_summary = {
        role: {
            "median_kib": statistics.median(values[1:]),
            "maximum_kib": max(values),
            "all_rounds_kib": values,
        }
        for role, values in rss.items()
    }
    evidence_hashes["qacct"] = sha256(accounting_path)
    repeated_rhs_count = 1 + 3 * 200
    two_x_pass = candidate_over_matlab <= 0.5
    peak_memory_pass = (
        rss_summary["candidate"]["maximum_kib"]
        <= rss_summary["matlab"]["maximum_kib"]
    )

    payload = {
        "schema": "VCKSS_FULL_CMG_CZ18_SCC_MATRIX_VALIDATION_V1",
        "status": "PASS",
        "promotion_status": (
            "CZ18_P200_WARM_MEDIAN_TWO_X_CHECKPOINT_ONLY"
            if two_x_pass and peak_memory_pass
            else "CZ18_P200_WARM_MEDIAN_NOT_PROMOTED"
        ),
        "source_commit": args.source_commit,
        "baseline_commit": BASELINE_COMMIT,
        "cmg_commit": CMG_COMMIT,
        "job_id": args.job_id,
        "host": accounting["hostname"],
        "task_sha256": sha256(task_path),
        "input_sha256": RETAINED_SHA,
        "probes": 200,
        "candidate_probe_inner_tolerance": 1e-9,
        "candidate_fast_preparation": True,
        "candidate_raw_match": True,
        "rounds": [
            {
                "name": round_name,
                "temperature": temperature,
                "order": list(order),
            }
            for round_name, temperature, order in ROUNDS
        ],
        "warm_median_seconds": {
            "baseline": baseline_seconds,
            "candidate": candidate_seconds,
            "matlab": matlab_seconds,
        },
        "warm_median_phases_seconds": {
            "baseline": baseline_phases,
            "candidate": candidate_phases,
            "matlab": matlab_phases,
        },
        "warm_median_ratios": {
            "candidate_over_baseline": candidate_over_baseline,
            "candidate_over_matlab": candidate_over_matlab,
            "baseline_speedup": baseline_seconds / candidate_seconds,
            "matlab_speedup": matlab_seconds / candidate_seconds,
        },
        "repeated_rhs": {
            "count": repeated_rhs_count,
            "candidate_native_solve_median_seconds": candidate_phases[
                "native_solve_seconds"
            ],
            "candidate_native_solve_seconds_per_rhs": candidate_phases[
                "native_solve_seconds"
            ]
            / repeated_rhs_count,
            "matlab_internal_solve_timing": "NOT_EXPOSED_BY_MAINTAINED_COMPARATOR",
        },
        "gates": {
            "all_application_and_state_gates_pass": True,
            "all_common_probe_corrected_target_gates_pass": True,
            "all_complete_original_system_residual_gates_pass": True,
            "all_matlab_process_tree_gates_pass": True,
            "candidate_no_slower_than_half_matlab_warm_median": two_x_pass,
            "candidate_peak_rss_no_greater_than_matlab": peak_memory_pass,
            "full_alpha_promotion": False,
        },
        "common_probe_corrected_target_gates": corrected_gates,
        "within_role_repeatability_gates": repeatability_gates,
        "matlab_corrected_target_gate": (
            "DESCRIPTIVE_REPEATED_SAME_SEED_NO_REGISTERED_DISTRIBUTION"
        ),
        "max_rss": rss_summary,
        "qacct": {
            "wall_seconds": float(accounting["ru_wallclock"]),
            "cpu": accounting["cpu"],
            "maxvmem": accounting["maxvmem"],
        },
        "evidence_sha256": evidence_hashes,
    }
    require(not args.output.exists(), "validation output already exists")
    args.output.write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"VCKSS_FULL_CMG_CZ18_SCC_MATRIX_VALIDATION_PASS {args.job_id}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

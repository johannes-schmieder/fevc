#!/usr/bin/env python3
"""Validate the six-round 8,192-firm P200 A/C/MATLAB SCC matrix."""

from __future__ import annotations

import argparse
import json
import math
import re
import statistics
from pathlib import Path
from typing import Any

from validate_scc_cz18_matrix import (
    BASELINE_COMMIT,
    CMG_COMMIT,
    MATLAB_PHASES,
    ROUNDS,
    STATA_PHASES,
    common_probe_gate,
    expected_order_rows,
    finite,
    key_values,
    max_rss_kib,
    one_csv,
    phase_medians,
    qacct,
    read_order,
    require,
    sha256,
)

ROWS = 1_966_080
WORKERS = 327_680
FIRMS = 8_192
DEGREE = 6
PROBES = 200
SEED = 2_026_082_501
THREADS = 4
RHS_COUNT = 1 + 3 * PROBES
SYNTHETIC_MATLAB_PHASES = ("import_seconds", *MATLAB_PHASES)


def parse_diagnostics(path: Path) -> dict[str, Any]:
    setup: dict[str, str] | None = None
    batches: list[dict[str, str]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if "CMG_FULL_SPIKE_V1" not in line:
            continue
        payload = line[line.index("CMG_FULL_SPIKE_V1") :].split()
        require(len(payload) >= 3, f"malformed full-CMG diagnostic: {line}")
        values = dict(token.split("=", 1) for token in payload[2:] if "=" in token)
        if payload[1] == "SETUP":
            require(setup is None, "candidate emitted multiple setup receipts")
            setup = values
        elif payload[1] == "BATCH":
            batches.append(values)
    require(setup is not None and batches, "candidate full-CMG diagnostics missing")
    integer_fields = (
        "rhs",
        "concurrency",
        "rhs_ns",
        "solve_ns",
        "extraction_ns",
        "max_iterations",
        "total_iterations",
        "total_operator_applications",
        "total_preconditioner_applications",
    )
    require(
        all(all(field in batch for field in integer_fields) for batch in batches),
        "incomplete full-CMG batch receipt",
    )
    rhs_count = sum(int(batch["rhs"]) for batch in batches)
    require(rhs_count == RHS_COUNT, f"expected {RHS_COUNT} RHS, found {rhs_count}")
    require(
        int(setup["threads"]) == THREADS
        and int(setup["fast_preparation"]) == 1
        and int(setup["raw_match"]) == 1,
        "candidate setup route changed",
    )
    require(
        max(int(batch["concurrency"]) for batch in batches) <= THREADS,
        "candidate concurrency exceeds requested threads",
    )
    result = {
        "batch_count": len(batches),
        "rhs_count": rhs_count,
        "rhs_seconds": sum(int(batch["rhs_ns"]) for batch in batches) / 1e9,
        "solve_seconds": sum(int(batch["solve_ns"]) for batch in batches) / 1e9,
        "extraction_seconds": sum(
            int(batch["extraction_ns"]) for batch in batches
        )
        / 1e9,
        "maximum_iterations": max(int(batch["max_iterations"]) for batch in batches),
        "total_iterations": sum(int(batch["total_iterations"]) for batch in batches),
        "total_operator_applications": sum(
            int(batch["total_operator_applications"]) for batch in batches
        ),
        "total_preconditioner_applications": sum(
            int(batch["total_preconditioner_applications"]) for batch in batches
        ),
        "maximum_reduced_residual": max(
            float(batch["max_reduced_residual"]) for batch in batches
        ),
        "maximum_complete_residual": max(
            float(batch["max_complete_residual"]) for batch in batches
        ),
        "maximum_concurrency": max(int(batch["concurrency"]) for batch in batches),
        "executions": sorted({batch["execution"] for batch in batches}),
    }
    require(
        all(
            math.isfinite(float(result[field])) and float(result[field]) >= 0
            for field in (
                "rhs_seconds",
                "solve_seconds",
                "extraction_seconds",
                "maximum_reduced_residual",
                "maximum_complete_residual",
            )
        ),
        "nonfinite full-CMG diagnostic",
    )
    return result


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
        task["schema"] == "VCKSS_FULL_CMG_SYNTHETIC_MATRIX_TASK_V1",
        "task schema changed",
    )
    require(
        node["schema"] == "VCKSS_FULL_CMG_SYNTHETIC_SCC_MATRIX_V1"
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
    for field, expected in (
        ("structure", "strong_d6"),
        ("connectivity", "strong"),
        ("rows", str(ROWS)),
        ("workers", str(WORKERS)),
        ("firms", str(FIRMS)),
        ("degree", str(DEGREE)),
        ("probes", str(PROBES)),
        ("seed", str(SEED)),
        ("application_threads", str(THREADS)),
        ("requested_slots", "14"),
        ("candidate_fast_preparation", "1"),
        ("candidate_raw_match", "1"),
        ("rounds", "6"),
        ("cold_rounds", "1"),
        ("warm_rounds", "5"),
        ("order_schema", "POSITION_BALANCED_V1"),
    ):
        require(task[field] == node[field] == expected, f"matrix field changed: {field}")
    require(node["matlab_pool_workers"] == "4", "MATLAB pool size changed")
    require(read_order(order_path) == expected_order_rows(), "matrix order changed")
    require(sha256(order_path) == node["order_sha256"], "order hash changed")
    require(
        sha256(task_path)
        == node["task_sha256"]
        == (args.run_dir / "receipts/task.sha256").read_text().strip(),
        "task hash changed",
    )

    input_receipt = one_csv(
        args.run_dir / "artifacts/preparation/input_receipt.csv"
    )
    require(
        input_receipt["schema"] == "PAPER-MATLAB-SCALING-INPUT-V1"
        and input_receipt["structure"] == "strong_d6"
        and input_receipt["connectivity"] == "strong",
        "synthetic input receipt failed",
    )
    for field, expected in (
        ("rows", ROWS),
        ("workers", WORKERS),
        ("firms", FIRMS),
        ("cells_per_worker", DEGREE),
        ("coefficient_cells", ROWS),
    ):
        require(int(finite(input_receipt, field)) == expected, f"input {field} changed")
    source_path = args.run_dir / "artifacts/preparation/matlab_source_identity.json"
    source = json.loads(source_path.read_text(encoding="utf-8"))
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
    diagnostics: list[dict[str, Any]] = []
    corrected_gates: list[dict[str, Any]] = []
    repeatability_gates: list[dict[str, Any]] = []
    reference_rows: dict[str, dict[str, str]] = {}
    evidence_hashes: dict[str, str] = {
        "validator": sha256(Path(__file__)),
        "task": sha256(task_path),
        "node": sha256(node_path),
        "order": sha256(order_path),
        "input_receipt": sha256(
            args.run_dir / "artifacts/preparation/input_receipt.csv"
        ),
        "matlab_source_identity": sha256(source_path),
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
                row["schema"] == "VCKSS-FULL-CMG-SPIKE-STATA-V1"
                and row["role"] == role
                and row["application_status"] == "PASS",
                f"{round_name} {role} application receipt failed",
            )
            require(
                row["source_commit"] == commit
                and row["task_sha256"] == node["task_sha256"]
                and row["input_sha256"] == node["input_sha256"],
                f"{round_name} {role} identity changed",
            )
            require(row["structure"] == "strong_d6", "Stata structure changed")
            for field, expected in (
                ("rows", ROWS),
                ("workers", WORKERS),
                ("firms", FIRMS),
                ("cells_per_worker", DEGREE),
                ("probes", PROBES),
                ("seed", SEED),
                ("processors", THREADS),
                ("sample_count", ROWS),
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
            if role == "candidate":
                receipt = parse_diagnostics(application_path)
                require(
                    receipt["maximum_complete_residual"]
                    <= finite(row, "residual_acceptance"),
                    f"{round_name} diagnostic residual failed",
                )
                diagnostics.append(receipt)
            else:
                require(
                    "CMG_FULL_SPIKE_V1 SETUP"
                    not in application_path.read_text(encoding="utf-8"),
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
        matlab_path = matlab_root / "matlab_aggregate.json"
        matlab = json.loads(matlab_path.read_text(encoding="utf-8"))
        process_tree = json.loads(
            (matlab_root / "process_tree.json").read_text(encoding="utf-8")
        )
        expected_experiment = f"full_cmg_synthetic_matrix_{round_name}_matlab"
        require(
            matlab["schema"] == "PAPER-MATLAB-SCALING-MATLAB-V1"
            and matlab["status"] == "PASS"
            and matlab["experiment_id"] == expected_experiment,
            f"{round_name} MATLAB application failed",
        )
        require(
            matlab["source_commit"] == args.source_commit
            and matlab["bundle_sha256"] == node["candidate_archive_sha256"]
            and matlab["task_sha256"] == node["task_sha256"]
            and matlab["input_sha256"] == node["input_sha256"],
            f"{round_name} MATLAB identity changed",
        )
        require(
            matlab["structure"] == "strong_d6"
            and matlab["connectivity"] == "strong",
            "MATLAB structure changed",
        )
        for field, expected in (
            ("rows", ROWS),
            ("workers", WORKERS),
            ("firms", FIRMS),
            ("cells_per_worker", DEGREE),
            ("probes", PROBES),
            ("seed", SEED),
            ("pool_workers", THREADS),
            ("detail_matches", ROWS),
        ):
            require(int(finite(matlab, field)) == expected, f"MATLAB {field} changed")
        require(
            finite(matlab, "command_seconds") > 0
            and finite(matlab, "target_identity_scaled_error") <= 1e-12,
            f"{round_name} MATLAB timing or identity failed",
        )
        for field in SYNTHETIC_MATLAB_PHASES:
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
        evidence_hashes[f"{round_name}_matlab_aggregate"] = sha256(matlab_path)
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
        and all(len(rows) == 5 for rows in warm_rows.values())
        and len(diagnostics) == 6,
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
    matlab_phases = phase_medians(
        warm_rows["matlab"], SYNTHETIC_MATLAB_PHASES
    )
    diagnostic_fields = (
        "rhs_seconds",
        "solve_seconds",
        "extraction_seconds",
        "maximum_iterations",
        "total_iterations",
        "total_operator_applications",
        "total_preconditioner_applications",
        "maximum_reduced_residual",
        "maximum_complete_residual",
        "maximum_concurrency",
    )
    diagnostic_medians = {
        field: statistics.median(float(row[field]) for row in diagnostics[1:])
        for field in diagnostic_fields
    }
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
    two_x_pass = candidate_over_matlab <= 0.5
    peak_memory_pass = (
        rss_summary["candidate"]["maximum_kib"]
        <= rss_summary["matlab"]["maximum_kib"]
    )

    payload = {
        "schema": "VCKSS_FULL_CMG_SYNTHETIC_SCC_MATRIX_VALIDATION_V1",
        "status": "PASS",
        "promotion_status": (
            "SYNTHETIC_P200_WARM_MEDIAN_TWO_X_CHECKPOINT_ONLY"
            if two_x_pass and peak_memory_pass
            else "SYNTHETIC_P200_WARM_MEDIAN_NOT_PROMOTED"
        ),
        "source_commit": args.source_commit,
        "baseline_commit": BASELINE_COMMIT,
        "cmg_commit": CMG_COMMIT,
        "job_id": args.job_id,
        "host": accounting["hostname"],
        "task_sha256": sha256(task_path),
        "input_sha256": node["input_sha256"],
        "case": {
            "structure": "strong_d6",
            "rows": ROWS,
            "workers": WORKERS,
            "firms": FIRMS,
            "degree": DEGREE,
            "probes": PROBES,
            "seed": SEED,
            "application_threads": THREADS,
            "candidate_fast_preparation": True,
            "candidate_raw_match": True,
        },
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
        "candidate_full_cmg_warm_medians": diagnostic_medians,
        "warm_median_ratios": {
            "candidate_over_baseline": candidate_over_baseline,
            "candidate_over_matlab": candidate_over_matlab,
            "baseline_speedup": baseline_seconds / candidate_seconds,
            "matlab_speedup": matlab_seconds / candidate_seconds,
        },
        "repeated_rhs": {
            "count": RHS_COUNT,
            "candidate_full_cmg_solve_median_seconds": diagnostic_medians[
                "solve_seconds"
            ],
            "candidate_full_cmg_solve_seconds_per_rhs": diagnostic_medians[
                "solve_seconds"
            ]
            / RHS_COUNT,
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
    print(f"VCKSS_FULL_CMG_SYNTHETIC_SCC_MATRIX_VALIDATION_PASS {args.job_id}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

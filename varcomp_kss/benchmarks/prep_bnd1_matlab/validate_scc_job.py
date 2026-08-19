#!/usr/bin/env python3
"""Validate one same-host PREP-BND-1 Stata/MATLAB comparison job."""

from __future__ import annotations

import argparse
import csv
import json
import math
import re
from pathlib import Path

from common import (
    TARGETS,
    finite,
    integer,
    load_json,
    one_csv,
    parse_matlab_pcg,
    qacct,
    read_task,
    require,
    same_host,
    scaled_target_gap,
    sha256,
)

GIB = 1024**3


def key_values(path: Path) -> dict[str, str]:
    require(path.is_file() and not path.is_symlink(), f"invalid TSV: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.reader(handle, delimiter="\t"))
    require(rows and rows[0] == ["key", "value"], "invalid TSV header")
    require(all(len(row) == 2 for row in rows[1:]), "invalid TSV row")
    result = {row[0]: row[1] for row in rows[1:]}
    require(len(result) == len(rows) - 1, "duplicate TSV key")
    return result


def max_rss(path: Path) -> int:
    require(path.is_file(), f"missing resource receipt: {path}")
    match = re.search(
        r"Maximum resident set size \(kbytes\):\s*([0-9]+)",
        path.read_text(encoding="utf-8", errors="replace"),
    )
    require(match is not None and int(match.group(1)) > 0,
            f"missing maximum RSS: {path}")
    return int(match.group(1)) * 1024


def parse_memory(value: str) -> int:
    units = {"": 1, "K": 1024, "M": 1024**2, "G": GIB, "T": 1024**4}
    text = value.strip().upper()
    suffix = text[-1] if text and text[-1] in units else ""
    number = text[:-1] if suffix else text
    result = finite(number, "qacct maxvmem") * units[suffix]
    require(result >= 0, "negative qacct maxvmem")
    return math.ceil(result)


def validate(
    job_dir: Path,
    qacct_path: Path,
    expected_source_commit: str,
    expected_bundle: str,
) -> dict[str, object]:
    task_path = job_dir / "task.tsv"
    task = read_task(task_path)
    task_sha = sha256(task_path)
    require((job_dir / "task.sha256").read_text().strip() == task_sha,
            "task hash changed")
    require(task["source_commit"] == expected_source_commit,
            "source commit changed")
    require(task["bundle_sha256"] == expected_bundle, "bundle changed")
    require((job_dir / "source_manifest.check").is_file() and
            (job_dir / "source_manifest.check").stat().st_size > 0,
            "source-manifest check is missing")

    input_receipt = one_csv(job_dir / "input_receipt.csv")
    input_sha = (job_dir / "input.sha256").read_text().strip()
    require(re.fullmatch(r"[0-9a-f]{64}", input_sha) is not None,
            "invalid input hash")
    for field in ("rows", "workers", "firms", "cells_per_worker"):
        require(integer(input_receipt[field], f"input {field}") ==
                integer(task[field], f"task {field}"),
                f"input dimensions changed: {field}")
    require(input_receipt["schema"] == "PREP-BND-1-INPUT-V1" and
            input_receipt["sample_contract"] == task["sample_contract"] and
            input_receipt["target_contract"] == task["target_contract"],
            "input contract changed")

    stata = one_csv(job_dir / "stata.csv")
    require(stata["schema"] == "PREP-BND-1-STATA-AGGREGATE-V1" and
            stata["application_status"] == "PASS", "Stata application failed")
    require(stata["source_commit"] == expected_source_commit and
            stata["task_sha256"] == task_sha and
            stata["input_sha256"] == input_sha, "Stata source/input changed")
    for field in ("rows", "workers", "firms", "cells_per_worker", "probes", "seed"):
        require(integer(stata[field], f"Stata {field}") ==
                integer(task[field], f"task {field}"),
                f"Stata task changed: {field}")
    require(stata["processors"] == "4" and
            stata["sample_contract"] == task["sample_contract"] and
            stata["target_contract"] == task["target_contract"],
            "Stata processor or sample contract changed")
    require(stata["engine"] == "compressed" and
            stata["preconditioner"] in {"DIAGONAL", "CMG"},
            "Stata route changed")
    require(finite(stata["command_seconds"], "Stata command") > 0 and
            finite(stata["max_complete_residual"], "Stata residual") <=
            finite(stata["residual_acceptance"], "Stata acceptance") and
            finite(stata["target_identity_residual"], "Stata identity") <= 1e-12,
            "Stata timing or numerical gate failed")
    require(stata["data_restored"] == stata["rng_restored"] ==
            stata["sort_rng_restored"] == "1", "Stata caller state changed")
    require(finite(stata["resource_peak_bytes"], "Stata direct peak") <= 16 * GIB,
            "Stata exceeded its direct memory envelope")

    matlab = load_json(job_dir / "matlab_aggregate.json")
    source = load_json(job_dir / "matlab_source_identity.json")
    identity = load_json(job_dir / "matlab_process_identity.json")
    tree = load_json(job_dir / "matlab_process_tree_rss.json")
    require(matlab.get("schema") == "PREP-BND-1-MATLAB-AGGREGATE-V1" and
            matlab.get("status") == source.get("status") == "PASS",
            "MATLAB application or source identity failed")
    require(matlab["source_commit"] == expected_source_commit and
            matlab["bundle_sha256"] == expected_bundle and
            matlab["task_sha256"] == task_sha and
            matlab["input_sha256"] == input_sha,
            "MATLAB source/input changed")
    require(source["matlab_runtime_tree_sha256"] ==
            matlab["matlab_runtime_tree_sha256"] and
            source["matlab_core_sha256"] == matlab["matlab_core_sha256"],
            "MATLAB maintained-source identity changed")
    for field in ("rows", "workers", "firms", "cells_per_worker", "probes", "seed"):
        require(integer(matlab[field], f"MATLAB {field}") ==
                integer(task[field], f"task {field}"),
                f"MATLAB task changed: {field}")
    require(matlab["pool_workers"] == 4 and matlab["detail_matches"] ==
            integer(task["rows"], "rows"), "MATLAB pool or sample changed")
    require(matlab["same_literal_rows"] is True and
            matlab["uniform_stored_row_targets"] is True and
            matlab["corrected_estimate_equality_gate"] ==
            "NONE_DESCRIPTIVE_ONLY" and
            matlab["rng_draws_comparable"] is False and
            matlab["solver_tolerance_comparable"] is False and
            matlab["correction_formula_comparable"] is False,
            "MATLAB comparison boundary changed")
    for field in ("import_seconds", "input_validation_seconds",
                  "pool_startup_seconds", "mex_setup_seconds",
                  "command_seconds", "pool_teardown_seconds"):
        require(finite(matlab[field], f"MATLAB {field}") >= 0,
                f"invalid MATLAB phase: {field}")
    require(finite(matlab["command_seconds"], "MATLAB command") > 0 and
            finite(matlab["target_identity_scaled_error"], "MATLAB identity") <= 1e-12,
            "MATLAB timing or target identity failed")
    require(identity.get("status") == "PASS" and
            identity.get("expected_pool_workers") == 4 and
            identity.get("worker_indices") == [1, 2, 3, 4] and
            len(set(identity.get("worker_pids", []))) == 4,
            "MATLAB process identity failed")
    require(tree.get("status") == "PASS" and
            tree.get("all_named_pids_observed_as_descendants") is True and
            finite(tree["peak_rss_kib"], "MATLAB process-tree RSS") > 0,
            "MATLAB process-tree evidence failed")
    require(finite(tree["peak_rss_kib"], "MATLAB process-tree RSS") * 1024 <=
            56 * GIB, "MATLAB process tree exceeded 56 GiB")

    node = key_values(job_dir / "node_receipt.tsv")
    accounting = qacct(qacct_path)
    require(node["schema"] == "PREP-BND-1-MATLAB-NODE-V1" and
            node["experiment_id"] == task["experiment_id"] and
            node["source_commit"] == expected_source_commit and
            node["bundle_sha256"] == expected_bundle and
            node["task_sha256"] == task_sha and
            node["input_sha256"] == input_sha and
            node["order"] == task["order"], "node receipt changed")
    require(node["job_id"] == accounting["jobnumber"] and
            same_host(node.get("hostname", ""), accounting["hostname"]),
            "scheduler identity changed")
    require(node["requested_slots"] == node["actual_slots"] ==
            node["stata_processors"] == node["matlab_pool_workers"] == "4" and
            node["mem_per_core_gib"] == "14", "node resource contract changed")
    require(parse_memory(accounting["maxvmem"]) <= 56 * GIB,
            "qacct maxvmem exceeded 56 GiB")
    require((job_dir / "pair.pass").read_text().strip() ==
            f"PREP_BND1_MATLAB_PAIR_PASS {task['experiment_id']} {task_sha}" and
            (job_dir / "matlab.pass").is_file() and
            not (job_dir / "wrapper.fail").exists(), "terminal marker failed")

    pcg = parse_matlab_pcg(job_dir / "matlab.application.txt")
    stata_targets = {
        key: finite(stata[f"corrected_{key}"], f"Stata {key}") for key in TARGETS
    }
    matlab_targets = {
        key: finite(matlab[f"corrected_{key}"], f"MATLAB {key}") for key in TARGETS
    }
    gaps = {key: abs(stata_targets[key] - matlab_targets[key]) for key in TARGETS}
    payload: dict[str, object] = {
        "schema": "PREP-BND-1-MATLAB-VALIDATION-V1",
        "status": ("PASS_NUMERICAL_AND_TIMING" if pcg["converged"] else
                   "PASS_TIMING_MATLAB_NUMERICAL_REJECTED"),
        "experiment_id": task["experiment_id"],
        "job_id": accounting["jobnumber"],
        "hostname": accounting["hostname"],
        "source_commit": expected_source_commit,
        "bundle_sha256": expected_bundle,
        "task_sha256": task_sha,
        "input_sha256": input_sha,
        "firms": integer(task["firms"], "firms"),
        "workers": integer(task["workers"], "workers"),
        "rows": integer(task["rows"], "rows"),
        "probes": integer(task["probes"], "probes"),
        "seed": integer(task["seed"], "seed"),
        "order": task["order"],
        "stata_command_seconds": finite(stata["command_seconds"], "Stata command"),
        "matlab_command_seconds": finite(matlab["command_seconds"], "MATLAB command"),
        "stata_phases_seconds": {
            field.removesuffix("_seconds"): finite(stata[field], f"Stata {field}")
            for field in (
                "import_seconds", "selection_seconds", "graph_seconds",
                "compression_seconds", "setup_seconds", "work_seconds",
                "fit_seconds", "leverage_seconds", "target_seconds",
                "correction_seconds", "rng_seconds", "schur_seconds",
                "pcg_seconds",
            )
        },
        "matlab_phases_seconds": {
            field.removesuffix("_seconds"): finite(matlab[field], f"MATLAB {field}")
            for field in (
                "import_seconds", "input_validation_seconds",
                "pool_startup_seconds", "mex_setup_seconds",
                "command_seconds", "pool_teardown_seconds",
            )
        },
        "stata_over_matlab_command_ratio": finite(
            stata["command_seconds"], "Stata command") /
            finite(matlab["command_seconds"], "MATLAB command"),
        "matlab_pcg": pcg,
        "matlab_numerical_result_accepted": pcg["converged"],
        "corrected_estimate_equality_gate": "NONE_DESCRIPTIVE_ONLY",
        "stata_targets": stata_targets,
        "matlab_targets": matlab_targets,
        "absolute_target_gaps_descriptive": gaps,
        "scaled_target_gap_descriptive": scaled_target_gap(
            stata_targets, matlab_targets),
        "qacct_wall_seconds": finite(accounting["ru_wallclock"], "qacct wall"),
        "qacct_cpu_seconds": finite(accounting["cpu"], "qacct cpu"),
        "qacct_maxvmem_bytes": parse_memory(accounting["maxvmem"]),
        "stata_gnu_max_rss_bytes": max_rss(job_dir / "stata.resources.txt"),
        "matlab_gnu_max_rss_bytes": max_rss(job_dir / "matlab.resources.txt"),
        "matlab_process_tree_peak_rss_bytes": int(
            finite(tree["peak_rss_kib"], "MATLAB process tree") * 1024),
        "artifact_sha256": {
            name: sha256(job_dir / name) for name in (
                "task.tsv", "input_receipt.csv", "stata.csv",
                "matlab_aggregate.json", "matlab_source_identity.json",
                "matlab_process_identity.json", "matlab_process_tree_rss.json",
                "node_receipt.tsv", "pair.pass",
            )
        },
    }
    return payload


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--job-dir", type=Path, required=True)
    parser.add_argument("--qacct", type=Path, required=True)
    parser.add_argument("--expected-source-commit", required=True)
    parser.add_argument("--expected-bundle", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(not args.output.exists(), "validation target already exists")
    payload = validate(args.job_dir, args.qacct, args.expected_source_commit,
                       args.expected_bundle)
    args.output.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n",
                           encoding="utf-8")
    print(f"PREP_BND1_MATLAB_VALIDATION_PASS {payload['experiment_id']} "
          f"{payload['status']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

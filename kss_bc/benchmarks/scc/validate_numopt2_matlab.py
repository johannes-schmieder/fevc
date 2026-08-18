#!/usr/bin/env python3
"""Validate a source-bound MATLAB comparison against one KSS-NUMOPT-2 task."""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import re
from pathlib import Path
from typing import Any


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def load_json(path: Path) -> dict[str, Any]:
    require(path.is_file() and not path.is_symlink(), f"invalid JSON: {path}")
    value = json.loads(path.read_text(encoding="utf-8"))
    require(isinstance(value, dict), f"invalid JSON object: {path}")
    return value


def key_values(path: Path) -> dict[str, str]:
    require(path.is_file() and not path.is_symlink(), f"invalid receipt: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.reader(handle, delimiter="\t")
        require(next(reader, None) == ["key", "value"], f"invalid header: {path}")
        rows = list(reader)
    require(all(len(row) == 2 for row in rows), f"invalid row: {path}")
    result = {row[0]: row[1] for row in rows}
    require(len(result) == len(rows), f"duplicate key: {path}")
    return result


def one_csv(path: Path) -> dict[str, str]:
    require(path.is_file(), f"missing CSV: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    require(len(rows) == 1, f"expected one CSV row: {path}")
    return rows[0]


def finite(value: Any, label: str) -> float:
    result = float(value)
    require(math.isfinite(result), f"nonfinite {label}")
    return result


def qacct(path: Path) -> dict[str, str]:
    result: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        fields = line.strip().split(None, 1)
        if len(fields) == 2:
            result[fields[0]] = fields[1]
    required = {
        "jobnumber", "granted_pe", "slots", "failed", "exit_status",
        "ru_wallclock", "maxvmem", "hostname", "qname",
    }
    require(required <= result.keys(), "incomplete qacct")
    require(result["failed"] == result["exit_status"] == "0", "job failed")
    return result


def max_rss(path: Path) -> int:
    match = re.search(
        r"Maximum resident set size \(kbytes\):\s*([0-9]+)",
        path.read_text(encoding="utf-8"),
    )
    require(match is not None, "missing GNU-time RSS")
    return int(match.group(1))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--run-dir", type=Path, required=True)
    parser.add_argument("--experiment", required=True)
    parser.add_argument("--kss-experiment-dir", type=Path, required=True)
    parser.add_argument("--expected-source-commit", required=True)
    parser.add_argument("--expected-bundle", required=True)
    parser.add_argument("--expected-kss-source-commit", required=True)
    parser.add_argument("--expected-kss-bundle", required=True)
    args = parser.parse_args()
    require(re.fullmatch(r"[A-Za-z0-9._-]+", args.experiment) is not None,
            "invalid experiment")
    base = args.run_dir / "matlab_numopt2" / args.experiment
    task_path = base / "task.tsv"
    task = key_values(task_path)
    require(sha256(task_path) == (base / "task.sha256").read_text().strip(),
            "task hash changed")
    require(task["task_version"] == "KSS-NUMOPT-2-MATLAB-TASK-V1",
            "task version changed")
    require(task["experiment_id"] == args.experiment, "experiment changed")
    require(task["comparison_source_commit"] == args.expected_source_commit,
            "comparison source changed")
    require(task["comparison_bundle_sha256"] == args.expected_bundle,
            "comparison bundle changed")
    require(task["kss_source_commit"] == args.expected_kss_source_commit,
            "KSS source changed")
    require(task["kss_bundle_sha256"] == args.expected_kss_bundle,
            "KSS bundle changed")

    kss_task = key_values(args.kss_experiment_dir / "task.tsv")
    kss_validation = load_json(args.kss_experiment_dir / "validation.json")
    require(kss_validation.get("status") == "PASS", "KSS evidence did not pass")
    mapping = {
        "workers": "workers",
        "firms": "firms",
        "cells_per_worker": "cells_per_worker",
        "rows_per_cell": "rows_per_cell",
        "connectivity": "connectivity",
        "probes": "probes",
        "seed": "seed",
    }
    for matlab_field, kss_field in mapping.items():
        require(task[matlab_field] == kss_task[kss_field],
                f"KSS/MATLAB task mismatch: {matlab_field}")
    require(kss_task["source_commit"] == args.expected_kss_source_commit,
            "KSS task source changed")
    require(kss_task["bundle_sha256"] == args.expected_kss_bundle,
            "KSS task bundle changed")

    aggregate = load_json(base / "aggregate.json")
    source = load_json(base / "source_identity.json")
    identity = load_json(base / "process_identity.json")
    tree = load_json(base / "process_tree_rss.json")
    node = key_values(base / "node_receipt.tsv")
    require(aggregate.get("status") == source.get("status") == "PASS",
            "application or source identity failed")
    require(aggregate.get("schema") == "KSS-NUMOPT-2-MATLAB-AGGREGATE-V1",
            "aggregate schema changed")
    require(source.get("schema") == "KSS-NUMOPT-2-MATLAB-SOURCE-V1",
            "source schema changed")
    require(aggregate["experiment_id"] == args.experiment, "aggregate changed")
    require(aggregate["task_sha256"] == sha256(task_path), "aggregate task changed")
    require(aggregate["comparison_source_commit"] == args.expected_source_commit,
            "aggregate source changed")
    require(aggregate["comparison_bundle_sha256"] == args.expected_bundle,
            "aggregate bundle changed")
    require(aggregate["kss_source_commit"] == args.expected_kss_source_commit,
            "aggregate KSS source changed")
    require(aggregate["kss_bundle_sha256"] == args.expected_kss_bundle,
            "aggregate KSS bundle changed")
    require(finite(aggregate["pool_workers"], "pool workers") == 4,
            "MATLAB pool changed")
    require(finite(aggregate["probes"], "probes") == int(task["probes"]),
            "probe count changed")
    require(finite(aggregate["command_seconds"], "command time") > 0,
            "command time missing")
    cells = int(task["workers"]) * int(task["cells_per_worker"])
    rows = cells * int(task["rows_per_cell"])
    require(finite(aggregate["coefficient_cells"], "cells") == cells,
            "cell count changed")
    require(finite(aggregate["stored_rows"], "rows") == rows,
            "row count changed")
    require(finite(aggregate["detail_matches"], "detail matches") == cells,
            "retained match count changed")
    require(finite(aggregate["target_identity_scaled_error"], "identity") <= 1e-12,
            "target identity failed")
    require(aggregate["corrected_estimate_equality_gate"] ==
            "NONE_DESCRIPTIVE_ONLY", "cross-language equality gate changed")
    require(aggregate["target_weight_semantics_comparable"] is False,
            "unsupported target weights claimed comparable")
    expected_frequency = int(task["rows_per_cell"]) == 8
    require(aggregate["frequency_semantics_comparable"] is expected_frequency,
            "frequency comparability changed")

    require(identity.get("schema") == "kss_matlab_scale_process_identity_v1",
            "process identity schema changed")
    require(identity.get("status") == "PASS" and identity.get("mode") == "cold",
            "process identity failed")
    require(identity.get("case_sha256") == sha256(task_path),
            "process identity task changed")
    require(identity.get("expected_pool_workers") == 4,
            "process identity pool changed")
    require(identity.get("worker_indices") == [1, 2, 3, 4],
            "worker indices changed")
    pids = identity.get("worker_pids")
    require(isinstance(pids, list) and len(set(pids)) == 4,
            "worker PID identities changed")
    require(tree.get("status") == "PASS" and
            tree.get("all_named_pids_observed_as_descendants") is True,
            "process-tree RSS monitor failed")
    require(finite(tree["peak_rss_kib"], "process-tree RSS") > 0,
            "process-tree RSS missing")
    require(finite(tree["identity_peak_rss_kib"], "identity RSS") > 0,
            "named-process RSS missing")

    accounting = qacct(
        args.run_dir / "qacct" / f"matlab_numopt2_{args.experiment}.txt"
    )
    job_id = (
        args.run_dir / "submissions" / f"matlab_numopt2_{args.experiment}.job_id"
    ).read_text().strip()
    require(accounting["jobnumber"] == job_id == node["job_id"],
            "job identity changed")
    require(int(accounting["slots"]) == int(task["requested_slots"]) == 4,
            "slot count changed")
    pe = accounting["granted_pe"]
    require(pe == "omp" or pe == "omp4", "scheduler binding changed")
    require(node["source_commit"] == args.expected_source_commit and
            node["bundle_sha256"] == args.expected_bundle,
            "node source changed")
    require(node["kss_source_commit"] == args.expected_kss_source_commit and
            node["kss_bundle_sha256"] == args.expected_kss_bundle,
            "node KSS source changed")
    require((base / "wrapper.pass").is_file() and
            (base / "matlab.pass").is_file(), "pass marker missing")

    payload = {
        "validation_version": "KSS-NUMOPT-2-MATLAB-VALIDATION-V1",
        "status": "PASS",
        "experiment_id": args.experiment,
        "job_id": job_id,
        "comparison_source_commit": args.expected_source_commit,
        "comparison_bundle_sha256": args.expected_bundle,
        "kss_source_commit": args.expected_kss_source_commit,
        "kss_bundle_sha256": args.expected_kss_bundle,
        "task_sha256": sha256(task_path),
        "kss_validation_sha256": sha256(args.kss_experiment_dir / "validation.json"),
        "aggregate_sha256": sha256(base / "aggregate.json"),
        "source_identity_sha256": sha256(base / "source_identity.json"),
        "process_tree_sha256": sha256(base / "process_tree_rss.json"),
        "gnu_time_max_rss_kib": max_rss(base / "resources.txt"),
        "process_tree_peak_rss_kib": tree["peak_rss_kib"],
        "qacct_wall_seconds": finite(accounting["ru_wallclock"], "qacct wall"),
        "qacct_maxvmem": accounting["maxvmem"],
        "frequency_semantics_comparable": expected_frequency,
        "target_weight_semantics_comparable": False,
        "corrected_estimate_equality_gate": "NONE_DESCRIPTIVE_ONLY",
    }
    (base / "validation.json").write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"KSS_NUMOPT2_MATLAB_VALIDATION_PASS {args.experiment} {job_id}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

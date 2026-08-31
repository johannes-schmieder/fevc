#!/usr/bin/env python3
"""Validate one source-bound CMG-MATA-1 hierarchy SCC benchmark."""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import re
from pathlib import Path


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def key_values(path: Path) -> dict[str, str]:
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.reader(handle, delimiter="\t")
        require(next(reader, None) == ["key", "value"], "invalid TSV header")
        rows = list(reader)
    require(all(len(row) == 2 for row in rows), "invalid TSV row")
    result = {row[0]: row[1] for row in rows}
    require(len(result) == len(rows), "duplicate TSV key")
    return result


def qacct(path: Path) -> dict[str, str]:
    result: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        fields = line.strip().split(None, 1)
        if len(fields) == 2:
            result[fields[0]] = fields[1]
    required = {"jobnumber", "taskid", "project", "granted_pe", "slots",
                "failed", "exit_status", "ru_wallclock", "cpu", "maxvmem",
                "hostname"}
    require(required <= result.keys(), "incomplete qacct")
    return result


def finite(row: dict[str, str], field: str) -> float:
    value = float(row[field])
    require(math.isfinite(value), f"nonfinite {field}")
    return value


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--run-dir", type=Path, required=True)
    parser.add_argument("--experiment", required=True)
    parser.add_argument("--expected-commit", required=True)
    parser.add_argument("--expected-bundle", required=True)
    args = parser.parse_args()
    require(re.fullmatch(r"[A-Za-z0-9._-]+", args.experiment) is not None,
            "invalid experiment")
    require(re.fullmatch(r"[0-9a-f]{40}", args.expected_commit) is not None,
            "invalid commit")
    require(re.fullmatch(r"[0-9a-f]{64}", args.expected_bundle) is not None,
            "invalid bundle")
    output = args.run_dir / "hierarchy" / args.experiment
    task = key_values(output / "task.tsv")
    require(task["task_version"] == "CMG-MATA-1-HIERARCHY-TASK-V1",
            "task version changed")
    require(task["experiment_id"] == args.experiment, "experiment changed")
    require(task["source_commit"] == args.expected_commit and
            task["bundle_sha256"] == args.expected_bundle,
            "task source binding changed")
    require(sha256(output / "task.tsv") ==
            (output / "task.sha256").read_text().strip(), "task hash changed")
    job_id = (args.run_dir / "submissions" /
              f"cmg_hierarchy_{args.experiment}.job_id").read_text().strip()
    accounting = qacct(args.run_dir / "qacct" /
                       f"cmg_hierarchy_{args.experiment}.txt")
    require(accounting["jobnumber"] == job_id and
            accounting["taskid"] == "undefined", "job identity changed")
    require(accounting["project"] == "welfgr", "project changed")
    slots = int(task["requested_slots"])
    require(int(accounting["slots"]) == slots, "slot count changed")
    require(accounting["granted_pe"] in {"omp", f"omp{slots}"},
            "parallel environment changed")
    require(accounting["failed"] == accounting["exit_status"] == "0",
            "scheduler or wrapper failed")
    require((output / "wrapper.pass").is_file(), "missing wrapper pass")
    require(not (output / "wrapper.fail").exists(), "wrapper failure exists")
    node = key_values(output / "node_receipt.tsv")
    require(node["source_commit"] == args.expected_commit and
            node["bundle_sha256"] == args.expected_bundle,
            "node binding changed")
    require(int(node["actual_slots"]) == slots and
            int(node["stata_processors"]) == int(task["stata_processors"]),
            "node processor contract changed")
    with (output / "results.csv").open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    require(rows, "empty result")
    if task["kind"] == "degree":
        first, last = int(task["arg2"]), int(task["arg3"])
        require(len(rows) == last - first + 1, "degree row count changed")
        require({int(row["degree"]) for row in rows} == set(range(first, last + 1)),
                "degree grid changed")
        for row in rows:
            require(row["algorithm"] == task["arg4"], "algorithm changed")
            require(int(float(row["firms"])) == int(task["arg1"]),
                    "firm count changed")
            require(row["cells_status"] == row["graph_status"] ==
                    row["hierarchy_status"] == "CONVERGED", "degree failed")
            require(finite(row, "edge_complexity") <= 12 and
                    finite(row, "vertex_complexity") <= 5,
                    "complexity gate failed")
            if finite(row, "levels") > 1:
                require(finite(row, "minimum_reduction") >= .20,
                        "reduction gate failed")
    else:
        require(len(rows) == 1, "family row count changed")
        row = rows[0]
        require(row["graph_family"] == task["arg3"], "family changed")
        require(int(float(row["vertices"])) == int(task["arg1"]),
                "vertex count changed")
        require(int(float(row["batch_columns"])) == int(task["arg2"]),
                "batch width changed")
        require(finite(row, "levels") >= 1 and
                finite(row, "edge_complexity") <= 12 and
                finite(row, "vertex_complexity") <= 5,
                "family hierarchy gate failed")
        if finite(row, "levels") > 1:
            require(finite(row, "minimum_reduction") >= .20,
                    "family reduction gate failed")
    payload = {
        "validation_version": "CMG-MATA-1-HIERARCHY-VALIDATION-V1",
        "status": "PASS",
        "experiment_id": args.experiment,
        "kind": task["kind"],
        "job_id": job_id,
        "source_commit": args.expected_commit,
        "bundle_sha256": args.expected_bundle,
        "task_sha256": sha256(output / "task.tsv"),
        "results_sha256": sha256(output / "results.csv"),
        "result_rows": len(rows),
        "qacct": accounting,
    }
    (output / "validation.json").write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"CMG_HIERARCHY_VALIDATION_PASS {args.experiment} {job_id}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

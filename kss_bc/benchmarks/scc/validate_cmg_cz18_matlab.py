#!/usr/bin/env python3
"""Validate the matched official-MATLAB CMG run on fixed CZ18."""
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
        rows = list(csv.reader(handle, delimiter="\t"))
    require(rows and rows[0] == ["key", "value"], f"invalid TSV: {path}")
    require(all(len(row) == 2 for row in rows[1:]), f"invalid row: {path}")
    result = {row[0]: row[1] for row in rows[1:]}
    require(len(result) == len(rows) - 1, f"duplicate key: {path}")
    return result


def one_csv(path: Path) -> dict[str, str]:
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    require(len(rows) == 1, f"expected one CSV row: {path}")
    return rows[0]


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


def finite(value: object, label: str) -> float:
    result = float(value)
    require(math.isfinite(result), f"nonfinite {label}")
    return result


def max_rss(path: Path) -> int:
    match = re.search(r"Maximum resident set size \(kbytes\):\s*([0-9]+)",
                      path.read_text(encoding="utf-8"))
    require(match is not None, f"missing GNU-time RSS: {path}")
    return int(match.group(1))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--run-dir", type=Path, required=True)
    parser.add_argument("--experiment", required=True)
    parser.add_argument("--stata-experiment-dir", type=Path, required=True)
    parser.add_argument("--expected-commit", required=True)
    parser.add_argument("--expected-bundle", required=True)
    parser.add_argument("--expected-input-sha256", required=True)
    args = parser.parse_args()
    require(re.fullmatch(r"[A-Za-z0-9._-]+", args.experiment) is not None,
            "invalid experiment")
    require(re.fullmatch(r"[0-9a-f]{40}", args.expected_commit) is not None,
            "invalid commit")
    require(re.fullmatch(r"[0-9a-f]{64}", args.expected_bundle) is not None,
            "invalid bundle")
    require(re.fullmatch(r"[0-9a-f]{64}", args.expected_input_sha256)
            is not None, "invalid input hash")

    base = args.run_dir / "matlab_cz18" / args.experiment
    task_path = base / "task.tsv"
    task = key_values(task_path)
    require(task["task_version"] == "CMG-MATA-1-CZ18-MATLAB-TASK-V1",
            "task version changed")
    require(task["experiment_id"] == args.experiment, "experiment changed")
    require(task["source_commit"] == args.expected_commit and
            task["bundle_sha256"] == args.expected_bundle,
            "task source binding changed")
    require(task["input_sha256"] == args.expected_input_sha256,
            "task input changed")
    require(sha256(task_path) == (base / "task.sha256").read_text().strip(),
            "task hash changed")
    require(task["probes"] == "20" and task["seed"] == "8675309",
            "MATLAB probe or seed contract changed")
    require(task["requested_slots"] == task["matlab_pool_workers"] == "4",
            "MATLAB core contract changed")
    require(task["stata_prepare_processors"] == "4",
            "preparation processor contract changed")
    require(task["stored_rows"] == "8201888" and
            task["workers"] == "117529" and task["firms"] == "10603" and
            task["coefficient_cells"] == "311730", "CZ18 task changed")

    stata_summary = one_csv(args.stata_experiment_dir / "summary.csv")
    stata_validation = json.loads(
        (args.stata_experiment_dir / "validation.json").read_text())
    require(stata_validation["status"] == "KSS_STREAMLINE_SCIENTIFIC_PASS",
            "matched Stata evidence did not pass")
    require(stata_summary["input_sha256"] == args.expected_input_sha256 and
            stata_summary["requested_probes"] == "20" and
            stata_summary["seed"] == "8675309", "Stata comparison changed")
    for field, expected in (("N_stored", 8201888),
                            ("worker_levels", 117529),
                            ("firm_levels", 10603),
                            ("coefficient_cells", 311730)):
        require(int(float(stata_summary[field])) == expected,
                f"Stata dimension changed: {field}")
    require(stata_summary["preconditioner_selected"] == "CMG",
            "matched Stata run did not select CMG")

    prepare = one_csv(base / "prepare.csv")
    require(prepare["experiment_id"] == args.experiment and
            prepare["source_commit"] == args.expected_commit and
            prepare["bundle_sha256"] == args.expected_bundle and
            prepare["input_sha256"] == args.expected_input_sha256,
            "preparation binding changed")
    for field, expected in (("rows", 8201888), ("workers", 117529),
                            ("firms", 10603),
                            ("coefficient_cells", 311730),
                            ("processors", 4)):
        require(int(float(prepare[field])) == expected,
                f"preparation dimension changed: {field}")
    prepared_sha = (base / "prepared_input.sha256").read_text().strip()
    require(re.fullmatch(r"[0-9a-f]{64}", prepared_sha) is not None,
            "invalid prepared input hash")

    aggregate = json.loads((base / "aggregate.json").read_text())
    source = json.loads((base / "source_identity.json").read_text())
    identity = json.loads((base / "process_identity.json").read_text())
    tree = json.loads((base / "process_tree_rss.json").read_text())
    require(aggregate["schema"] ==
            "CMG-MATA-1-CZ18-MATLAB-AGGREGATE-V1" and
            aggregate["status"] == source["status"] == "PASS",
            "MATLAB application or source identity failed")
    require(aggregate["experiment_id"] == args.experiment and
            aggregate["comparison_source_commit"] == args.expected_commit and
            aggregate["comparison_bundle_sha256"] == args.expected_bundle and
            aggregate["task_sha256"] == sha256(task_path) and
            aggregate["input_sha256"] == args.expected_input_sha256 and
            aggregate["prepared_input_sha256"] == prepared_sha,
            "MATLAB aggregate binding changed")
    for field, expected in (("stored_rows", 8201888), ("workers", 117529),
                            ("firms", 10603),
                            ("coefficient_cells", 311730),
                            ("detail_matches", 311730), ("probes", 20),
                            ("seed", 8675309), ("pool_workers", 4)):
        require(int(finite(aggregate[field], field)) == expected,
                f"MATLAB aggregate changed: {field}")
    require(finite(aggregate["command_seconds"], "command time") > 0 and
            finite(aggregate["target_identity_scaled_error"], "identity")
            <= 1e-12, "MATLAB timing or identity failed")
    require(aggregate["same_retained_input"] is True and
            aggregate["same_probe_count"] is True and
            aggregate["corrected_estimate_equality_gate"] ==
            "NONE_DESCRIPTIVE_ONLY", "comparison boundary changed")
    require(identity["status"] == "PASS" and
            identity["expected_pool_workers"] == 4 and
            identity["worker_indices"] == [1, 2, 3, 4] and
            len(set(identity["worker_pids"])) == 4,
            "MATLAB process identity failed")
    require(tree["status"] == "PASS" and
            tree["all_named_pids_observed_as_descendants"] is True and
            finite(tree["peak_rss_kib"], "process tree RSS") > 0,
            "MATLAB process tree failed")

    node = key_values(base / "node_receipt.tsv")
    job_file = (args.run_dir / "submissions" /
                f"cmg_cz18_matlab_{args.experiment}.job_id")
    job_id = job_file.read_text().strip()
    accounting = qacct(args.run_dir / "qacct" /
                       f"cmg_cz18_matlab_{args.experiment}.txt")
    require(accounting["jobnumber"] == node["job_id"] == job_id,
            "job identity changed")
    require(accounting["taskid"] == "undefined" and
            accounting["project"] == "welfgr", "scheduler identity changed")
    require(accounting["failed"] == accounting["exit_status"] == "0",
            "scheduler or wrapper failed")
    require(int(accounting["slots"]) == int(node["actual_slots"]) == 4 and
            accounting["granted_pe"] in {"omp", "omp4"},
            "scheduler core contract changed")
    require(node["source_commit"] == args.expected_commit and
            node["bundle_sha256"] == args.expected_bundle and
            node["input_sha256"] == args.expected_input_sha256 and
            node["prepared_input_sha256"] == prepared_sha,
            "node binding changed")
    require((base / "wrapper.pass").is_file() and
            (base / "matlab.pass").is_file() and
            not (base / "wrapper.fail").exists(), "pass marker failed")

    payload = {
        "validation_version": "CMG-MATA-1-CZ18-MATLAB-VALIDATION-V1",
        "status": "PASS",
        "experiment_id": args.experiment,
        "job_id": job_id,
        "source_commit": args.expected_commit,
        "bundle_sha256": args.expected_bundle,
        "input_sha256": args.expected_input_sha256,
        "prepared_input_sha256": prepared_sha,
        "task_sha256": sha256(task_path),
        "stata_validation_sha256": sha256(
            args.stata_experiment_dir / "validation.json"),
        "aggregate_sha256": sha256(base / "aggregate.json"),
        "probes": 20,
        "stata_processors": 4,
        "matlab_pool_workers": 4,
        "stata_command_seconds": finite(stata_summary["command_seconds"],
                                        "Stata command time"),
        "matlab_command_seconds": finite(aggregate["command_seconds"],
                                         "MATLAB command time"),
        "prepare_gnu_time_max_rss_kib": max_rss(
            base / "prepare_resources.txt"),
        "matlab_gnu_time_max_rss_kib": max_rss(
            base / "matlab_resources.txt"),
        "process_tree_peak_rss_kib": tree["peak_rss_kib"],
        "qacct_wall_seconds": finite(accounting["ru_wallclock"], "qacct wall"),
        "qacct_maxvmem": accounting["maxvmem"],
        "corrected_estimate_equality_gate": "NONE_DESCRIPTIVE_ONLY",
    }
    (base / "validation.json").write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"CMG_CZ18_MATLAB_VALIDATION_PASS {args.experiment} {job_id}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

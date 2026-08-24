#!/usr/bin/env python3
"""Validate an Optimization III rung stopped by time, not science."""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
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


def qacct(path: Path) -> dict[str, str]:
    require(path.is_file(), f"missing qacct: {path}")
    result: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        fields = line.strip().split(None, 1)
        if len(fields) == 2:
            result[fields[0]] = fields[1]
    required = {
        "jobnumber", "failed", "exit_status", "ru_wallclock", "maxvmem",
        "slots", "hostname", "qname",
    }
    require(required <= result.keys(), "incomplete qacct")
    return result


def finite(value: Any, label: str) -> float:
    result = float(value)
    require(math.isfinite(result) and result >= 0, f"invalid {label}")
    return result


def parse_memory(value: str) -> int:
    units = {"": 1, "K": 1024, "M": 1024**2, "G": 1024**3, "T": 1024**4}
    text = value.strip().upper()
    suffix = text[-1] if text and text[-1] in units and not text[-1].isdigit() else ""
    number = text[:-1] if suffix else text
    result = float(number) * units[suffix]
    require(math.isfinite(result) and result >= 0, f"invalid memory: {value}")
    return math.ceil(result)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--evidence-dir", type=Path, required=True)
    parser.add_argument("--expected-source-commit", required=True)
    parser.add_argument("--expected-bundle", required=True)
    parser.add_argument(
        "--status",
        choices=("CENSORED_APPLICATION_TIMEOUT", "STOPPED_AFTER_DECISION_BOUND"),
        required=True,
    )
    parser.add_argument("--command-lower-bound-seconds", type=float, required=True)
    args = parser.parse_args()

    directory = args.evidence_dir
    task_path = directory / "task.tsv"
    task = key_values(task_path)
    require(task["task_version"] == "KSS-NUMOPT-2-TASK-V1", "task changed")
    require(task["source_commit"] == args.expected_source_commit, "source changed")
    require(task["bundle_sha256"] == args.expected_bundle, "bundle changed")
    require(
        sha256(task_path) == (directory / "task.sha256").read_text().strip(),
        "task hash changed",
    )
    job_id = (directory / "job_id").read_text().strip()
    accounting = qacct(directory / "qacct.txt")
    require(accounting["jobnumber"] == job_id, "job identity changed")
    require(int(accounting["slots"]) == int(task["slots"]), "slots changed")
    require(
        accounting["failed"] != "0" or accounting["exit_status"] != "0",
        "censored rung unexpectedly passed",
    )
    require(not (directory / "wrapper.pass").exists(), "wrapper pass is invalid")
    require(not (directory / "stata.pass").exists(), "Stata pass is invalid")
    require(not (directory / "summary.csv").exists(), "summary is invalid")
    application_path = directory / "application.log"
    application = application_path.read_text(encoding="utf-8", errors="replace")
    require("capture noisily vckss" in application, "estimator did not start")
    require("KSS-STREAMLINE ESTIMATOR PASS" not in application, "unexpected pass")
    lower = finite(args.command_lower_bound_seconds, "command lower bound")
    require(lower > 0, "zero command lower bound")
    if args.status == "CENSORED_APPLICATION_TIMEOUT":
        require(accounting["failed"] == "0", "timeout was a scheduler failure")
        require(accounting["exit_status"] == "124", "timeout exit changed")
        expected = int(task["hard_wall_seconds"]) - 180
        require(math.isclose(lower, expected), "timeout lower bound changed")

    payload = {
        "validation_version": "KSS-NUMOPT-2-CENSORED-VALIDATION-V1",
        "status": args.status,
        "experiment_id": task["experiment_id"],
        "job_id": job_id,
        "source_commit": args.expected_source_commit,
        "bundle_sha256": args.expected_bundle,
        "task_sha256": sha256(task_path),
        "application_sha256": sha256(application_path),
        "qacct_sha256": sha256(directory / "qacct.txt"),
        "command_lower_bound_seconds": lower,
        "scientific_result_available": False,
        "qacct_failed": accounting["failed"],
        "qacct_exit_status": accounting["exit_status"],
        "qacct_wall_seconds": finite(accounting["ru_wallclock"], "qacct wall"),
        "qacct_maxvmem_bytes": parse_memory(accounting["maxvmem"]),
        "hostname": accounting["hostname"],
        "queue": accounting["qname"],
    }
    (directory / "validation.json").write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(
        "KSS_NUMOPT2_CENSORED_VALIDATION_PASS "
        f"{task['experiment_id']} {job_id} {args.status}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

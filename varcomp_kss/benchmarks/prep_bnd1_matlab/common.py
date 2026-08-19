#!/usr/bin/env python3
"""Receipt helpers for the PREP-BND-1 maintained-MATLAB comparison."""

from __future__ import annotations

import csv
import hashlib
import json
import math
import re
from pathlib import Path
from typing import Any

HEX40 = re.compile(r"[0-9a-f]{40}")
HEX64 = re.compile(r"[0-9a-f]{64}")
LABEL = re.compile(r"[A-Za-z0-9._-]+")
TASK_SCHEMA = "PREP-BND-1-MATLAB-TASK-V1"
TASK_FIELDS = (
    "task_schema",
    "source_commit",
    "bundle_sha256",
    "experiment_id",
    "firms",
    "workers",
    "cells_per_worker",
    "rows",
    "probes",
    "seed",
    "order",
    "requested_slots",
    "stata_processors",
    "matlab_pool_workers",
    "mem_per_core_gib",
    "hard_wall_seconds",
    "sample_contract",
    "target_contract",
    "comparison_contract",
)
TARGETS = ("worker", "firm", "covariance", "total")


class EvidenceError(ValueError):
    """A source, scheduler, application, or scientific receipt is invalid."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise EvidenceError(message)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def finite(value: Any, label: str) -> float:
    try:
        result = float(value)
    except (TypeError, ValueError) as exc:
        raise EvidenceError(f"nonfinite {label}") from exc
    require(math.isfinite(result), f"nonfinite {label}")
    return result


def integer(value: Any, label: str, minimum: int = 0) -> int:
    result = finite(value, label)
    require(result == math.floor(result) and result >= minimum,
            f"invalid integer {label}")
    return int(result)


def load_json(path: Path) -> dict[str, Any]:
    require(path.is_file() and not path.is_symlink(), f"invalid JSON: {path}")
    value = json.loads(path.read_text(encoding="utf-8"))
    require(isinstance(value, dict), f"invalid JSON object: {path}")
    return value


def one_csv(path: Path) -> dict[str, str]:
    require(path.is_file() and not path.is_symlink(), f"invalid CSV: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    require(len(rows) == 1, f"expected one CSV row: {path}")
    return rows[0]


def read_task(path: Path) -> dict[str, str]:
    require(path.is_file() and not path.is_symlink(), f"invalid task: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle, delimiter="\t")
        require(tuple(reader.fieldnames or ()) == TASK_FIELDS,
                "task fields changed")
        rows = list(reader)
    require(len(rows) == 1, "task must contain exactly one row")
    task = rows[0]
    require(task["task_schema"] == TASK_SCHEMA, "task schema changed")
    require(HEX40.fullmatch(task["source_commit"]) is not None,
            "invalid source commit")
    require(HEX64.fullmatch(task["bundle_sha256"]) is not None,
            "invalid bundle hash")
    require(LABEL.fullmatch(task["experiment_id"]) is not None,
            "invalid experiment id")
    firms = integer(task["firms"], "firms", 2)
    workers = integer(task["workers"], "workers", 1)
    degree = integer(task["cells_per_worker"], "degree", 2)
    rows = integer(task["rows"], "rows", 1)
    require(workers == 40 * firms and degree == 3 and rows == workers * degree,
            "task dimensions changed")
    require(integer(task["probes"], "probes", 2) in {20, 200},
            "unsupported probe count")
    integer(task["seed"], "seed", 1)
    require(task["order"] in {"stata_matlab", "matlab_stata"},
            "invalid source order")
    require(task["requested_slots"] == task["stata_processors"] ==
            task["matlab_pool_workers"] == "4", "processor contract changed")
    require(task["mem_per_core_gib"] == "14", "memory contract changed")
    require(integer(task["hard_wall_seconds"], "hard wall", 600) <= 42600,
            "wall contract changed")
    require(task["sample_contract"] == "same_literal_rows_v1" and
            task["target_contract"] == "uniform_stored_rows_v1" and
            task["comparison_contract"] ==
            "descriptive_jla_own_pcg_gate_v1", "comparison contract changed")
    return task


def qacct(path: Path) -> dict[str, str]:
    require(path.is_file() and not path.is_symlink(), f"invalid qacct: {path}")
    result: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        fields = line.strip().split(None, 1)
        if len(fields) == 2:
            result[fields[0]] = fields[1]
    required = {
        "jobnumber", "taskid", "project", "granted_pe", "slots",
        "failed", "exit_status", "ru_wallclock", "cpu", "maxvmem",
        "hostname",
    }
    require(required <= result.keys(), "incomplete qacct")
    require(result["failed"] == result["exit_status"] == "0",
            "scheduler or wrapper failed")
    require(result["taskid"] == "undefined" and result["project"] == "welfgr",
            "scheduler identity changed")
    require(result["granted_pe"] in {"omp", "omp4"} and result["slots"] == "4",
            "scheduler processor contract changed")
    finite(result["ru_wallclock"], "qacct wall")
    finite(result["cpu"], "qacct cpu")
    return result


def parse_matlab_pcg(path: Path) -> dict[str, Any]:
    require(path.is_file(), f"missing MATLAB application log: {path}")
    source = path.read_text(encoding="utf-8", errors="replace")
    converged = re.search(
        r"pcg converged at iteration ([0-9]+) to a solution with relative "
        r"residual ([0-9.eE+-]+)\.", source,
    )
    stopped = re.search(
        r"pcg stopped at iteration ([0-9]+) without converging.*?"
        r"The iterate returned \(number ([0-9]+)\) has relative residual "
        r"([0-9.eE+-]+)\.", source, re.DOTALL,
    )
    require((converged is None) != (stopped is None),
            "ambiguous MATLAB PCG status")
    if converged:
        return {
            "converged": True,
            "termination_iteration": int(converged.group(1)),
            "returned_iteration": int(converged.group(1)),
            "relative_residual": finite(converged.group(2), "MATLAB residual"),
        }
    assert stopped is not None
    return {
        "converged": False,
        "termination_iteration": int(stopped.group(1)),
        "returned_iteration": int(stopped.group(2)),
        "relative_residual": finite(stopped.group(3), "MATLAB residual"),
    }


def scaled_target_gap(left: dict[str, float], right: dict[str, float]) -> float:
    numerator = math.sqrt(sum((left[key] - right[key]) ** 2 for key in TARGETS))
    denominator = 1.0 + math.sqrt(sum(left[key] ** 2 for key in TARGETS))
    return numerator / denominator

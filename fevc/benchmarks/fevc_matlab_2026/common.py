#!/usr/bin/env python3
"""Frozen contracts for the FEVC--MATLAB R2026a benchmark campaign."""

from __future__ import annotations

import csv
import hashlib
import json
import math
import re
from collections.abc import Iterable
from pathlib import Path
from typing import Any

TASK_SCHEMA = "FEVC-MATLAB-2026-MAIN-CELL-V1"
SMOKE_TASK_SCHEMA = "FEVC-MATLAB-2026-SMOKE-CELL-V1"
RESULT_SCHEMA = "FEVC-MATLAB-2026-RESULT-V1"
NODE_SCHEMA = "FEVC-MATLAB-2026-NODE-V1"
COLLECTION_SCHEMA = "FEVC-MATLAB-2026-COLLECTION-V1"

HEX40 = re.compile(r"[0-9a-f]{40}")
HEX64 = re.compile(r"[0-9a-f]{64}")
LABEL = re.compile(r"[A-Za-z0-9._-]+")

ROW_GRID = (7_680, 30_720, 122_880, 491_520, 1_966_080)
CORE_GRID = (1, 2, 4, 8, 14, 28)
STRUCTURES = {
    "strong_d2": ("strong", 2, "Multi-offset (2)"),
    "strong_d3": ("strong", 3, "Long-range (3)"),
    "strong_d6": ("strong", 6, "Long-range (6)"),
    "weak_d3": ("weak", 3, "Shallow hub-tree leaf-panel bottleneck (3)"),
}
REPLICATES = (
    (1, 104_729, "rust,matlab"),
    (2, 8_675_309, "matlab,rust"),
    (3, 20_260_819, "rust,matlab"),
    (4, 31_415_927, "matlab,rust"),
    (5, 67_280_421, "rust,matlab"),
    (6, 91_824_733, "matlab,rust"),
)
ESTIMATORS = ("rust", "matlab")
TARGETS = ("worker", "firm", "covariance", "total")
PROBES = 200
MAX_ITERATIONS = 10_000
REQUESTED_SLOTS = 28
SMOKE_REQUESTED_SLOTS = 4
STATA_MAX_PROCESSORS = 4
HARD_WALL_SECONDS = 28_800
SMOKE_HARD_WALL_SECONDS = 1_200
ESTIMATOR_TIMEOUT_SECONDS = 10_800
SMOKE_ESTIMATOR_TIMEOUT_SECONDS = 600

TASK_FIELDS = (
    "task_schema",
    "task_id",
    "experiment_id",
    "source_commit",
    "bundle_sha256",
    "structure",
    "connectivity",
    "cells_per_worker",
    "rows",
    "workers",
    "firms",
    "active_cores",
    "stata_processors",
    "rust_threads",
    "matlab_workers",
    "replicate",
    "seed",
    "execution_order",
    "probes",
    "requested_slots",
    "mem_per_core_gib",
    "command_memory_gib",
    "hard_wall_seconds",
    "estimator_timeout_seconds",
    "sample_contract",
    "target_contract",
    "comparison_contract",
)


class EvidenceError(ValueError):
    """Raised when benchmark evidence violates its registered contract."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise EvidenceError(message)


def registered_firms(structure: str, workers: int) -> int:
    """Return the topology-specific firm count for a registered task."""
    require(structure in STRUCTURES and workers > 0, "invalid graph dimensions")
    if structure == "weak_d3":
        require(workers % 5 == 0, "registered weak dimensions are not divisible")
        return workers // 5 + registered_weak_hubs(workers)
    require(workers % 40 == 0, "registered strong dimensions are not divisible")
    return workers // 40


def registered_weak_branches(workers: int) -> int:
    """Return the frozen adaptive branch count for a weak registered task."""
    require(workers % 5 == 0 and workers > 0,
            "registered weak dimensions are not divisible")
    # The 7,680-row cell has only 512 leaves.  Forty branches give 720 outer
    # hubs degree one, creating bridge matches and worker articulations.  A
    # 20-branch tree preserves the same depth-two bottleneck while giving
    # every hub at least two independent leaf-panel incidences.  All larger
    # registered cells retain the original 40-branch topology.
    return min(40, workers // 128)


def registered_weak_hubs(workers: int) -> int:
    """Return root + branches + 39 grandchildren per weak branch."""
    branches = registered_weak_branches(workers)
    return 1 + branches * 40


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


def one_csv(path: Path) -> dict[str, str]:
    require(path.is_file() and not path.is_symlink(), f"invalid CSV: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    require(len(rows) == 1, f"expected one CSV row: {path}")
    return rows[0]


def key_values(path: Path) -> dict[str, str]:
    require(path.is_file() and not path.is_symlink(), f"invalid TSV: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.reader(handle, delimiter="\t"))
    require(rows and rows[0] == ["key", "value"], f"invalid TSV header: {path}")
    require(all(len(row) == 2 for row in rows[1:]), f"invalid TSV row: {path}")
    value = {row[0]: row[1] for row in rows[1:]}
    require(len(value) == len(rows) - 1, f"duplicate TSV key: {path}")
    return value


def read_single_task(path: Path) -> dict[str, str]:
    require(path.is_file() and not path.is_symlink(), f"invalid task: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle, delimiter="\t")
        require(tuple(reader.fieldnames or ()) == TASK_FIELDS,
                "task fields changed")
        rows = list(reader)
    require(len(rows) == 1, "task must contain exactly one row")
    return validate_task(dict(rows[0]))


def parse_qacct(path: Path, *, expected_slots: int = REQUESTED_SLOTS) -> dict[str, str]:
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
    require(result["project"] == "welfgr", "scheduler project changed")
    require(result["granted_pe"] in {"omp", "omp16"} and
            integer(result["slots"], "qacct slots", 1) == expected_slots,
            "scheduler slot contract changed")
    integer(result["taskid"], "qacct task ID", 1)
    finite(result["ru_wallclock"], "qacct wall")
    finite(result["cpu"], "qacct CPU")
    return result


def parse_memory(value: str) -> int:
    units = {"": 1, "K": 1024, "M": 1024**2, "G": 1024**3, "T": 1024**4}
    text_value = value.strip().upper()
    suffix = text_value[-1] if text_value and text_value[-1] in units else ""
    number = text_value[:-1] if suffix else text_value
    result = finite(number, "memory") * units[suffix]
    require(result >= 0, "negative memory")
    return math.ceil(result)


def parse_gnu_time(path: Path) -> dict[str, float | int]:
    require(path.is_file(), f"missing GNU time receipt: {path}")
    source = path.read_text(encoding="utf-8", errors="replace")
    rss = re.search(r"Maximum resident set size \(kbytes\):\s*([0-9]+)", source)
    user = re.search(r"User time \(seconds\):\s*([0-9.]+)", source)
    system = re.search(r"System time \(seconds\):\s*([0-9.]+)", source)
    elapsed = re.search(
        r"Elapsed \(wall clock\) time \(h:mm:ss or m:ss\):\s*([0-9:.]+)",
        source,
    )
    require(all(match is not None for match in (rss, user, system, elapsed)),
            f"incomplete GNU time receipt: {path}")
    assert rss is not None and user is not None and system is not None
    assert elapsed is not None
    pieces = [float(item) for item in elapsed.group(1).split(":")]
    require(len(pieces) in (2, 3), "invalid GNU elapsed time")
    wall = pieces[-1] + 60 * pieces[-2]
    if len(pieces) == 3:
        wall += 3600 * pieces[0]
    return {
        "maximum_rss_bytes": int(rss.group(1)) * 1024,
        "user_seconds": float(user.group(1)),
        "system_seconds": float(system.group(1)),
        "wall_seconds": wall,
    }


def parse_matlab_pcg(path: Path) -> dict[str, Any]:
    """Parse the maintained comparator's single reported PCG termination."""
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
    if converged is not None:
        return {
            "converged": True,
            "termination_iteration": int(converged.group(1)),
            "returned_iteration": int(converged.group(1)),
            "relative_residual": finite(converged.group(2),
                                         "MATLAB PCG residual"),
        }
    assert stopped is not None
    return {
        "converged": False,
        "termination_iteration": int(stopped.group(1)),
        "returned_iteration": int(stopped.group(2)),
        "relative_residual": finite(stopped.group(3), "MATLAB PCG residual"),
    }


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


def execution_roles(value: str) -> tuple[str, ...]:
    roles = tuple(value.split(","))
    require(len(roles) == 2 and set(roles) == set(ESTIMATORS),
            "invalid execution order")
    return roles


def validate_task(task: dict[str, str]) -> dict[str, str]:
    require(tuple(task) == TASK_FIELDS, "task fields changed")
    is_smoke = task["task_schema"] == SMOKE_TASK_SCHEMA
    require(task["task_schema"] in {TASK_SCHEMA, SMOKE_TASK_SCHEMA},
            "task schema changed")
    require(HEX40.fullmatch(task["source_commit"]) is not None,
            "invalid source commit")
    require(HEX64.fullmatch(task["bundle_sha256"]) is not None,
            "invalid bundle hash")
    require(LABEL.fullmatch(task["experiment_id"]) is not None,
            "invalid experiment ID")
    task_id = integer(task["task_id"], "task ID", 1)
    require(task["structure"] in STRUCTURES, "unknown graph structure")
    connectivity, degree, _ = STRUCTURES[task["structure"]]
    require(task["connectivity"] == connectivity, "connectivity changed")
    require(integer(task["cells_per_worker"], "degree", 2) == degree,
            "degree changed")
    rows = integer(task["rows"], "rows", 1)
    workers = integer(task["workers"], "workers", 1)
    firms = integer(task["firms"], "firms", 1)
    require(rows in ROW_GRID and rows == workers * degree,
            "row/worker dimensions changed")
    require(firms == registered_firms(task["structure"], workers),
            "worker/firm dimensions changed")
    active_cores = integer(task["active_cores"], "active cores", 1)
    require(active_cores in CORE_GRID, "active-core grid changed")
    if is_smoke:
        require(task["structure"] == "strong_d2" and rows == 7_680 and
                active_cores == SMOKE_REQUESTED_SLOTS,
                "smoke dimensions changed")
    else:
        require(active_cores == 28 or rows == 491_520,
                "main matrix must be the registered row or core slice")
    stata_processors = min(active_cores, STATA_MAX_PROCESSORS)
    require(integer(task["stata_processors"], "Stata processors", 1) ==
            stata_processors, "Stata processor ceiling changed")
    require(integer(task["rust_threads"], "Rust threads", 1) == active_cores,
            "Rust scaling grid changed")
    require(integer(task["matlab_workers"], "MATLAB workers", 1) == active_cores,
            "MATLAB scaling grid changed")
    replicate = integer(task["replicate"], "replicate", 1)
    matches = [item for item in REPLICATES if item[0] == replicate]
    require(len(matches) == 1, "unknown replicate")
    _, seed, order = matches[0]
    require(integer(task["seed"], "seed", 1) == seed, "seed changed")
    require(task["execution_order"] == order, "execution order changed")
    execution_roles(order)
    require(integer(task["probes"], "probes", 1) == PROBES,
            "probe count changed")
    requested_slots = integer(task["requested_slots"], "requested slots", 1)
    require(requested_slots ==
            (SMOKE_REQUESTED_SLOTS if is_smoke else REQUESTED_SLOTS),
            "slot contract changed")
    mem_per_core = integer(task["mem_per_core_gib"], "memory per core", 1)
    command_memory = integer(task["command_memory_gib"], "command memory", 1)
    require(command_memory <= mem_per_core * requested_slots,
            "command memory exceeds scheduler allocation")
    require(integer(task["hard_wall_seconds"], "hard wall", 1) ==
            (SMOKE_HARD_WALL_SECONDS if is_smoke else HARD_WALL_SECONDS),
            "hard wall changed")
    require(integer(task["estimator_timeout_seconds"], "estimator timeout", 1) ==
            (SMOKE_ESTIMATOR_TIMEOUT_SECONDS
             if is_smoke else ESTIMATOR_TIMEOUT_SECONDS),
            "estimator timeout changed")
    require(task["sample_contract"] == "same_literal_match_rows_v3",
            "sample contract changed")
    require(task["target_contract"] == "uniform_stored_rows_v1",
            "target contract changed")
    require(task["comparison_contract"]
            == "paired_same_host_rust_matlab_time_absolute_memory_v1",
            "comparison contract changed")
    prefix = "smoke" if is_smoke else "scale"
    expected = f"{prefix}_{task['structure']}_n{rows}_c{active_cores}_r{replicate}"
    require(task["experiment_id"] == expected, "experiment ID changed")
    require(task_id >= 1, "invalid task ID")
    return task


def read_manifest(path: Path) -> list[dict[str, str]]:
    require(path.is_file() and not path.is_symlink(), "invalid task manifest")
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle, delimiter="\t")
        require(tuple(reader.fieldnames or ()) == TASK_FIELDS,
                "manifest fields changed")
        rows = [validate_task(dict(row)) for row in reader]
    require(len(rows) == 240, "manifest must contain exactly 240 cells")
    require([integer(row["task_id"], "task ID") for row in rows]
            == list(range(1, 241)), "task IDs are not consecutive")
    require(len({row["experiment_id"] for row in rows}) == 240,
            "duplicate experiment ID")
    return rows


def read_task(path: Path, task_id: int) -> dict[str, str]:
    rows = read_manifest(path)
    require(1 <= task_id <= len(rows), "task ID outside manifest")
    task = rows[task_id - 1]
    require(integer(task["task_id"], "task ID") == task_id,
            "task position does not match task ID")
    return task


def write_tsv(path: Path, fields: Iterable[str], rows: Iterable[dict[str, Any]]) -> None:
    fields = tuple(fields)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields, delimiter="\t",
                                lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)

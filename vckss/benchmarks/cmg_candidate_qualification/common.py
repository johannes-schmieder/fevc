#!/usr/bin/env python3
"""Frozen contracts for the paired CMG-candidate qualification study."""

from __future__ import annotations

import csv
import hashlib
import json
import math
import re
from pathlib import Path
from typing import Any, Iterable

COMPARISON_COMMIT = "427063bd3ba982d044f6f5b949cf8910ef67ec2d"
COMPARISON_CMG_COMMIT = "761a0f022f20d1114d9f20589b60563eab6fcb84"
CANDIDATE_BASE_COMMIT = "170e34bf060291fec1506c13ab2c60428b3f574a"
CANDIDATE_CMG_COMMIT = "92a12f2d572ca56b30a035220953f9dd4bced999"
TASK_SCHEMA = "VCKSS-CMG-CANDIDATE-QUALIFICATION-TASK-V2"
RESULT_SCHEMA = "VCKSS-CMG-CANDIDATE-QUALIFICATION-RESULT-V2"
NODE_SCHEMA = "VCKSS-CMG-CANDIDATE-QUALIFICATION-NODE-V3"
ACCEPTANCE_SCHEMA = "VCKSS-CMG-CANDIDATE-QUALIFICATION-ACCEPTANCE-V2"
RUN_SCHEMA = "VCKSS-CMG-CANDIDATE-QUALIFICATION-RUN-V2"
STRUCTURES = {
    "strong_d2": ("strong", 2),
    "strong_d3": ("strong", 3),
    "strong_d6": ("strong", 6),
    "weak_d3": ("weak", 3),
}
CORES = (1, 8, 16)
REPETITIONS = (
    (1, 104_729, "comparison,candidate"),
    (2, 8_675_309, "candidate,comparison"),
    (3, 20_260_819, "comparison,candidate"),
    (4, 41_024_633, "candidate,comparison"),
    (5, 73_939_133, "comparison,candidate"),
    (6, 97_451_087, "candidate,comparison"),
)
ROWS = 1_966_080
PROBES = 200
TASK_FIELDS = (
    "task_schema", "task_id", "experiment_id", "candidate_commit",
    "comparison_commit", "candidate_bundle_sha256", "comparison_bundle_sha256",
    "structure", "connectivity", "cells_per_worker", "rows", "workers",
    "firms", "active_cores", "stata_processors", "rust_threads",
    "replicate", "seed", "execution_order", "probes",
    "requested_slots", "mem_per_core_gib", "command_memory_gib",
    "hard_wall_seconds", "estimator_timeout_seconds",
)
HEX40 = re.compile(r"[0-9a-f]{40}")
HEX64 = re.compile(r"[0-9a-f]{64}")


class EvidenceError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise EvidenceError(message)


def registered_firms(structure: str, workers: int) -> int:
    """Return the topology-specific firm count for a registered task."""
    require(structure in STRUCTURES and workers > 0, "invalid graph dimensions")
    if structure == "weak_d3":
        require(workers % 5 == 0, "registered weak dimensions are not divisible")
        return workers // 5 + 1_601
    require(workers % 40 == 0, "registered strong dimensions are not divisible")
    return workers // 40


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


def key_values(path: Path) -> dict[str, str]:
    require(path.is_file() and not path.is_symlink(), f"invalid TSV: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.reader(handle, delimiter="\t"))
    require(rows and rows[0] == ["key", "value"], f"invalid TSV header: {path}")
    require(all(len(row) == 2 for row in rows[1:]), f"invalid TSV row: {path}")
    value = {row[0]: row[1] for row in rows[1:]}
    require(len(value) == len(rows) - 1, f"duplicate TSV key: {path}")
    return value


def write_tsv(path: Path, fields: Iterable[str], rows: Iterable[dict[str, Any]]) -> None:
    names = tuple(fields)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=names, delimiter="\t",
                                lineterminator="\n", extrasaction="raise")
        writer.writeheader()
        writer.writerows(rows)


def validate_task(value: dict[str, str]) -> dict[str, str]:
    require(value.get("task_schema") == TASK_SCHEMA, "task schema changed")
    task_id = integer(value.get("task_id"), "task ID", 1)
    require(task_id <= 72, "task ID outside 1..72")
    require(value.get("candidate_commit") and
            HEX40.fullmatch(value["candidate_commit"]) is not None,
            "invalid candidate commit")
    require(value.get("comparison_commit") == COMPARISON_COMMIT,
            "comparison commit changed")
    require(HEX64.fullmatch(value.get("candidate_bundle_sha256", "")) is not None
            and HEX64.fullmatch(value.get("comparison_bundle_sha256", "")) is not None,
            "invalid source bundle hash")
    structure = value.get("structure", "")
    require(structure in STRUCTURES, "invalid structure")
    connectivity, degree = STRUCTURES[structure]
    require(value.get("connectivity") == connectivity and
            integer(value.get("cells_per_worker"), "degree", 1) == degree,
            "graph identity changed")
    workers = ROWS // degree
    require(integer(value.get("rows"), "rows", 1) == ROWS and
            integer(value.get("workers"), "workers", 1) == workers and
            integer(value.get("firms"), "firms", 1) ==
            registered_firms(structure, workers), "task dimensions changed")
    active_cores = integer(value.get("active_cores"), "cores", 1)
    require(active_cores in CORES, "active cores changed")
    require(integer(value.get("stata_processors"), "Stata processors", 1) ==
            min(active_cores, 4), "Stata processor ceiling changed")
    require(integer(value.get("rust_threads"), "Rust threads", 1) == active_cores,
            "Rust thread grid changed")
    replicate = integer(value.get("replicate"), "replicate", 1)
    require(replicate <= 6, "replicate changed")
    expected = REPETITIONS[replicate - 1]
    require(integer(value.get("seed"), "seed", 1) == expected[1] and
            value.get("execution_order") == expected[2], "repetition changed")
    require(integer(value.get("probes"), "probes", 1) == PROBES and
            integer(value.get("requested_slots"), "slots", 1) == 16 and
            integer(value.get("hard_wall_seconds"), "wall", 1) == 43_200 and
            integer(value.get("estimator_timeout_seconds"), "timeout", 1) == 18_000,
            "execution contract changed")
    memory = integer(value.get("mem_per_core_gib"), "memory per core", 1)
    command = integer(value.get("command_memory_gib"), "command memory", 1)
    require(command <= 16 * memory, "command memory exceeds allocation")
    return value


def read_manifest(path: Path) -> list[dict[str, str]]:
    require(path.is_file() and not path.is_symlink(), "invalid task manifest")
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle, delimiter="\t")
        require(tuple(reader.fieldnames or ()) == TASK_FIELDS, "task fields changed")
        rows = [validate_task(dict(row)) for row in reader]
    require(len(rows) == 72 and [int(row["task_id"]) for row in rows] ==
            list(range(1, 73)), "qualification manifest must contain tasks 1..72")
    return rows


def read_task(path: Path, task_id: int) -> dict[str, str]:
    return read_manifest(path)[task_id - 1]


def read_single_task(path: Path) -> dict[str, str]:
    require(path.is_file() and not path.is_symlink(), "invalid single task")
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle, delimiter="\t")
        require(tuple(reader.fieldnames or ()) == TASK_FIELDS, "task fields changed")
        rows = list(reader)
    require(len(rows) == 1, "expected one task row")
    return validate_task(dict(rows[0]))


def parse_memory(value: str) -> int:
    units = {"": 1, "K": 1024, "M": 1024**2, "G": 1024**3, "T": 1024**4}
    text = value.strip().upper()
    suffix = text[-1] if text and text[-1] in units else ""
    return math.ceil(finite(text[:-1] if suffix else text, "memory") * units[suffix])


def parse_qacct(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        fields = line.strip().split(None, 1)
        if len(fields) == 2:
            values[fields[0]] = fields[1]
    required = {"jobnumber", "taskid", "hostname", "project", "granted_pe",
                "slots", "failed", "exit_status", "ru_wallclock", "cpu", "maxvmem"}
    require(required <= values.keys(), "incomplete qacct")
    require(values["failed"] == values["exit_status"] == "0",
            "qacct failure")
    require(values["project"] == "welfgr" and values["slots"] == "16" and
            values["granted_pe"] in {"omp", "omp16"}, "qacct resource mismatch")
    return values


def geometric_mean(values: Iterable[float]) -> float:
    selected = list(values)
    require(selected and all(value > 0 and math.isfinite(value) for value in selected),
            "geometric mean requires positive finite values")
    return math.exp(sum(math.log(value) for value in selected) / len(selected))

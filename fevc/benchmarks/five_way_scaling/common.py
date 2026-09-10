#!/usr/bin/env python3
"""Shared contracts for the five-way FE variance-component benchmark."""

from __future__ import annotations

import csv
import hashlib
import json
import os
from pathlib import Path
from typing import Any, Iterable

SCHEMA = "FEVC-FIVE-WAY-SCALING-V1"
SOURCE_COMMIT = "f3098bc1369992fccbc1d276aac5fc65ceb3f404"
PROBES = 280
ROLES = ("fevc", "matlab", "julia", "r", "pytwoway")
SIZE_ROWS = (7_680, 30_720, 122_880, 491_520)
CORE_GRID = (1, 2, 4, 8, 14, 28)
SEEDS = (202609091, 202609193, 202609299, 202609401, 202609507)
TASK_FIELDS = (
    "task_id", "cell_id", "sweep", "rows", "workers", "firms", "cores",
    "repeat", "seed", "probes", "algorithm", "role_order",
)
TARGETS = ("worker", "firm", "covariance", "total")

COMPARATORS = {
    "fevc": {
        "identity": SOURCE_COMMIT,
        "archive_sha256": None,
    },
    "matlab": {
        "identity": "8b957ffeb10b8465a3584fceb0265cccc48379e1",
        "archive_sha256": "d8b48ae7f994d8f9b007b9d6b2c750f82499d4e3e5279e05fbe8a2945548c417",
    },
    "julia": {
        "identity": "fa3d66ea160cd6e2169a992fe773fc20341b3f71",
        "archive_sha256": "9ad42cdaa88c47eee66729c99b1ef86f57fd97ce83a28755bc081270af76e571",
        "version": "0.1.0",
    },
    "r": {
        "identity": "LeaveOutKSS_0.1.0",
        "archive_sha256": "f70360b6d880dba0f87ddc2604fe289cc9a731b1d1f98dd5e2291fc8333d5d33",
        "version": "0.1.0",
    },
    "pytwoway": {
        "identity": "7208bda3aee23ba8db2400f4d7ac7ac9107c2d13",
        "archive_sha256": "b3a221e0f145b6f1b5f66b6073d98dd9297ea0ab80008e670fab3a2e18d13675",
        "version": "0.3.21",
    },
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def atomic_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n",
                         encoding="utf-8")
    os.replace(temporary, path)


def write_tsv(path: Path, fields: Iterable[str], rows: Iterable[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    with temporary.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=tuple(fields), delimiter="\t",
                                lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)
    os.replace(temporary, path)


def read_manifest(path: Path) -> list[dict[str, str]]:
    require(path.is_file() and not path.is_symlink(), "manifest is missing or a symlink")
    with path.open(encoding="utf-8", newline="") as handle:
        reader = csv.DictReader(handle, delimiter="\t")
        require(tuple(reader.fieldnames or ()) == TASK_FIELDS, "manifest fields changed")
        rows = list(reader)
    require(bool(rows), "manifest is empty")
    validate_rows(rows)
    return rows


def validate_rows(rows: list[dict[str, str]]) -> None:
    seen_tasks: set[int] = set()
    seen_keys: set[tuple[str, int]] = set()
    for expected, row in enumerate(rows, 1):
        task_id = int(row["task_id"])
        n = int(row["rows"])
        cores = int(row["cores"])
        repeat = int(row["repeat"])
        roles = tuple(row["role_order"].split(","))
        require(task_id == expected and task_id not in seen_tasks,
                "task ids must be consecutive and unique")
        require(n in (*SIZE_ROWS, 960), "row count is outside the registered grid")
        require(n % 120 == 0, "strong_d3 dimensions are not integral")
        require(int(row["workers"]) == n // 3, "worker count changed")
        require(int(row["firms"]) == n // 120, "firm count changed")
        require(cores in CORE_GRID, "core count is outside the registered grid")
        require(1 <= repeat <= 5, "repeat is outside the registered range")
        require(int(row["probes"]) == PROBES or row["algorithm"] == "exact",
                "projection count changed")
        require(row["algorithm"] in {"jla", "exact"}, "invalid algorithm")
        require(len(roles) == len(ROLES) and set(roles) == set(ROLES),
                "role order is not a permutation")
        require((row["cell_id"], repeat) not in seen_keys, "duplicate task key")
        seen_tasks.add(task_id)
        seen_keys.add((row["cell_id"], repeat))


def normalization_factor(role: str, n: int) -> float:
    require(role in ROLES and n > 1, "invalid normalization request")
    return (n - 1) / n if role in {"matlab", "julia", "r"} else 1.0


def normalized_targets(role: str, n: int, raw: dict[str, float]) -> dict[str, float]:
    factor = normalization_factor(role, n)
    value = {target: float(raw[target]) * factor for target in TARGETS}
    scale = 1.0 + sum(abs(value[target]) for target in TARGETS)
    require(abs(value["total"] - value["worker"] - value["firm"] -
                2 * value["covariance"]) / scale <= 1e-10,
            "normalized target identity failed")
    return value


def exact_tolerance(oracle: float) -> float:
    return max(1e-8, 1e-5 * max(1.0, abs(oracle)))

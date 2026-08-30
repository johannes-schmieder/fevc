#!/usr/bin/env python3
"""Registered contract for the staged AKM-shaped projection comparison."""

from __future__ import annotations

import csv
import hashlib
import json
import math
from dataclasses import asdict, dataclass
from pathlib import Path


SCHEMA = "VCKSS-PROJECTION-AKM-TASK-V2"
SOURCE_RE = r"^[0-9a-f]{40}$"
ROWS = (480_000, 1_920_000, 7_680_000)
CORES = (4, 16)
PROBES = {
    6_000: 1_256,
    480_000: 1_888,
    1_920_000: 2_088,
    7_680_000: 2_288,
}
MEMORY_GIB = {480_000: 64, 1_920_000: 128, 7_680_000: 256}
ROLE_CAP_SECONDS = {480_000: 10_800, 1_920_000: 36_000, 7_680_000: 43_200}
MAXITER = {6_000: 20_000, 480_000: 40_000, 1_920_000: 40_000, 7_680_000: 40_000}
PAIR_H_RT = {480_000: "06:30:00", 1_920_000: "21:00:00", 7_680_000: "25:00:00"}
MEM_PER_CORE_GIB = {480_000: 4, 1_920_000: 8, 7_680_000: 16}
UPSTREAM_COMMIT = "8b957ffeb10b8465a3584fceb0265cccc48379e1"
LEAVE_OUT_SHA256 = "54b30ebdc51b4c94873e2e3f205bbf865179220e3ad0df0e382922db7c84fc58"
LINCOM_SHA256 = "71fb47ce35d26c91dbf97926031359ed0eb31d9916b07c167c577867620ee5a9"


@dataclass(frozen=True)
class Task:
    schema: str
    stage: str
    task_id: int
    rows: int
    workers: int
    firms: int
    cores: int
    replicate: int
    probes: int
    seed: int
    order: str
    role_cap_seconds: int
    maxiter: int
    memory_gib: int

    @property
    def task_sha256(self) -> str:
        payload = json.dumps(asdict(self), sort_keys=True, separators=(",", ":"))
        return hashlib.sha256(payload.encode("utf-8")).hexdigest()


def order_for(rows: int, cores: int, replicate: int) -> str:
    """Rotate first role across cores and repetitions without randomness."""

    size_index = ROWS.index(rows)
    core_index = CORES.index(cores)
    return (
        "rust_matlab"
        if (size_index + core_index + replicate) % 2 == 1
        else "matlab_rust"
    )


def task(rows: int, cores: int, replicate: int, task_id: int, stage: str) -> Task:
    if rows not in ROWS or cores not in CORES or replicate not in (1, 2, 3):
        raise ValueError("task is outside the registered grid")
    workers = rows // 6
    firms = workers // 2
    return Task(
        schema=SCHEMA,
        stage=stage,
        task_id=task_id,
        rows=rows,
        workers=workers,
        firms=firms,
        cores=cores,
        replicate=replicate,
        probes=PROBES[rows],
        seed=20_260_830 + replicate,
        order=order_for(rows, cores, replicate),
        role_cap_seconds=ROLE_CAP_SECONDS[rows],
        maxiter=MAXITER[rows],
        memory_gib=MEMORY_GIB[rows],
    )


def gate_task() -> Task:
    return Task(
        schema=SCHEMA,
        stage="gate",
        task_id=1,
        rows=6_000,
        workers=1_000,
        firms=500,
        cores=4,
        replicate=0,
        probes=PROBES[6_000],
        seed=20_260_830,
        order="rust_matlab",
        role_cap_seconds=7_200,
        maxiter=MAXITER[6_000],
        memory_gib=16,
    )


def feasibility_tasks(rows: int) -> list[Task]:
    if rows not in ROWS:
        raise ValueError("unregistered feasibility size")
    stage = f"feasibility-{rows}"
    return [task(rows, cores, 1, index, stage) for index, cores in enumerate(CORES, 1)]


def topup_tasks(cells: list[tuple[int, int]]) -> list[Task]:
    values: list[Task] = []
    for rows, cores in sorted(cells):
        for replicate in (2, 3):
            values.append(task(
                rows, cores, replicate, len(values) + 1, f"topup-{rows}"
            ))
    return values


TASK_FIELDS = [*Task.__dataclass_fields__, "task_sha256"]


def write_manifest(path: Path, tasks: list[Task]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=TASK_FIELDS, delimiter="\t", lineterminator="\n")
        writer.writeheader()
        for value in tasks:
            writer.writerow({**asdict(value), "task_sha256": value.task_sha256})


def read_manifest(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as handle:
        values = list(csv.DictReader(handle, delimiter="\t"))
    if not values or list(values[0]) != TASK_FIELDS:
        raise ValueError("manifest schema changed")
    for index, value in enumerate(values, 1):
        parsed = gate_task() if value["stage"] == "gate" else task(
            int(value["rows"]), int(value["cores"]), int(value["replicate"]),
            index, value["stage"],
        )
        expected = {key: str(item) for key, item in asdict(parsed).items()}
        expected["task_sha256"] = parsed.task_sha256
        if value != expected:
            raise ValueError(f"manifest task {index} changed")
    return values


def expected_probes(rows: int) -> int:
    registered = PROBES.get(rows)
    if registered is None:
        raise ValueError("unregistered row count")
    if rows != 6_000 and registered != math.ceil(math.log2(rows) / 0.01):
        raise AssertionError("registered probe rule drifted")
    return registered

from __future__ import annotations

import collections
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[4]
BENCHMARK = ROOT / "vckss" / "benchmarks" / "comparative_scaling"
sys.path.insert(0, str(BENCHMARK))

from build_manifest import build_rows  # noqa: E402
from common import (  # noqa: E402
    CORE_GRID,
    ESTIMATORS,
    REPLICATES,
    ROW_GRID,
    STRUCTURES,
    TASK_FIELDS,
    EvidenceError,
    execution_roles,
    read_manifest,
    write_tsv,
)

SOURCE = "1" * 40
BUNDLE = "2" * 64


def test_registered_manifest_is_complete_and_position_balanced(tmp_path: Path) -> None:
    rows = build_rows(SOURCE, BUNDLE, mem_per_core_gib=8, command_memory_gib=112)
    assert len(rows) == 300
    assert [row["task_id"] for row in rows] == list(range(1, 301))
    assert len({row["experiment_id"] for row in rows}) == 300
    cells: dict[tuple[str, int, int], list[dict[str, object]]] = (
        collections.defaultdict(list)
    )
    for row in rows:
        target = int(row["active_cores"])
        assert int(row["stata_processors"]) == min(target, 4)
        assert int(row["mata_active_cores"]) == min(target, 4)
        assert int(row["rust_threads"]) == target
        assert int(row["matlab_workers"]) == target
        cells[(str(row["structure"]), int(row["rows"]),
               int(row["active_cores"]))].append(row)
    assert len(cells) == len(STRUCTURES) * len(ROW_GRID) * len(CORE_GRID)
    for repetitions in cells.values():
        assert [row["replicate"] for row in repetitions] == [1, 2, 3]
        orders = [execution_roles(str(row["execution_order"]))
                  for row in repetitions]
        for position in range(3):
            assert {order[position] for order in orders} == set(ESTIMATORS)


def test_manifest_round_trip_and_scheduler_backed_memory_variants(
        tmp_path: Path) -> None:
    for mem_per_core, command_memory in ((4, 48), (8, 112), (12, 176)):
        path = tmp_path / f"tasks-{mem_per_core}.tsv"
        rows = build_rows(
            SOURCE,
            BUNDLE,
            mem_per_core_gib=mem_per_core,
            command_memory_gib=command_memory,
        )
        write_tsv(path, TASK_FIELDS, rows)
        loaded = read_manifest(path)
        assert len(loaded) == 300
        assert loaded[0]["seed"] == str(REPLICATES[0][1])
        assert loaded[-1]["mem_per_core_gib"] == str(mem_per_core)
        assert loaded[-1]["command_memory_gib"] == str(command_memory)


def test_dimensions_are_exact_for_every_graph_and_size() -> None:
    rows = build_rows(SOURCE, BUNDLE, mem_per_core_gib=8, command_memory_gib=112)
    for row in rows:
        degree = int(row["cells_per_worker"])
        assert int(row["rows"]) == degree * int(row["workers"])
        if row["structure"] == "weak_d3":
            assert int(row["workers"]) % 5 == 0
            assert int(row["firms"]) == int(row["workers"]) // 5 + 1_601
        else:
            assert int(row["workers"]) == 40 * int(row["firms"])


def test_command_memory_must_fit_scheduler_allocation() -> None:
    with pytest.raises(EvidenceError, match="fit the scheduler allocation"):
        build_rows(
            SOURCE,
            BUNDLE,
            mem_per_core_gib=8,
            command_memory_gib=129,
        )

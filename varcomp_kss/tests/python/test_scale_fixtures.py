from __future__ import annotations

from collections import defaultdict, deque
from itertools import combinations
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
MATA = ROOT / "benchmarks" / "kss_scale_fixtures.mata"
STATA = ROOT / "benchmarks" / "kss_scale_fixtures.do"


Row = tuple[int, int, int, int]


def _base() -> list[Row]:
    # worker, firm, deletion unit, frequency.  Unit 901 has repeated rows and
    # unit 905 shares its coefficient cell, so cells and units are distinct.
    return [
        (101, 501, 901, 1),
        (101, 501, 901, 2),
        (101, 501, 905, 1),
        (101, 503, 902, 1),
        (107, 501, 903, 1),
        (107, 503, 904, 1),
    ]


def _dense(values: list[int]) -> dict[int, int]:
    return {value: index + 1 for index, value in enumerate(sorted(set(values)))}


def _pairs(design: str, copies: int) -> list[tuple[int, int]]:
    if design in {"replicated_blocks", "well_connected"}:
        return list(combinations(range(1, copies + 1), 2))
    if copies == 2:
        return [(1, 2)]
    return [(left, left + 1) for left in range(1, copies)] + [(1, copies)]


def _replicate(base: list[Row], design: str, copies: int) -> list[Row]:
    workers = _dense([row[0] for row in base])
    firms = _dense([row[1] for row in base])
    units = _dense([row[2] for row in base])
    output: list[Row] = []
    for copy in range(copies):
        for worker, firm, unit, frequency in base:
            output.append(
                (
                    workers[worker] + copy * len(workers),
                    firms[firm] + copy * len(firms),
                    units[unit] + copy * len(units),
                    frequency,
                )
            )
    for pair, (left, right) in enumerate(_pairs(design, copies)):
        first_worker = copies * len(workers) + 2 * pair + 1
        first_unit = copies * len(units) + 4 * pair + 1
        left_firm = (left - 1) * len(firms) + 1
        right_firm = (right - 1) * len(firms) + 1
        output.extend(
            [
                (first_worker, left_firm, first_unit, 1),
                (first_worker, right_firm, first_unit + 1, 1),
                (first_worker + 1, left_firm, first_unit + 2, 1),
                (first_worker + 1, right_firm, first_unit + 3, 1),
            ]
        )
    return output


def _edges(rows: list[Row]) -> dict[int, tuple[tuple[str, int], tuple[str, int]]]:
    by_unit: dict[int, set[tuple[int, int]]] = defaultdict(set)
    for worker, firm, unit, _ in rows:
        by_unit[unit].add((worker, firm))
    assert all(len(cells) == 1 for cells in by_unit.values())
    return {
        unit: (("w", next(iter(cells))[0]), ("f", next(iter(cells))[1]))
        for unit, cells in by_unit.items()
    }


def _components(
    edges: dict[int, tuple[tuple[str, int], tuple[str, int]]],
    omitted: int | None = None,
) -> int:
    adjacency: dict[tuple[str, int], set[tuple[str, int]]] = defaultdict(set)
    nodes: set[tuple[str, int]] = set()
    for unit, (left, right) in edges.items():
        nodes.update((left, right))
        if unit == omitted:
            continue
        adjacency[left].add(right)
        adjacency[right].add(left)
    unseen = set(nodes)
    count = 0
    while unseen:
        count += 1
        queue = deque([unseen.pop()])
        while queue:
            for neighbor in adjacency[queue.popleft()]:
                if neighbor in unseen:
                    unseen.remove(neighbor)
                    queue.append(neighbor)
    return count


def _assert_deletion_safe(rows: list[Row]) -> None:
    edges = _edges(rows)
    assert _components(edges) == 1
    assert all(_components(edges, omitted=unit) == 1 for unit in edges)


def _counts(rows: list[Row]) -> tuple[int, int, int, int, int, int]:
    return (
        len(rows),
        sum(row[3] for row in rows),
        len({row[0] for row in rows}),
        len({row[1] for row in rows}),
        len({row[:2] for row in rows}),
        len({row[2] for row in rows}),
    )


def _meta_spectrum(design: str, copies: int) -> tuple[float, float, float]:
    adjacency = np.zeros((copies, copies))
    for left, right in _pairs(design, copies):
        adjacency[left - 1, right - 1] = 2
        adjacency[right - 1, left - 1] = 2
    degree = adjacency.sum(axis=1)
    normalized = np.eye(copies) - (
        adjacency / np.sqrt(np.outer(degree, degree))
    )
    values = np.linalg.eigvalsh(normalized)
    positive = values[values > 1e-12]
    return float(positive[0]), float(positive[-1]), float(positive[-1] / positive[0])


def test_replicated_blocks_fixture_has_exact_dimensions_and_no_bridges() -> None:
    rows = _replicate(_base(), "replicated_blocks", 4)
    assert _counts(rows) == (48, 52, 20, 8, 40, 44)
    _assert_deletion_safe(rows)
    lambda2, lambda_max, condition = _meta_spectrum("replicated_blocks", 4)
    assert np.isclose(lambda2, 4 / 3)
    assert np.isclose(lambda_max, 4 / 3)
    assert np.isclose(condition, 1)


def test_ring_is_separate_deletion_safe_adverse_connectivity_fixture() -> None:
    rows = _replicate(_base(), "ring", 4)
    assert _counts(rows) == (40, 44, 16, 8, 32, 36)
    _assert_deletion_safe(rows)
    lambda2_4, _, condition_4 = _meta_spectrum("ring", 4)
    lambda2_8, _, condition_8 = _meta_spectrum("ring", 8)
    assert np.isclose(lambda2_4, 1)
    assert np.isclose(condition_4, 2)
    assert lambda2_8 < lambda2_4
    assert condition_8 > condition_4


def test_cell_and_deletion_unit_counts_are_independent() -> None:
    base = _base()
    assert len({row[:2] for row in base}) == 4
    assert len({row[2] for row in base}) == 5
    for design in ("replicated_blocks", "ring"):
        rows = _replicate(base, design, 4)
        assert len({row[2] for row in rows}) > len({row[:2] for row in rows})


def test_stata_fixture_api_records_required_certificates() -> None:
    mata = MATA.read_text(encoding="utf-8")
    stata = STATA.read_text(encoding="utf-8")
    for token in (
        "CROSS_CELL_DELETION_UNIT",
        "bridge_units",
        "connector_meta_conductance",
        "connector_meta_condition_proxy",
        "connector_volume_ratio",
        "minimum_weighted_degree",
        "maximum_weighted_degree",
    ):
        assert token in mata + stata
    assert 'design == "replicated_blocks" | design == "well_connected"' in mata
    assert "if \"`design'\" == \"well_connected\" local design replicated_blocks" in stata
    assert "return local design \"`design'\"" in stata
    assert 'design == "ring"' in mata
    assert "expected_cells" in stata
    assert "expected_deletion_units" in stata
    assert "base_cells = el(`base_diagnostics',1,5)" in stata
    assert "base_units = el(`base_diagnostics',1,6)" in stata
    assert "base_cells = `base_units'" not in stata
    assert "vckss_scale_fixture__api_level" in mata + stata
    assert "real scalar vckss_scale__api_level()" not in mata

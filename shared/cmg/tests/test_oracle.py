from __future__ import annotations

from fractions import Fraction
from itertools import product

import numpy as np
import pytest

from shared.cmg.oracle.cmg_oracle import (
    CMGError,
    HierarchyOptions,
    apply_vcycle,
    build_hierarchy,
    build_hybrid_graph,
    collapse_cells,
    component_projector,
    dense_schur,
    grounded_pullback,
    hybrid_firm_schur,
    kss_pullback,
    laplacian,
    materialize_vcycle,
)


def _cells(worker: list[int], firm: list[int], weight: list[float]):
    return collapse_cells(worker, firm, weight)


def _path_cells(firms: int):
    worker: list[int] = []
    firm: list[int] = []
    weight: list[float] = []
    for edge in range(firms - 1):
        worker.extend([edge, edge])
        firm.extend([edge, edge + 1])
        weight.extend([1.0, 1.0])
    return _cells(worker, firm, weight)


def test_duplicate_cells_are_collapsed_deterministically() -> None:
    first = collapse_cells([9, 9, 1, 9], [2, 2, 8, 4], [0.1, 0.2, 2.0, 1.0])
    second = collapse_cells([9, 1, 9, 9], [4, 8, 2, 2], [1.0, 2.0, 0.2, 0.1])
    np.testing.assert_array_equal(first.worker, second.worker)
    np.testing.assert_array_equal(first.firm, second.firm)
    np.testing.assert_allclose(first.weight, second.weight, rtol=0, atol=0)


@pytest.mark.parametrize(
    ("worker", "firm", "weight"),
    [
        ([0], [0], [1.0]),
        ([0, 0], [0, 1], [2.0, 3.0]),
        ([0, 0, 0], [0, 1, 2], [1.0, 2.0, 5.0]),
        ([0, 0, 0, 0], [0, 1, 2, 3], [1.0, 2.0, 3.0, 4.0]),
        ([0, 0, 1, 1, 2, 2], [0, 1, 1, 2, 0, 2], [1, 2, 3, 4, 5, 6]),
    ],
)
def test_hybrid_schur_is_exact(worker, firm, weight) -> None:
    cells = collapse_cells(worker, firm, weight)
    graph = build_hybrid_graph(cells)
    np.testing.assert_allclose(hybrid_firm_schur(graph), dense_schur(cells), rtol=2e-13, atol=2e-13)


def test_hub_never_expands_to_a_clique() -> None:
    firms = 500
    cells = collapse_cells([0] * firms, list(range(firms)), np.linspace(1.0, 2.0, firms))
    graph = build_hybrid_graph(cells)
    assert graph.edge_count == firms
    assert graph.n_vertex == firms + 1
    assert graph.edge_count < firms * (firms - 1) // 2


def test_small_incidence_grid_matches_dense_schur() -> None:
    # Exhaust the nonempty two-worker/three-firm incidence matrices that use
    # every worker and firm.  Vary each retained cell over a rational grid.
    locations = list(product(range(2), range(3)))
    checked = 0
    for mask in range(1, 1 << len(locations)):
        chosen = [locations[i] for i in range(len(locations)) if mask & (1 << i)]
        if {w for w, _ in chosen} != {0, 1} or {f for _, f in chosen} != {0, 1, 2}:
            continue
        for pivot in range(len(chosen)):
            weights = [1.0] * len(chosen)
            weights[pivot] = 0.5
            cells = collapse_cells(
                [row[0] for row in chosen],
                [row[1] for row in chosen],
                weights,
            )
            graph = build_hybrid_graph(cells)
            np.testing.assert_allclose(
                hybrid_firm_schur(graph), dense_schur(cells), rtol=2e-12, atol=2e-12
            )
            checked += 1
    assert checked > 100


def _connected_bipartite_mask(mask: int, workers: int, firms: int) -> bool:
    adjacency = [set() for _ in range(workers + firms)]
    for worker in range(workers):
        for firm in range(firms):
            bit = worker * firms + firm
            if mask & (1 << bit):
                adjacency[worker].add(workers + firm)
                adjacency[workers + firm].add(worker)
    if any(not neighbors for neighbors in adjacency):
        return False
    reached = {0}
    stack = [0]
    while stack:
        node = stack.pop()
        for neighbor in adjacency[node]:
            if neighbor not in reached:
                reached.add(neighbor)
                stack.append(neighbor)
    return len(reached) == workers + firms


def _exact_schur(
    worker: list[int], firm: list[int], weight: list[Fraction], n_firm: int
) -> np.ndarray:
    matrix = [[Fraction(0) for _ in range(n_firm)] for _ in range(n_firm)]
    for one_worker in sorted(set(worker)):
        positions = [index for index, value in enumerate(worker) if value == one_worker]
        mass = sum((weight[index] for index in positions), start=Fraction(0))
        for left in positions:
            matrix[firm[left]][firm[left]] += weight[left]
            for right in positions:
                matrix[firm[left]][firm[right]] -= weight[left] * weight[right] / mass
    return np.asarray([[float(value) for value in row] for row in matrix])


def test_all_connected_incidence_masks_through_three_workers_four_firms() -> None:
    checked = 0
    for workers in range(1, 4):
        for firms in range(1, 5):
            for mask in range(1, 1 << (workers * firms)):
                if not _connected_bipartite_mask(mask, workers, firms):
                    continue
                locations = [
                    (worker, firm)
                    for worker in range(workers)
                    for firm in range(firms)
                    if mask & (1 << (worker * firms + firm))
                ]
                rational_cases = [[Fraction(1)] * len(locations)]
                for pivot in range(len(locations)):
                    perturbed = [Fraction(1)] * len(locations)
                    perturbed[pivot] = Fraction(1, 2)
                    rational_cases.append(perturbed)
                for rational_weight in rational_cases:
                    worker = [location[0] for location in locations]
                    firm = [location[1] for location in locations]
                    cells = collapse_cells(
                        worker, firm, [float(value) for value in rational_weight]
                    )
                    graph = build_hybrid_graph(cells)
                    expected = _exact_schur(worker, firm, rational_weight, firms)
                    np.testing.assert_allclose(
                        hybrid_firm_schur(graph),
                        expected,
                        rtol=2e-13,
                        atol=2e-13,
                    )
                    checked += 1
    assert checked >= 10_000


def test_random_hybrid_equivalence() -> None:
    rng = np.random.default_rng(20260814)
    for _ in range(10_000):
        workers = int(rng.integers(1, 6))
        firms = int(rng.integers(1, 7))
        count = int(rng.integers(max(workers, firms), 11))
        worker = rng.integers(0, workers, count)
        firm = rng.integers(0, firms, count)
        # Ensure every declared level appears.
        worker[:workers] = np.arange(workers)
        firm[:firms] = np.arange(firms)
        weights = rng.choice(np.array([0.125, 0.5, 1.0, 3.0, 8.0]), count)
        cells = collapse_cells(worker, firm, weights)
        graph = build_hybrid_graph(cells)
        np.testing.assert_allclose(
            hybrid_firm_schur(graph), dense_schur(cells), rtol=2e-12, atol=2e-12
        )


def test_hierarchy_is_galerkin_and_component_preserving() -> None:
    cells = _path_cells(300)
    graph = build_hybrid_graph(cells)
    hierarchy = build_hierarchy(graph, HierarchyOptions(coarse_max=16))
    assert len(hierarchy.levels) > 1
    assert hierarchy.edge_complexity <= 3.0
    assert hierarchy.vertex_complexity <= 4.0
    for fine, coarse in zip(hierarchy.levels[:-1], hierarchy.levels[1:], strict=True):
        assert fine.prolongation is not None
        np.testing.assert_allclose(
            fine.prolongation.T @ laplacian(fine.graph) @ fine.prolongation,
            laplacian(coarse.graph),
            rtol=2e-13,
            atol=2e-13,
        )
        assert len(np.unique(fine.component)) == len(np.unique(coarse.component))


def test_vcycle_is_linear_symmetric_and_positive_on_quotient() -> None:
    graph = build_hybrid_graph(_path_cells(40))
    hierarchy = build_hierarchy(graph, HierarchyOptions(coarse_max=8))
    cycle = materialize_vcycle(hierarchy)
    np.testing.assert_allclose(cycle, cycle.T, rtol=1e-12, atol=1e-12)
    projector = component_projector(hierarchy.levels[0].component)
    basis = np.linalg.qr(projector[:, :-1])[0][:, : graph.n_vertex - 1]
    eigenvalues = np.linalg.eigvalsh(basis.T @ cycle @ basis)
    assert np.min(eigenvalues) > 0

    rng = np.random.default_rng(11)
    r = projector @ rng.normal(size=graph.n_vertex)
    s = projector @ rng.normal(size=graph.n_vertex)
    left = apply_vcycle(hierarchy, 1.25 * r - 0.75 * s)
    right = 1.25 * apply_vcycle(hierarchy, r) - 0.75 * apply_vcycle(hierarchy, s)
    np.testing.assert_allclose(left, right, rtol=5e-13, atol=5e-13)


def test_kss_pullback_is_symmetric_positive() -> None:
    cells = collapse_cells(
        [0, 0, 1, 1, 2, 2, 3, 3, 3, 3],
        [0, 1, 1, 2, 2, 3, 0, 1, 2, 3],
        [1, 2, 2, 3, 3, 4, 1, 1, 1, 1],
    )
    hierarchy = build_hierarchy(build_hybrid_graph(cells), HierarchyOptions(coarse_max=2))
    preconditioner = kss_pullback(hierarchy, cells.n_firm)
    np.testing.assert_allclose(preconditioner, preconditioner.T, rtol=1e-12, atol=1e-12)
    projector = np.eye(cells.n_firm) - np.ones((cells.n_firm, cells.n_firm)) / cells.n_firm
    basis = np.linalg.qr(projector[:, :-1])[0][:, : cells.n_firm - 1]
    assert np.min(np.linalg.eigvalsh(basis.T @ preconditioner @ basis)) > 0


def test_grounded_pullback_matches_grounded_inverse_and_is_symmetric() -> None:
    cells = _path_cells(3)
    hierarchy = build_hierarchy(build_hybrid_graph(cells))
    schur = dense_schur(cells)
    expected = np.linalg.inv(schur[:2, :2])
    actual = grounded_pullback(hierarchy, 3, 2)
    np.testing.assert_allclose(actual, expected, rtol=2e-12, atol=2e-12)
    np.testing.assert_allclose(actual, actual.T, rtol=0, atol=2e-13)

    # The forbidden extraction after balancing is nonsymmetric on this path.
    cycle = np.linalg.pinv(schur)
    balancing = np.array([[1.0, 0.0], [0.0, 1.0], [-1.0, -1.0]])
    extracted = cycle[:2, :] @ balancing
    assert not np.allclose(extracted, extracted.T, rtol=1e-12, atol=1e-12)


def test_invalid_weights_fail_closed() -> None:
    for bad in (0.0, -1.0, np.nan, np.inf):
        with pytest.raises(CMGError):
            collapse_cells([0], [0], [bad])

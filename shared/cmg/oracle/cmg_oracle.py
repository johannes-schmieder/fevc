"""Independent dense oracle for the clean-room CMG-inspired design.

This module is intentionally written for clarity and falsification, not as a
runtime implementation.  It may form dense matrices and use NumPy linear
algebra.  Production Mata code must not call it and must not share helper code
with it.
"""

from __future__ import annotations

from dataclasses import dataclass
from itertools import combinations
from math import fsum, isfinite
from typing import Iterable, Sequence

import numpy as np


class CMGError(ValueError):
    """Typed oracle failure with the production-style status attached."""

    def __init__(self, status: str, message: str) -> None:
        super().__init__(f"{status}: {message}")
        self.status = status
        self.message = message


VertexKey = tuple[int, int]


@dataclass(frozen=True)
class Cells:
    worker: np.ndarray
    firm: np.ndarray
    weight: np.ndarray
    n_worker: int
    n_firm: int
    worker_key: tuple[int, ...]
    firm_key: tuple[int, ...]

    @property
    def count(self) -> int:
        return int(self.weight.size)


@dataclass(frozen=True)
class HybridGraph:
    n_firm: int
    n_vertex: int
    u: np.ndarray
    v: np.ndarray
    weight: np.ndarray
    vertex_key: tuple[VertexKey, ...]
    auxiliary_worker: tuple[int, ...]

    @property
    def edge_count(self) -> int:
        return int(self.weight.size)


@dataclass(frozen=True)
class HierarchyOptions:
    kappa: float = 8.0
    target_size: int = 4
    aggregate_cap: int = 8
    min_reduction: float = 0.20
    max_edge_complexity: float = 3.0
    max_vertex_complexity: float = 4.0
    max_levels: int = 32
    coarse_max: int = 128
    omega: float = 2.0 / 3.0


@dataclass(frozen=True)
class Level:
    graph: HybridGraph
    component: np.ndarray
    aggregate: np.ndarray | None
    prolongation: np.ndarray | None


@dataclass(frozen=True)
class Hierarchy:
    levels: tuple[Level, ...]
    options: HierarchyOptions

    @property
    def edge_complexity(self) -> float:
        base = max(self.levels[0].graph.edge_count, 1)
        return sum(level.graph.edge_count for level in self.levels) / base

    @property
    def vertex_complexity(self) -> float:
        base = max(self.levels[0].graph.n_vertex, 1)
        return sum(level.graph.n_vertex for level in self.levels) / base


def _as_int_vector(name: str, values: Sequence[int] | np.ndarray) -> np.ndarray:
    out = np.asarray(values)
    if out.ndim != 1:
        raise CMGError("INVALID_INPUT", f"{name} must be one-dimensional")
    if not np.issubdtype(out.dtype, np.integer):
        if not np.all(np.isfinite(out)) or not np.all(out == np.floor(out)):
            raise CMGError("INVALID_IDENTIFIER", f"{name} must contain integers")
    return out.astype(np.int64, copy=False)


def _dense_codes(values: np.ndarray) -> tuple[np.ndarray, tuple[int, ...]]:
    labels = tuple(int(x) for x in np.unique(values))
    mapping = {label: i for i, label in enumerate(labels)}
    return np.fromiter((mapping[int(x)] for x in values), dtype=np.int64), labels


def collapse_cells(
    worker: Sequence[int] | np.ndarray,
    firm: Sequence[int] | np.ndarray,
    weight: Sequence[float] | np.ndarray,
    *,
    worker_key: Sequence[int] | None = None,
    firm_key: Sequence[int] | None = None,
) -> Cells:
    """Collapse duplicate worker--firm rows with deterministic ``fsum``."""

    raw_worker = _as_int_vector("worker", worker)
    raw_firm = _as_int_vector("firm", firm)
    raw_weight = np.asarray(weight, dtype=np.float64)
    if raw_weight.ndim != 1:
        raise CMGError("INVALID_INPUT", "weight must be one-dimensional")
    if not (raw_worker.size == raw_firm.size == raw_weight.size):
        raise CMGError("INVALID_INPUT", "worker, firm, and weight lengths differ")
    if raw_weight.size == 0:
        raise CMGError("INVALID_INPUT", "at least one cell is required")
    if not np.all(np.isfinite(raw_weight)) or not np.all(raw_weight > 0):
        raise CMGError("INVALID_WEIGHT", "all weights must be finite and positive")

    worker_code, worker_labels = _dense_codes(raw_worker)
    firm_code, firm_labels = _dense_codes(raw_firm)
    order = np.lexsort((np.arange(raw_weight.size), firm_code, worker_code))
    worker_code = worker_code[order]
    firm_code = firm_code[order]
    raw_weight = raw_weight[order]

    out_w: list[int] = []
    out_f: list[int] = []
    out_a: list[float] = []
    start = 0
    while start < raw_weight.size:
        stop = start + 1
        while (
            stop < raw_weight.size
            and worker_code[stop] == worker_code[start]
            and firm_code[stop] == firm_code[start]
        ):
            stop += 1
        total = fsum(float(x) for x in raw_weight[start:stop])
        if not isfinite(total) or total <= 0:
            raise CMGError("NONFINITE_CELL_SUM", "collapsed cell weight is invalid")
        out_w.append(int(worker_code[start]))
        out_f.append(int(firm_code[start]))
        out_a.append(total)
        start = stop

    n_worker = len(worker_labels)
    n_firm = len(firm_labels)
    if worker_key is None:
        worker_key_tuple = tuple(range(n_worker))
    else:
        if len(worker_key) != n_worker or len(set(worker_key)) != n_worker:
            raise CMGError("INVALID_CANONICAL_KEY", "worker keys must be unique")
        worker_key_tuple = tuple(int(x) for x in worker_key)
    if firm_key is None:
        firm_key_tuple = tuple(range(n_firm))
    else:
        if len(firm_key) != n_firm or len(set(firm_key)) != n_firm:
            raise CMGError("INVALID_CANONICAL_KEY", "firm keys must be unique")
        firm_key_tuple = tuple(int(x) for x in firm_key)

    return Cells(
        worker=np.asarray(out_w, dtype=np.int64),
        firm=np.asarray(out_f, dtype=np.int64),
        weight=np.asarray(out_a, dtype=np.float64),
        n_worker=n_worker,
        n_firm=n_firm,
        worker_key=worker_key_tuple,
        firm_key=firm_key_tuple,
    )


def dense_schur(cells: Cells) -> np.ndarray:
    """Materialize the worker-eliminated firm Schur complement."""

    out = np.zeros((cells.n_firm, cells.n_firm), dtype=np.float64)
    for worker in range(cells.n_worker):
        take = cells.worker == worker
        firms = cells.firm[take]
        a = cells.weight[take]
        mass = fsum(float(x) for x in a)
        if not isfinite(mass) or mass <= 0:
            raise CMGError("NONFINITE_WORKER_MASS", "worker mass is invalid")
        out[firms, firms] += a
        out[np.ix_(firms, firms)] -= np.outer(a, a) / mass
    return (out + out.T) / 2.0


def _collapse_edges(
    raw: Iterable[tuple[int, int, float, tuple[int, ...]]],
) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    canonical: list[tuple[int, int, float, tuple[int, ...]]] = []
    for u, v, weight, contributor in raw:
        if u == v:
            continue
        if not isfinite(weight) or weight <= 0:
            raise CMGError("NONFINITE_EDGE", "edge weight is not finite and positive")
        if v < u:
            u, v = v, u
        canonical.append((u, v, float(weight), contributor))
    canonical.sort(key=lambda row: (row[0], row[1], row[3]))
    out_u: list[int] = []
    out_v: list[int] = []
    out_w: list[float] = []
    start = 0
    while start < len(canonical):
        stop = start + 1
        while (
            stop < len(canonical)
            and canonical[stop][0] == canonical[start][0]
            and canonical[stop][1] == canonical[start][1]
        ):
            stop += 1
        total = fsum(row[2] for row in canonical[start:stop])
        if not isfinite(total) or total <= 0:
            raise CMGError("NONFINITE_EDGE_SUM", "collapsed edge weight is invalid")
        out_u.append(canonical[start][0])
        out_v.append(canonical[start][1])
        out_w.append(total)
        start = stop
    return (
        np.asarray(out_u, dtype=np.int64),
        np.asarray(out_v, dtype=np.int64),
        np.asarray(out_w, dtype=np.float64),
    )


def build_hybrid_graph(cells: Cells) -> HybridGraph:
    """Build the exact degree-three hybrid graph."""

    vertex_key: list[VertexKey] = [(cells.firm_key[f], 0) for f in range(cells.n_firm)]
    auxiliary_worker: list[int] = []
    raw_edges: list[tuple[int, int, float, tuple[int, ...]]] = []
    for worker in range(cells.n_worker):
        take = cells.worker == worker
        firms = cells.firm[take]
        a = cells.weight[take]
        degree = int(firms.size)
        mass = fsum(float(x) for x in a)
        if degree == 1:
            continue
        if degree <= 3:
            for left, right in combinations(range(degree), 2):
                conductance = (float(a[left]) / mass) * float(a[right])
                raw_edges.append(
                    (
                        int(firms[left]),
                        int(firms[right]),
                        conductance,
                        (cells.worker_key[worker], left, right),
                    )
                )
            continue
        aux = len(vertex_key)
        auxiliary_worker.append(worker)
        vertex_key.append((cells.worker_key[worker], 1))
        for position, (firm, conductance) in enumerate(zip(firms, a, strict=True)):
            raw_edges.append(
                (
                    int(firm),
                    aux,
                    float(conductance),
                    (cells.worker_key[worker], position),
                )
            )

    u, v, edge_weight = _collapse_edges(raw_edges)
    graph = HybridGraph(
        n_firm=cells.n_firm,
        n_vertex=len(vertex_key),
        u=u,
        v=v,
        weight=edge_weight,
        vertex_key=tuple(vertex_key),
        auxiliary_worker=tuple(auxiliary_worker),
    )
    if graph.edge_count > cells.count:
        raise CMGError("HYBRID_EDGE_BOUND", "hybrid edge count exceeds cell count")
    if graph.n_vertex > cells.n_firm + cells.count // 4:
        raise CMGError("HYBRID_VERTEX_BOUND", "hybrid vertex count exceeds bound")
    return graph


def laplacian(graph: HybridGraph) -> np.ndarray:
    out = np.zeros((graph.n_vertex, graph.n_vertex), dtype=np.float64)
    for u, v, weight in zip(graph.u, graph.v, graph.weight, strict=True):
        out[u, u] += weight
        out[v, v] += weight
        out[u, v] -= weight
        out[v, u] -= weight
    return (out + out.T) / 2.0


def hybrid_firm_schur(graph: HybridGraph) -> np.ndarray:
    """Eliminate hybrid auxiliary vertices with a dense oracle solve."""

    matrix = laplacian(graph)
    ff = matrix[: graph.n_firm, : graph.n_firm]
    if graph.n_vertex == graph.n_firm:
        return ff
    fa = matrix[: graph.n_firm, graph.n_firm :]
    aa = matrix[graph.n_firm :, graph.n_firm :]
    # Every retained auxiliary worker has strictly positive diagonal and no
    # auxiliary--auxiliary edge, so this inverse is diagonal and exact in form.
    if np.any(np.diag(aa) <= 0) or np.count_nonzero(aa - np.diag(np.diag(aa))):
        raise CMGError("INVALID_AUXILIARY_BLOCK", "hybrid auxiliary block is invalid")
    return ff - (fa / np.diag(aa)) @ fa.T


def _components(graph: HybridGraph) -> np.ndarray:
    adjacency: list[list[int]] = [[] for _ in range(graph.n_vertex)]
    for u, v in zip(graph.u, graph.v, strict=True):
        adjacency[int(u)].append(int(v))
        adjacency[int(v)].append(int(u))
    component = np.full(graph.n_vertex, -1, dtype=np.int64)
    count = 0
    for seed in sorted(range(graph.n_vertex), key=lambda x: graph.vertex_key[x]):
        if component[seed] >= 0:
            continue
        stack = [seed]
        component[seed] = count
        while stack:
            vertex = stack.pop()
            for neighbor in adjacency[vertex]:
                if component[neighbor] < 0:
                    component[neighbor] = count
                    stack.append(neighbor)
        count += 1
    return component


def component_projector(component: Sequence[int] | np.ndarray) -> np.ndarray:
    labels = _as_int_vector("component", component)
    n = labels.size
    projector = np.eye(n, dtype=np.float64)
    for value in np.unique(labels):
        index = np.flatnonzero(labels == value)
        projector[np.ix_(index, index)] -= 1.0 / index.size
    return projector


def _edge_priority(graph: HybridGraph, edge: int) -> tuple[float, VertexKey, VertexKey]:
    u = int(graph.u[edge])
    v = int(graph.v[edge])
    ku, kv = graph.vertex_key[u], graph.vertex_key[v]
    if kv < ku:
        ku, kv = kv, ku
    # Sorting this tuple ascending selects the strict key described in the
    # contract: descending weight and then ascending endpoints.
    return (-float(graph.weight[edge]), ku, kv)


def _selected_forest(graph: HybridGraph, component: np.ndarray, kappa: float) -> list[int]:
    incident: list[list[int]] = [[] for _ in range(graph.n_vertex)]
    degree = np.zeros(graph.n_vertex, dtype=np.float64)
    maximum = np.zeros(graph.n_vertex, dtype=np.float64)
    for edge, (u, v, weight) in enumerate(zip(graph.u, graph.v, graph.weight, strict=True)):
        incident[int(u)].append(edge)
        incident[int(v)].append(edge)
        degree[int(u)] += weight
        degree[int(v)] += weight
        maximum[int(u)] = max(maximum[int(u)], weight)
        maximum[int(v)] = max(maximum[int(v)], weight)
    weighted_degree = np.ones(graph.n_vertex, dtype=np.float64)
    positive = maximum > 0
    weighted_degree[positive] = degree[positive] / maximum[positive]
    awd = float(np.mean(weighted_degree))

    nominated = np.full(graph.n_vertex, -1, dtype=np.int64)
    for vertex in range(graph.n_vertex):
        if incident[vertex]:
            nominated[vertex] = min(incident[vertex], key=lambda edge: _edge_priority(graph, edge))
    selected = {int(edge) for edge in nominated if edge >= 0}
    forest_volume = np.zeros(graph.n_vertex, dtype=np.float64)
    for edge in selected:
        u, v, weight = int(graph.u[edge]), int(graph.v[edge]), graph.weight[edge]
        forest_volume[u] += weight
        forest_volume[v] += weight
    removed: set[int] = set()
    for vertex, edge in enumerate(nominated):
        if edge < 0:
            continue
        if weighted_degree[vertex] > kappa * awd and forest_volume[vertex] < degree[vertex] / awd:
            removed.add(int(edge))
    selected.difference_update(removed)

    # The strict nomination construction is a forest.  Check it explicitly so
    # a tie/order mistake cannot silently enter the hierarchy.
    parent = list(range(graph.n_vertex))

    def find(vertex: int) -> int:
        while parent[vertex] != vertex:
            parent[vertex] = parent[parent[vertex]]
            vertex = parent[vertex]
        return vertex

    for edge in sorted(selected, key=lambda e: _edge_priority(graph, e)):
        u, v = int(graph.u[edge]), int(graph.v[edge])
        ru, rv = find(u), find(v)
        if ru == rv:
            raise CMGError("FOREST_CYCLE", "nominated edges contain a cycle")
        parent[rv] = ru
        if component[u] != component[v]:
            raise CMGError("COMPONENT_MERGE", "forest edge crosses components")
    return sorted(selected, key=lambda edge: _edge_priority(graph, edge))


def _forest_components(vertices: set[int], forest_edges: list[int], graph: HybridGraph) -> list[set[int]]:
    adjacency = {vertex: [] for vertex in vertices}
    for edge in forest_edges:
        u, v = int(graph.u[edge]), int(graph.v[edge])
        if u in vertices and v in vertices:
            adjacency[u].append(v)
            adjacency[v].append(u)
    out: list[set[int]] = []
    unseen = set(vertices)
    while unseen:
        seed = min(unseen, key=lambda vertex: graph.vertex_key[vertex])
        stack = [seed]
        part = {seed}
        unseen.remove(seed)
        while stack:
            vertex = stack.pop()
            for neighbor in adjacency[vertex]:
                if neighbor in unseen:
                    unseen.remove(neighbor)
                    part.add(neighbor)
                    stack.append(neighbor)
        out.append(part)
    return out


def _cluster_screen(
    cluster: set[int], graph: HybridGraph, degree: np.ndarray, awd: float
) -> tuple[float, set[int] | None]:
    ordered = sorted(cluster, key=lambda vertex: graph.vertex_key[vertex])
    if len(ordered) <= 1:
        return float("inf"), None
    best = float("inf")
    best_side: set[int] | None = None
    anchor = ordered[0]
    remaining = ordered[1:]
    # Fix the canonical anchor in the first side to avoid enumerating both a
    # cut and its complement.
    for mask in range(1 << len(remaining)):
        side = {anchor}
        side.update(remaining[i] for i in range(len(remaining)) if mask & (1 << i))
        if len(side) == len(ordered):
            continue
        other = cluster - side
        cut = 0.0
        for u, v, weight in zip(graph.u, graph.v, graph.weight, strict=True):
            if (int(u) in side and int(v) in other) or (int(v) in side and int(u) in other):
                cut += weight
        denominator = min(float(np.sum(degree[list(side)])), float(np.sum(degree[list(other)])))
        score = 0.0 if denominator <= 0 else cut / denominator
        side_key = tuple(graph.vertex_key[x] for x in sorted(side, key=lambda x: graph.vertex_key[x]))
        best_key = (
            tuple(graph.vertex_key[x] for x in sorted(best_side, key=lambda x: graph.vertex_key[x]))
            if best_side is not None
            else None
        )
        if score < best or (score == best and (best_key is None or side_key < best_key)):
            best = score
            best_side = side
    threshold = 1.0 / (8.0 * awd)
    return best - threshold, best_side


def _aggregate_vertices(
    graph: HybridGraph,
    component: np.ndarray,
    forest_edges: list[int],
    options: HierarchyOptions,
) -> np.ndarray:
    forest_adjacency: list[list[tuple[int, int]]] = [[] for _ in range(graph.n_vertex)]
    for edge in forest_edges:
        u, v = int(graph.u[edge]), int(graph.v[edge])
        forest_adjacency[u].append((edge, v))
        forest_adjacency[v].append((edge, u))
    for adjacency in forest_adjacency:
        adjacency.sort(key=lambda pair: _edge_priority(graph, pair[0]))

    unassigned = set(range(graph.n_vertex))
    clusters: list[set[int]] = []
    while unassigned:
        seed = min(unassigned, key=lambda vertex: graph.vertex_key[vertex])
        cluster = {seed}
        unassigned.remove(seed)
        while len(cluster) < options.target_size:
            boundary: list[tuple[int, int]] = []
            for vertex in cluster:
                for edge, neighbor in forest_adjacency[vertex]:
                    if neighbor in unassigned:
                        boundary.append((edge, neighbor))
            if not boundary:
                break
            edge, neighbor = min(boundary, key=lambda pair: _edge_priority(graph, pair[0]))
            del edge
            cluster.add(neighbor)
            unassigned.remove(neighbor)
        clusters.append(cluster)

    clusters.sort(key=lambda cluster: min(graph.vertex_key[x] for x in cluster))
    changed = True
    while changed:
        changed = False
        for index, cluster in enumerate(list(clusters)):
            if len(cluster) >= options.target_size:
                continue
            candidates: list[tuple[tuple[float, VertexKey, VertexKey], int]] = []
            for edge in forest_edges:
                u, v = int(graph.u[edge]), int(graph.v[edge])
                in_u, in_v = u in cluster, v in cluster
                if in_u == in_v:
                    continue
                outside = v if in_u else u
                for other_index, other in enumerate(clusters):
                    if other_index != index and outside in other:
                        if len(cluster) + len(other) <= options.aggregate_cap:
                            candidates.append((_edge_priority(graph, edge), other_index))
                        break
            if candidates:
                _, other_index = min(candidates)
                if other_index < index:
                    index, other_index = other_index, index
                clusters[index] = clusters[index] | clusters[other_index]
                del clusters[other_index]
                changed = True
                break

    degree = np.diag(laplacian(graph))
    maximum = np.zeros(graph.n_vertex, dtype=np.float64)
    for u, v, weight in zip(graph.u, graph.v, graph.weight, strict=True):
        maximum[int(u)] = max(maximum[int(u)], weight)
        maximum[int(v)] = max(maximum[int(v)], weight)
    positive = maximum > 0
    weighted_degree = np.ones(graph.n_vertex, dtype=np.float64)
    weighted_degree[positive] = degree[positive] / maximum[positive]
    awd = float(np.mean(weighted_degree))

    refined: list[set[int]] = []
    queue = list(clusters)
    while queue:
        cluster = queue.pop(0)
        margin, side = _cluster_screen(cluster, graph, degree, awd)
        if margin >= 0 or side is None:
            refined.append(cluster)
            continue
        pieces = _forest_components(side, forest_edges, graph)
        pieces.extend(_forest_components(cluster - side, forest_edges, graph))
        pieces.sort(key=lambda part: min(graph.vertex_key[x] for x in part))
        queue = pieces + queue

    refined.sort(key=lambda cluster: min(graph.vertex_key[x] for x in cluster))
    aggregate = np.full(graph.n_vertex, -1, dtype=np.int64)
    for coarse, cluster in enumerate(refined):
        for vertex in cluster:
            if aggregate[vertex] >= 0:
                raise CMGError("AGGREGATE_OVERLAP", "vertex appears in two aggregates")
            aggregate[vertex] = coarse
    if np.any(aggregate < 0):
        raise CMGError("AGGREGATE_MISSING", "not every vertex was aggregated")
    for coarse in range(int(np.max(aggregate)) + 1):
        labels = np.unique(component[aggregate == coarse])
        if labels.size != 1:
            raise CMGError("COMPONENT_MERGE", "aggregate crosses graph components")
    return aggregate


def _contract_graph(graph: HybridGraph, aggregate: np.ndarray) -> HybridGraph:
    count = int(np.max(aggregate)) + 1
    members = [np.flatnonzero(aggregate == coarse) for coarse in range(count)]
    keys = tuple(min(graph.vertex_key[int(v)] for v in member) for member in members)
    raw: list[tuple[int, int, float, tuple[int, ...]]] = []
    for edge, (u, v, weight) in enumerate(zip(graph.u, graph.v, graph.weight, strict=True)):
        cu, cv = int(aggregate[int(u)]), int(aggregate[int(v)])
        if cu == cv:
            continue
        raw.append((cu, cv, float(weight), (edge,)))
    u, v, weight = _collapse_edges(raw)
    return HybridGraph(
        n_firm=count,
        n_vertex=count,
        u=u,
        v=v,
        weight=weight,
        vertex_key=keys,
        auxiliary_worker=(),
    )


def build_hierarchy(
    graph: HybridGraph, options: HierarchyOptions | None = None
) -> Hierarchy:
    options = options or HierarchyOptions()
    if not 0 < options.omega < 1:
        raise CMGError("INVALID_OPTIONS", "omega must lie strictly between zero and one")
    levels: list[Level] = []
    current = graph
    base_edges = max(graph.edge_count, 1)
    base_vertices = max(graph.n_vertex, 1)
    while True:
        component = _components(current)
        component_sizes = np.bincount(component)
        if int(np.max(component_sizes)) <= options.coarse_max:
            levels.append(Level(current, component, None, None))
            break
        if len(levels) + 1 >= options.max_levels:
            raise CMGError("HIERARCHY_LEVEL_LIMIT", "hierarchy exceeds level cap")
        forest = _selected_forest(current, component, options.kappa)
        aggregate = _aggregate_vertices(current, component, forest, options)
        coarse_count = int(np.max(aggregate)) + 1
        if coarse_count > (1.0 - options.min_reduction) * current.n_vertex:
            raise CMGError("HIERARCHY_STALLED", "coarsening reduction is too small")
        prolongation = np.zeros((current.n_vertex, coarse_count), dtype=np.float64)
        prolongation[np.arange(current.n_vertex), aggregate] = 1.0
        coarse = _contract_graph(current, aggregate)
        dense_galerkin = prolongation.T @ laplacian(current) @ prolongation
        if not np.allclose(dense_galerkin, laplacian(coarse), rtol=2e-13, atol=2e-13):
            raise CMGError("GALERKIN_MISMATCH", "contracted graph is not Galerkin exact")
        levels.append(Level(current, component, aggregate, prolongation))
        current = coarse
        edge_complexity = (sum(level.graph.edge_count for level in levels) + current.edge_count) / base_edges
        vertex_complexity = (
            sum(level.graph.n_vertex for level in levels) + current.n_vertex
        ) / base_vertices
        if edge_complexity > options.max_edge_complexity:
            raise CMGError("HIERARCHY_EDGE_LIMIT", "edge complexity exceeds cap")
        if vertex_complexity > options.max_vertex_complexity:
            raise CMGError("HIERARCHY_VERTEX_LIMIT", "vertex complexity exceeds cap")
    return Hierarchy(tuple(levels), options)


def _constrained_jacobi(matrix: np.ndarray, component: np.ndarray, omega: float) -> np.ndarray:
    diagonal = np.diag(matrix)
    out = np.zeros_like(matrix)
    for label in np.unique(component):
        index = np.flatnonzero(component == label)
        if index.size <= 1 and np.all(diagonal[index] == 0):
            continue
        if np.any(diagonal[index] <= 0):
            raise CMGError("INVALID_DIAGONAL", "nontrivial component has nonpositive diagonal")
        inverse = 1.0 / diagonal[index]
        block = np.diag(inverse) - np.outer(inverse, inverse) / float(np.sum(inverse))
        out[np.ix_(index, index)] = omega * block
    return (out + out.T) / 2.0


def _coarse_matrix(level: Level) -> np.ndarray:
    matrix = laplacian(level.graph)
    projector = component_projector(level.component)
    keep: list[int] = []
    for label in np.unique(level.component):
        index = np.flatnonzero(level.component == label)
        if index.size <= 1:
            continue
        ground = min(index, key=lambda vertex: level.graph.vertex_key[int(vertex)])
        keep.extend(int(vertex) for vertex in index if vertex != ground)
    if not keep:
        return np.zeros_like(matrix)
    insertion = np.zeros((matrix.shape[0], len(keep)), dtype=np.float64)
    insertion[keep, np.arange(len(keep))] = 1.0
    grounded = insertion.T @ matrix @ insertion
    try:
        factor = np.linalg.cholesky(grounded)
    except np.linalg.LinAlgError as exc:
        raise CMGError("COARSE_CHOLESKY_FAILED", "coarse grounded matrix is not SPD") from exc
    inverse = np.linalg.solve(factor.T, np.linalg.solve(factor, np.eye(factor.shape[0])))
    residual = grounded @ inverse - np.eye(grounded.shape[0])
    if np.linalg.norm(residual, ord=np.inf) > 5e-11:
        raise CMGError("COARSE_RESIDUAL_FAILED", "coarse inverse residual is too large")
    return projector @ insertion @ inverse @ insertion.T @ projector


def materialize_vcycle(hierarchy: Hierarchy, level_index: int = 0) -> np.ndarray:
    level = hierarchy.levels[level_index]
    matrix = laplacian(level.graph)
    if level_index == len(hierarchy.levels) - 1:
        return _coarse_matrix(level)
    assert level.prolongation is not None
    smoother = _constrained_jacobi(matrix, level.component, hierarchy.options.omega)
    child = materialize_vcycle(hierarchy, level_index + 1)
    identity = np.eye(matrix.shape[0])
    pre = identity - smoother @ matrix
    post = identity - matrix @ smoother
    out = (
        2.0 * smoother
        - smoother @ matrix @ smoother
        + pre @ level.prolongation @ child @ level.prolongation.T @ post
    )
    return (out + out.T) / 2.0


def apply_vcycle(
    hierarchy: Hierarchy, rhs: np.ndarray, level_index: int = 0
) -> np.ndarray:
    values = np.asarray(rhs, dtype=np.float64)
    was_vector = values.ndim == 1
    if was_vector:
        values = values[:, None]
    level = hierarchy.levels[level_index]
    if values.shape[0] != level.graph.n_vertex:
        raise CMGError("INVALID_RHS", "right-hand side has the wrong dimension")
    matrix = laplacian(level.graph)
    if level_index == len(hierarchy.levels) - 1:
        out = _coarse_matrix(level) @ values
    else:
        assert level.prolongation is not None
        smoother = _constrained_jacobi(matrix, level.component, hierarchy.options.omega)
        out = smoother @ values
        residual = values - matrix @ out
        coarse_rhs = level.prolongation.T @ residual
        correction = apply_vcycle(hierarchy, coarse_rhs, level_index + 1)
        if correction.ndim == 1:
            correction = correction[:, None]
        out = out + level.prolongation @ correction
        out = out + smoother @ (values - matrix @ out)
    return out[:, 0] if was_vector else out


def kss_pullback(hierarchy: Hierarchy, n_firm: int) -> np.ndarray:
    fine = hierarchy.levels[0]
    if n_firm > fine.graph.n_vertex:
        raise CMGError("INVALID_FIRM_COUNT", "firm count exceeds graph dimension")
    injection = np.zeros((fine.graph.n_vertex, n_firm), dtype=np.float64)
    injection[:n_firm, :] = np.eye(n_firm)
    firm_component = fine.component[:n_firm]
    projector = component_projector(firm_component)
    cycle = materialize_vcycle(hierarchy)
    out = projector @ injection.T @ cycle @ injection @ projector
    return (out + out.T) / 2.0


def grounded_pullback(hierarchy: Hierarchy, n_firm: int, ground: int) -> np.ndarray:
    if not 0 <= ground < n_firm:
        raise CMGError("INVALID_GROUND", "ground lies outside firm coordinates")
    free = [firm for firm in range(n_firm) if firm != ground]
    insertion = np.zeros((n_firm, n_firm - 1), dtype=np.float64)
    insertion[free, np.arange(n_firm - 1)] = 1.0
    balancing = insertion.copy()
    balancing[ground, :] = -1.0
    fine = hierarchy.levels[0]
    injection = np.zeros((fine.graph.n_vertex, n_firm), dtype=np.float64)
    injection[:n_firm, :] = np.eye(n_firm)
    cycle = materialize_vcycle(hierarchy)
    out = balancing.T @ injection.T @ cycle @ injection @ balancing
    return (out + out.T) / 2.0

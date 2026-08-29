from __future__ import annotations

import collections
import sys


def weak_graph(rows: int, branches: int) -> dict[int, set[int]]:
    workers = rows // 3
    leaves = workers // 5
    hubs = 1 + 40 * branches
    graph: dict[int, set[int]] = collections.defaultdict(set)
    for worker_index in range(workers):
        leaf_index = worker_index % leaves
        branch = leaf_index % branches
        pattern = (5 * (leaf_index // branches)) % 39
        panel = worker_index // leaves
        child = 1 + branch
        outer = 1 + branches + 39 * branch + (pattern + panel) % 39
        if pattern < 7 and panel == 4:
            outer = 0
        leaf = hubs + leaf_index
        worker = -(worker_index + 1)
        for firm in (child, outer, leaf):
            graph[worker].add(firm)
            graph[firm].add(worker)
    return graph


def articulation_and_bridge_counts(
    graph: dict[int, set[int]],
) -> tuple[int, int, int]:
    old_limit = sys.getrecursionlimit()
    sys.setrecursionlimit(max(old_limit, 20_000))
    discovered: dict[int, int] = {}
    low: dict[int, int] = {}
    parent: dict[int, int] = {}
    articulations: set[int] = set()
    bridges = 0
    tick = 0

    def visit(vertex: int) -> None:
        nonlocal bridges, tick
        tick += 1
        discovered[vertex] = low[vertex] = tick
        children = 0
        for neighbor in graph[vertex]:
            if neighbor not in discovered:
                parent[neighbor] = vertex
                children += 1
                visit(neighbor)
                low[vertex] = min(low[vertex], low[neighbor])
                if vertex not in parent and children > 1:
                    articulations.add(vertex)
                if vertex in parent and low[neighbor] >= discovered[vertex]:
                    articulations.add(vertex)
                if low[neighbor] > discovered[vertex]:
                    bridges += 1
            elif parent.get(vertex) != neighbor:
                low[vertex] = min(low[vertex], discovered[neighbor])

    try:
        components = 0
        for vertex in graph:
            if vertex not in discovered:
                components += 1
                visit(vertex)
    finally:
        sys.setrecursionlimit(old_limit)
    worker_articulations = sum(vertex < 0 for vertex in articulations)
    return components, worker_articulations, bridges


def test_smallest_weak_graph_has_no_worker_articulation_or_bridge() -> None:
    graph = weak_graph(7_680, branches=20)
    assert articulation_and_bridge_counts(graph) == (1, 0, 0)


def test_rejected_fixed_small_tree_reproduces_observed_failure() -> None:
    graph = weak_graph(7_680, branches=40)
    assert articulation_and_bridge_counts(graph) == (1, 720, 720)

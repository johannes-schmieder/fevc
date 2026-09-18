"""Five exact-size graph classes for the gated September 13 paper refresh.

Derived from the frozen mixed_degree_20260912/study/harness/generate.py.
The original input generator and all historical input hashes remain unchanged.
"""
from __future__ import annotations
import argparse
import hashlib
import json
from pathlib import Path
import numpy as np

PATTERNS = ("degree5_well_mixed", "degree5_segmented", "mixed_well_mixed",
            "mixed_segmented", "degree4_bottleneck")
PRIMARY_PATTERNS = PATTERNS
SIZES = (8_000, 100_000, 400_000, 1_600_000)
LABELS = ("Degree five: well-mixed", "Degree five: segmented",
          "Mixed degrees 2–8: well-mixed", "Mixed degrees 2–8: segmented",
          "Degree four: bottleneck")


def sha(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def atomic_json(path, value):
    path = Path(path)
    temp = path.with_suffix(path.suffix + ".tmp")
    temp.write_text(json.dumps(value, indent=2, allow_nan=False) + "\n")
    temp.replace(path)


def half_degrees(workers, family):
    if family in ("degree4", "degree5"):
        return np.full(workers, int(family[-1]), dtype=np.int64)
    if family != "mixed":
        raise ValueError("unregistered degree family")
    quotient, remainder = divmod(workers, 7)
    extras = [5] if remainder % 2 else []
    for pair in [(2, 8), (3, 7), (4, 6)][:remainder // 2]:
        extras.extend(pair)
    result = np.concatenate((np.repeat(np.arange(2, 9), quotient), extras)).astype(np.int64)
    if len(result) != workers or int(result.sum()) != 5 * workers:
        raise ValueError("balanced-degree rounding")
    return result


def offsets_for(degrees):
    return np.concatenate(([0], np.cumsum(degrees, dtype=np.int64)))


def distinct_ragged(stubs, degrees, rng):
    """Degree-preserving stub swaps, with no padded or degree-four representation."""
    offsets = offsets_for(degrees)
    if int(offsets[-1]) != len(stubs):
        raise ValueError("stub inventory")
    rng.shuffle(stubs)
    for _ in range(100):
        bad = []
        for degree in np.unique(degrees):
            workers = np.flatnonzero(degrees == degree)
            indices = offsets[workers, None] + np.arange(degree)
            ordered = np.sort(stubs[indices], axis=1)
            bad.extend(workers[np.any(np.diff(ordered, axis=1) == 0, axis=1)].tolist())
        if not bad:
            return stubs
        for worker in bad:
            lo, hi = offsets[worker:worker+2]
            row = stubs[lo:hi]
            for column in range(1, len(row)):
                if row[column] not in row[:column]:
                    continue
                for _ in range(10_000):
                    index = int(rng.integers(len(stubs)))
                    other = int(np.searchsorted(offsets, index, side="right") - 1)
                    other_row = stubs[offsets[other]:offsets[other+1]]
                    a, b = row[column], stubs[index]
                    if other != worker and b not in row and a not in other_row:
                        row[column], stubs[index] = b, a
                        break
                else:
                    raise ValueError("simple bipartite pairing did not complete")
    raise ValueError("duplicate worker-firm pairs remain")


def stratified_pairs(degrees, pairs):
    """Largest remainder with ascending-degree tie break and integer arithmetic."""
    values, counts = np.unique(degrees, return_counts=True)
    numerators = counts * pairs
    allocated = numerators // len(degrees)
    remainder = int(pairs - allocated.sum())
    order = np.lexsort((values, -(numerators % len(degrees))))
    allocated[order[:remainder]] += 1
    return dict(zip(map(int, values), map(int, allocated)))


def graph(rows, pattern):
    if pattern not in PATTERNS or rows not in SIZES:
        raise ValueError("unregistered pattern or exact row count")
    family, mobility = pattern.split("_", 1)
    mobility = "segmented" if mobility == "bottleneck" else mobility
    mixed = family == "mixed"
    mean_degree = 4 if family == "degree4" else 5
    workers, firms = rows // mean_degree, rows // 100
    degrees_half = half_degrees(workers // 2, family)
    graph_seed = [20260913, rows, {"degree4": 4, "degree5": 5, "mixed": 28}[family], 1]
    rng = np.random.default_rng(np.random.SeedSequence(graph_seed))
    rng.shuffle(degrees_half)
    degrees = np.tile(degrees_half, 2)
    offsets = offsets_for(degrees)
    if mobility == "well_mixed":
        assignment = distinct_ragged(np.repeat(np.arange(firms), 100), degrees, rng)
    else:
        left = distinct_ragged(np.repeat(np.arange(firms // 2), 100), degrees_half, rng)
        right = distinct_ragged(np.repeat(np.arange(firms // 2, firms), 100), degrees_half, rng)
        half_offsets = offsets_for(degrees_half)
        for degree, count in stratified_pairs(degrees_half, workers // 200).items():
            eligible = np.flatnonzero(degrees_half == degree)
            a = rng.choice(eligible, count, replace=False)
            b = rng.choice(eligible, count, replace=False)
            for first, second in zip(a, b):
                width = degree // 2
                ai, bi = half_offsets[first], half_offsets[second]
                temp = left[ai:ai+width].copy()
                left[ai:ai+width] = right[bi:bi+width]
                right[bi:bi+width] = temp
        assignment = np.concatenate((left, right))
    observed = np.bincount(assignment, minlength=firms)
    worker = np.repeat(np.arange(workers), degrees)
    if len(np.unique(worker * firms + assignment)) != rows or not np.all(observed == 100):
        raise ValueError("worker-firm uniqueness or firm-degree contract")
    lower = np.minimum.reduceat(assignment, offsets[:-1]) < firms // 2
    upper = np.maximum.reduceat(assignment, offsets[:-1]) >= firms // 2
    crossing = int(np.count_nonzero(lower & upper))
    if mobility == "segmented" and crossing != workers // 100:
        raise ValueError("crossing share contract")
    unique, counts = np.unique(degrees, return_counts=True)
    retained = int(np.count_nonzero(degrees > 4))
    meta = dict(schema="FEVC-PAPER-FIVE-GRAPHS-INPUT-V1", pattern=pattern, mobility=mobility,
                degree_family="mixed_2_8" if mixed else family, rows=rows,
                workers=workers, firms=firms, matches=rows, worker_degree=None if mixed else mean_degree,
                worker_degree_histogram={str(int(d)): int(c) for d, c in zip(unique, counts)},
                min_worker_degree=int(degrees.min()), max_worker_degree=int(degrees.max()),
                mean_worker_degree=float(degrees.mean()), min_firm_degree=100,
                max_firm_degree=100, mean_firm_degree=100., firm_degree_cv=0.,
                crossing_workers=crossing, crossing_share=crossing/workers,
                expected_auxiliary_workers=retained, expected_hybrid_vertices=firms+retained,
                expected_eliminated_workers=workers-retained,
                hybrid_edges_before_coalescing=int(np.where(degrees <= 4,
                    degrees*(degrees-1)//2, degrees).sum()),
                generator_seed=graph_seed)
    return assignment, offsets, meta


def connectivity(assignment, offsets, firms):
    """Independent sparse Schur audit; unit-weight worker cliques have weight 1/d."""
    from scipy.sparse import coo_matrix
    from scipy.sparse.csgraph import connected_components
    degrees = np.diff(offsets)
    first, second, weights = [], [], []
    for degree in np.unique(degrees):
        workers = np.flatnonzero(degrees == degree)
        block = assignment[offsets[workers, None] + np.arange(degree)]
        for a in range(degree):
            for b in range(a+1, degree):
                first.extend((block[:, a], block[:, b]))
                second.extend((block[:, b], block[:, a]))
                weights.extend((np.full(len(workers), 1/degree),) * 2)
    adjacency = coo_matrix((np.concatenate(weights),
                           (np.concatenate(first), np.concatenate(second))), shape=(firms, firms)).tocsr()
    components = int(connected_components(adjacency, directed=False, return_labels=False))
    if components != 1:
        raise ValueError("generated firm graph disconnected")
    degree = np.asarray(adjacency.sum(axis=1)).ravel()
    boundary = float(adjacency[:firms//2, firms//2:].sum())
    result = dict(components=components, schur_nonzeros=int(adjacency.nnz),
                  fixed_half_cut_conductance=float(boundary / min(degree[:firms//2].sum(), degree[firms//2:].sum())))
    if firms <= 200:
        inv = 1 / np.sqrt(degree)
        normalized = np.eye(firms) - inv[:, None] * adjacency.toarray() * inv[None, :]
        result["normalized_spectral_gap"] = float(np.linalg.eigvalsh(normalized)[1])
    return result


def generate(path, rows, pattern, diagnostics=True):
    path = Path(path)
    if path.exists() or path.with_suffix(".json").exists():
        raise ValueError("input already exists")
    assignment, offsets, meta = graph(rows, pattern)
    if diagnostics:
        meta.update(connectivity(assignment, offsets, meta["firms"]))
    rng = np.random.default_rng(np.random.SeedSequence([20260913, rows, 2]))
    alpha, psi = rng.normal(size=meta["workers"]), .6*rng.normal(size=meta["firms"])
    worker = np.repeat(np.arange(meta["workers"]), np.diff(offsets))
    y = alpha[worker] + psi[assignment] + rng.normal(size=rows)
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(".csv.tmp")
    with temporary.open("x") as stream:
        stream.write("observation_key,worker,firm,period,match,y\n")
        for start in range(0, rows, 100_000):
            stop = min(start+100_000, rows)
            i = np.arange(start, stop)
            block = np.column_stack((i+1, worker[start:stop]+1, assignment[start:stop]+1,
                                     i-offsets[worker[start:stop]]+1, i+1, y[start:stop]))
            np.savetxt(stream, block, delimiter=",", fmt=["%d"]*5+["%.17g"])
    temporary.replace(path)
    meta.update(sha256=sha(path), bytes=path.stat().st_size,
                outcome_seed=[20260913, rows, 2], generator_sha256=sha(__file__))
    atomic_json(path.with_suffix(".json"), meta)
    return meta


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=Path)
    parser.add_argument("rows", type=int, choices=SIZES)
    parser.add_argument("pattern", choices=PATTERNS)
    args = parser.parse_args()
    print(json.dumps(generate(args.output, args.rows, args.pattern)))

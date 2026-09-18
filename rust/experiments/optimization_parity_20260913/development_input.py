#!/usr/bin/env python3
"""Write one immutable public-path development input for the bounded screen."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np

from campaign import PROFILES
from generate import atomic_json, distinct_ragged, half_degrees, offsets_for, sha


STAYER_PROFILES = {"pooled_stayer_match", "observation_stayers"}


def build(output: Path, profile: str, rows: int) -> dict:
    if output.exists() or output.with_suffix(".json").exists():
        raise ValueError("development input already exists")
    if profile not in PROFILES:
        raise ValueError("unregistered development profile")
    expected = 8_000 if PROFILES.index(profile) >= 5 else 100_000
    if rows != expected:
        raise ValueError("profile row count differs from the frozen campaign")

    repeats_per_match = 4
    edges = rows // repeats_per_match
    workers, firms = edges // 5, edges // 100
    degrees_half = half_degrees(workers // 2, "mixed")
    rng_graph = np.random.default_rng(
        np.random.SeedSequence([20260913, rows, repeats_per_match, 31])
    )
    rng_graph.shuffle(degrees_half)
    degrees = np.tile(degrees_half, 2)
    edge_offsets = offsets_for(degrees)
    assignment = distinct_ragged(
        np.repeat(np.arange(firms, dtype=np.int64), 100), degrees, rng_graph
    )
    graph_receipt = {"firms": firms}
    offsets = edge_offsets * repeats_per_match
    worker = np.repeat(
        np.arange(len(degrees), dtype=np.int64), degrees * repeats_per_match
    )
    firm = np.repeat(assignment, repeats_per_match)
    converted_workers = 0
    if profile in STAYER_PROFILES:
        converted_workers = max(1, len(degrees) // 20)
        for index in range(converted_workers):
            firm[offsets[index] : offsets[index + 1]] = firm[offsets[index]]

    observation = np.arange(rows, dtype=np.int64)
    period = observation - offsets[worker] + 1
    match = (worker + 1) * 1_000_000 + firm + 1
    rng = np.random.default_rng(np.random.SeedSequence([20260913, rows, 31]))
    control_1 = rng.normal(size=rows)
    control_2 = rng.normal(size=rows)
    projection = rng.normal(size=rows) + 0.15 * control_1
    frequency = 1 + observation % 3
    target_weight = 0.75 + (observation % 7) / 10.0
    worker_effect = rng.normal(size=len(degrees))
    firm_effect = 0.6 * rng.normal(size=graph_receipt["firms"])
    outcome = (
        worker_effect[worker]
        + firm_effect[firm]
        + 0.3 * control_1
        - 0.2 * control_2
        + rng.normal(size=rows)
    )

    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = output.with_suffix(output.suffix + ".tmp")
    with temporary.open("x", encoding="utf-8") as stream:
        stream.write(
            "observation_key,worker,firm,period,match,y,frequency,"
            "target_weight,control_1,control_2,projection\n"
        )
        for start in range(0, rows, 100_000):
            stop = min(start + 100_000, rows)
            block = np.column_stack(
                (
                    observation[start:stop] + 1,
                    worker[start:stop] + 1,
                    firm[start:stop] + 1,
                    period[start:stop],
                    match[start:stop],
                    outcome[start:stop],
                    frequency[start:stop],
                    target_weight[start:stop],
                    control_1[start:stop],
                    control_2[start:stop],
                    projection[start:stop],
                )
            )
            np.savetxt(
                stream,
                block,
                delimiter=",",
                fmt=["%d"] * 5 + ["%.17g", "%d"] + ["%.17g"] * 4,
            )
    temporary.replace(output)
    receipt = {
        "schema": "FEVC-OPTIMIZATION-DEVELOPMENT-INPUT-V1",
        "profile": profile,
        "rows": rows,
        "graph": "mixed_well_mixed",
        "workers": int(len(degrees)),
        "firms": int(np.unique(firm).size),
        "repeats_per_match": repeats_per_match,
        "converted_stayer_workers": converted_workers,
        "actual_stayer_workers": int(
            sum(np.unique(firm[offsets[i] : offsets[i + 1]]).size == 1 for i in range(len(degrees)))
        ),
        "sha256": sha(output),
        "bytes": output.stat().st_size,
        "generator_sha256": sha(Path(__file__)),
        "base_graph_generator_sha256": sha(Path(__file__).with_name("generate.py")),
    }
    atomic_json(output.with_suffix(".json"), receipt)
    return receipt


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("profile", choices=PROFILES)
    parser.add_argument("rows", type=int, choices=(8_000, 100_000))
    args = parser.parse_args()
    print(json.dumps(build(args.output, args.profile, args.rows), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

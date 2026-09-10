#!/usr/bin/env python3
"""Build frozen task manifests for smoke, pilot, exact, or confirmation runs."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

try:
    from .common import (COMPARATORS, CORE_GRID, PROBES, ROLES, SCHEMA, SEEDS,
                         SIZE_ROWS, SOURCE_COMMIT, TASK_FIELDS, atomic_json,
                         read_manifest, sha256, write_tsv)
except ImportError:
    from common import (COMPARATORS, CORE_GRID, PROBES, ROLES, SCHEMA, SEEDS,
                        SIZE_ROWS, SOURCE_COMMIT, TASK_FIELDS, atomic_json,
                        read_manifest, sha256, write_tsv)


def latin_order(repeat: int) -> tuple[str, ...]:
    offset = (repeat - 1) % len(ROLES)
    return ROLES[offset:] + ROLES[:offset]


def cells() -> list[tuple[str, str, int, int]]:
    values = [(f"n{n:06d}-c28", "size", n, 28) for n in SIZE_ROWS]
    values.extend((f"n122880-c{c:02d}", "cores", 122_880, c)
                  for c in CORE_GRID if c != 28)
    return values


def build_rows(profile: str) -> list[dict[str, object]]:
    if profile == "confirmation":
        selected = [(cell, sweep, n, c, repeat)
                    for cell, sweep, n, c in cells() for repeat in range(1, 6)]
    elif profile == "smoke":
        selected = [("smoke-n7680-c2", "smoke", 7_680, 2, 1)]
    elif profile == "pilot":
        selected = [("pilot-n491520-c28", "pilot", 491_520, 28, 1)]
    elif profile == "exact":
        selected = [("exact-n960-c1", "exact", 960, 1, 1)]
    else:
        raise ValueError("unknown profile")
    rows: list[dict[str, object]] = []
    for task_id, (cell, sweep, n, cores, repeat) in enumerate(selected, 1):
        rows.append({
            "task_id": task_id,
            "cell_id": cell,
            "sweep": sweep,
            "rows": n,
            "workers": n // 3,
            "firms": n // 120,
            "cores": cores,
            "repeat": repeat,
            "seed": SEEDS[repeat - 1],
            "probes": 0 if profile == "exact" else PROBES,
            "algorithm": "exact" if profile == "exact" else "jla",
            "role_order": ",".join(latin_order(repeat)),
        })
    return rows


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", choices=("smoke", "pilot", "exact", "confirmation"),
                        required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--identity", type=Path, required=True)
    args = parser.parse_args()
    rows = build_rows(args.profile)
    write_tsv(args.output, TASK_FIELDS, rows)
    read_manifest(args.output)
    identity = {
        "schema": f"{SCHEMA}-MANIFEST",
        "status": "FROZEN",
        "profile": args.profile,
        "source_commit": SOURCE_COMMIT,
        "comparators": COMPARATORS,
        "manifest_sha256": sha256(args.output),
        "tasks": len(rows),
        "estimator_calls": len(rows) * len(ROLES),
        "design": "strong_d3_unique_worker_firm_all_movers",
        "deletion": "match",
        "target_weighting": "uniform_stored_rows_population_N",
        "primary_phase": "post_generic_csv_validation_through_target_extraction",
        "rss_interval_seconds": 0.1,
    }
    atomic_json(args.identity, identity)
    print(f"FEVC_FIVE_WAY_MANIFEST_PASS {args.profile} {len(rows)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

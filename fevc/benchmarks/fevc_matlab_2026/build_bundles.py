#!/usr/bin/env python3
"""Build the 24 same-host topology-by-repetition SCC bundle manifest."""

from __future__ import annotations

import argparse
import csv
from pathlib import Path
from typing import Any

try:
    from .common import REPLICATES, STRUCTURES, integer, read_manifest, require, write_tsv
except ImportError:
    from common import REPLICATES, STRUCTURES, integer, read_manifest, require, write_tsv  # type: ignore


BUNDLE_SCHEMA = "FEVC-MATLAB-2026-MAIN-BUNDLE-V1"
BUNDLE_FIELDS = (
    "bundle_schema", "bundle_task_id", "structure", "replicate", "seed",
    "execution_order", "cell_task_ids", "cell_count",
)


def build_rows(cells: list[dict[str, str]]) -> list[dict[str, Any]]:
    output: list[dict[str, Any]] = []
    bundle_task_id = 0
    for structure in STRUCTURES:
        for replicate, seed, order in REPLICATES:
            selected = [
                row for row in cells
                if row["structure"] == structure
                and integer(row["replicate"], "replicate") == replicate
            ]
            require(len(selected) == 10, "each bundle must contain ten main cells")
            require({row["execution_order"] for row in selected} == {order},
                    "bundle execution order changed")
            bundle_task_id += 1
            output.append({
                "bundle_schema": BUNDLE_SCHEMA,
                "bundle_task_id": bundle_task_id,
                "structure": structure,
                "replicate": replicate,
                "seed": seed,
                "execution_order": order,
                "cell_task_ids": ",".join(row["task_id"] for row in selected),
                "cell_count": len(selected),
            })
    require(bundle_task_id == 24, "main campaign must contain 24 bundles")
    return output


def read_bundles(path: Path) -> list[dict[str, str]]:
    require(path.is_file() and not path.is_symlink(), "invalid bundle manifest")
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle, delimiter="\t")
        require(tuple(reader.fieldnames or ()) == BUNDLE_FIELDS,
                "bundle fields changed")
        rows = [dict(row) for row in reader]
    require(len(rows) == 24, "bundle manifest must contain 24 rows")
    for position, row in enumerate(rows, start=1):
        require(row["bundle_schema"] == BUNDLE_SCHEMA, "bundle schema changed")
        require(integer(row["bundle_task_id"], "bundle task ID", 1) == position,
                "bundle task IDs are not consecutive")
        require(row["structure"] in STRUCTURES, "unknown bundle structure")
        require(integer(row["cell_count"], "bundle cell count", 1) == 10,
                "bundle cell count changed")
        ids = [integer(item, "cell task ID", 1)
               for item in row["cell_task_ids"].split(",")]
        require(len(ids) == len(set(ids)) == 10, "bundle cell IDs changed")
        matches = [item for item in REPLICATES
                   if item[0] == integer(row["replicate"], "replicate", 1)]
        require(len(matches) == 1, "unknown bundle replicate")
        _, seed, order = matches[0]
        require(integer(row["seed"], "bundle seed", 1) == seed,
                "bundle seed changed")
        require(row["execution_order"] == order, "bundle order changed")
    all_ids = [integer(item, "cell task ID", 1) for row in rows
               for item in row["cell_task_ids"].split(",")]
    require(sorted(all_ids) == list(range(1, 241)),
            "bundles do not partition all main cells")
    return rows


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cells", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(not args.output.exists(), "bundle target already exists")
    require(args.output.parent.is_dir(), "bundle target parent is missing")
    write_tsv(args.output, BUNDLE_FIELDS, build_rows(read_manifest(args.cells)))
    read_bundles(args.output)
    print("FEVC_MATLAB_2026_BUNDLE_MANIFEST_PASS bundles=24 cells=240")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Build the frozen 300-task VCkss comparative-scaling manifest."""

from __future__ import annotations

import argparse
from pathlib import Path

try:
    from .common import (
        CORE_GRID,
        ESTIMATOR_TIMEOUT_SECONDS,
        HARD_WALL_SECONDS,
        HEX40,
        HEX64,
        PROBES,
        REPLICATES,
        REQUESTED_SLOTS,
        ROW_GRID,
        STATA_MAX_PROCESSORS,
        STRUCTURES,
        TASK_FIELDS,
        TASK_SCHEMA,
        read_manifest,
        registered_firms,
        require,
        sha256,
        write_tsv,
    )
except ImportError:
    from common import (  # type: ignore
        CORE_GRID,
        ESTIMATOR_TIMEOUT_SECONDS,
        HARD_WALL_SECONDS,
        HEX40,
        HEX64,
        PROBES,
        REPLICATES,
        REQUESTED_SLOTS,
        ROW_GRID,
        STATA_MAX_PROCESSORS,
        STRUCTURES,
        TASK_FIELDS,
        TASK_SCHEMA,
        read_manifest,
        registered_firms,
        require,
        sha256,
        write_tsv,
    )


def build_rows(
    source_commit: str,
    bundle_sha256: str,
    *,
    mem_per_core_gib: int,
    command_memory_gib: int,
) -> list[dict[str, object]]:
    require(HEX40.fullmatch(source_commit) is not None, "invalid source commit")
    require(HEX64.fullmatch(bundle_sha256) is not None, "invalid bundle hash")
    require(mem_per_core_gib > 0, "memory per core must be positive")
    require(0 < command_memory_gib <= mem_per_core_gib * REQUESTED_SLOTS,
            "command memory must fit the scheduler allocation")
    result: list[dict[str, object]] = []
    task_id = 0
    for structure, (connectivity, degree, _) in STRUCTURES.items():
        for rows in ROW_GRID:
            workers = rows // degree
            firms = registered_firms(structure, workers)
            require(rows == workers * degree,
                    "registered dimensions are not divisible")
            for cores in CORE_GRID:
                for replicate, seed, order in REPLICATES:
                    task_id += 1
                    result.append({
                        "task_schema": TASK_SCHEMA,
                        "task_id": task_id,
                        "experiment_id": (
                            f"scale_{structure}_n{rows}_c{cores}_r{replicate}"
                        ),
                        "source_commit": source_commit,
                        "bundle_sha256": bundle_sha256,
                        "structure": structure,
                        "connectivity": connectivity,
                        "cells_per_worker": degree,
                        "rows": rows,
                        "workers": workers,
                        "firms": firms,
                        "active_cores": cores,
                        "stata_processors": min(cores, STATA_MAX_PROCESSORS),
                        "mata_active_cores": min(cores, STATA_MAX_PROCESSORS),
                        "rust_threads": cores,
                        "matlab_workers": cores,
                        "replicate": replicate,
                        "seed": seed,
                        "execution_order": order,
                        "probes": PROBES,
                        "requested_slots": REQUESTED_SLOTS,
                        "mem_per_core_gib": mem_per_core_gib,
                        "command_memory_gib": command_memory_gib,
                        "hard_wall_seconds": HARD_WALL_SECONDS,
                        "estimator_timeout_seconds": ESTIMATOR_TIMEOUT_SECONDS,
                        "sample_contract": "same_literal_match_rows_v2",
                        "target_contract": "uniform_stored_rows_v1",
                        "comparison_contract": (
                            "fresh_process_role_specific_cores_paired_host_time_rss_v3"
                        ),
                    })
    require(task_id == 300, "comparative matrix must contain 300 tasks")
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--bundle", required=True)
    parser.add_argument("--mem-per-core-gib", type=int, default=8)
    parser.add_argument("--command-memory-gib", type=int, default=112)
    args = parser.parse_args()
    require(not args.output.exists(), "manifest target already exists")
    require(args.output.parent.is_dir(), "manifest parent is missing")
    rows = build_rows(
        args.source_commit,
        args.bundle,
        mem_per_core_gib=args.mem_per_core_gib,
        command_memory_gib=args.command_memory_gib,
    )
    write_tsv(args.output, TASK_FIELDS, rows)
    read_manifest(args.output)
    print(
        "VCKSS_COMPARATIVE_SCALING_MANIFEST_PASS "
        f"tasks=300 sha256={sha256(args.output)}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

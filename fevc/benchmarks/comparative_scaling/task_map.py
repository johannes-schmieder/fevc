#!/usr/bin/env python3
"""Create and resolve dense SGE-array maps for sparse manifest task IDs."""

from __future__ import annotations

import argparse
from pathlib import Path

try:
    from .expand_task_ids import expand
except ImportError:
    from expand_task_ids import expand  # type: ignore


SCHEMA = "FEVC-COMPARATIVE-SCALING-TASK-MAP-V1"
HEADER = "schema\tscheduler_task_id\tmanifest_task_id"


def rows(path: Path) -> list[tuple[int, int]]:
    lines = path.read_text(encoding="utf-8").splitlines()
    if not lines or lines[0] != HEADER:
        raise ValueError("task-map header changed")
    parsed: list[tuple[int, int]] = []
    for line in lines[1:]:
        fields = line.split("\t")
        if len(fields) != 3 or fields[0] != SCHEMA:
            raise ValueError("task-map row changed")
        scheduler_task_id, manifest_task_id = map(int, fields[1:])
        parsed.append((scheduler_task_id, manifest_task_id))
    if not parsed:
        raise ValueError("task map is empty")
    scheduler_ids = [scheduler for scheduler, _ in parsed]
    manifest_ids = [manifest for _, manifest in parsed]
    if scheduler_ids != list(range(1, len(parsed) + 1)):
        raise ValueError("scheduler task IDs are not dense and ordered")
    if (len(manifest_ids) != len(set(manifest_ids)) or
            any(value < 1 or value > 300 for value in manifest_ids)):
        raise ValueError("manifest task IDs are invalid")
    return parsed


def write(specification: str, output: Path) -> list[tuple[int, int]]:
    manifest_ids = expand(specification)
    mapped = list(enumerate(manifest_ids, 1))
    text = HEADER + "\n" + "".join(
        f"{SCHEMA}\t{scheduler}\t{manifest}\n"
        for scheduler, manifest in mapped
    )
    with output.open("x", encoding="utf-8") as stream:
        stream.write(text)
    return mapped


def manifest_id(path: Path, scheduler_task_id: int) -> int:
    matches = [manifest for scheduler, manifest in rows(path)
               if scheduler == scheduler_task_id]
    if len(matches) != 1:
        raise ValueError("scheduler task ID is absent from task map")
    return matches[0]


def scheduler_id(path: Path, manifest_task_id: int) -> int:
    matches = [scheduler for scheduler, manifest in rows(path)
               if manifest == manifest_task_id]
    if len(matches) != 1:
        raise ValueError("manifest task ID is absent from task map")
    return matches[0]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    actions = parser.add_mutually_exclusive_group(required=True)
    actions.add_argument("--write-task-ids")
    actions.add_argument("--scheduler-task-id", type=int)
    actions.add_argument("--manifest-task-id", type=int)
    parser.add_argument("--map", type=Path, required=True)
    args = parser.parse_args()
    if args.write_task_ids is not None:
        print(len(write(args.write_task_ids, args.map)))
    elif args.scheduler_task_id is not None:
        print(manifest_id(args.map, args.scheduler_task_id))
    else:
        print(scheduler_id(args.map, args.manifest_task_id))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

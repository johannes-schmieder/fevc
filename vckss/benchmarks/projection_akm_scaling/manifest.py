#!/usr/bin/env python3
"""Create and read immutable task manifests for staged SCC submission."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from common import ROWS, feasibility_tasks, gate_task, read_manifest, write_manifest


def main() -> None:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    create = subparsers.add_parser("create-feasibility")
    create.add_argument("rows", type=int, choices=ROWS)
    create.add_argument("output", type=Path)
    gate = subparsers.add_parser("create-gate")
    gate.add_argument("output", type=Path)
    read = subparsers.add_parser("read")
    read.add_argument("manifest", type=Path)
    read.add_argument("task_id", type=int)
    args = parser.parse_args()
    if args.command == "create-feasibility":
        values = feasibility_tasks(args.rows)
        write_manifest(args.output, values)
        print(json.dumps({"status": "PASS", "tasks": len(values), "rows": args.rows}))
        return
    if args.command == "create-gate":
        write_manifest(args.output, [gate_task()])
        print(json.dumps({"status": "PASS", "tasks": 1, "rows": 6000}))
        return
    values = read_manifest(args.manifest)
    if args.task_id < 1 or args.task_id > len(values):
        raise ValueError("task id outside manifest")
    value = values[args.task_id - 1]
    print("\t".join(value[field] for field in value))


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Print one registered manifest task as ordered newline-delimited values."""

from __future__ import annotations

import argparse
from pathlib import Path

try:
    from .common import TASK_FIELDS, read_single_task, read_task, require, write_tsv
except ImportError:
    from common import TASK_FIELDS, read_single_task, read_task, require, write_tsv  # type: ignore


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--task-id", type=int, required=True)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if args.manifest.name == "smoke.tsv":
        require(args.task_id == 1, "smoke task ID must be one")
        task = read_single_task(args.manifest)
    else:
        task = read_task(args.manifest, args.task_id)
    if args.output is not None:
        require(not args.output.exists(), "task target already exists")
        require(args.output.parent.is_dir(), "task parent is missing")
        write_tsv(args.output, TASK_FIELDS, (task,))
        return 0
    for field in TASK_FIELDS:
        print(task[field])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
from __future__ import annotations

import argparse
from pathlib import Path

from common import TASK_FIELDS, read_task, require, write_tsv


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--task-id", type=int, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(not args.output.exists() and args.output.parent.is_dir(),
            "invalid task output")
    write_tsv(args.output, TASK_FIELDS, (read_task(args.manifest, args.task_id),))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

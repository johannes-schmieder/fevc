#!/usr/bin/env python3
"""Read one manifest row as shell-safe tab-separated values."""

from __future__ import annotations

import argparse
import csv
import json
import os
from pathlib import Path

try:
    from .common import TASK_FIELDS, read_manifest
except ImportError:
    from common import TASK_FIELDS, read_manifest


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", type=Path)
    parser.add_argument("task_id", type=int)
    parser.add_argument("--json-output", type=Path)
    args = parser.parse_args()
    rows = read_manifest(args.manifest)
    if not 1 <= args.task_id <= len(rows):
        raise ValueError("task id outside manifest")
    row = rows[args.task_id - 1]
    if args.json_output:
        value = {field: row[field] for field in TASK_FIELDS}
        for field in ("task_id", "rows", "workers", "firms", "cores", "repeat",
                      "seed", "probes"):
            value[field] = int(value[field])
        temporary = args.json_output.with_suffix(args.json_output.suffix + ".tmp")
        temporary.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n",
                             encoding="utf-8")
        os.replace(temporary, args.json_output)
    writer = csv.writer(__import__("sys").stdout, delimiter="\t", lineterminator="\n")
    writer.writerow(row[field] for field in TASK_FIELDS)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

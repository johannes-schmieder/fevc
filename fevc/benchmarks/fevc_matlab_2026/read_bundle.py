#!/usr/bin/env python3
"""Print one registered bundle row as ordered newline-delimited values."""

from __future__ import annotations

import argparse
from pathlib import Path

try:
    from .build_bundles import BUNDLE_FIELDS, read_bundles
    from .common import require
except ImportError:
    from build_bundles import BUNDLE_FIELDS, read_bundles  # type: ignore
    from common import require  # type: ignore


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--bundle-task-id", type=int, required=True)
    args = parser.parse_args()
    rows = read_bundles(args.manifest)
    require(1 <= args.bundle_task_id <= len(rows), "bundle task ID outside manifest")
    row = rows[args.bundle_task_id - 1]
    for field in BUNDLE_FIELDS:
        print(row[field])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

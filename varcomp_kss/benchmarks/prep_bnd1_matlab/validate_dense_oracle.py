#!/usr/bin/env python3
"""Hard-gate the clean-room dense MATLAB oracle against Stata exact mode."""

from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path

from common import finite, require, sha256

TARGETS = (
    "worker_variance",
    "firm_variance",
    "worker_firm_covariance",
    "total_variance",
)
FIELDS = ("plugin", "correction", "corrected")
TOLERANCE = {"plugin": 2e-10, "correction": 2e-9, "corrected": 2e-9}


def rows(path: Path) -> list[dict[str, str]]:
    require(path.is_file() and not path.is_symlink(), f"invalid oracle CSV: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        result = list(csv.DictReader(handle))
    require(len(result) == 4, "oracle CSV must have four target rows")
    return result


def validate(matlab_path: Path, stata_path: Path, source_commit: str) -> dict[str, object]:
    matlab = rows(matlab_path)
    stata = rows(stata_path)
    gaps: dict[str, dict[str, float]] = {}
    for left, right, target in zip(matlab, stata, TARGETS, strict=True):
        require(left["source_commit"] == right["source_commit"] == source_commit,
                "dense-oracle source binding changed")
        require(left["target"] == right["target"] == target,
                "dense-oracle target order changed")
        gaps[target] = {}
        for field in FIELDS:
            gap = abs(finite(left[field], f"MATLAB {field}") -
                      finite(right[field], f"Stata {field}"))
            require(gap <= TOLERANCE[field],
                    f"dense-oracle {target} {field} gap {gap:g}")
            gaps[target][field] = gap
    return {
        "schema": "PREP-BND-1-DENSE-ORACLE-V1",
        "status": "PASS_EXACT_DENSE_ORACLE",
        "source_commit": source_commit,
        "matlab_oracle_sha256": sha256(matlab_path),
        "stata_oracle_sha256": sha256(stata_path),
        "tolerances": TOLERANCE,
        "absolute_gaps": gaps,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--matlab", type=Path, required=True)
    parser.add_argument("--stata", type=Path, required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    require(not args.output.exists(), "dense-oracle output already exists")
    payload = validate(args.matlab, args.stata, args.source_commit)
    args.output.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n",
                           encoding="utf-8")
    print(f"PREP_BND1_DENSE_ORACLE_PASS {args.source_commit}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

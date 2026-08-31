#!/usr/bin/env python3
"""Expand a bounded SGE task specification into unique task IDs."""

from __future__ import annotations

import re
import sys


def expand(specification: str) -> list[int]:
    if re.fullmatch(r"[0-9,-]+", specification) is None:
        raise ValueError("invalid task specification")
    values: list[int] = []
    for item in specification.split(","):
        if not item:
            raise ValueError("empty task specification item")
        if "-" in item:
            fields = item.split("-")
            if len(fields) != 2:
                raise ValueError("invalid task range")
            first, last = map(int, fields)
            if first > last:
                raise ValueError("reversed task range")
            values.extend(range(first, last + 1))
        else:
            values.append(int(item))
    if not values or any(value < 1 or value > 300 for value in values):
        raise ValueError("task ID outside 1..300")
    if len(values) != len(set(values)):
        raise ValueError("duplicate task ID")
    return values


def main() -> int:
    if len(sys.argv) != 2:
        raise ValueError("usage: expand_task_ids.py TASK_IDS")
    print("\n".join(str(value) for value in expand(sys.argv[1])))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

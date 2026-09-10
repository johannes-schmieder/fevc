#!/usr/bin/env python3
"""Select a deterministic subset of the scheduler CPU affinity."""

from __future__ import annotations

import argparse
import os

ALLOWED = (1, 2, 4, 8, 14, 28)


def subset(count: int) -> tuple[int, ...]:
    if count not in ALLOWED:
        raise ValueError("core count outside benchmark grid")
    available = tuple(sorted(os.sched_getaffinity(0)))
    if len(available) < count:
        raise ValueError("scheduler affinity is smaller than requested subset")
    return available[:count]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--count", required=True, type=int)
    args = parser.parse_args()
    print(",".join(map(str, subset(args.count))))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

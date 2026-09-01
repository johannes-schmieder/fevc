#!/usr/bin/env python3
"""Select and verify a deterministic subset of the current Linux CPU affinity."""

from __future__ import annotations

import argparse
import os


def subset(count: int, *, offset: int = 0) -> tuple[int, ...]:
    if count not in (1, 2, 4, 8, 14, 28):
        raise ValueError("active-core count is outside the registered grid")
    if offset < 0:
        raise ValueError("CPU offset must be nonnegative")
    available = tuple(sorted(os.sched_getaffinity(0)))
    if len(available) < offset + count:
        raise ValueError("scheduler affinity is smaller than the requested subset")
    return available[offset:offset + count]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--count", type=int, required=True)
    parser.add_argument("--offset", type=int, default=0)
    args = parser.parse_args()
    print(",".join(str(cpu) for cpu in subset(args.count, offset=args.offset)))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

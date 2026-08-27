#!/usr/bin/env python3
"""Select and verify a deterministic subset of the current Linux CPU affinity."""

from __future__ import annotations

import argparse
import os


def subset(count: int) -> tuple[int, ...]:
    if count not in (1, 2, 4, 8, 16):
        raise ValueError("active-core count is outside the registered grid")
    available = tuple(sorted(os.sched_getaffinity(0)))
    if len(available) < count:
        raise ValueError("scheduler affinity is smaller than the requested subset")
    return available[:count]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--count", type=int, required=True)
    args = parser.parse_args()
    print(",".join(str(cpu) for cpu in subset(args.count)))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Sample the physical RSS sum of one process tree."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import time
from pathlib import Path


def descendants(root: int) -> tuple[set[int], dict[int, int]]:
    output = subprocess.run(
        ["ps", "-eo", "pid=,ppid=,rss="], check=True, capture_output=True, text=True
    ).stdout
    parents: dict[int, list[int]] = {}
    rss: dict[int, int] = {}
    for line in output.splitlines():
        fields = line.split()
        if len(fields) != 3:
            continue
        pid, parent, kib = map(int, fields)
        parents.setdefault(parent, []).append(pid)
        rss[pid] = kib
    found, pending = {root}, [root]
    while pending:
        for child in parents.get(pending.pop(), []):
            if child not in found:
                found.add(child)
                pending.append(child)
    return found, rss


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pid", required=True, type=int)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--interval", type=float, default=0.1)
    args = parser.parse_args()
    maximum = samples = 0
    started = time.time()
    while True:
        try:
            pids, rss = descendants(args.pid)
        except subprocess.CalledProcessError:
            pids, rss = {args.pid}, {}
        total = sum(rss.get(pid, 0) for pid in pids)
        maximum = max(maximum, total)
        samples += 1
        try:
            os.kill(args.pid, 0)
        except ProcessLookupError:
            break
        time.sleep(args.interval)
    payload = {
        "schema": "FEVC-PROCESS-TREE-RSS-V1",
        "root_pid": args.pid,
        "samples": samples,
        "interval_seconds": args.interval,
        "maximum_physical_rss_kib": maximum,
        "elapsed_seconds": time.time() - started,
    }
    temporary = args.output.with_suffix(args.output.suffix + ".tmp")
    temporary.write_text(json.dumps(payload, sort_keys=True) + "\n")
    temporary.replace(args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

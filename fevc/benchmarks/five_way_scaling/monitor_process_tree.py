#!/usr/bin/env python3
"""Sample whole-lifetime and primary-phase process-tree RSS on Linux."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import time
from pathlib import Path


def snapshot(proc: Path) -> dict[int, tuple[int, int]]:
    found: dict[int, tuple[int, int]] = {}
    for entry in proc.iterdir():
        if not entry.name.isdigit():
            continue
        try:
            tail = (entry / "stat").read_text(encoding="utf-8").rpartition(")")[2].split()
            ppid = int(tail[1])
            lines = (entry / "status").read_text(encoding="utf-8").splitlines()
            rss = next(line for line in lines if line.startswith("VmRSS:")).split()
            if len(rss) == 3 and rss[2] == "kB":
                found[int(entry.name)] = (ppid, int(rss[1]))
        except (OSError, StopIteration, ValueError, IndexError):
            pass
    return found


def descendants(processes: dict[int, tuple[int, int]], root: int) -> set[int]:
    selected = {root}
    changed = True
    while changed:
        changed = False
        for pid, (ppid, _) in processes.items():
            if pid not in selected and ppid in selected:
                selected.add(pid)
                changed = True
    return selected.intersection(processes)


def atomic(path: Path, value: dict[str, object]) -> None:
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, sort_keys=True) + "\n", encoding="utf-8")
    os.replace(temporary, path)


def monitor(root: int, output: Path, start: Path, end: Path,
            interval: float = 0.1, proc: Path = Path("/proc")) -> int:
    now = dt.datetime.now(dt.UTC).isoformat()
    record: dict[str, object] = {
        "schema": "FEVC-FIVE-WAY-PROCESS-TREE-V1", "status": "FAIL",
        "failure_message": "monitor incomplete", "root_pid": root,
        "sample_interval_seconds": interval, "sample_count": 0,
        "whole_peak_rss_kib": 0, "whole_peak_process_count": 0,
        "phase_sample_count": 0, "phase_peak_rss_kib": 0,
        "phase_peak_process_count": 0, "phase_start_observed": False,
        "phase_end_observed": False, "started_utc": now, "finished_utc": now,
    }
    try:
        if root <= 1 or not 0.05 <= interval <= 1:
            raise ValueError("invalid monitoring arguments")
        while True:
            processes = snapshot(proc)
            selected = descendants(processes, root)
            record["phase_start_observed"] = bool(record["phase_start_observed"] or start.is_file())
            record["phase_end_observed"] = bool(record["phase_end_observed"] or end.is_file())
            if not selected:
                break
            rss = sum(processes[pid][1] for pid in selected)
            record["sample_count"] = int(record["sample_count"]) + 1
            if rss > int(record["whole_peak_rss_kib"]):
                record["whole_peak_rss_kib"] = rss
                record["whole_peak_process_count"] = len(selected)
            if record["phase_start_observed"] and not record["phase_end_observed"]:
                record["phase_sample_count"] = int(record["phase_sample_count"]) + 1
                if rss > int(record["phase_peak_rss_kib"]):
                    record["phase_peak_rss_kib"] = rss
                    record["phase_peak_process_count"] = len(selected)
            time.sleep(interval)
        deadline = time.monotonic() + max(0.5, interval * 2)
        while not end.is_file() and time.monotonic() < deadline:
            time.sleep(min(interval, 0.05))
        record["phase_end_observed"] = end.is_file()
        if (not record["phase_start_observed"] or not record["phase_end_observed"] or
                int(record["phase_sample_count"]) < 1 or
                int(record["phase_peak_rss_kib"]) <= 0):
            raise ValueError("primary-phase RSS evidence is incomplete")
        record["status"] = "PASS"
        record["failure_message"] = "NONE"
    except (OSError, ValueError) as exc:
        record["failure_message"] = str(exc)
    record["finished_utc"] = dt.datetime.now(dt.UTC).isoformat()
    atomic(output, record)
    return 0 if record["status"] == "PASS" else 2


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root-pid", type=int, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--phase-start", type=Path, required=True)
    parser.add_argument("--phase-end", type=Path, required=True)
    parser.add_argument("--interval-seconds", type=float, default=0.1)
    parser.add_argument("--proc-root", type=Path, default=Path("/proc"))
    args = parser.parse_args()
    return monitor(args.root_pid, args.output, args.phase_start, args.phase_end,
                   args.interval_seconds, args.proc_root)


if __name__ == "__main__":
    raise SystemExit(main())

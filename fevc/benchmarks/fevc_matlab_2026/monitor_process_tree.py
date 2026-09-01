#!/usr/bin/env python3
"""Measure complete and estimator-phase RSS for one Linux process tree."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import statistics
import sys
import time
from pathlib import Path
from typing import Any

SCHEMA = "FEVC-MATLAB-2026-PROCESS-TREE-V1"


def process_snapshot(proc_root: Path) -> dict[int, tuple[int, int]]:
    result: dict[int, tuple[int, int]] = {}
    for item in proc_root.iterdir():
        if not item.name.isdigit():
            continue
        try:
            stat_text = (item / "stat").read_text(encoding="utf-8")
            _, separator, stat_tail = stat_text.rpartition(")")
            if not separator:
                continue
            fields = stat_tail.split()
            status = (item / "status").read_text(encoding="utf-8").splitlines()
            ppid = int(fields[1])
            rss = next(line for line in status if line.startswith("VmRSS:")).split()
            if len(rss) != 3 or rss[2] != "kB":
                continue
            result[int(item.name)] = (ppid, int(rss[1]))
        except (OSError, StopIteration, ValueError, IndexError):
            continue
    return result


def descendant_pids(
    snapshot: dict[int, tuple[int, int]], root_pid: int
) -> set[int]:
    selected = {root_pid}
    changed = True
    while changed:
        changed = False
        for pid, (ppid, _) in snapshot.items():
            if pid not in selected and ppid in selected:
                selected.add(pid)
                changed = True
    return selected.intersection(snapshot)


def process_tree_pss_kib(proc_root: Path, pids: set[int]) -> int:
    """Return summed Linux PSS for the currently observed process tree."""
    total = 0
    for pid in pids:
        try:
            lines = (proc_root / str(pid) / "smaps_rollup").read_text(
                encoding="utf-8", errors="replace").splitlines()
            field = next(line for line in lines if line.startswith("Pss:"))
            pieces = field.split()
            if len(pieces) != 3 or pieces[2] != "kB":
                raise ValueError("invalid PSS field")
            total += int(pieces[1])
        except (OSError, StopIteration, ValueError):
            continue
    return total


def load_identity(path: Path, expected_workers: int) -> dict[str, Any]:
    if path.is_symlink() or not path.is_file():
        raise ValueError("process identity is missing or a symlink")
    value = json.loads(path.read_text(encoding="utf-8"))
    if value.get("status") != "PASS":
        raise ValueError("process identity did not pass")
    if int(value.get("expected_pool_workers", -1)) != expected_workers:
        raise ValueError("process identity worker count changed")
    client = int(value.get("client_pid", -1))
    worker_value = value.get("worker_pids", [])
    # MATLAB's jsonencode serializes a one-element numeric vector as a JSON
    # number, while larger worker vectors remain arrays.  Normalize that
    # representation boundary before enforcing the registered worker count.
    if (isinstance(worker_value, (int, float)) and
            not isinstance(worker_value, bool)):
        worker_value = [worker_value]
    if not isinstance(worker_value, list):
        raise ValueError("process identity worker PIDs changed representation")
    workers = [int(item) for item in worker_value]
    if client <= 1 or len(workers) != expected_workers:
        raise ValueError("process identity PID count changed")
    named = [client, *workers]
    if len(set(named)) != len(named) or any(pid <= 1 for pid in named):
        raise ValueError("process identity PIDs are invalid")
    return {"client_pid": client, "worker_pids": workers, "named_pids": named}


def atomic_json(path: Path, value: dict[str, Any]) -> None:
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, sort_keys=True) + "\n", encoding="utf-8")
    os.replace(temporary, path)


def atomic_marker(path: Path, value: str) -> None:
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(value + "\n", encoding="utf-8")
    os.replace(temporary, path)


def observe_phase_markers(
    record: dict[str, Any], phase_start: Path, phase_end: Path
) -> bool:
    """Refresh marker evidence and return whether the measured phase is active."""

    if phase_start.is_file():
        record["phase_start_observed"] = True
    if phase_end.is_file():
        record["phase_end_observed"] = True
    return bool(record["phase_start_observed"] and
                not record["phase_end_observed"])


def monitor(
    root_pid: int,
    output: Path,
    *,
    interval: float,
    empty_ready: Path,
    data_ready: Path,
    empty_ack: Path,
    data_ack: Path,
    phase_start: Path,
    phase_end: Path,
    expected_workers: int | None = None,
    identity_path: Path | None = None,
    proc_root: Path = Path("/proc"),
) -> int:
    started = dt.datetime.now(dt.UTC).isoformat()
    record: dict[str, Any] = {
        "schema": SCHEMA,
        "status": "FAIL",
        "failure_message": "monitor did not complete",
        "root_pid": root_pid,
        "sample_interval_seconds": interval,
        "sample_count": 0,
        "whole_peak_rss_kib": 0,
        "whole_peak_process_count": 0,
        "empty_sample_count": 0,
        "empty_rss_kib_median": 0,
        "empty_rss_kib_last": 0,
        "empty_pss_kib_last": 0,
        "data_sample_count": 0,
        "data_rss_kib_median": 0,
        "data_rss_kib_last": 0,
        "data_pss_kib_last": 0,
        "phase_sample_count": 0,
        "phase_peak_rss_kib": 0,
        "phase_peak_process_count": 0,
        "estimator_start_pss_kib": 0,
        "estimator_end_pss_kib": 0,
        "empty_ready_observed": False,
        "data_ready_observed": False,
        "empty_ack_written": False,
        "data_ack_written": False,
        "phase_start_observed": False,
        "phase_end_observed": False,
        "expected_pool_workers": expected_workers,
        "identity_observation_count": 0,
        "identity_peak_rss_kib": 0,
        "identity_all_named_observed": identity_path is None,
        "started_utc": started,
        "finished_utc": started,
    }
    identity: dict[str, Any] | None = None
    empty_rss_samples: list[int] = []
    data_rss_samples: list[int] = []
    try:
        if root_pid <= 1:
            raise ValueError("root PID must exceed one")
        if not 0.05 <= interval <= 5.0:
            raise ValueError("sample interval is outside [0.05, 5]")
        if (identity_path is None) != (expected_workers is None):
            raise ValueError("identity path and worker count must be supplied together")
        if any(path.exists() for path in (empty_ack, data_ack)):
            raise ValueError("baseline acknowledgment already exists")
        if expected_workers is not None and expected_workers not in (1, 2, 4, 8, 14, 28):
            raise ValueError("unexpected worker count")
        phase_active = False
        while True:
            snapshot = process_snapshot(proc_root)
            selected = descendant_pids(snapshot, root_pid)
            # Stata writes phase.end immediately before its root process exits.
            # Refresh marker state before testing for an empty process tree so
            # that a marker created between the preceding sample and exit is
            # not lost to the final-poll race.
            phase_active = observe_phase_markers(record, phase_start, phase_end)
            if empty_ready.is_file():
                record["empty_ready_observed"] = True
            if data_ready.is_file():
                record["data_ready_observed"] = True
            if not selected:
                break
            if identity is None and identity_path is not None and identity_path.is_file():
                identity = load_identity(identity_path, int(expected_workers))
            rss_kib = sum(snapshot[pid][1] for pid in selected)
            record["sample_count"] += 1
            if rss_kib > record["whole_peak_rss_kib"]:
                record["whole_peak_rss_kib"] = rss_kib
                record["whole_peak_process_count"] = len(selected)
            empty_active = bool(record["empty_ready_observed"] and
                                not record["data_ready_observed"])
            data_active = bool(record["data_ready_observed"] and
                               not record["phase_start_observed"])
            if empty_active:
                empty_rss_samples.append(rss_kib)
                record["empty_sample_count"] += 1
                record["empty_rss_kib_last"] = rss_kib
                record["empty_pss_kib_last"] = process_tree_pss_kib(proc_root, selected)
                if len(empty_rss_samples) >= 2 and not record["empty_ack_written"]:
                    atomic_marker(empty_ack, "EMPTY_BASELINE_SAMPLED")
                    record["empty_ack_written"] = True
            if data_active:
                data_rss_samples.append(rss_kib)
                record["data_sample_count"] += 1
                record["data_rss_kib_last"] = rss_kib
                record["data_pss_kib_last"] = process_tree_pss_kib(proc_root, selected)
                if len(data_rss_samples) >= 2 and not record["data_ack_written"]:
                    atomic_marker(data_ack, "DATA_BASELINE_SAMPLED")
                    record["data_ack_written"] = True
            if phase_active:
                record["phase_sample_count"] += 1
                if rss_kib > record["phase_peak_rss_kib"]:
                    record["phase_peak_rss_kib"] = rss_kib
                    record["phase_peak_process_count"] = len(selected)
                if record["estimator_start_pss_kib"] == 0:
                    record["estimator_start_pss_kib"] = process_tree_pss_kib(
                        proc_root, selected)
            elif (record["phase_end_observed"] and
                  record["estimator_end_pss_kib"] == 0):
                record["estimator_end_pss_kib"] = process_tree_pss_kib(
                    proc_root, selected)
            if identity is not None:
                named = set(identity["named_pids"])
                if named.issubset(selected):
                    record["identity_observation_count"] += 1
                    record["identity_all_named_observed"] = True
                    record["identity_peak_rss_kib"] = max(
                        record["identity_peak_rss_kib"], rss_kib
                    )
            time.sleep(interval)
        # Allow a short bounded metadata-coherence grace period after the root
        # exits.  This is outside the measured estimator/process lifetime and
        # cannot inflate either RSS peak.
        marker_deadline = time.monotonic() + max(0.5, 2 * interval)
        while (record["phase_start_observed"] and
               not record["phase_end_observed"] and
               time.monotonic() < marker_deadline):
            time.sleep(min(0.05, interval))
            observe_phase_markers(record, phase_start, phase_end)
        if empty_rss_samples:
            record["empty_rss_kib_median"] = round(statistics.median(empty_rss_samples))
        if data_rss_samples:
            record["data_rss_kib_median"] = round(statistics.median(data_rss_samples))
        if record["sample_count"] < 1 or record["whole_peak_rss_kib"] <= 0:
            raise ValueError("no positive whole-process RSS sample")
        if (not record["empty_ready_observed"] or record["empty_sample_count"] < 2 or
                record["empty_rss_kib_median"] <= 0 or
                not record["empty_ack_written"]):
            raise ValueError("empty-runtime memory baseline was incomplete")
        if (not record["data_ready_observed"] or record["data_sample_count"] < 2 or
                record["data_rss_kib_median"] <= 0 or
                not record["data_ack_written"]):
            raise ValueError("loaded-data memory baseline was incomplete")
        if not record["phase_start_observed"] or not record["phase_end_observed"]:
            raise ValueError("estimator phase markers were incomplete")
        if record["phase_sample_count"] < 1 or record["phase_peak_rss_kib"] <= 0:
            raise ValueError("no positive estimator-phase RSS sample")
        if identity_path is not None:
            if identity is None or not record["identity_all_named_observed"]:
                raise ValueError("MATLAB client and workers were not observed together")
        record["status"] = "PASS"
        record["failure_message"] = "NONE"
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        record["failure_message"] = str(exc)
    record["finished_utc"] = dt.datetime.now(dt.UTC).isoformat()
    atomic_json(output, record)
    return 0 if record["status"] == "PASS" else 2


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root-pid", type=int, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--interval-seconds", type=float, default=0.25)
    parser.add_argument("--empty-ready", type=Path, required=True)
    parser.add_argument("--data-ready", type=Path, required=True)
    parser.add_argument("--empty-ack", type=Path, required=True)
    parser.add_argument("--data-ack", type=Path, required=True)
    parser.add_argument("--phase-start", type=Path, required=True)
    parser.add_argument("--phase-end", type=Path, required=True)
    parser.add_argument("--expected-pool-workers", type=int)
    parser.add_argument("--process-identity", type=Path)
    parser.add_argument("--proc-root", type=Path, default=Path("/proc"))
    return parser.parse_args(argv)


if __name__ == "__main__":
    args = parse_args()
    sys.exit(monitor(
        args.root_pid,
        args.output,
        interval=args.interval_seconds,
        empty_ready=args.empty_ready,
        data_ready=args.data_ready,
        empty_ack=args.empty_ack,
        data_ack=args.data_ack,
        phase_start=args.phase_start,
        phase_end=args.phase_end,
        expected_workers=args.expected_pool_workers,
        identity_path=args.process_identity,
        proc_root=args.proc_root,
    ))

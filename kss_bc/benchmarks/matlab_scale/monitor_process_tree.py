#!/usr/bin/env python3
"""Sample summed RSS for one Linux process and all current descendants."""

import argparse
import datetime as dt
import sys
import time
from pathlib import Path

from common import (
    PROCESS_TREE_SCHEMA,
    BenchmarkError,
    atomic_write_json,
    load_json,
    sha256_file,
    validate_process_identity,
)


def process_snapshot(proc_root):
    """Return ``pid -> (ppid, rss_kib)`` from one bounded /proc scan."""
    result = {}
    for item in Path(proc_root).iterdir():
        if not item.name.isdigit():
            continue
        try:
            stat_text = (item / "stat").read_text(encoding="utf-8")
            _, separator, stat_tail = stat_text.rpartition(")")
            if not separator:
                continue
            stat_fields = stat_tail.split()
            status_lines = (item / "status").read_text(encoding="utf-8").splitlines()
            ppid = int(stat_fields[1])
            rss_line = next(line for line in status_lines if line.startswith("VmRSS:"))
            rss_fields = rss_line.split()
            if len(rss_fields) != 3 or rss_fields[2] != "kB":
                continue
            result[int(item.name)] = (ppid, int(rss_fields[1]))
        # Linux may report ESRCH/ProcessLookupError, not only ENOENT, when a
        # process disappears between the directory scan and a procfs read.
        # That race invalidates only this process sample, not the monitor.
        except (OSError, StopIteration, ValueError, IndexError):
            continue
    return result


def descendant_pids(snapshot, root_pid):
    selected = {root_pid}
    changed = True
    while changed:
        changed = False
        for pid, (ppid, _) in snapshot.items():
            if pid not in selected and ppid in selected:
                selected.add(pid)
                changed = True
    return selected.intersection(snapshot)


def certified_identity_sample(snapshot, selected, identity):
    """Return summed descendant RSS only when every MATLAB PID is present."""
    named = set(identity["named_pids"])
    if not named.issubset(selected):
        return None
    return {
        "rss_kib": sum(snapshot[pid][1] for pid in selected),
        "process_count": len(selected),
    }


def monitor(
    root_pid,
    interval,
    output,
    *,
    expected_pool_workers,
    process_identity_path,
    proc_root="/proc",
):
    started = dt.datetime.now(dt.UTC).isoformat()
    minimum_process_count = expected_pool_workers + 1
    process_identity_path = Path(process_identity_path).absolute()
    record = {
        "schema": PROCESS_TREE_SCHEMA,
        "status": "FAIL",
        "failure_code": "KSS_MATLAB_SCALE_RSS_MONITOR_INCOMPLETE",
        "failure_message": "process-tree monitor did not observe a positive sample",
        "root_pid": root_pid,
        "expected_pool_workers": expected_pool_workers,
        "minimum_expected_process_count": minimum_process_count,
        "sample_interval_seconds": interval,
        "sample_count": 0,
        "peak_rss_kib": 0,
        "peak_rss_process_count": 0,
        "peak_process_count": 0,
        "process_identity_path": str(process_identity_path),
        "process_identity_sha256": None,
        "client_pid": None,
        "worker_pids": [],
        "named_pid_count": 0,
        "identity_observation_count": 0,
        "identity_peak_rss_kib": 0,
        "identity_peak_process_count": 0,
        "all_named_pids_observed_as_descendants": False,
        "started_utc": started,
        "finished_utc": started,
    }
    identity = None
    identity_sha256 = None
    try:
        if root_pid <= 1:
            raise ValueError("root PID must exceed one")
        if expected_pool_workers != 4:
            raise ValueError("expected pool size must be exactly four")
        if not (0.05 <= interval <= 5.0):
            raise ValueError("sample interval is outside [0.05, 5] seconds")
        while True:
            snapshot = process_snapshot(proc_root)
            selected = descendant_pids(snapshot, root_pid)
            if not selected:
                break
            if identity is None and process_identity_path.is_file():
                if process_identity_path.is_symlink():
                    raise ValueError("MATLAB process identity is a symlink")
                identity_sha256 = sha256_file(process_identity_path)
                identity = validate_process_identity(load_json(process_identity_path))
                record["process_identity_sha256"] = identity_sha256
                record["client_pid"] = identity["client_pid"]
                record["worker_pids"] = identity["worker_pids"]
                record["named_pid_count"] = len(identity["named_pids"])
            if identity is not None and sha256_file(process_identity_path) != identity_sha256:
                raise ValueError("MATLAB process identity changed during monitoring")
            rss_kib = sum(snapshot[pid][1] for pid in selected)
            record["sample_count"] += 1
            if rss_kib > record["peak_rss_kib"]:
                record["peak_rss_kib"] = rss_kib
                record["peak_rss_process_count"] = len(selected)
            record["peak_process_count"] = max(
                record["peak_process_count"], len(selected)
            )
            if identity is not None:
                certified = certified_identity_sample(snapshot, selected, identity)
                if certified is not None:
                    record["identity_observation_count"] += 1
                    if certified["rss_kib"] > record["identity_peak_rss_kib"]:
                        record["identity_peak_rss_kib"] = certified["rss_kib"]
                        record["identity_peak_process_count"] = certified["process_count"]
            time.sleep(interval)
        if record["sample_count"] < 1 or record["peak_rss_kib"] <= 0:
            raise ValueError("process-tree monitor observed no positive RSS sample")
        if identity is None:
            raise ValueError("MATLAB process identity artifact was not observed")
        if (
            not process_identity_path.is_file()
            or process_identity_path.is_symlink()
            or sha256_file(process_identity_path) != identity_sha256
        ):
            raise ValueError("MATLAB process identity changed after monitoring")
        if record["identity_observation_count"] < 1:
            raise ValueError("named MATLAB client and worker PIDs were not observed as descendants")
        record["all_named_pids_observed_as_descendants"] = True
        record["status"] = "PASS"
        record["failure_code"] = "NONE"
        record["failure_message"] = "NONE"
    except (BenchmarkError, OSError, ValueError) as exc:
        record["failure_code"] = "KSS_MATLAB_SCALE_RSS_MONITOR_REJECTED"
        record["failure_message"] = str(exc)
    record["finished_utc"] = dt.datetime.now(dt.UTC).isoformat()
    atomic_write_json(output, record)
    return 0 if record["status"] == "PASS" else 2


def parse_args(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument("--root-pid", type=int, required=True)
    parser.add_argument("--interval-seconds", type=float, default=0.25)
    parser.add_argument("--expected-pool-workers", type=int, required=True)
    parser.add_argument("--process-identity", required=True)
    parser.add_argument("--proc-root", default="/proc")
    parser.add_argument("--output", required=True)
    return parser.parse_args(argv)


if __name__ == "__main__":
    args = parse_args()
    sys.exit(
        monitor(
            args.root_pid,
            args.interval_seconds,
            args.output,
            expected_pool_workers=args.expected_pool_workers,
            process_identity_path=args.process_identity,
            proc_root=args.proc_root,
        )
    )

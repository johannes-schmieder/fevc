#!/usr/bin/env python3
"""Launch Stata in an isolated process group and always record what happened."""

from __future__ import annotations

import argparse
import datetime as dt
import fcntl
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z")


def write_json_atomic(path: Path, payload: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    temporary.replace(path)


def stop_process_group(process: subprocess.Popen[bytes]) -> None:
    if process.poll() is not None:
        return
    try:
        os.killpg(process.pid, signal.SIGTERM)
        process.wait(timeout=10)
    except (ProcessLookupError, subprocess.TimeoutExpired):
        if process.poll() is None:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.wait(timeout=10)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--timeout-seconds", required=True, type=int)
    parser.add_argument("--process-json", required=True, type=Path)
    parser.add_argument("--stdout-log", required=True, type=Path)
    parser.add_argument("--cwd", required=True, type=Path)
    parser.add_argument(
        "--lock-file",
        type=Path,
        default=Path("/private/tmp/varcomp-kss-stata-ci.lock"),
    )
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()

    command = args.command
    if command and command[0] == "--":
        command = command[1:]
    if not command:
        parser.error("a command is required after --")

    started_at = utc_now()
    started_monotonic = time.monotonic()
    result: dict[str, object] = {
        "schema_version": 1,
        "started_at": started_at,
        "command_executable": command[0],
        "launch_error": None,
        "timed_out": False,
        "process_rc": None,
    }

    args.stdout_log.parent.mkdir(parents=True, exist_ok=True)
    args.lock_file.parent.mkdir(parents=True, exist_ok=True)

    with args.lock_file.open("a+", encoding="utf-8") as lock_handle:
        fcntl.flock(lock_handle.fileno(), fcntl.LOCK_EX)
        with args.stdout_log.open("wb") as output_handle:
            try:
                process = subprocess.Popen(
                    command,
                    cwd=args.cwd,
                    stdin=subprocess.DEVNULL,
                    stdout=output_handle,
                    stderr=subprocess.STDOUT,
                    start_new_session=True,
                )
            except OSError as exc:
                result["launch_error"] = f"{type(exc).__name__}: {exc}"
            else:
                result["pid"] = process.pid
                try:
                    result["process_rc"] = process.wait(timeout=args.timeout_seconds)
                except subprocess.TimeoutExpired:
                    result["timed_out"] = True
                    stop_process_group(process)
                    result["process_rc"] = process.returncode

    result["completed_at"] = utc_now()
    result["duration_seconds"] = round(time.monotonic() - started_monotonic, 3)
    write_json_atomic(args.process_json, result)
    return 0


if __name__ == "__main__":
    sys.exit(main())

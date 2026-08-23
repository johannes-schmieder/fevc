#!/usr/bin/env python3
"""Build the authoritative machine-readable receipt for one Stata CI run."""

from __future__ import annotations

import argparse
import datetime as dt
import json
from pathlib import Path
import re
import sys


SHA_RE = re.compile(r"^[0-9a-f]{40}$")


def read_json(path: Path) -> dict[str, object]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ValueError(f"cannot read JSON from {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise ValueError(f"expected a JSON object in {path}")
    return value


def read_status(path: Path) -> tuple[dict[str, str], str | None]:
    if not path.is_file():
        return {}, "status file was not written"
    values: dict[str, str] = {}
    try:
        for raw_line in path.read_text(encoding="utf-8").splitlines():
            if not raw_line.strip():
                continue
            key, separator, value = raw_line.partition("=")
            if not separator or not key:
                return {}, f"malformed status line: {raw_line!r}"
            values[key] = value
    except OSError as exc:
        return {}, f"cannot read status file: {exc}"
    return values, None


def optional_int(value: object) -> int | None:
    if value is None:
        return None
    try:
        return int(value)
    except (TypeError, ValueError):
        return None


def write_json_atomic(path: Path, payload: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    temporary.replace(path)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--profile-config", required=True, type=Path)
    parser.add_argument("--profile", required=True)
    parser.add_argument("--process-json", required=True, type=Path)
    parser.add_argument("--status-file", required=True, type=Path)
    parser.add_argument("--run-dir", required=True, type=Path)
    parser.add_argument("--receipt", required=True, type=Path)
    parser.add_argument("--tested-sha", required=True)
    parser.add_argument("--repository", default="")
    parser.add_argument("--ref", default="")
    parser.add_argument("--run-id", default="local")
    parser.add_argument("--run-attempt", default="1")
    parser.add_argument("--runner-name", default="local")
    parser.add_argument("--stata-executable", required=True)
    parser.add_argument("--stata-bundle-version", default="unknown")
    args = parser.parse_args()

    if not SHA_RE.fullmatch(args.tested_sha):
        parser.error("--tested-sha must be a lowercase 40-character Git SHA")

    config = read_json(args.profile_config)
    profiles = config.get("profiles")
    if not isinstance(profiles, dict) or args.profile not in profiles:
        parser.error(f"unknown profile: {args.profile}")
    profile_config = profiles[args.profile]
    if not isinstance(profile_config, dict):
        parser.error(f"invalid profile configuration: {args.profile}")

    process = read_json(args.process_json)
    stata, status_error = read_status(args.status_file)
    process_rc = optional_int(process.get("process_rc"))
    stata_rc = optional_int(stata.get("stata_rc"))

    failure_kind: str | None = None
    failure_detail: str | None = None
    if process.get("launch_error"):
        failure_kind = "launch_error"
        failure_detail = str(process["launch_error"])
    elif process.get("timed_out") is True:
        failure_kind = "timeout"
        failure_detail = "Stata exceeded the profile timeout and its process group was stopped"
    elif status_error is not None:
        failure_kind = "crash" if process_rc not in (None, 0) else "missing_status"
        failure_detail = status_error
    elif stata.get("completed") != "1" or stata_rc is None:
        failure_kind = "missing_status"
        failure_detail = "status file did not contain a completed Stata result"
    elif stata_rc != 0:
        failure_kind = "stata_error"
        failure_detail = f"Stata test suite returned r({stata_rc})"
    elif process_rc not in (None, 0):
        failure_kind = "crash"
        failure_detail = f"Stata process exited with shell code {process_rc}"

    required_outputs = profile_config.get("required_outputs", [])
    output_checks: list[dict[str, object]] = []
    if not isinstance(required_outputs, list):
        parser.error("required_outputs must be a list")
    for relative_name in required_outputs:
        if not isinstance(relative_name, str):
            parser.error("required_outputs entries must be strings")
        candidate = args.run_dir / relative_name
        present = candidate.is_file()
        output_checks.append({"path": relative_name, "present": present})
        if failure_kind is None and not present:
            failure_kind = "missing_result"
            failure_detail = f"required result was absent: {relative_name}"

    status = "success" if failure_kind is None else "failure"
    platform = "; ".join(
        part
        for part in (
            stata.get("stata_os", ""),
            stata.get("stata_machine_type", ""),
        )
        if part
    ) or "unknown"

    receipt: dict[str, object] = {
        "schema_version": 1,
        "tested_sha": args.tested_sha,
        "repository": args.repository,
        "ref": args.ref,
        "run_id": args.run_id,
        "run_attempt": args.run_attempt,
        "profile": args.profile,
        "suite": profile_config.get("suite"),
        "status": status,
        "failure_kind": failure_kind,
        "failure_detail": failure_detail,
        "process_rc": process_rc,
        "stata_rc": stata_rc,
        "stata_version": stata.get("stata_version"),
        "stata_edition": stata.get("stata_edition"),
        "stata_bundle_version": args.stata_bundle_version,
        "stata_executable": args.stata_executable,
        "platform": platform,
        "stata_processors": optional_int(stata.get("stata_processors")),
        "runner_name": args.runner_name,
        "started_at": process.get("started_at"),
        "completed_at": process.get("completed_at", dt.datetime.now(dt.timezone.utc).isoformat()),
        "duration_seconds": process.get("duration_seconds"),
        "tests_passed": 1 if status == "success" else 0,
        "tests_failed": 0 if status == "success" else 1,
        "required_outputs": output_checks,
    }
    write_json_atomic(args.receipt, receipt)
    print(
        f"STATA_CI_RECEIPT status={status} profile={args.profile} "
        f"tested_sha={args.tested_sha} stata_rc={stata_rc} failure_kind={failure_kind}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())

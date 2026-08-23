#!/usr/bin/env python3
"""Build the exact-SHA JSON receipt for macOS Rust/Stata qualification."""

from __future__ import annotations

import argparse
import datetime as dt
import json
from pathlib import Path
import re
import sys


SHA_RE = re.compile(r"^[0-9a-f]{40}$")
STATA_ENV_RE = re.compile(
    r"^VCKSS_STATA_ENV version=(?P<version>\S+) edition=(?P<edition>\S+) "
    r"os=(?P<os>\S+) machine=(?P<machine>.+)$"
)


def read_json(path: Path) -> dict[str, object]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ValueError(f"cannot read JSON from {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise ValueError(f"expected a JSON object in {path}")
    return value


def read_qualification(path: Path) -> dict[str, str]:
    lines = path.read_text(encoding="utf-8").splitlines()
    if not lines or lines[0] != "VCKSS_MACOS_CANDIDATE_RECEIPT_V1":
        raise ValueError("qualification receipt has an invalid header")
    values: dict[str, str] = {}
    last_key: str | None = None
    for line in lines[1:]:
        key, separator, value = line.partition("=")
        if separator and key:
            values[key] = value
            last_key = key
        elif last_key is not None:
            # Utilities such as macOS `file` describe a universal binary on
            # one line per architecture.  Preserve those continuation lines
            # without weakening validation of the required PASS fields.
            values[last_key] += "\n" + line
        else:
            raise ValueError(f"malformed qualification receipt line: {line!r}")
    return values


def optional_int(value: object) -> int | None:
    if value is None:
        return None
    try:
        return int(value)
    except (TypeError, ValueError):
        return None


def failure_from_log(path: Path) -> str:
    try:
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    except OSError:
        return "macOS plugin qualifier did not complete successfully"
    prefix = "macOS Rust plugin qualification failed: "
    for line in reversed(lines):
        if line.startswith(prefix):
            return line[len(prefix) :]
    return "macOS plugin qualifier did not complete successfully"


def write_json_atomic(path: Path, payload: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    temporary.replace(path)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--profile", required=True)
    parser.add_argument("--process-json", required=True, type=Path)
    parser.add_argument("--process-log", required=True, type=Path)
    parser.add_argument("--qualification", required=True, type=Path)
    parser.add_argument("--receipt", required=True, type=Path)
    parser.add_argument("--tested-sha", required=True)
    parser.add_argument("--repository", default="")
    parser.add_argument("--ref", default="")
    parser.add_argument("--run-id", default="local")
    parser.add_argument("--run-attempt", default="1")
    parser.add_argument("--runner-name", default="local-mac")
    parser.add_argument("--stata-executable", required=True)
    parser.add_argument("--stata-bundle-version", default="unknown")
    args = parser.parse_args()

    if not SHA_RE.fullmatch(args.tested_sha):
        parser.error("--tested-sha must be a lowercase 40-character Git SHA")

    process = read_json(args.process_json)
    process_rc = optional_int(process.get("process_rc"))
    qualification: dict[str, str] = {}
    qualification_error: str | None = None
    if args.qualification.is_file():
        try:
            qualification = read_qualification(args.qualification)
        except (OSError, ValueError) as exc:
            qualification_error = str(exc)

    failure_kind: str | None = None
    failure_detail: str | None = None
    if process.get("launch_error"):
        failure_kind = "launch_error"
        failure_detail = str(process["launch_error"])
    elif process.get("timed_out") is True:
        failure_kind = "timeout"
        failure_detail = "plugin qualification exceeded its timeout and its process group was stopped"
    elif process_rc is None:
        failure_kind = "missing_process_status"
        failure_detail = "plugin qualifier process status was not recorded"
    elif process_rc not in (None, 0):
        failure_kind = "plugin_error"
        failure_detail = failure_from_log(args.process_log)
    elif qualification_error is not None:
        failure_kind = "invalid_result"
        failure_detail = qualification_error
    elif not qualification:
        failure_kind = "missing_result"
        failure_detail = "plugin qualifier did not write its sanitized receipt"
    elif qualification.get("commit") != args.tested_sha:
        failure_kind = "identity_mismatch"
        failure_detail = (
            f"qualification tested {qualification.get('commit')!r}, "
            f"expected {args.tested_sha}"
        )
    elif qualification.get("dirty_status_start") != "clean" or qualification.get(
        "dirty_status_end"
    ) != "clean":
        failure_kind = "dirty_checkout"
        failure_detail = "plugin qualification did not run from a clean tested checkout"
    else:
        required_passes = (
            "cargo_fmt",
            "cargo_clippy",
            "cargo_test",
            "cshim_interrupt_test",
            "cshim_error_transport_test",
            "abi_header_compat_test",
        )
        missing_passes = [
            key for key in required_passes if qualification.get(key) != "PASS"
        ]
        if missing_passes or qualification.get("arm64_test_status") != "PASS_NATIVE":
            failure_kind = "invalid_result"
            failure_detail = "qualification receipt omitted required PASS evidence"

    environment = STATA_ENV_RE.match(qualification.get("stata_arm64_environment", ""))
    stata_version = environment.group("version") if environment else None
    stata_edition = environment.group("edition") if environment else None
    platform = (
        f"{environment.group('os')}; {environment.group('machine')}"
        if environment
        else qualification.get("host_macos", "unknown")
    )
    status = "success" if failure_kind is None else "failure"
    receipt: dict[str, object] = {
        "schema_version": 1,
        "tested_sha": args.tested_sha,
        "repository": args.repository,
        "ref": args.ref,
        "run_id": args.run_id,
        "run_attempt": args.run_attempt,
        "profile": args.profile,
        "suite": "rust/stata_backend/qualify_macos.sh",
        "status": status,
        "failure_kind": failure_kind,
        "failure_detail": failure_detail,
        "process_rc": process_rc,
        "stata_rc": 0 if status == "success" else None,
        "stata_version": stata_version,
        "stata_edition": stata_edition,
        "stata_bundle_version": args.stata_bundle_version,
        "stata_executable": args.stata_executable,
        "platform": platform,
        "runner_name": args.runner_name,
        "started_at": process.get("started_at"),
        "completed_at": process.get(
            "completed_at", dt.datetime.now(dt.timezone.utc).isoformat()
        ),
        "duration_seconds": process.get("duration_seconds"),
        "tests_passed": 1 if status == "success" else 0,
        "tests_failed": 0 if status == "success" else 1,
        "qualification": {
            key: qualification.get(key)
            for key in (
                "classification",
                "scope",
                "cargo_fmt",
                "cargo_clippy",
                "cargo_test",
                "cshim_interrupt_test",
                "cshim_error_transport_test",
                "abi_header_compat_test",
                "rosetta_status",
                "arm64_test_status",
                "x86_64_test_status",
                "source_manifest_sha256",
                "artifact_arm64_sha256",
                "artifact_x86_64_sha256",
                "artifact_universal_sha256",
            )
            if qualification.get(key) is not None
        },
    }
    write_json_atomic(args.receipt, receipt)
    print(
        f"STATA_PLUGIN_CI_RECEIPT status={status} profile={args.profile} "
        f"tested_sha={args.tested_sha} failure_kind={failure_kind}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())

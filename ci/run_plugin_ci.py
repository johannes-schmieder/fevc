#!/usr/bin/env python3
"""Run the bounded macOS plugin qualifier and emit the common CI receipt."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys


DEFAULT_STATA = Path("/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp")
def command_output(command: list[str], cwd: Path) -> str:
    return subprocess.check_output(command, cwd=cwd, text=True).strip()


def clear_directory(path: Path) -> None:
    path.mkdir(parents=True, exist_ok=True)
    for child in path.iterdir():
        if child.is_dir() and not child.is_symlink():
            shutil.rmtree(child)
        else:
            child.unlink()


def bundle_version(stata_executable: Path) -> str:
    info_plist = stata_executable.parents[1] / "Info.plist"
    try:
        with info_plist.open("rb") as handle:
            value = plistlib.load(handle).get("CFBundleShortVersionString")
    except (OSError, plistlib.InvalidFileException):
        return "unknown"
    return str(value) if value is not None else "unknown"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("profile")
    args = parser.parse_args()

    root = Path(command_output(["git", "rev-parse", "--show-toplevel"], Path.cwd())).resolve()
    config = json.loads((root / "ci" / "stata_profiles.json").read_text(encoding="utf-8"))
    profile_config = config.get("profiles", {}).get(args.profile)
    if not isinstance(profile_config, dict) or profile_config.get("driver") != "plugin-qualifier":
        parser.error(f"profile {args.profile!r} is not a plugin-qualifier profile")
    timeout_seconds = int(profile_config["timeout_seconds"])
    artifact_dir = root / ".ci" / "stata" / "run"
    clear_directory(artifact_dir)
    evidence_dir = artifact_dir / "plugin-evidence"
    evidence_dir.mkdir()

    process_json = artifact_dir / "process.json"
    process_log = artifact_dir / "plugin-qualification.log"
    qualification = artifact_dir / "plugin-qualification.txt"
    receipt = artifact_dir / "receipt.json"
    tested_sha = os.environ.get("GITHUB_SHA") or command_output(["git", "rev-parse", "HEAD"], root)
    stata_executable = Path(os.environ.get("STATA_BIN", str(DEFAULT_STATA))).expanduser().resolve()

    qualifier_command = [
        str(root / "rust" / "stata_backend" / "qualify_macos.sh"),
        "--receipt",
        str(qualification),
        "--stata",
        str(stata_executable),
        "--artifacts-dir",
        str(evidence_dir),
    ]
    supervisor = [
        sys.executable,
        str(root / "ci" / "supervise_stata.py"),
        "--timeout-seconds",
        str(timeout_seconds),
        "--process-json",
        str(process_json),
        "--stdout-log",
        str(process_log),
        "--cwd",
        str(root),
        "--",
        *qualifier_command,
    ]
    supervisor_rc = subprocess.run(supervisor, check=False).returncode
    if supervisor_rc != 0 and not process_json.exists():
        process_json.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "started_at": None,
                    "completed_at": None,
                    "duration_seconds": None,
                    "process_rc": supervisor_rc,
                    "timed_out": False,
                    "launch_error": f"qualifier supervisor exited with code {supervisor_rc}",
                },
                indent=2,
            )
            + "\n",
            encoding="utf-8",
        )

    make_receipt = [
        sys.executable,
        str(root / "ci" / "make_plugin_receipt.py"),
        "--profile",
        args.profile,
        "--process-json",
        str(process_json),
        "--process-log",
        str(process_log),
        "--qualification",
        str(qualification),
        "--receipt",
        str(receipt),
        "--tested-sha",
        tested_sha,
        "--repository",
        os.environ.get("GITHUB_REPOSITORY", "local/fevc"),
        "--ref",
        os.environ.get("GITHUB_REF", "local"),
        "--run-id",
        os.environ.get("GITHUB_RUN_ID", "local"),
        "--run-attempt",
        os.environ.get("GITHUB_RUN_ATTEMPT", "1"),
        "--runner-name",
        os.environ.get("RUNNER_NAME", "local-mac"),
        "--stata-executable",
        str(stata_executable),
        "--stata-bundle-version",
        bundle_version(stata_executable),
    ]
    make_rc = subprocess.run(make_receipt, check=False).returncode
    if make_rc != 0:
        return make_rc

    return subprocess.run(
        [
            sys.executable,
            str(root / "ci" / "check_stata_receipt.py"),
            str(receipt),
            "--expect-tested-sha",
            tested_sha,
            "--expect-profile",
            args.profile,
            "--require-success",
        ],
        check=False,
    ).returncode


if __name__ == "__main__":
    sys.exit(main())

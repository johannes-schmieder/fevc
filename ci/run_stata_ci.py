#!/usr/bin/env python3
"""Stage the repository, run one Stata profile, and emit a durable receipt."""

from __future__ import annotations

import argparse
import errno
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys
import tempfile
import time


DEFAULT_STATA = Path("/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp")
RUN_DIRECTORY_REMOVAL_ATTEMPTS = 6
RUN_DIRECTORY_REMOVAL_DELAY_SECONDS = 0.25


def command_output(command: list[str], cwd: Path) -> str:
    return subprocess.check_output(command, cwd=cwd, text=True).strip()


def repo_root() -> Path:
    return Path(command_output(["git", "rev-parse", "--show-toplevel"], Path.cwd())).resolve()


def tracked_and_untracked_files(root: Path) -> list[Path]:
    raw = subprocess.check_output(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
        cwd=root,
    )
    files: list[Path] = []
    for name in raw.split(b"\0"):
        if not name:
            continue
        relative = Path(os.fsdecode(name))
        source = (root / relative).resolve()
        if source.is_file() and source.is_relative_to(root):
            files.append(relative)
    return files


def stage_repository(root: Path, destination: Path) -> None:
    for relative in tracked_and_untracked_files(root):
        source = root / relative
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, target)


def remove_tree_with_retries(
    path: Path,
    *,
    attempts: int = RUN_DIRECTORY_REMOVAL_ATTEMPTS,
    delay_seconds: float = RUN_DIRECTORY_REMOVAL_DELAY_SECONDS,
) -> None:
    """Remove a tree, tolerating a bounded macOS ENOTEMPTY race."""
    if attempts < 1:
        raise ValueError("attempts must be positive")
    for attempt in range(attempts):
        try:
            shutil.rmtree(path)
        except FileNotFoundError:
            return
        except OSError as exc:
            if exc.errno != errno.ENOTEMPTY or attempt + 1 == attempts:
                raise
            time.sleep(delay_seconds * (attempt + 1))
        else:
            return


def clear_artifact_directory(path: Path) -> None:
    path.mkdir(parents=True, exist_ok=True)
    for child in path.iterdir():
        if child.is_dir() and not child.is_symlink():
            remove_tree_with_retries(child)
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


def seed_stata_plus(profile: dict[str, object], run_root: Path) -> None:
    source_root = Path(
        os.environ.get(
            "STATA_PLUS_SOURCE",
            "~/Library/Application Support/Stata/ado/plus",
        )
    ).expanduser().resolve()
    destination_root = (run_root / "stata-plus").resolve()
    for raw_name in profile.get("stata_plus_files", []):
        if not isinstance(raw_name, str):
            raise ValueError("stata_plus_files entries must be strings")
        relative = Path(raw_name)
        source = (source_root / relative).resolve()
        destination = (destination_root / relative).resolve()
        if not source.is_relative_to(source_root) or not destination.is_relative_to(destination_root):
            raise ValueError(f"unsafe Stata PLUS dependency path: {raw_name}")
        if not source.is_file():
            print(f"STATA_CI_DEPENDENCY_MISSING={source}", file=sys.stderr)
            continue
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, destination)
        print(f"STATA_CI_DEPENDENCY={raw_name}")


def copy_run_evidence(run_root: Path, artifact_dir: Path) -> None:
    for source in sorted(run_root.iterdir()):
        if source.is_file():
            shutil.copy2(source, artifact_dir / source.name)


def tail(path: Path, lines: int = 80) -> str:
    try:
        values = path.read_text(encoding="utf-8", errors="replace").splitlines()
    except OSError:
        return ""
    return "\n".join(values[-lines:])


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("profile", nargs="?", default="smoke")
    args = parser.parse_args()

    root = repo_root()
    config_path = root / "ci" / "stata_profiles.json"
    config = json.loads(config_path.read_text(encoding="utf-8"))
    profiles = config.get("profiles", {})
    if args.profile not in profiles:
        choices = ", ".join(sorted(profiles))
        parser.error(f"unknown profile {args.profile!r}; choose one of: {choices}")
    profile = profiles[args.profile]
    if profile.get("driver", "stata") != "stata":
        parser.error(
            f"profile {args.profile!r} uses the plugin qualifier; "
            "invoke it through ci/run_ci_profile.sh"
        )

    artifact_dir = root / ".ci" / "stata" / "run"
    clear_artifact_directory(artifact_dir)

    parent = Path(os.environ.get("RUNNER_TEMP", tempfile.gettempdir())).resolve()
    parent.mkdir(parents=True, exist_ok=True)
    run_root = Path(tempfile.mkdtemp(prefix="vckss-stata-ci-", dir=parent))
    staged_root = run_root / "repo"
    staged_root.mkdir()
    stage_repository(root, staged_root)
    for directory in ("stata-plus", "stata-personal", "stata-oldplace", "output"):
        (run_root / directory).mkdir()
    seed_stata_plus(profile, run_root)

    tested_sha = os.environ.get("GITHUB_SHA") or command_output(["git", "rev-parse", "HEAD"], root)
    stata_executable = Path(os.environ.get("STATA_BIN", str(DEFAULT_STATA))).expanduser().resolve()
    status_file = run_root / "stata.status"
    stata_log = run_root / "stata.log"
    process_json = run_root / "process.json"
    process_log = run_root / "process.stdout.log"
    suite_file = staged_root / str(profile["suite"])
    timeout_seconds = int(profile["timeout_seconds"])

    stata_command = [
        str(stata_executable),
        "-q",
        "-b",
        "do",
        str(staged_root / "ci" / "stata_ci.do"),
        str(suite_file),
        str(status_file),
        str(stata_log),
        args.profile,
        str(staged_root),
        str(run_root),
        *[str(value) for value in profile.get("arguments", [])],
    ]

    supervisor = [
        sys.executable,
        str(staged_root / "ci" / "supervise_stata.py"),
        "--timeout-seconds",
        str(timeout_seconds),
        "--process-json",
        str(process_json),
        "--stdout-log",
        str(process_log),
        "--cwd",
        str(run_root),
        "--",
        *stata_command,
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
                    "launch_error": f"Stata supervisor exited with code {supervisor_rc}",
                },
                indent=2,
            )
            + "\n",
            encoding="utf-8",
        )

    copy_run_evidence(run_root, artifact_dir)
    receipt_path = artifact_dir / "receipt.json"
    make_receipt = [
        sys.executable,
        str(staged_root / "ci" / "make_stata_receipt.py"),
        "--profile-config",
        str(staged_root / "ci" / "stata_profiles.json"),
        "--profile",
        args.profile,
        "--process-json",
        str(process_json),
        "--status-file",
        str(status_file),
        "--run-dir",
        str(run_root),
        "--receipt",
        str(receipt_path),
        "--tested-sha",
        tested_sha,
        "--repository",
        os.environ.get("GITHUB_REPOSITORY", "local/vckss"),
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
    receipt_rc = subprocess.run(make_receipt, check=False).returncode
    copy_run_evidence(run_root, artifact_dir)

    keep_run = os.environ.get("STATA_CI_KEEP_TEMP") == "1"
    if keep_run:
        print(f"STATA_CI_TEMP={run_root}")
    else:
        remove_tree_with_retries(run_root)

    if receipt_rc != 0:
        return receipt_rc

    checker = [
        sys.executable,
        str(root / "ci" / "check_stata_receipt.py"),
        str(receipt_path),
        "--expect-tested-sha",
        tested_sha,
        "--expect-profile",
        args.profile,
        "--require-success",
    ]
    checker_result = subprocess.run(checker, check=False)
    if checker_result.returncode != 0:
        log_tail = tail(artifact_dir / "stata.log")
        if log_tail:
            print("--- Stata log tail ---", file=sys.stderr)
            print(log_tail, file=sys.stderr)
    return checker_result.returncode


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Guarantee a valid exact-SHA receipt for every completed licensed-CI job."""

from __future__ import annotations

import argparse
import datetime as dt
import json
from pathlib import Path
import re
import sys
from typing import Any


SHA_RE = re.compile(r"^[0-9a-f]{40}$")
VALID_OUTCOMES = {"success", "failure", "skipped", "cancelled"}
REQUIRED_RECEIPT_KEYS = {
    "schema_version",
    "tested_sha",
    "profile",
    "status",
    "process_rc",
    "stata_rc",
    "run_id",
    "platform",
}


def write_json_atomic(path: Path, payload: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    temporary.replace(path)


def read_valid_receipt(
    path: Path, *, tested_sha: str, profile: str
) -> tuple[dict[str, object] | None, str | None]:
    if not path.is_file():
        return None, "receipt was not written"
    try:
        value: Any = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        return None, f"receipt could not be read: {exc}"
    if not isinstance(value, dict):
        return None, "receipt root is not an object"
    missing = sorted(REQUIRED_RECEIPT_KEYS.difference(value))
    if missing:
        return None, f"receipt is missing required keys: {', '.join(missing)}"
    if value.get("schema_version") != 1:
        return None, "receipt has an unsupported schema_version"
    if value.get("tested_sha") != tested_sha:
        return None, "receipt tested_sha does not match the workflow source SHA"
    if value.get("profile") != profile:
        return None, "receipt profile does not match the resolved workflow profile"
    if value.get("status") not in {"success", "failure"}:
        return None, "receipt status is not success or failure"
    return value, None


def profile_metadata(
    path: Path, profile: str
) -> tuple[object | None, list[dict[str, object]], str | None]:
    try:
        config: Any = json.loads(path.read_text(encoding="utf-8"))
        profiles = config["profiles"]
        profile_config = profiles[profile]
        if not isinstance(profile_config, dict):
            raise TypeError("profile configuration is not an object")
        required = profile_config.get("required_outputs", [])
        if not isinstance(required, list) or not all(
            isinstance(value, str) for value in required
        ):
            raise TypeError("required_outputs is not a string list")
        checks = [{"path": value, "present": False} for value in required]
        return profile_config.get("suite"), checks, None
    except (OSError, KeyError, TypeError, json.JSONDecodeError) as exc:
        return None, [], f"profile metadata unavailable: {exc}"


def normalize_outcome(value: str) -> str:
    normalized = value.strip().lower()
    return normalized if normalized in VALID_OUTCOMES else "unknown"


def failure_classification(
    outcomes: dict[str, str], *, rust_quick_required: bool
) -> tuple[str, str]:
    if outcomes["resolve_profile"] != "success":
        kind = "profile_resolution"
        lead = "the trusted profile-selection gate did not complete successfully"
    elif outcomes["receipt_tests"] != "success":
        kind = "pre_stata_gate"
        lead = "a pre-Stata regression or receipt-machinery gate failed"
    elif outcomes["licensed_profile"] == "skipped":
        kind = "licensed_profile_skipped"
        lead = "the licensed Stata profile was skipped before producing a receipt"
    elif outcomes["licensed_profile"] != "success":
        kind = "ci_harness_error"
        lead = "the licensed profile step ended without a valid receipt"
    elif rust_quick_required and outcomes["rust_quick"] != "success":
        kind = "rust_gate"
        lead = "the required quick Rust gate failed without a valid lane receipt"
    else:
        kind = "missing_receipt"
        lead = "the licensed CI lane completed without a valid receipt"
    detail = lead + "; step outcomes: " + ", ".join(
        f"{name}={value}" for name, value in outcomes.items()
    )
    return kind, detail


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--receipt", required=True, type=Path)
    parser.add_argument("--profile-config", required=True, type=Path)
    parser.add_argument("--profile", required=True)
    parser.add_argument("--tested-sha", required=True)
    parser.add_argument("--repository", default="")
    parser.add_argument("--ref", default="")
    parser.add_argument("--run-id", default="local")
    parser.add_argument("--run-attempt", default="1")
    parser.add_argument("--runner-name", default="local")
    parser.add_argument("--stata-executable", default="unknown")
    parser.add_argument("--resolve-profile-outcome", default="unknown")
    parser.add_argument("--receipt-tests-outcome", default="unknown")
    parser.add_argument("--licensed-profile-outcome", default="unknown")
    parser.add_argument("--rust-quick-outcome", default="skipped")
    parser.add_argument("--rust-quick-required", action="store_true")
    args = parser.parse_args()

    if not SHA_RE.fullmatch(args.tested_sha):
        parser.error("--tested-sha must be a lowercase 40-character Git SHA")
    profile = args.profile.strip()
    if not profile:
        parser.error("--profile must not be empty")

    existing, receipt_error = read_valid_receipt(
        args.receipt,
        tested_sha=args.tested_sha,
        profile=profile,
    )
    if existing is not None:
        print(
            "STATA_CI_FINALIZE preserved "
            f"status={existing['status']} profile={profile} tested_sha={args.tested_sha}"
        )
        return 0

    outcomes = {
        "resolve_profile": normalize_outcome(args.resolve_profile_outcome),
        "receipt_tests": normalize_outcome(args.receipt_tests_outcome),
        "licensed_profile": normalize_outcome(args.licensed_profile_outcome),
        "rust_quick": normalize_outcome(args.rust_quick_outcome),
    }
    failure_kind, failure_detail = failure_classification(
        outcomes,
        rust_quick_required=args.rust_quick_required,
    )
    if receipt_error:
        failure_detail += f"; receipt diagnosis: {receipt_error}"

    suite, required_outputs, profile_error = profile_metadata(
        args.profile_config,
        profile,
    )
    if profile_error:
        failure_detail += f"; {profile_error}"

    receipt: dict[str, object] = {
        "schema_version": 1,
        "tested_sha": args.tested_sha,
        "repository": args.repository,
        "ref": args.ref,
        "run_id": args.run_id,
        "run_attempt": args.run_attempt,
        "profile": profile,
        "suite": suite,
        "status": "failure",
        "failure_kind": failure_kind,
        "failure_detail": failure_detail,
        "process_rc": None,
        "stata_rc": None,
        "stata_version": None,
        "stata_edition": None,
        "stata_bundle_version": "unknown",
        "stata_executable": args.stata_executable,
        "platform": "not-run",
        "stata_processors": None,
        "runner_name": args.runner_name,
        "started_at": None,
        "completed_at": dt.datetime.now(dt.timezone.utc).isoformat(),
        "duration_seconds": None,
        "tests_passed": 0,
        "tests_failed": 1,
        "required_outputs": required_outputs,
        "ci_step_outcomes": outcomes,
        "synthetic_receipt": True,
    }
    write_json_atomic(args.receipt, receipt)
    print(
        "STATA_CI_FINALIZE synthesized "
        f"status=failure profile={profile} tested_sha={args.tested_sha} "
        f"failure_kind={failure_kind}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())

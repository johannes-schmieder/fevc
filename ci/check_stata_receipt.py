#!/usr/bin/env python3
"""Validate a Stata CI receipt and optionally require a successful result."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import sys


SHA_RE = re.compile(r"^[0-9a-f]{40}$")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("receipt", type=Path)
    parser.add_argument("--expect-tested-sha")
    parser.add_argument("--expect-profile")
    parser.add_argument("--require-success", action="store_true")
    args = parser.parse_args()

    try:
        receipt = json.loads(args.receipt.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        print(f"invalid Stata receipt: {exc}", file=sys.stderr)
        return 2
    if not isinstance(receipt, dict):
        print("invalid Stata receipt: root must be an object", file=sys.stderr)
        return 2

    required = {
        "schema_version",
        "tested_sha",
        "profile",
        "status",
        "process_rc",
        "stata_rc",
        "run_id",
        "platform",
    }
    missing = sorted(required.difference(receipt))
    if missing:
        print(f"invalid Stata receipt: missing {', '.join(missing)}", file=sys.stderr)
        return 2
    if receipt["schema_version"] != 1:
        print("invalid Stata receipt: unsupported schema_version", file=sys.stderr)
        return 2
    tested_sha = receipt["tested_sha"]
    if not isinstance(tested_sha, str) or not SHA_RE.fullmatch(tested_sha):
        print("invalid Stata receipt: tested_sha is not a full Git SHA", file=sys.stderr)
        return 2
    if receipt["status"] not in {"success", "failure"}:
        print("invalid Stata receipt: status must be success or failure", file=sys.stderr)
        return 2
    if args.expect_tested_sha and tested_sha != args.expect_tested_sha:
        print(
            f"Stata receipt SHA mismatch: expected {args.expect_tested_sha}, got {tested_sha}",
            file=sys.stderr,
        )
        return 3
    if args.expect_profile and receipt["profile"] != args.expect_profile:
        print(
            f"Stata receipt profile mismatch: expected {args.expect_profile}, got {receipt['profile']}",
            file=sys.stderr,
        )
        return 3
    if args.require_success and receipt["status"] != "success":
        print(
            f"Stata CI failed: kind={receipt.get('failure_kind')} "
            f"stata_rc={receipt.get('stata_rc')} detail={receipt.get('failure_detail')}",
            file=sys.stderr,
        )
        return 1

    print(
        f"valid Stata receipt: status={receipt['status']} profile={receipt['profile']} "
        f"tested_sha={tested_sha}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Generate and freeze the 16-cell bounded development screen inputs."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from campaign import PROFILES, protocol
from development_input import build
from generate import sha


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--candidate-source-sha256", required=True)
    parser.add_argument("--candidate-plugin", required=True, type=Path)
    parser.add_argument("--baseline-snapshot", required=True, type=Path)
    args = parser.parse_args()
    if args.output.exists():
        parser.error("--output is write-once")
    for value in (args.candidate_source_sha256, sha(args.candidate_plugin)):
        if len(value) != 64 or any(character not in "0123456789abcdef" for character in value):
            parser.error("candidate hashes must be lowercase SHA-256")
    if sha(args.baseline_snapshot) != protocol()["baseline_manifest_sha256"]:
        parser.error("baseline snapshot differs from the frozen comparison source")
    inputs = args.output / "inputs"
    inputs.mkdir(parents=True)
    receipts = {}
    for index, profile in enumerate(PROFILES):
        rows = 8_000 if index >= 5 else 100_000
        receipt = build(inputs / f"{profile}.csv", profile, rows)
        receipts[profile] = receipt
        (inputs / f"{profile}.csv.sha256").write_text(receipt["sha256"] + "\n")
    definition = protocol()
    definition["status"] = "FROZEN_DEVELOPMENT_READY"
    definition["candidate_source"] = args.candidate_source_sha256
    definition["candidate_binaries"] = {
        "linux_x86_64": sha(args.candidate_plugin)
    }
    definition["input_hashes"] = {
        profile: receipt["sha256"] for profile, receipt in receipts.items()
    }
    definition["baseline_snapshot_sha256"] = sha(args.baseline_snapshot)
    definition["development_input_generator_sha256"] = sha(
        Path(__file__).with_name("development_input.py")
    )
    definition["development_stata_sha256"] = sha(
        Path(__file__).with_name("development_stata.do")
    )
    definition["development_call_sha256"] = sha(
        Path(__file__).with_name("development_call.sh")
    )
    (args.output / "development-manifest.json").write_text(
        json.dumps(definition, indent=2, sort_keys=True, allow_nan=False) + "\n"
    )
    print(
        "FEVC_OPTIMIZATION_DEVELOPMENT_FROZEN "
        f"manifest_sha256={sha(args.output / 'development-manifest.json')} "
        f"inputs={len(receipts)} timed=96 warmups=32"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())


#!/usr/bin/env python3
"""Admit the full larger-than-CZ18 stress run from its bound calibration."""
from __future__ import annotations

import argparse
import csv
import hashlib
import math
import os
import re
from pathlib import Path


HEX64 = re.compile(r"[0-9a-f]{64}")
HEX40 = re.compile(r"[0-9a-f]{40}")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def finite(row: dict[str, str], field: str) -> float:
    value = float(row[field])
    require(math.isfinite(value), f"nonfinite {field}")
    return value


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--calibration", required=True, type=Path)
    parser.add_argument("--retained-sha-file", required=True, type=Path)
    parser.add_argument("--bundle-sha", required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--manifest-sha", required=True)
    parser.add_argument("--timeout", required=True, type=int)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    require(HEX64.fullmatch(args.bundle_sha) is not None, "invalid bundle SHA")
    require(HEX40.fullmatch(args.source_commit) is not None, "invalid source commit")
    require(HEX64.fullmatch(args.manifest_sha) is not None, "invalid manifest SHA")
    require(60 <= args.timeout <= 10800, "invalid stress timeout")
    with args.calibration.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    require(len(rows) == 1, "stress calibration must contain exactly one row")
    row = rows[0]
    retained_sha = args.retained_sha_file.read_text(encoding="utf-8").strip()
    require(HEX64.fullmatch(retained_sha) is not None, "invalid retained-sample SHA")
    require(row["experiment_id"] == "cz18_stress2x_cal20" and
            row["stage"] == "stress2x" and row["temperature"] == "cold" and
            int(finite(row, "repetition")) == 1, "wrong stress calibration row")
    require(row["bundle_sha256"] == args.bundle_sha and
            row["source_commit"] == args.source_commit and
            row["data_manifest_sha256"] == args.manifest_sha,
            "stress calibration source identity changed")
    require(row["estimator_input_sha256"] == retained_sha,
            "stress calibration used another retained CZ18 input")
    require(row["estimator_status"] == "KSS_POINT_ESTIMATES_ONLY" and
            finite(row, "command_rc") == 0, "stress calibration failed")
    require(row["algorithm_requested"] == row["algorithm_selected"] == "jla" and
            row["preconditioner_selected"] == "cmg",
            "stress calibration did not use CMG JLA")
    require(int(finite(row, "requested_probes")) == 20 and
            abs(finite(row, "tolerance") - 1e-10) <= 1e-20 and
            0 <= finite(row, "solver_max_residual") <= 1e-9 * (1 + 1e-10),
            "stress calibration changed probes/tolerance/residual gate")
    command_seconds = finite(row, "command_seconds")
    require(command_seconds >= 0, "negative stress calibration time")
    projection = math.ceil(command_seconds * (200 / 20) * 1.5 + 120)
    require(projection <= args.timeout,
            "stress calibration does not justify the full-run timeout")
    payload = (
        "status=PASS\n"
        "calibration=cz18_stress2x_cal20\n"
        f"calibration_sha256={sha256(args.calibration)}\n"
        f"estimator_input_sha256={retained_sha}\n"
        f"projected_seconds={projection}\n"
        f"timeout_seconds={args.timeout}\n"
        "formula=ceil(calibration_seconds*(200/20)*1.5+120)\n"
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    temporary = args.output.with_name(args.output.name + f".tmp.{os.getpid()}")
    temporary.write_text(payload, encoding="utf-8")
    os.replace(temporary, args.output)
    print("KSS_PROD STRESS PROJECTION PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

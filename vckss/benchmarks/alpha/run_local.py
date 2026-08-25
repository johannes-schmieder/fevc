#!/usr/bin/env python3
"""Run VCKSS-ALPHA-BENCH-V1 local cases in fresh Stata processes."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import os
import platform
import re
import socket
import statistics
import subprocess
import sys
from datetime import UTC, datetime
from pathlib import Path

MARKER = "VCKSS_ALPHA_BENCH_V1_PASS"
REQUIRED_ROWS = 4


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def git(repo: Path, *args: str) -> str:
    return subprocess.run(
        ["git", *args],
        cwd=repo,
        check=True,
        text=True,
        capture_output=True,
    ).stdout.strip()


def read_cases(path: Path) -> dict[str, dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle, delimiter="\t"))
    cases = {row["case_id"]: row for row in rows}
    if len(cases) != len(rows):
        raise RuntimeError("duplicate case_id in cases.tsv")
    return cases


def parse_peak_rss(path: Path) -> int:
    text = path.read_text(encoding="utf-8")
    match = re.search(r"^\s*(\d+)\s+maximum resident set size\s*$", text, re.M)
    if not match:
        raise RuntimeError(f"could not parse macOS peak RSS from {path}")
    return int(match.group(1))


def validate_rows(
    path: Path,
    *,
    case_id: str,
    backend: str,
    source_commit: str,
    fixture_sha: str,
) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    if len(rows) != REQUIRED_ROWS:
        raise RuntimeError(f"{case_id}/{backend}: expected {REQUIRED_ROWS} rows")
    for index, row in enumerate(rows, 1):
        expected_temperature = "cold" if index == 1 else "warm"
        if (
            row["case_id"] != case_id
            or row["backend"] != backend
            or row["source_commit"] != source_commit
            or row["fixture_spec_sha256"] != fixture_sha
            or int(float(row["run"])) != index
            or row["temperature"] != expected_temperature
        ):
            raise RuntimeError(f"{case_id}/{backend}: row binding failed")
        for field in ("sample_ok", "data_ok", "rng_ok", "sort_ok"):
            if int(float(row[field])) != 1:
                raise RuntimeError(f"{case_id}/{backend}: failed gate {field}")
        if float(row["result_diff"]) != 0:
            raise RuntimeError(f"{case_id}/{backend}: repeated result drift")
        if float(row["max_resid"]) > float(row["accept_tol"]):
            raise RuntimeError(f"{case_id}/{backend}: residual gate failed")
        if abs(float(row["identity_resid"])) > 1e-12:
            raise RuntimeError(f"{case_id}/{backend}: accounting gate failed")
        if row["estimator_status"] != "KSS_SCALE_EXPERIMENTAL_POINT_ESTIMATES":
            raise RuntimeError(f"{case_id}/{backend}: unexpected estimator status")
    return rows


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--case", action="append", dest="cases")
    parser.add_argument(
        "--stata",
        default=os.environ.get(
            "VCKSS_STATA",
            "/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp",
        ),
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo = Path(__file__).resolve().parents[3]
    if git(repo, "status", "--porcelain"):
        raise RuntimeError("alpha benchmark source worktree must be clean")
    source_commit = git(repo, "rev-parse", "HEAD")
    source_tree = git(repo, "rev-parse", "HEAD^{tree}")
    case_path = Path(__file__).with_name("cases.tsv")
    driver = Path(__file__).with_name("synthetic_driver.do")
    fixture_sha = sha256(case_path)
    cases = read_cases(case_path)
    selected = args.cases or [
        case_id for case_id, case in cases.items() if case["local"] == "1"
    ]
    unknown = sorted(set(selected) - set(cases))
    if unknown:
        raise RuntimeError(f"unknown case(s): {', '.join(unknown)}")
    if any(cases[case_id]["local"] != "1" for case_id in selected):
        raise RuntimeError("SCC-only cases cannot run through run_local.py")
    stata = Path(args.stata).resolve()
    if not stata.is_file():
        raise RuntimeError(f"Stata executable not found: {stata}")
    if args.output.exists() and any(args.output.iterdir()):
        raise RuntimeError("output directory must be new or empty")
    args.output.mkdir(parents=True, exist_ok=True)

    all_rows: list[dict[str, str]] = []
    artifacts: list[dict[str, object]] = []
    for case_id in selected:
        case = cases[case_id]
        for backend in ("rust", "mata"):
            stem = f"{case_id}-{backend}"
            raw_csv = args.output / f"{stem}.csv"
            log = args.output / f"{stem}.log"
            resources = args.output / f"{stem}.resources.txt"
            command = [
                "/usr/bin/time",
                "-l",
                "-o",
                str(resources),
                str(stata),
                "-q",
                "do",
                str(driver),
                str(repo),
                str(raw_csv),
                case_id,
                source_commit,
                fixture_sha,
                backend,
                case["workers"],
                case["firms"],
                case["probes"],
                case["controls"],
                case["deletion"],
                str(REQUIRED_ROWS),
            ]
            completed = subprocess.run(
                command,
                cwd=args.output,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                check=False,
            )
            log.write_text(completed.stdout, encoding="utf-8")
            expected_marker = f"{MARKER} {case_id} {backend} {source_commit}"
            if completed.returncode != 0 or expected_marker not in completed.stdout:
                raise RuntimeError(f"{case_id}/{backend}: Stata PASS marker missing")
            rows = validate_rows(
                raw_csv,
                case_id=case_id,
                backend=backend,
                source_commit=source_commit,
                fixture_sha=fixture_sha,
            )
            peak_rss = parse_peak_rss(resources)
            for row in rows:
                row["run_scope"] = "local_macos"
                row["peak_rss_bytes"] = str(peak_rss)
                row["host"] = socket.gethostname()
                row["platform"] = platform.platform()
            all_rows.extend(rows)
            warm = statistics.median(float(row["total_s"]) for row in rows[1:])
            print(
                f"{case_id}/{backend}: cold={float(rows[0]['total_s']):.3f}s "
                f"warm-median={warm:.3f}s rss={peak_rss}"
            )
            artifacts.append(
                {
                    "case_id": case_id,
                    "backend": backend,
                    "raw_csv": raw_csv.name,
                    "raw_csv_sha256": sha256(raw_csv),
                    "log": log.name,
                    "log_sha256": sha256(log),
                    "resources": resources.name,
                    "resources_sha256": sha256(resources),
                    "peak_rss_bytes": peak_rss,
                }
            )

    combined = args.output / "rows.csv"
    with combined.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(all_rows[0]))
        writer.writeheader()
        writer.writerows(all_rows)
    receipt = {
        "schema": "vckss-alpha-benchmark-local-v1",
        "status": "PASS",
        "generated_at_utc": datetime.now(UTC).isoformat(),
        "source_commit": source_commit,
        "source_tree": source_tree,
        "fixture_spec": str(case_path.relative_to(repo)),
        "fixture_spec_sha256": fixture_sha,
        "driver": str(driver.relative_to(repo)),
        "driver_sha256": sha256(driver),
        "stata_executable": str(stata),
        "stata_executable_sha256": sha256(stata),
        "python": sys.version,
        "host": socket.gethostname(),
        "platform": platform.platform(),
        "cases": selected,
        "rows_csv": combined.name,
        "rows_csv_sha256": sha256(combined),
        "artifacts": artifacts,
    }
    (args.output / "receipt.json").write_text(
        json.dumps(receipt, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(f"VCKSS_ALPHA_LOCAL_PASS {source_commit}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

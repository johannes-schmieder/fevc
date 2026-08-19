#!/usr/bin/env python3
"""Run the FE-BUF-1 local archive-isolated benchmark.

The numerical workload is the tracked Stata driver ``fe_buf1/local_driver.do``.
Python only archives revisions, launches fresh Stata processes, validates
receipts, hashes evidence, and summarizes timing.  It contains no estimator,
fixture-generation, routing, or extrapolation logic.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import os
import statistics
import subprocess
import tarfile
import tempfile
from datetime import UTC, datetime
from pathlib import Path

MARKER = "KSS_FE_BUF1_LOCAL_PASS"
TIMING_FIELDS = (
    "command_s",
    "selection_s",
    "graph_s",
    "compression_s",
    "transition_s",
    "work_s",
    "restore_s",
    "fit_s",
    "leverage_s",
    "target_s",
    "correction_s",
    "rng_s",
    "schur_s",
    "precond_s",
    "pcg_s",
)
EXACT_FIELDS = (
    "processors",
    "probes",
    "seed",
    "iterations",
    "schur_actions",
    "schur_batches",
    "precond_apps",
    "precond_batches",
    "n_rows",
    "cells",
    "units",
    "strata",
    "workers",
    "firms",
    "identity_residual",
    "r11",
    "r21",
    "r31",
    "r41",
    "r12",
    "r22",
    "r32",
    "r42",
    "r13",
    "r23",
    "r33",
    "r43",
    "r14",
    "r24",
    "r34",
    "r44",
    "route",
    "engine",
)

PROFILE_FIELDS = (
    "fe_applicable", "fe_workspace_builds", "fe_buffered_batches",
    "fe_legacy_batches", "fe_buffered_columns", "fe_legacy_columns",
    "fe_fallback_batches", "fe_max_width", "fe_workspace_bytes",
    "fe_avoided_bytes",
)


def _run(args: list[str], *, cwd: Path, stdout=None) -> subprocess.CompletedProcess:
    if stdout is None:
        return subprocess.run(
            args,
            cwd=cwd,
            check=True,
            text=True,
            capture_output=True,
        )
    return subprocess.run(
        args,
        cwd=cwd,
        check=True,
        text=False,
        stdout=stdout,
        stderr=subprocess.PIPE,
    )


def _git(repo: Path, *args: str) -> str:
    result = _run(["git", *args], cwd=repo)
    return result.stdout.strip()


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def _archive(repo: Path, commit: str, destination: Path) -> None:
    archive = destination.with_suffix(".tar")
    with archive.open("wb") as handle:
        _run(["git", "archive", commit], cwd=repo, stdout=handle)
    with tarfile.open(archive, "r:") as bundle:
        bundle.extractall(destination, filter="data")
    archive.unlink()


def _read_receipt(path: Path, role: str, commit: str) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    if len(rows) != 4:
        raise RuntimeError(f"{role}: expected four benchmark rows, found {len(rows)}")
    for index, row in enumerate(rows, start=1):
        if row["source_label"] != role or row["source_commit"] != commit:
            raise RuntimeError(f"{role}: receipt source binding failed")
        if int(row["run"]) != index:
            raise RuntimeError(f"{role}: noncanonical run order")
        if row["temperature"] != ("cold" if index == 1 else "warm"):
            raise RuntimeError(f"{role}: temperature label mismatch")
        if float(row["result_mreldif"]) != 0:
            raise RuntimeError(f"{role}: repeated result drifted")
    return rows


def _warm_medians(rows: list[dict[str, str]]) -> dict[str, float]:
    warm = rows[1:]
    return {
        field: statistics.median(float(row[field]) for row in warm)
        for field in TIMING_FIELDS
    }


def _compare_science(
    baseline: list[dict[str, str]], candidate: list[dict[str, str]]
) -> None:
    for run, (left, right) in enumerate(zip(baseline, candidate, strict=True), 1):
        for field in EXACT_FIELDS:
            if left[field] != right[field]:
                raise RuntimeError(
                    f"scientific/structural mismatch run={run} field={field}: "
                    f"{left[field]!r} != {right[field]!r}"
                )
        if float(left["max_residual"]) != float(right["max_residual"]):
            raise RuntimeError(f"solver residual mismatch in run {run}")


def _validate_profiles(
    baseline: list[dict[str, str]], candidate: list[dict[str, str]]
) -> None:
    for run, (left, right) in enumerate(zip(baseline, candidate, strict=True), 1):
        for row in (left, right):
            if int(float(row["fe_applicable"])) != 1:
                raise RuntimeError(f"run {run}: FE buffer profile is not applicable")
            for field in PROFILE_FIELDS:
                if float(row[field]) < 0:
                    raise RuntimeError(f"run {run}: negative profile field {field}")
        if int(float(left["fe_buffered_columns"])) != 0:
            raise RuntimeError(f"run {run}: measurement baseline used buffered path")
        left_columns = int(float(left["fe_legacy_columns"]))
        right_columns = int(float(right["fe_buffered_columns"])) + int(
            float(right["fe_legacy_columns"])
        )
        if left_columns != right_columns:
            raise RuntimeError(f"run {run}: Schur-column accounting changed")


def _parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--candidate", required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--baseline", required=True)
    parser.add_argument(
        "--order",
        choices=("baseline-first", "candidate-first"),
        default="baseline-first",
    )
    parser.add_argument(
        "--stata",
        default=os.environ.get(
            "STATA_EXE", "/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp"
        ),
    )
    return parser.parse_args()


def main() -> int:
    args = _parse_args()
    repo = Path(__file__).resolve().parents[3]
    baseline = _git(repo, "rev-parse", f"{args.baseline}^{{commit}}")
    candidate = _git(repo, "rev-parse", f"{args.candidate}^{{commit}}")
    if len(baseline) != 40 or len(candidate) != 40:
        raise RuntimeError("benchmark revisions must resolve to full commits")
    if candidate == _git(repo, "rev-parse", "HEAD") and _git(
        repo, "status", "--porcelain"
    ):
        raise RuntimeError("candidate HEAD must have a clean worktree")
    if args.output_dir.exists() and any(args.output_dir.iterdir()):
        raise RuntimeError("output directory must be new or empty")
    args.output_dir.mkdir(parents=True, exist_ok=True)
    stata = Path(args.stata).resolve()
    if not stata.is_file():
        raise RuntimeError(f"Stata executable not found: {stata}")

    evidence: dict[str, dict[str, object]] = {}
    rows_by_role: dict[str, list[dict[str, str]]] = {}
    commits = {"baseline": baseline, "candidate": candidate}
    order = (
        ("baseline", "candidate")
        if args.order == "baseline-first"
        else ("candidate", "baseline")
    )
    with tempfile.TemporaryDirectory(prefix="varcomp-kss-fe-buf1-") as tmp:
        temporary = Path(tmp)
        roots = {role: temporary / role for role in commits}
        for role, commit in commits.items():
            roots[role].mkdir()
            _archive(repo, commit, roots[role])
        driver = roots["candidate"] / "varcomp_kss/benchmarks/fe_buf1/local_driver.do"
        baseline_driver = roots["baseline"] / "varcomp_kss/benchmarks/fe_buf1/local_driver.do"
        if _sha256(driver) != _sha256(baseline_driver):
            raise RuntimeError("baseline and candidate benchmark drivers differ")
        for role in order:
            receipt = args.output_dir / f"{role}.csv"
            log = args.output_dir / f"{role}.log"
            command = [
                str(stata),
                "-q",
                "do",
                str(driver),
                str(roots[role]),
                str(receipt),
                role,
                commits[role],
            ]
            completed = subprocess.run(
                command,
                cwd=args.output_dir,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                check=False,
            )
            log.write_text(completed.stdout, encoding="utf-8")
            marker = f"{MARKER} {role} {commits[role]}"
            if completed.returncode != 0 or marker not in completed.stdout:
                raise RuntimeError(f"{role}: Stata run did not reach its PASS marker")
            rows_by_role[role] = _read_receipt(receipt, role, commits[role])
            evidence[role] = {
                "commit": commits[role],
                "tree": _git(repo, "rev-parse", f"{commits[role]}^{{tree}}"),
                "receipt": receipt.name,
                "receipt_sha256": _sha256(receipt),
                "log": log.name,
                "log_sha256": _sha256(log),
            }

    _compare_science(rows_by_role["baseline"], rows_by_role["candidate"])
    _validate_profiles(rows_by_role["baseline"], rows_by_role["candidate"])
    baseline_timing = _warm_medians(rows_by_role["baseline"])
    candidate_timing = _warm_medians(rows_by_role["candidate"])
    changes = {
        field: 100 * (candidate_timing[field] / baseline_timing[field] - 1)
        if baseline_timing[field] > 0
        else 0.0
        for field in TIMING_FIELDS
    }
    command_change = changes["command_s"]
    if command_change <= -2:
        performance_status = "IMPROVED"
    elif command_change <= 2:
        performance_status = "NEUTRAL_RETAIN_COUNTER_GAINS"
    else:
        performance_status = "REPEAT_INTERLEAVED_BEFORE_PROMOTION"
    summary = {
        "schema": "varcomp-kss-fe-buf1-local-v1",
        "status": "PASS",
        "generated_at_utc": datetime.now(UTC).isoformat(),
        "order": list(order),
        "environment": {
            "stata_executable": str(stata),
            "stata_executable_sha256": _sha256(stata),
        },
        "evidence": evidence,
        "scientific_comparison": "EXACT_PASS",
        "performance_policy": {
            "neutral_band_percent": 2,
            "targets_are_advisory": True,
            "small_safe_counter_gains_are_retained": True,
        },
        "performance_status": performance_status,
        "warm_median_seconds": {
            "baseline": baseline_timing,
            "candidate": candidate_timing,
        },
        "candidate_change_percent": changes,
    }
    summary_path = args.output_dir / "summary.json"
    summary_path.write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(
        "FE_BUF1_LOCAL_PASS "
        f"baseline={baseline} candidate={candidate} "
        f"command_change={command_change:.3f}% status={performance_status}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

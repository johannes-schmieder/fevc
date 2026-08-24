#!/usr/bin/env python3
"""Run archive-isolated AB and BA PREP-BND-1 comparisons locally."""

from __future__ import annotations

import argparse
import json
import os
import statistics
import subprocess
import tarfile
import tempfile
from datetime import UTC, datetime
from pathlib import Path

from common import (
    MARKER_LOCAL,
    PREP_BND_METRICS,
    SCIENTIFIC_ABS_REL_TOLERANCE,
    TIMING_FIELDS,
    compare_scientific_contract,
    profile_map,
    read_profiles,
    read_rows,
    require,
    sha256,
    validate_causal_transition,
)

ORDERS = ("ab", "ba")
ROLES = ("baseline", "candidate")
REPETITIONS = 3


def run(args: list[str], cwd: Path, *, stdout=None) -> subprocess.CompletedProcess:
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


def git(repo: Path, *args: str) -> str:
    return run(["git", *args], repo).stdout.strip()


def archive(repo: Path, commit: str, destination: Path) -> None:
    tar_path = destination.with_suffix(".tar")
    with tar_path.open("wb") as handle:
        run(["git", "archive", commit], repo, stdout=handle)
    with tarfile.open(tar_path, "r:") as bundle:
        bundle.extractall(destination, filter="data")
    tar_path.unlink()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--baseline", required=True)
    parser.add_argument("--candidate", required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument(
        "--stata",
        default=os.environ.get(
            "STATA_EXE", "/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp"
        ),
    )
    return parser.parse_args()


def warm_medians(rows: list[dict[str, str]]) -> dict[str, float]:
    warm = rows[1:]
    return {
        field: statistics.median(float(row[field]) for row in warm)
        for field in TIMING_FIELDS
    }


def main() -> int:
    args = parse_args()
    repo = Path(__file__).resolve().parents[3]
    commits = {
        role: git(repo, "rev-parse", f"{getattr(args, role)}^{{commit}}")
        for role in ROLES
    }
    require(all(len(commit) == 40 for commit in commits.values()), "commits must be full SHA-1")
    require(not args.output_dir.exists() or not any(args.output_dir.iterdir()), "output must be empty")
    require(not git(repo, "status", "--porcelain"), "benchmark tool worktree must be clean")
    tool_commit = git(repo, "rev-parse", "HEAD^{commit}")
    tool_tree = git(repo, "rev-parse", "HEAD^{tree}")
    stata = Path(args.stata).resolve()
    require(stata.is_file(), f"Stata executable not found: {stata}")
    args.output_dir.mkdir(parents=True, exist_ok=True)

    evidence: dict[str, object] = {}
    all_rows: dict[tuple[str, str], list[dict[str, str]]] = {}
    all_profiles: dict[tuple[str, str], list[dict[str, str]]] = {}
    with tempfile.TemporaryDirectory(prefix="vckss-prep-bnd1-") as temporary_name:
        temporary = Path(temporary_name)
        roots = {role: temporary / role for role in ROLES}
        for role in ROLES:
            roots[role].mkdir()
            archive(repo, commits[role], roots[role])
        driver = repo / "vckss/benchmarks/prep_bnd1/local_driver.do"
        require(driver.is_file(), "tracked benchmark driver is missing")
        git(
            repo,
            "ls-files",
            "--error-unmatch",
            "vckss/benchmarks/prep_bnd1/local_driver.do",
        )
        driver_sha = sha256(driver)
        for order in ORDERS:
            order_dir = args.output_dir / order
            order_dir.mkdir()
            launch_order = ROLES if order == "ab" else tuple(reversed(ROLES))
            for role in launch_order:
                summary_path = order_dir / f"{role}.csv"
                profile_path = order_dir / f"{role}_profiles.csv"
                log_path = order_dir / f"{role}.log"
                completed = subprocess.run(
                    [
                        str(stata),
                        "-q",
                        "do",
                        str(driver),
                        str(roots[role]),
                        str(summary_path),
                        str(profile_path),
                        role,
                        commits[role],
                        "256",
                        "40",
                        str(REPETITIONS),
                        "local",
                    ],
                    cwd=order_dir,
                    text=True,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.STDOUT,
                    check=False,
                )
                log_path.write_text(completed.stdout, encoding="utf-8")
                marker = f"{MARKER_LOCAL} {role} {commits[role]}"
                require(
                    completed.returncode == 0 and marker in completed.stdout,
                    f"{order}/{role}: Stata did not reach PASS",
                )
                all_rows[order, role] = read_rows(
                    summary_path, role, commits[role], REPETITIONS
                )
                all_profiles[order, role] = read_profiles(
                    profile_path, role, commits[role], REPETITIONS
                )
                evidence[f"{order}_{role}"] = {
                    "summary": str(summary_path.relative_to(args.output_dir)),
                    "summary_sha256": sha256(summary_path),
                    "profiles": str(profile_path.relative_to(args.output_dir)),
                    "profiles_sha256": sha256(profile_path),
                    "log": str(log_path.relative_to(args.output_dir)),
                    "log_sha256": sha256(log_path),
                }

    for order in ORDERS:
        compare_scientific_contract(
            all_rows[order, "baseline"], all_rows[order, "candidate"], order
        )
    for role in ROLES:
        compare_scientific_contract(
            all_rows["ab", role], all_rows["ba", role], f"order/{role}"
        )

    pairs: list[dict[str, object]] = []
    for order in ORDERS:
        baseline_times = warm_medians(all_rows[order, "baseline"])
        candidate_times = warm_medians(all_rows[order, "candidate"])
        profile = profile_map(
            all_profiles[order, "candidate"], "prep_boundary_profile", REPETITIONS
        )
        causal_transition = validate_causal_transition(
            all_profiles[order, "baseline"],
            all_profiles[order, "candidate"],
            all_rows[order, "candidate"],
        )
        pairs.append(
            {
                "order": order,
                "baseline_warm_median_seconds": baseline_times,
                "candidate_warm_median_seconds": candidate_times,
                "candidate_change_percent": {
                    field: (
                        None
                        if baseline_times[field] == 0
                        else 100 * (candidate_times[field] / baseline_times[field] - 1)
                    )
                    for field in TIMING_FIELDS
                },
                "candidate_prep_boundary_last_run_seconds": {
                    metric: profile.get(metric) for metric in PREP_BND_METRICS
                },
                "causal_exposure_transition": causal_transition,
            }
        )
    summary = {
        "schema": "vckss-prep-bnd1-local-v1",
        "status": "PASS",
        "generated_at_utc": datetime.now(UTC).isoformat(),
        "baseline": commits["baseline"],
        "candidate": commits["candidate"],
        "tool_commit": tool_commit,
        "tool_tree": tool_tree,
        "driver_sha256": driver_sha,
        "orders": list(ORDERS),
        "fresh_stata_processes": 4,
        "repetitions_per_process": REPETITIONS,
        "structural_sample_rng_comparison": "EXACT_PASS",
        "scientific_comparison": "TOLERANCE_PASS",
        "scientific_abs_rel_tolerance": SCIENTIFIC_ABS_REL_TOLERANCE,
        "performance_thresholds_are_advisory": True,
        "pairs": pairs,
        "evidence": evidence,
    }
    output = args.output_dir / "summary.json"
    output.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"PREP_BND1_LOCAL_MATRIX_PASS baseline={commits['baseline']} candidate={commits['candidate']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Validate privacy-safe MATLAB phase-profile evidence.

This validator has no MATLAB dependency.  The profiled MATLAB source remains
external and unmodified; only aggregate timers, target values, hashes, and
scheduler/resource receipts are accepted here.
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import re
from datetime import datetime
from pathlib import Path


HEX40 = re.compile(r"[0-9a-f]{40}")
HEX64 = re.compile(r"[0-9a-f]{64}")
LABEL = re.compile(r"[A-Za-z0-9._-]+")
JOB_ID = re.compile(r"[0-9]+")
EXPECTED_UPSTREAM_COMMIT = "8b957ffeb10b8465a3584fceb0265cccc48379e1"
EXPECTED_CORE_SHA256 = (
    "7ab72bcf1f9e1a0091a6a423b1ef5cbd23688f7c64d753cf9adcc6243989a120"
)
EXPECTED_CORE_NEWLINES = 632
MAX_MEMORY_BYTES = 60 * 1024**3

PHASE_RANGES = (
    ("options", 1, 333),
    ("selection", 334, 428),
    ("residual_collapse", 429, 476),
    ("leverage", 477, 518),
    ("variance_estimation", 519, 573),
    ("reporting", 574, 616),
    ("maintained_serialization", 617, 621),
    ("disabled_lincom", 622, 632),
)

TEXT_FIELDS = (
    "status",
    "failure_code",
    "failure_explanation",
    "label",
    "source_commit",
    "bundle_sha256",
    "input_sha256",
    "matlab_upstream_commit",
    "matlab_core_sha256",
    "matlab_cmg_sha256",
    "matlab_cmg_mex_sha256",
    "matlab_cmg_solver_sha256",
    "profiler_sha256",
    "matlab_version",
    "algorithm",
    "deletion_level",
    "profile_timer",
    "profile_function",
    "rng_protocol",
    "projection_basis",
    "process_start_utc",
    "first_matlab_utc",
    "final_matlab_utc",
    "cold_target_sha256",
    "profiled_target_sha256",
    "warm_target_sha256",
    "cold_detail_sha256",
    "profiled_detail_sha256",
    "warm_detail_sha256",
)

INTEGER_FIELDS = (
    "matlab_core_newline_count",
    "matlab_core_profile_max_line",
    "processors",
    "seed",
    "probes",
    "input_rows",
    "input_workers",
    "input_firms",
    "profile_function_calls",
    "profile_executed_lines",
    "profile_line_calls",
    "targets_identical",
    "details_identical",
    "rng_replay_verified",
    "timeout_seconds",
) + tuple(
    field
    for phase, _, _ in PHASE_RANGES
    for field in (
        f"phase_{phase}_line_first",
        f"phase_{phase}_line_last",
        f"phase_{phase}_executed_lines",
        f"phase_{phase}_line_calls",
    )
)

FLOAT_FIELDS = (
    "wrapper_seconds",
    "mex_seconds",
    "import_seconds",
    "pool_seconds",
    "cold_call_seconds",
    "warm_profiled_call_seconds",
    "warm_unprofiled_call_seconds",
    "serialization_seconds",
    "pool_teardown_seconds",
    "profile_top_level_seconds",
    "projected_seconds",
    "target_worker",
    "target_firm",
    "target_covariance",
    "target_total",
) + tuple(f"phase_{phase}_seconds" for phase, _, _ in PHASE_RANGES)

CSV_FIELDS = TEXT_FIELDS + INTEGER_FIELDS + FLOAT_FIELDS


class EvidenceError(RuntimeError):
    """A typed rejection of incomplete or inconsistent run evidence."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise EvidenceError(message)


def read_text(path: Path) -> str:
    require(path.is_file(), f"missing evidence file: {path}")
    return path.read_text(encoding="utf-8")


def read_single_csv(path: Path) -> dict[str, str]:
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        require(tuple(reader.fieldnames or ()) == CSV_FIELDS,
                "aggregate CSV schema changed")
        rows = list(reader)
    require(len(rows) == 1, "aggregate CSV must contain exactly one row")
    return rows[0]


def integer(value: object, field: str) -> int:
    try:
        number = float(value)
    except (TypeError, ValueError) as exc:
        raise EvidenceError(f"{field} is not numeric") from exc
    require(math.isfinite(number) and number.is_integer(),
            f"{field} is not a finite integer")
    return int(number)


def finite(value: object, field: str) -> float:
    try:
        number = float(value)
    except (TypeError, ValueError) as exc:
        raise EvidenceError(f"{field} is not numeric") from exc
    require(math.isfinite(number), f"{field} is not finite")
    return number


def timestamp(value: object, field: str) -> datetime:
    require(isinstance(value, str) and value.endswith("Z"),
            f"{field} is not an explicit UTC timestamp")
    try:
        parsed = datetime.fromisoformat(value[:-1] + "+00:00")
    except ValueError as exc:
        raise EvidenceError(f"{field} is not ISO-8601") from exc
    return parsed


def parse_memory(value: str) -> int:
    match = re.fullmatch(r"([0-9]+(?:\.[0-9]+)?)([BKMGTP]?)", value.strip(), re.I)
    require(match is not None, f"invalid qacct memory value: {value}")
    units = {"": 1, "B": 1, "K": 1024, "M": 1024**2,
             "G": 1024**3, "T": 1024**4, "P": 1024**5}
    return int(float(match.group(1)) * units[match.group(2).upper()])


def parse_key_value(path: Path) -> dict[str, str]:
    result: dict[str, str] = {}
    for line in read_text(path).splitlines():
        fields = line.split("\t")
        require(len(fields) == 2 and fields[0] and fields[0] not in result,
                f"malformed or duplicate key in {path}")
        result[fields[0]] = fields[1]
    return result


def parse_qacct(path: Path, expected_job_id: str, expected_slots: int) -> tuple[int, float]:
    report = read_text(path)

    def field(name: str) -> str:
        values = re.findall(rf"(?m)^{re.escape(name)}\s+([^\s]+)\s*$", report)
        require(len(values) == 1, f"qacct missing/duplicate {name}")
        return values[0]

    require(field("jobnumber") == expected_job_id,
            "qacct job number mismatch")
    require(field("failed") == "0", "qacct reports scheduler failure")
    require(field("exit_status") == "0", "qacct reports nonzero exit")
    require(integer(field("slots"), "qacct slots") == expected_slots,
            "qacct slot reservation mismatch")
    maxvmem = parse_memory(field("maxvmem"))
    require(0 < maxvmem <= MAX_MEMORY_BYTES,
            "qacct maximum virtual memory exceeds the 60-GiB envelope")
    wall = finite(field("ru_wallclock"), "qacct ru_wallclock")
    require(wall > 0, "qacct wall time is not positive")
    return maxvmem, wall


def parse_rss(path: Path) -> int:
    report = read_text(path)
    match = re.search(r"(?m)^\s*Maximum resident set size \(kbytes\):\s*([0-9]+)\s*$", report)
    require(match is not None, "GNU time maximum RSS is missing")
    rss_kib = int(match.group(1))
    require(0 < rss_kib * 1024 <= MAX_MEMORY_BYTES,
            "GNU time maximum RSS exceeds the 60-GiB envelope")
    return rss_kib


def compare_csv_json(csv_row: dict[str, str], record: dict[str, object]) -> None:
    require(set(record) == set(CSV_FIELDS), "aggregate JSON schema changed")
    for field in TEXT_FIELDS:
        require(isinstance(record[field], str) and csv_row[field] == record[field],
                f"CSV/JSON text mismatch: {field}")
    for field in INTEGER_FIELDS:
        require(integer(csv_row[field], field) == integer(record[field], field),
                f"CSV/JSON integer mismatch: {field}")
    for field in FLOAT_FIELDS:
        left = finite(csv_row[field], field)
        right = finite(record[field], field)
        require(abs(left - right) <= 1e-13 * (1 + abs(right)),
                f"CSV/JSON numeric mismatch: {field}")


def validate_record(record: dict[str, object], args: argparse.Namespace) -> None:
    require(record["status"] == "PASS", "profile status is not PASS")
    require(record["failure_code"] == "NONE" and
            record["failure_explanation"] == "NONE",
            "successful profile contains a failure status")
    require(record["label"] == args.label, "label mismatch")
    require(record["source_commit"] == args.expected_source_commit,
            "source commit mismatch")
    require(record["bundle_sha256"] == args.expected_bundle_sha256,
            "bundle hash mismatch")
    require(record["input_sha256"] == args.expected_input_sha256,
            "input hash mismatch")
    require(record["matlab_upstream_commit"] == args.expected_upstream_commit,
            "MATLAB upstream commit mismatch")
    require(record["matlab_core_sha256"] == args.expected_core_sha256 ==
            EXPECTED_CORE_SHA256, "unregistered MATLAB core hash")
    require(record["matlab_cmg_sha256"] == args.expected_cmg_sha256,
            "MATLAB CMG hash mismatch")
    require(record["matlab_cmg_mex_sha256"] == args.expected_cmg_mex_sha256,
            "MATLAB CMG MEX-family hash mismatch")
    require(record["matlab_cmg_solver_sha256"] == args.expected_cmg_solver_sha256,
            "MATLAB CMG solver-family hash mismatch")
    require(record["profiler_sha256"] == args.expected_profiler_sha256,
            "phase-profiler source hash mismatch")
    for field in ("bundle_sha256", "input_sha256", "matlab_core_sha256",
                  "matlab_cmg_sha256", "matlab_cmg_mex_sha256",
                  "matlab_cmg_solver_sha256", "profiler_sha256"):
        require(HEX64.fullmatch(str(record[field])) is not None,
                f"invalid hash syntax: {field}")
    require(integer(record["matlab_core_newline_count"], "core lines") ==
            EXPECTED_CORE_NEWLINES, "MATLAB core line registration changed")
    require(integer(record["matlab_core_profile_max_line"], "profile max line") ==
            EXPECTED_CORE_NEWLINES, "MATLAB profile line ceiling changed")
    require(integer(record["processors"], "processors") == args.expected_slots,
            "MATLAB processor count changed")
    require("(R2025b)" in str(record["matlab_version"]),
            "MATLAB release is not R2025b")
    require(integer(record["seed"], "seed") == args.expected_seed,
            "seed mismatch")
    require(integer(record["probes"], "probes") == args.expected_probes,
            "probe count mismatch")
    for field in ("input_rows", "input_workers", "input_firms"):
        require(integer(record[field], field) > 0, f"{field} is not positive")
    require(record["algorithm"] == "JLA" and
            record["deletion_level"] == "matches", "MATLAB call semantics changed")
    require(record["profile_timer"] == "real" and
            record["profile_function"] == "leave_out_KSS",
            "profiler did not use the registered top-level real-time target")
    require(integer(record["profile_function_calls"], "profile calls") == 1,
            "profiled top-level call count changed")
    require(record["rng_protocol"] ==
            "client_and_worker_state_restored_before_each_call",
            "RNG replay protocol changed")
    require(integer(record["rng_replay_verified"], "RNG replay") == 1,
            "RNG replay was not verified")
    require(record["projection_basis"] == args.expected_projection_basis,
            "runtime projection basis mismatch")
    projected_seconds = finite(record["projected_seconds"], "projected seconds")
    require(abs(projected_seconds - args.expected_projected_seconds) <=
            1e-13 * (1 + abs(args.expected_projected_seconds)),
            "runtime projection changed")
    expected_timeout = max(300, math.ceil(1.5 * projected_seconds + 180))
    require(expected_timeout <= 3600 and
            integer(record["timeout_seconds"], "timeout seconds") == expected_timeout,
            "hard timeout does not follow the registered projection rule")

    hashes = (
        str(record["cold_target_sha256"]),
        str(record["profiled_target_sha256"]),
        str(record["warm_target_sha256"]),
        str(record["cold_detail_sha256"]),
        str(record["profiled_detail_sha256"]),
        str(record["warm_detail_sha256"]),
    )
    require(all(HEX64.fullmatch(value) for value in hashes),
            "invalid reproducibility hash")
    require(len(set(hashes[:3])) == 1 and len(set(hashes[3:])) == 1,
            "profiled and unprofiled calls are not exactly reproducible")
    require(integer(record["targets_identical"], "targets identical") == 1 and
            integer(record["details_identical"], "details identical") == 1,
            "reproducibility flags are not asserted")

    process_start = timestamp(record["process_start_utc"], "process start")
    first_matlab = timestamp(record["first_matlab_utc"], "first MATLAB")
    final_matlab = timestamp(record["final_matlab_utc"], "final MATLAB")
    require(process_start <= first_matlab <= final_matlab,
            "process/MATLAB timestamps are out of order")

    positive = ("wrapper_seconds", "cold_call_seconds",
                "warm_profiled_call_seconds", "warm_unprofiled_call_seconds",
                "serialization_seconds")
    nonnegative = ("mex_seconds", "import_seconds", "pool_seconds",
                   "pool_teardown_seconds", "profile_top_level_seconds")
    timings = {field: finite(record[field], field)
               for field in positive + nonnegative}
    require(all(timings[field] > 0 for field in positive),
            "required direct timer is not positive")
    require(all(timings[field] >= 0 for field in nonnegative),
            "direct timer is negative")
    accounted = sum(timings[field] for field in (
        "mex_seconds", "import_seconds", "pool_seconds", "cold_call_seconds",
        "warm_profiled_call_seconds", "warm_unprofiled_call_seconds",
        "pool_teardown_seconds"))
    require(accounted <= timings["wrapper_seconds"] + 0.5,
            "direct stage timers exceed wrapper time")
    matlab_elapsed = (final_matlab - first_matlab).total_seconds()
    require(abs(matlab_elapsed - timings["wrapper_seconds"]) <= 2.0,
            "MATLAB timestamps disagree with the direct wrapper timer")
    require(timings["warm_profiled_call_seconds"] <=
            max(10 * timings["warm_unprofiled_call_seconds"],
                timings["warm_unprofiled_call_seconds"] + 300),
            "profiling overhead is implausibly large")

    executed = 0
    line_calls = 0
    phase_seconds = 0.0
    for phase, first, last in PHASE_RANGES:
        require(integer(record[f"phase_{phase}_line_first"], f"{phase} first") == first and
                integer(record[f"phase_{phase}_line_last"], f"{phase} last") == last,
                f"registered phase bounds changed: {phase}")
        count_lines = integer(record[f"phase_{phase}_executed_lines"],
                              f"{phase} executed lines")
        count_calls = integer(record[f"phase_{phase}_line_calls"],
                              f"{phase} line calls")
        seconds = finite(record[f"phase_{phase}_seconds"], f"{phase} seconds")
        require(count_lines > 0 and count_lines <= last - first + 1,
                f"invalid executed-line count: {phase}")
        require(count_calls >= count_lines and seconds >= 0,
                f"invalid phase aggregate: {phase}")
        require(seconds <= timings["warm_profiled_call_seconds"] + 0.5,
                f"phase time exceeds profiled call: {phase}")
        executed += count_lines
        line_calls += count_calls
        phase_seconds += seconds
    require(executed == integer(record["profile_executed_lines"],
                                "profile executed lines"),
            "phase executed-line accounting failed")
    require(line_calls == integer(record["profile_line_calls"],
                                  "profile line calls"),
            "phase line-call accounting failed")
    top_seconds = timings["profile_top_level_seconds"]
    require(abs(phase_seconds - top_seconds) <= 1e-10 * (1 + abs(top_seconds)),
            "phase timing accounting failed")
    require(top_seconds <= timings["warm_profiled_call_seconds"] + 0.5,
            "top-level profile time exceeds profiled call")

    target_worker = finite(record["target_worker"], "worker target")
    target_firm = finite(record["target_firm"], "firm target")
    target_covariance = finite(record["target_covariance"], "covariance target")
    target_total = finite(record["target_total"], "total target")
    identity = target_worker + target_firm + 2 * target_covariance
    require(abs(target_total - identity) <= 1e-12 * (1 + abs(identity)),
            "MATLAB total-variance identity failed")


def validate(args: argparse.Namespace) -> None:
    require(LABEL.fullmatch(args.label) is not None, "invalid label")
    require(HEX40.fullmatch(args.expected_source_commit) is not None,
            "invalid expected source commit")
    require(HEX40.fullmatch(args.expected_upstream_commit) is not None,
            "invalid expected upstream commit")
    for name in ("expected_bundle_sha256", "expected_input_sha256",
                 "expected_core_sha256", "expected_cmg_sha256",
                 "expected_cmg_mex_sha256", "expected_cmg_solver_sha256",
                 "expected_profiler_sha256"):
        require(HEX64.fullmatch(getattr(args, name)) is not None,
                f"invalid {name}")
    require(args.expected_core_sha256 == EXPECTED_CORE_SHA256,
            "validator only accepts the registered maintained MATLAB core")
    require(args.expected_upstream_commit == EXPECTED_UPSTREAM_COMMIT,
            "validator only accepts the registered maintained MATLAB commit")
    require(args.expected_slots == 4, "registered profile uses four slots")
    require(0 <= args.expected_seed < 2**32 and args.expected_probes > 0,
            "invalid expected RNG settings")
    require(math.isfinite(args.expected_projected_seconds) and
            0 < args.expected_projected_seconds <= 3600,
            "invalid expected runtime projection")
    require(re.fullmatch(r"[A-Za-z0-9._-]+", args.expected_projection_basis)
            is not None, "invalid expected projection basis")
    require(max(300, math.ceil(1.5 * args.expected_projected_seconds + 180))
            <= 3600, "projected hard timeout exceeds one hour")

    run_dir = args.run_dir.resolve()
    require(read_text(run_dir / "source_commit.txt").strip() ==
            args.expected_source_commit, "run source identity mismatch")
    require(read_text(run_dir / "bundle.sha256").strip() ==
            args.expected_bundle_sha256, "run bundle identity mismatch")
    job_dir = run_dir / "matlab_phase_profile" / args.label
    output_dir = job_dir / "output"
    require(output_dir.is_dir(), "profile output directory is missing")
    inventory = sorted(path.name for path in output_dir.iterdir())
    require(inventory == ["aggregate.csv", "aggregate.json", "wrapper.pass"],
            "profile output inventory is not privacy-safe and exact")

    csv_row = read_single_csv(output_dir / "aggregate.csv")
    record = json.loads(read_text(output_dir / "aggregate.json"))
    require(isinstance(record, dict), "aggregate JSON is not an object")
    compare_csv_json(csv_row, record)
    validate_record(record, args)

    submission = parse_key_value(job_dir / "submission.tsv")
    expected_submission = {
        "label": args.label,
        "source_commit": args.expected_source_commit,
        "bundle_sha256": args.expected_bundle_sha256,
        "input_sha256": args.expected_input_sha256,
        "matlab_upstream_commit": args.expected_upstream_commit,
        "matlab_core_sha256": args.expected_core_sha256,
        "matlab_cmg_sha256": args.expected_cmg_sha256,
        "matlab_cmg_mex_sha256": args.expected_cmg_mex_sha256,
        "matlab_cmg_solver_sha256": args.expected_cmg_solver_sha256,
        "profiler_sha256": args.expected_profiler_sha256,
        "seed": str(args.expected_seed),
        "probes": str(args.expected_probes),
        "processors": str(args.expected_slots),
        "projection_basis": args.expected_projection_basis,
    }
    require(set(submission) == set(expected_submission) |
            {"projected_seconds", "timeout_seconds"},
            "submission schema changed")
    for field, expected in expected_submission.items():
        require(submission[field] == expected,
                f"submission identity changed: {field}")
    require(abs(finite(submission["projected_seconds"], "submission projection") -
                args.expected_projected_seconds) <=
            1e-13 * (1 + abs(args.expected_projected_seconds)),
            "submission runtime projection changed")
    require(integer(submission["timeout_seconds"], "submission timeout") ==
            max(300, math.ceil(1.5 * args.expected_projected_seconds + 180)),
            "submission hard timeout changed")

    receipt_name = f"matlab_phase_profile_{args.label}"
    job_id = read_text(run_dir / "submissions" / f"{receipt_name}.job_id").strip()
    require(JOB_ID.fullmatch(job_id) is not None, "invalid recorded job ID")
    _, qacct_wall = parse_qacct(run_dir / "qacct" / f"{receipt_name}.txt",
                                job_id, args.expected_slots)
    rss_kib = parse_rss(job_dir / "resources.txt")
    wrapper_seconds = finite(record["wrapper_seconds"], "wrapper seconds")
    startup_seconds = (
        timestamp(record["first_matlab_utc"], "first MATLAB") -
        timestamp(record["process_start_utc"], "process start")
    ).total_seconds()
    require(0 <= startup_seconds <= qacct_wall + 2,
            "MATLAB startup timestamp exceeds scheduler wall time")
    require(wrapper_seconds <= qacct_wall + 2,
            "MATLAB wrapper timer exceeds scheduler wall time")

    application = read_text(job_dir / "application.txt")
    require(f"KSS_BC MATLAB PHASE PROFILE PASS: {args.label}" in application,
            "application success marker is missing")
    expected_pass = (
        f"KSS_BC_MATLAB_PHASE_PROFILE_PASS {args.label} "
        f"{args.expected_bundle_sha256} {args.expected_source_commit} "
        f"{args.expected_input_sha256} {args.expected_core_sha256} "
        f"{record['warm_target_sha256']} {record['warm_detail_sha256']}\n"
    )
    require(read_text(output_dir / "wrapper.pass") == expected_pass,
            "identity-bound MATLAB pass marker changed")
    print(
        "KSS_BC MATLAB PHASE PROFILE EVIDENCE PASS; "
        f"label={args.label}; wrapper={wrapper_seconds:.3f}s; "
        f"profiled={finite(record['warm_profiled_call_seconds'], 'profiled'):.3f}s; "
        f"warm={finite(record['warm_unprofiled_call_seconds'], 'warm'):.3f}s; "
        f"RSS={rss_kib} KiB"
    )


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser()
    result.add_argument("--run-dir", type=Path, required=True)
    result.add_argument("--label", required=True)
    result.add_argument("--expected-source-commit", required=True)
    result.add_argument("--expected-bundle-sha256", required=True)
    result.add_argument("--expected-input-sha256", required=True)
    result.add_argument("--expected-upstream-commit", required=True)
    result.add_argument("--expected-core-sha256", required=True)
    result.add_argument("--expected-cmg-sha256", required=True)
    result.add_argument("--expected-cmg-mex-sha256", required=True)
    result.add_argument("--expected-cmg-solver-sha256", required=True)
    result.add_argument("--expected-profiler-sha256", required=True)
    result.add_argument("--expected-projected-seconds", type=float, required=True)
    result.add_argument("--expected-projection-basis", required=True)
    result.add_argument("--expected-seed", type=int, default=8675309)
    result.add_argument("--expected-probes", type=int, default=200)
    result.add_argument("--expected-slots", type=int, default=4)
    return result


def main() -> None:
    args = parser().parse_args()
    try:
        validate(args)
    except (EvidenceError, OSError, json.JSONDecodeError) as exc:
        raise SystemExit(f"MATLAB_PHASE_PROFILE_EVIDENCE_FAIL: {exc}") from exc


if __name__ == "__main__":
    main()

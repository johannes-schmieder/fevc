#!/usr/bin/env python3
"""Validate one source-bound Stata/MP 19 RNG-K1 SCC compatibility job."""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import os
import re
from pathlib import Path
from typing import Any

HEX40 = re.compile(r"[0-9a-f]{40}")
HEX64 = re.compile(r"[0-9a-f]{64}")
JOB_ID = re.compile(r"[0-9]+")
FATAL_STATA = re.compile(r"(?:^|\n)r\([0-9]+\);(?:\n|$)")
GIB = 1024**3

EXPECTED_GOLDENS: dict[tuple[str, str], tuple[tuple[int, ...], ...]] = {
    ("per_probe_stream", "leverage"): (
        (1, -1, 1), (-2, 2, 2), (-3, 3, 3), (-1, 1, 7),
    ),
    ("per_probe_stream", "target"): (
        (-1, 1, 1), (0, 0, 0), (5, 3, -3), (7, 7, -3),
    ),
    ("per_domain_stream", "leverage"): (
        (1, 1, 1), (-2, 0, 0), (-3, -3, -1), (-1, 3, -3),
    ),
    ("per_domain_stream", "target"): (
        (-1, 1, -1), (2, 0, 0), (3, 1, 1), (1, 3, -1),
    ),
}
SEMANTIC_KEYS = ("a", "b", "c", "d")
GOLDEN_FIELDS = [
    "candidate", "domain", "semantic_key", "probe_1", "probe_2", "probe_3",
]
TRUE_FIELDS = (
    "production_runtime_fail_closed",
    "golden_vectors_match",
    "canonical_order_invariant",
    "per_probe_partition_invariant",
    "per_domain_partition_invariant",
    "cursor_partition_invariant",
    "processor_per_probe_invariant",
    "processor_per_domain_invariant",
    "processor_switch_ok",
    "small_callshape_atoms_equal",
    "small_callshape_states_equal",
    "boundary_callshape_atoms_equal",
    "boundary_callshape_states_equal",
    "production_callshape_atoms_equal",
    "production_callshape_states_equal",
    "production_timing_streams_restored",
    "exact_integer_gate_pass",
    "boundary_chunk_pass",
    "large_generation_pass",
    "timing_includes_stream_snapshot_restore",
    "timing_selects_domain",
    "per_probe_hidden_streams_restored",
    "per_probe_test_cleanup_restored",
    "nonselected_leverage_streams_restored",
    "nonselected_target_streams_restored",
    "nonselected_caller_restored",
    "per_probe_candidate_qualified",
    "per_domain_candidate_qualified",
    "active_algorithm_restored",
    "active_stream_restored",
    "active_state_restored",
    "sort_state_restored",
    "domain_stream1_restored",
    "domain_stream2_restored",
    "selected_stream_state_restored",
    "domain_guard_full_restored",
    "every_touched_stream_restored",
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def sha256(path: Path) -> str:
    require(path.is_file(), f"missing file: {path}")
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def read_key_values(path: Path, label: str) -> dict[str, str]:
    require(path.is_file(), f"missing {label}: {path}")
    values: dict[str, str] = {}
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.reader(handle, delimiter="\t")
        require(next(reader, None) == ["key", "value"],
                f"invalid {label} header")
        for fields in reader:
            require(len(fields) == 2 and fields[0], f"invalid {label} row")
            require(fields[0] not in values,
                    f"duplicate {label} key: {fields[0]}")
            values[fields[0]] = fields[1]
    return values


def finite(values: dict[str, str], key: str) -> float:
    try:
        value = float(values[key])
    except (KeyError, TypeError, ValueError) as exc:
        raise ValueError(f"invalid {key}") from exc
    require(math.isfinite(value), f"nonfinite {key}")
    return value


def integer(values: dict[str, str], key: str) -> int:
    value = finite(values, key)
    require(value.is_integer(), f"noninteger {key}")
    return int(value)


def parse_duration(value: str) -> float:
    try:
        parsed = float(value)
    except ValueError:
        hours = re.fullmatch(r"(\d+):(\d{2}):(\d{2}(?:\.\d+)?)", value)
        minutes = re.fullmatch(r"(\d+):(\d{2}(?:\.\d+)?)", value)
        require(hours is not None or minutes is not None,
                f"invalid duration: {value}")
        if hours is not None:
            parsed = (3600 * int(hours.group(1)) +
                      60 * int(hours.group(2)) + float(hours.group(3)))
        else:
            assert minutes is not None
            parsed = 60 * int(minutes.group(1)) + float(minutes.group(2))
    require(math.isfinite(parsed) and parsed >= 0,
            f"invalid duration: {value}")
    return parsed


def parse_memory(value: str) -> int:
    match = re.fullmatch(r"([0-9]+(?:\.[0-9]+)?)([KMGTP]?)", value, re.I)
    require(match is not None, f"invalid memory value: {value}")
    multipliers = {
        "": 1, "K": 1024, "M": 1024**2, "G": GIB,
        "T": 1024**4, "P": 1024**5,
    }
    return math.ceil(float(match.group(1)) *
                     multipliers[match.group(2).upper()])


def parse_qacct(path: Path) -> dict[str, str]:
    require(path.is_file(), f"missing qacct receipt: {path}")
    values: dict[str, str] = {}
    separators = 0
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line:
            continue
        if set(line) == {"="}:
            separators += 1
            continue
        fields = line.split(None, 1)
        if len(fields) != 2:
            continue
        key, value = fields
        require(key not in values, f"ambiguous qacct field: {key}")
        values[key] = value.strip()
    require(separators <= 1, "qacct contains multiple records")
    for key in (
        "jobnumber", "project", "slots", "failed", "exit_status",
        "ru_wallclock", "cpu", "maxvmem",
    ):
        require(key in values, f"qacct missing {key}")
    return values


def validate_golden_vectors(path: Path) -> list[dict[str, str]]:
    require(path.is_file(), f"missing golden vectors: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        require(reader.fieldnames == GOLDEN_FIELDS,
                "golden-vector schema changed")
        rows = list(reader)
    require(len(rows) == 16, "golden-vector row count changed")
    expected_order = [
        (candidate, domain, key)
        for candidate in ("per_probe_stream", "per_domain_stream")
        for domain in ("leverage", "target")
        for key in SEMANTIC_KEYS
    ]
    observed_order = [
        (row["candidate"], row["domain"], row["semantic_key"])
        for row in rows
    ]
    require(observed_order == expected_order,
            "golden-vector canonical order changed")
    for row in rows:
        candidate = row["candidate"]
        domain = row["domain"]
        atom = SEMANTIC_KEYS.index(row["semantic_key"])
        observed = tuple(int(row[f"probe_{probe}"]) for probe in range(1, 4))
        require(observed == EXPECTED_GOLDENS[(candidate, domain)][atom],
                f"Stata 19 differs from Stata 18 golden vector: "
                f"{candidate}/{domain}/{row['semantic_key']}")
    return rows


def validate_stata_receipt(
    path: Path, *, experiment_id: str, source_commit: str,
    bundle_sha: str, job_id: str,
) -> dict[str, str]:
    values = read_key_values(path, "Stata receipt")
    expected = {
        "receipt_version": "KSS-RNG-K1-STATA19-V1",
        "experiment_id": experiment_id,
        "source_commit": source_commit,
        "bundle_sha256": bundle_sha,
        "job_id": job_id,
        "stata_version": "19",
        "stata_flavor": "MP",
        "rng_build_id": "kss-bc-rng-k1-mt64s-complete-guard-v2",
        "rng_invariant_version": "KSS-RNG-K1-INVARIANT-V1",
        "stata18_reference_contract":
            "KSS-MT64S-DOMAIN-CURSOR-V2-STATA18",
        "production_contract_at_runtime": "",
        "per_probe_leverage_status": "OK",
        "per_probe_target_status": "OK",
        "per_domain_leverage_status": "OK",
        "per_domain_target_status": "OK",
        "per_probe_contract": "KSS-MT64S-PER-PROBE-CANDIDATE-V2",
        "per_domain_contract": "KSS-MT64S-PER-DOMAIN-CANDIDATE-V2",
        "tiny_timing_role": "SMOKE_ONLY_NOT_SELECTION",
        "selection_timing_contract":
            "50000_ATOMS_P40_BOTH_DOMAINS_3_PAIRED_REPS_V1",
        "per_probe_streams_tested": "1,2,3,16384,16385,16386",
        "nonselected_probe_range": "7:8",
        "nonselected_selected_stream": "9",
        "nonselected_leverage_streams": "7,8",
        "nonselected_target_streams": "16390,16391",
        "per_probe_candidate_result": "QUALIFIED_STATA19_CANDIDATE",
        "per_domain_streams_tested": "1,2",
        "per_domain_candidate_result":
            "QUALIFIED_STATA19_CANDIDATE_NOT_REGISTERED",
        "selected_candidate": "per_domain_stream_cursor",
        "overall_status": "KSS_RNG_K1_STATA19_COMPATIBLE_FAIL_CLOSED",
    }
    for key, expected_value in expected.items():
        require(values.get(key) == expected_value,
                f"Stata receipt mismatch: {key}")
    require(integer(values, "stata_mp") == 1, "Stata/MP receipt changed")
    require(integer(values, "requested_slots") == 14 and
            integer(values, "actual_slots") == 14,
            "scheduler slot receipt changed")
    require(integer(values, "requested_stata_processors") == 4 and
            integer(values, "actual_stata_processors") == 4,
            "Stata processor receipt changed")
    require(integer(values, "rng_api_level") == 2,
            "RNG API level changed")
    for key in TRUE_FIELDS:
        require(integer(values, key) == 1, f"K1 gate failed: {key}")
    require(integer(values, "per_probe_changed_stream_count") == 0,
            "per-probe candidate changed a latent mt64s stream")
    for key in (
        "core_rc", "processor_restore_rc", "guard_restore_rc",
        "all_stream_restore_rc",
    ):
        require(integer(values, key) == 0, f"nonzero K1 return: {key}")
    for key in (
        "production_scalar_seconds", "production_vector_seconds",
        "leverage_per_probe_seconds", "leverage_per_domain_seconds",
        "target_per_probe_seconds", "target_per_domain_seconds",
    ):
        require(finite(values, key) >= 0, f"negative K1 timing: {key}")
    require(integer(values, "production_candidate_atoms") == 50_000,
            "production timing atom count changed")
    require(integer(values, "production_candidate_probes") == 40,
            "production timing probe count changed")
    repetitions = integer(values, "production_candidate_repetitions")
    require(repetitions >= 3,
            "production timing needs at least three paired repetitions")
    probe_min = finite(values, "production_per_probe_min_seconds")
    probe_median = finite(values, "production_per_probe_median_seconds")
    probe_max = finite(values, "production_per_probe_max_seconds")
    domain_min = finite(values, "production_per_domain_min_seconds")
    domain_median = finite(values, "production_per_domain_median_seconds")
    domain_max = finite(values, "production_per_domain_max_seconds")
    require(0 <= probe_min <= probe_median <= probe_max,
            "invalid per-probe production timing spread")
    require(0 <= domain_min <= domain_median <= domain_max,
            "invalid per-domain production timing spread")
    domain_wins = integer(values, "production_domain_wins")
    require(2 <= domain_wins <= repetitions,
            "per-domain candidate did not win paired production timings")
    require(domain_median < probe_median,
            "production timing median does not select domain cursor")
    require(integer(values, "chunked_calls") == 2,
            "large-count chunk count changed")
    boundary = integer(values, "boundary_atom")
    chunked = integer(values, "chunked_atom")
    require(abs(boundary) <= 100_000_000_000 and boundary % 2 == 0,
            "invalid boundary binomial atom")
    require(abs(chunked) <= 100_000_000_007 and chunked % 2 != 0,
            "invalid chunked binomial atom")
    return values


def validate_node_receipt(
    path: Path, *, experiment_id: str, source_commit: str,
    bundle_sha: str, job_id: str,
) -> dict[str, str]:
    values = read_key_values(path, "node receipt")
    expected = {
        "receipt_version": "KSS-RNG-K1-NODE-V1",
        "experiment_id": experiment_id,
        "job_id": job_id,
        "source_commit": source_commit,
        "bundle_sha256": bundle_sha,
        "requested_slots": "14",
        "actual_slots": "14",
        "requested_stata_processors": "4",
        "mem_per_core_gib": "4",
        "total_reserved_gib": "56",
        "hard_wall_seconds": "3600",
        "timeout_basis": "local_stata18_target_per_probe_p3_63.364s",
        "timeout_projected_k1_seconds": "2100",
        "scalar_job": "1",
        "stata_module": "stata-mp/19",
    }
    for key, expected_value in expected.items():
        require(values.get(key) == expected_value,
                f"node receipt mismatch: {key}")
    return values


def validate_wrapper_receipt(
    path: Path, *, output_dir: Path, experiment_id: str,
    source_commit: str, bundle_sha: str, job_id: str,
) -> dict[str, str]:
    values = read_key_values(path, "wrapper receipt")
    expected = {
        "receipt_version": "KSS-RNG-K1-WRAPPER-V1",
        "experiment_id": experiment_id,
        "job_id": job_id,
        "source_commit": source_commit,
        "bundle_sha256": bundle_sha,
        "requested_slots": "14",
        "actual_slots": "14",
        "mem_per_core_gib": "4",
        "total_reserved_gib": "56",
        "requested_stata_processors": "4",
        "actual_stata_processors": "4",
        "hard_wall_seconds": "3600",
        "timeout_basis": "local_stata18_target_per_probe_p3_63.364s",
        "timeout_projected_k1_seconds": "2100",
        "application_timeout_seconds": "3480",
        "qacct_jobnumber_binding": job_id,
        "qacct_status": "PENDING_POST_EXIT_VALIDATION",
        "execution_boundary": "one_scalar_job_one_stata_process_no_data",
    }
    for key, expected_value in expected.items():
        require(values.get(key) == expected_value,
                f"wrapper receipt mismatch: {key}")
    hash_bindings = {
        "golden_vectors_sha256": output_dir / "golden_vectors.csv",
        "caller_rng_snapshot_sha256": output_dir / "caller_rng_before.tsv",
        "stata_receipt_sha256": output_dir / "stata_receipt.tsv",
        "process_resources_sha256": output_dir / "process_resources.txt",
        "application_log_sha256": output_dir / "application.log",
    }
    for key, artifact in hash_bindings.items():
        require(values.get(key) == sha256(artifact),
                f"wrapper artifact hash mismatch: {key}")
    require(finite(values, "stata_process_seconds") >= 0 and
            finite(values, "total_wrapper_seconds") >=
            finite(values, "stata_process_seconds"),
            "invalid wrapper timing")
    return values


def validate_markers(
    output_dir: Path, *, experiment_id: str, source_commit: str,
    bundle_sha: str, job_id: str,
) -> None:
    stata = output_dir / "stata.pass"
    wrapper = output_dir / "wrapper.pass"
    expected_stata = (
        f"KSS_RNG_K1_STATA_PASS {experiment_id} {bundle_sha} "
        f"{source_commit} {job_id}"
    )
    expected_wrapper = (
        f"KSS_RNG_K1_WRAPPER_PASS {experiment_id} {bundle_sha} "
        f"{source_commit} {job_id}"
    )
    require(stata.is_file() and stata.read_text(encoding="utf-8").strip() ==
            expected_stata, "Stata pass marker mismatch")
    require(wrapper.is_file() and
            wrapper.read_text(encoding="utf-8").strip() == expected_wrapper,
            "wrapper pass marker mismatch")
    require(not list(output_dir.glob("*.fail")), "failure marker present")
    log = (output_dir / "application.log").read_text(
        encoding="utf-8", errors="replace")
    require(f"KSS-RNG-K1 STATA19 COMPATIBILITY PASS: {experiment_id}" in log,
            "Stata application success marker missing")
    require(FATAL_STATA.search(log) is None, "fatal Stata return code in log")


def validate_process_resources(
    path: Path, *, scheduler_wall: float,
) -> dict[str, float]:
    require(path.is_file(), f"missing process resource receipt: {path}")
    text = path.read_text(encoding="utf-8", errors="replace")
    require("stata-mp" in text and "rng_k1_driver.do" in text,
            "resource receipt did not time the K1 Stata process")

    def one(pattern: str, label: str) -> str:
        found = re.findall(pattern, text, flags=re.MULTILINE)
        require(len(found) == 1,
                f"resource receipt missing/duplicate {label}")
        return found[0]

    exit_status = int(one(r"^\s*Exit status:\s*([0-9]+)\s*$", "exit status"))
    rss_kib = int(one(
        r"^\s*Maximum resident set size \(kbytes\):\s*([0-9]+)\s*$",
        "maximum RSS"))
    elapsed = parse_duration(one(
        r"^\s*Elapsed \(wall clock\) time \(h:mm:ss or m:ss\):\s*(\S+)\s*$",
        "elapsed wall"))
    user = float(one(r"^\s*User time \(seconds\):\s*(\S+)\s*$", "user"))
    system = float(one(
        r"^\s*System time \(seconds\):\s*(\S+)\s*$", "system"))
    require(exit_status == 0, "timed Stata process failed")
    require(rss_kib * 1024 <= 56 * GIB, "Stata RSS exceeds reservation")
    require(elapsed <= scheduler_wall + 2, "Stata wall exceeds qacct wall")
    require(min(user, system) >= 0, "invalid Stata CPU receipt")
    return {
        "elapsed_seconds": elapsed,
        "cpu_seconds": user + system,
        "peak_rss_bytes": rss_kib * 1024,
    }


def validate_run(
    *, output_dir: Path, qacct_path: Path, job_id_file: Path,
    experiment_id: str, source_commit: str, bundle_sha: str,
) -> dict[str, Any]:
    require(re.fullmatch(r"[A-Za-z0-9._-]+", experiment_id) is not None,
            "invalid experiment ID")
    require(HEX40.fullmatch(source_commit) is not None,
            "invalid source commit")
    require(HEX64.fullmatch(bundle_sha) is not None, "invalid bundle hash")
    require(job_id_file.is_file(), f"missing job-id receipt: {job_id_file}")
    job_id = job_id_file.read_text(encoding="utf-8").strip()
    require(JOB_ID.fullmatch(job_id) is not None, "invalid job-id receipt")
    require(output_dir.is_dir(), f"missing output directory: {output_dir}")

    qacct = parse_qacct(qacct_path)
    require(qacct["jobnumber"] == job_id, "qacct job number mismatch")
    require(qacct["project"] == "welfgr", "qacct project mismatch")
    require(int(qacct["slots"]) == 14, "qacct slot count mismatch")
    require(int(qacct["failed"]) == 0, "SGE failed is nonzero")
    require(int(qacct["exit_status"]) == 0, "SGE exit_status is nonzero")
    scheduler_wall = parse_duration(qacct["ru_wallclock"])
    scheduler_cpu = parse_duration(qacct["cpu"])
    scheduler_vmem = parse_memory(qacct["maxvmem"])
    require(scheduler_wall <= 3602, "qacct wall exceeds hard request")
    require(scheduler_vmem <= 56 * GIB, "qacct maxvmem exceeds reservation")

    validate_markers(
        output_dir, experiment_id=experiment_id, source_commit=source_commit,
        bundle_sha=bundle_sha, job_id=job_id)
    node = validate_node_receipt(
        output_dir / "node_receipt.tsv", experiment_id=experiment_id,
        source_commit=source_commit, bundle_sha=bundle_sha, job_id=job_id)
    stata = validate_stata_receipt(
        output_dir / "stata_receipt.tsv", experiment_id=experiment_id,
        source_commit=source_commit, bundle_sha=bundle_sha, job_id=job_id)
    goldens = validate_golden_vectors(output_dir / "golden_vectors.csv")
    before = output_dir / "caller_rng_before.tsv"
    after = output_dir / "caller_rng_after.tsv"
    require(before.is_file() and after.is_file() and
            before.read_bytes() == after.read_bytes(),
            "full caller RNG snapshots differ")
    wrapper = validate_wrapper_receipt(
        output_dir / "wrapper_receipt.tsv", output_dir=output_dir,
        experiment_id=experiment_id, source_commit=source_commit,
        bundle_sha=bundle_sha, job_id=job_id)
    process = validate_process_resources(
        output_dir / "process_resources.txt", scheduler_wall=scheduler_wall)

    return {
        "status": "KSS_RNG_K1_SCC_VALIDATION_PASS",
        "experiment_id": experiment_id,
        "source_commit": source_commit,
        "bundle_sha256": bundle_sha,
        "scheduler": {
            "job_id": job_id,
            "slots": 14,
            "wall_seconds": scheduler_wall,
            "cpu_seconds": scheduler_cpu,
            "maxvmem_bytes": scheduler_vmem,
            "hostname": qacct.get("hostname", ""),
            "queue": qacct.get("qname", ""),
            "qacct_sha256": sha256(qacct_path),
        },
        "runtime": {
            "stata_version": stata["stata_version"],
            "stata_flavor": stata["stata_flavor"],
            "stata_processors": integer(stata, "actual_stata_processors"),
            "selected_candidate": stata["selected_candidate"],
            "production_registration": "FAIL_CLOSED_UNREGISTERED",
        },
        "artifacts": {
            "golden_vector_rows": len(goldens),
            "golden_vectors_sha256": sha256(
                output_dir / "golden_vectors.csv"),
            "caller_rng_snapshot_sha256": sha256(before),
            "stata_receipt_sha256": sha256(
                output_dir / "stata_receipt.tsv"),
            "wrapper_receipt_sha256": sha256(
                output_dir / "wrapper_receipt.tsv"),
            "process": process,
        },
        "node": node,
        "wrapper": wrapper,
    }


def write_validation_receipt(path: Path, report: dict[str, Any]) -> None:
    scheduler = report["scheduler"]
    runtime = report["runtime"]
    artifacts = report["artifacts"]
    rows = {
        "receipt_version": "KSS-RNG-K1-QACCT-BOUND-V1",
        "validation_status": report["status"],
        "experiment_id": report["experiment_id"],
        "source_commit": report["source_commit"],
        "bundle_sha256": report["bundle_sha256"],
        "job_id": scheduler["job_id"],
        "qacct_sha256": scheduler["qacct_sha256"],
        "qacct_slots": scheduler["slots"],
        "qacct_wall_seconds": scheduler["wall_seconds"],
        "qacct_cpu_seconds": scheduler["cpu_seconds"],
        "qacct_maxvmem_bytes": scheduler["maxvmem_bytes"],
        "stata_version": runtime["stata_version"],
        "stata_flavor": runtime["stata_flavor"],
        "stata_processors": runtime["stata_processors"],
        "selected_candidate": runtime["selected_candidate"],
        "production_registration": runtime["production_registration"],
        "golden_vectors_sha256": artifacts["golden_vectors_sha256"],
        "caller_rng_snapshot_sha256":
            artifacts["caller_rng_snapshot_sha256"],
        "stata_receipt_sha256": artifacts["stata_receipt_sha256"],
        "wrapper_receipt_sha256": artifacts["wrapper_receipt_sha256"],
    }
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(path.name + f".tmp.{os.getpid()}")
    with temporary.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle, delimiter="\t", lineterminator="\n")
        writer.writerow(["key", "value"])
        writer.writerows(rows.items())
    os.replace(temporary, path)


def write_json(path: Path, report: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(path.name + f".tmp.{os.getpid()}")
    temporary.write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n",
        encoding="utf-8")
    os.replace(temporary, path)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--qacct", required=True, type=Path)
    parser.add_argument("--job-id-file", required=True, type=Path)
    parser.add_argument("--experiment-id", required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--bundle-sha", required=True)
    parser.add_argument("--receipt-out", required=True, type=Path)
    parser.add_argument("--report-out", required=True, type=Path)
    args = parser.parse_args()
    report = validate_run(
        output_dir=args.output_dir,
        qacct_path=args.qacct,
        job_id_file=args.job_id_file,
        experiment_id=args.experiment_id,
        source_commit=args.source_commit,
        bundle_sha=args.bundle_sha,
    )
    write_validation_receipt(args.receipt_out, report)
    write_json(args.report_out, report)
    print(
        "KSS_RNG_K1_SCC_VALIDATION_PASS "
        f"job_id={report['scheduler']['job_id']} "
        f"bundle_sha256={report['bundle_sha256']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Build an immutable wrapper receipt after success, timeout, or failure."""

import argparse
import datetime as dt
import math
import sys
from pathlib import Path

from common import (
    BenchmarkError,
    atomic_write_json,
    load_json,
    parse_gnu_time,
    sha256_file,
    validate_process_identity,
    validate_process_tree_record,
)


def optional_hash(path):
    path = Path(path)
    if not path.is_file():
        return None, 0
    return sha256_file(path), path.stat().st_size


def utc_from_ns(value):
    try:
        stamp = int(value) / 1_000_000_000.0
    except (TypeError, ValueError):
        return None
    return dt.datetime.fromtimestamp(stamp, tz=dt.UTC).isoformat()


def nonnegative_float(value):
    try:
        parsed = float(value)
    except (TypeError, ValueError):
        return None
    return parsed if math.isfinite(parsed) and parsed >= 0 else None


def build(args):
    exit_status = int(args.exit_status)
    started_ns = int(args.started_epoch_ns)
    finished_ns = int(args.finished_epoch_ns)
    case = {}
    try:
        case = load_json(args.case)
    except Exception:
        pass
    aggregate = {}
    aggregate_path = Path(args.aggregate)
    if aggregate_path.is_file():
        try:
            aggregate = load_json(aggregate_path)
        except Exception:
            pass
    identity = {}
    identity_path = Path(args.identity)
    if identity_path.is_file():
        try:
            identity = load_json(identity_path)
        except Exception:
            pass
    process_identity = {}
    process_identity_summary = {}
    process_identity_path = Path(args.process_identity)
    if process_identity_path.is_file():
        try:
            process_identity = load_json(process_identity_path)
            process_identity_summary = validate_process_identity(
                process_identity,
                expected_mode=args.mode,
                expected_label=case.get("label", args.label),
                expected_case_sha256=args.case_sha256,
            )
        except (BenchmarkError, KeyError, OSError, TypeError, ValueError):
            process_identity = {}
            process_identity_summary = {}
    process_tree = {}
    process_tree_path = Path(args.process_tree_rss)
    if process_tree_path.is_file():
        try:
            process_tree = load_json(process_tree_path)
        except Exception:
            pass

    resource = parse_gnu_time(args.time_report)
    artifacts = {}
    for name, path in (
        ("application", args.application_log),
        ("aggregate", args.aggregate),
        ("calls", args.calls),
        ("identity", args.identity),
        ("process_identity", args.process_identity),
        ("process_tree_rss", args.process_tree_rss),
        ("time_report", args.time_report),
    ):
        digest, size = optional_hash(path)
        artifacts[name] = {"path": str(path), "sha256": digest, "bytes": size}

    aggregate_pass = (
        aggregate.get("status") == "PASS"
        and aggregate.get("pool_workers") == 4
        and aggregate.get("process_identity_status") == "PASS"
        and aggregate.get("matlab_client_pid") == process_identity_summary.get("client_pid")
        and aggregate.get("matlab_worker_pids") == process_identity_summary.get("worker_pids")
    )
    identity_pass = identity.get("status") == "PASS"
    process_tree_pass = False
    try:
        validate_process_tree_record(
            process_tree,
            process_identity_summary,
            sha256_file(process_identity_path),
            identity_path=process_identity_path,
        )
        process_tree_pass = int(args.expected_pool_workers) == 4
    except (BenchmarkError, KeyError, OSError, TypeError, ValueError):
        pass
    total_reserved_gib = int(args.requested_slots) * int(args.mem_per_core_gib)
    resource_policy_pass = (
        int(args.requested_slots) == int(args.actual_slots) == 4
        and int(args.mem_per_core_gib) == 14
        and total_reserved_gib == 56
    )
    pass_marker = Path(args.pass_marker).is_file()
    status = (
        "PASS"
        if (
            exit_status == 0
            and aggregate_pass
            and identity_pass
            and process_tree_pass
            and resource_policy_pass
            and pass_marker
        )
        else "FAIL"
    )
    process_tree_peak_rss_kib = nonnegative_float(process_tree.get("peak_rss_kib"))
    process_tree_exceeds_reservation = (
        status == "PASS" and process_tree_peak_rss_kib * 1024 > total_reserved_gib * 1024**3
    )
    if process_tree_exceeds_reservation:
        status = "FAIL"
    failure_code = aggregate.get("failure_code")
    failure_message = aggregate.get("failure_message")
    if process_tree_exceeds_reservation:
        failure_code = "KSS_MATLAB_SCALE_PROCESS_TREE_RSS_EXCEEDS_RESERVATION"
        failure_message = "summed client-and-worker RSS exceeds the scheduler reservation"
    elif status == "FAIL" and failure_code in (None, "", "NONE"):
        if exit_status == 124:
            failure_code = "KSS_MATLAB_SCALE_TIMEOUT"
            failure_message = "process timeout expired"
        elif not identity_pass and identity:
            failure_code = identity.get("failure_code", "KSS_MATLAB_SCALE_IDENTITY_REJECTED")
            failure_message = identity.get("failure_message", "identity rejected")
        elif not process_tree_pass:
            failure_code = "KSS_MATLAB_SCALE_PROCESS_TREE_RSS_REJECTED"
            failure_message = "client-and-worker process-tree RSS receipt did not pass"
        else:
            failure_code = "KSS_MATLAB_SCALE_WRAPPER_FAILURE"
            failure_message = "wrapper or application did not reach its pass gate"
    if status == "PASS":
        failure_code = "NONE"
        failure_message = "NONE"

    record = {
        "schema": "kss_matlab_scale_wrapper_v1",
        "status": status,
        "failure_stage": args.failure_stage,
        "failure_code": failure_code,
        "failure_message": failure_message,
        "process_exit_status": exit_status,
        "timeout": exit_status == 124,
        "termination_signal": resource.get("termination_signal"),
        "job_id": args.job_id or None,
        "hostname": args.hostname or None,
        "sge_task_id": args.sge_task_id or None,
        "mode": args.mode,
        "label": case.get("label", args.label),
        "scale": case.get("scale"),
        "topology": case.get("topology"),
        "sample_mode": case.get("sample_mode"),
        "case_sha256": args.case_sha256,
        "source_commit": case.get("source", {}).get("source_commit"),
        "bundle_sha256": case.get("source", {}).get("bundle_sha256"),
        "input_sha256": case.get("input", {}).get("sha256"),
        "started_utc": utc_from_ns(started_ns),
        "finished_utc": utc_from_ns(finished_ns),
        "wrapper_wall_seconds": max(0.0, (finished_ns - started_ns) / 1e9),
        "input_staging_seconds": nonnegative_float(args.staging_seconds),
        "module_setup_seconds": nonnegative_float(args.module_seconds),
        "application_timeout_seconds": int(args.timeout_seconds),
        "timeout_basis": args.timeout_basis,
        "requested_slots": int(args.requested_slots),
        "actual_slots": int(args.actual_slots),
        "mem_per_core_gib": int(args.mem_per_core_gib),
        "total_reserved_gib": total_reserved_gib,
        "scheduler_hard_wall_seconds": int(args.scheduler_hard_wall_seconds),
        "identity_verification_seconds": identity.get("verification_seconds"),
        "aggregate_status": aggregate.get("status"),
        "identity_status": identity.get("status"),
        "process_identity_status": process_identity.get("status"),
        "process_identity_sha256": artifacts["process_identity"]["sha256"],
        "matlab_client_pid": process_identity_summary.get("client_pid"),
        "matlab_worker_pids": process_identity_summary.get("worker_pids", []),
        "process_tree_status": process_tree.get("status"),
        "expected_pool_workers": int(args.expected_pool_workers),
        "process_tree_sample_count": process_tree.get("sample_count"),
        "process_tree_peak_rss_kib": process_tree_peak_rss_kib,
        "process_tree_peak_process_count": process_tree.get("peak_process_count"),
        "process_tree_identity_sha256": process_tree.get("process_identity_sha256"),
        "process_tree_identity_observation_count": process_tree.get(
            "identity_observation_count"
        ),
        "process_tree_identity_peak_rss_kib": process_tree.get("identity_peak_rss_kib"),
        "process_tree_identity_peak_process_count": process_tree.get(
            "identity_peak_process_count"
        ),
        "pass_marker_present": pass_marker,
        "artifacts": artifacts,
    }
    record.update(resource)
    for field in (
        "startup_seconds",
        "import_seconds",
        "input_validation_seconds",
        "sample_selection_seconds",
        "retained_validation_seconds",
        "pool_startup_seconds",
        "mex_setup_seconds",
        "warmup_seconds",
        "core_call_median_seconds",
        "serialization_seconds",
        "pool_teardown_seconds",
        "matlab_wrapper_seconds",
    ):
        if field in aggregate:
            record[field] = aggregate[field]
    atomic_write_json(args.output, record)
    return 0 if status == "PASS" or exit_status != 0 else 2


def parse_args(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument("--case", required=True)
    parser.add_argument("--case-sha256", required=True)
    parser.add_argument("--mode", choices=("cold", "warm"), required=True)
    parser.add_argument("--label", required=True)
    parser.add_argument("--failure-stage", required=True)
    parser.add_argument("--exit-status", required=True)
    parser.add_argument("--job-id", default="")
    parser.add_argument("--hostname", default="")
    parser.add_argument("--sge-task-id", default="")
    parser.add_argument("--started-epoch-ns", required=True)
    parser.add_argument("--finished-epoch-ns", required=True)
    parser.add_argument("--staging-seconds", default="")
    parser.add_argument("--module-seconds", default="")
    parser.add_argument("--timeout-seconds", required=True)
    parser.add_argument("--timeout-basis", required=True)
    parser.add_argument("--requested-slots", required=True)
    parser.add_argument("--actual-slots", required=True)
    parser.add_argument("--mem-per-core-gib", required=True)
    parser.add_argument("--scheduler-hard-wall-seconds", required=True)
    parser.add_argument("--time-report", required=True)
    parser.add_argument("--aggregate", required=True)
    parser.add_argument("--calls", required=True)
    parser.add_argument("--identity", required=True)
    parser.add_argument("--process-identity", required=True)
    parser.add_argument("--process-tree-rss", required=True)
    parser.add_argument("--expected-pool-workers", required=True)
    parser.add_argument("--application-log", required=True)
    parser.add_argument("--pass-marker", required=True)
    parser.add_argument("--output", required=True)
    return parser.parse_args(argv)


if __name__ == "__main__":
    sys.exit(build(parse_args()))

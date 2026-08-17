#!/usr/bin/env python3
"""Build and validate a source-bound fixed-sample preparation receipt."""

import argparse
import datetime as dt
import hashlib
import sys
from pathlib import Path

from common import (
    BenchmarkError,
    atomic_write_json,
    finite,
    hash_value,
    integer,
    parse_gnu_time,
    read_one_row,
    require,
    sha256_file,
)


def optional_hash(path, *, hash_payload=True):
    item = Path(path)
    if not item.is_file():
        return {"path": str(item), "sha256": None, "bytes": 0}
    return {
        "path": str(item),
        "sha256": sha256_file(item) if hash_payload else None,
        "bytes": item.stat().st_size,
    }


def utc_from_ns(value):
    stamp = int(value) / 1_000_000_000.0
    return dt.datetime.fromtimestamp(stamp, tz=dt.UTC).isoformat()


def row_integer(row, field, minimum=0):
    try:
        value = float(row[field])
    except (KeyError, TypeError, ValueError) as exc:
        raise BenchmarkError(f"preparation summary lacks integer {field}") from exc
    return integer(value, f"preparation.{field}", minimum)


def row_finite(row, field):
    try:
        value = row[field]
    except KeyError as exc:
        raise BenchmarkError(f"preparation summary lacks finite {field}") from exc
    return finite(value, f"preparation.{field}")


def validate_canonical_keys(path, matches, workers, firms):
    """Validate and hash sorted ``worker,firm\n`` bytes in bounded memory."""
    path = Path(path)
    require(path.is_file(), "canonical retained-key input is missing")
    digest = hashlib.sha256()
    previous = None
    count = 0
    worker_count = 0
    previous_worker = None
    firms_seen = bytearray(firms + 1)
    with path.open("rb") as handle:
        for raw in handle:
            digest.update(raw)
            require(
                raw.endswith(b"\n") and not raw.endswith(b"\r\n"),
                "canonical retained-key input must use LF lines",
            )
            fields = raw[:-1].split(b",")
            require(len(fields) == 2, "canonical retained-key line must have two fields")
            try:
                worker = int(fields[0])
                firm = int(fields[1])
            except ValueError as exc:
                raise BenchmarkError("canonical retained-key IDs must be integers") from exc
            require(
                raw == f"{worker},{firm}\n".encode(),
                "canonical retained-key line is not canonical integer CSV",
            )
            require(
                1 <= worker <= workers and 1 <= firm <= firms,
                "canonical retained-key ID exceeds the dense binding",
            )
            current = (worker, firm)
            require(
                previous is None or current > previous,
                "canonical retained keys are not strictly sorted",
            )
            if worker != previous_worker:
                worker_count += 1
                previous_worker = worker
            firms_seen[firm] = 1
            previous = current
            count += 1
    require(count == matches, "canonical retained-key count differs from matches")
    require(worker_count == workers, "canonical retained-key worker count changed")
    require(sum(firms_seen) == firms, "canonical retained-key firm count changed")
    return digest.hexdigest()


def validate_success(args):
    summary = read_one_row(args.summary)
    scale = int(args.scale)
    require(scale in (1, 2, 4), "preparation scale is unsupported")
    require(
        (scale == 1 and args.topology == "well")
        or (scale in (2, 4) and args.topology in ("well_connected", "ring")),
        "preparation scale/topology pair is unsupported",
    )
    require(
        summary.get("schema") == "kss_matlab_scale_prepare_summary_v1",
        "preparation summary schema changed",
    )
    require(summary.get("status") == "PASS", "preparation summary did not pass")
    require(summary.get("label") == args.label, "preparation label changed")
    require(row_integer(summary, "scale", 1) == int(args.scale), "preparation scale changed")
    require(summary.get("topology") == args.topology, "preparation topology changed")
    require(summary.get("sample_mode") == "fixed_retained", "preparation sample mode changed")
    require(summary.get("source_commit") == args.source_commit, "preparation source commit changed")
    require(summary.get("bundle_sha256") == args.bundle_sha256, "preparation bundle changed")
    require(
        summary.get("source_input_sha256") == args.source_input_sha256,
        "preparation source input changed",
    )
    require(row_integer(summary, "requested_slots", 1) == 14, "preparation slot request changed")
    require(
        row_integer(summary, "actual_slots", 1) == int(args.actual_slots) == 14,
        "preparation actual slots changed",
    )
    require(
        row_integer(summary, "requested_processors", 1) == 4,
        "preparation processor request changed",
    )
    require(
        row_integer(summary, "actual_processors", 1) == 4,
        "preparation did not use four Stata processors",
    )
    require(row_integer(summary, "stata_mp", 0) == 1, "preparation did not run under Stata/MP")
    require(
        summary.get("retained_key_contract") == "SORTED_INTEGER_CSV_UTF8_LF_V1",
        "canonical retained-key contract changed",
    )

    dimensions = {
        field: row_integer(summary, field, 1) for field in ("rows", "workers", "firms", "matches")
    }
    input_dimensions = {
        field.removeprefix("input_"): row_integer(summary, field, 1)
        for field in ("input_rows", "input_workers", "input_firms", "input_matches")
    }
    if scale == 1:
        require(dimensions == input_dimensions, "base preparation dimensions changed")
        require(
            row_integer(summary, "connector_rows", 0) == 0, "base preparation added connector rows"
        )
    else:
        if args.topology == "well_connected":
            connector_pairs = scale * (scale - 1) // 2
        else:
            require(args.topology == "ring", "scaled preparation topology changed")
            connector_pairs = 1 if scale == 2 else scale
        connector_rows = 4 * connector_pairs
        expected_dimensions = {
            "rows": scale * input_dimensions["rows"] + connector_rows,
            "workers": scale * input_dimensions["workers"] + 2 * connector_pairs,
            "firms": scale * input_dimensions["firms"],
            "matches": scale * input_dimensions["matches"] + connector_rows,
        }
        require(
            dimensions == expected_dimensions,
            "fixture dimensions differ from the registered connector construction",
        )
        require(
            row_integer(summary, "connector_rows", 1) == connector_rows,
            "scaled fixture connector count changed",
        )

    for field in (
        "load_seconds",
        "normalization_seconds",
        "fixture_seconds",
        "export_seconds",
        "total_seconds",
    ):
        require(row_finite(summary, field) >= 0, f"preparation {field} is negative")
    require(row_finite(summary, "total_seconds") > 0, "preparation total time is not positive")

    require(Path(args.input_csv).is_file(), "prepared MATLAB input is missing")
    input_sha = hash_value(args.prepared_input_sha256, "prepared input SHA-256")
    key_sha = validate_canonical_keys(
        args.retained_keys,
        dimensions["matches"],
        dimensions["workers"],
        dimensions["firms"],
    )
    require(
        key_sha == hash_value(args.retained_key_sha256, "retained-key SHA-256"),
        "wrapper and canonical retained-key digests differ",
    )
    marker = Path(args.pass_marker)
    require(marker.is_file(), "Stata preparation pass marker is missing")
    expected_marker = (
        "KSS_MATLAB_SCALE_PREPARE_PASS "
        f"{args.label} {args.source_commit} {args.bundle_sha256} "
        f"{args.source_input_sha256}\n"
    )
    require(
        marker.read_text(encoding="utf-8") == expected_marker,
        "Stata preparation pass marker changed",
    )
    return summary, dimensions, input_dimensions, input_sha, key_sha


def build(args):
    process_exit_status = int(args.exit_status)
    started_ns = int(args.started_epoch_ns)
    finished_ns = int(args.finished_epoch_ns)
    source_commit = hash_value(args.source_commit, "source commit", 40)
    bundle_sha = hash_value(args.bundle_sha256, "bundle SHA-256")
    source_input_sha = hash_value(args.source_input_sha256, "source input SHA-256")
    artifacts = {
        name: optional_hash(path)
        for name, path in (
            ("application", args.application_log),
            ("summary", args.summary),
            ("stata_pass_marker", args.pass_marker),
            ("time_report", args.time_report),
        )
    }
    artifacts["input_csv"] = optional_hash(args.input_csv, hash_payload=False)
    artifacts["retained_keys"] = optional_hash(args.retained_keys, hash_payload=False)
    status = "FAIL"
    failure_code = "KSS_MATLAB_SCALE_PREPARE_WRAPPER_FAILURE"
    failure_message = "preparation did not reach its validated output gate"
    summary = {}
    dimensions = {}
    input_dimensions = {}
    input_csv_sha = None
    retained_key_sha = None
    if process_exit_status == 124:
        failure_code = "KSS_MATLAB_SCALE_PREPARE_TIMEOUT"
        failure_message = "Stata preparation timeout expired"
    elif process_exit_status == 0:
        try:
            summary, dimensions, input_dimensions, input_csv_sha, retained_key_sha = (
                validate_success(args)
            )
            status = "PASS"
            failure_code = "NONE"
            failure_message = "NONE"
            artifacts["input_csv"]["sha256"] = input_csv_sha
            artifacts["retained_keys"]["sha256"] = retained_key_sha
        except (BenchmarkError, OSError, ValueError) as exc:
            failure_code = "KSS_MATLAB_SCALE_PREPARE_OUTPUT_REJECTED"
            failure_message = str(exc)

    record = {
        "schema": "kss_matlab_scale_preparation_v1",
        "status": status,
        "failure_stage": args.failure_stage,
        "failure_code": failure_code,
        "failure_message": failure_message,
        "process_exit_status": process_exit_status,
        "timeout": process_exit_status == 124,
        "job_id": args.job_id or None,
        "hostname": args.hostname or None,
        "label": args.label,
        "scale": int(args.scale),
        "topology": args.topology,
        "sample_mode": "fixed_retained",
        "source_commit": source_commit,
        "bundle_sha256": bundle_sha,
        "source_input_sha256": source_input_sha,
        "prepared_input_sha256": input_csv_sha,
        "prepared_input_hash_source": "wrapper_sha256sum" if input_csv_sha else None,
        "retained_key_sha256": retained_key_sha,
        "dimensions": dimensions,
        "source_dimensions": input_dimensions,
        "requested_slots": int(args.requested_slots),
        "actual_slots": int(args.actual_slots),
        "mem_per_core_gib": int(args.mem_per_core_gib),
        "total_reserved_gib": int(args.requested_slots) * int(args.mem_per_core_gib),
        "requested_stata_processors": int(args.stata_processors),
        "application_timeout_seconds": int(args.timeout_seconds),
        "timeout_basis": args.timeout_basis,
        "scheduler_hard_wall_seconds": int(args.scheduler_hard_wall_seconds),
        "started_utc": utc_from_ns(started_ns),
        "finished_utc": utc_from_ns(finished_ns),
        "wrapper_wall_seconds": max(0.0, (finished_ns - started_ns) / 1e9),
        "input_staging_seconds": finite(args.staging_seconds, "staging seconds"),
        "module_setup_seconds": finite(args.module_seconds, "module seconds"),
        "output_promotion_seconds": finite(args.promotion_seconds, "promotion seconds"),
        "artifacts": artifacts,
    }
    record.update(parse_gnu_time(args.time_report))
    for field in (
        "load_seconds",
        "normalization_seconds",
        "fixture_seconds",
        "export_seconds",
        "total_seconds",
        "connector_rows",
        "copy_cut_conductance",
        "normalized_lambda2",
        "normalized_lambda_max",
        "normalized_condition_proxy",
        "minimum_weighted_degree",
        "maximum_weighted_degree",
        "stata_version",
        "stata_flavor",
    ):
        if field in summary and summary[field] not in (None, ""):
            value = summary[field]
            if (
                field.endswith("_seconds")
                or field.startswith("normalized_")
                or field
                in {
                    "copy_cut_conductance",
                    "minimum_weighted_degree",
                    "maximum_weighted_degree",
                }
            ):
                value = float(value)
            elif field == "connector_rows":
                value = int(float(value))
            record[field] = value
    atomic_write_json(args.output, record)
    return 0 if status == "PASS" or process_exit_status != 0 else 2


def parse_args(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument("--label", required=True)
    parser.add_argument("--scale", required=True)
    parser.add_argument("--topology", required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--bundle-sha256", required=True)
    parser.add_argument("--source-input-sha256", required=True)
    parser.add_argument("--prepared-input-sha256", default="UNKNOWN")
    parser.add_argument("--retained-key-sha256", default="UNKNOWN")
    parser.add_argument("--failure-stage", required=True)
    parser.add_argument("--exit-status", required=True)
    parser.add_argument("--job-id", default="")
    parser.add_argument("--hostname", default="")
    parser.add_argument("--requested-slots", required=True)
    parser.add_argument("--actual-slots", required=True)
    parser.add_argument("--mem-per-core-gib", required=True)
    parser.add_argument("--stata-processors", required=True)
    parser.add_argument("--timeout-seconds", required=True)
    parser.add_argument("--timeout-basis", required=True)
    parser.add_argument("--scheduler-hard-wall-seconds", required=True)
    parser.add_argument("--started-epoch-ns", required=True)
    parser.add_argument("--finished-epoch-ns", required=True)
    parser.add_argument("--staging-seconds", default="0")
    parser.add_argument("--module-seconds", default="0")
    parser.add_argument("--promotion-seconds", default="0")
    parser.add_argument("--time-report", required=True)
    parser.add_argument("--application-log", required=True)
    parser.add_argument("--input-csv", required=True)
    parser.add_argument("--retained-keys", required=True)
    parser.add_argument("--summary", required=True)
    parser.add_argument("--pass-marker", required=True)
    parser.add_argument("--output", required=True)
    return parser.parse_args(argv)


if __name__ == "__main__":
    try:
        sys.exit(build(parse_args()))
    except (BenchmarkError, OSError, TypeError, ValueError) as exc:
        print(f"KSS MATLAB SCALE PREPARATION RECEIPT FAIL: {exc}", file=sys.stderr)
        sys.exit(2)

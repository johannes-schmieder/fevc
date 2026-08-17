#!/usr/bin/env python3
"""Validate cold/warm MATLAB scale evidence and write a comparison receipt."""

import argparse
import csv
import statistics
import sys
from pathlib import Path

from common import (
    BenchmarkError,
    atomic_write_json,
    finite,
    hash_value,
    integer,
    load_json,
    matrix_relative_difference,
    read_one_row,
    require,
    sha256_file,
    target_identity,
    validate_case,
)

TARGETS = ("worker", "firm", "covariance", "total")


def load_calls(path):
    try:
        with Path(path).open("r", encoding="utf-8", newline="") as handle:
            rows = list(csv.DictReader(handle))
    except OSError as exc:
        raise BenchmarkError(f"cannot read calls receipt: {exc}") from exc
    require(rows, "calls receipt is empty")
    required = {
        "role",
        "call_index",
        "seconds",
        "corrected_worker",
        "corrected_firm",
        "corrected_covariance",
        "corrected_total",
        "identity_scaled_error",
        "target_sha256",
        "detail_sha256",
        "retained_key_sha256",
        "detail_matches",
        "retained_workers",
        "retained_firms",
        "retained_physical_rows",
    }
    require(set(rows[0]) == required, "calls receipt schema changed")
    for row in rows:
        require(row["role"] in ("cold", "warmup", "measured"), "invalid call role")
        integer(int(row["call_index"]), "call_index", 0)
        require(finite(row["seconds"], "call seconds") >= 0, "negative call time")
        corrected = {
            target: finite(row["corrected_" + target], "corrected_" + target) for target in TARGETS
        }
        recorded_error = finite(row["identity_scaled_error"], "identity_scaled_error")
        require(
            recorded_error <= 1e-10 and target_identity(corrected) <= 1e-10,
            "corrected accounting identity failed",
        )
        for field in ("target_sha256", "detail_sha256", "retained_key_sha256"):
            hash_value(row[field], field)
        for field in (
            "detail_matches",
            "retained_workers",
            "retained_firms",
            "retained_physical_rows",
        ):
            integer(int(row[field]), field, 1)
    return rows


def validate_job(job_dir, mode, case, case_sha):
    job_dir = Path(job_dir)
    output = job_dir / "output"
    wrapper = load_json(job_dir / "wrapper.json")
    aggregate = load_json(output / "aggregate.json")
    identity = load_json(job_dir / "identity.json")
    calls = load_calls(output / "calls.csv")
    require(
        wrapper.get("schema") == "kss_matlab_scale_wrapper_v1", f"{mode} wrapper schema changed"
    )
    require(
        aggregate.get("schema") == "kss_matlab_scale_aggregate_v1",
        f"{mode} aggregate schema changed",
    )
    require(
        identity.get("schema") == "kss_matlab_scale_identity_v1"
        and identity.get("status") == "PASS",
        f"{mode} identity receipt did not pass",
    )
    require(
        wrapper.get("status") == "PASS" and aggregate.get("status") == "PASS",
        f"{mode} job did not pass",
    )
    require(wrapper.get("mode") == mode and aggregate.get("mode") == mode, f"{mode} mode mismatch")
    for record, name in ((wrapper, "wrapper"), (aggregate, "aggregate")):
        require(record.get("label") == case["label"], f"{mode} {name} label mismatch")
        require(record.get("scale") == case["scale"], f"{mode} {name} scale mismatch")
        require(record.get("topology") == case["topology"], f"{mode} {name} topology mismatch")
        require(
            record.get("sample_mode") == case["sample_mode"], f"{mode} {name} sample mode mismatch"
        )
        require(record.get("case_sha256") == case_sha, f"{mode} {name} case checksum mismatch")
        require(
            record.get("input_sha256") == case["input"]["sha256"],
            f"{mode} {name} input checksum mismatch",
        )
        require(
            record.get("source_commit") == case["source"]["source_commit"],
            f"{mode} {name} source commit mismatch",
        )
        require(
            record.get("bundle_sha256") == case["source"]["bundle_sha256"],
            f"{mode} {name} bundle checksum mismatch",
        )
    require(
        identity.get("case_sha256") == case_sha
        and identity.get("input_sha256") == case["input"]["sha256"]
        and identity.get("matlab_runtime_tree_sha256")
        == case["source"]["matlab_runtime_tree_sha256"],
        f"{mode} identity binding changed",
    )
    require(integer(aggregate.get("probes"), "aggregate probes", 1) == 200, "MATLAB probes changed")
    require(
        integer(aggregate.get("pool_workers"), "pool workers", 1) == 4, "MATLAB pool size changed"
    )
    require(
        wrapper.get("process_exit_status") == 0 and wrapper.get("pass_marker_present") is True,
        f"{mode} wrapper pass gate failed",
    )
    marker = (output / "wrapper.pass").read_text(encoding="utf-8").strip()
    require(
        marker
        == "KSS_MATLAB_SCALE_PASS {} {} {} {} {}".format(
            mode, case["label"], case_sha, case["input"]["sha256"], calls[0]["retained_key_sha256"]
        ),
        f"{mode} MATLAB pass marker binding changed",
    )
    artifacts = wrapper.get("artifacts")
    require(isinstance(artifacts, dict), f"{mode} wrapper artifacts missing")
    expected_artifacts = {
        "aggregate": output / "aggregate.json",
        "calls": output / "calls.csv",
        "identity": job_dir / "identity.json",
        "application": job_dir / "application.txt",
        "time_report": job_dir / "resources.txt",
    }
    for name, path in expected_artifacts.items():
        require(
            path.is_file() and artifacts.get(name, {}).get("sha256") == sha256_file(path),
            f"{mode} {name} artifact binding changed",
        )
    for field in ("wrapper_wall_seconds", "process_wall_seconds", "cpu_seconds", "peak_rss_kib"):
        require(finite(wrapper.get(field), f"{mode} {field}") > 0, f"{mode} lacks positive {field}")
    for field in ("input_staging_seconds", "module_setup_seconds"):
        require(
            finite(wrapper.get(field), f"{mode} {field}") >= 0,
            f"{mode} lacks wrapper stage {field}",
        )
    require(
        finite(aggregate.get("startup_seconds"), f"{mode} startup_seconds") > 0,
        f"{mode} lacks executable startup timing",
    )
    for field in (
        "import_seconds",
        "input_validation_seconds",
        "retained_validation_seconds",
        "pool_startup_seconds",
        "mex_setup_seconds",
        "serialization_seconds",
        "pool_teardown_seconds",
    ):
        require(
            finite(aggregate.get(field), f"{mode} {field}") >= 0,
            f"{mode} lacks stage timing {field}",
        )
    if case["sample_mode"] == "selection":
        require(
            aggregate.get("profile_status") == "REGISTERED_SOURCE_SELF_TIME",
            f"{mode} selection profile is unavailable",
        )
    else:
        require(
            aggregate.get("profile_status") == "NOT_PROFILED_FIXED_SAMPLE",
            f"{mode} fixed-sample profile status changed",
        )

    if mode == "cold":
        require(
            len(calls) == 1 and calls[0]["role"] == "cold",
            "cold process must contain exactly one estimator call",
        )
        require(
            integer(aggregate.get("estimator_call_count"), "cold estimator calls", 0) == 1,
            "cold process call count changed",
        )
        require(
            integer(aggregate.get("warmup_call_count"), "cold warmup calls", 0) == 0,
            "cold process contains a warmup",
        )
        require(
            integer(aggregate.get("measured_call_count"), "cold measured calls", 0) == 1,
            "cold process measured call count changed",
        )
    else:
        repetitions = case["warm_repetitions"]
        require(len(calls) == repetitions + 1, "warm process call count changed")
        require(
            calls[0]["role"] == "warmup" and all(row["role"] == "measured" for row in calls[1:]),
            "warm call roles changed",
        )
        require(
            integer(aggregate.get("warmup_call_count"), "warmup calls", 0) == 1,
            "warm process must contain one warmup",
        )
        require(
            integer(aggregate.get("measured_call_count"), "warm measured calls", 0) == repetitions,
            "warm repetition count changed",
        )
        require(
            integer(aggregate.get("estimator_call_count"), "warm estimator calls", 0)
            == repetitions + 1,
            "warm total call count changed",
        )

    key_hashes = {row["retained_key_sha256"] for row in calls}
    retained_dimensions = {
        (
            row["detail_matches"],
            row["retained_workers"],
            row["retained_firms"],
            row["retained_physical_rows"],
        )
        for row in calls
    }
    require(
        len(key_hashes) == 1 and len(retained_dimensions) == 1,
        f"{mode} retained sample changed across calls",
    )
    reference = case["reference_sample"]
    first = calls[0]
    sample_exact = (
        first["retained_key_sha256"] == reference["retained_key_sha256"]
        and int(first["detail_matches"]) == reference["matches"]
        and int(first["retained_workers"]) == reference["workers"]
        and int(first["retained_firms"]) == reference["firms"]
        and int(first["retained_physical_rows"]) == reference["rows"]
    )
    if case["sample_mode"] == "fixed":
        require(sample_exact, f"{mode} fixed retained sample changed")
    return {
        "wrapper": wrapper,
        "aggregate": aggregate,
        "calls": calls,
        "sample_exact": sample_exact,
        "retained_key_sha256": first["retained_key_sha256"],
        "retained_dimensions": {
            "rows": int(first["retained_physical_rows"]),
            "workers": int(first["retained_workers"]),
            "firms": int(first["retained_firms"]),
            "matches": int(first["detail_matches"]),
        },
    }


def candidate_comparison(path, case, tolerance):
    if path is None:
        return {
            "status": "REFERENCE_BOUND_ONLY",
            "explanation": (
                "The maintained interface does not expose plug-in components; "
                "the checksum-bound reference plug-in is reported without "
                "inferring it from corrected estimates."
            ),
        }
    require(
        sha256_file(path) == case["reference_sample"]["receipt_sha256"],
        "candidate aggregate does not match the bound reference receipt",
    )
    candidate = read_one_row(path)
    values = [
        finite(candidate["plugin_" + target], "candidate plugin_" + target) for target in TARGETS
    ]
    candidate_plugin = dict(zip(TARGETS, values, strict=False))
    require(
        target_identity(candidate_plugin) <= 1e-10, "candidate plug-in accounting identity failed"
    )
    reference_values = [
        finite(case["reference_sample"]["plugin"][target], "reference plugin") for target in TARGETS
    ]
    difference = matrix_relative_difference(values, reference_values)
    require(difference <= tolerance, "candidate/reference plug-in difference exceeds tolerance")
    dimension_map = {
        "rows": "N_retained",
        "workers": "worker_levels",
        "firms": "firm_levels",
        "matches": "deletion_units",
    }
    for name, field in dimension_map.items():
        require(
            integer(float(candidate[field]), "candidate " + field, 1)
            == case["reference_sample"][name],
            f"candidate/reference {name} mismatch",
        )
    return {
        "status": "COMPARABLE",
        "receipt_sha256": sha256_file(path),
        "mreldif": difference,
        "tolerance": tolerance,
        "plugin": candidate_plugin,
    }


def validate(args):
    contract = load_json(args.contract)
    case_sha = sha256_file(args.case)
    require(case_sha == args.case_sha256, "case checksum mismatch")
    case = validate_case(load_json(args.case), contract)
    require(
        case["source"]["benchmark_contract_sha256"] == sha256_file(args.contract),
        "case/source contract mismatch",
    )
    cold = validate_job(args.cold_job_dir, "cold", case, case_sha)
    warm = validate_job(args.warm_job_dir, "warm", case, case_sha)
    require(
        cold["retained_key_sha256"] == warm["retained_key_sha256"]
        and cold["retained_dimensions"] == warm["retained_dimensions"],
        "cold/warm retained sample changed",
    )
    plugin = candidate_comparison(args.reference_aggregate, case, args.plugin_tolerance)
    plugin["matlab_sample_comparability"] = (
        "COMPARABLE" if cold["sample_exact"] else "NOT_COMPARABLE_SAMPLE_DIFFERENCE"
    )
    measured_seconds = [float(row["seconds"]) for row in warm["calls"][1:]]
    comparison = {
        "schema": "kss_matlab_scale_comparison_v1",
        "status": "PASS",
        "label": case["label"],
        "scale": case["scale"],
        "topology": case["topology"],
        "sample_mode": case["sample_mode"],
        "case_sha256": case_sha,
        "source_commit": case["source"]["source_commit"],
        "bundle_sha256": case["source"]["bundle_sha256"],
        "input_sha256": case["input"]["sha256"],
        "reference_sample": case["reference_sample"],
        "matlab_retained_sample": cold["retained_dimensions"],
        "matlab_retained_key_sha256": cold["retained_key_sha256"],
        "reference_sample_exact_match": cold["sample_exact"],
        "sample_comparison_status": ("EXACT" if cold["sample_exact"] else "DESCRIPTIVE_DIFFERENCE"),
        "plugin_comparison": plugin,
        "corrected_comparison_policy": "IDENTITY_ONLY_NO_EQUALITY_GATE",
        "corrected_accounting_identity_pass": True,
        "cold": {
            "core_call_seconds": float(cold["calls"][0]["seconds"]),
            "wrapper_wall_seconds": cold["wrapper"]["wrapper_wall_seconds"],
            "cpu_seconds": cold["wrapper"]["cpu_seconds"],
            "peak_rss_kib": cold["wrapper"]["peak_rss_kib"],
            "stages": {
                field: cold["aggregate"].get(field)
                for field in (
                    "startup_seconds",
                    "import_seconds",
                    "input_validation_seconds",
                    "sample_selection_seconds",
                    "retained_validation_seconds",
                    "pool_startup_seconds",
                    "mex_setup_seconds",
                    "serialization_seconds",
                    "pool_teardown_seconds",
                )
            },
        },
        "warm": {
            "repetitions": len(measured_seconds),
            "call_seconds": measured_seconds,
            "median_call_seconds": statistics.median(measured_seconds),
            "min_call_seconds": min(measured_seconds),
            "max_call_seconds": max(measured_seconds),
            "wrapper_wall_seconds": warm["wrapper"]["wrapper_wall_seconds"],
            "cpu_seconds": warm["wrapper"]["cpu_seconds"],
            "peak_rss_kib": warm["wrapper"]["peak_rss_kib"],
            "stages": {
                field: warm["aggregate"].get(field)
                for field in (
                    "startup_seconds",
                    "import_seconds",
                    "input_validation_seconds",
                    "sample_selection_seconds",
                    "retained_validation_seconds",
                    "pool_startup_seconds",
                    "mex_setup_seconds",
                    "warmup_seconds",
                    "serialization_seconds",
                    "pool_teardown_seconds",
                )
            },
        },
    }
    if args.output:
        atomic_write_json(args.output, comparison)
    print("KSS MATLAB SCALE VALIDATION PASS: {}".format(case["label"]))
    return 0


def parse_args(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument("--case", required=True)
    parser.add_argument("--case-sha256", required=True)
    parser.add_argument("--contract", required=True)
    parser.add_argument("--cold-job-dir", required=True)
    parser.add_argument("--warm-job-dir", required=True)
    parser.add_argument("--reference-aggregate")
    parser.add_argument("--plugin-tolerance", type=float, default=1e-8)
    parser.add_argument("--output")
    return parser.parse_args(argv)


if __name__ == "__main__":
    try:
        sys.exit(validate(parse_args()))
    except (BenchmarkError, KeyError, ValueError) as exc:
        print(f"KSS MATLAB SCALE VALIDATION FAIL: {exc}", file=sys.stderr)
        sys.exit(2)

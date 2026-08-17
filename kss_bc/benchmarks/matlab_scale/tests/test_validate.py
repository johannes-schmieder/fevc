import argparse
import csv
import json
from pathlib import Path

import pytest
from common import BenchmarkError, sha256_file
from conftest import write_csv, write_json
from test_common import CONTRACT_PATH, valid_case
from validate import validate

CALL_FIELDS = [
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
]


def call(role, index, seconds, key_hash, shift=0.0):
    worker = 0.9 + shift
    firm = 0.2 - shift
    covariance = 0.05
    total = worker + firm + 2 * covariance
    return {
        "role": role,
        "call_index": index,
        "seconds": seconds,
        "corrected_worker": worker,
        "corrected_firm": firm,
        "corrected_covariance": covariance,
        "corrected_total": total,
        "identity_scaled_error": 0.0,
        "target_sha256": "a" * 64,
        "detail_sha256": "b" * 64,
        "retained_key_sha256": key_hash,
        "detail_matches": 20,
        "retained_workers": 10,
        "retained_firms": 5,
        "retained_physical_rows": 100,
    }


def write_job(root, mode, case, case_sha, shifts):
    output = root / "output"
    output.mkdir(parents=True)
    if mode == "cold":
        rows = [call("cold", 1, 7.0, case["reference_sample"]["retained_key_sha256"], shifts[0])]
        warmups = 0
        measured = 1
    else:
        rows = [call("warmup", 0, 5.0, case["reference_sample"]["retained_key_sha256"], shifts[0])]
        rows.extend(
            call(
                "measured",
                index,
                3.0 + index / 10.0,
                case["reference_sample"]["retained_key_sha256"],
                shift,
            )
            for index, shift in enumerate(shifts[1:], start=1)
        )
        warmups = 1
        measured = len(rows) - 1
    write_csv(output / "calls.csv", rows)
    aggregate = {
        "schema": "kss_matlab_scale_aggregate_v1",
        "status": "PASS",
        "mode": mode,
        "label": case["label"],
        "scale": case["scale"],
        "topology": case["topology"],
        "sample_mode": case["sample_mode"],
        "case_sha256": case_sha,
        "input_sha256": case["input"]["sha256"],
        "source_commit": case["source"]["source_commit"],
        "bundle_sha256": case["source"]["bundle_sha256"],
        "probes": 200,
        "pool_workers": 4,
        "estimator_call_count": len(rows),
        "warmup_call_count": warmups,
        "measured_call_count": measured,
        "startup_seconds": 60.0,
        "import_seconds": 2.0,
        "input_validation_seconds": 1.0,
        "sample_selection_seconds": 0.0,
        "profile_status": "NOT_PROFILED_FIXED_SAMPLE",
        "retained_validation_seconds": 0.5,
        "pool_startup_seconds": 40.0,
        "mex_setup_seconds": 5.0,
        "warmup_seconds": rows[0]["seconds"] if mode == "warm" else 0.0,
        "serialization_seconds": 0.2,
        "pool_teardown_seconds": 1.0,
    }
    write_json(output / "aggregate.json", aggregate)
    (output / "wrapper.pass").write_text(
        "KSS_MATLAB_SCALE_PASS {} {} {} {} {}\n".format(
            mode,
            case["label"],
            case_sha,
            case["input"]["sha256"],
            case["reference_sample"]["retained_key_sha256"],
        ),
        encoding="utf-8",
    )
    identity = {
        "schema": "kss_matlab_scale_identity_v1",
        "status": "PASS",
        "case_sha256": case_sha,
        "input_sha256": case["input"]["sha256"],
        "matlab_runtime_tree_sha256": case["source"]["matlab_runtime_tree_sha256"],
    }
    write_json(root / "identity.json", identity)
    (root / "application.txt").write_text("PASS\n", encoding="utf-8")
    (root / "resources.txt").write_text("RSS\n", encoding="utf-8")
    artifacts = {}
    for name, path in {
        "aggregate": output / "aggregate.json",
        "calls": output / "calls.csv",
        "identity": root / "identity.json",
        "application": root / "application.txt",
        "time_report": root / "resources.txt",
    }.items():
        artifacts[name] = {"sha256": sha256_file(path), "path": str(path)}
    wrapper = {
        "schema": "kss_matlab_scale_wrapper_v1",
        "status": "PASS",
        "mode": mode,
        "label": case["label"],
        "scale": case["scale"],
        "topology": case["topology"],
        "sample_mode": case["sample_mode"],
        "case_sha256": case_sha,
        "input_sha256": case["input"]["sha256"],
        "source_commit": case["source"]["source_commit"],
        "bundle_sha256": case["source"]["bundle_sha256"],
        "process_exit_status": 0,
        "pass_marker_present": True,
        "wrapper_wall_seconds": 120.0,
        "process_wall_seconds": 110.0,
        "cpu_seconds": 160.0,
        "input_staging_seconds": 2.0,
        "module_setup_seconds": 3.0,
        "peak_rss_kib": 2_000_000,
        "artifacts": artifacts,
    }
    write_json(root / "wrapper.json", wrapper)


def setup_evidence(tmp_path):
    candidate = tmp_path / "reference.csv"
    write_csv(
        candidate,
        [
            {
                "N_retained": 100,
                "worker_levels": 10,
                "firm_levels": 5,
                "deletion_units": 20,
                "plugin_worker": 1.0,
                "plugin_firm": 2.0,
                "plugin_covariance": 0.5,
                "plugin_total": 4.0,
            }
        ],
    )
    case = valid_case()
    case["reference_sample"]["receipt_sha256"] = sha256_file(candidate)
    case_path = tmp_path / "case.json"
    write_json(case_path, case)
    case_sha = sha256_file(case_path)
    cold = tmp_path / "cold"
    warm = tmp_path / "warm"
    write_job(cold, "cold", case, case_sha, [0.0])
    # Deliberately vary corrected targets: the validator may check each
    # identity but must never impose corrected-estimate equality.
    write_job(warm, "warm", case, case_sha, [0.01, 0.02, -0.01, 0.03])
    args = argparse.Namespace(
        case=str(case_path),
        case_sha256=case_sha,
        contract=str(CONTRACT_PATH),
        cold_job_dir=str(cold),
        warm_job_dir=str(warm),
        reference_aggregate=str(candidate),
        plugin_tolerance=1e-8,
        output=str(tmp_path / "comparison.json"),
    )
    return args


def refresh_calls_artifact(job_dir):
    job_dir = Path(job_dir)
    wrapper_path = job_dir / "wrapper.json"
    wrapper = json.loads(wrapper_path.read_text(encoding="utf-8"))
    wrapper["artifacts"]["calls"]["sha256"] = sha256_file(job_dir / "output" / "calls.csv")
    rows = list(csv.DictReader((job_dir / "output" / "calls.csv").open(encoding="utf-8")))
    (job_dir / "output" / "wrapper.pass").write_text(
        "KSS_MATLAB_SCALE_PASS {} {} {} {} {}\n".format(
            wrapper["mode"],
            wrapper["label"],
            wrapper["case_sha256"],
            wrapper["input_sha256"],
            rows[0]["retained_key_sha256"],
        ),
        encoding="utf-8",
    )
    write_json(wrapper_path, wrapper)


def test_validator_compares_sample_plugin_stages_and_resources(tmp_path):
    args = setup_evidence(tmp_path)
    assert validate(args) == 0
    result = json.loads(Path(args.output).read_text(encoding="utf-8"))
    assert result["status"] == "PASS"
    assert result["reference_sample_exact_match"] is True
    assert result["plugin_comparison"]["status"] == "COMPARABLE"
    assert result["plugin_comparison"]["mreldif"] == 0
    assert result["corrected_comparison_policy"] == "IDENTITY_ONLY_NO_EQUALITY_GATE"
    assert result["warm"]["repetitions"] == 3
    assert result["warm"]["peak_rss_kib"] == 2_000_000
    assert result["cold"]["stages"]["mex_setup_seconds"] == 5.0


def test_validator_rejects_a_cold_process_with_more_than_one_call(tmp_path):
    args = setup_evidence(tmp_path)
    calls_path = Path(args.cold_job_dir) / "output" / "calls.csv"
    rows = list(csv.DictReader(calls_path.open(encoding="utf-8")))
    rows.append(dict(rows[0], call_index="2"))
    write_csv(calls_path, rows)
    refresh_calls_artifact(args.cold_job_dir)
    with pytest.raises(BenchmarkError, match="exactly one estimator call"):
        validate(args)


def test_validator_rejects_fixed_sample_drift(tmp_path):
    args = setup_evidence(tmp_path)
    calls_path = Path(args.warm_job_dir) / "output" / "calls.csv"
    rows = list(csv.DictReader(calls_path.open(encoding="utf-8")))
    for row in rows:
        row["retained_key_sha256"] = "d" * 64
    write_csv(calls_path, rows)
    refresh_calls_artifact(args.warm_job_dir)
    with pytest.raises(BenchmarkError, match="fixed retained sample changed"):
        validate(args)

import argparse
import csv
import json
from pathlib import Path

import pytest
from common import BenchmarkError, sha256_file
from conftest import write_csv, write_json, write_process_evidence, write_scc_acceptance
from test_common import CONTRACT_PATH, reference_provenance, valid_case
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
    process_identity_path, process_tree_path = write_process_evidence(
        root, mode=mode, label=case["label"], case_sha256=case_sha
    )
    process_identity = json.loads(process_identity_path.read_text(encoding="utf-8"))
    process_tree = json.loads(process_tree_path.read_text(encoding="utf-8"))
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
        "process_identity_status": "PASS",
        "matlab_client_pid": process_identity["client_pid"],
        "matlab_worker_pids": process_identity["worker_pids"],
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
        "preparation_receipt_sha256": case["preparation"]["receipt_sha256"],
        "preparation_acceptance_sha256": case["preparation"]["acceptance_sha256"],
        "executing_source_commit": case["source"]["source_commit"],
        "executing_bundle_sha256": case["source"]["bundle_sha256"],
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
        "process_identity": process_identity_path,
        "process_tree_rss": root / "process_tree_rss.json",
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
        "job_id": "12345",
        "hostname": "scc-test",
        "sge_task_id": "undefined",
        "pass_marker_present": True,
        "wrapper_wall_seconds": 120.0,
        "process_wall_seconds": 110.0,
        "cpu_seconds": 160.0,
        "input_staging_seconds": 2.0,
        "module_setup_seconds": 3.0,
        "peak_rss_kib": 2_000_000,
        "time_exit_status": 0,
        "requested_slots": 4,
        "actual_slots": 4,
        "mem_per_core_gib": 14,
        "total_reserved_gib": 56,
        "scheduler_hard_wall_seconds": 4200,
        "application_timeout_seconds": 3600,
        "timeout_basis": "measured-v1",
        "process_tree_status": "PASS",
        "process_identity_status": "PASS",
        "process_identity_sha256": sha256_file(process_identity_path),
        "matlab_client_pid": process_identity["client_pid"],
        "matlab_worker_pids": process_identity["worker_pids"],
        "expected_pool_workers": 4,
        "process_tree_sample_count": process_tree["sample_count"],
        "process_tree_peak_rss_kib": process_tree["peak_rss_kib"],
        "process_tree_peak_process_count": process_tree["peak_process_count"],
        "process_tree_identity_sha256": process_tree["process_identity_sha256"],
        "process_tree_identity_observation_count": process_tree[
            "identity_observation_count"
        ],
        "process_tree_identity_peak_rss_kib": process_tree["identity_peak_rss_kib"],
        "process_tree_identity_peak_process_count": process_tree[
            "identity_peak_process_count"
        ],
        "artifacts": artifacts,
    }
    write_json(root / "wrapper.json", wrapper)


def setup_evidence(tmp_path):
    candidate = tmp_path / "reference.json"
    write_json(
        candidate,
            {
                "schema": "kss_matlab_scale_reference_v1",
                "status": "PASS",
                "kind": "stata_kss_bc",
                "label": "scale4-well",
                "sample_mode": "fixed_retained",
                "source_commit": "1" * 40,
                "bundle_sha256": "2" * 64,
                "prepared_input_sha256": "6" * 64,
                "retained_key_sha256": "8" * 64,
                "N_retained": 100,
                "worker_levels": 10,
                "firm_levels": 5,
                "deletion_units": 20,
                "algorithm_requested": "jla",
                "algorithm_selected": "jla",
                "deletion": "match",
                "controls": "none",
                "frequency_semantics": "literal_physical_rows_v1",
                "target_weight_semantics": "uniform_stored_rows_v1",
                "requested_probes": 200,
                "seed": 8675309,
                "plugin_worker": 1.0,
                "plugin_firm": 2.0,
                "plugin_covariance": 0.5,
                "plugin_total": 4.0,
                "provenance": reference_provenance(),
            },
    )
    case = valid_case()
    case["reference_sample"]["receipt_sha256"] = sha256_file(candidate)
    case_root = tmp_path / "matlab_scale" / case["label"]
    case_root.mkdir(parents=True)
    case_path = case_root / "case.json"
    write_json(case_path, case)
    case_sha = sha256_file(case_path)
    cold = case_root / "cold"
    warm = case_root / "warm"
    write_job(cold, "cold", case, case_sha, [0.0])
    # Deliberately vary corrected targets: the validator may check each
    # identity but must never impose corrected-estimate equality.
    write_job(warm, "warm", case, case_sha, [0.01, 0.02, -0.01, 0.03])
    cold_acceptance = write_scc_acceptance(
        tmp_path,
        stage="cold",
        label=case["label"],
        source_commit=case["source"]["source_commit"],
        bundle_sha256=case["source"]["bundle_sha256"],
        input_sha256=case["input"]["sha256"],
        job_dir=cold,
        case_path=case_path,
        case_sha256=case_sha,
        contract_path=CONTRACT_PATH,
        preparation_receipt_sha256=case["preparation"]["receipt_sha256"],
        preparation_acceptance_sha256=case["preparation"]["acceptance_sha256"],
        case_gate_sha256="7" * 64,
    )
    warm_acceptance = write_scc_acceptance(
        tmp_path,
        stage="warm",
        label=case["label"],
        source_commit=case["source"]["source_commit"],
        bundle_sha256=case["source"]["bundle_sha256"],
        input_sha256=case["input"]["sha256"],
        job_dir=warm,
        case_path=case_path,
        case_sha256=case_sha,
        contract_path=CONTRACT_PATH,
        preparation_receipt_sha256=case["preparation"]["receipt_sha256"],
        preparation_acceptance_sha256=case["preparation"]["acceptance_sha256"],
        case_gate_sha256="7" * 64,
    )
    args = argparse.Namespace(
        case=str(case_path),
        case_sha256=case_sha,
        contract=str(CONTRACT_PATH),
        cold_job_dir=str(cold),
        warm_job_dir=str(warm),
        cold_acceptance=str(cold_acceptance),
        warm_acceptance=str(warm_acceptance),
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
    assert result["warm"]["process_tree_peak_rss_kib"] == 4096
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


def test_final_validator_replays_qacct_bytes_not_acceptance_summary(tmp_path):
    args = setup_evidence(tmp_path)
    acceptance = json.loads(Path(args.cold_acceptance).read_text(encoding="utf-8"))
    qacct = Path(acceptance["evidence"]["qacct"]["path"])
    qacct.write_text(
        qacct.read_text(encoding="utf-8").replace("exit_status 0", "exit_status 137"),
        encoding="utf-8",
    )
    with pytest.raises(BenchmarkError, match="evidence bytes changed: qacct"):
        validate(args)


def test_final_validator_rejects_tampered_scheduler_summary(tmp_path):
    args = setup_evidence(tmp_path)
    acceptance = json.loads(Path(args.cold_acceptance).read_text(encoding="utf-8"))
    acceptance["scheduler"]["slots"] = 3
    write_json(args.cold_acceptance, acceptance)
    with pytest.raises(BenchmarkError, match="replayed source-byte validation"):
        validate(args)


def test_final_validator_rejects_removed_qacct_hash(tmp_path):
    args = setup_evidence(tmp_path)
    acceptance = json.loads(Path(args.cold_acceptance).read_text(encoding="utf-8"))
    acceptance["evidence"]["qacct"].pop("sha256")
    acceptance.pop("qacct_sha256")
    write_json(args.cold_acceptance, acceptance)
    with pytest.raises(BenchmarkError, match="evidence bytes changed: qacct"):
        validate(args)


def test_final_validator_rejects_missing_named_pool_worker_process(tmp_path):
    args = setup_evidence(tmp_path)
    tree_path = Path(args.warm_job_dir) / "process_tree_rss.json"
    process_tree = json.loads(tree_path.read_text(encoding="utf-8"))
    process_tree["all_named_pids_observed_as_descendants"] = False
    process_tree["identity_observation_count"] = 0
    write_json(tree_path, process_tree)
    wrapper_path = Path(args.warm_job_dir) / "wrapper.json"
    wrapper = json.loads(wrapper_path.read_text(encoding="utf-8"))
    wrapper["artifacts"]["process_tree_rss"]["sha256"] = sha256_file(tree_path)
    wrapper["process_tree_identity_observation_count"] = 0
    write_json(wrapper_path, wrapper)
    with pytest.raises(BenchmarkError, match="not all MATLAB PIDs"):
        validate(args)


def test_final_validator_rejects_eight_slot_seven_gib_wrapper_shape(tmp_path):
    args = setup_evidence(tmp_path)
    wrapper_path = Path(args.cold_job_dir) / "wrapper.json"
    wrapper = json.loads(wrapper_path.read_text(encoding="utf-8"))
    wrapper["requested_slots"] = wrapper["actual_slots"] = 8
    wrapper["mem_per_core_gib"] = 7
    wrapper["total_reserved_gib"] = 56
    write_json(wrapper_path, wrapper)
    with pytest.raises(BenchmarkError, match="wrapper resource policy changed"):
        validate(args)


def test_final_validator_rejects_tampered_worker_pid_artifact(tmp_path):
    args = setup_evidence(tmp_path)
    identity_path = Path(args.warm_job_dir) / "process_identity.json"
    identity = json.loads(identity_path.read_text(encoding="utf-8"))
    identity["worker_pids"][-1] = 9999
    write_json(identity_path, identity)
    with pytest.raises(BenchmarkError, match="process_identity artifact binding changed"):
        validate(args)

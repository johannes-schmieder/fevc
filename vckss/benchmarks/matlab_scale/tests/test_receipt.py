import argparse
import json
from pathlib import Path

from build_wrapper_receipt import build
from common import sha256_file
from conftest import write_json, write_process_evidence
from test_common import valid_case


def arguments(tmp_path, exit_status=0):
    case_path = tmp_path / "case.json"
    write_json(case_path, valid_case())
    output = tmp_path / "output"
    output.mkdir()
    aggregate = output / "aggregate.json"
    write_json(
        aggregate,
        {
            "status": "PASS",
            "failure_code": "NONE",
            "failure_message": "NONE",
            "pool_workers": 4,
            "process_identity_status": "PASS",
            "matlab_client_pid": 1001,
            "matlab_worker_pids": [1011, 1012, 1013, 1014],
            "import_seconds": 1.0,
            "input_validation_seconds": 2.0,
            "retained_validation_seconds": 3.0,
            "pool_startup_seconds": 4.0,
            "mex_setup_seconds": 5.0,
            "serialization_seconds": 0.1,
            "pool_teardown_seconds": 0.2,
        },
    )
    calls = output / "calls.csv"
    calls.write_text("role\ncold\n", encoding="utf-8")
    identity = tmp_path / "identity.json"
    write_json(identity, {"status": "PASS", "verification_seconds": 0.25})
    application = tmp_path / "application.txt"
    application.write_text("PASS\n", encoding="utf-8")
    time_report = tmp_path / "resources.txt"
    time_report.write_text(
        "User time (seconds): 12\n"
        "System time (seconds): 2\n"
        "Elapsed (wall clock) time (h:mm:ss or m:ss): 0:20\n"
        "Maximum resident set size (kbytes): 1024\n"
        f"Exit status: {exit_status}\n",
        encoding="utf-8",
    )
    marker = output / "wrapper.pass"
    marker.write_text("PASS\n", encoding="utf-8")
    process_identity, process_tree = write_process_evidence(
        tmp_path,
        mode="cold",
        label="scale4-well",
        case_sha256=sha256_file(case_path),
    )
    return argparse.Namespace(
        case=str(case_path),
        case_sha256=sha256_file(case_path),
        mode="cold",
        label="scale4-well",
        failure_stage="complete",
        exit_status=str(exit_status),
        job_id="12345",
        hostname="scc-test",
        sge_task_id="undefined",
        started_epoch_ns="1000000000",
        finished_epoch_ns="22000000000",
        staging_seconds="1.25",
        module_seconds="2.5",
        timeout_seconds="3600",
        timeout_basis="cz18-measured-v1",
        requested_slots="4",
        actual_slots="4",
        mem_per_core_gib="14",
        scheduler_hard_wall_seconds="4200",
        time_report=str(time_report),
        aggregate=str(aggregate),
        calls=str(calls),
        identity=str(identity),
        process_identity=str(process_identity),
        process_tree_rss=str(process_tree),
        expected_pool_workers="4",
        application_log=str(application),
        pass_marker=str(marker),
        output=str(tmp_path / "wrapper.json"),
    )


def test_success_receipt_preserves_process_resources_and_stages(tmp_path):
    args = arguments(tmp_path)
    assert build(args) == 0
    receipt = json.loads(Path(args.output).read_text(encoding="utf-8"))
    assert receipt["status"] == "PASS"
    assert receipt["wrapper_wall_seconds"] == 21.0
    assert receipt["process_wall_seconds"] == 20.0
    assert receipt["cpu_seconds"] == 14.0
    assert receipt["peak_rss_kib"] == 1024
    assert receipt["process_tree_peak_rss_kib"] == 4096
    assert receipt["process_tree_peak_process_count"] == 7
    assert receipt["matlab_client_pid"] == 1001
    assert receipt["matlab_worker_pids"] == [1011, 1012, 1013, 1014]
    assert receipt["process_tree_identity_observation_count"] == 17
    assert receipt["mex_setup_seconds"] == 5.0
    assert receipt["application_timeout_seconds"] == 3600
    assert receipt["timeout_basis"] == "cz18-measured-v1"
    assert receipt["requested_slots"] == receipt["actual_slots"] == 4
    assert receipt["mem_per_core_gib"] == 14
    assert receipt["total_reserved_gib"] == 56
    assert receipt["artifacts"]["application"]["sha256"]


def test_timeout_receipt_is_written_even_without_matlab_aggregate(tmp_path):
    args = arguments(tmp_path, exit_status=124)
    Path(args.aggregate).unlink()
    Path(args.pass_marker).unlink()
    args.failure_stage = "matlab_application"
    assert build(args) == 0
    receipt = json.loads(Path(args.output).read_text(encoding="utf-8"))
    assert receipt["status"] == "FAIL"
    assert receipt["timeout"] is True
    assert receipt["failure_code"] == "KSS_MATLAB_SCALE_TIMEOUT"
    assert receipt["failure_stage"] == "matlab_application"
    assert receipt["artifacts"]["application"]["bytes"] > 0


def test_success_fails_closed_without_process_tree_receipt(tmp_path):
    args = arguments(tmp_path)
    Path(args.process_tree_rss).unlink()
    assert build(args) == 2
    receipt = json.loads(Path(args.output).read_text(encoding="utf-8"))
    assert receipt["status"] == "FAIL"
    assert receipt["failure_code"] == "KSS_MATLAB_SCALE_PROCESS_TREE_RSS_REJECTED"


def test_success_fails_closed_without_matlab_process_identity(tmp_path):
    args = arguments(tmp_path)
    Path(args.process_identity).unlink()
    assert build(args) == 2
    receipt = json.loads(Path(args.output).read_text(encoding="utf-8"))
    assert receipt["status"] == "FAIL"
    assert receipt["failure_code"] == "KSS_MATLAB_SCALE_PROCESS_TREE_RSS_REJECTED"


def test_success_fails_closed_when_named_pool_worker_was_not_observed(tmp_path):
    args = arguments(tmp_path)
    process_tree = json.loads(Path(args.process_tree_rss).read_text(encoding="utf-8"))
    process_tree["all_named_pids_observed_as_descendants"] = False
    process_tree["identity_observation_count"] = 0
    write_json(args.process_tree_rss, process_tree)
    assert build(args) == 2
    receipt = json.loads(Path(args.output).read_text(encoding="utf-8"))
    assert receipt["status"] == "FAIL"
    assert receipt["failure_code"] == "KSS_MATLAB_SCALE_PROCESS_TREE_RSS_REJECTED"


def test_wrapper_rejects_consistent_eight_slot_seven_gib_resources(tmp_path):
    args = arguments(tmp_path)
    args.requested_slots = args.actual_slots = "8"
    args.mem_per_core_gib = "7"
    assert build(args) == 2
    receipt = json.loads(Path(args.output).read_text(encoding="utf-8"))
    assert receipt["status"] == "FAIL"
    assert receipt["failure_code"] == "KSS_MATLAB_SCALE_WRAPPER_FAILURE"

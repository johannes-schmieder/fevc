import argparse
import json
from pathlib import Path

from build_prepare_receipt import build
from common import sha256_file
from conftest import write_csv


def arguments(tmp_path, exit_status=0):
    input_csv = tmp_path / "input.csv"
    input_csv.write_text(
        "worker,firm,period,outcome\n1,1,1,0.1\n1,2,2,0.2\n2,1,1,0.3\n3,2,1,0.4\n3,2,2,0.5\n",
        encoding="utf-8",
    )
    retained_keys = tmp_path / "retained_keys.canonical.txt"
    retained_keys.write_bytes(b"1,1\n1,2\n2,1\n3,2\n")
    summary = tmp_path / "prepare.csv"
    write_csv(
        summary,
        [
            {
                "schema": "kss_matlab_scale_prepare_summary_v1",
                "status": "PASS",
                "label": "cz18-fixed",
                "scale": 1,
                "topology": "well",
                "sample_mode": "fixed_retained",
                "source_commit": "1" * 40,
                "bundle_sha256": "2" * 64,
                "source_input_sha256": "3" * 64,
                "input_rows": 5,
                "input_workers": 3,
                "input_firms": 2,
                "input_matches": 4,
                "rows": 5,
                "workers": 3,
                "firms": 2,
                "matches": 4,
                "connector_rows": 0,
                "copy_cut_conductance": "",
                "normalized_lambda2": "",
                "normalized_lambda_max": "",
                "normalized_condition_proxy": "",
                "minimum_weighted_degree": 1,
                "maximum_weighted_degree": 3,
                "load_seconds": 1.0,
                "normalization_seconds": 2.0,
                "fixture_seconds": 3.0,
                "export_seconds": 4.0,
                "total_seconds": 10.0,
                "requested_slots": 14,
                "actual_slots": 14,
                "requested_processors": 4,
                "actual_processors": 4,
                "stata_mp": 1,
                "stata_version": "19",
                "stata_flavor": "MP",
                "retained_key_contract": "SORTED_INTEGER_CSV_UTF8_LF_V1",
            }
        ],
    )
    marker = tmp_path / "prepare.stata.pass"
    marker.write_text(
        "KSS_MATLAB_SCALE_PREPARE_PASS cz18-fixed "
        + "1" * 40
        + " "
        + "2" * 64
        + " "
        + "3" * 64
        + "\n",
        encoding="utf-8",
    )
    application = tmp_path / "application.txt"
    application.write_text("PASS\n", encoding="utf-8")
    resources = tmp_path / "resources.txt"
    resources.write_text(
        "User time (seconds): 12\n"
        "System time (seconds): 2\n"
        "Elapsed (wall clock) time (h:mm:ss or m:ss): 0:20\n"
        "Maximum resident set size (kbytes): 2048\n"
        f"Exit status: {exit_status}\n",
        encoding="utf-8",
    )
    return argparse.Namespace(
        label="cz18-fixed",
        scale="1",
        topology="well",
        source_commit="1" * 40,
        bundle_sha256="2" * 64,
        source_input_sha256="3" * 64,
        prepared_input_sha256=sha256_file(input_csv),
        retained_key_sha256=sha256_file(retained_keys),
        failure_stage="complete",
        exit_status=str(exit_status),
        job_id="12345",
        hostname="compute.example",
        sge_task_id="undefined",
        requested_slots="14",
        actual_slots="14",
        mem_per_core_gib="4",
        stata_processors="4",
        timeout_seconds="3600",
        timeout_basis="cz18-measured-v1",
        scheduler_hard_wall_seconds="4200",
        started_epoch_ns="1000000000",
        finished_epoch_ns="22000000000",
        staging_seconds="1.25",
        module_seconds="2.5",
        promotion_seconds="3.75",
        time_report=str(resources),
        application_log=str(application),
        input_csv=str(input_csv),
        retained_keys=str(retained_keys),
        summary=str(summary),
        pass_marker=str(marker),
        output=str(tmp_path / "wrapper.json"),
    )


def test_preparation_receipt_binds_dimensions_keys_resources_and_timeout(tmp_path):
    args = arguments(tmp_path)
    assert build(args) == 0
    receipt = json.loads(Path(args.output).read_text(encoding="utf-8"))
    assert receipt["status"] == "PASS"
    assert receipt["dimensions"] == {
        "rows": 5,
        "workers": 3,
        "firms": 2,
        "matches": 4,
    }
    assert receipt["retained_key_sha256"]
    assert receipt["prepared_input_sha256"]
    assert receipt["requested_slots"] == receipt["actual_slots"] == 14
    assert receipt["total_reserved_gib"] == 56
    assert receipt["requested_stata_processors"] == 4
    assert receipt["application_timeout_seconds"] == 3600
    assert receipt["timeout_basis"] == "cz18-measured-v1"
    assert receipt["output_promotion_seconds"] == 3.75
    assert receipt["cpu_seconds"] == 14.0
    assert receipt["peak_rss_kib"] == 2048


def test_preparation_receipt_rejects_noncanonical_key_bytes(tmp_path):
    args = arguments(tmp_path)
    Path(args.retained_keys).write_bytes(b"01,1\n1,2\n2,1\n3,2\n")
    assert build(args) == 2
    receipt = json.loads(Path(args.output).read_text(encoding="utf-8"))
    assert receipt["status"] == "FAIL"
    assert receipt["failure_code"] == "KSS_MATLAB_SCALE_PREPARE_OUTPUT_REJECTED"
    assert "canonical integer CSV" in receipt["failure_message"]


def test_preparation_receipt_rejects_ring_at_scale_four(tmp_path):
    args = arguments(tmp_path)
    args.scale = "4"
    args.topology = "ring"
    assert build(args) == 2
    receipt = json.loads(Path(args.output).read_text(encoding="utf-8"))
    assert receipt["status"] == "FAIL"
    assert receipt["failure_code"] == "KSS_MATLAB_SCALE_PREPARE_OUTPUT_REJECTED"
    assert "scale/topology pair is unsupported" in receipt["failure_message"]


def test_preparation_receipt_recomputes_prepared_input_hash(tmp_path):
    args = arguments(tmp_path)
    Path(args.input_csv).write_text("tampered prepared rows\n", encoding="utf-8")
    assert build(args) == 2
    receipt = json.loads(Path(args.output).read_text(encoding="utf-8"))
    assert "prepared MATLAB input checksum" in receipt["failure_message"]


def test_preparation_receipt_requires_successful_gnu_time_report(tmp_path):
    args = arguments(tmp_path)
    resources = Path(args.time_report)
    resources.write_text(
        resources.read_text(encoding="utf-8").replace("Exit status: 0", "Exit status: 3"),
        encoding="utf-8",
    )
    assert build(args) == 2
    receipt = json.loads(Path(args.output).read_text(encoding="utf-8"))
    assert "GNU-time exit status is nonzero" in receipt["failure_message"]


def test_timeout_receipt_survives_missing_row_outputs(tmp_path):
    args = arguments(tmp_path, exit_status=124)
    Path(args.input_csv).unlink()
    Path(args.retained_keys).unlink()
    Path(args.summary).unlink()
    Path(args.pass_marker).unlink()
    args.failure_stage = "stata_preparation"
    assert build(args) == 0
    receipt = json.loads(Path(args.output).read_text(encoding="utf-8"))
    assert receipt["status"] == "FAIL"
    assert receipt["timeout"] is True
    assert receipt["failure_code"] == "KSS_MATLAB_SCALE_PREPARE_TIMEOUT"

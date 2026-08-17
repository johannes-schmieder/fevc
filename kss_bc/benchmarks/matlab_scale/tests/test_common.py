import json
from pathlib import Path

import pytest
from common import (
    BenchmarkError,
    parse_gnu_time,
    sha256_file,
    validate_case,
    validate_preparation_receipt,
    validate_reference_record,
)

PACKAGE = Path(__file__).resolve().parents[1]
CONTRACT_PATH = PACKAGE / "source_contract.json"
CONTRACT = json.loads(CONTRACT_PATH.read_text(encoding="utf-8"))


def reference_provenance():
    return {
        "producer": "kss_matlab_scale_reference_converter_v1",
        "experiment_id": "cz18_p200",
        "fixture": "cz18",
        "scale_factor": 1,
        "job_id": "12345",
        "option_contract": "KSS-SCALE-OPTIONS-V1",
        "frequency_var": "-",
        "target_var": "-",
        "deletion_var": "-",
        "deletion_mode": "match",
        "kss_admission_receipt_sha256": "a" * 64,
        "kss_certificate_sha256": "b" * 64,
        "kss_driver_sha256": "3" * 64,
        "kss_job_id_file_sha256": "c" * 64,
        "kss_node_receipt_sha256": "d" * 64,
        "kss_qacct_sha256": "e" * 64,
        "kss_reservation_sha256": "f" * 64,
        "kss_scheduler_request_sha256": "4" * 64,
        "kss_source_manifest_sha256": "0" * 64,
        "kss_summary_sha256": "1" * 64,
        "kss_validator_sha256": "2" * 64,
        "preparation_acceptance_sha256": "a" * 64,
        "preparation_receipt_sha256": "9" * 64,
        "prepared_input_sha256": "6" * 64,
        "retained_key_sha256": "8" * 64,
        "source_input_sha256": "b" * 64,
    }


def valid_case():
    return {
        "schema": "kss_matlab_scale_case_v2",
        "label": "scale4-well",
        "scale": 4,
        "topology": "well_connected",
        "sample_mode": "fixed",
        "seed": 8675309,
        "probes": 200,
        "warm_repetitions": 3,
        "source": {
            "source_commit": "1" * 40,
            "bundle_sha256": "2" * 64,
            "matlab_upstream_commit": CONTRACT["maintained_upstream_commit"],
            "matlab_runtime_tree_sha256": CONTRACT["runtime_tree"]["sha256"],
            "matlab_core_sha256": CONTRACT["core"]["sha256"],
            "matlab_cmg_entry_sha256": "3" * 64,
            "matlab_hierarchy_sha256": "4" * 64,
            "matlab_solver_sha256": "5" * 64,
            "benchmark_contract_sha256": sha256_file(CONTRACT_PATH),
        },
        "input": {
            "sha256": "6" * 64,
            "rows": 100,
            "workers": 10,
            "firms": 5,
            "matches": 20,
            "id_contract": CONTRACT["input_id_contract"],
            "order_contract": CONTRACT["input_order_contract"],
        },
        "preparation": {
            "receipt_sha256": "9" * 64,
            "acceptance_sha256": "a" * 64,
            "receipt_schema": "kss_matlab_scale_preparation_v1",
            "label": "scale4-well",
            "scale": 4,
            "topology": "well_connected",
            "source_commit": "1" * 40,
            "bundle_sha256": "2" * 64,
            "source_input_sha256": "b" * 64,
            "prepared_input_sha256": "6" * 64,
            "retained_key_sha256": "8" * 64,
            "dimensions": {"rows": 100, "workers": 10, "firms": 5, "matches": 20},
            "source_dimensions": {"rows": 25, "workers": 4, "firms": 2, "matches": 5},
        },
        "reference_sample": {
            "kind": "stata_kss_bc",
            "receipt_sha256": "7" * 64,
            "receipt_schema": "kss_matlab_scale_reference_v1",
            "label": "scale4-well",
            "source_commit": "1" * 40,
            "bundle_sha256": "2" * 64,
            "input_bindings": {"prepared_input_sha256": "6" * 64},
            "retained_key_sha256": "8" * 64,
            "rows": 100,
            "workers": 10,
            "firms": 5,
            "matches": 20,
            "estimator": {
                "algorithm": "jla",
                "deletion": "match",
                "controls": "none",
                "frequency_semantics": "literal_physical_rows_v1",
                "target_weight_semantics": "uniform_stored_rows_v1",
                "probes": 200,
                "seed": 8675309,
            },
            "plugin": {
                "worker": 1.0,
                "firm": 2.0,
                "covariance": 0.5,
                "total": 4.0,
            },
            "provenance": reference_provenance(),
        },
    }


@pytest.mark.parametrize(
    ("scale", "topology"),
    [(1, "well"), (2, "well_connected"), (2, "ring"), (4, "well_connected")],
)
def test_registered_fixed_preparation_case_is_accepted(scale, topology):
    case = valid_case()
    case["scale"] = scale
    case["topology"] = topology
    case["preparation"]["scale"] = scale
    case["preparation"]["topology"] = topology
    assert validate_case(case, CONTRACT) is case


@pytest.mark.parametrize(
    ("scale", "topology"),
    [(1, "ring"), (2, "well"), (4, "ring"), (4, "well"), (8, "well_connected")],
)
def test_unregistered_fixed_preparation_case_is_rejected(scale, topology):
    case = valid_case()
    case["scale"] = scale
    case["topology"] = topology
    case["preparation"]["scale"] = scale
    case["preparation"]["topology"] = topology
    with pytest.raises(BenchmarkError, match="scale/topology pair is unsupported"):
        validate_case(case, CONTRACT)


def test_fixed_sample_dimensions_are_bound():
    case = valid_case()
    case["reference_sample"]["rows"] += 1
    with pytest.raises(BenchmarkError, match="reference rows mismatch"):
        validate_case(case, CONTRACT)


def test_reference_plugin_identity_is_a_hard_gate():
    case = valid_case()
    case["reference_sample"]["plugin"]["total"] = 4.1
    with pytest.raises(BenchmarkError, match="plug-in accounting identity"):
        validate_case(case, CONTRACT)


def test_weak_or_unbound_source_metadata_is_rejected():
    case = valid_case()
    case["source"]["matlab_core_sha256"] = "0" * 64
    with pytest.raises(BenchmarkError, match="maintained core changed"):
        validate_case(case, CONTRACT)


def test_gnu_time_parser_records_wall_cpu_rss_and_exit(tmp_path):
    report = tmp_path / "resources.txt"
    report.write_text(
        "\tUser time (seconds): 12.25\n"
        "\tSystem time (seconds): 1.75\n"
        "\tElapsed (wall clock) time (h:mm:ss or m:ss): 1:02.50\n"
        "\tMaximum resident set size (kbytes): 2048000\n"
        "Command terminated by signal 15\n"
        "\tExit status: 0\n",
        encoding="utf-8",
    )
    parsed = parse_gnu_time(report)
    assert parsed["cpu_seconds"] == 14.0
    assert parsed["process_wall_seconds"] == 62.5
    assert parsed["peak_rss_kib"] == 2048000
    assert parsed["time_exit_status"] == 0
    assert parsed["termination_signal"] == 15


def preparation_receipt():
    return {
        "schema": "kss_matlab_scale_preparation_v1",
        "status": "PASS",
        "label": "scale4-well",
        "scale": 4,
        "topology": "well_connected",
        "sample_mode": "fixed_retained",
        "source_commit": "1" * 40,
        "bundle_sha256": "2" * 64,
        "source_input_sha256": "b" * 64,
        "prepared_input_sha256": "6" * 64,
        "retained_key_sha256": "8" * 64,
        "dimensions": {"rows": 100, "workers": 10, "firms": 5, "matches": 20},
        "source_dimensions": {"rows": 25, "workers": 4, "firms": 2, "matches": 5},
        "artifacts": {
            "input_csv": {"sha256": "6" * 64},
            "retained_keys": {"sha256": "8" * 64},
        },
    }


def reference_receipt():
    return {
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
    }


def test_preparation_receipt_artifact_binding_is_a_hard_gate():
    receipt = preparation_receipt()
    receipt["artifacts"]["input_csv"]["sha256"] = "0" * 64
    with pytest.raises(BenchmarkError, match="input artifact is not bound"):
        validate_preparation_receipt(receipt)


@pytest.mark.parametrize(
    ("field", "message"),
    [
        ("retained_key_sha256", "retained key differs"),
        ("prepared_input_sha256", "prepared_input_sha256 changed"),
        ("requested_probes", "probe count changed"),
        ("controls", "controls changed"),
    ],
)
def test_reference_receipt_must_self_bind_sample_input_and_options(field, message):
    preparation = validate_preparation_receipt(preparation_receipt())
    receipt = reference_receipt()
    if field == "requested_probes":
        receipt[field] = 40
    elif field.endswith("sha256"):
        receipt[field] = "0" * 64
    else:
        receipt[field] = "joint"
    with pytest.raises(BenchmarkError, match=message):
        validate_reference_record(receipt, preparation, probes=200, seed=8675309)


def test_unbound_reference_aggregate_is_rejected():
    preparation = validate_preparation_receipt(preparation_receipt())
    receipt = reference_receipt()
    receipt.pop("prepared_input_sha256")
    with pytest.raises(BenchmarkError, match="binds neither prepared nor source input"):
        validate_reference_record(receipt, preparation, probes=200, seed=8675309)

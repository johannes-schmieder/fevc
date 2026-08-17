import json
from pathlib import Path

import pytest
from common import BenchmarkError, parse_gnu_time, sha256_file, validate_case

PACKAGE = Path(__file__).resolve().parents[1]
CONTRACT_PATH = PACKAGE / "source_contract.json"
CONTRACT = json.loads(CONTRACT_PATH.read_text(encoding="utf-8"))


def valid_case():
    return {
        "schema": "kss_matlab_scale_case_v1",
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
        "reference_sample": {
            "kind": "stata_kss_bc",
            "receipt_sha256": "7" * 64,
            "retained_key_sha256": "8" * 64,
            "rows": 100,
            "workers": 10,
            "firms": 5,
            "matches": 20,
            "plugin": {
                "worker": 1.0,
                "firm": 2.0,
                "covariance": 0.5,
                "total": 4.0,
            },
        },
    }


@pytest.mark.parametrize("scale", [1, 2, 4, 8, 16])
@pytest.mark.parametrize("topology", ["well", "well_connected", "ring"])
def test_registered_scale_and_topology_are_accepted(scale, topology):
    case = valid_case()
    case["scale"] = scale
    case["topology"] = topology
    assert validate_case(case, CONTRACT) is case


def test_fixed_sample_dimensions_are_bound():
    case = valid_case()
    case["reference_sample"]["rows"] += 1
    with pytest.raises(BenchmarkError, match="fixed input/reference rows mismatch"):
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

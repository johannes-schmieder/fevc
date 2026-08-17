import json
from pathlib import Path

PACKAGE = Path(__file__).resolve().parents[1]


def text(name):
    return (PACKAGE / name).read_text(encoding="utf-8")


def test_cold_and_warm_entries_share_one_maintained_call_site():
    assert "matlab_scale_run('cold')" in text("matlab_scale_cold.m")
    assert "matlab_scale_run('warm')" in text("matlab_scale_warm.m")
    runtime = text("matlab_scale_run.m")
    assert runtime.count("leave_out_KSS(") == 1
    assert "record.estimator_call_count = 1" in runtime
    assert "for repetition = 1:binding.warm_repetitions" in runtime
    assert "record.warmup_call_count = 1" in runtime


def test_runtime_records_identity_without_corrected_equality_gate():
    runtime = text("matlab_scale_run.m")
    validator = text("validate.py")
    assert "IDENTITY_ONLY_NO_EQUALITY_GATE" in runtime
    assert "IDENTITY_ONLY_NO_EQUALITY_GATE" in validator
    assert "corrected_mreldif" not in runtime
    assert "corrected_mreldif" not in validator
    assert "CHECKSUM_BOUND_REFERENCE_NOT_MATLAB_OUTPUT" in runtime


def test_scc_wrapper_is_non_submitting_node_local_and_resource_bound():
    wrapper = text("run_matlab_scale.sge")
    assert "qsub" not in wrapper
    assert "#$ -pe omp 4" in wrapper
    assert "#$ -l mem_per_core=14G" in wrapper
    assert "TMPDIR/kss-matlab-scale" in wrapper
    assert "/usr/bin/time -v" in wrapper
    assert "/usr/bin/timeout" in wrapper
    assert "build_wrapper_receipt.py" in wrapper
    assert "make_case.py" not in wrapper


def test_contract_registers_all_scale_and_graph_cases():
    contract = json.loads(text("source_contract.json"))
    assert contract["supported_scales"] == [1, 2, 4, 8, 16]
    assert set(contract["supported_topologies"]) == {"well", "well_connected", "ring"}
    assert contract["required_probes"] == 200
    assert contract["required_pool_workers"] == 4
    assert contract["minimum_warm_repetitions"] >= 3

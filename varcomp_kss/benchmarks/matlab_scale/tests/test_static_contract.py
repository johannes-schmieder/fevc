import json
import re
import subprocess
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
    assert "monitor_process_tree.py" in wrapper
    assert "process_tree_rss.json" in wrapper
    assert "build_wrapper_receipt.py" in wrapper
    assert "make_case.py" not in wrapper


def test_fixed_sample_preparation_is_scalar_stata_and_separates_reservation():
    driver = text("prepare_fixed_sample.do")
    wrapper = text("run_prepare_fixed_sample.sge")
    assert "vckss_scale_fixture" in driver
    assert """inlist(`scale', 2, 4) & "`topology'" == "well_connected""" in driver
    assert """`scale' == 2 & "`topology'" == "ring""" in driver
    assert "retained_keys.canonical.txt" in driver
    assert "SORTED_INTEGER_CSV_UTF8_LF_V1" in driver
    assert "set processors `requested_processors'" in driver
    assert "qsub" not in wrapper
    assert "#$ -pe omp 14" in wrapper
    assert "#$ -l mem_per_core=4G" in wrapper
    assert '[[ "$KSS_MS_STATA_PROCESSORS" == 4 ]]' in wrapper
    assert '[[ "$KSS_MS_SCALE" == 4 && "$KSS_MS_TOPOLOGY" == well_connected ]]' in wrapper
    assert '[[ "${NSLOTS:-}" == "$KSS_MS_REQUESTED_SLOTS" ]]' in wrapper
    assert "TMPDIR/kss-matlab-scale" in wrapper
    assert "/usr/bin/time -v" in wrapper
    assert "/usr/bin/timeout" in wrapper
    assert "SGE_TASK_ID" in wrapper


def test_submitter_has_one_scalar_qsub_and_no_automatic_cascade():
    submitter = text("submit_matlab_scale.sh")
    assert len(re.findall(r"(?m)^raw_job_id=\$\(qsub ", submitter)) == 1
    assert " -t " not in submitter
    assert "hold_jid" not in submitter
    assert "automatic_cascade\\tdisabled" in submitter
    assert "one_scalar_job_no_array" in submitter
    assert "requested_slots=14" in submitter
    assert "mem_per_core_gib=4" in submitter
    assert "stata_processors=4" in submitter
    assert '[[ "$scale" == 4 && "$topology" == well_connected ]]' in submitter
    assert "requested_slots=4" in submitter
    assert "mem_per_core_gib=14" in submitter
    assert "scheduler_hard_wall_seconds=$(( timeout_seconds + 600 ))" in submitter
    assert 'sha256sum "$case_json"' in submitter
    assert 'sha256sum "$input_path"' in submitter
    assert 'sha256sum "$bundle_archive"' in submitter
    assert ".request.tsv" in submitter
    assert ".scheduler_request.txt" in submitter
    assert 'qsub -verify "${qsub_args[@]}"' in submitter
    assert 'qsub -terse "${qsub_args[@]}"' in submitter
    assert ".job_id" in submitter
    assert "verify_submission.py" in submitter
    assert "PREPARATION_RECEIPT" in submitter
    assert "PREPARATION_ACCEPTANCE" in submitter
    assert 'sample_mode=fixed' in submitter
    assert "job_id=$raw_job_id" in submitter
    assert "raw_job_id%%.*" not in submitter


def test_both_matlab_entry_points_have_valid_shell_syntax():
    for name in ("run_prepare_fixed_sample.sge", "submit_matlab_scale.sh"):
        subprocess.run(["bash", "-n", str(PACKAGE / name)], check=True)


def test_matlab_wrapper_records_measured_timeout_and_scheduler_request():
    wrapper = text("run_matlab_scale.sge")
    receipt = text("build_wrapper_receipt.py")
    assert "KSS_MS_TIMEOUT_BASIS" in wrapper
    assert "KSS_MS_SCHEDULER_HARD_WALL_SECONDS" in wrapper
    assert "KSS_MS_REQUESTED_SLOTS" in wrapper
    assert "KSS_MS_MEM_PER_CORE_GIB" in wrapper
    assert '"application_timeout_seconds"' in receipt
    assert '"timeout_basis"' in receipt
    assert '"scheduler_hard_wall_seconds"' in receipt
    assert '"process_tree_peak_rss_kib"' in receipt
    assert "--expected-pool-workers 4" in wrapper
    assert '"expected_pool_workers"' in receipt
    assert "validate_process_tree_record" in receipt


def test_matlab_records_named_client_and_worker_pid_identity():
    runtime = text("matlab_scale_run.m")
    monitor = text("monitor_process_tree.py")
    wrapper = text("run_matlab_scale.sge")
    assert "matlabProcessID" in runtime
    assert "spmdIndex" in runtime
    assert "kss_matlab_scale_process_identity_v1" in runtime
    assert "certified_identity_sample" in monitor
    assert "all_named_pids_observed_as_descendants" in monitor
    assert "--process-identity" in wrapper


def test_acceptance_consumers_replay_canonical_scheduler_source_bytes():
    acceptance = text("validate_scc_job.py")
    assert "expected_evidence_paths" in acceptance
    assert "revalidate_acceptance_sources" in acceptance
    assert '"request"' in acceptance
    assert '"submission"' in acceptance
    assert '"scheduler_request"' in acceptance
    assert '"job_id_file"' in acceptance
    assert '"qacct"' in acceptance
    assert '"category"' not in acceptance
    for consumer in ("make_case.py", "verify_submission.py", "validate.py"):
        assert "revalidate_acceptance_sources" in text(consumer)


def test_reference_converter_is_bundled_and_derives_kss_kind():
    converter = text("build_reference_receipt.py")
    case_builder = text("make_case.py")
    bundle_builder = (PACKAGE.parent / "build_scale_bundle.py").read_text(encoding="utf-8")
    allowlist = (PACKAGE.parent / "scale_bundle_allowlist.txt").read_text(
        encoding="utf-8"
    )
    assert '"kind": "stata_vckss"' in converter
    assert "REFERENCE_KSS_OPTIONS" in converter
    assert "validator.validate_run" in converter
    assert "--reference-kind" not in case_builder
    assert "matlab_scale/build_reference_receipt.py" in bundle_builder
    assert "varcomp_kss/benchmarks/matlab_scale/build_reference_receipt.py\n" in allowlist


def test_wrappers_and_accounting_validator_fail_closed_on_arrays():
    for name in ("run_matlab_scale.sge", "run_prepare_fixed_sample.sge"):
        wrapper = text(name)
        assert 'case "${SGE_TASK_ID:-undefined}"' in wrapper
        assert "array tasks are prohibited" in wrapper
    validator = text("validate_scc_job.py")
    assert 'qacct["taskid"] == "undefined"' in validator
    assert "qacct reports an array task" in validator


def test_accounting_validator_hard_gates_stage_resource_shapes():
    validator = text("validate_scc_job.py")
    assert '"prepare": {"slots": 14, "mem_per_core_gib": 4' in validator
    assert '"cold": {"slots": 4, "mem_per_core_gib": 14' in validator
    assert '"warm": {"slots": 4, "mem_per_core_gib": 14' in validator
    assert "total_reserved_gib == requested_slots * mem_per_core_gib == 56" in validator


def test_contract_registers_all_scale_and_graph_cases():
    contract = json.loads(text("source_contract.json"))
    assert contract["supported_scales"] == [1, 2, 4, 8, 16]
    assert set(contract["supported_topologies"]) == {"well", "well_connected", "ring"}
    assert contract["required_probes"] == 200
    assert contract["required_pool_workers"] == 4
    assert contract["minimum_warm_repetitions"] >= 3
    assert contract["case_schema"] == "kss_matlab_scale_case_v2"
    assert contract["prepared_submission_modes"] == ["fixed"]
    assert contract["fixed_preparation_cases"] == [
        {"scale": 1, "topology": "well"},
        {"scale": 2, "topology": "well_connected"},
        {"scale": 2, "topology": "ring"},
        {"scale": 4, "topology": "well_connected"},
    ]

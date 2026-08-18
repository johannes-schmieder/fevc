from pathlib import Path

import pytest
from common import BenchmarkError, sha256_file
from validate_scc_job import (
    parse_key_value_receipt,
    parse_qacct,
    parse_scheduler_request,
    validate_scheduler,
    validate_scheduler_request,
    validate_stage_resource_policy,
)


def write_receipt(path, values):
    with Path(path).open("w", encoding="utf-8") as handle:
        handle.write("key\tvalue\n")
        for key, value in values.items():
            handle.write(f"{key}\t{value}\n")


def scheduler_fixture(tmp_path):
    job_dir = "/projectnb/welfgr/varcomp-kss/runs/test/matlab_scale/scale4-well/cold"
    source_dir = "/projectnb/welfgr/varcomp-kss/bundles/" + "2" * 64 + "/source"
    script_path = source_dir + "/varcomp_kss/benchmarks/matlab_scale/run_matlab_scale.sge"
    stdout_path = (
        "/projectnb/welfgr/varcomp-kss/runs/test/logs/"
        "matlab_scale_scale4-well_cold.stdout.txt"
    )
    scheduler_request_path = tmp_path / "scheduler_request.txt"
    request_values = {
        "receipt_version": "KSS-MATLAB-SCALE-SUBMISSION-REQUEST-V1",
        "stage": "cold",
        "label": "scale4-well",
        "source_commit": "1" * 40,
        "bundle_sha256": "2" * 64,
        "input_path": (
            "/projectnb/welfgr/varcomp-kss/runs/test/matlab_scale/"
            "scale4-well/prepare/input.csv"
        ),
        "input_sha256": "3" * 64,
        "case_sha256": "4" * 64,
        "contract_sha256": "NOT_APPLICABLE",
        "sample_mode": "fixed",
        "preparation_receipt_sha256": "5" * 64,
        "preparation_acceptance_sha256": "6" * 64,
        "case_gate_sha256": "7" * 64,
        "scale": "NOT_APPLICABLE",
        "topology": "NOT_APPLICABLE",
        "matlab_root": "NOT_APPLICABLE",
        "requested_slots": "4",
        "mem_per_core_gib": "14",
        "total_reserved_gib": "56",
        "stata_processors": "NOT_APPLICABLE",
        "application_timeout_seconds": "3600",
        "timeout_basis": "measured-v1",
        "scheduler_hard_wall_seconds": "4200",
        "scheduler_hard_wall_hms": "01:10:00",
        "execution_shape": "one_scalar_job_no_array",
        "project": "welfgr",
        "parallel_environment": "omp",
        "job_name": "kms_cold",
        "job_dir": job_dir,
        "source_dir": source_dir,
        "script_path": script_path,
        "stdout_path": stdout_path,
        "scheduler_request_path": str(scheduler_request_path),
        "qacct_path": "/projectnb/welfgr/varcomp-kss/runs/test/qacct/matlab_scale_scale4-well_cold.txt",
        "automatic_cascade": "disabled",
    }
    request_path = tmp_path / "request.tsv"
    write_receipt(request_path, request_values)
    scheduler_environment = {
        "KSS_MS_RUN_DIR": "/projectnb/welfgr/varcomp-kss/runs/test",
        "KSS_MS_SOURCE_DIR": source_dir,
        "KSS_MS_SOURCE_COMMIT": "1" * 40,
        "KSS_MS_BUNDLE_SHA256": "2" * 64,
        "KSS_MS_LABEL": "scale4-well",
        "KSS_MS_TIMEOUT_SECONDS": "3600",
        "KSS_MS_TIMEOUT_BASIS": "measured-v1",
        "KSS_MS_REQUESTED_SLOTS": "4",
        "KSS_MS_MEM_PER_CORE_GIB": "14",
        "KSS_MS_SCHEDULER_HARD_WALL_SECONDS": "4200",
        "KSS_MS_JOB_DIR": job_dir,
        "KSS_MS_MODE": "cold",
        "KSS_MS_CASE_JSON": (
            "/projectnb/welfgr/varcomp-kss/runs/test/matlab_scale/scale4-well/case.json"
        ),
        "KSS_MS_CASE_SHA256": "4" * 64,
        "KSS_MS_INPUT_CSV": request_values["input_path"],
        "KSS_MS_INPUT_SHA256": "3" * 64,
        "KSS_MS_MATLAB_ROOT": "NOT_APPLICABLE",
        "KSS_MS_CONTRACT_SHA256": "NOT_APPLICABLE",
        "KSS_MS_PREPARATION_RECEIPT": (
            "/projectnb/welfgr/varcomp-kss/runs/test/matlab_scale/"
            "scale4-well/prepare/wrapper.json"
        ),
        "KSS_MS_PREPARATION_RECEIPT_SHA256": "5" * 64,
        "KSS_MS_PREPARATION_ACCEPTANCE": (
            "/projectnb/welfgr/varcomp-kss/runs/test/matlab_scale/"
            "scale4-well/prepare/acceptance.json"
        ),
        "KSS_MS_PREPARATION_ACCEPTANCE_SHA256": "6" * 64,
    }
    env_text = ",".join(
        ["PATH=/usr/bin:/bin"]
        + [f"{name}={value}" for name, value in scheduler_environment.items()]
    )
    scheduler_request_path.write_text(
        "job_number:                 unassigned\n"
        "merge:                      y\n"
        "hard resource_list:         mem_per_core=14G,h_rt=01:10:00\n"
        "job_name:                   kms_cold\n"
        f"stdout_path_list:           NONE:NONE:{stdout_path}\n"
        "verify:                     -verify\n"
        f"env_list:                   {env_text}\n"
        f"script_file:                {script_path}\n"
        "parallel environment:  omp range: 4\n"
        "project:                    welfgr\n",
        encoding="utf-8",
    )
    submission_values = {
        **request_values,
        "receipt_version": "KSS-MATLAB-SCALE-SUBMISSION-V1",
        "request_sha256": sha256_file(request_path),
        "scheduler_request_sha256": sha256_file(scheduler_request_path),
        "job_id": "12345",
    }
    submission_path = tmp_path / "submission.tsv"
    write_receipt(submission_path, submission_values)
    job_id_path = tmp_path / "job_id"
    job_id_path.write_text("12345\n", encoding="utf-8")
    qacct_path = tmp_path / "qacct.txt"
    qacct_path.write_text(
        "==============================================================\n"
        "jobnumber 12345\n"
        "taskid undefined\n"
        "jobname kms_cold\n"
        "project welfgr\n"
        "hostname scc-test.scc.bu.edu\n"
        "qname local.q\n"
        "granted_pe omp\n"
        "slots 4\n"
        "failed 0\n"
        "exit_status 0\n"
        "ru_wallclock 120\n"
        "cpu 360\n"
        "maxvmem 40G\n",
        encoding="utf-8",
    )
    request = parse_key_value_receipt(request_path)
    request["_path"] = str(request_path)
    submission = parse_key_value_receipt(submission_path)
    wrapper = {
        "job_id": "12345",
        "sge_task_id": "undefined",
        "hostname": "scc-test",
        "requested_slots": 4,
        "actual_slots": 4,
        "mem_per_core_gib": 14,
        "scheduler_hard_wall_seconds": 4200,
        "wrapper_wall_seconds": 119.0,
        "process_wall_seconds": 110.0,
        "cpu_seconds": 350.0,
        "peak_rss_kib": 20_000_000,
        "time_exit_status": 0,
        "process_tree_status": "PASS",
        "process_identity_status": "PASS",
        "process_identity_sha256": "8" * 64,
        "matlab_client_pid": 1001,
        "matlab_worker_pids": [1011, 1012, 1013, 1014],
        "expected_pool_workers": 4,
        "process_tree_peak_rss_kib": 30_000_000,
        "process_tree_peak_process_count": 6,
        "process_tree_identity_sha256": "8" * 64,
        "process_tree_identity_observation_count": 20,
        "process_tree_identity_peak_rss_kib": 29_000_000,
        "process_tree_identity_peak_process_count": 6,
    }
    return request, submission, scheduler_request_path, job_id_path, qacct_path, wrapper


def replace_qacct(path, old, new):
    text = Path(path).read_text(encoding="utf-8")
    Path(path).write_text(text.replace(old, new), encoding="utf-8")


def test_scheduler_acceptance_binds_job_resources_and_process_tree(tmp_path):
    request, submission, scheduler_request, job_id, qacct, wrapper = scheduler_fixture(
        tmp_path
    )
    result = validate_scheduler(
        request, submission, scheduler_request, job_id, qacct, wrapper
    )
    assert result["job_id"] == "12345"
    assert result["slots"] == 4
    assert result["maxvmem_bytes"] == 40 * 1024**3


def test_qacct_rejects_array_task_even_when_base_job_id_matches(tmp_path):
    request, submission, scheduler_request, job_id, qacct, wrapper = scheduler_fixture(
        tmp_path
    )
    replace_qacct(qacct, "taskid undefined", "taskid 7")
    with pytest.raises(BenchmarkError, match="array task"):
        validate_scheduler(request, submission, scheduler_request, job_id, qacct, wrapper)


def test_qacct_rejects_job_id_mismatch(tmp_path):
    request, submission, scheduler_request, job_id, qacct, wrapper = scheduler_fixture(
        tmp_path
    )
    replace_qacct(qacct, "jobnumber 12345", "jobnumber 12346")
    with pytest.raises(BenchmarkError, match="job number mismatch"):
        validate_scheduler(request, submission, scheduler_request, job_id, qacct, wrapper)


@pytest.mark.parametrize(
    ("old", "new", "message"),
    [
        ("slots 4", "slots 3", "slot count mismatch"),
        ("maxvmem 40G", "maxvmem 57G", "maxvmem"),
        ("cpu 360", "cpu 900", "CPU is invalid"),
    ],
)
def test_qacct_resource_fields_are_hard_gates(tmp_path, old, new, message):
    request, submission, scheduler_request, job_id, qacct, wrapper = scheduler_fixture(
        tmp_path
    )
    replace_qacct(qacct, old, new)
    with pytest.raises(BenchmarkError, match=message):
        validate_scheduler(request, submission, scheduler_request, job_id, qacct, wrapper)


@pytest.mark.parametrize(
    ("old", "new", "message"),
    [
        ("h_rt=01:10:00", "h_rt=01:11:00", "hard resource list changed"),
        ("mem_per_core=14G", "mem_per_core=13G", "hard resource list changed"),
        ("omp range: 4", "omp range: 8", "parallel environment or range"),
        ("project:                    welfgr", "project:                    other", "project"),
        (
            "matlab_scale_scale4-well_cold.stdout.txt",
            "other.stdout.txt",
            "stdout path",
        ),
        ("run_matlab_scale.sge", "other.sge", "script path"),
        ("KSS_MS_INPUT_CSV=/projectnb/", "KSS_MS_INPUT_CSV=/wrong/", "KSS_MS_INPUT_CSV"),
    ],
)
def test_qsub_verify_resource_fields_are_hard_gates(tmp_path, old, new, message):
    request, submission, scheduler_request, job_id, qacct, wrapper = scheduler_fixture(
        tmp_path
    )
    replace_qacct(scheduler_request, old, new)
    submission["scheduler_request_sha256"] = sha256_file(scheduler_request)
    with pytest.raises(BenchmarkError, match=message):
        validate_scheduler(request, submission, scheduler_request, job_id, qacct, wrapper)


def test_real_bu_qsub_verify_spelling_is_parsed_without_qacct_category(tmp_path):
    request, _, scheduler_request, _, qacct, _ = scheduler_fixture(tmp_path)
    parsed = parse_scheduler_request(scheduler_request)
    assert parsed["job_number"] == "unassigned"
    assert parsed["stdout_path_list"].startswith("NONE:NONE:/projectnb/")
    validate_scheduler_request(scheduler_request, request)
    assert "category" not in parse_qacct(qacct)


def test_qsub_verify_parser_ignores_embedded_script_text(tmp_path):
    request, _, scheduler_request, _, _, _ = scheduler_fixture(tmp_path)
    with scheduler_request.open("a", encoding="utf-8") as handle:
        handle.write(
            "script_ptr:\n"
            ": \"${KSS_MS_RUN_DIR:?}\"\n"
            ": \"${KSS_MS_SOURCE_DIR:?}\"\n"
            "case \"${SGE_TASK_ID:-undefined}\" in\n"
        )
    validate_scheduler_request(scheduler_request, request)


def test_qacct_missing_cpu_is_rejected(tmp_path):
    _, _, _, _, qacct, _ = scheduler_fixture(tmp_path)
    replace_qacct(qacct, "cpu 360\n", "")
    with pytest.raises(BenchmarkError, match="qacct missing fields: cpu"):
        parse_qacct(qacct)


def test_scheduler_rejects_missing_named_pool_worker_processes(tmp_path):
    request, submission, scheduler_request, job_id, qacct, wrapper = scheduler_fixture(
        tmp_path
    )
    wrapper["process_tree_identity_observation_count"] = 0
    with pytest.raises(BenchmarkError, match="named-process observation count"):
        validate_scheduler(request, submission, scheduler_request, job_id, qacct, wrapper)


def test_scheduler_rejects_consistent_eight_slot_seven_gib_tampering(tmp_path):
    request, submission, scheduler_request, job_id, qacct, wrapper = scheduler_fixture(
        tmp_path
    )
    for record in (request, submission):
        record["requested_slots"] = "8"
        record["mem_per_core_gib"] = "7"
        record["total_reserved_gib"] = "56"
    request_path = Path(request["_path"])
    write_receipt(request_path, {key: value for key, value in request.items() if key != "_path"})
    submission["request_sha256"] = sha256_file(request_path)
    replace_qacct(qacct, "slots 4", "slots 8")
    replace_qacct(scheduler_request, "omp range: 4", "omp range: 8")
    replace_qacct(scheduler_request, "mem_per_core=14G", "mem_per_core=7G")
    replace_qacct(
        scheduler_request,
        "KSS_MS_REQUESTED_SLOTS=4",
        "KSS_MS_REQUESTED_SLOTS=8",
    )
    replace_qacct(
        scheduler_request,
        "KSS_MS_MEM_PER_CORE_GIB=14",
        "KSS_MS_MEM_PER_CORE_GIB=7",
    )
    submission["scheduler_request_sha256"] = sha256_file(scheduler_request)
    wrapper["requested_slots"] = wrapper["actual_slots"] = 8
    wrapper["mem_per_core_gib"] = 7
    with pytest.raises(BenchmarkError, match="cold scheduler resource policy changed"):
        validate_scheduler(request, submission, scheduler_request, job_id, qacct, wrapper)


def test_prepare_resource_policy_requires_four_stata_processors():
    request = {
        "stage": "prepare",
        "requested_slots": "14",
        "mem_per_core_gib": "4",
        "total_reserved_gib": "56",
        "stata_processors": "4",
    }
    assert validate_stage_resource_policy(request) == (14, 4, 56)
    request["stata_processors"] = "3"
    with pytest.raises(BenchmarkError, match="prepare scheduler resource policy changed"):
        validate_stage_resource_policy(request)


def test_prepare_resource_policy_rejects_consistent_eight_by_seven_shape():
    request = {
        "stage": "prepare",
        "requested_slots": "8",
        "mem_per_core_gib": "7",
        "total_reserved_gib": "56",
        "stata_processors": "4",
    }
    with pytest.raises(BenchmarkError, match="prepare scheduler resource policy changed"):
        validate_stage_resource_policy(request)

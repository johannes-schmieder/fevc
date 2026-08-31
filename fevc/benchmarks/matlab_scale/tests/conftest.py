import csv
import json
import sys
from argparse import Namespace
from pathlib import Path

PACKAGE = Path(__file__).resolve().parents[1]
if str(PACKAGE) not in sys.path:
    sys.path.insert(0, str(PACKAGE))


def write_json(path, value):
    Path(path).write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def write_csv(path, rows):
    path = Path(path)
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)


def write_kv(path, values):
    path = Path(path)
    with path.open("w", encoding="utf-8") as handle:
        handle.write("key\tvalue\n")
        for key, value in values.items():
            handle.write(f"{key}\t{value}\n")


def write_process_evidence(job_dir, *, mode, label, case_sha256):
    """Write mutually bound MATLAB PID identity and monitor fixtures."""
    from common import sha256_file

    job_dir = Path(job_dir)
    identity_path = job_dir / "process_identity.json"
    identity = {
        "schema": "kss_matlab_scale_process_identity_v1",
        "status": "PASS",
        "pid_api": "matlabProcessID_R2025a",
        "mode": mode,
        "label": label,
        "case_sha256": case_sha256,
        "expected_pool_workers": 4,
        "client_pid": 1001,
        "worker_indices": [1, 2, 3, 4],
        "worker_pids": [1011, 1012, 1013, 1014],
    }
    write_json(identity_path, identity)
    identity_sha = sha256_file(identity_path)
    tree_path = job_dir / "process_tree_rss.json"
    tree = {
        "schema": "kss_matlab_scale_process_tree_rss_v1",
        "status": "PASS",
        "failure_code": "NONE",
        "failure_message": "NONE",
        "root_pid": 999,
        "expected_pool_workers": 4,
        "minimum_expected_process_count": 5,
        "sample_interval_seconds": 0.25,
        "sample_count": 20,
        "peak_rss_kib": 4096,
        "peak_rss_process_count": 7,
        "peak_process_count": 7,
        "process_identity_path": str(identity_path.absolute()),
        "process_identity_sha256": identity_sha,
        "client_pid": identity["client_pid"],
        "worker_pids": identity["worker_pids"],
        "named_pid_count": 5,
        "identity_observation_count": 17,
        "identity_peak_rss_kib": 4000,
        "identity_peak_process_count": 7,
        "all_named_pids_observed_as_descendants": True,
        "started_utc": "2026-08-17T12:00:00+00:00",
        "finished_utc": "2026-08-17T12:01:00+00:00",
    }
    write_json(tree_path, tree)
    return identity_path, tree_path


def write_scc_acceptance(
    run_dir,
    *,
    stage,
    label,
    source_commit,
    bundle_sha256,
    input_sha256,
    job_dir,
    case_path=None,
    case_sha256="NOT_APPLICABLE",
    contract_path=None,
    preparation_receipt_sha256="NOT_APPLICABLE",
    preparation_acceptance_sha256="NOT_APPLICABLE",
    case_gate_sha256="NOT_APPLICABLE",
):
    """Write canonical scalar scheduler bytes and the replayed acceptance."""
    from common import sha256_file
    from validate_scc_job import build_acceptance

    run_dir = Path(run_dir).absolute()
    job_dir = Path(job_dir).absolute()
    wrapper = json.loads((job_dir / "wrapper.json").read_text(encoding="utf-8"))
    submissions = run_dir / "submissions"
    qacct_dir = run_dir / "qacct"
    submissions.mkdir(parents=True, exist_ok=True)
    qacct_dir.mkdir(parents=True, exist_ok=True)
    record = f"matlab_scale_{label}_{stage}"
    request_path = submissions / f"{record}.request.tsv"
    submission_path = submissions / f"{record}.tsv"
    scheduler_request_path = submissions / f"{record}.scheduler_request.txt"
    job_id_path = submissions / f"{record}.job_id"
    qacct_path = qacct_dir / f"{record}.txt"
    slots = int(wrapper["requested_slots"])
    mem_per_core = int(wrapper["mem_per_core_gib"])
    hard_wall = int(wrapper["scheduler_hard_wall_seconds"])
    hours, remainder = divmod(hard_wall, 3600)
    minutes, seconds = divmod(remainder, 60)
    hard_wall_hms = f"{hours:02d}:{minutes:02d}:{seconds:02d}"
    source_dir = run_dir / "bundle-source"
    script_name = (
        "run_prepare_fixed_sample.sge" if stage == "prepare" else "run_matlab_scale.sge"
    )
    script_path = source_dir / "fevc" / "benchmarks" / "matlab_scale" / script_name
    stdout_path = run_dir / "logs" / f"{record}.stdout.txt"
    request = {
        "receipt_version": "KSS-MATLAB-SCALE-SUBMISSION-REQUEST-V1",
        "stage": stage,
        "label": label,
        "source_commit": source_commit,
        "bundle_sha256": bundle_sha256,
        "input_path": str(job_dir / "input.csv"),
        "input_sha256": input_sha256,
        "case_sha256": case_sha256,
        "contract_sha256": "NOT_APPLICABLE",
        "sample_mode": "fixed",
        "preparation_receipt_sha256": preparation_receipt_sha256,
        "preparation_acceptance_sha256": preparation_acceptance_sha256,
        "case_gate_sha256": case_gate_sha256,
        "scale": str(wrapper.get("scale", "NOT_APPLICABLE")),
        "topology": str(wrapper.get("topology", "NOT_APPLICABLE")),
        "matlab_root": "NOT_APPLICABLE",
        "requested_slots": str(slots),
        "mem_per_core_gib": str(mem_per_core),
        "total_reserved_gib": str(slots * mem_per_core),
        "stata_processors": (
            str(wrapper.get("requested_stata_processors", "NOT_APPLICABLE"))
        ),
        "application_timeout_seconds": str(wrapper["application_timeout_seconds"]),
        "timeout_basis": wrapper["timeout_basis"],
        "scheduler_hard_wall_seconds": str(hard_wall),
        "scheduler_hard_wall_hms": hard_wall_hms,
        "execution_shape": "one_scalar_job_no_array",
        "project": "welfgr",
        "parallel_environment": "omp",
        "job_name": f"kms_{stage}",
        "job_dir": str(job_dir),
        "source_dir": str(source_dir),
        "script_path": str(script_path),
        "stdout_path": str(stdout_path),
        "scheduler_request_path": str(scheduler_request_path),
        "qacct_path": str(qacct_path),
        "automatic_cascade": "disabled",
    }
    write_kv(request_path, request)
    scheduler_environment = {
        "KSS_MS_RUN_DIR": str(run_dir),
        "KSS_MS_SOURCE_DIR": str(source_dir),
        "KSS_MS_SOURCE_COMMIT": source_commit,
        "KSS_MS_BUNDLE_SHA256": bundle_sha256,
        "KSS_MS_LABEL": label,
        "KSS_MS_TIMEOUT_SECONDS": request["application_timeout_seconds"],
        "KSS_MS_TIMEOUT_BASIS": request["timeout_basis"],
        "KSS_MS_REQUESTED_SLOTS": request["requested_slots"],
        "KSS_MS_MEM_PER_CORE_GIB": request["mem_per_core_gib"],
        "KSS_MS_SCHEDULER_HARD_WALL_SECONDS": request[
            "scheduler_hard_wall_seconds"
        ],
    }
    if stage == "prepare":
        scheduler_environment.update(
            {
                "KSS_MS_PREP_JOB_DIR": str(job_dir),
                "KSS_MS_SCALE": request["scale"],
                "KSS_MS_TOPOLOGY": request["topology"],
                "KSS_MS_SOURCE_INPUT_DTA": request["input_path"],
                "KSS_MS_SOURCE_INPUT_SHA256": input_sha256,
                "KSS_MS_STATA_PROCESSORS": request["stata_processors"],
            }
        )
    else:
        preparation_dir = job_dir.parent / "prepare"
        scheduler_environment.update(
            {
                "KSS_MS_JOB_DIR": str(job_dir),
                "KSS_MS_MODE": stage,
                "KSS_MS_CASE_JSON": str(job_dir.parent / "case.json"),
                "KSS_MS_CASE_SHA256": case_sha256,
                "KSS_MS_INPUT_CSV": request["input_path"],
                "KSS_MS_INPUT_SHA256": input_sha256,
                "KSS_MS_MATLAB_ROOT": request["matlab_root"],
                "KSS_MS_CONTRACT_SHA256": request["contract_sha256"],
                "KSS_MS_PREPARATION_RECEIPT": str(preparation_dir / "wrapper.json"),
                "KSS_MS_PREPARATION_RECEIPT_SHA256": preparation_receipt_sha256,
                "KSS_MS_PREPARATION_ACCEPTANCE": str(
                    preparation_dir / "acceptance.json"
                ),
                "KSS_MS_PREPARATION_ACCEPTANCE_SHA256": preparation_acceptance_sha256,
            }
        )
    env_text = ",".join(
        ["PATH=/usr/bin:/bin"]
        + [f"{name}={value}" for name, value in scheduler_environment.items()]
    )
    scheduler_request_path.write_text(
        "job_number:                 unassigned\n"
        "merge:                      y\n"
        f"hard resource_list:         mem_per_core={mem_per_core}G,h_rt={hard_wall_hms}\n"
        f"job_name:                   kms_{stage}\n"
        f"stdout_path_list:           NONE:NONE:{stdout_path}\n"
        "verify:                     -verify\n"
        f"env_list:                   {env_text}\n"
        f"script_file:                {script_path}\n"
        f"parallel environment:  omp range: {slots}\n"
        "project:                    welfgr\n",
        encoding="utf-8",
    )
    job_id = str(wrapper["job_id"])
    submission = {
        **request,
        "receipt_version": "KSS-MATLAB-SCALE-SUBMISSION-V1",
        "job_id": job_id,
        "submitted_utc": "2026-08-17T12:00:00Z",
        "request_sha256": sha256_file(request_path),
        "scheduler_request_sha256": sha256_file(scheduler_request_path),
    }
    write_kv(submission_path, submission)
    job_id_path.write_text(job_id + "\n", encoding="utf-8")
    qacct_path.write_text(
        "==============================================================\n"
        f"jobnumber {job_id}\n"
        "taskid undefined\n"
        f"jobname kms_{stage}\n"
        "project welfgr\n"
        f"hostname {wrapper['hostname']}.scc.bu.edu\n"
        "qname local.q\n"
        "granted_pe omp\n"
        f"slots {slots}\n"
        "failed 0\n"
        "exit_status 0\n"
        "ru_wallclock 120\n"
        "cpu 160\n"
        "maxvmem 1G\n",
        encoding="utf-8",
    )
    args = Namespace(
        stage=stage,
        request=str(request_path),
        submission=str(submission_path),
        scheduler_request=str(scheduler_request_path),
        job_id_file=str(job_id_path),
        qacct=str(qacct_path),
        job_dir=str(job_dir),
        case=str(case_path) if case_path is not None else None,
        case_sha256=case_sha256 if case_path is not None else None,
        contract=str(contract_path) if contract_path is not None else None,
    )
    acceptance = build_acceptance(args)
    acceptance_path = job_dir / "acceptance.json"
    write_json(acceptance_path, acceptance)
    return acceptance_path

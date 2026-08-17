#!/usr/bin/env python3
"""Validate scheduler, application, and output layers for one scalar SCC job."""

import argparse
import math
import re
import sys
from pathlib import Path
from types import SimpleNamespace

from build_prepare_receipt import validate_success as validate_prepare_success
from common import (
    ACCEPTANCE_SCHEMA,
    BenchmarkError,
    atomic_write_json,
    finite,
    hash_value,
    integer,
    load_json,
    require,
    sha256_file,
    validate_case,
    validate_preparation_receipt,
)

JOB_ID = re.compile(r"^[0-9]+$")
GIB = 1024**3
EXPECTED_POOL_WORKERS = 4
MINIMUM_PROCESS_COUNT = EXPECTED_POOL_WORKERS + 1
STAGE_RESOURCE_POLICY = {
    "prepare": {"slots": 14, "mem_per_core_gib": 4, "stata_processors": "4"},
    "cold": {"slots": 4, "mem_per_core_gib": 14, "stata_processors": "NOT_APPLICABLE"},
    "warm": {"slots": 4, "mem_per_core_gib": 14, "stata_processors": "NOT_APPLICABLE"},
}


def parse_key_value_receipt(path):
    path = Path(path)
    require(path.is_file(), f"missing key-value receipt: {path}")
    rows = path.read_text(encoding="utf-8").splitlines()
    require(rows and rows[0] == "key\tvalue", f"malformed receipt header: {path}")
    result = {}
    for line in rows[1:]:
        fields = line.split("\t")
        require(
            len(fields) == 2 and fields[0] and fields[0] not in result,
            f"malformed or duplicate receipt field: {path}",
        )
        result[fields[0]] = fields[1]
    return result


def parse_duration(value):
    value = str(value).strip()
    if ":" not in value:
        result = finite(value, "qacct duration")
    else:
        fields = value.split(":")
        require(len(fields) in (2, 3), f"invalid qacct duration: {value}")
        numbers = [finite(item, "qacct duration component") for item in fields]
        if len(numbers) == 3:
            result = 3600 * numbers[0] + 60 * numbers[1] + numbers[2]
        else:
            result = 60 * numbers[0] + numbers[1]
    require(math.isfinite(result) and result >= 0, f"invalid qacct duration: {value}")
    return result


def parse_memory(value):
    match = re.fullmatch(r"([0-9]+(?:\.[0-9]+)?)([KMGTP]?)", str(value).strip(), re.I)
    require(match is not None, f"invalid qacct memory: {value}")
    multiplier = {
        "": 1,
        "K": 1024,
        "M": 1024**2,
        "G": GIB,
        "T": 1024**4,
        "P": 1024**5,
    }[match.group(2).upper()]
    return math.ceil(float(match.group(1)) * multiplier)


def parse_qacct(path):
    path = Path(path)
    require(path.is_file(), f"missing qacct receipt: {path}")
    values = {}
    separators = 0
    for raw in path.read_text(encoding="utf-8", errors="replace").splitlines():
        line = raw.strip()
        if not line:
            continue
        if set(line) == {"="}:
            separators += 1
            continue
        fields = line.split(None, 1)
        if len(fields) != 2:
            continue
        key, value = fields
        require(key not in values, f"ambiguous qacct field: {key}")
        values[key] = value.strip()
    require(separators <= 1, "qacct contains multiple records")
    required = {
        "jobnumber",
        "taskid",
        "jobname",
        "project",
        "hostname",
        "qname",
        "granted_pe",
        "slots",
        "failed",
        "exit_status",
        "ru_wallclock",
        "cpu",
        "maxvmem",
    }
    missing = sorted(required.difference(values))
    require(not missing, "qacct missing fields: " + ", ".join(missing))
    return values


def parse_scheduler_request(path):
    """Parse the canonical raw output emitted by BU SCC ``qsub -verify``."""
    path = Path(path)
    require(path.is_file(), f"missing qsub-verify receipt: {path}")
    values = {}
    recognized = {
        "job_number",
        "merge",
        "hard resource_list",
        "job_name",
        "stdout_path_list",
        "verify",
        "env_list",
        "script_file",
        "parallel environment",
        "project",
    }
    for raw in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if ":" not in raw:
            continue
        key, value = raw.split(":", 1)
        key = key.strip()
        value = value.strip()
        # BU includes the complete submitted script below ``script_ptr``.
        # Ignore its arbitrary shell text and every other unregistered display
        # field; only scheduler-owned fields enter the request certificate.
        if key not in recognized:
            continue
        require(key not in values, f"ambiguous qsub-verify field: {key}")
        values[key] = value
    missing = sorted(recognized.difference(values))
    require(not missing, "qsub-verify receipt missing fields: " + ", ".join(missing))
    return values


def parse_resource_list(value):
    resources = {}
    for token in str(value).split(","):
        fields = token.strip().split("=", 1)
        require(
            len(fields) == 2 and fields[0] and fields[1] and fields[0] not in resources,
            "malformed or duplicate qsub-verify hard resource",
        )
        resources[fields[0]] = fields[1]
    return resources


def parse_environment_list(value):
    environment = {}
    for token in str(value).split(","):
        fields = token.split("=", 1)
        require(
            len(fields) == 2 and fields[0] and fields[0] not in environment,
            "malformed or duplicate qsub-verify environment binding",
        )
        environment[fields[0]] = fields[1]
    return environment


def scheduler_stdout_path(value):
    """Normalize BU's ``NONE:NONE:/absolute/path`` stdout spelling."""
    match = re.fullmatch(r"(?:NONE:NONE:)?(/[^,\r\n]+)", str(value).strip())
    require(match is not None, "qsub-verify stdout path is not one absolute path")
    return Path(match.group(1)).absolute()


def hostname_key(value, label, *, require_short=False):
    """Normalize SCC's short ``HOSTNAME`` and qacct's FQDN spelling."""
    value = str(value).strip().lower()
    require(
        re.fullmatch(r"[a-z0-9-]+(?:\.[a-z0-9-]+)*", value) is not None,
        f"invalid {label} hostname",
    )
    require(
        not require_short or "." not in value,
        f"{label} hostname is not the short SCC name",
    )
    return value.split(".", 1)[0]


def expected_scheduler_environment(request):
    job_dir = Path(request["job_dir"]).absolute()
    run_dir = job_dir.parents[2]
    source_dir = Path(request["source_dir"]).absolute()
    expected = {
        "KSS_MS_RUN_DIR": str(run_dir),
        "KSS_MS_SOURCE_DIR": str(source_dir),
        "KSS_MS_SOURCE_COMMIT": request["source_commit"],
        "KSS_MS_BUNDLE_SHA256": request["bundle_sha256"],
        "KSS_MS_LABEL": request["label"],
        "KSS_MS_TIMEOUT_SECONDS": request["application_timeout_seconds"],
        "KSS_MS_TIMEOUT_BASIS": request["timeout_basis"],
        "KSS_MS_REQUESTED_SLOTS": request["requested_slots"],
        "KSS_MS_MEM_PER_CORE_GIB": request["mem_per_core_gib"],
        "KSS_MS_SCHEDULER_HARD_WALL_SECONDS": request[
            "scheduler_hard_wall_seconds"
        ],
    }
    if request["stage"] == "prepare":
        expected.update(
            {
                "KSS_MS_PREP_JOB_DIR": str(job_dir),
                "KSS_MS_SCALE": request["scale"],
                "KSS_MS_TOPOLOGY": request["topology"],
                "KSS_MS_SOURCE_INPUT_DTA": request["input_path"],
                "KSS_MS_SOURCE_INPUT_SHA256": request["input_sha256"],
                "KSS_MS_STATA_PROCESSORS": request["stata_processors"],
            }
        )
    else:
        label_dir = job_dir.parent
        preparation_dir = label_dir / "prepare"
        expected.update(
            {
                "KSS_MS_JOB_DIR": str(job_dir),
                "KSS_MS_MODE": request["stage"],
                "KSS_MS_CASE_JSON": str(label_dir / "case.json"),
                "KSS_MS_CASE_SHA256": request["case_sha256"],
                "KSS_MS_INPUT_CSV": request["input_path"],
                "KSS_MS_INPUT_SHA256": request["input_sha256"],
                "KSS_MS_MATLAB_ROOT": request["matlab_root"],
                "KSS_MS_CONTRACT_SHA256": request["contract_sha256"],
                "KSS_MS_PREPARATION_RECEIPT": str(preparation_dir / "wrapper.json"),
                "KSS_MS_PREPARATION_RECEIPT_SHA256": request[
                    "preparation_receipt_sha256"
                ],
                "KSS_MS_PREPARATION_ACCEPTANCE": str(
                    preparation_dir / "acceptance.json"
                ),
                "KSS_MS_PREPARATION_ACCEPTANCE_SHA256": request[
                    "preparation_acceptance_sha256"
                ],
            }
        )
    return expected


def validate_scheduler_request(path, request):
    """Bind the submitted project, PE, resources, paths, and KSS environment."""
    values = parse_scheduler_request(path)
    require(values["job_number"] == "unassigned", "qsub-verify assigned a job number")
    require(values["merge"] == "y", "qsub-verify merge mode changed")
    require(values["verify"] == "-verify", "qsub-verify mode changed")
    require(values["job_name"] == request["job_name"], "qsub-verify job name changed")
    require(values["project"] == request["project"] == "welfgr", "qsub-verify project changed")
    resources = parse_resource_list(values["hard resource_list"])
    require(
        resources
        == {
            "mem_per_core": f"{request['mem_per_core_gib']}G",
            "h_rt": request["scheduler_hard_wall_hms"],
        },
        "qsub-verify hard resource list changed",
    )
    pe_match = re.fullmatch(r"(\S+)\s+range:\s+(\S+)", values["parallel environment"])
    require(pe_match is not None, "qsub-verify parallel environment is malformed")
    require(
        pe_match.group(1) == request["parallel_environment"] == "omp"
        and pe_match.group(2) == request["requested_slots"],
        "qsub-verify parallel environment or range changed",
    )
    require(
        scheduler_stdout_path(values["stdout_path_list"])
        == Path(request["stdout_path"]).absolute(),
        "qsub-verify stdout path changed",
    )
    require(
        Path(values["script_file"]).absolute() == Path(request["script_path"]).absolute(),
        "qsub-verify script path changed",
    )
    environment = parse_environment_list(values["env_list"])
    for name, expected in expected_scheduler_environment(request).items():
        require(
            environment.get(name) == expected,
            f"qsub-verify environment binding changed: {name}",
        )
    return values


def validate_stage_resource_policy(request):
    """Apply the fixed, unversioned MATLAB benchmark reservation policy."""
    stage = request.get("stage")
    require(stage in STAGE_RESOURCE_POLICY, "scheduler stage is unsupported")
    policy = STAGE_RESOURCE_POLICY[stage]
    requested_slots = integer(int(request["requested_slots"]), "requested slots", 1)
    mem_per_core_gib = integer(
        int(request["mem_per_core_gib"]), "requested memory per core", 1
    )
    total_reserved_gib = integer(
        int(request["total_reserved_gib"]), "total reserved GiB", 1
    )
    require(
        requested_slots == policy["slots"]
        and mem_per_core_gib == policy["mem_per_core_gib"]
        and total_reserved_gib == requested_slots * mem_per_core_gib == 56
        and request.get("stata_processors") == policy["stata_processors"],
        f"{stage} scheduler resource policy changed",
    )
    return requested_slots, mem_per_core_gib, total_reserved_gib


def validate_scheduler(
    request, submission, scheduler_request_path, job_id_path, qacct_path, wrapper
):
    require(
        request.get("receipt_version") == "KSS-MATLAB-SCALE-SUBMISSION-REQUEST-V1",
        "submission request version changed",
    )
    require(
        submission.get("receipt_version") == "KSS-MATLAB-SCALE-SUBMISSION-V1",
        "submission receipt version changed",
    )
    require(
        submission.get("request_sha256") == sha256_file(request["_path"]),
        "submission does not bind its request",
    )
    for field in (
        "stage",
        "label",
        "source_commit",
        "bundle_sha256",
        "input_sha256",
        "case_sha256",
        "sample_mode",
        "preparation_receipt_sha256",
        "preparation_acceptance_sha256",
        "case_gate_sha256",
        "requested_slots",
        "mem_per_core_gib",
        "total_reserved_gib",
        "stata_processors",
        "application_timeout_seconds",
        "timeout_basis",
        "scheduler_hard_wall_seconds",
        "execution_shape",
        "project",
        "parallel_environment",
        "job_name",
        "job_dir",
        "source_dir",
        "script_path",
        "stdout_path",
        "scheduler_request_path",
        "qacct_path",
        "automatic_cascade",
    ):
        require(request.get(field) == submission.get(field), f"submission/request {field} changed")
    require(request["execution_shape"] == "one_scalar_job_no_array", "execution shape changed")
    require(request["automatic_cascade"] == "disabled", "automatic cascade was enabled")
    require(request["sample_mode"] == "fixed", "fixed submitter accepted a selection case")
    requested_slots, mem_per_core_gib, total_reserved_gib = (
        validate_stage_resource_policy(request)
    )
    scheduler_request_path = Path(scheduler_request_path).absolute()
    require(
        Path(request["scheduler_request_path"]).absolute() == scheduler_request_path,
        "qsub-verify receipt path changed",
    )
    require(
        submission.get("scheduler_request_sha256")
        == sha256_file(scheduler_request_path),
        "submission does not bind the qsub-verify receipt",
    )
    validate_scheduler_request(scheduler_request_path, request)

    job_id_path = Path(job_id_path)
    require(job_id_path.is_file(), "job-id file is missing")
    job_id = job_id_path.read_text(encoding="utf-8").strip()
    require(JOB_ID.fullmatch(job_id) is not None, "invalid scalar job-id receipt")
    require(submission.get("job_id") == job_id, "submission/job-id receipt mismatch")
    qacct = parse_qacct(qacct_path)
    require(qacct["jobnumber"] == job_id, "qacct job number mismatch")
    require(qacct["taskid"] == "undefined", "qacct reports an array task")
    require(qacct["jobname"] == request["job_name"], "qacct job name mismatch")
    require(qacct["project"] == request["project"] == "welfgr", "qacct project mismatch")
    require(
        qacct["granted_pe"] == request["parallel_environment"] == "omp",
        "qacct parallel environment mismatch",
    )
    require(qacct["failed"] == "0", "SGE failed is nonzero")
    require(qacct["exit_status"] == "0", "SGE exit_status is nonzero")

    slots = integer(int(qacct["slots"]), "qacct slots", 1)
    require(slots == requested_slots, "qacct slot count mismatch")
    hard_wall = integer(int(request["scheduler_hard_wall_seconds"]), "hard wall", 1)
    wall = parse_duration(qacct["ru_wallclock"])
    cpu = parse_duration(qacct["cpu"])
    require(0 < wall <= hard_wall + 2, "qacct wall is nonpositive or exceeds hard request")
    require(0 < cpu <= wall * slots * 1.10 + 120, "qacct CPU is invalid for the slot reservation")
    maxvmem = parse_memory(qacct["maxvmem"])
    reserved_bytes = total_reserved_gib * GIB
    require(0 < maxvmem <= reserved_bytes, "qacct maxvmem is invalid for the reservation")

    require(wrapper.get("job_id") == job_id, "application receipt job ID mismatch")
    require(wrapper.get("sge_task_id") in (None, "", "undefined"), "application ran as array task")
    require(
        hostname_key(
            wrapper.get("hostname", ""), "application", require_short=True
        )
        == hostname_key(qacct["hostname"], "qacct"),
        "application/qacct hostname mismatch",
    )
    require(wrapper.get("requested_slots") == requested_slots, "wrapper slot request mismatch")
    require(wrapper.get("actual_slots") == slots, "wrapper actual slot count mismatch")
    require(
        wrapper.get("mem_per_core_gib") == int(request["mem_per_core_gib"]),
        "wrapper memory request mismatch",
    )
    require(
        wrapper.get("scheduler_hard_wall_seconds") == hard_wall,
        "wrapper hard-wall request mismatch",
    )
    for field in ("wrapper_wall_seconds", "process_wall_seconds", "cpu_seconds", "peak_rss_kib"):
        require(finite(wrapper.get(field), field) > 0, f"wrapper lacks positive {field}")
    require(wrapper.get("time_exit_status") == 0, "GNU time exit status changed")
    if request["stage"] in ("cold", "warm"):
        tree_peak = finite(wrapper.get("process_tree_peak_rss_kib"), "process-tree RSS")
        require(tree_peak > 0, "process-tree RSS is not positive")
        require(tree_peak * 1024 <= reserved_bytes, "process-tree RSS exceeds reservation")
        require(wrapper.get("process_tree_status") == "PASS", "process-tree monitor did not pass")
        require(
            wrapper.get("expected_pool_workers") == EXPECTED_POOL_WORKERS,
            "wrapper expected pool size changed",
        )
        identity_sha = hash_value(
            wrapper.get("process_identity_sha256"), "wrapper process identity SHA-256"
        )
        client_pid = integer(wrapper.get("matlab_client_pid"), "MATLAB client PID", 2)
        worker_pids = wrapper.get("matlab_worker_pids")
        require(
            isinstance(worker_pids, list) and len(worker_pids) == EXPECTED_POOL_WORKERS,
            "wrapper MATLAB worker PID count changed",
        )
        worker_pids = [integer(pid, "MATLAB worker PID", 2) for pid in worker_pids]
        require(
            wrapper.get("process_identity_status") == "PASS"
            and len(set(worker_pids)) == EXPECTED_POOL_WORKERS
            and client_pid not in worker_pids
            and wrapper.get("process_tree_identity_sha256") == identity_sha
            and integer(
                wrapper.get("process_tree_identity_observation_count"),
                "named-process observation count",
                1,
            )
            >= 1
            and integer(
                wrapper.get("process_tree_identity_peak_process_count"),
                "named-process sample count",
                MINIMUM_PROCESS_COUNT,
            )
            >= MINIMUM_PROCESS_COUNT
            and finite(
                wrapper.get("process_tree_identity_peak_rss_kib"),
                "named-process RSS",
            )
            > 0,
            "process tree lacks named MATLAB client and worker proof",
        )
    return {
        "job_id": job_id,
        "hostname": qacct["hostname"],
        "queue": qacct["qname"],
        "slots": slots,
        "wall_seconds": wall,
        "cpu_seconds": cpu,
        "maxvmem_bytes": maxvmem,
        "scheduler_request_sha256": sha256_file(scheduler_request_path),
        "qacct_sha256": sha256_file(qacct_path),
    }


def validate_prepare_output(job_dir, wrapper):
    job_dir = Path(job_dir)
    preparation = validate_preparation_receipt(wrapper)
    require(wrapper.get("process_exit_status") == 0, "preparation process exit is nonzero")
    require(wrapper.get("timeout") is False, "preparation timed out")
    require(wrapper.get("requested_stata_processors") == 4, "Stata processor request changed")
    expected_artifacts = {
        "application": job_dir / "application.txt",
        "summary": job_dir / "prepare.csv",
        "stata_pass_marker": job_dir / "prepare.stata.pass",
        "time_report": job_dir / "resources.txt",
        "input_csv": job_dir / "input.csv",
        "retained_keys": job_dir / "retained_keys.canonical.txt",
    }
    for name, path in expected_artifacts.items():
        require(path.is_file(), f"preparation artifact is missing: {name}")
        require(
            wrapper.get("artifacts", {}).get(name, {}).get("sha256") == sha256_file(path),
            f"preparation artifact binding changed: {name}",
        )
    replay_args = SimpleNamespace(
        label=preparation["label"],
        scale=str(preparation["scale"]),
        topology=preparation["topology"],
        source_commit=preparation["source_commit"],
        bundle_sha256=preparation["bundle_sha256"],
        source_input_sha256=preparation["source_input_sha256"],
        prepared_input_sha256=preparation["prepared_input_sha256"],
        retained_key_sha256=preparation["retained_key_sha256"],
        actual_slots=str(wrapper.get("actual_slots")),
        input_csv=str(expected_artifacts["input_csv"]),
        retained_keys=str(expected_artifacts["retained_keys"]),
        summary=str(expected_artifacts["summary"]),
        pass_marker=str(expected_artifacts["stata_pass_marker"]),
        time_report=str(expected_artifacts["time_report"]),
    )
    (
        summary,
        dimensions,
        source_dimensions,
        prepared_input_sha,
        retained_key_sha,
        time_metrics,
    ) = validate_prepare_success(replay_args)
    require(dimensions == preparation["dimensions"], "replayed preparation dimensions changed")
    require(
        source_dimensions == preparation["source_dimensions"],
        "replayed preparation source dimensions changed",
    )
    require(
        prepared_input_sha == preparation["prepared_input_sha256"]
        and retained_key_sha == preparation["retained_key_sha256"],
        "replayed preparation output digest changed",
    )
    for field, value in time_metrics.items():
        require(wrapper.get(field) == value, f"replayed GNU-time field changed: {field}")
    for field in (
        "load_seconds",
        "normalization_seconds",
        "fixture_seconds",
        "export_seconds",
        "total_seconds",
        "connector_rows",
        "copy_cut_conductance",
        "normalized_lambda2",
        "normalized_lambda_max",
        "normalized_condition_proxy",
        "minimum_weighted_degree",
        "maximum_weighted_degree",
        "stata_version",
        "stata_flavor",
    ):
        if field not in summary or summary[field] in (None, ""):
            continue
        value = summary[field]
        if (
            field.endswith("_seconds")
            or field.startswith("normalized_")
            or field
            in {
                "copy_cut_conductance",
                "minimum_weighted_degree",
                "maximum_weighted_degree",
            }
        ):
            value = float(value)
        elif field == "connector_rows":
            value = int(float(value))
        require(wrapper.get(field) == value, f"replayed preparation field changed: {field}")
    application = expected_artifacts["application"].read_text(encoding="utf-8", errors="replace")
    require(
        f"KSS MATLAB SCALE PREPARE PASS: {preparation['label']}" in application,
        "preparation application success signal is missing",
    )
    wrapper_marker = job_dir / "wrapper.pass"
    require(wrapper_marker.is_file(), "preparation wrapper pass marker is missing")
    expected_marker = (
        "KSS_MATLAB_SCALE_PREPARE_WRAPPER_PASS "
        f"{preparation['label']} {preparation['source_commit']} "
        f"{preparation['bundle_sha256']} {preparation['prepared_input_sha256']} "
        f"{preparation['retained_key_sha256']}\n"
    )
    require(wrapper_marker.read_text(encoding="utf-8") == expected_marker, "preparation pass marker changed")
    return preparation


def expected_evidence_paths(job_dir, stage, label):
    """Return the only scheduler-evidence paths accepted for one scalar job."""
    job_dir = Path(job_dir).absolute()
    require(job_dir.name == stage, "job directory stage changed")
    require(job_dir.parent.name == label, "job directory label changed")
    require(job_dir.parent.parent.name == "matlab_scale", "job directory layout changed")
    run_dir = job_dir.parents[2]
    record = f"matlab_scale_{label}_{stage}"
    return {
        "request": run_dir / "submissions" / f"{record}.request.tsv",
        "submission": run_dir / "submissions" / f"{record}.tsv",
        "scheduler_request": run_dir
        / "submissions"
        / f"{record}.scheduler_request.txt",
        "job_id_file": run_dir / "submissions" / f"{record}.job_id",
        "qacct": run_dir / "qacct" / f"{record}.txt",
        "wrapper": job_dir / "wrapper.json",
    }


def evidence_record(paths):
    result = {}
    for name, path in paths.items():
        path = Path(path).absolute()
        require(path.is_file() and not path.is_symlink(), f"invalid acceptance evidence: {name}")
        result[name] = {"path": str(path), "sha256": sha256_file(path)}
    return result


def build_acceptance(args):
    """Recompute one acceptance record entirely from the named source bytes."""
    job_dir = Path(args.job_dir).absolute()
    request_path = Path(args.request).absolute()
    submission_path = Path(args.submission).absolute()
    scheduler_request_path = Path(args.scheduler_request).absolute()
    job_id_path = Path(args.job_id_file).absolute()
    qacct_path = Path(args.qacct).absolute()
    request = parse_key_value_receipt(request_path)
    request["_path"] = str(request_path)
    submission = parse_key_value_receipt(submission_path)
    require(request.get("stage") == args.stage, "requested stage differs from validator stage")
    require(Path(request.get("job_dir", "")).absolute() == job_dir, "job directory receipt changed")
    require(Path(request.get("qacct_path", "")).absolute() == qacct_path, "qacct path receipt changed")
    expected_paths = expected_evidence_paths(job_dir, args.stage, request.get("label"))
    actual_paths = {
        "request": request_path,
        "submission": submission_path,
        "scheduler_request": scheduler_request_path,
        "job_id_file": job_id_path,
        "qacct": qacct_path,
        "wrapper": job_dir / "wrapper.json",
    }
    require(actual_paths == expected_paths, "scheduler evidence path layout changed")
    wrapper_path = actual_paths["wrapper"]
    wrapper = load_json(wrapper_path)
    scheduler = validate_scheduler(
        request,
        submission,
        scheduler_request_path,
        job_id_path,
        qacct_path,
        wrapper,
    )

    if args.stage == "prepare":
        preparation = validate_prepare_output(job_dir, wrapper)
        source_commit = preparation["source_commit"]
        bundle_sha = preparation["bundle_sha256"]
        case_sha = None
    else:
        require(args.case and args.contract and args.case_sha256, "MATLAB job validation needs case data")
        contract = load_json(args.contract)
        require(sha256_file(args.case) == args.case_sha256, "case checksum changed")
        case = validate_case(load_json(args.case), contract)
        # Delayed import avoids a module cycle when the final validator asks
        # this module to replay the source-byte acceptance gate.
        from validate import validate_job

        validate_job(job_dir, args.stage, case, args.case_sha256)
        source_commit = case["source"]["source_commit"]
        bundle_sha = case["source"]["bundle_sha256"]
        case_sha = args.case_sha256

    sources = evidence_record(actual_paths)
    return {
        "schema": ACCEPTANCE_SCHEMA,
        "status": "PASS",
        "failure_code": "NONE",
        "failure_message": "NONE",
        "stage": args.stage,
        "label": request["label"],
        "source_commit": source_commit,
        "bundle_sha256": bundle_sha,
        "case_sha256": case_sha,
        "wrapper_sha256": sources["wrapper"]["sha256"],
        "request_sha256": sources["request"]["sha256"],
        "submission_sha256": sources["submission"]["sha256"],
        "scheduler_request_sha256": sources["scheduler_request"]["sha256"],
        "job_id_file_sha256": sources["job_id_file"]["sha256"],
        "qacct_sha256": scheduler["qacct_sha256"],
        "evidence": sources,
        "layers": {"scheduler": "PASS", "application": "PASS", "output": "PASS"},
        "scheduler": scheduler,
    }


def revalidate_acceptance_sources(
    acceptance_path,
    *,
    stage,
    job_dir,
    case_path=None,
    case_sha256=None,
    contract_path=None,
    expected_acceptance_sha256=None,
):
    """Replay acceptance from request, submission, job-ID, qacct, and output bytes."""
    acceptance_path = Path(acceptance_path).absolute()
    job_dir = Path(job_dir).absolute()
    require(
        acceptance_path == job_dir / "acceptance.json",
        "acceptance receipt path changed",
    )
    if expected_acceptance_sha256 is not None:
        require(
            sha256_file(acceptance_path) == expected_acceptance_sha256,
            "acceptance receipt checksum changed",
        )
    acceptance = load_json(acceptance_path)
    require(acceptance.get("schema") == ACCEPTANCE_SCHEMA, "acceptance schema changed")
    require(acceptance.get("status") == "PASS", "SCC acceptance did not pass")
    require(acceptance.get("stage") == stage, "acceptance stage changed")
    expected_paths = expected_evidence_paths(job_dir, stage, acceptance.get("label"))
    evidence = acceptance.get("evidence")
    require(isinstance(evidence, dict), "acceptance evidence map is missing")
    require(set(evidence) == set(expected_paths), "acceptance evidence set changed")
    for name, expected_path in expected_paths.items():
        source = evidence.get(name)
        require(isinstance(source, dict), f"acceptance evidence is malformed: {name}")
        require(
            Path(source.get("path", "")).absolute() == expected_path,
            f"acceptance evidence path changed: {name}",
        )
        require(
            source.get("sha256") == sha256_file(expected_path),
            f"acceptance evidence bytes changed: {name}",
        )

    rebuilt = build_acceptance(
        SimpleNamespace(
            stage=stage,
            request=str(expected_paths["request"]),
            submission=str(expected_paths["submission"]),
            scheduler_request=str(expected_paths["scheduler_request"]),
            job_id_file=str(expected_paths["job_id_file"]),
            qacct=str(expected_paths["qacct"]),
            job_dir=str(job_dir),
            case=str(case_path) if case_path is not None else None,
            case_sha256=case_sha256,
            contract=str(contract_path) if contract_path is not None else None,
        )
    )
    require(acceptance == rebuilt, "acceptance does not equal replayed source-byte validation")
    return acceptance


def validate(args):
    record = build_acceptance(args)
    atomic_write_json(args.output, record)
    print(f"KSS MATLAB SCALE SCC JOB VALIDATION PASS: {record['label']} {args.stage}")
    return 0


def parse_args(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument("--stage", choices=("prepare", "cold", "warm"), required=True)
    parser.add_argument("--request", required=True)
    parser.add_argument("--submission", required=True)
    parser.add_argument("--scheduler-request", required=True)
    parser.add_argument("--job-id-file", required=True)
    parser.add_argument("--qacct", required=True)
    parser.add_argument("--job-dir", required=True)
    parser.add_argument("--case")
    parser.add_argument("--case-sha256")
    parser.add_argument("--contract")
    parser.add_argument("--output", required=True)
    return parser.parse_args(argv)


if __name__ == "__main__":
    try:
        sys.exit(validate(parse_args()))
    except (BenchmarkError, KeyError, OSError, TypeError, ValueError) as exc:
        print(f"KSS MATLAB SCALE SCC JOB VALIDATION FAIL: {exc}", file=sys.stderr)
        sys.exit(2)

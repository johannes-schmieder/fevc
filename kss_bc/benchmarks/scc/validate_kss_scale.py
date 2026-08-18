#!/usr/bin/env python3
"""Validate one independently specified KSS SCC run.

The validator distinguishes a scientifically accepted estimate from a useful
typed diagnostic.  It enforces provenance, scheduler success, direct memory
safety, and (for estimates) numerical identities and complete residuals.
Forecast misses, wall projections, and earlier runs are diagnostics only.
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import re
from pathlib import Path
from typing import Any

HEX40 = re.compile(r"[0-9a-f]{40}")
HEX64 = re.compile(r"[0-9a-f]{64}")
IDENTIFIER = re.compile(r"[A-Za-z0-9._-]+")
STATA_VARIABLE = re.compile(r"[A-Za-z_][A-Za-z0-9_]{0,31}")
FATAL_STATA = re.compile(r"(?:^|\n)r\([0-9]+\);(?:\n|$)")
GIB = 1024**3
PROJECT = "welfgr"
PARALLEL_ENVIRONMENT = "omp"
OPTION_CONTRACT = "KSS-STREAMLINE-OPTIONS-V1"
RUN_RECEIPT_VERSION = "KSS-STREAMLINE-RUN-V1"
ADMISSION_RECEIPT_VERSION = RUN_RECEIPT_VERSION  # compatibility export
NODE_RECEIPT_VERSION = "KSS-STREAMLINE-NODE-V1"
TMP_CAPACITY_RECEIPT_VERSION = "KSS-STREAMLINE-TMP-CAPACITY-V1"
RSS_SAMPLER_RECEIPT_VERSION = "KSS-STREAMLINE-RSS-SAMPLER-V1"
REGISTERED_RNG_CONTRACTS = {
    "18": "KSS-MT64S-DOMAIN-CURSOR-V3-STATA18",
    "19": "KSS-MT64S-DOMAIN-CURSOR-V3-STATA19",
}
OPTION_FIELDS = (
    "option_contract", "frequency_var", "target_var", "deletion_var",
    "deletion_mode",
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def sha256(path: Path) -> str:
    require(path.is_file(), f"missing file: {path}")
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def read_one_csv(path: Path, label: str) -> dict[str, str]:
    require(path.is_file(), f"missing {label}: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    require(len(rows) == 1, f"{label} must contain exactly one row")
    return rows[0]


def read_key_values(path: Path, label: str) -> dict[str, str]:
    require(path.is_file(), f"missing {label}: {path}")
    values: dict[str, str] = {}
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.reader(handle, delimiter="\t")
        require(next(reader, None) == ["key", "value"],
                f"invalid {label} header")
        for row in reader:
            require(len(row) == 2 and row[0], f"invalid {label} row")
            require(row[0] not in values, f"duplicate {label} key: {row[0]}")
            values[row[0]] = row[1]
    return values


def number(value: str, label: str) -> float:
    try:
        parsed = float(value)
    except (TypeError, ValueError) as exc:
        raise ValueError(f"invalid {label}") from exc
    require(math.isfinite(parsed), f"nonfinite {label}")
    return parsed


def finite(row: dict[str, str], field: str) -> float:
    require(field in row, f"missing {field}")
    return number(row[field], field)


def integer(row: dict[str, str], field: str) -> int:
    value = finite(row, field)
    require(value.is_integer(), f"noninteger {field}")
    return int(value)


def close(left: float, right: float, tolerance: float = 1e-9) -> bool:
    return abs(left - right) <= tolerance * (1 + max(abs(left), abs(right)))


def parse_duration(value: str) -> float:
    try:
        result = float(value)
    except ValueError:
        match = re.fullmatch(r"(\d+):(\d{2}):(\d{2}(?:\.\d+)?)", value)
        require(match is not None, f"invalid duration: {value}")
        result = (3600 * int(match.group(1)) + 60 * int(match.group(2)) +
                  float(match.group(3)))
    require(math.isfinite(result) and result >= 0, f"invalid duration: {value}")
    return result


def parse_memory(value: str) -> int:
    match = re.fullmatch(r"([0-9]+(?:\.[0-9]+)?)([KMGTP]?)", value, re.I)
    require(match is not None, f"invalid memory value: {value}")
    multiplier = {"": 1, "K": 1024, "M": 1024**2, "G": GIB,
                  "T": 1024**4, "P": 1024**5}[match.group(2).upper()]
    return int(math.ceil(float(match.group(1)) * multiplier))


def validate_options(values: dict[str, str], label: str) -> dict[str, str]:
    require(values.get("option_contract") == OPTION_CONTRACT,
            f"{label} option contract changed")
    for field in ("frequency_var", "target_var", "deletion_var"):
        value = values.get(field, "")
        require(value == "-" or STATA_VARIABLE.fullmatch(value) is not None,
                f"invalid {label} {field}")
    require(values.get("deletion_mode") == "match",
            f"invalid {label} deletion mode")
    return {field: values[field] for field in OPTION_FIELDS}


def validate_reservation(
    path: Path, *, experiment_id: str, source_commit: str,
    bundle_sha: str, input_sha: str,
) -> dict[str, str]:
    values = read_key_values(path, "reservation")
    expected = {
        "experiment_id": experiment_id,
        "source_commit": source_commit,
        "bundle_sha256": bundle_sha,
        "input_sha256": input_sha,
    }
    for field, wanted in expected.items():
        require(values.get(field) == wanted, f"reservation mismatch: {field}")
    validate_options(values, "reservation")
    slots = integer(values, "requested_slots")
    mem_per_core = integer(values, "mem_per_core_gib")
    total = integer(values, "total_reserved_gib")
    processors = integer(values, "stata_processors")
    wall = integer(values, "hard_wall_seconds")
    estimator_wall = integer(values, "estimator_hard_wall_seconds")
    require(slots >= 1 and mem_per_core >= 1 and total == slots * mem_per_core,
            "reservation memory arithmetic failed")
    require(1 <= processors <= slots,
            "Stata processors exceed the scheduler slot reservation")
    require(wall >= 60, "scheduler wall request is too short")
    reserve = 120 if wall > 120 else wall // 10
    require(estimator_wall == wall - reserve and estimator_wall > 0,
            "invalid wrapper wall reserve")
    fixture = values.get("fixture", "")
    scale = integer(values, "scale_factor")
    require(fixture in {"local", "cz24", "cz25", "cz18",
                        "replicated_blocks", "ring"},
            "unknown fixture")
    require(scale >= 1, "invalid scale factor")
    require(fixture in {"replicated_blocks", "ring"} or scale == 1,
            "base dataset carries a false scale label")
    require(fixture not in {"replicated_blocks", "ring"} or scale >= 2,
            "replication fixture requires at least two copies")
    require(not any(key.startswith("prior_") for key in values),
            "streamlined runs cannot depend on predecessor receipts")
    return values


def parse_scheduler_request(path: Path) -> dict[str, str]:
    require(path.is_file(), f"missing scheduler request: {path}")
    values: dict[str, str] = {}
    accepted = {"hard resource list", "parallel environment", "project",
                "script file", "stdout path list", "env list"}
    for raw in path.read_text(encoding="utf-8").splitlines():
        if ":" not in raw:
            continue
        key, value = raw.split(":", 1)
        key = " ".join(key.lower().replace("_", " ").split())
        if key in accepted:
            require(key not in values, f"duplicate scheduler field: {key}")
            values[key] = value.strip()
    require(accepted <= values.keys(), "scheduler request is incomplete")
    return values


def parse_pairs(value: str, label: str) -> dict[str, str]:
    result: dict[str, str] = {}
    for token in value.split(","):
        fields = token.strip().split("=", 1)
        require(len(fields) == 2 and all(fields), f"invalid {label}")
        require(fields[0] not in result, f"duplicate {label}: {fields[0]}")
        result[fields[0]] = fields[1]
    return result


def validate_scheduler_request(
    path: Path, *, experiment_id: str, source_commit: str, bundle_sha: str,
    input_sha: str, probes: int, reservation: dict[str, str],
) -> dict[str, Any]:
    values = parse_scheduler_request(path)
    resources = parse_pairs(values["hard resource list"], "hard resources")
    require(set(resources) == {"mem_per_core", "h_rt"},
            "unexpected hard resource")
    mem_per_core = integer(reservation, "mem_per_core_gib")
    require(resources["mem_per_core"] == f"{mem_per_core}G",
            "scheduler memory differs from run specification")
    wall = integer(reservation, "hard_wall_seconds")
    require(close(parse_duration(resources["h_rt"]), wall, 0),
            "scheduler wall differs from run specification")
    pe = re.fullmatch(r"([^\s]+)\s+range:\s*([0-9]+)",
                      values["parallel environment"])
    require(pe is not None and pe.group(1) == PARALLEL_ENVIRONMENT,
            "invalid scheduler parallel environment")
    slots = integer(reservation, "requested_slots")
    require(int(pe.group(2)) == slots, "scheduler slots changed")
    require(values["project"] == PROJECT, "scheduler project changed")
    environment = parse_pairs(values["env list"], "scheduler environment")
    run_dir = environment.get("KSS_RUN_DIR", "")
    require(run_dir.startswith("/projectnb/welfgr/kss-bc/runs/") and
            ".." not in run_dir.split("/"), "invalid run directory")
    source_dir = f"/projectnb/welfgr/kss-bc/bundles/{bundle_sha}/source"
    expected = {
        "KSS_SOURCE_DIR": source_dir,
        "KSS_SOURCE_COMMIT": source_commit,
        "KSS_BUNDLE_SHA256": bundle_sha,
        "KSS_INPUT_SHA256": input_sha,
        "KSS_EXPERIMENT_ID": experiment_id,
        "KSS_FIXTURE": reservation["fixture"],
        "KSS_SCALE_FACTOR": reservation["scale_factor"],
        "KSS_PROBES": str(probes),
        "KSS_FREQUENCY_VAR": reservation["frequency_var"],
        "KSS_TARGET_VAR": reservation["target_var"],
        "KSS_DELETION_VAR": reservation["deletion_var"],
        "KSS_REQUESTED_SLOTS": reservation["requested_slots"],
        "KSS_STATA_PROCESSORS": reservation["stata_processors"],
        "KSS_MEM_PER_CORE_GIB": reservation["mem_per_core_gib"],
        "KSS_MEMORY_GIB": reservation["total_reserved_gib"],
        "KSS_HARD_WALL_SECONDS": reservation["hard_wall_seconds"],
        "KSS_OUTPUT_DIR": f"{run_dir}/experiments/{experiment_id}",
    }
    for field, wanted in expected.items():
        require(environment.get(field) == wanted,
                f"scheduler environment mismatch: {field}")
    require(not any(key.startswith("KSS_PRIOR_") for key in environment),
            "scheduler environment contains a predecessor gate")
    seed = environment.get("KSS_SEED", "")
    batch = environment.get("KSS_BATCH", "")
    require(seed.isdigit() and int(seed) <= 2147483646,
            "invalid scheduler seed")
    require(batch == "auto" or (batch.isdigit() and int(batch) >= 1),
            "invalid scheduler batch")
    return {"receipt_sha256": sha256(path), "slots": slots,
            "mem_per_core_gib": mem_per_core, "hard_wall_seconds": wall,
            "environment": {**expected, "KSS_SEED": seed,
                            "KSS_BATCH": batch}}


def parse_qacct(path: Path) -> dict[str, str]:
    require(path.is_file(), f"missing qacct: {path}")
    values: dict[str, str] = {}
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or set(line) == {"="}:
            continue
        fields = line.split(None, 1)
        if len(fields) == 2:
            require(fields[0] not in values, f"ambiguous qacct: {fields[0]}")
            values[fields[0]] = fields[1].strip()
    required = {"jobnumber", "taskid", "project", "granted_pe", "slots",
                "failed", "exit_status", "ru_wallclock", "ru_maxrss",
                "cpu", "maxvmem", "hostname"}
    require(required <= values.keys(), "qacct is incomplete")
    return values


def validate_scheduler(
    qacct: Path, job_id_file: Path, reservation: dict[str, str],
) -> dict[str, Any]:
    job = job_id_file.read_text(encoding="utf-8").strip()
    require(job.isdigit(), "invalid job ID")
    values = parse_qacct(qacct)
    require(values["jobnumber"] == job and values["taskid"] == "undefined",
            "qacct is not the requested scalar job")
    require(values["project"] == PROJECT and
            values["granted_pe"] == PARALLEL_ENVIRONMENT,
            "qacct scheduler binding changed")
    require(int(values["slots"]) == integer(reservation, "requested_slots"),
            "qacct slot count changed")
    require(int(values["failed"]) == 0 and int(values["exit_status"]) == 0,
            "scheduler or wrapper failed")
    wall = parse_duration(values["ru_wallclock"])
    hard_wall = integer(reservation, "hard_wall_seconds")
    require(wall <= hard_wall + 2, "actual wall exceeds scheduler limit")
    maxvmem = parse_memory(values["maxvmem"])
    maxrss = int(values["ru_maxrss"]) * 1024
    reserved = integer(reservation, "total_reserved_gib") * GIB
    require(0 < maxvmem <= reserved and 0 < maxrss <= reserved,
            "actual process memory exceeds the reservation")
    return {"job_id": job, "slots": int(values["slots"]),
            "wall_seconds": wall, "cpu_seconds": parse_duration(values["cpu"]),
            "maxvmem_bytes": maxvmem, "ru_maxrss_bytes": maxrss,
            "hostname": values["hostname"]}


def validate_node_receipt(
    path: Path, *, experiment_id: str, source_commit: str, bundle_sha: str,
    input_sha: str, reservation: dict[str, str], scheduler: dict[str, Any],
) -> dict[str, str]:
    values = read_key_values(path, "node receipt")
    expected = {
        "receipt_version": NODE_RECEIPT_VERSION,
        "experiment_id": experiment_id,
        "job_id": scheduler["job_id"],
        "requested_slots": reservation["requested_slots"],
        "actual_slots": reservation["requested_slots"],
        "requested_stata_processors": reservation["stata_processors"],
        "mem_per_core_gib": reservation["mem_per_core_gib"],
        "reserved_memory_gib": reservation["total_reserved_gib"],
        "scheduler_hard_wall_seconds": reservation["hard_wall_seconds"],
        "estimator_hard_wall_seconds": reservation["estimator_hard_wall_seconds"],
        "source_commit": source_commit,
        "bundle_sha256": bundle_sha,
        "input_sha256": input_sha,
        "fixture": reservation["fixture"],
        "scale_factor": reservation["scale_factor"],
    }
    for field, wanted in expected.items():
        require(values.get(field) == wanted, f"node receipt mismatch: {field}")
    require(validate_options(values, "node") ==
            validate_options(reservation, "reservation"),
            "node option tuple changed")
    require(int(values["tmp_available_bytes"]) >=
            int(values["tmp_required_bytes"]) > 0,
            "node-local temporary capacity is insufficient")
    return values


def validate_fixture(row: dict[str, str]) -> None:
    fixture = row.get("fixture", "")
    scale = integer(row, "scale_factor")
    if fixture not in {"replicated_blocks", "ring"}:
        require(scale == 1, "base fixture has a false scale label")
        return
    require(scale >= 2, "replication fixture has fewer than two copies")
    pairs = scale * (scale - 1) // 2 if fixture == "replicated_blocks" \
        else (1 if scale == 2 else scale)
    connectors = 4 * pairs
    base = [integer(row, field) for field in (
        "fixture_base_rows", "fixture_base_physical", "fixture_base_workers",
        "fixture_base_firms", "fixture_base_cells", "fixture_base_units")]
    expected = [integer(row, field) for field in (
        "fixture_expected_rows", "fixture_expected_physical",
        "fixture_expected_workers", "fixture_expected_firms",
        "fixture_expected_cells", "fixture_expected_units")]
    calculated = [scale * base[0] + connectors,
                  scale * base[1] + connectors,
                  scale * base[2] + 2 * pairs, scale * base[3],
                  scale * base[4] + connectors,
                  scale * base[5] + connectors]
    require(expected == calculated and
            integer(row, "fixture_connector_rows") == connectors,
            "fixture dimension certificate failed")
    ratio = finite(row, "fixture_connector_volume_ratio")
    require(close(ratio, connectors / expected[1], 1e-12),
            "fixture connector-volume ratio changed")
    conductance = finite(row, "fixture_meta_conductance")
    lambda2 = finite(row, "fixture_meta_lambda2")
    lambda_max = finite(row, "fixture_meta_lambda_max")
    condition = finite(row, "fixture_meta_cond_proxy")
    require(0 < conductance <= 1 and 0 < lambda2 <= lambda_max <= 2 + 1e-12,
            "invalid connector meta-graph diagnostics")
    require(close(condition, lambda_max / lambda2, 1e-10),
            "connector meta-graph condition identity failed")


def validate_identity(row: dict[str, str], prefix: str) -> None:
    parts = (finite(row, f"{prefix}_worker") +
             finite(row, f"{prefix}_firm") +
             2 * finite(row, f"{prefix}_covariance"))
    require(close(finite(row, f"{prefix}_total"), parts, 1e-7),
            f"{prefix} identity failed")


def validate_resource_forecast(
    row: dict[str, str], reservation: dict[str, str],
) -> dict[str, Any]:
    peak = finite(row, "resource_peak_bytes")
    hard = finite(row, "resource_hard_mem_bytes")
    reserved = integer(reservation, "total_reserved_gib") * GIB
    require(row.get("resource_status") == "ADMITTED",
            "command did not pass direct memory admission")
    require(0 < peak <= hard <= reserved, "direct memory admission failed")
    headroom = finite(row, "resource_mem_headroom")
    advisory = finite(row, "resource_mem_admit_bytes")
    require(headroom >= 0 and close(advisory, math.ceil(peak * (1 + headroom)),
                                    1e-12),
            "advisory memory forecast is internally inconsistent")
    wall_upper = finite(row, "resource_wall_upper_seconds")
    wall_advisory = finite(row, "resource_wall_admit_seconds")
    require(wall_upper > 0 and wall_advisory >= wall_upper,
            "advisory wall forecast is invalid")
    return {"direct_peak_bytes": peak, "hard_memory_bytes": hard,
            "headroom_fits": advisory <= hard,
            "wall_advisory_fits": wall_advisory <=
            finite(row, "resource_hard_wall_seconds")}


def validate_summary(
    path: Path, *, experiment_id: str, source_commit: str, bundle_sha: str,
    input_sha: str, probes: int, reservation: dict[str, str],
) -> tuple[dict[str, str], bool, dict[str, Any] | None]:
    row = read_one_csv(path, "summary")
    expected = {"experiment_id": experiment_id, "source_commit": source_commit,
                "bundle_sha256": bundle_sha, "input_sha256": input_sha,
                "fixture": reservation["fixture"],
                "option_contract": OPTION_CONTRACT}
    for field, wanted in expected.items():
        require(row.get(field) == wanted, f"summary mismatch: {field}")
    require(validate_options(row, "summary") ==
            validate_options(reservation, "reservation"),
            "summary option tuple changed")
    require(integer(row, "scale_factor") == integer(reservation, "scale_factor"),
            "summary scale changed")
    require(integer(row, "requested_probes") == probes, "probe count changed")
    for field in ("requested_slots", "actual_slots"):
        require(integer(row, field) == integer(reservation, "requested_slots"),
                "summary slot receipt changed")
    for field in ("requested_stata_processors", "actual_stata_processors"):
        require(integer(row, field) == integer(reservation, "stata_processors"),
                "summary processor receipt changed")
    require(integer(row, "declared_memory_gib") ==
            integer(reservation, "total_reserved_gib"),
            "summary declared memory changed")
    validate_fixture(row)
    success = integer(row, "command_rc") == 0 and row.get("estimator_status") in {
        "KSS_SCALE_EXPERIMENTAL_POINT_ESTIMATES", "KSS_POINT_ESTIMATES_ONLY"}
    if not success:
        require(row.get("estimator_status") not in {"", "COMMAND_FAILURE"},
                "diagnostic run lacks a typed estimator status")
        return row, False, None
    require(row.get("route_api") == "KSS-ROUTE-STRUCTURAL-V1",
            "installed command did not use structural routing")
    require(integer(row, "sample_semantics_valid") == 1,
            "estimation sample was not restored")
    runtime = row.get("rng_runtime", "")
    require(row.get("rng_contract") == REGISTERED_RNG_CONTRACTS.get(runtime),
            "runtime-scoped RNG contract is invalid")
    require(integer(row, "rng_master_seed") == integer(row, "seed"),
            "RNG seed changed")
    for prefix in ("plugin", "correction", "corrected"):
        validate_identity(row, prefix)
    for target in ("worker", "firm", "covariance", "total"):
        require(close(finite(row, f"corrected_{target}"),
                      finite(row, f"plugin_{target}") -
                      finite(row, f"correction_{target}"), 1e-7),
                f"corrected {target} arithmetic failed")
    tolerance = finite(row, "tolerance")
    gate = max(1e-11, 10 * tolerance)
    require(finite(row, "solver_max_residual") <= gate * (1 + 1e-10) and
            finite(row, "rhs_max_residual") <= gate * (1 + 1e-10),
            "complete original-system residual failed")
    return row, True, validate_resource_forecast(row, reservation)


def validate_rhs(path: Path, *, experiment_id: str, probes: int,
                 tolerance: float, rhs_max_residual: float) -> dict[str, Any]:
    require(path.is_file(), f"missing RHS evidence: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    require(len(rows) == 3 * probes + 1, "wrong number of RHS certificates")
    residuals = []
    for row in rows:
        require(row.get("experiment_id") == experiment_id,
                "RHS experiment changed")
        require(integer(row, "converged") == 1, "an RHS did not converge")
        residuals.append(finite(row, "relative_residual"))
    maximum = max(residuals)
    require(maximum <= max(1e-11, 10 * tolerance) * (1 + 1e-10) and
            close(maximum, rhs_max_residual, 1e-10),
            "RHS residual receipt changed")
    return {"count": len(rows), "maximum_residual": maximum}


def validate_auxiliary_csv(path: Path, label: str) -> list[dict[str, str]]:
    require(path.is_file(), f"missing {label}: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    require(rows, f"empty {label}")
    return rows


def validate_tmp_capacity(path: Path, *, source_commit: str, bundle_sha: str,
                          input_sha: str) -> dict[str, Any]:
    values = read_key_values(path, "TMPDIR receipt")
    require(values.get("receipt_version") == TMP_CAPACITY_RECEIPT_VERSION,
            "TMPDIR receipt version changed")
    require(values.get("source_commit") == source_commit and
            values.get("bundle_sha256") == bundle_sha and
            values.get("input_sha256") == input_sha,
            "TMPDIR provenance changed")
    required = int(values["required_bytes"])
    available = int(values["available_bytes"])
    require(values.get("status") == "ADMITTED" and available >= required > 0,
            "TMPDIR direct capacity failed")
    return {"required_bytes": required, "available_bytes": available}


def validate_application(
    *, wrapper_pass: Path, stata_pass: Path, stata_diagnostic: Path | None,
    application_log: Path, experiment_id: str, source_commit: str,
    bundle_sha: str, input_sha: str, scientific_success: bool,
    estimator_status: str,
) -> str:
    expected_wrapper = (f"KSS_STREAMLINE_WRAPPER_PASS {experiment_id} {bundle_sha} "
                        f"{source_commit} {input_sha}")
    require(wrapper_pass.read_text(encoding="utf-8").strip() == expected_wrapper,
            "wrapper marker changed")
    log = application_log.read_text(encoding="utf-8", errors="replace")
    if scientific_success:
        expected = (f"KSS_STREAMLINE_ESTIMATOR_PASS {experiment_id} {bundle_sha} "
                    f"{source_commit} {input_sha}")
        require(stata_pass.read_text(encoding="utf-8").strip() == expected,
                "Stata success marker changed")
        require(f"KSS-STREAMLINE ESTIMATOR PASS: {experiment_id}" in log,
                "Stata success signal missing")
        require(FATAL_STATA.search(log) is None, "fatal Stata error in success log")
        return "SCIENTIFIC_PASS"
    marker = stata_diagnostic or stata_pass.parent / "stata.diagnostic"
    text = marker.read_text(encoding="utf-8").strip()
    require(text.startswith(f"KSS_STREAMLINE_ESTIMATOR_DIAGNOSTIC {experiment_id} ")
            and estimator_status in text, "typed diagnostic marker changed")
    require(f"KSS-STREAMLINE ESTIMATOR DIAGNOSTIC: {experiment_id}" in log,
            "diagnostic signal missing")
    return "DIAGNOSTIC_COLLECTED"


def reconcile_resources(
    row: dict[str, str], scheduler: dict[str, Any], reservation: dict[str, str],
    resource: dict[str, Any] | None,
) -> dict[str, Any]:
    reserved = integer(reservation, "total_reserved_gib") * GIB
    actual = max(scheduler["maxvmem_bytes"], scheduler["ru_maxrss_bytes"])
    require(actual <= reserved, "actual memory exceeds the run allocation")
    forecast_ratio = None
    if resource is not None:
        forecast_ratio = actual / resource["direct_peak_bytes"]
    return {"direct_memory_safe": True, "actual_peak_bytes": actual,
            "reserved_bytes": reserved, "forecast_ratio": forecast_ratio,
            "wall_seconds": scheduler["wall_seconds"],
            "future_run_authorized": False}


def validate_run(
    *, summary: Path, rhs: Path, stage_memory: Path, qacct: Path,
    phase_rss_samples: Path, phase_rss_peaks: Path,
    phase_rss_sampler: Path, tmp_capacity: Path,
    job_id_file: Path, scheduler_request: Path, node_receipt: Path,
    wrapper_pass: Path, stata_pass: Path, application_log: Path,
    reservation_path: Path, wrapper_metrics: Path, process_resources: Path,
    experiment_id: str, source_commit: str, bundle_sha: str,
    input_sha: str, probes: int, stata_diagnostic: Path | None = None,
) -> dict[str, Any]:
    require(IDENTIFIER.fullmatch(experiment_id) is not None,
            "invalid experiment ID")
    require(HEX40.fullmatch(source_commit) is not None, "invalid source commit")
    require(HEX64.fullmatch(bundle_sha) is not None, "invalid bundle hash")
    require(HEX64.fullmatch(input_sha) is not None, "invalid input hash")
    require(probes >= 2, "invalid probe count")
    reservation = validate_reservation(
        reservation_path, experiment_id=experiment_id,
        source_commit=source_commit, bundle_sha=bundle_sha, input_sha=input_sha)
    request = validate_scheduler_request(
        scheduler_request, experiment_id=experiment_id,
        source_commit=source_commit, bundle_sha=bundle_sha,
        input_sha=input_sha, probes=probes, reservation=reservation)
    scheduler = validate_scheduler(qacct, job_id_file, reservation)
    node = validate_node_receipt(
        node_receipt, experiment_id=experiment_id, source_commit=source_commit,
        bundle_sha=bundle_sha, input_sha=input_sha,
        reservation=reservation, scheduler=scheduler)
    row, scientific_success, resource = validate_summary(
        summary, experiment_id=experiment_id, source_commit=source_commit,
        bundle_sha=bundle_sha, input_sha=input_sha, probes=probes,
        reservation=reservation)
    application = validate_application(
        wrapper_pass=wrapper_pass, stata_pass=stata_pass,
        stata_diagnostic=stata_diagnostic, application_log=application_log,
        experiment_id=experiment_id, source_commit=source_commit,
        bundle_sha=bundle_sha, input_sha=input_sha,
        scientific_success=scientific_success,
        estimator_status=row.get("estimator_status", ""))
    rhs_report = None
    if scientific_success:
        rhs_report = validate_rhs(
            rhs, experiment_id=experiment_id, probes=probes,
            tolerance=finite(row, "tolerance"),
            rhs_max_residual=finite(row, "rhs_max_residual"))
    stages = validate_auxiliary_csv(stage_memory, "stage-memory evidence")
    phase_samples = validate_auxiliary_csv(phase_rss_samples, "phase RSS samples")
    phase_peaks = validate_auxiliary_csv(phase_rss_peaks, "phase RSS peaks")
    sampler = read_key_values(phase_rss_sampler, "RSS sampler receipt")
    require(sampler.get("receipt_version") == RSS_SAMPLER_RECEIPT_VERSION,
            "RSS sampler receipt changed")
    capacity = validate_tmp_capacity(
        tmp_capacity, source_commit=source_commit,
        bundle_sha=bundle_sha, input_sha=input_sha)
    require(int(node["tmp_required_bytes"]) == capacity["required_bytes"] and
            int(node["tmp_available_bytes"]) == capacity["available_bytes"],
            "node and TMPDIR receipts disagree")
    wrapper = read_key_values(wrapper_metrics, "wrapper metrics")
    expected_outcome = "PASS" if scientific_success else "DIAGNOSTIC"
    require(wrapper.get("run_outcome") == expected_outcome,
            "wrapper outcome changed")
    require(process_resources.is_file(), "missing process resource report")
    reconciliation = reconcile_resources(row, scheduler, reservation, resource)
    status = ("KSS_STREAMLINE_SCIENTIFIC_PASS" if scientific_success
              else "KSS_STREAMLINE_DIAGNOSTIC_COLLECTED")
    return {
        "status": status,
        "experiment_id": experiment_id,
        "source_commit": source_commit,
        "bundle_sha256": bundle_sha,
        "input_sha256": input_sha,
        "fixture": row.get("fixture", ""),
        "scale_factor": integer(row, "scale_factor"),
        "requested_probes": probes,
        "options": validate_options(row, "summary"),
        "scheduler_request": request,
        "scheduler": scheduler,
        "node": node,
        "application": {"status": application,
                        "estimator_status": row.get("estimator_status")},
        "output": {
            "engine": row.get("engine_selected", ""),
            "rhs": rhs_report,
            "stages": stages,
            "phase_rss": {"samples": len(phase_samples),
                          "peaks": len(phase_peaks)},
            "tmp_capacity": capacity,
            "resource_forecast": resource,
            "resource_reconciliation": reconciliation,
        },
    }


def write_admission_receipt(path: Path, report: dict[str, Any]) -> None:
    """Write a self-contained run receipt; it never unlocks another run."""
    values = (
        ("receipt_version", RUN_RECEIPT_VERSION),
        ("validation_status", report["status"]),
        ("experiment_id", report["experiment_id"]),
        ("fixture", report["fixture"]),
        ("scale_factor", report["scale_factor"]),
        ("source_commit", report["source_commit"]),
        ("bundle_sha256", report["bundle_sha256"]),
        ("input_sha256", report["input_sha256"]),
        ("option_contract", report["options"]["option_contract"]),
        ("frequency_var", report["options"]["frequency_var"]),
        ("target_var", report["options"]["target_var"]),
        ("deletion_var", report["options"]["deletion_var"]),
        ("deletion_mode", report["options"]["deletion_mode"]),
        ("requested_probes", report["requested_probes"]),
        ("engine", report["output"]["engine"]),
        ("future_run_authorized", 0),
    )
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle, delimiter="\t", lineterminator="\n")
        writer.writerow(("key", "value"))
        writer.writerows(values)


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    for name in ("summary", "rhs", "stage-memory", "phase-rss-samples",
                 "phase-rss-peaks", "phase-rss-sampler", "tmp-capacity",
                 "qacct", "job-id-file", "scheduler-request", "node-receipt",
                 "wrapper-pass", "stata-pass", "application-log",
                 "reservation", "wrapper-metrics", "process-resources"):
        result.add_argument(f"--{name}", type=Path, required=True)
    result.add_argument("--stata-diagnostic", type=Path)
    result.add_argument("--experiment-id", required=True)
    result.add_argument("--source-commit", required=True)
    result.add_argument("--bundle-sha256", required=True)
    result.add_argument("--input-sha256", required=True)
    result.add_argument("--probes", type=int, required=True)
    result.add_argument("--certificate", type=Path)
    result.add_argument("--admission-receipt", type=Path)
    return result


def main() -> int:
    args = parser().parse_args()
    try:
        report = validate_run(
            summary=args.summary, rhs=args.rhs,
            stage_memory=args.stage_memory, qacct=args.qacct,
            phase_rss_samples=args.phase_rss_samples,
            phase_rss_peaks=args.phase_rss_peaks,
            phase_rss_sampler=args.phase_rss_sampler,
            tmp_capacity=args.tmp_capacity, job_id_file=args.job_id_file,
            scheduler_request=args.scheduler_request,
            node_receipt=args.node_receipt, wrapper_pass=args.wrapper_pass,
            stata_pass=args.stata_pass, stata_diagnostic=args.stata_diagnostic,
            application_log=args.application_log,
            reservation_path=args.reservation,
            wrapper_metrics=args.wrapper_metrics,
            process_resources=args.process_resources,
            experiment_id=args.experiment_id,
            source_commit=args.source_commit, bundle_sha=args.bundle_sha256,
            input_sha=args.input_sha256, probes=args.probes)
    except (OSError, ValueError) as exc:
        print(f"KSS_STREAMLINE_VALIDATION_FAILURE: {exc}")
        return 1
    encoded = json.dumps(report, indent=2, sort_keys=True)
    if args.certificate is not None:
        args.certificate.write_text(encoded + "\n", encoding="utf-8")
    if args.admission_receipt is not None:
        write_admission_receipt(args.admission_receipt, report)
    print(encoded)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

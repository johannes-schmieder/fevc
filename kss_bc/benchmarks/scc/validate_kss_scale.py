#!/usr/bin/env python3
"""Validate one source-bound, single-process KSS-SCALE SCC run.

Acceptance has three independent layers: SGE accounting, Stata/wrapper
markers, and scientific/resource outputs.  A successful scheduler exit is
never sufficient by itself.
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
JOB_ID = re.compile(r"[0-9]+")
FATAL_STATA = re.compile(r"(?:^|\n)r\([0-9]+\);(?:\n|$)")
GIB = 1024**3
REGISTERED_RNG_CONTRACTS = {
    "18": "KSS-MT64S-DOMAIN-CURSOR-V2-STATA18-19",
    "19": "KSS-MT64S-DOMAIN-CURSOR-V2-STATA18-19",
}
EXPECTED_STAGES = (
    "import_selection",
    "compression_transition",
    "numerical_computation",
    "restoration",
)
RHS_FIELDS = [
    "experiment_id", "stage", "batch_start", "rhs", "iterations",
    "relative_residual", "converged",
]
STAGE_FIELDS = [
    "stage", "elapsed_seconds", "forecast_peak_bytes",
    "observed_allocation_bytes", "measurement_kind",
]
PHASE_TOKENS = (
    "import_selection",
    "compression_transition",
    "numerical",
    "restoration",
)
PHASE_SAMPLE_FIELDS = ["timestamp_utc_seconds", "phase", "rss_bytes"]
PHASE_PEAK_FIELDS = [
    "phase", "peak_rss_bytes", "sample_count",
    "first_timestamp_utc_seconds", "last_timestamp_utc_seconds",
]
ADMISSION_RECEIPT_VERSION = "KSS-SCALE-ADMISSION-V1"
TMP_CAPACITY_RECEIPT_VERSION = "KSS-SCALE-TMP-CAPACITY-V1"
RSS_SAMPLER_RECEIPT_VERSION = "KSS-SCALE-RSS-SAMPLER-V1"


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


def finite(row: dict[str, str], field: str) -> float:
    try:
        value = float(row[field])
    except (KeyError, TypeError, ValueError) as exc:
        raise ValueError(f"invalid {field}") from exc
    require(math.isfinite(value), f"nonfinite {field}")
    return value


def integer(row: dict[str, str], field: str) -> int:
    value = finite(row, field)
    require(value.is_integer(), f"noninteger {field}")
    return int(value)


def close(left: float, right: float, tolerance: float = 1e-9) -> bool:
    return abs(left - right) <= tolerance * (1 + max(abs(left), abs(right)))


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
        header = next(reader, None)
        require(header == ["key", "value"], f"invalid {label} header")
        for fields in reader:
            require(len(fields) == 2 and fields[0], f"invalid {label} row")
            require(fields[0] not in values, f"duplicate {label} key: {fields[0]}")
            values[fields[0]] = fields[1]
    return values


def parse_memory(value: str) -> int:
    match = re.fullmatch(r"([0-9]+(?:\.[0-9]+)?)([KMGTP]?)", value.strip(), re.I)
    require(match is not None, f"invalid qacct memory value: {value}")
    number = float(match.group(1))
    multiplier = {"": 1, "K": 1024, "M": 1024**2,
                  "G": 1024**3, "T": 1024**4,
                  "P": 1024**5}[match.group(2).upper()]
    return int(math.ceil(number * multiplier))


def parse_duration(value: str) -> float:
    try:
        parsed = float(value)
    except ValueError:
        hour_match = re.fullmatch(
            r"(\d+):(\d{2}):(\d{2}(?:\.\d+)?)", value)
        minute_match = re.fullmatch(r"(\d+):(\d{2}(?:\.\d+)?)", value)
        require(hour_match is not None or minute_match is not None,
                f"invalid qacct duration: {value}")
        if hour_match is not None:
            parsed = (3600 * int(hour_match.group(1)) +
                      60 * int(hour_match.group(2)) +
                      float(hour_match.group(3)))
        else:
            assert minute_match is not None
            parsed = (60 * int(minute_match.group(1)) +
                      float(minute_match.group(2)))
    require(math.isfinite(parsed) and parsed >= 0,
            f"invalid qacct duration: {value}")
    return parsed


def parse_qacct(path: Path) -> dict[str, str]:
    require(path.is_file(), f"missing qacct: {path}")
    values: dict[str, str] = {}
    records = 0
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or set(line) == {"="}:
            if set(line) == {"="}:
                records += 1
            continue
        fields = line.split(None, 1)
        if len(fields) == 2:
            key, value = fields
            require(key not in values, f"ambiguous qacct field: {key}")
            values[key] = value.strip()
    require(records <= 1, "qacct contains multiple records")
    for field in ("jobnumber", "slots", "failed", "exit_status",
                  "ru_wallclock", "cpu", "maxvmem"):
        require(field in values, f"qacct missing {field}")
    return values


def validate_scheduler(
    qacct_path: Path, job_id_file: Path, reservation: dict[str, str],
) -> dict[str, Any]:
    require(job_id_file.is_file(), f"missing job-id receipt: {job_id_file}")
    expected_job = job_id_file.read_text(encoding="utf-8").strip()
    require(JOB_ID.fullmatch(expected_job) is not None, "invalid job-id receipt")
    qacct = parse_qacct(qacct_path)
    require(qacct["jobnumber"] == expected_job, "qacct job number mismatch")
    require(int(qacct["failed"]) == 0, "SGE failed is nonzero")
    require(int(qacct["exit_status"]) == 0, "SGE exit_status is nonzero")
    requested_slots = int(reservation["requested_slots"])
    require(int(qacct["slots"]) == requested_slots, "qacct slot count mismatch")
    wall_seconds = parse_duration(qacct["ru_wallclock"])
    cpu_seconds = parse_duration(qacct["cpu"])
    hard_wall = int(reservation["hard_wall_seconds"])
    require(wall_seconds <= hard_wall + 2, "qacct wall exceeds hard request")
    maxvmem_bytes = parse_memory(qacct["maxvmem"])
    reserved_bytes = int(reservation["total_reserved_gib"]) * GIB
    require(maxvmem_bytes <= reserved_bytes,
            "qacct maxvmem exceeds reserved memory")
    return {
        "job_id": expected_job,
        "slots": requested_slots,
        "wall_seconds": wall_seconds,
        "cpu_seconds": cpu_seconds,
        "maxvmem_bytes": maxvmem_bytes,
        "hostname": qacct.get("hostname", ""),
        "queue": qacct.get("qname", ""),
    }


def validate_application(
    *, wrapper_pass: Path, stata_pass: Path, application_log: Path,
    experiment_id: str, source_commit: str, bundle_sha: str, input_sha: str,
) -> None:
    expected_wrapper = (
        f"KSS_SCALE_WRAPPER_PASS {experiment_id} {bundle_sha} "
        f"{source_commit} {input_sha}"
    )
    expected_stata = (
        f"KSS_SCALE_ESTIMATOR_PASS {experiment_id} {bundle_sha} "
        f"{source_commit} {input_sha}"
    )
    require(wrapper_pass.is_file(), "missing wrapper pass marker")
    require(stata_pass.is_file(), "missing Stata pass marker")
    require(wrapper_pass.read_text(encoding="utf-8").strip() == expected_wrapper,
            "wrapper pass marker mismatch")
    require(stata_pass.read_text(encoding="utf-8").strip() == expected_stata,
            "Stata pass marker mismatch")
    for path in wrapper_pass.parent.glob("*.fail"):
        raise ValueError(f"failure marker present: {path.name}")
    require(application_log.is_file(), "missing Stata application log")
    log = application_log.read_text(encoding="utf-8", errors="replace")
    require(f"KSS-SCALE ESTIMATOR PASS: {experiment_id}" in log,
            "application success signal missing")
    require(FATAL_STATA.search(log) is None, "fatal Stata return code in log")


def validate_identity(row: dict[str, str], prefix: str) -> None:
    total = finite(row, f"{prefix}_total")
    parts = finite(row, f"{prefix}_worker") + finite(row, f"{prefix}_firm")
    parts += 2 * finite(row, f"{prefix}_covariance")
    require(close(total, parts, 1e-7), f"{prefix} accounting identity failed")


def validate_scale_fixture(row: dict[str, str]) -> None:
    """Validate the independent connected/deletion-safe fixture certificate."""
    fixture = row.get("fixture", "")
    scale = integer(row, "scale_factor")
    require(scale in {1, 2, 4, 8, 16}, "invalid scale factor")
    require(fixture in {"well_connected", "ring"} or scale == 1,
            "nonreplication fixture has a false scale label")
    require(fixture != "ring" or scale == 2,
            "ring fixture must use scale factor 2")
    if fixture not in {"well_connected", "ring"} or scale == 1:
        return

    base_fields = (
        "fixture_base_rows", "fixture_base_physical",
        "fixture_base_workers", "fixture_base_firms",
        "fixture_base_cells", "fixture_base_units",
    )
    expected_fields = (
        "fixture_expected_rows", "fixture_expected_physical",
        "fixture_expected_workers", "fixture_expected_firms",
        "fixture_expected_cells", "fixture_expected_units",
    )
    base = [integer(row, field) for field in base_fields]
    expected = [integer(row, field) for field in expected_fields]
    require(min(base) > 0, "invalid base fixture dimensions")
    require(min(expected) > 0, "invalid constructed fixture dimensions")
    pairs = (scale * (scale - 1) // 2 if fixture == "well_connected"
             else (1 if scale == 2 else scale))
    connector_rows = 4 * pairs
    require(integer(row, "fixture_connector_rows") == connector_rows,
            "fixture connector count disagrees with construction")
    calculated = [
        scale * base[0] + connector_rows,
        scale * base[1] + connector_rows,
        scale * base[2] + 2 * pairs,
        scale * base[3],
        scale * base[4] + connector_rows,
        scale * base[5] + connector_rows,
    ]
    require(expected == calculated,
            "fixture dimensions disagree with construction arithmetic")
    require(integer(row, "input_rows") == expected[0],
            "constructed fixture row count disagrees with certificate")
    require(integer(row, "N_retained") == expected[0],
            "deletion-safe fixture lost retained rows")
    require(integer(row, "N_physical") == expected[1],
            "fixture physical count disagrees with certificate")
    require(integer(row, "worker_levels") == expected[2] and
            integer(row, "firm_levels") == expected[3],
            "fixture FE dimensions disagree with certificate")
    require(integer(row, "diagnostic_coefficient_cells") == expected[4] and
            integer(row, "diagnostic_deletion_units") == expected[5],
            "fixture cell or deletion-unit count disagrees with certificate")

    conductance = finite(row, "fixture_conductance")
    lambda2 = finite(row, "fixture_lambda2")
    lambda_max = finite(row, "fixture_lambda_max")
    condition = finite(row, "fixture_condition_proxy")
    minimum_degree = finite(row, "fixture_min_degree")
    maximum_degree = finite(row, "fixture_max_degree")
    require(0 < conductance <= 1, "invalid fixture conductance proxy")
    require(0 < lambda2 <= lambda_max <= 2 * (1 + 1e-12),
            "invalid fixture normalized-Laplacian spectrum")
    require(condition >= 1 and close(condition, lambda_max / lambda2, 1e-10),
            "invalid fixture condition proxy")
    require(0 < minimum_degree <= maximum_degree,
            "invalid fixture weighted-degree bounds")


def validate_rng_contract(row: dict[str, str], probes: int) -> None:
    """Require the registered path-independent logical probe contract."""
    contract = row.get("rng_contract", "")
    runtime = row.get("rng_runtime", "")
    require(contract == REGISTERED_RNG_CONTRACTS.get(runtime),
            "missing or invalid RNG contract")
    if row.get("engine_selected") == "compressed":
        require(row.get("rng_implementation") == "per_domain_stream_cursor",
                "unregistered RNG implementation for selected engine")
    else:
        require(row.get("rng_implementation") in {
            "per_domain_stream_physical_copies",
            "per_domain_stream_semantic_atoms",
        }, "unregistered RNG implementation for selected engine")
    require(runtime == row.get("stata_version"),
            "RNG runtime does not match the executing Stata runtime")
    require(integer(row, "rng_master_seed") == integer(row, "seed"),
            "RNG master seed changed")
    require(row.get("rng_leverage_domain") == "leverage" and
            row.get("rng_target_domain") == "target",
            "RNG domains are missing or not separate")
    require(integer(row, "rng_leverage_probe_first") == 1 and
            integer(row, "rng_leverage_probe_last") == probes and
            integer(row, "rng_target_probe_first") == 1 and
            integer(row, "rng_target_probe_last") == probes,
            "RNG logical probe ranges changed")


def validate_resource_forecast(
    row: dict[str, str], reservation: dict[str, str],
) -> None:
    """Validate overlap arithmetic and pre-RNG admission for either route."""
    engine = row.get("engine_selected")
    require(engine in {"compressed", "generic"},
            "unrecognized estimator engine")
    require(row.get("resource_status") == "ADMITTED",
            "route lacks a successful pre-RNG resource admission")
    if engine == "generic":
        require(row.get("fastpath_status") not in
                {"", "NOT_REPORTED", "ELIGIBLE"},
                "generic fallback lacks a typed fast-path rejection")

    component_fields = (
        "resource_raw_stata_bytes",
        "resource_cell_bytes",
        "resource_deletion_unit_bytes",
        "resource_target_stratum_bytes",
        "resource_cmg_hierarchy_bytes",
        "resource_phase_scratch_bytes",
        "resource_sort_compress_bytes",
        "resource_solve_ahead_bytes",
        "resource_output_cert_bytes",
        "resource_preserve_bytes",
    )
    components = [finite(row, field) for field in component_fields]
    require(components[0] > 0 and min(components) >= 0,
            "invalid resource component forecast")
    raw, cell, deletion, strata, cmg, scratch, sorting, solve_ahead, output, \
        preservation = components
    persistent = cell + deletion + strata
    expected_phases = [
        raw + sorting + output,
        raw + persistent + sorting + preservation + output,
        (raw if engine == "generic" else 0) + persistent + cmg +
        scratch + solve_ahead + output,
        raw + preservation + output,
    ]
    phase_fields = (
        "resource_selection_peak_bytes",
        "resource_transition_peak_bytes",
        "resource_numerical_peak_bytes",
        "resource_restoration_peak_bytes",
    )
    phase_forecasts = [finite(row, field) for field in phase_fields]
    require(all(close(actual, expected, 1e-12)
                for actual, expected in
                zip(phase_forecasts, expected_phases, strict=True)),
            "resource phase overlap arithmetic failed")
    peak = finite(row, "resource_peak_bytes")
    require(peak > 0 and close(peak, max(phase_forecasts), 1e-12),
            "resource peak is not the maximum overlapping phase")
    expected_peak_phase = (
        "selection", "transition", "numerical", "restoration"
    )[phase_forecasts.index(max(phase_forecasts))]
    require(row.get("resource_peak_phase") == expected_peak_phase,
            "resource peak phase changed")

    memory_headroom = finite(row, "resource_mem_headroom")
    memory_admission = finite(row, "resource_mem_admit_bytes")
    hard_memory = finite(row, "resource_hard_mem_bytes")
    require(0.25 <= memory_headroom <= 0.30,
            "memory headroom is outside the registered range")
    require(close(memory_admission, math.ceil(peak * (1 + memory_headroom)),
                  1e-12),
            "memory admission does not include registered headroom")
    require(memory_admission <= hard_memory <=
            int(reservation["total_reserved_gib"]) * GIB,
            "route memory admission failed")

    wall_upper = finite(row, "resource_wall_upper_seconds")
    wall_headroom = finite(row, "resource_wall_headroom")
    wall_admission = finite(row, "resource_wall_admit_seconds")
    hard_wall = finite(row, "resource_hard_wall_seconds")
    require(close(wall_headroom, 0.50, 1e-12),
            "wall headroom changed")
    require(wall_upper > 0 and
            close(wall_admission, math.ceil(wall_upper * 1.5), 1e-12),
            "wall admission does not include registered headroom")
    require(wall_admission <= hard_wall <=
            int(reservation["hard_wall_seconds"]),
            "route wall admission failed")
    require(hard_wall == int(reservation["estimator_hard_wall_seconds"]),
            "estimator hard-wall receipt changed")


def validate_summary(
    path: Path, *, experiment_id: str, source_commit: str, bundle_sha: str,
    input_sha: str, probes: int, reservation: dict[str, str],
) -> dict[str, str]:
    row = read_one_csv(path, "scale summary")
    require(row.get("experiment_id") == experiment_id, "wrong experiment ID")
    require(row.get("source_commit") == source_commit, "wrong source commit")
    require(row.get("bundle_sha256") == bundle_sha, "wrong bundle hash")
    require(row.get("input_sha256") == input_sha, "wrong input hash")
    require(row.get("fixture") == reservation.get("fixture") and
            integer(row, "scale_factor") ==
            int(reservation["scale_factor"]),
            "summary and reservation scale binding changed")
    load_seconds = finite(row, "load_seconds")
    fixture_seconds = finite(row, "fixture_construction_seconds")
    import_selection_seconds = finite(row, "import_selection_seconds")
    require(load_seconds >= 0 and fixture_seconds >= 0 and
            import_selection_seconds + 1e-9 >=
            load_seconds + fixture_seconds,
            "invalid import-selection timing accounting")
    constructs_fixture = (
        row.get("fixture") in {"well_connected", "ring"} and
        integer(row, "scale_factor") >= 2
    )
    require(constructs_fixture or fixture_seconds == 0,
            "unscaled input reports fixture-construction time")
    require(integer(row, "command_rc") == 0, "estimator command failed")
    engine = row.get("engine_selected")
    expected_status = (
        "KSS_SCALE_EXPERIMENTAL_POINT_ESTIMATES"
        if engine == "compressed"
        else "KSS_POINT_ESTIMATES_ONLY"
    )
    require(row.get("estimator_status") == expected_status,
            "estimator status does not match selected engine")
    require(row.get("engine_requested") == "auto", "scale engine was not automatic")
    require(integer(row, "requested_probes") == probes, "probe count changed")
    requested_slots = int(reservation["requested_slots"])
    stata_processors = int(reservation["stata_processors"])
    require(integer(row, "requested_slots") == requested_slots and
            integer(row, "actual_slots") == requested_slots,
            "scheduler slot receipt changed")
    require(integer(row, "requested_stata_processors") == stata_processors and
            integer(row, "actual_stata_processors") == stata_processors == 4,
            "Stata processor receipt changed")
    require(integer(row, "stata_mp") == 1, "Stata/MP was not used")
    require(requested_slots >= stata_processors,
            "Stata processor use exceeds scheduler reservation")
    require(integer(row, "declared_memory_gib") ==
            int(reservation["total_reserved_gib"]),
            "declared memory differs from scheduler reservation")
    for field in ("input_rows", "N_retained", "worker_levels", "firm_levels",
                  "deletion_units"):
        require(integer(row, field) > 0, f"invalid dimension: {field}")
    require(integer(row, "sample_semantics_valid") == 1,
            "restored e(sample) certificate failed")
    validate_scale_fixture(row)
    validate_rng_contract(row, probes)
    validate_resource_forecast(row, reservation)
    validate_identity(row, "plugin")
    validate_identity(row, "correction")
    validate_identity(row, "corrected")
    for target in ("worker", "firm", "covariance", "total"):
        expected = (finite(row, f"plugin_{target}") -
                    finite(row, f"correction_{target}"))
        require(close(finite(row, f"corrected_{target}"), expected, 1e-7),
                f"corrected {target} decomposition failed")
    tolerance = finite(row, "tolerance")
    residual_gate = max(1e-11, 10 * tolerance)
    require(row.get("residual_normalization") ==
            "l2_rhs_or_absolute_zero_rhs",
            "complete residual normalization changed")
    require(row.get("residual_equation_contract") ==
            "original_rhs_worker_firm_v1",
            "complete original-equation residual contract changed")
    require(row.get("quotient_convention") == "full_firm_zero_sum",
            "solver quotient convention changed")
    require(row.get("grounding_convention") ==
            "last_firm_zero_after_quotient_with_grounded_equation_checked",
            "display grounding convention changed")
    require(row.get("grounded_coordinate_handling") ==
            "included_in_full_residual",
            "grounded coordinate was omitted from the full residual")
    require(close(finite(row, "residual_acceptance_tolerance"),
                  residual_gate, 1e-12),
            "complete residual tolerance changed")
    residual = finite(row, "solver_max_residual")
    rhs_residual = finite(row, "rhs_max_residual")
    require(0 <= residual <= residual_gate * (1 + 1e-10),
            "aggregate complete residual failed")
    require(0 <= rhs_residual <= residual_gate * (1 + 1e-10),
            "reported RHS complete residual failed")
    require(rhs_residual <= residual,
            "reported RHS residual exceeds aggregate solver residual")

    if row.get("engine_selected") == "compressed":
        for field in ("coefficient_cells", "target_strata"):
            require(integer(row, field) > 0,
                    f"compressed diagnostic missing: {field}")
        require(integer(row, "diagnostic_coefficient_cells") ==
                integer(row, "coefficient_cells"),
                "independent coefficient-cell diagnostic disagrees")
        require(integer(row, "diagnostic_deletion_units") ==
                integer(row, "deletion_units"),
                "independent deletion-unit diagnostic disagrees")
        require(row.get("lifecycle_method") == "PRESERVE_DISK",
                "compressed lifecycle method is not standard Stata")
        require(integer(row, "life_sample_restored") == 1,
                "compressed lifecycle did not restore the sample")
        require(finite(row, "rng_seconds") >= 0 and
                finite(row, "correction_seconds") >= 0,
                "compressed nested timing attribution is missing")
        require(row.get("timing_attribution_contract") ==
                "rng_and_correction_nested_nonadditive",
                "compressed timing attribution contract changed")
    return row


def validate_rhs(
    path: Path, *, experiment_id: str, probes: int, tolerance: float,
    rhs_max_residual: float,
) -> dict[str, Any]:
    require(path.is_file(), f"missing RHS certificate: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        require(reader.fieldnames == RHS_FIELDS, "RHS certificate schema changed")
        rows = list(reader)
    require(len(rows) == 3 * probes + 1, "RHS certificate count changed")
    keys: set[tuple[str, str, str]] = set()
    residuals: list[float] = []
    stage_counts = {stage: 0 for stage in range(1, 6)}
    logical_rhs = {stage: set() for stage in range(1, 6)}
    gate = max(1e-11, 10 * tolerance)
    for row in rows:
        require(row["experiment_id"] == experiment_id,
                "RHS experiment ID changed")
        key = (row["stage"], row["batch_start"], row["rhs"])
        require(key not in keys, "duplicate RHS certificate")
        keys.add(key)
        stage = integer(row, "stage")
        batch_start = integer(row, "batch_start")
        require(stage in stage_counts, "invalid RHS stage")
        require((batch_start >= 1 if stage in {4, 5} else batch_start == 0),
                "invalid RHS batch marker")
        require(integer(row, "rhs") >= 1, "invalid RHS index")
        logical_index = integer(row, "rhs")
        require(logical_index not in logical_rhs[stage],
                "duplicate logical RHS certificate")
        logical_rhs[stage].add(logical_index)
        stage_counts[stage] += 1
        require(integer(row, "iterations") >= 0 and
                integer(row, "converged") == 1,
                "unaccepted RHS certificate")
        residual = finite(row, "relative_residual")
        require(0 <= residual <= gate * (1 + 1e-10),
                "complete RHS residual failed")
        residuals.append(residual)
    maximum = max(residuals)
    require(sum(stage_counts[stage] for stage in (1, 2, 3)) == 1,
            "fit RHS certificate count changed")
    fit_stages = [stage for stage in (1, 2, 3) if stage_counts[stage]]
    require(logical_rhs[fit_stages[0]] == {1}, "fit RHS index changed")
    require(stage_counts[4] == probes, "leverage RHS certificate count changed")
    require(stage_counts[5] == 2 * probes,
            "target RHS certificate count changed")
    require(logical_rhs[4] == set(range(1, probes + 1)),
            "leverage RHS index coverage changed")
    require(logical_rhs[5] == set(range(1, 2 * probes + 1)),
            "target RHS index coverage changed")
    receipt_rounding_tolerance = max(
        1e-30, 1e-10 * max(abs(maximum), abs(rhs_max_residual))
    )
    require(abs(maximum - rhs_max_residual) <= receipt_rounding_tolerance,
            "summary and per-RHS residuals disagree")
    return {"count": len(rows), "maximum_relative_residual": maximum}


def validate_stage_memory(
    path: Path, *, row: dict[str, str], compressed: bool,
) -> list[dict[str, str]]:
    require(path.is_file(), f"missing stage-memory receipt: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        require(reader.fieldnames == STAGE_FIELDS,
                "stage-memory receipt schema changed")
        rows = list(reader)
    require(tuple(row["stage"] for row in rows) == EXPECTED_STAGES,
            "stage-memory rows changed")
    summary_forecasts = [
        finite(row, field) for field in (
            "resource_selection_peak_bytes",
            "resource_transition_peak_bytes",
            "resource_numerical_peak_bytes",
            "resource_restoration_peak_bytes",
        )
    ]
    for receipt, summary_forecast in zip(
        rows, summary_forecasts, strict=True,
    ):
        elapsed = finite(receipt, "elapsed_seconds")
        require(elapsed >= 0, "negative stage elapsed time")
        forecast_text = receipt["forecast_peak_bytes"].strip()
        observed_text = receipt["observed_allocation_bytes"].strip()
        require(forecast_text not in {"", "."} and
                close(float(forecast_text), summary_forecast, 1e-12),
                "stage and summary forecasts disagree")
        require(receipt["measurement_kind"] ==
                "stata_allocated_endpoint_not_peak",
                "stage memory measurement kind changed")
        if compressed:
            require(observed_text not in {"", "."},
                    "compressed stage memory endpoint diagnostic missing")
            require(float(forecast_text) >= 0 and float(observed_text) >= 0,
                    "negative stage memory diagnostic")
    require(close(float(rows[0]["elapsed_seconds"]),
                  finite(row, "import_selection_seconds"), 1e-10),
            "import-selection stage timing disagrees with summary")
    return rows


def validate_phase_rss(
    samples_path: Path, peaks_path: Path, sampler_path: Path,
) -> dict[str, Any]:
    """Validate process-tree RSS samples and independently rebuild peaks."""
    require(samples_path.is_file(), f"missing phase RSS samples: {samples_path}")
    with samples_path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        require(reader.fieldnames == PHASE_SAMPLE_FIELDS,
                "phase RSS sample schema changed")
        samples = list(reader)
    require(samples, "phase RSS sample receipt is empty")
    sampler = read_key_values(sampler_path, "phase RSS sampler receipt")
    require(sampler.get("receipt_version") == RSS_SAMPLER_RECEIPT_VERSION,
            "phase RSS sampler receipt version changed")
    poll_count = int(sampler["poll_count"])
    valid_sample_count = int(sampler["valid_sample_count"])
    invalid_marker_reads = int(sampler["invalid_marker_read_count"])
    zero_rss_reads = int(sampler["zero_rss_read_count"])
    interval = float(sampler["sampler_interval_seconds"])
    require(poll_count >= 1 and valid_sample_count == len(samples) and
            poll_count == valid_sample_count + zero_rss_reads and
            0 <= invalid_marker_reads <= poll_count and
            close(interval, 0.25, 1e-12),
            "phase RSS sampler accounting changed")

    observed: dict[str, list[tuple[int, int]]] = {
        phase: [] for phase in PHASE_TOKENS
    }
    previous_timestamp = -1
    previous_phase = -1
    phase_order = {phase: index for index, phase in enumerate(PHASE_TOKENS)}
    for sample in samples:
        phase = sample.get("phase", "")
        require(phase in phase_order, "unregistered phase RSS token")
        timestamp = integer(sample, "timestamp_utc_seconds")
        rss_bytes = integer(sample, "rss_bytes")
        require(timestamp >= previous_timestamp and rss_bytes > 0,
                "invalid phase RSS sample")
        require(phase_order[phase] >= previous_phase,
                "phase RSS tokens moved backward")
        previous_timestamp = timestamp
        previous_phase = phase_order[phase]
        observed[phase].append((timestamp, rss_bytes))

    require(peaks_path.is_file(), f"missing phase RSS peaks: {peaks_path}")
    with peaks_path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        require(reader.fieldnames == PHASE_PEAK_FIELDS,
                "phase RSS peak schema changed")
        peak_rows = list(reader)
    require(tuple(row.get("phase", "") for row in peak_rows) == PHASE_TOKENS,
            "phase RSS peak rows changed")

    phase_peaks: dict[str, int | None] = {}
    phase_counts: dict[str, int] = {}
    for receipt in peak_rows:
        phase = receipt["phase"]
        values = observed[phase]
        count = integer(receipt, "sample_count")
        require(count == len(values), "phase RSS sample count disagrees")
        phase_counts[phase] = count
        if not values:
            require(receipt["peak_rss_bytes"].strip() in {"", "."} and
                    receipt["first_timestamp_utc_seconds"].strip() in
                    {"", "."} and
                    receipt["last_timestamp_utc_seconds"].strip() in
                    {"", "."},
                    "unmeasured phase RSS peak must be explicitly missing")
            phase_peaks[phase] = None
            continue
        expected_peak = max(value[1] for value in values)
        expected_first = values[0][0]
        expected_last = values[-1][0]
        require(integer(receipt, "peak_rss_bytes") == expected_peak and
                integer(receipt, "first_timestamp_utc_seconds") ==
                expected_first and
                integer(receipt, "last_timestamp_utc_seconds") == expected_last,
                "phase RSS peak receipt disagrees with samples")
        phase_peaks[phase] = expected_peak

    return {
        "sample_count": len(samples),
        "poll_count": poll_count,
        "invalid_marker_read_count": invalid_marker_reads,
        "zero_rss_read_count": zero_rss_reads,
        "sampler_interval_seconds": interval,
        "phase_peaks_bytes": phase_peaks,
        "phase_sample_counts": phase_counts,
        "phase_peak_complete": all(
            phase_peaks[phase] is not None for phase in PHASE_TOKENS
        ),
        "measurement_kind": "sampled_process_tree_rss_peak",
    }


def validate_tmp_capacity(
    path: Path, *, source_commit: str, bundle_sha: str, input_sha: str,
    fixture: str, scale_factor: int,
) -> dict[str, Any]:
    """Validate the node-local expanded-data and preserve-spool forecast."""
    values = read_key_values(path, "TMPDIR capacity receipt")
    require(values.get("receipt_version") == TMP_CAPACITY_RECEIPT_VERSION,
            "TMPDIR capacity receipt version changed")
    require(values.get("source_commit") == source_commit and
            values.get("bundle_sha256") == bundle_sha and
            values.get("input_sha256") == input_sha,
            "TMPDIR capacity receipt source binding changed")
    require(values.get("fixture") == fixture and
            int(values["scale_factor"]) == scale_factor,
            "TMPDIR capacity fixture binding changed")
    input_bytes = int(values["staged_input_bytes"])
    expansion_factor = int(values["fixture_expansion_factor"])
    expanded = int(values["expanded_caller_forecast_bytes"])
    preservation = int(values["preserve_spool_forecast_bytes"])
    sorting = int(values["sort_temp_forecast_bytes"])
    overhead = int(values["fixed_output_temp_overhead_bytes"])
    required = int(values["required_bytes"])
    available = int(values["available_bytes"])
    expected_factor = (
        scale_factor
        if fixture in {"well_connected", "ring"} and scale_factor >= 2
        else 1
    )
    require(input_bytes > 0 and expansion_factor == expected_factor,
            "TMPDIR fixture expansion factor changed")
    require(expanded == 2 * input_bytes * expansion_factor,
            "TMPDIR expanded-caller forecast changed")
    require(preservation == expanded and sorting == expanded and
            overhead == 4 * GIB,
            "TMPDIR spool or overhead forecast changed")
    require(required == input_bytes + expanded + preservation + sorting +
            overhead,
            "TMPDIR capacity arithmetic failed")
    require(values.get("status") == "ADMITTED" and available >= required,
            "TMPDIR capacity was not admitted")
    return {
        "status": values["status"],
        "staged_input_bytes": input_bytes,
        "fixture_expansion_factor": expansion_factor,
        "expanded_caller_forecast_bytes": expanded,
        "preserve_spool_forecast_bytes": preservation,
        "sort_temp_forecast_bytes": sorting,
        "fixed_output_temp_overhead_bytes": overhead,
        "required_bytes": required,
        "available_bytes": available,
        "available_over_required": available / required,
    }


def validate_wrapper_metrics(
    path: Path, *, scheduler_wall: float, source_commit: str,
    input_sha: str,
) -> dict[str, str]:
    values = read_key_values(path, "wrapper metrics")
    require(values.get("source_commit") == source_commit,
            "wrapper metric source mismatch")
    require(values.get("input_sha256") == input_sha,
            "wrapper metric input mismatch")
    components = [
        float(values[field]) for field in (
            "input_staging_seconds", "stata_process_seconds",
            "output_validation_seconds",
        )
    ]
    total = float(values["total_wrapper_seconds"])
    require(all(math.isfinite(value) and value >= 0 for value in components) and
            math.isfinite(total) and total >= sum(components),
            "invalid wrapper timing receipt")
    require(total <= scheduler_wall + 2, "wrapper wall exceeds qacct wall")
    return values


def validate_process_resources(
    path: Path, *, reserved_bytes: int, scheduler_wall: float,
) -> dict[str, float]:
    require(path.is_file(), f"missing process resource receipt: {path}")
    text = path.read_text(encoding="utf-8", errors="replace")
    require("stata-mp" in text, "resource receipt did not time stata-mp")
    def field(pattern: str, label: str) -> str:
        matches = re.findall(pattern, text, flags=re.MULTILINE)
        require(len(matches) == 1, f"resource receipt missing/duplicate {label}")
        return matches[0]
    exit_status = int(field(r"^\s*Exit status:\s*([0-9]+)\s*$", "exit status"))
    require(exit_status == 0, "timed Stata process failed")
    rss_kib = int(field(
        r"^\s*Maximum resident set size \(kbytes\):\s*([0-9]+)\s*$",
        "maximum RSS"))
    elapsed = parse_duration(field(
        r"^\s*Elapsed \(wall clock\) time \(h:mm:ss or m:ss\):\s*(\S+)\s*$",
        "elapsed wall"))
    user = float(field(r"^\s*User time \(seconds\):\s*(\S+)\s*$", "user time"))
    system = float(field(
        r"^\s*System time \(seconds\):\s*(\S+)\s*$", "system time"))
    require(min(user, system) >= 0 and
            all(math.isfinite(value) for value in (user, system)),
            "invalid timed CPU receipt")
    require(rss_kib * 1024 <= reserved_bytes,
            "timed process RSS exceeds reserved memory")
    require(elapsed <= scheduler_wall + 2,
            "timed process wall exceeds qacct wall")
    return {
        "elapsed_seconds": elapsed,
        "cpu_seconds": user + system,
        "peak_rss_bytes": rss_kib * 1024,
    }


def reconcile_resources(
    row: dict[str, str], stages: list[dict[str, str]],
    phase_rss: dict[str, Any], scheduler: dict[str, Any],
    process: dict[str, float],
) -> dict[str, Any]:
    phase_forecast = [float(stage["forecast_peak_bytes"]) for stage in stages]
    phase_endpoints: list[float | None] = []
    for stage in stages:
        value = stage["observed_allocation_bytes"].strip()
        phase_endpoints.append(None if value in {"", "."} else float(value))
    process_peak = float(process["peak_rss_bytes"])
    qacct_peak = float(scheduler["maxvmem_bytes"])
    endpoint_values = [
        value for value in phase_endpoints if value is not None
    ]
    measured_phase_peaks = [
        phase_rss["phase_peaks_bytes"][phase] for phase in PHASE_TOKENS
    ]
    available_phase_peaks = [
        float(value) for value in measured_phase_peaks if value is not None
    ]
    overall_observed_peak = max(process_peak, qacct_peak)
    comparison_peak = max(
        (*endpoint_values, *available_phase_peaks, overall_observed_peak)
    )
    registered_peak = finite(row, "resource_peak_bytes")
    actual_wall = float(scheduler["wall_seconds"])
    wall_upper = finite(row, "resource_wall_upper_seconds")
    phase_endpoint_complete = len(endpoint_values) == len(phase_forecast)
    endpoints_within_forecasts = all(
        actual is None or actual <= forecast
        for actual, forecast in
        zip(phase_endpoints, phase_forecast, strict=True)
    )
    phase_peaks_within_forecasts = all(
        actual is not None and actual <= forecast
        for actual, forecast in
        zip(measured_phase_peaks, phase_forecast, strict=True)
    )
    phase_peak_complete = bool(phase_rss["phase_peak_complete"])
    within_peak = comparison_peak <= registered_peak
    within_wall = actual_wall <= wall_upper
    within_hard = (
        comparison_peak <= finite(row, "resource_hard_mem_bytes") and
        actual_wall <= finite(row, "resource_hard_wall_seconds")
    )
    compressed_route = row.get("engine_selected") == "compressed"
    fixture = row.get("fixture", "")
    scale_factor = integer(row, "scale_factor")
    next_rung_projection_required = (
        fixture == "well_connected" and scale_factor >= 4
    )
    registered_successor = (
        fixture == "cz18" or
        (fixture == "well_connected" and scale_factor == 2)
    )
    next_scale = (
        compressed_route and phase_peak_complete and
        endpoints_within_forecasts and
        phase_peaks_within_forecasts and within_peak and within_wall and
        within_hard and registered_successor and
        not next_rung_projection_required
    )
    if next_scale:
        status = "RECONCILED"
    elif not within_hard:
        status = "ACTUAL_RESOURCE_LIMIT_EXCEEDED"
    elif not compressed_route:
        status = "GENERIC_ROUTE_NOT_SCALE_QUALIFYING"
    elif not phase_peak_complete:
        status = "PHASE_PEAK_EVIDENCE_INCOMPLETE"
    elif not (endpoints_within_forecasts and
              phase_peaks_within_forecasts and within_peak and within_wall):
        status = "FORECAST_UNDERESTIMATED"
    elif next_rung_projection_required:
        status = "SCALE_PROJECTION_REQUIRED"
    else:
        status = "NO_REGISTERED_SUCCESSOR"
    return {
        "status": status,
        "route": row.get("engine_selected", ""),
        "phase_forecast_bytes": phase_forecast,
        "phase_observed_allocation_endpoint_bytes": phase_endpoints,
        "phase_endpoint_complete": phase_endpoint_complete,
        "phase_observed_peak_rss_bytes": measured_phase_peaks,
        "phase_peak_sample_counts": [
            phase_rss["phase_sample_counts"][phase]
            for phase in PHASE_TOKENS
        ],
        "phase_peak_measurement_available": phase_peak_complete,
        "phase_peak_measurement_kind": phase_rss["measurement_kind"],
        "process_peak_rss_bytes": int(process_peak),
        "qacct_maxvmem_bytes": int(qacct_peak),
        "overall_observed_peak_rss_bytes": int(overall_observed_peak),
        "comparison_peak_bytes": int(comparison_peak),
        "memory_forecast_ratio": comparison_peak / registered_peak,
        "actual_wall_seconds": actual_wall,
        "wall_forecast_ratio": actual_wall / wall_upper,
        "within_phase_endpoint_lower_bounds": endpoints_within_forecasts,
        "within_phase_peak_forecasts": phase_peaks_within_forecasts,
        "within_peak_forecast": within_peak,
        "within_wall_forecast": within_wall,
        "within_hard_limits": within_hard,
        "registered_successor": registered_successor,
        "next_rung_projection_required": next_rung_projection_required,
        "next_scale_allowed": next_scale,
    }


def validate_reservation(
    path: Path, *, experiment_id: str, source_commit: str,
    bundle_sha: str, input_sha: str,
) -> dict[str, str]:
    values = read_key_values(path, "reservation")
    require(values.get("experiment_id") == experiment_id,
            "reservation experiment mismatch")
    require(values.get("source_commit") == source_commit,
            "reservation source mismatch")
    require(values.get("bundle_sha256") == bundle_sha,
            "reservation bundle mismatch")
    require(values.get("input_sha256") == input_sha,
            "reservation input mismatch")
    slots = int(values["requested_slots"])
    memory_per_core = int(values["mem_per_core_gib"])
    total = int(values["total_reserved_gib"])
    require(slots * memory_per_core == total,
            "reservation memory arithmetic failed")
    require(total <= 56, "reservation exceeds 56 GiB")
    require(int(values["stata_processors"]) == 4,
            "scale jobs require four Stata processors")
    require(300 <= int(values["hard_wall_seconds"]) <= 43200,
            "invalid hard wall request")
    scheduler_wall = int(values["hard_wall_seconds"])
    reserve = min(120, scheduler_wall - 300)
    require(int(values["estimator_hard_wall_seconds"]) ==
            scheduler_wall - reserve,
            "invalid estimator hard-wall reservation")
    fixture = values.get("fixture", "")
    scale_factor = int(values["scale_factor"])
    require(fixture in {"local", "cz24", "cz25", "cz18",
                        "well_connected", "ring"} and
            scale_factor in {1, 2, 4, 8, 16},
            "invalid reservation fixture or scale")
    require(fixture in {"well_connected", "ring"} or scale_factor == 1,
            "nonreplication reservation has a false scale label")
    require(fixture != "ring" or scale_factor == 2,
            "ring reservation must use scale factor 2")
    require(not (fixture == "well_connected" and scale_factor >= 8),
            "SCALE_PROJECTION_REQUIRED: well-connected 8x/16x SCC runs lack "
            "a registered cold-path projection receipt")
    if fixture == "well_connected" and scale_factor >= 2:
        require(HEX64.fullmatch(
            values.get("prior_admission_sha256", "")) is not None,
            "large-rung reservation lacks prior receipt checksum")
        require(re.fullmatch(
            r"[A-Za-z0-9._-]+",
            values.get("prior_experiment_id", "")) is not None,
            "large-rung reservation lacks prior experiment")
        require(int(values["prior_scale_factor"]) == scale_factor // 2,
                "large-rung reservation has wrong prior scale")
        require(values.get("prior_admission_receipt", "").endswith(
            "/experiments/" + values["prior_experiment_id"] +
            "/admission_receipt.tsv"),
            "large-rung reservation has wrong prior receipt path")
        prior_path = Path(values["prior_admission_receipt"])
        require(prior_path.is_file() and
                sha256(prior_path) == values["prior_admission_sha256"],
                "prior admission receipt checksum changed")
        prior = read_key_values(prior_path, "prior admission receipt")
        expected_prior_fixture = (
            "cz18" if scale_factor == 2 else "well_connected"
        )
        require(prior.get("receipt_version") ==
                ADMISSION_RECEIPT_VERSION and
                prior.get("validation_status") ==
                "KSS_SCALE_VALIDATION_PASS" and
                prior.get("experiment_id") ==
                values["prior_experiment_id"] and
                prior.get("fixture") == expected_prior_fixture and
                int(prior["scale_factor"]) == scale_factor // 2 and
                prior.get("source_commit") == source_commit and
                prior.get("bundle_sha256") == bundle_sha and
                prior.get("input_sha256") == input_sha and
                int(prior["requested_probes"]) == 200 and
                prior.get("engine") == "compressed" and
                prior.get("phase_peak_complete") == "1" and
                prior.get("next_scale_allowed") == "1",
                "prior admission receipt does not unlock this rung")
    else:
        for field in (
            "prior_admission_receipt", "prior_admission_sha256",
            "prior_experiment_id", "prior_scale_factor",
        ):
            require(values.get(field) == "-",
                    "lower rung unexpectedly depends on prior evidence")
    return values


def validate_run(
    *, summary: Path, rhs: Path, stage_memory: Path, qacct: Path,
    phase_rss_samples: Path, phase_rss_peaks: Path,
    phase_rss_sampler: Path, tmp_capacity: Path,
    job_id_file: Path, wrapper_pass: Path, stata_pass: Path,
    application_log: Path, reservation_path: Path, wrapper_metrics: Path,
    process_resources: Path,
    experiment_id: str, source_commit: str, bundle_sha: str,
    input_sha: str, probes: int,
) -> dict[str, Any]:
    require(HEX40.fullmatch(source_commit) is not None, "invalid source commit")
    require(HEX64.fullmatch(bundle_sha) is not None, "invalid bundle hash")
    require(HEX64.fullmatch(input_sha) is not None, "invalid input hash")
    require(re.fullmatch(r"[A-Za-z0-9._-]+", experiment_id) is not None,
            "invalid experiment ID")
    require(probes >= 2, "invalid probe count")
    reservation = validate_reservation(
        reservation_path, experiment_id=experiment_id,
        source_commit=source_commit, bundle_sha=bundle_sha, input_sha=input_sha)
    scheduler = validate_scheduler(qacct, job_id_file, reservation)
    validate_application(
        wrapper_pass=wrapper_pass, stata_pass=stata_pass,
        application_log=application_log, experiment_id=experiment_id,
        source_commit=source_commit, bundle_sha=bundle_sha,
        input_sha=input_sha)
    row = validate_summary(
        summary, experiment_id=experiment_id, source_commit=source_commit,
        bundle_sha=bundle_sha, input_sha=input_sha, probes=probes,
        reservation=reservation)
    rhs_report = validate_rhs(
        rhs, experiment_id=experiment_id, probes=probes,
        tolerance=finite(row, "tolerance"),
        rhs_max_residual=finite(row, "rhs_max_residual"))
    stages = validate_stage_memory(
        stage_memory, row=row,
        compressed=row.get("engine_selected") == "compressed")
    phase_rss = validate_phase_rss(
        phase_rss_samples, phase_rss_peaks, phase_rss_sampler)
    capacity = validate_tmp_capacity(
        tmp_capacity, source_commit=source_commit, bundle_sha=bundle_sha,
        input_sha=input_sha, fixture=row.get("fixture", ""),
        scale_factor=integer(row, "scale_factor"))
    wrapper = validate_wrapper_metrics(
        wrapper_metrics, scheduler_wall=float(scheduler["wall_seconds"]),
        source_commit=source_commit, input_sha=input_sha)
    process = validate_process_resources(
        process_resources,
        reserved_bytes=int(reservation["total_reserved_gib"]) * GIB,
        scheduler_wall=float(scheduler["wall_seconds"]))
    reconciliation = reconcile_resources(
        row, stages, phase_rss, scheduler, process)
    validation_status = (
        "KSS_SCALE_VALIDATION_PASS"
        if row.get("engine_selected") == "compressed"
        else "KSS_SCALE_GENERIC_EVIDENCE_ONLY"
    )
    load_seconds = finite(row, "load_seconds")
    fixture_seconds = finite(row, "fixture_construction_seconds")
    import_selection_seconds = finite(row, "import_selection_seconds")
    return {
        "status": validation_status,
        "experiment_id": experiment_id,
        "source_commit": source_commit,
        "bundle_sha256": bundle_sha,
        "input_sha256": input_sha,
        "fixture": row.get("fixture", ""),
        "scale_factor": integer(row, "scale_factor"),
        "requested_probes": probes,
        "scheduler": scheduler,
        "application": {"status": "PASS"},
        "output": {
            "status": "PASS",
            "engine": row.get("engine_selected", ""),
            "rhs": rhs_report,
            "input_preparation_timing": {
                "load_seconds": load_seconds,
                "fixture_construction_seconds": fixture_seconds,
                "sample_selection_seconds_inferred": max(
                    0.0,
                    import_selection_seconds - load_seconds - fixture_seconds,
                ),
                "import_selection_seconds": import_selection_seconds,
            },
            "stages": stages,
            "phase_rss": phase_rss,
            "tmp_capacity": capacity,
            "wrapper_metrics": wrapper,
            "process_resources": process,
            "resource_reconciliation": reconciliation,
        },
    }


def write_admission_receipt(path: Path, report: dict[str, Any]) -> None:
    """Write the fixed TSV gate consumed by the scalar rung submitter."""
    reconciliation = report["output"]["resource_reconciliation"]
    phase_rss = report["output"]["phase_rss"]
    values = (
        ("receipt_version", ADMISSION_RECEIPT_VERSION),
        ("validation_status", report["status"]),
        ("experiment_id", report["experiment_id"]),
        ("fixture", report["fixture"]),
        ("scale_factor", report["scale_factor"]),
        ("source_commit", report["source_commit"]),
        ("bundle_sha256", report["bundle_sha256"]),
        ("input_sha256", report["input_sha256"]),
        ("requested_probes", report["requested_probes"]),
        ("engine", report["output"]["engine"]),
        ("phase_peak_complete", int(phase_rss["phase_peak_complete"])),
        ("next_scale_allowed", int(reconciliation["next_scale_allowed"])),
    )
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle, delimiter="\t", lineterminator="\n")
        writer.writerow(("key", "value"))
        writer.writerows(values)


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    for name in (
        "summary", "rhs", "stage-memory", "phase-rss-samples",
        "phase-rss-peaks", "phase-rss-sampler", "tmp-capacity", "qacct",
        "job-id-file",
        "wrapper-pass", "stata-pass", "application-log", "reservation",
        "wrapper-metrics", "process-resources",
    ):
        result.add_argument(f"--{name}", type=Path, required=True)
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
            tmp_capacity=args.tmp_capacity,
            job_id_file=args.job_id_file, wrapper_pass=args.wrapper_pass,
            stata_pass=args.stata_pass, application_log=args.application_log,
            reservation_path=args.reservation,
            wrapper_metrics=args.wrapper_metrics,
            process_resources=args.process_resources,
            experiment_id=args.experiment_id,
            source_commit=args.source_commit,
            bundle_sha=args.bundle_sha256, input_sha=args.input_sha256,
            probes=args.probes)
    except (OSError, ValueError) as exc:
        print(f"KSS_SCALE_VALIDATION_FAILURE: {exc}")
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

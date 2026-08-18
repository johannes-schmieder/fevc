#!/usr/bin/env python3
"""Admit the full larger-than-CZ18 stress run from three bound calibrations."""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import os
import re
import statistics
import sys
from pathlib import Path

HEX64 = re.compile(r"[0-9a-f]{64}")
HEX40 = re.compile(r"[0-9a-f]{40}")
JOB_ID = re.compile(r"[0-9]+")
EXPECTED_EXPERIMENTS = tuple(
    f"cz18_stress2x_cal20_r{repetition}" for repetition in (1, 2, 3)
)
STRESS_TIMEOUT_FLOOR_SECONDS = 1800
MAX_MEMORY_BYTES = 60 * 1024**3
DECLARED_MEMORY_GIB = 56
CALIBRATION_PROBES = 20
CALIBRATION_RHS = 3 * CALIBRATION_PROBES + 1
STRESS_PROJECTION_FORMULA = (
    "alpha=max_r(max(0,qacct_wall_r-correction_seconds_r));"
    "beta=max_r(correction_seconds_r/20);"
    "projected=ceil(max(300,1.25*alpha+1.5*200*beta+120));"
    "timeout=max(1800,projected)"
)
HOSTNAME_POLICY = (
    "INDEPENDENT_PARALLEL_QSUB_HOSTNAMES_DESCRIPTIVE_NONCAUSAL"
)

try:
    from .select_prod_calibration import (
        COMPLETE_RESIDUAL_GATE,
        FIXED_HEADROOM_SECONDS,
        FULL_PROBES,
        MARGINAL_SAFETY_FACTOR,
        MAXIMUM_TIMEOUT_SECONDS,
        REQUESTED_TOLERANCE,
        SETUP_SAFETY_FACTOR,
        node_characteristics_pass,
        qacct_pass,
        timing_identity_after_csv,
        timing_identity_diagnostic,
    )
except ImportError:
    from select_prod_calibration import (
        COMPLETE_RESIDUAL_GATE,
        FIXED_HEADROOM_SECONDS,
        FULL_PROBES,
        MARGINAL_SAFETY_FACTOR,
        MAXIMUM_TIMEOUT_SECONDS,
        REQUESTED_TOLERANCE,
        SETUP_SAFETY_FACTOR,
        node_characteristics_pass,
        qacct_pass,
        timing_identity_after_csv,
        timing_identity_diagnostic,
    )


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def finite(row: dict[str, str], field: str) -> float:
    try:
        value = float(row[field])
    except (KeyError, ValueError) as exc:
        raise ValueError(f"invalid {field}") from exc
    require(math.isfinite(value), f"nonfinite {field}")
    return value


def integer(row: dict[str, str], field: str) -> int:
    value = finite(row, field)
    require(value.is_integer(), f"noninteger {field}")
    return int(value)


def close(left: float, right: float, tolerance: float = 1e-10) -> bool:
    return abs(left - right) <= tolerance * (1 + abs(left))


def sha256(path: Path) -> str:
    require(path.is_file(), f"missing input: {path}")
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def read_one(path: Path, label: str) -> dict[str, str]:
    require(path.is_file(), f"missing {label}: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    require(len(rows) == 1, f"{label} must contain exactly one row: {path}")
    return rows[0]


def identity(row: dict[str, str], prefix: str, experiment: str) -> None:
    total = finite(row, f"{prefix}_total")
    parts = finite(row, f"{prefix}_worker") + finite(row, f"{prefix}_firm")
    parts += 2 * finite(row, f"{prefix}_covariance")
    require(close(total, parts, 1e-7),
            f"{experiment}: {prefix} accounting identity failed")


def validate_full_result(
    path: Path, *, source_commit: str, bundle_sha: str, manifest_sha: str,
) -> dict[str, str]:
    row = read_one(path, "CZ18 full result")
    require(row["experiment_id"] == "cz18_full200" and
            row["stage"] == "full" and row["temperature"] == "cold" and
            integer(row, "repetition") == 1,
            "wrong CZ18 full result")
    require(row["source_commit"] == source_commit and
            row["bundle_sha256"] == bundle_sha and
            row["data_manifest_sha256"] == manifest_sha,
            "CZ18 full source identity changed")
    require(row["estimator_status"] == "KSS_POINT_ESTIMATES_ONLY" and
            integer(row, "command_rc") == 0,
            "CZ18 full estimator failed")
    require(row["algorithm_requested"] == row["algorithm_selected"] == "jla" and
            row["preconditioner_selected"] == "cmg" and
            integer(row, "requested_probes") == FULL_PROBES and
            integer(row, "seed") == 8675309 and
            abs(finite(row, "tolerance") - REQUESTED_TOLERANCE) <= 1e-20,
            "CZ18 full estimator semantics changed")
    require(integer(row, "requested_processors") ==
            integer(row, "actual_processors") in {4, 8} and
            integer(row, "declared_memory_gib") == DECLARED_MEMORY_GIB,
            "CZ18 full processor/memory configuration changed")
    require(integer(row, "route_hierarchy_levels") > 1 and
            integer(row, "route_hybrid_vertices") > 6144 and
            1 <= integer(row, "route_terminal_vertices") <= 6144,
            "CZ18 full result did not use bounded multilevel CMG")
    for field in ("N_retained", "worker_levels", "firm_levels",
                  "deletion_units", "route_hybrid_vertices",
                  "route_hybrid_edges"):
        require(integer(row, field) > 0, f"invalid CZ18 full dimension: {field}")
    return row


def validate_rhs(
    path: Path, *, experiment: str,
) -> tuple[tuple[str, ...], float]:
    require(path.is_file(), f"missing RHS evidence: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.DictReader(handle)
        require(reader.fieldnames == [
            "experiment_id", "repetition", "stage", "batch_start", "rhs",
            "iterations", "relative_residual", "converged",
        ], f"RHS evidence schema changed: {experiment}")
        rows = list(reader)
    require(len(rows) == CALIBRATION_RHS,
            f"RHS evidence count changed: {experiment}")
    signature: list[tuple[str, ...]] = []
    residuals: list[float] = []
    keys: set[tuple[str, str, str]] = set()
    for row in rows:
        require(row["experiment_id"] == experiment and
                integer(row, "repetition") == 1,
                f"RHS experiment/repetition changed: {experiment}")
        require(integer(row, "iterations") >= 0 and
                integer(row, "converged") == 1,
                f"unaccepted RHS: {experiment}")
        residual = finite(row, "relative_residual")
        require(0 <= residual <= COMPLETE_RESIDUAL_GATE * (1 + 1e-10),
                f"complete RHS residual failed: {experiment}")
        key = (row["stage"], row["batch_start"], row["rhs"])
        require(key not in keys, f"duplicate RHS certificate: {experiment}")
        keys.add(key)
        signature.append((row["stage"], row["batch_start"], row["rhs"],
                          row["iterations"], row["converged"]))
        residuals.append(residual)
    return tuple(signature), max(residuals)


def validate_calibration(
    *, experiment: str, calibration: Path, qacct: Path, job_id_file: Path,
    rhs: Path, node_characteristics: Path, retained_sha: str,
    source_commit: str, bundle_sha: str, manifest_sha: str,
    full: dict[str, str],
) -> dict[str, object]:
    row = read_one(calibration, "stress calibration")
    require(row["experiment_id"] == experiment and
            row["stage"] == "stress2x" and row["temperature"] == "cold" and
            integer(row, "repetition") == 1,
            f"wrong stress calibration row: {experiment}")
    require(row["bundle_sha256"] == bundle_sha and
            row["source_commit"] == source_commit and
            row["data_manifest_sha256"] == manifest_sha,
            f"stress calibration source identity changed: {experiment}")
    require(row["estimator_input_sha256"] == retained_sha,
            f"stress calibration retained input changed: {experiment}")
    require(row["prepared_sha256"] == full["prepared_sha256"] and
            row["wage_input_sha256"] == full["wage_input_sha256"],
            f"stress calibration CZ18 parent input changed: {experiment}")
    require(row["estimator_status"] == "KSS_POINT_ESTIMATES_ONLY" and
            integer(row, "command_rc") == 0,
            f"stress calibration failed: {experiment}")
    require(row["algorithm_requested"] == row["algorithm_selected"] == "jla" and
            row["batch_requested"] == "auto" and
            row["preconditioner_requested"] == "auto" and
            row["preconditioner_selected"] == "cmg" and
            row["fallback_status"] == "NOT_NEEDED" and
            row["routing_reason"] != "",
            f"stress calibration did not route auto to CMG: {experiment}")
    require(integer(row, "requested_probes") == CALIBRATION_PROBES and
            integer(row, "seed") == 8675309 and
            abs(finite(row, "tolerance") - REQUESTED_TOLERANCE) <= 1e-20 and
            integer(row, "rng_state_reproducible") == 1,
            f"stress calibration probe semantics changed: {experiment}")
    processors = integer(row, "requested_processors")
    require(processors == integer(row, "actual_processors") and
            processors in {4, 8} and
            integer(row, "declared_memory_gib") == DECLARED_MEMORY_GIB and
            row["stata_version"].split(".")[0] == "19" and
            integer(row, "stata_mp") == 1 and row["stata_flavor"].strip() != "",
            f"stress calibration processor/memory configuration changed: {experiment}")
    selected_batch = integer(row, "selected_batch")
    require(selected_batch > 0,
            f"stress calibration selected invalid batch: {experiment}")
    memory_limit = DECLARED_MEMORY_GIB * 1024**3
    for field in ("batch_scratch_forecast_bytes", "memory_forecast_bytes",
                  "route_forecast_peak_bytes"):
        forecast = finite(row, field)
        require(0 <= forecast <= memory_limit,
                f"stress calibration forecast exceeds memory: {experiment}: {field}")
    require(integer(row, "route_planned_rhs") == CALIBRATION_RHS and
            integer(row, "route_hierarchy_levels") > 1 and
            integer(row, "route_hybrid_vertices") > 6144 and
            integer(row, "route_hybrid_edges") >= 1 and
            1 <= integer(row, "route_terminal_vertices") <= 6144,
            f"stress calibration did not use bounded multilevel CMG: {experiment}")
    for field, strict in (
        ("N_retained", False), ("worker_levels", True), ("firm_levels", False),
        ("deletion_units", False), ("route_hybrid_vertices", False),
        ("route_hybrid_edges", False),
    ):
        observed = integer(row, field)
        boundary = 2 * integer(full, field)
        require(observed > boundary if strict else observed >= boundary,
                f"stress calibration is not 2x CZ18: {experiment}: {field}")
    require(integer(row, "graph_retained_edges") >= 2 and
            integer(row, "graph_fixedpoint_iterations") >= 0,
            f"invalid stress fixed-point graph certificate: {experiment}")
    identity(row, "plugin", experiment)
    identity(row, "corrected", experiment)
    command_seconds = finite(row, "command_seconds")
    correction_seconds = finite(row, "correction_seconds")
    leverage_seconds = finite(row, "leverage_seconds")
    target_seconds = finite(row, "target_seconds")
    require(min(command_seconds, correction_seconds, leverage_seconds,
                target_seconds) >= 0 and
            timing_identity_after_csv(
                correction_seconds, leverage_seconds, target_seconds),
            f"stress calibration correction timing identity failed: {experiment}: "
            f"{timing_identity_diagnostic(correction_seconds, leverage_seconds, target_seconds)}")
    require(correction_seconds <= command_seconds + 1.0,
            f"stress correction timer exceeds command timer: {experiment}")
    require(job_id_file.is_file(),
            f"missing stress calibration job receipt: {experiment}")
    expected_job = job_id_file.read_text(encoding="utf-8").strip()
    require(JOB_ID.fullmatch(expected_job) is not None,
            f"invalid stress calibration job receipt: {experiment}")
    accounting = qacct_pass(qacct, processors, expected_job)
    require(float(accounting["maxvmem_bytes"]) <= MAX_MEMORY_BYTES,
            f"stress qacct exceeded 60 GiB: {experiment}")
    wall_seconds = float(accounting["wall_seconds"])
    require(command_seconds <= wall_seconds + 1.0,
            f"stress command timer exceeds qacct wall: {experiment}")
    node = node_characteristics_pass(node_characteristics, accounting)
    rhs_signature, rhs_max_residual = validate_rhs(rhs, experiment=experiment)
    aggregate_residual = finite(row, "solver_max_residual")
    require(0 <= aggregate_residual <= COMPLETE_RESIDUAL_GATE * (1 + 1e-10) and
            close(aggregate_residual, rhs_max_residual),
            f"aggregate/RHS residual certificate failed: {experiment}")
    require(integer(row, "solver_iterations") == max(
                int(item[3]) for item in rhs_signature),
            f"aggregate/RHS iteration certificate failed: {experiment}")
    return {
        "row": row,
        "job_id": expected_job,
        "qacct": accounting,
        "node": node,
        "rhs_signature": rhs_signature,
        "rhs_max_residual": rhs_max_residual,
        "command_seconds": command_seconds,
        "correction_seconds": correction_seconds,
        "leverage_seconds": leverage_seconds,
        "target_seconds": target_seconds,
        "alpha_seconds": max(0.0, wall_seconds - correction_seconds),
        "beta_seconds_per_probe": correction_seconds / CALIBRATION_PROBES,
        "selected_batch": selected_batch,
    }


def compare_calibrations(
    reference: dict[str, object], candidate: dict[str, object], label: str,
) -> None:
    left = reference["row"]
    right = candidate["row"]
    require(isinstance(left, dict) and isinstance(right, dict),
            f"invalid stress calibration comparison: {label}")
    scientific = (
        "plugin_worker", "plugin_firm", "plugin_covariance", "plugin_total",
        "corrected_worker", "corrected_firm", "corrected_covariance",
        "corrected_total", "max_leverage",
    )
    for field in scientific:
        require(close(finite(left, field), finite(right, field)),
                f"stress calibration scientific result changed: {label}: {field}")
    numeric_config = (
        "requested_processors", "actual_processors", "declared_memory_gib",
        "input_rows", "requested_probes", "seed", "tolerance", "N_stored",
        "N_retained", "N_requested", "N_mover_input", "N_graph_dropped",
        "worker_levels", "firm_levels", "deletion_units", "graph_edges",
        "graph_retained_edges", "graph_bridge_units_removed",
        "graph_bridge_rows_removed", "graph_fixedpoint_iterations",
        "selected_batch", "batch_scratch_forecast_bytes",
        "memory_forecast_bytes", "route_planned_rhs",
        "route_hierarchy_levels", "route_hybrid_vertices",
        "route_hybrid_edges", "route_terminal_vertices",
        "route_diagonal_max_iterations", "route_cmg_max_iterations",
        "route_forecast_peak_bytes", "solver_iterations",
        "solver_schur_batches", "solver_precond_batches",
        "installed_public_path", "rng_state_reproducible",
    )
    for field in numeric_config:
        require(finite(left, field) == finite(right, field),
                f"stress calibration configuration changed: {label}: {field}")
    string_config = (
        "source_commit", "bundle_sha256", "data_manifest_sha256",
        "prepared_sha256", "estimator_input_sha256", "wage_input_sha256",
        "stata_version", "stata_flavor", "algorithm_requested",
        "algorithm_selected", "batch_requested", "preconditioner_requested",
        "preconditioner_selected", "routing_reason", "fallback_status",
        "fallback_message", "estimator_status",
    )
    for field in string_config:
        require(left[field] == right[field],
                f"stress calibration configuration changed: {label}: {field}")
    require(reference["rhs_signature"] == candidate["rhs_signature"],
            f"stress calibration RHS discrete certificate changed: {label}")
    require(reference["selected_batch"] == candidate["selected_batch"],
            f"stress calibration selected batch changed: {label}")


def stress_projection_from_calibrations(
    calibrations: list[dict[str, object]],
) -> tuple[float, float, int, int]:
    require(len(calibrations) == 3,
            "stress projection requires three independent calibrations")
    alpha = max(float(item["alpha_seconds"]) for item in calibrations)
    beta = max(float(item["beta_seconds_per_probe"]) for item in calibrations)
    projected = math.ceil(max(
        300.0,
        SETUP_SAFETY_FACTOR * alpha +
        MARGINAL_SAFETY_FACTOR * FULL_PROBES * beta +
        FIXED_HEADROOM_SECONDS,
    ))
    timeout = max(STRESS_TIMEOUT_FLOOR_SECONDS, projected)
    return alpha, beta, projected, timeout


def main() -> int:
    # Reject an impossible scheduler request before argparse attempts to open
    # or structurally group the much larger evidence argument set.
    timeout_positions = [index for index, token in enumerate(sys.argv[1:])
                         if token == "--timeout"]
    if len(timeout_positions) == 1:
        raw_arguments = sys.argv[1:]
        position = timeout_positions[0]
        if position + 1 < len(raw_arguments):
            raw_timeout = raw_arguments[position + 1]
            if raw_timeout != "auto":
                require(re.fullmatch(r"[0-9]+", raw_timeout) is not None and
                        STRESS_TIMEOUT_FLOOR_SECONDS <= int(raw_timeout) <=
                        MAXIMUM_TIMEOUT_SECONDS, "invalid stress timeout")
    parser = argparse.ArgumentParser()
    parser.add_argument("--calibration", required=True, nargs=3, type=Path)
    parser.add_argument("--calibration-qacct", required=True, nargs=3, type=Path)
    parser.add_argument("--calibration-job-id-file", required=True, nargs=3, type=Path)
    parser.add_argument("--calibration-rhs", required=True, nargs=3, type=Path)
    parser.add_argument("--calibration-node-characteristics", required=True,
                        nargs=3, type=Path)
    parser.add_argument("--cz18-full-result", required=True, type=Path)
    parser.add_argument("--retained-sha-file", required=True, type=Path)
    parser.add_argument("--bundle-sha", required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--manifest-sha", required=True)
    parser.add_argument("--timeout", required=True,
                        help="'auto' for admission, or the bound certificate value")
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    require(HEX64.fullmatch(args.bundle_sha) is not None, "invalid bundle SHA")
    require(HEX40.fullmatch(args.source_commit) is not None, "invalid source commit")
    require(HEX64.fullmatch(args.manifest_sha) is not None, "invalid manifest SHA")
    requested_timeout: int | None
    if args.timeout == "auto":
        requested_timeout = None
    else:
        require(re.fullmatch(r"[0-9]+", args.timeout) is not None,
                "invalid stress timeout")
        requested_timeout = int(args.timeout)
        require(STRESS_TIMEOUT_FLOOR_SECONDS <= requested_timeout <=
                MAXIMUM_TIMEOUT_SECONDS, "invalid stress timeout")
    grouped_paths = (
        args.calibration, args.calibration_qacct, args.calibration_job_id_file,
        args.calibration_rhs, args.calibration_node_characteristics,
    )
    for paths in grouped_paths:
        require(len({str(path.resolve()) for path in paths}) == 3,
                "stress calibration evidence paths must be distinct")
    retained_sha = args.retained_sha_file.read_text(encoding="utf-8").strip()
    require(HEX64.fullmatch(retained_sha) is not None,
            "invalid retained-sample SHA")
    full = validate_full_result(
        args.cz18_full_result, source_commit=args.source_commit,
        bundle_sha=args.bundle_sha, manifest_sha=args.manifest_sha,
    )

    calibrations: list[dict[str, object]] = []
    for index, experiment in enumerate(EXPECTED_EXPERIMENTS):
        calibrations.append(validate_calibration(
            experiment=experiment,
            calibration=args.calibration[index],
            qacct=args.calibration_qacct[index],
            job_id_file=args.calibration_job_id_file[index],
            rhs=args.calibration_rhs[index],
            node_characteristics=args.calibration_node_characteristics[index],
            retained_sha=retained_sha, source_commit=args.source_commit,
            bundle_sha=args.bundle_sha, manifest_sha=args.manifest_sha,
            full=full,
        ))
    for index, candidate in enumerate(calibrations[1:], 2):
        compare_calibrations(calibrations[0], candidate, f"r1/r{index}")
    require(len({str(item["job_id"]) for item in calibrations}) == 3,
            "stress calibration jobs are not independent")

    alpha, beta, projection, timeout = stress_projection_from_calibrations(
        calibrations)
    require(timeout <= MAXIMUM_TIMEOUT_SECONDS,
            "stress projection exceeds the SCC runtime envelope")
    if requested_timeout is not None:
        require(requested_timeout == timeout,
                "stress timeout is not the exact robust projected timeout")

    node_classes = sorted([
        (str(item["node"]["uname_machine"]),
         str(item["node"]["cpu_model"]),
         int(str(item["node"]["logical_cpus"])))
        for item in calibrations
    ])
    node_class_multiset = json.dumps(node_classes, separators=(",", ":"))
    node_characteristics_identical = len(set(node_classes)) == 1
    hostnames = [str(item["qacct"]["hostname"]) for item in calibrations]
    qnames = [str(item["qacct"]["qname"]) for item in calibrations]
    walls = [float(item["qacct"]["wall_seconds"]) for item in calibrations]
    corrections = [float(item["correction_seconds"]) for item in calibrations]

    lines = [
        "schema=kss_stress_projection_v2",
        "status=PASS",
        f"source_commit={args.source_commit}",
        f"bundle_sha256={args.bundle_sha}",
        f"data_manifest_sha256={args.manifest_sha}",
        f"cz18_full_result_sha256={sha256(args.cz18_full_result)}",
        f"retained_sha_file_sha256={sha256(args.retained_sha_file)}",
        f"estimator_input_sha256={retained_sha}",
        f"calibration_count={len(calibrations)}",
        f"calibration_experiment_ids={'|'.join(EXPECTED_EXPERIMENTS)}",
        f"processors={int(calibrations[0]['row']['requested_processors'])}",
        f"declared_memory_gib={DECLARED_MEMORY_GIB}",
        f"selected_batch={calibrations[0]['selected_batch']}",
        f"node_class_multiset_json={node_class_multiset}",
        f"node_characteristics_identical={int(node_characteristics_identical)}",
        f"qacct_hostnames_json={json.dumps(hostnames, separators=(',', ':'))}",
        f"qacct_qnames_json={json.dumps(qnames, separators=(',', ':'))}",
        f"hostname_policy={HOSTNAME_POLICY}",
    ]
    for index, (experiment, item) in enumerate(
            zip(EXPECTED_EXPERIMENTS, calibrations, strict=False), 1):
        accounting = item["qacct"]
        lines.extend((
            f"calibration_r{index}_experiment_id={experiment}",
            f"calibration_r{index}_csv_sha256={sha256(args.calibration[index - 1])}",
            f"calibration_r{index}_qacct_sha256={sha256(args.calibration_qacct[index - 1])}",
            f"calibration_r{index}_job_id_file_sha256={sha256(args.calibration_job_id_file[index - 1])}",
            f"calibration_r{index}_rhs_sha256={sha256(args.calibration_rhs[index - 1])}",
            f"calibration_r{index}_node_characteristics_sha256={sha256(args.calibration_node_characteristics[index - 1])}",
            f"calibration_r{index}_job_id={item['job_id']}",
            f"calibration_r{index}_hostname={accounting['hostname']}",
            f"calibration_r{index}_qname={accounting['qname']}",
            f"calibration_r{index}_cpu_seconds={float(accounting['cpu_seconds']):.17g}",
            f"calibration_r{index}_qacct_wall_seconds={float(accounting['wall_seconds']):.17g}",
            f"calibration_r{index}_qacct_maxvmem_bytes={float(accounting['maxvmem_bytes']):.17g}",
            f"calibration_r{index}_command_seconds={float(item['command_seconds']):.17g}",
            f"calibration_r{index}_leverage_seconds={float(item['leverage_seconds']):.17g}",
            f"calibration_r{index}_target_seconds={float(item['target_seconds']):.17g}",
            f"calibration_r{index}_correction_seconds={float(item['correction_seconds']):.17g}",
            f"calibration_r{index}_alpha_seconds={float(item['alpha_seconds']):.17g}",
            f"calibration_r{index}_beta_seconds_per_probe={float(item['beta_seconds_per_probe']):.17g}",
            f"calibration_r{index}_rhs_max_residual={float(item['rhs_max_residual']):.17g}",
        ))
    lines.extend((
        f"median_qacct_wall_seconds={statistics.median(walls):.17g}",
        f"median_correction_seconds={statistics.median(corrections):.17g}",
        f"alpha_seconds={alpha:.17g}",
        f"beta_seconds_per_probe={beta:.17g}",
        f"setup_safety_factor={SETUP_SAFETY_FACTOR:.17g}",
        f"marginal_safety_factor={MARGINAL_SAFETY_FACTOR:.17g}",
        f"headroom_seconds={FIXED_HEADROOM_SECONDS}",
        f"calibration_probes={CALIBRATION_PROBES}",
        f"full_probes={FULL_PROBES}",
        f"projected_seconds={projection}",
        f"timeout_floor_seconds={STRESS_TIMEOUT_FLOOR_SECONDS}",
        f"timeout_seconds={timeout}",
        f"formula={STRESS_PROJECTION_FORMULA}",
    ))
    payload = "\n".join(lines) + "\n"
    args.output.parent.mkdir(parents=True, exist_ok=True)
    temporary = args.output.with_name(args.output.name + f".tmp.{os.getpid()}")
    temporary.write_text(payload, encoding="utf-8")
    os.replace(temporary, args.output)
    print("KSS_PROD STRESS PROJECTION PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

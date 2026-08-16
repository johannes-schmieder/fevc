#!/usr/bin/env python3
"""Validate the KSS-PROD SCC plan and complete production evidence."""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import os
import re
from pathlib import Path

try:
    from .scc.select_prod_calibration import (
        BATCH_BASE_SCRATCH_BYTES, BATCH_MEMORY_FRACTION,
        CALIBRATION_MEMORY_GIB, CALIBRATION_PATTERN, COMPLETE_RESIDUAL_GATE,
        FEASIBLE_EXPLICIT_BATCHES, FIXED_HEADROOM_SECONDS,
        FULL_RETAINED_SAVE_ALLOWANCE_SECONDS, FORMULA,
        FULL_PROBES, INFEASIBLE_BATCHES, MAXIMUM_TIMEOUT_SECONDS,
        REQUESTED_TOLERANCE, SAFETY_FACTOR, batch_memory_budget_bytes,
        candidate_sort_key, evidence_digest, forecast_batch_bytes,
    )
except ImportError:
    from scc.select_prod_calibration import (
        BATCH_BASE_SCRATCH_BYTES, BATCH_MEMORY_FRACTION,
        CALIBRATION_MEMORY_GIB, CALIBRATION_PATTERN, COMPLETE_RESIDUAL_GATE,
        FEASIBLE_EXPLICIT_BATCHES, FIXED_HEADROOM_SECONDS,
        FULL_RETAINED_SAVE_ALLOWANCE_SECONDS, FORMULA,
        FULL_PROBES, INFEASIBLE_BATCHES, MAXIMUM_TIMEOUT_SECONDS,
        REQUESTED_TOLERANCE, SAFETY_FACTOR, batch_memory_budget_bytes,
        candidate_sort_key, evidence_digest, forecast_batch_bytes,
    )


PLAN_FIELDS = (
    "experiment_id", "phase", "stage", "dataset", "stata_version",
    "processors", "memory_gib", "probes", "batch", "preconditioner",
    "temperature", "repetitions", "depends",
)
DATA_FIELDS = (
    "dataset", "wage_input_dta", "wage_input_sha256", "sample_mode",
    "max_workers", "separations_commit", "matlab_detail",
    "matlab_detail_sha256",
)
HEX64 = re.compile(r"[0-9a-f]{64}")
HEX40 = re.compile(r"[0-9a-f]{40}")
# Aggregate CSVs serialize Stata scalars to about eight significant decimal places.
# This tolerance applies only when rechecking identities in those exported files.
CSV_IDENTITY_SERIALIZATION_TOLERANCE = 1e-7
CALIBRATION_SETUP_SAFETY_FACTOR = 1.25
CALIBRATION_MARGINAL_SAFETY_FACTOR = 1.5
CALIBRATION_MINIMUM_TIMEOUT_SECONDS = 300
ESTIMATOR_STAGES = {
    "bundle_smoke", "install_auto", "install_cmg", "selector", "fixed",
    "cz18_preflight", "calibration", "full", "stress2x",
}
ALL_STAGES = ESTIMATOR_STAGES | {
    "license", "hierarchy_stress", "prepare", "sample_compare",
    "calibration_selector",
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def read_text(path: Path) -> str:
    require(path.is_file(), f"missing file: {path}")
    return path.read_text(encoding="utf-8", errors="replace")


def read_rows(path: Path) -> list[dict[str, str]]:
    require(path.is_file(), f"missing CSV/TSV: {path}")
    delimiter = "\t" if path.suffix == ".tsv" else ","
    with path.open(newline="", encoding="utf-8") as handle:
        return list(csv.DictReader(handle, delimiter=delimiter))


def file_sha256(path: Path) -> str:
    require(path.is_file(), f"missing file: {path}")
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def finite(row: dict[str, str], field: str) -> float:
    try:
        value = float(row[field])
    except (KeyError, ValueError) as exc:
        raise ValueError(f"invalid numeric field {field}") from exc
    require(math.isfinite(value), f"nonfinite field {field}")
    return value


def close(left: float, right: float, tolerance: float) -> bool:
    return abs(left - right) <= tolerance * (1 + abs(left))


def load_plan(path: Path) -> list[dict[str, str]]:
    rows = read_rows(path)
    require(rows and tuple(rows[0]) == PLAN_FIELDS, "unexpected production plan columns")
    seen: set[str] = set()
    for row in rows:
        experiment = row["experiment_id"]
        require(re.fullmatch(r"[A-Za-z0-9._-]+", experiment) is not None, "invalid experiment ID")
        require(experiment not in seen, f"duplicate experiment: {experiment}")
        require(row["phase"] in {"preflight", "calibration", "production"}, "invalid phase")
        require(row["stage"] in ALL_STAGES, "invalid stage")
        require(row["dataset"] in {"synthetic", "cz18", "cz24", "cz25"}, "invalid dataset")
        require(row["stata_version"] in {"18", "19"}, "invalid Stata version")
        require(row["processors"] in {"0", "4", "8"}, "invalid processor request")
        require(row["preconditioner"] in {"-", "auto", "diagonal", "cmg", "selected"}, "bad route")
        require(row["temperature"] in {"cold", "warm"}, "bad temperature")
        require(row["repetitions"] in {"1", "2"}, "bad repetition count")
        require(row["memory_gib"].isdigit() and 1 <= int(row["memory_gib"]) <= 56,
                "declared memory exceeds 56 GiB")
        require(row["probes"].isdigit(), "invalid probe count")
        if row["stage"] in ESTIMATOR_STAGES:
            require(int(row["probes"]) >= 2, "estimator stage has too few probes")
        else:
            require(row["probes"] == "0", "non-estimator stage consumes probes")
        if row["depends"] != "-":
            for dependency in row["depends"].split(","):
                require(dependency in seen, f"dependency is absent or not topological: {dependency}")
        seen.add(experiment)
    stages = {row["stage"] for row in rows}
    require(ALL_STAGES <= stages, "required DAG stage absent")
    licenses = {(row["stata_version"], row["processors"]) for row in rows
                if row["stage"] == "license"}
    require(licenses == {("18", "4"), ("19", "4")},
            "license matrix must cover the available four-slot Stata 18/19 modules")
    install = {(row["stata_version"], row["preconditioner"], row["batch"])
               for row in rows if row["stage"] == "install_auto"}
    require(install == {("18", "auto", "auto"), ("19", "auto", "auto")},
            "normal-install algorithm(auto) matrix incomplete")
    install_cmg = {(row["stata_version"], row["preconditioner"], row["batch"])
                   for row in rows if row["stage"] == "install_cmg"}
    require(install_cmg == {("18", "cmg", "4"), ("19", "cmg", "4")},
            "normal-install JLA/CMG matrix incomplete")
    hierarchy = {(row["stata_version"], row["processors"]) for row in rows
                 if row["stage"] == "hierarchy_stress"}
    require(hierarchy == {("19", "4")}, "four-slot hierarchy stress cell absent")
    prepare = {row["dataset"] for row in rows if row["stage"] == "prepare"}
    require(prepare == {"cz18", "cz24", "cz25"}, "pure-Stata preparation matrix incomplete")
    comparison = {row["dataset"] for row in rows if row["stage"] == "sample_compare"}
    require(comparison == {"cz24", "cz25"}, "MATLAB sample comparator matrix incomplete")
    calibration = [row for row in rows if row["stage"] == "calibration"]
    require({row["preconditioner"] for row in calibration} == {"auto", "cmg"},
            "CZ18 calibration must cover automatic and forced-CMG routes")
    require({row["processors"] for row in calibration} == {"4"},
            "calibration must use the available four-slot SCC Stata module")
    require({row["batch"] for row in calibration} == {"8", "16", "auto"},
            "calibration must contain only feasible batch requests")
    require({row["probes"] for row in calibration} == {"20", "40"},
            "calibration must identify setup and marginal probe cost")
    require(len(calibration) == 24 and
            all(CALIBRATION_PATTERN.fullmatch(row["experiment_id"])
                for row in calibration),
            "calibration must be the registered 24-cell paired matrix")
    expected_calibrations = {
        f"cal_{route}_b{batch}_p{probes}_{temperature}"
        for route in ("auto", "cmg")
        for batch in ("8", "16", "auto")
        for probes in (20, 40)
        for temperature in ("cold", "warm")
    }
    require({row["experiment_id"] for row in calibration} == expected_calibrations,
            "paired calibration matrix is incomplete")
    calibration_by_id = {row["experiment_id"]: row for row in calibration}
    for route in ("auto", "cmg"):
        for batch in ("8", "16", "auto"):
            chain = [
                f"cal_{route}_b{batch}_p20_cold",
                f"cal_{route}_b{batch}_p20_warm",
                f"cal_{route}_b{batch}_p40_cold",
                f"cal_{route}_b{batch}_p40_warm",
            ]
            require(calibration_by_id[chain[0]]["depends"] == "cz18_preflight",
                    "calibration chain must start from accepted CZ18 preflight")
            for predecessor, successor in zip(chain, chain[1:]):
                require(calibration_by_id[successor]["depends"] == predecessor,
                        "calibration cells must be serialized within route/batch chain")
    require(not any(row["batch"] in {str(width) for width in INFEASIBLE_BATCHES}
                    for row in calibration),
            "forecast-infeasible batch was scheduled")
    production = [row for row in rows if row["phase"] == "production"]
    require(all(row["processors"] == "0" and
                row["preconditioner"] == "selected" and
                ((row["stage"] == "full" and row["batch"] == "selected") or
                 (row["stage"] == "stress2x" and row["batch"] == "auto"))
                for row in production),
            "production rows must use the calibrated route and safe batch policy")
    return rows


def load_data_manifest(path: Path) -> dict[str, dict[str, str]]:
    rows = read_rows(path)
    require(rows and tuple(rows[0]) == DATA_FIELDS, "unexpected data manifest columns")
    result: dict[str, dict[str, str]] = {}
    for row in rows:
        dataset = row["dataset"]
        require(dataset in {"cz18", "cz24", "cz25"} and dataset not in result,
                "invalid or duplicate data row")
        require(row["wage_input_dta"].startswith("/projectnb/welfgr/"), "unsafe wage path")
        require(HEX64.fullmatch(row["wage_input_sha256"]) is not None, "invalid wage hash")
        require(row["sample_mode"] in {"small", "full"} and row["max_workers"].isdigit(),
                "invalid preparation policy")
        maximum = int(row["max_workers"])
        require((row["sample_mode"] == "full" and maximum == 0) or
                (row["sample_mode"] == "small" and maximum >= 100),
                "invalid preparation limit")
        require(HEX40.fullmatch(row["separations_commit"]) is not None,
                "invalid Separations commit")
        if dataset in {"cz24", "cz25"}:
            require(row["matlab_detail"].startswith("/projectnb/welfgr/"),
                    "unsafe MATLAB detail path")
            require(HEX64.fullmatch(row["matlab_detail_sha256"]) is not None,
                    "invalid MATLAB detail hash")
        else:
            require(row["matlab_detail"] == row["matlab_detail_sha256"] == "-",
                    "CZ18 must not require MATLAB detail")
        result[dataset] = row
    require(set(result) == {"cz18", "cz24", "cz25"}, "data manifest incomplete")
    return result


def parse_memory(value: str) -> float:
    match = re.fullmatch(r"([0-9]+(?:\.[0-9]+)?)([KMGTP]?)", value.strip(), re.I)
    require(match is not None, f"unparseable memory: {value}")
    scales = {"": 1, "K": 1024, "M": 1024**2, "G": 1024**3,
              "T": 1024**4, "P": 1024**5}
    return float(match.group(1)) * scales[match.group(2).upper()]


def calibration_qacct_wall_seconds(run_dir: Path, experiment: str) -> float:
    report = read_text(run_dir / "qacct" / f"{experiment}.txt")
    matches = re.findall(r"(?m)^ru_wallclock\s+(\S+)", report)
    require(len(matches) == 1, f"qacct {experiment}: missing/duplicate ru_wallclock")
    try:
        wall = float(matches[0])
    except ValueError as exc:
        raise ValueError(f"qacct {experiment}: invalid ru_wallclock") from exc
    require(math.isfinite(wall) and wall >= 0,
            f"qacct {experiment}: invalid ru_wallclock")
    return wall


def recompute_calibration_projection(
    measurements: list[dict[str, float | str]],
) -> tuple[float, float, float, float, int]:
    require(len(measurements) == 4, "projection requires four paired measurements")
    by_cell: dict[tuple[int, str], dict[str, float | str]] = {}
    for row in measurements:
        probes = int(row["probes"])
        temperature = str(row["temperature"])
        command = float(row["command_seconds"])
        correction = float(row["correction_seconds"])
        wall = float(row["qacct_wall_seconds"])
        require(probes in {20, 40} and temperature in {"cold", "warm"},
                "invalid projection cell")
        require((probes, temperature) not in by_cell, "duplicate projection cell")
        require(all(math.isfinite(value) for value in (command, correction, wall)) and
                command >= 0 and correction >= 0 and wall >= 0,
                "invalid projection timing")
        require(correction <= command + 1.0,
                "correction timer exceeds enclosing command timer")
        by_cell[(probes, temperature)] = row
    require(set(by_cell) == {(probes, temperature)
                             for probes in (20, 40)
                             for temperature in ("cold", "warm")},
            "paired projection matrix is incomplete")
    paired_slopes = [
        (float(by_cell[(40, temperature)]["command_seconds"]) -
         float(by_cell[(20, temperature)]["command_seconds"])) / 20.0
        for temperature in ("cold", "warm")
    ]
    correction_rates = [
        float(row["correction_seconds"]) / int(row["probes"])
        for row in measurements
    ]
    beta = max(0.0, *paired_slopes, *correction_rates)
    alpha = max(0.0, *(float(row["command_seconds"]) -
                       int(row["probes"]) * beta for row in measurements))
    for row in measurements:
        probes = int(row["probes"])
        command = float(row["command_seconds"])
        correction = float(row["correction_seconds"])
        require(alpha + probes * beta + 1e-9 >= command,
                "projection envelope does not cover command timing")
        require(probes * beta + 1e-9 >= correction,
                "projection slope does not cover correction timing")
    cold_overhead = max(
        0.0,
        *(float(row["qacct_wall_seconds"]) - float(row["command_seconds"])
          for row in measurements if row["temperature"] == "cold"),
    )
    headroom = max(
        float(FIXED_HEADROOM_SECONDS),
        cold_overhead + FULL_RETAINED_SAVE_ALLOWANCE_SECONDS,
    )
    projected = (
        CALIBRATION_SETUP_SAFETY_FACTOR * alpha +
        CALIBRATION_MARGINAL_SAFETY_FACTOR * FULL_PROBES * beta +
        headroom
    )
    timeout = math.ceil(max(CALIBRATION_MINIMUM_TIMEOUT_SECONDS, projected))
    return alpha, beta, headroom, projected, timeout


def validate_qacct(run_dir: Path, experiment: str, processors: int) -> dict[str, str]:
    report = read_text(run_dir / "qacct" / f"{experiment}.txt")
    def field(name: str) -> str:
        matches = re.findall(rf"(?m)^{re.escape(name)}\s+(\S+)", report)
        require(len(matches) == 1, f"qacct {experiment}: missing/duplicate {name}")
        return matches[0]
    result = {name: field(name) for name in (
        "jobnumber", "failed", "exit_status", "slots", "maxvmem", "ru_wallclock",
        "cpu", "hostname", "qname",
    )}
    submitted = read_text(run_dir / "submissions" / f"{experiment}.job_id").strip()
    require(result["jobnumber"] == submitted.split(".", 1)[0],
            f"qacct {experiment}: job number mismatch")
    require(result["failed"] == "0", f"qacct {experiment}: failed")
    require(result["exit_status"] == "0", f"qacct {experiment}: nonzero exit")
    require(int(result["slots"]) == processors, f"qacct {experiment}: slot mismatch")
    require(parse_memory(result["maxvmem"]) <= 60 * 1024**3,
            f"qacct {experiment}: maxvmem exceeds 60 GiB")
    return result


def validate_resource(path: Path) -> int:
    report = read_text(path)
    match = re.search(r"Maximum resident set size \(kbytes\):\s*(\d+)", report)
    require(match is not None and int(match.group(1)) > 0, f"missing peak RSS: {path}")
    rss = int(match.group(1))
    require(rss <= 60 * 1024**2, f"peak RSS exceeds 60 GiB: {path}")
    return rss


def validate_wrapper(root: Path, experiment: str, source_commit: str,
                     bundle_sha: str, manifest_sha: str) -> None:
    expected = f"KSS_PROD_WRAPPER_PASS {experiment} {bundle_sha} {source_commit} {manifest_sha}\n"
    require(read_text(root / "wrapper.pass") == expected, f"wrapper identity mismatch: {experiment}")


def validate_submission(run_dir: Path, experiment: str, processors: int,
                        memory_gib: int, timeout_seconds: int,
                        bundle_sha: str, manifest_sha: str,
                        ledger: dict[str, dict[str, str]],
                        dependencies: str,
                        phase_by_experiment: dict[str, str]) -> None:
    row = ledger.get(experiment)
    require(row is not None, f"submission ledger row absent: {experiment}")
    receipt = read_text(run_dir / "submissions" / f"{experiment}.job_id").strip()
    require(row["job_id"] == receipt, f"submission receipt mismatch: {experiment}")
    require(row["bundle_sha256"] == bundle_sha and
            row["data_manifest_sha256"] == manifest_sha,
            f"submission identity mismatch: {experiment}")
    mem_per_core = (memory_gib + processors - 1) // processors
    hard_seconds = timeout_seconds + 600
    hard_runtime = (f"{hard_seconds // 3600:02d}:"
                    f"{(hard_seconds % 3600) // 60:02d}:"
                    f"{hard_seconds % 60:02d}")
    expected_resource = (f"slots={processors},mem_per_core={mem_per_core}G,"
                         f"timeout={timeout_seconds},h_rt={hard_runtime}")
    require(row["resource_request"] == expected_resource,
            f"submission resources changed: {experiment}")
    expected_dependencies: list[str] = []
    if dependencies != "-":
        for dependency in dependencies.split(","):
            if phase_by_experiment[dependency] != phase_by_experiment[experiment]:
                continue
            job_id = read_text(run_dir / "submissions" / f"{dependency}.job_id").strip()
            expected_dependencies.append(job_id.split(".", 1)[0])
    observed = [] if row["direct_dependencies"] == "-" else row["direct_dependencies"].split(",")
    require(observed == expected_dependencies, f"submission dependency mismatch: {experiment}")


def identity(row: dict[str, str], prefix: str, experiment: str) -> None:
    total = finite(row, f"{prefix}_total")
    parts = finite(row, f"{prefix}_worker") + finite(row, f"{prefix}_firm")
    parts += 2 * finite(row, f"{prefix}_covariance")
    require(
        close(total, parts, CSV_IDENTITY_SERIALIZATION_TOLERANCE),
        (f"{experiment}: {prefix} identity failed after CSV serialization: "
         f"total={total:.17g}, parts={parts:.17g}, "
         f"difference={abs(total - parts):.17g}"),
    )


def validate_fixedpoint_graph_certificate(row: dict[str, str], experiment: str) -> None:
    """Validate pruning diagnostics without counting the no-change certificate pass."""

    retained_edges = finite(row, "graph_retained_edges")
    iterations = finite(row, "graph_fixedpoint_iterations")
    context = (f"{experiment}: graph_retained_edges={retained_edges:g}, "
               f"graph_fixedpoint_iterations={iterations:g}")
    require(retained_edges >= 2, f"invalid retained graph dimensions: {context}")
    require(iterations >= 0 and iterations.is_integer(),
            f"invalid fixed-point graph certificate: {context}")


def validate_selector_route(row: dict[str, str], experiment: str) -> None:
    """Require the easy selector fixture to route directly to B1 without fallback."""

    route = row["preconditioner_selected"]
    fallback = row["fallback_status"]
    require(
        route == "diagonal" and fallback == "NOT_NEEDED",
        (f"{experiment}: easy automatic graph did not route directly to B1: "
         f"preconditioner_selected={route}, fallback_status={fallback}"),
    )


def expected_spec(row: dict[str, str], selection: dict[str, str] | None) -> dict[str, str]:
    spec = dict(row)
    if spec["processors"] == "0":
        require(selection is not None, "production configuration selected too late")
        spec["processors"] = str(int(float(selection["processors"])))
    if spec["batch"] == "selected":
        require(selection is not None, "production batch selected too late")
        spec["batch"] = str(int(float(selection["batch"])))
    if spec["preconditioner"] == "selected":
        require(selection is not None, "production route selected too late")
        spec["preconditioner"] = selection["preconditioner"]
    return spec


def expected_timeout(row: dict[str, str],
                     selection: dict[str, str] | None) -> int:
    stage = row["stage"]
    fixed = {
        "bundle_smoke": 600, "install_auto": 600, "install_cmg": 600,
        "license": 600, "hierarchy_stress": 1800,
        "sample_compare": 1800, "cz18_preflight": 2100,
        "selector": 900, "calibration_selector": 900,
        "prepare": 3600, "fixed": 3600, "calibration": 3600,
        "full": 5400, "stress2x": 5400,
    }
    timeout = fixed[stage]
    if row["phase"] == "production":
        require(selection is not None, "production timeout selected too late")
        timeout = int(float(selection["timeout_seconds"]))
    if stage == "stress2x":
        timeout = min(10800, max(1800, 2 * timeout))
    return timeout


def validate_prepare(run_dir: Path, plan_row: dict[str, str], source_commit: str,
                     bundle_sha: str, manifest_sha: str,
                     data: dict[str, str]) -> str:
    experiment = plan_row["experiment_id"]
    root = run_dir / "experiments" / experiment
    validate_resource(root / "resources.txt")
    require("KSS_BC SEPARATIONS PREPARE PASS" in read_text(root / "application.txt"),
            f"prepare application marker absent: {experiment}")
    validate_wrapper(root, experiment, source_commit, bundle_sha, manifest_sha)
    rows = read_rows(root / "prepare.csv")
    require(len(rows) == 1, "prepare output must contain one row")
    row = rows[0]
    require(row["label"] == experiment and row["sample_mode"] == data["sample_mode"],
            "prepare policy mismatch")
    require(row["source_commit"] == source_commit, "prepare source mismatch")
    require(row["separations_commit"] == data["separations_commit"],
            "prepare Separations source mismatch")
    require(row["wage_input_sha256"] == data["wage_input_sha256"], "prepare input mismatch")
    require(int(finite(row, "requested_max_workers")) == int(data["max_workers"]),
            "prepare worker limit mismatch")
    require(int(finite(row, "prepared_csv_written")) == 0 and
            not (root / "prepared.csv").exists(),
            "production preparation wrote the unused row-level CSV")
    require(row["stata_version"].split(".")[0] == plan_row["stata_version"],
            "prepare Stata version mismatch")
    require(row["stata_flavor"].strip() != "" and
            int(finite(row, "stata_mp")) == 1,
            "prepare did not use Stata/MP")
    require(int(finite(row, "requested_processors")) == int(plan_row["processors"]) and
            int(finite(row, "actual_processors")) == int(plan_row["processors"]),
            "prepare processor binding failed")
    for field in ("stored_rows", "workers", "firms"):
        require(finite(row, field) >= 2, f"invalid prepare dimension {field}")
    require(finite(row, "preparation_seconds") >= 0, "invalid preparation time")
    prepared_sha = read_text(root / "prepared.sha256").strip()
    require(HEX64.fullmatch(prepared_sha) is not None, "invalid prepared hash")
    prepared = root / "prepared.dta"
    if prepared.is_file():
        require(file_sha256(prepared) == prepared_sha, "prepared DTA hash mismatch")
    return prepared_sha


def validate_hierarchy(run_dir: Path, plan_row: dict[str, str], source_commit: str,
                       bundle_sha: str, manifest_sha: str) -> dict[str, str]:
    experiment = plan_row["experiment_id"]
    root = run_dir / "experiments" / experiment
    validate_resource(root / "resources.txt")
    require("CMG HIERARCHY SCALE STRESS PASS" in read_text(root / "application.txt"),
            "hierarchy stress marker absent")
    validate_wrapper(root, experiment, source_commit, bundle_sha, manifest_sha)
    rows = read_rows(root / "cmg_hierarchy.csv")
    require(len(rows) == 1, "hierarchy stress output must contain one row")
    row = rows[0]
    require(row["stata_version"].split(".")[0] == plan_row["stata_version"],
            "hierarchy Stata version mismatch")
    require(row["stata_flavor"].strip() != "" and
            int(finite(row, "stata_mp")) == 1,
            "hierarchy did not use Stata/MP")
    require(int(finite(row, "processors")) == int(plan_row["processors"]),
            "hierarchy processor binding failed")
    require(finite(row, "vertices") == 65536 and finite(row, "batch_columns") == 8,
            "hierarchy SCC scale changed")
    require(finite(row, "levels") >= 3 and finite(row, "terminal_vertices") <= 128,
            "hierarchy did not reach bounded terminal")
    require(finite(row, "predicted_peak_bytes") <= int(plan_row["memory_gib"]) * 1024**3,
            "hierarchy forecast exceeds envelope")
    require(finite(row, "symmetry_relative_error") <= 5e-11,
            "hierarchy symmetry check failed")
    require(finite(row, "workspace_mreldif") <= 1e-13,
            "hierarchy workspace changed result")
    return row


def validate_comparison(run_dir: Path, plan_row: dict[str, str], source_commit: str,
                        bundle_sha: str, manifest_sha: str, prepared_sha: str,
                        data: dict[str, str]) -> None:
    experiment = plan_row["experiment_id"]
    root = run_dir / "experiments" / experiment
    validate_resource(root / "resources.txt")
    require("KSS_PROD SCC SAMPLE COMPARISON PASS" in read_text(root / "application.txt"),
            "sample comparison marker absent")
    validate_wrapper(root, experiment, source_commit, bundle_sha, manifest_sha)
    rows = read_rows(root / "sample_comparison.csv")
    require(len(rows) == 1, "sample comparison must contain one row")
    row = rows[0]
    require(row["comparison_status"] == "RETAINED_SAMPLE_EQUAL", "retained sample differs")
    require(row["source_commit"] == source_commit and row["bundle_sha256"] == bundle_sha,
            "sample comparison source mismatch")
    require(row["data_manifest_sha256"] == manifest_sha, "sample comparison manifest mismatch")
    require(row["prepared_sha256"] == prepared_sha, "sample comparison prepared hash mismatch")
    require(row["wage_input_sha256"] == data["wage_input_sha256"],
            "sample comparison wage mismatch")
    require(row["matlab_detail_sha256"] == data["matlab_detail_sha256"],
            "sample comparison MATLAB detail mismatch")
    require(row["stata_version"].split(".")[0] == plan_row["stata_version"] and
            row["stata_flavor"].strip() != "" and
            int(finite(row, "stata_mp")) == 1,
            "sample comparison Stata mismatch")
    require(int(finite(row, "requested_processors")) == int(plan_row["processors"]) and
            int(finite(row, "actual_processors")) == int(plan_row["processors"]),
            "sample comparison processor binding failed")
    require(finite(row, "kss_matches") == finite(row, "matlab_matches") > 0,
            "sample comparison count mismatch")
    require(finite(row, "kss_only") == finite(row, "matlab_only") == 0,
            "sample comparison overlap mismatch")


def validate_result(run_dir: Path, plan_row: dict[str, str], source_commit: str,
                    bundle_sha: str, manifest_sha: str, prepared_hashes: dict[str, str],
                    data_rows: dict[str, dict[str, str]], selection: dict[str, str] | None,
                    stress_parent_sha: str | None) -> tuple[list[dict[str, str]], float]:
    spec = expected_spec(plan_row, selection)
    experiment = plan_row["experiment_id"]
    root = run_dir / "experiments" / experiment
    validate_resource(root / "resources.txt")
    application = read_text(root / "application.txt")
    require("KSS_PROD SCC" in application, f"application marker absent: {experiment}")
    require(re.search(r"(?m)^r\([0-9]+\);", application.lower()) is None and
            "segmentation" not in application.lower(), f"fatal application pattern: {experiment}")
    validate_wrapper(root, experiment, source_commit, bundle_sha, manifest_sha)
    rows = read_rows(root / f"prod_{experiment}.csv")
    require(len(rows) == int(plan_row["repetitions"]), f"unexpected result rows: {experiment}")
    worst_residual = 0.0
    for index, row in enumerate(rows, 1):
        require(row["experiment_id"] == experiment and row["source_commit"] == source_commit,
                "experiment/source output mismatch")
        require(row["bundle_sha256"] == bundle_sha, "job used another bundle")
        require(int(finite(row, "repetition")) == index, "bad repetition order")
        require(row["stata_version"].split(".")[0] == spec["stata_version"],
                "Stata version mismatch")
        require(row["stata_flavor"].strip() != "" and
                int(finite(row, "stata_mp")) == 1,
                "job did not use Stata/MP")
        require(int(finite(row, "requested_processors")) == int(spec["processors"]),
                "processor request mismatch")
        require(int(finite(row, "actual_processors")) == int(spec["processors"]),
                "processor count was not bound exactly")
        if plan_row["stage"] == "license":
            require(row["estimator_status"] == "LICENSE_CAPACITY_PASS",
                    "required Stata/MP license capacity unavailable")
            continue
        require(row["stage"] == plan_row["stage"] and row["temperature"] == plan_row["temperature"],
                "stage/temperature mismatch")
        require(row["data_manifest_sha256"] in {manifest_sha, "-"}, "manifest output mismatch")
        require(row["estimator_status"] == "KSS_POINT_ESTIMATES_ONLY", "estimator failed")
        require(finite(row, "command_rc") == 0, "command failed")
        require(int(finite(row, "requested_probes")) == int(spec["probes"]), "probe count changed")
        require(int(finite(row, "seed")) == 8675309, "seed changed")
        require(abs(finite(row, "tolerance") - REQUESTED_TOLERANCE) <= 1e-20,
                "requested tolerance changed")
        require(int(finite(row, "declared_memory_gib")) == int(spec["memory_gib"]) <= 56,
                "declared memory changed")
        require(finite(row, "rng_state_reproducible") == 1,
                "terminal Stata RNG state is not reproducible")
        if plan_row["dataset"] != "synthetic":
            dataset = plan_row["dataset"]
            require(row["data_manifest_sha256"] == manifest_sha, "data manifest mismatch")
            require(row["prepared_sha256"] == prepared_hashes[dataset], "prepared hash mismatch")
            require(row["wage_input_sha256"] == data_rows[dataset]["wage_input_sha256"],
                    "wage hash mismatch")
            expected_input = stress_parent_sha if plan_row["stage"] == "stress2x" else prepared_hashes[dataset]
            require(expected_input is not None and row["estimator_input_sha256"] == expected_input,
                    "estimator input hash mismatch")
        if plan_row["stage"] == "install_auto":
            require(row["algorithm_requested"] == "auto" and row["algorithm_selected"] == "exact",
                    "public algorithm(auto) did not select exact")
            require(finite(row, "installed_public_path") == 1,
                    "public auto smoke did not use normal net install")
            require(row["preconditioner_selected"] == "not_applicable",
                    "exact auto smoke unexpectedly used a solver route")
        else:
            require(row["algorithm_requested"] == row["algorithm_selected"] == "jla",
                    "routed stage did not use JLA")
            expected_installed = 1 if plan_row["stage"] == "install_cmg" else 0
            require(finite(row, "installed_public_path") == expected_installed,
                    "installed/source route binding changed")
        require(row["batch_requested"] == spec["batch"], "batch request mismatch")
        if spec["batch"] != "auto":
            require(int(finite(row, "selected_batch")) == int(spec["batch"]),
                    "explicit batch changed")
        require(row["preconditioner_requested"] == spec["preconditioner"],
                "route request mismatch")
        require(finite(row, "N_retained") > 0 and finite(row, "worker_levels") >= 2 and
                finite(row, "firm_levels") >= 2, "invalid retained dimensions")
        validate_fixedpoint_graph_certificate(row, experiment)
        for field in ("fit_seconds", "leverage_seconds", "target_seconds",
                      "correction_seconds", "setup_seconds", "schur_seconds",
                      "preconditioner_apply_seconds", "pcg_seconds",
                      "batch_scratch_forecast_bytes", "memory_forecast_bytes",
                      "solver_iterations"):
            require(finite(row, field) >= 0, f"invalid {field}")
        require(finite(row, "memory_forecast_bytes") <= finite(row, "declared_memory_gib") * 1024**3,
                "forecast exceeds declared memory envelope")
        identity(row, "plugin", experiment)
        identity(row, "corrected", experiment)
        if row["algorithm_selected"] == "jla":
            require(row["preconditioner_selected"] in {"diagonal", "cmg"}, "bad selected route")
            require(row["routing_reason"] and row["fallback_status"], "untyped routing")
            if row["preconditioner_selected"] == "cmg":
                require(finite(row, "route_hybrid_vertices") >= 2 and
                        finite(row, "route_hybrid_edges") >= 1 and
                        1 <= finite(row, "route_terminal_vertices") <= 6144,
                        "invalid CMG hybrid graph")
            if plan_row["stage"] == "selector":
                validate_selector_route(row, experiment)
            if plan_row["dataset"] == "cz18" and plan_row["stage"] in {
                    "cz18_preflight", "calibration", "full", "stress2x"}:
                require(row["preconditioner_selected"] == "cmg",
                        "CZ18-scale route did not select CMG")
                require(finite(row, "route_hierarchy_levels") > 1 and
                        1 <= finite(row, "route_terminal_vertices") <= 6144,
                        "CZ18-scale CMG did not reach a bounded multilevel terminal")
            expected_rhs = 3 * int(spec["probes"]) + 1
            require(int(finite(row, "route_planned_rhs")) == expected_rhs, "planned RHS changed")
            aggregate_residual = finite(row, "solver_max_residual")
            require(0 <= aggregate_residual <=
                    COMPLETE_RESIDUAL_GATE * (1 + 1e-10),
                    "complete residual gate failed")
            rhs_rows = read_rows(root / f"rhs_repetition_{index}.csv")
            require(len(rhs_rows) == expected_rhs, f"RHS certificate count changed: {experiment}")
            require(all(finite(rhs, "converged") == 1 for rhs in rhs_rows),
                    f"unaccepted RHS: {experiment}")
            require(all(finite(rhs, "iterations") >= 0 and
                        finite(rhs, "iterations").is_integer()
                        for rhs in rhs_rows),
                    f"invalid RHS iteration certificate: {experiment}")
            require(int(finite(row, "solver_iterations")) ==
                    int(max(finite(rhs, "iterations") for rhs in rhs_rows)),
                    f"aggregate and per-RHS iteration certificates disagree: {experiment}")
            require(all(finite(rhs, "relative_residual") >= 0 for rhs in rhs_rows),
                    f"negative RHS residual: {experiment}")
            rhs_residual = max(finite(rhs, "relative_residual") for rhs in rhs_rows)
            require(rhs_residual <= COMPLETE_RESIDUAL_GATE * (1 + 1e-10),
                    f"complete RHS residual failed: {experiment}")
            require(close(aggregate_residual, rhs_residual, 1e-10),
                    "aggregate and per-RHS residual certificates disagree")
            worst_residual = max(worst_residual, rhs_residual)
    if plan_row["stage"] != "license":
        sample = root / "retained_matches.csv"
        require(read_rows(sample), f"empty retained match certificate: {experiment}")
        rows[0]["_retained_sha256"] = file_sha256(sample)
    if plan_row["stage"] == "full":
        retained_sha = read_text(root / "retained_sample.sha256").strip()
        require(HEX64.fullmatch(retained_sha) is not None, "invalid retained-sample hash")
        retained_dta = root / "retained_sample.dta"
        if retained_dta.is_file():
            require(file_sha256(retained_dta) == retained_sha, "retained-sample hash mismatch")
        rows[0]["_retained_sample_sha256"] = retained_sha
    if plan_row["stage"] == "stress2x" and len(rows) > 1:
        for field in ("plugin_worker", "plugin_firm", "plugin_covariance", "plugin_total",
                      "corrected_worker", "corrected_firm", "corrected_covariance", "corrected_total"):
            require(rows[0][field] == rows[1][field], f"stress2x not byte-deterministic: {field}")
    return rows, worst_residual


def validate_selection(run_dir: Path, source_commit: str, bundle_sha: str,
                       manifest_sha: str, prepared_sha: str, wage_sha: str) -> dict[str, str]:
    path = run_dir / "experiments/calibration_selector/calibration_selection.csv"
    rows = read_rows(path)
    require(len(rows) == 1, "calibration selector must emit one row")
    row = rows[0]
    require(row["bundle_sha256"] == bundle_sha and row["source_commit"] == source_commit,
            "selection source identity mismatch")
    require(row["data_manifest_sha256"] == manifest_sha, "selection manifest mismatch")
    require(row["prepared_sha256"] == prepared_sha and row["wage_input_sha256"] == wage_sha,
            "selection input identity mismatch")
    require(row["formula"] == FORMULA and int(finite(row, "full_probes")) == FULL_PROBES,
            "selection projection contract changed")
    require(row["selected_preconditioner"] == "cmg",
            "CZ18 production selection did not qualify CMG")
    require(row["preconditioner"] == "auto",
            "CZ18 production did not select the public automatic route")
    require(row["batch_request"] in {"8", "16", "auto"},
            "invalid selected batch request")
    require(int(finite(row, "batch")) in FEASIBLE_EXPLICIT_BATCHES,
            "production selection did not freeze a feasible explicit batch")
    selected_ids = {
        key: row[key] for key in (
            "p20_cold_experiment", "p20_warm_experiment",
            "p40_cold_experiment", "p40_warm_experiment",
        )
    }
    require(all(CALIBRATION_PATTERN.fullmatch(experiment)
                for experiment in selected_ids.values()),
            "selection references an invalid calibration cell")
    selected_rows = {
        key: read_rows(run_dir / "experiments" / experiment /
                       f"prod_{experiment}.csv")[0]
        for key, experiment in selected_ids.items()
    }
    measurements = [
        {
            "probes": probes,
            "temperature": temperature,
            "command_seconds": finite(
                selected_rows[f"p{probes}_{temperature}_experiment"],
                "command_seconds",
            ),
            "correction_seconds": finite(
                selected_rows[f"p{probes}_{temperature}_experiment"],
                "correction_seconds",
            ),
            "qacct_wall_seconds": calibration_qacct_wall_seconds(
                run_dir, selected_ids[f"p{probes}_{temperature}_experiment"]
            ),
        }
        for probes in (20, 40) for temperature in ("cold", "warm")
    ]
    t20 = max(float(item["command_seconds"]) for item in measurements
              if item["probes"] == 20)
    t40 = max(float(item["command_seconds"]) for item in measurements
              if item["probes"] == 40)
    alpha, beta, headroom, projected, timeout = \
        recompute_calibration_projection(measurements)
    require(close(finite(row, "t20_seconds"), t20, 1e-12) and
            close(finite(row, "t40_seconds"), t40, 1e-12),
            "selection paired timings changed")
    require(close(finite(row, "alpha_seconds"), alpha, 1e-12) and
            close(finite(row, "beta_seconds_per_probe"), beta, 1e-12),
            "selection setup/slope decomposition changed")
    require(close(finite(row, "headroom_seconds"), headroom, 1e-12),
            "selection wrapper/save headroom changed")
    require(close(finite(row, "projected_seconds"), projected, 1e-9),
            "calibrated projection mismatch")
    require(finite(row, "timeout_seconds") == timeout <= MAXIMUM_TIMEOUT_SECONDS,
            "calibrated timeout inadmissible")
    require(int(finite(row, "measured_candidate_count")) == 6 and
            1 <= int(finite(row, "candidate_count")) <= 6,
            "selection candidate accounting changed")
    require(1 <= int(finite(row, "automatic_candidate_count")) <= 3,
            "automatic candidate accounting changed")
    require(close(finite(row, "batch_memory_fraction"), BATCH_MEMORY_FRACTION, 1e-15),
            "batch memory fraction changed")
    require(int(finite(row, "batch_memory_budget_bytes")) == batch_memory_budget_bytes(),
            "batch memory budget changed")
    require(int(finite(row, "batch8_scratch_forecast_bytes")) ==
            BATCH_BASE_SCRATCH_BYTES, "batch-8 forecast changed")
    for width in (16, 32, 64, 128):
        require(int(finite(row, f"batch{width}_scratch_forecast_bytes")) ==
                forecast_batch_bytes(width), f"batch-{width} forecast changed")
    require(row["feasible_batch_widths"] == "8|16|auto" and
            row["infeasible_batch_widths"] == "32|64|128",
            "batch feasibility certificate changed")
    require(all(forecast_batch_bytes(width) <= batch_memory_budget_bytes()
                for width in FEASIBLE_EXPLICIT_BATCHES) and
            all(forecast_batch_bytes(width) > batch_memory_budget_bytes()
                for width in INFEASIBLE_BATCHES),
            "registered batch feasibility classification is inconsistent")
    admissible: list[dict[str, object]] = []
    for route in ("auto", "cmg"):
        for batch_request in ("8", "16", "auto"):
            candidate_rows = {
                (probes, temperature): read_rows(
                    run_dir / "experiments" /
                    f"cal_{route}_b{batch_request}_p{probes}_{temperature}" /
                    f"prod_cal_{route}_b{batch_request}_p{probes}_{temperature}.csv"
                )[0]
                for probes in (20, 40) for temperature in ("cold", "warm")
            }
            selected_batches = {int(finite(value, "selected_batch"))
                                for value in candidate_rows.values()}
            selected_routes = {value["preconditioner_selected"]
                               for value in candidate_rows.values()}
            candidate_measurements = [
                {
                    "probes": probes,
                    "temperature": temperature,
                    "command_seconds": finite(candidate_rows[(probes, temperature)],
                                              "command_seconds"),
                    "correction_seconds": finite(candidate_rows[(probes, temperature)],
                                                 "correction_seconds"),
                    "qacct_wall_seconds": calibration_qacct_wall_seconds(
                        run_dir, f"cal_{route}_b{batch_request}_p{probes}_{temperature}"
                    ),
                }
                for probes in (20, 40) for temperature in ("cold", "warm")
            ]
            candidate_alpha, candidate_beta, candidate_headroom, \
                candidate_projected, candidate_timeout = \
                recompute_calibration_projection(candidate_measurements)
            if (len(selected_batches) != 1 or selected_routes != {"cmg"} or
                    candidate_timeout > MAXIMUM_TIMEOUT_SECONDS):
                continue
            admissible.append({
                "preconditioner": route,
                "batch": next(iter(selected_batches)),
                "projected_seconds": candidate_projected,
                "timeout_seconds": candidate_timeout,
                "p20_cold_experiment": f"cal_{route}_b{batch_request}_p20_cold",
                "p20_warm_experiment": f"cal_{route}_b{batch_request}_p20_warm",
                "p40_cold_experiment": f"cal_{route}_b{batch_request}_p40_cold",
                "p40_warm_experiment": f"cal_{route}_b{batch_request}_p40_warm",
                "alpha_seconds": candidate_alpha,
                "beta_seconds_per_probe": candidate_beta,
                "headroom_seconds": candidate_headroom,
            })
    require(admissible and int(finite(row, "candidate_count")) == len(admissible),
            "admissible candidate count changed")
    admissible_auto = [candidate for candidate in admissible
                       if candidate["preconditioner"] == "auto"]
    require(admissible_auto and
            int(finite(row, "automatic_candidate_count")) == len(admissible_auto),
            "admissible automatic candidate count changed")
    preferred = min(admissible_auto, key=candidate_sort_key)
    for field in ("preconditioner", "batch", "p20_cold_experiment",
                  "p20_warm_experiment", "p40_cold_experiment",
                  "p40_warm_experiment"):
        require(str(row[field]) == str(preferred[field]),
                f"selector did not preserve automatic-route tie preference: {field}")
    calibration_ids = [path.parent.name for path in sorted(
        (run_dir / "experiments").glob("cal_*_cold/prod_cal_*_cold.csv"))]
    both_ids = [item for cold in calibration_ids
                for item in (cold, cold.removesuffix("_cold") + "_warm")]
    require(row["calibration_evidence_sha256"] == evidence_digest(run_dir, both_ids),
            "calibration evidence changed after selection")
    return row


def phase_evidence_paths(run_dir: Path, plan: list[dict[str, str]],
                         phase: str) -> list[Path]:
    paths: list[Path] = []
    for row in plan:
        if row["phase"] != phase:
            continue
        experiment = row["experiment_id"]
        root = run_dir / "experiments" / experiment
        require(root.is_dir(), f"missing experiment directory: {experiment}")
        paths.extend(
            path for path in root.rglob("*")
            if path.is_file() and path.suffix.lower() != ".dta" and
            "plus" not in path.relative_to(root).parts
        )
        paths.append(run_dir / "qacct" / f"{experiment}.txt")
        paths.append(run_dir / "submissions" / f"{experiment}.job_id")
    unique = sorted(set(paths), key=lambda path: str(path.relative_to(run_dir)))
    require(unique and all(path.is_file() for path in unique),
            f"phase evidence is incomplete: {phase}")
    return unique


def verify_phase_manifest(run_dir: Path, plan: list[dict[str, str]],
                          phase: str, expected_sha: str) -> None:
    manifest = run_dir / "validation" / f"{phase}.evidence.sha256"
    require(file_sha256(manifest) == expected_sha,
            f"accepted {phase} evidence manifest changed")
    expected_paths = phase_evidence_paths(run_dir, plan, phase)
    expected_rows = [(file_sha256(path), str(path.relative_to(run_dir)))
                     for path in expected_paths]
    observed_rows: list[tuple[str, str]] = []
    for line in read_text(manifest).splitlines():
        digest, separator, relative = line.partition("  ")
        require(separator == "  " and HEX64.fullmatch(digest) is not None,
                f"invalid {phase} evidence-manifest row")
        observed_rows.append((digest, relative))
    require(observed_rows == expected_rows,
            f"accepted {phase} evidence files or hashes changed")


def validate_phase_pass(run_dir: Path, plan: list[dict[str, str]], phase: str,
                        source_commit: str, bundle_sha: str,
                        manifest_sha: str) -> dict[str, str]:
    lines = read_text(run_dir / "validation" / f"{phase}.pass").splitlines()
    values = dict(line.split("=", 1) for line in lines if "=" in line)
    require(values.get("status") == "PASS" and values.get("phase") == phase,
            f"prior {phase} certificate invalid")
    require(values.get("source_commit") == source_commit and
            values.get("bundle_sha256") == bundle_sha and
            values.get("data_manifest_sha256") == manifest_sha,
            f"prior {phase} certificate identity mismatch")
    evidence_sha = values.get("evidence_manifest_sha256", "")
    require(HEX64.fullmatch(evidence_sha) is not None,
            f"prior {phase} certificate omits its evidence digest")
    verify_phase_manifest(run_dir, plan, phase, evidence_sha)
    return values


def write_phase_pass(run_dir: Path, plan: list[dict[str, str]], phase: str,
                     source_commit: str,
                     bundle_sha: str, manifest_sha: str, worst_residual: float,
                     selection_sha: str | None) -> None:
    directory = run_dir / "validation"
    directory.mkdir(parents=True, exist_ok=True)
    path = directory / f"{phase}.pass"
    evidence_manifest = directory / f"{phase}.evidence.sha256"
    evidence_payload = "".join(
        f"{file_sha256(item)}  {item.relative_to(run_dir)}\n"
        for item in phase_evidence_paths(run_dir, plan, phase)
    )
    evidence_temporary = evidence_manifest.with_name(
        evidence_manifest.name + f".tmp.{os.getpid()}")
    evidence_temporary.write_text(evidence_payload, encoding="utf-8")
    os.replace(evidence_temporary, evidence_manifest)
    evidence_sha = file_sha256(evidence_manifest)
    lines = [
        "status=PASS", f"phase={phase}", f"bundle_sha256={bundle_sha}",
        f"source_commit={source_commit}", f"data_manifest_sha256={manifest_sha}",
        f"complete_residual_gate={COMPLETE_RESIDUAL_GATE:.17g}",
        f"worst_complete_residual={worst_residual:.17g}",
        f"all_complete_residuals_within_requested_tolerance={str(worst_residual <= REQUESTED_TOLERANCE).lower()}",
        f"evidence_manifest_sha256={evidence_sha}",
    ]
    if selection_sha is not None:
        lines.append(f"selection_sha256={selection_sha}")
    payload = "\n".join(lines) + "\n"
    temporary = path.with_name(path.name + f".tmp.{os.getpid()}")
    temporary.write_text(payload, encoding="utf-8")
    os.replace(temporary, path)


def main() -> int:
    parser = argparse.ArgumentParser()
    default_root = Path(__file__).resolve().parents[1]
    parser.add_argument("--plan", type=Path, default=default_root / "benchmarks/prod_experiments.tsv")
    parser.add_argument("--static", action="store_true")
    parser.add_argument("--run-dir", type=Path)
    parser.add_argument("--bundle-sha")
    parser.add_argument("--source-commit")
    parser.add_argument("--data-manifest", type=Path)
    parser.add_argument("--phase", choices=("preflight", "calibration", "production"))
    parser.add_argument("--write-pass", action="store_true")
    args = parser.parse_args()
    plan = load_plan(args.plan)
    if args.static:
        print(f"KSS_PROD SCC STATIC PLAN PASS: {len(plan)} experiments")
        return 0
    require(args.run_dir is not None and args.bundle_sha is not None and
            args.source_commit is not None and args.data_manifest is not None and
            args.phase is not None, "evidence mode requires run, bundle, data manifest, and phase")
    require(HEX64.fullmatch(args.bundle_sha) is not None, "invalid expected bundle SHA")
    require(HEX40.fullmatch(args.source_commit) is not None, "invalid expected source commit")
    metadata = json.loads(read_text(args.run_dir / "run.metadata.json"))
    require(metadata["source_commit"] == args.source_commit and
            metadata["bundle_sha256"] == args.bundle_sha, "run metadata identity mismatch")
    require(read_text(args.run_dir / "source_commit.txt").strip() == args.source_commit,
            "run source mismatch")
    require(read_text(args.run_dir / "bundle.sha256").strip() == args.bundle_sha,
            "run bundle mismatch")
    frozen_manifest = args.run_dir / "input/data_manifest.tsv"
    manifest_sha = file_sha256(frozen_manifest)
    require(read_text(args.run_dir / "input/data_manifest.sha256").strip() == manifest_sha,
            "frozen data manifest hash mismatch")
    require(file_sha256(args.data_manifest) == manifest_sha,
            "validator data manifest differs from frozen run manifest")
    data_rows = load_data_manifest(frozen_manifest)
    ledger_rows = read_rows(args.run_dir / "submissions/ledger.tsv")
    require(len({row["experiment_id"] for row in ledger_rows}) == len(ledger_rows),
            "duplicate submission ledger row")
    ledger = {row["experiment_id"]: row for row in ledger_rows}
    if args.phase in {"calibration", "production"}:
        validate_phase_pass(args.run_dir, plan, "preflight", args.source_commit,
                            args.bundle_sha, manifest_sha)
    if args.phase == "production":
        calibration_pass = validate_phase_pass(
            args.run_dir, plan, "calibration", args.source_commit,
            args.bundle_sha, manifest_sha)
        selection_path = args.run_dir / "experiments/calibration_selector/calibration_selection.csv"
        require(calibration_pass.get("selection_sha256") == file_sha256(selection_path),
                "selection changed after calibration acceptance")

    phases = {"preflight"}
    if args.phase in {"calibration", "production"}:
        phases.add("calibration")
    if args.phase == "production":
        phases.add("production")
    prepared_hashes: dict[str, str] = {}
    outputs: dict[str, list[dict[str, str]]] = {}
    hierarchy_outputs: dict[str, dict[str, str]] = {}
    selection: dict[str, str] | None = None
    worst_residual = 0.0
    stress_parent_sha: str | None = None
    phase_by_experiment = {row["experiment_id"]: row["phase"] for row in plan}
    for row in plan:
        if row["phase"] not in phases:
            continue
        spec = expected_spec(row, selection)
        validate_submission(
            args.run_dir, row["experiment_id"], int(spec["processors"]),
            int(spec["memory_gib"]), expected_timeout(row, selection),
            args.bundle_sha, manifest_sha, ledger, row["depends"],
            phase_by_experiment)
        validate_qacct(args.run_dir, row["experiment_id"], int(spec["processors"]))
        stage = row["stage"]
        if stage == "prepare":
            prepared_hashes[row["dataset"]] = validate_prepare(
                args.run_dir, row, args.source_commit, args.bundle_sha, manifest_sha,
                data_rows[row["dataset"]])
        elif stage == "hierarchy_stress":
            hierarchy_outputs[row["experiment_id"]] = validate_hierarchy(
                args.run_dir, row, args.source_commit, args.bundle_sha, manifest_sha)
        elif stage == "sample_compare":
            validate_comparison(args.run_dir, row, args.source_commit, args.bundle_sha,
                                manifest_sha, prepared_hashes[row["dataset"]],
                                data_rows[row["dataset"]])
        elif stage == "calibration_selector":
            root = args.run_dir / "experiments" / row["experiment_id"]
            validate_resource(root / "resources.txt")
            require("KSS_PROD CALIBRATION SELECTOR PASS" in
                    read_text(root / "application.txt"),
                    "calibration selector application marker absent")
            validate_wrapper(root, row["experiment_id"], args.source_commit,
                             args.bundle_sha, manifest_sha)
            selection = validate_selection(
                args.run_dir, args.source_commit, args.bundle_sha, manifest_sha,
                prepared_hashes["cz18"], data_rows["cz18"]["wage_input_sha256"])
        else:
            result, residual = validate_result(
                args.run_dir, row, args.source_commit, args.bundle_sha, manifest_sha,
                prepared_hashes, data_rows, selection, stress_parent_sha)
            outputs[row["experiment_id"]] = result
            worst_residual = max(worst_residual, residual)
            if stage == "full":
                stress_parent_sha = result[0]["_retained_sample_sha256"]

    for experiment, cold in outputs.items():
        if not experiment.endswith("_cold"):
            continue
        warm_name = experiment.removesuffix("_cold") + "_warm"
        if warm_name not in outputs:
            continue
        warm = outputs[warm_name]
        cold_job = read_text(args.run_dir / "submissions" / f"{experiment}.job_id").strip()
        warm_job = read_text(args.run_dir / "submissions" / f"{warm_name}.job_id").strip()
        require(cold_job != warm_job, "cold/warm evidence reused one process")
        require(cold[0]["prepared_sha256"] == warm[0]["prepared_sha256"] and
                cold[0].get("_retained_sha256") == warm[0].get("_retained_sha256"),
                "cold/warm scientific input changed")
        for field in ("plugin_worker", "plugin_firm", "plugin_covariance", "plugin_total",
                      "corrected_worker", "corrected_firm", "corrected_covariance", "corrected_total"):
            require(close(finite(cold[0], field), finite(warm[0], field), 1e-10),
                    f"cold/warm result changed: {experiment} {field}")

    if hierarchy_outputs:
        require(set(hierarchy_outputs) == {"hierarchy_scc_p4"},
                "unexpected hierarchy processor matrix")

    for dataset in ("cz24", "cz25"):
        fixed = [rows[0] for name, rows in outputs.items() if name.startswith(f"fixed_{dataset}")]
        require(len(fixed) == 4, f"fixed route matrix incomplete for {dataset}")
        require(len({row["_retained_sha256"] for row in fixed}) == 1,
                f"{dataset} routes retained different samples")
        reference = fixed[0]
        for candidate in fixed[1:]:
            for field in ("plugin_worker", "plugin_firm", "plugin_covariance", "plugin_total",
                          "corrected_worker", "corrected_firm", "corrected_covariance", "corrected_total"):
                require(close(finite(reference, field), finite(candidate, field), 2e-9),
                        f"{dataset} route equality failed: {field}")

    if args.phase in {"calibration", "production"}:
        for route in ("auto", "cmg"):
            for probes in (20, 40):
                for temperature in ("cold", "warm"):
                    names = [f"cal_{route}_b{batch}_p{probes}_{temperature}"
                             for batch in ("8", "16", "auto")]
                    require(all(name in outputs for name in names),
                            "feasible cross-batch matrix incomplete")
                    reference = outputs[names[0]][0]
                    require(int(finite(outputs[names[0]][0], "selected_batch")) == 8 and
                            int(finite(outputs[names[1]][0], "selected_batch")) == 16 and
                            int(finite(outputs[names[2]][0], "selected_batch")) in
                            FEASIBLE_EXPLICIT_BATCHES,
                            "selected feasible batches do not match requests")
                    for name in names[1:]:
                        candidate = outputs[name][0]
                        require(reference["_retained_sha256"] == candidate["_retained_sha256"],
                                "batch changed retained sample")
                        for field in (
                            "plugin_worker", "plugin_firm", "plugin_covariance", "plugin_total",
                            "corrected_worker", "corrected_firm", "corrected_covariance",
                            "corrected_total",
                        ):
                            require(close(finite(reference, field), finite(candidate, field), 2e-9),
                                    f"batch invariance failed: {route} P{probes} "
                                    f"{temperature} {field}")
        for route in ("auto", "cmg"):
            for batch in ("8", "16", "auto"):
                for temperature in ("cold", "warm"):
                    low = outputs[f"cal_{route}_b{batch}_p20_{temperature}"][0]
                    high = outputs[f"cal_{route}_b{batch}_p40_{temperature}"][0]
                    require(low["_retained_sha256"] == high["_retained_sha256"],
                            "P20/P40 retained sample changed")
                    for field in ("plugin_worker", "plugin_firm", "plugin_covariance",
                                  "plugin_total"):
                        require(close(finite(low, field), finite(high, field), 1e-10),
                                f"P20/P40 plug-in changed: {route} {batch} {field}")
                    for field in ("selected_batch", "N_retained", "worker_levels",
                                  "firm_levels", "deletion_units", "route_hybrid_vertices",
                                  "route_hybrid_edges", "route_hierarchy_levels"):
                        require(finite(low, field) == finite(high, field),
                                f"P20/P40 graph or route changed: {route} {batch} {field}")
    if args.phase == "production":
        full = outputs["cz18_full200"][0]
        require(full["preconditioner_selected"] == "cmg",
                "full CZ18 estimator did not use qualified CMG")
        require(finite(full, "route_hybrid_vertices") > 6144 and
                finite(full, "route_hierarchy_levels") > 1,
                "full CZ18 did not exercise the large-hybrid multilevel path")
        for name in ("cz18_stress2x_cal20", "cz18_stress2x_full200"):
            stress = outputs[name][0]
            require(stress["preconditioner_selected"] == "cmg" and
                    finite(stress, "route_hierarchy_levels") > 1 and
                    1 <= finite(stress, "route_terminal_vertices") <= 6144,
                    "larger stress case did not use bounded multilevel CMG")
            require(finite(stress, "N_retained") >= 2 * finite(full, "N_retained"),
                    "stress case is not at least twice CZ18 rows")
            require(finite(stress, "worker_levels") > 2 * finite(full, "worker_levels"),
                    "stress case did not exceed twice CZ18 workers")
            require(finite(stress, "firm_levels") >= 2 * finite(full, "firm_levels"),
                    "stress case did not reach twice CZ18 firms")
            require(finite(stress, "route_hybrid_vertices") >= 2 * finite(full, "route_hybrid_vertices"),
                    "stress case did not reach twice CZ18 hybrid vertices")
        stress_cal = outputs["cz18_stress2x_cal20"][0]
        stress_full = outputs["cz18_stress2x_full200"][0]
        require(stress_cal["_retained_sha256"] == stress_full["_retained_sha256"],
                "stress calibration and full run retained different samples")
        for field in ("N_retained", "worker_levels", "firm_levels",
                      "deletion_units", "route_hybrid_vertices",
                      "route_hybrid_edges", "route_hierarchy_levels",
                      "route_terminal_vertices"):
            require(finite(stress_cal, field) == finite(stress_full, field),
                    f"stress calibration/full graph changed: {field}")
        projection = math.ceil(
            finite(stress_cal, "command_seconds") *
            (FULL_PROBES / finite(stress_cal, "requested_probes")) *
            SAFETY_FACTOR + FIXED_HEADROOM_SECONDS)
        timeout = expected_timeout(
            next(row for row in plan
                 if row["experiment_id"] == "cz18_stress2x_full200"), selection)
        require(projection <= timeout,
                "stress calibration did not justify the full-run timeout")
        projection_text = read_text(
            args.run_dir / "experiments/cz18_stress2x_full200/stress_projection.txt")
        require(f"projected_seconds={projection}" in projection_text and
                f"timeout_seconds={timeout}" in projection_text,
                "stress projection certificate mismatch")

    selection_sha = None
    if selection is not None:
        selection_sha = file_sha256(
            args.run_dir / "experiments/calibration_selector/calibration_selection.csv")
    if args.write_pass:
        write_phase_pass(args.run_dir, plan, args.phase, args.source_commit,
                         args.bundle_sha, manifest_sha, worst_residual,
                         selection_sha)
    within = worst_residual <= REQUESTED_TOLERANCE
    print(f"KSS_PROD SCC {args.phase.upper()} EVIDENCE PASS; "
          f"worst_complete_residual={worst_residual:.6g}; within_requested_tolerance={str(within).lower()}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

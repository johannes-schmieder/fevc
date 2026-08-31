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
import statistics
from pathlib import Path

try:
    from .scc.select_prod_calibration import (
        BATCH_BASE_SCRATCH_BYTES,
        BATCH_MEMORY_FRACTION,
        CALIBRATION_PATTERN,
        CALIBRATION_REPETITIONS,
        COMPLETE_RESIDUAL_GATE,
        FEASIBLE_EXPLICIT_BATCHES,
        FIXED_HEADROOM_SECONDS,
        FORMULA,
        FULL_PROBES,
        FULL_RETAINED_SAVE_ALLOWANCE_SECONDS,
        INFEASIBLE_BATCHES,
        MAXIMUM_TIMEOUT_SECONDS,
        REQUESTED_TOLERANCE,
        SCC_HARD_RUNTIME_CEILING_SECONDS,
        WRAPPER_RUNTIME_MARGIN_SECONDS,
        batch_memory_budget_bytes,
        candidate_sort_key,
        evidence_digest,
        forecast_batch_bytes,
        timing_identity_after_csv,
        timing_identity_diagnostic,
    )
except ImportError:
    from scc.select_prod_calibration import (
        BATCH_BASE_SCRATCH_BYTES,
        BATCH_MEMORY_FRACTION,
        CALIBRATION_PATTERN,
        CALIBRATION_REPETITIONS,
        COMPLETE_RESIDUAL_GATE,
        FEASIBLE_EXPLICIT_BATCHES,
        FIXED_HEADROOM_SECONDS,
        FORMULA,
        FULL_PROBES,
        FULL_RETAINED_SAVE_ALLOWANCE_SECONDS,
        INFEASIBLE_BATCHES,
        MAXIMUM_TIMEOUT_SECONDS,
        REQUESTED_TOLERANCE,
        SCC_HARD_RUNTIME_CEILING_SECONDS,
        WRAPPER_RUNTIME_MARGIN_SECONDS,
        batch_memory_budget_bytes,
        candidate_sort_key,
        evidence_digest,
        forecast_batch_bytes,
        timing_identity_after_csv,
        timing_identity_diagnostic,
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
RHS_FIELDS = (
    "experiment_id", "repetition", "stage", "batch_start", "rhs",
    "iterations", "relative_residual", "converged",
)
HEX64 = re.compile(r"[0-9a-f]{64}")
HEX40 = re.compile(r"[0-9a-f]{40}")
# Aggregate CSVs serialize Stata scalars to about eight significant decimal places.
# This tolerance applies only when rechecking identities in those exported files.
CSV_IDENTITY_SERIALIZATION_TOLERANCE = 1e-7
CALIBRATION_SETUP_SAFETY_FACTOR = 1.25
CALIBRATION_MARGINAL_SAFETY_FACTOR = 1.5
CALIBRATION_MINIMUM_TIMEOUT_SECONDS = 300
STRESS_CALIBRATION_IDS = tuple(
    f"cz18_stress2x_cal20_r{repetition}"
    for repetition in CALIBRATION_REPETITIONS
)
STRESS_NODE_POLICY = "INDEPENDENT_PARALLEL_QSUB_HOSTNAMES_DESCRIPTIVE_NONCAUSAL"
STRESS_PROJECTION_FORMULA = (
    "alpha=max_r(max(0,qacct_wall_r-correction_seconds_r));"
    "beta=max_r(correction_seconds_r/20);"
    "projected=ceil(max(300,1.25*alpha+1.5*200*beta+120));"
    "timeout=max(1800,projected)"
)
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


def validate_correction_timing(row: dict[str, str], experiment: str) -> None:
    # Exact computation reports one enclosing correction timer.  Only JLA has
    # the disjoint leverage and target timers whose sum is the public total.
    if row["algorithm_selected"] != "jla":
        return
    correction = finite(row, "correction_seconds")
    leverage = finite(row, "leverage_seconds")
    target = finite(row, "target_seconds")
    require(timing_identity_after_csv(correction, leverage, target),
            "correction timing identity failed after CSV serialization: "
            f"{experiment}: "
            f"{timing_identity_diagnostic(correction, leverage, target)}")


def load_plan(path: Path) -> list[dict[str, str]]:
    rows = read_rows(path)
    require(rows and tuple(rows[0]) == PLAN_FIELDS, "unexpected production plan columns")
    seen: set[str] = set()
    for row in rows:
        experiment = row["experiment_id"]
        require(re.fullmatch(r"[A-Za-z0-9._-]+", experiment) is not None, "invalid experiment ID")
        require(experiment not in seen, f"duplicate experiment: {experiment}")
        require(row["phase"] in {"preflight", "calibration", "production", "stress"},
                "invalid phase")
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
    require(len(calibration) == 72 and
            all(CALIBRATION_PATTERN.fullmatch(row["experiment_id"])
                for row in calibration),
            "calibration must be the registered 72-job repetition matrix")
    expected_calibrations = {
        f"cal_{route}_b{batch}_p{probes}_{temperature}_r{repetition}"
        for route in ("auto", "cmg")
        for batch in ("8", "16", "auto")
        for probes in (20, 40)
        for temperature in ("cold", "warm")
        for repetition in CALIBRATION_REPETITIONS
    }
    require({row["experiment_id"] for row in calibration} == expected_calibrations,
            "calibration repetition matrix is incomplete")
    require(all(row["depends"] == "cz18_preflight" and row["repetitions"] == "1"
                for row in calibration),
            "calibration repetitions must be independent single jobs")
    selector = [row for row in rows if row["stage"] == "calibration_selector"]
    require(len(selector) == 1 and
            set(selector[0]["depends"].split(",")) == expected_calibrations,
            "calibration selector must wait for every independent repetition")
    require(not any(row["batch"] in {str(width) for width in INFEASIBLE_BATCHES}
                    for row in calibration),
            "forecast-infeasible batch was scheduled")
    production = [row for row in rows if row["phase"] == "production"]
    stress_calibration_ids = [
        f"cz18_stress2x_cal20_r{repetition}"
        for repetition in CALIBRATION_REPETITIONS
    ]
    expected_production = {
        "cz18_full200": {
            "stage": "full", "dataset": "cz18", "stata_version": "19",
            "processors": "0", "memory_gib": "56", "probes": "200",
            "batch": "selected", "preconditioner": "selected",
            "temperature": "cold", "repetitions": "1",
            "depends": "calibration_selector",
        },
        **{
            experiment: {
                "stage": "stress2x", "dataset": "cz18",
                "stata_version": "19", "processors": "0",
                "memory_gib": "56", "probes": "20", "batch": "auto",
                "preconditioner": "selected", "temperature": "cold",
                "repetitions": "1", "depends": "cz18_full200",
            }
            for experiment in stress_calibration_ids
        },
    }
    require({row["experiment_id"] for row in production} ==
            set(expected_production),
            "production full/stress repetition matrix is incomplete")
    for row in production:
        expected = expected_production[row["experiment_id"]]
        require(all(row[field] == value for field, value in expected.items()),
                f"production configuration/dependencies changed: "
                f"{row['experiment_id']}")
    stress = [row for row in rows if row["phase"] == "stress"]
    require(len(stress) == 1 and stress[0]["experiment_id"] ==
            "cz18_stress2x_full200",
            "stress phase must contain exactly the full larger-than-CZ18 run")
    expected_stress = {
        "stage": "stress2x", "dataset": "cz18", "stata_version": "19",
        "processors": "0", "memory_gib": "56", "probes": "200",
        "batch": "auto", "preconditioner": "selected",
        "temperature": "cold", "repetitions": "1",
        "depends": ",".join(stress_calibration_ids),
    }
    require(all(stress[0][field] == value
                for field, value in expected_stress.items()),
            "full stress configuration/dependencies changed")
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


def parse_duration(value: str) -> float:
    try:
        parsed = float(value)
    except ValueError:
        match = re.fullmatch(r"(\d+):(\d{2}):(\d{2}(?:\.\d+)?)", value)
        require(match is not None, f"unparseable accounting duration: {value}")
        parsed = (3600 * int(match.group(1)) + 60 * int(match.group(2)) +
                  float(match.group(3)))
    require(math.isfinite(parsed) and parsed >= 0,
            f"invalid accounting duration: {value}")
    return parsed


def calibration_experiment_id(route: str, batch: str, probes: int,
                              temperature: str, repetition: int) -> str:
    return f"cal_{route}_b{batch}_p{probes}_{temperature}_r{repetition}"


def validate_node_characteristics(path: Path, qacct_hostname: str) -> dict[str, str]:
    values: dict[str, str] = {}
    for line in read_text(path).splitlines():
        key, separator, value = line.partition("=")
        require(separator == "=" and key not in values,
                f"invalid node characteristics row: {path}")
        values[key] = value
    require(set(values) == {
        "schema", "hostname", "uname_machine", "cpu_model", "logical_cpus",
    }, f"node characteristics schema changed: {path}")
    require(values["schema"] == "kss_prod_node_v1" and
            re.fullmatch(r"[^\s,|]+", values["hostname"]) is not None and
            values["uname_machine"] != "" and values["cpu_model"] != "" and
            values["logical_cpus"].isdigit() and int(values["logical_cpus"]) >= 1,
            f"invalid node characteristics values: {path}")
    require(values["hostname"].split(".", 1)[0] ==
            qacct_hostname.split(".", 1)[0],
            f"node and qacct host disagree: {path}")
    return values


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


def recompute_conservative_calibration_projection(
    measurements: list[dict[str, float | str]],
) -> tuple[float, float, float, float, int, str]:
    expected = 2 * 2 * len(CALIBRATION_REPETITIONS)
    require(len(measurements) == expected,
            f"conservative projection requires {expected} measurements")
    cells: dict[tuple[int, str], list[dict[str, float | str]]] = {}
    for row in measurements:
        probes = int(row["probes"])
        temperature = str(row["temperature"])
        command = float(row["command_seconds"])
        correction = float(row["correction_seconds"])
        wall = float(row["qacct_wall_seconds"])
        hostname = str(row["hostname"])
        require(probes in {20, 40} and temperature in {"cold", "warm"} and
                hostname != "" and all(math.isfinite(value)
                for value in (command, correction, wall)) and
                command >= 0 and correction >= 0 and wall >= 0,
                "invalid conservative projection timing/accounting")
        require(correction <= command + 1.0,
                "correction timer exceeds enclosing command timer")
        cells.setdefault((probes, temperature), []).append(row)
    require(set(cells) == {(probes, temperature) for probes in (20, 40)
                           for temperature in ("cold", "warm")} and
            all(len(rows) == len(CALIBRATION_REPETITIONS)
                for rows in cells.values()),
            "conservative repetition matrix is incomplete")
    slopes: list[float] = []
    modes: list[str] = []
    for temperature in ("cold", "warm"):
        low = cells[(20, temperature)]
        high = cells[(40, temperature)]
        slopes.append(
            (max(float(row["command_seconds"]) for row in high) -
             min(float(row["command_seconds"]) for row in low)) / 20.0
        )
        modes.append(f"{temperature}=cross_host_upper_envelope")
    beta = max(
        0.0,
        *slopes,
        *(float(row["correction_seconds"]) / int(row["probes"])
          for row in measurements),
    )
    alpha = max(0.0, *(float(row["command_seconds"]) -
                       int(row["probes"]) * beta for row in measurements))
    for row in measurements:
        probes = int(row["probes"])
        require(alpha + probes * beta + 1e-9 >=
                float(row["command_seconds"]),
                "conservative projection does not cover every command timing")
        require(probes * beta + 1e-9 >= float(row["correction_seconds"]),
                "conservative projection does not cover every correction timing")
    cold_overhead = max(
        0.0,
        *(float(row["qacct_wall_seconds"]) - float(row["command_seconds"])
          for row in measurements if row["temperature"] == "cold"),
    )
    headroom = max(
        float(FIXED_HEADROOM_SECONDS),
        cold_overhead + FULL_RETAINED_SAVE_ALLOWANCE_SECONDS,
    )
    projected = (CALIBRATION_SETUP_SAFETY_FACTOR * alpha +
                 CALIBRATION_MARGINAL_SAFETY_FACTOR * FULL_PROBES * beta +
                 headroom)
    timeout = math.ceil(max(CALIBRATION_MINIMUM_TIMEOUT_SECONDS, projected))
    return alpha, beta, headroom, projected, timeout, "|".join(modes)


def recompute_same_host_coincidences(
    measurements: list[dict[str, float | str]],
) -> str:
    parts: list[str] = []
    for temperature in ("cold", "warm"):
        low_hosts = {str(row["hostname"]) for row in measurements
                     if row["probes"] == 20 and row["temperature"] == temperature}
        high_hosts = {str(row["hostname"]) for row in measurements
                      if row["probes"] == 40 and row["temperature"] == temperature}
        common = "&".join(sorted(low_hosts & high_hosts)) or "-"
        parts.append(f"{temperature}={common}")
    return "|".join(parts)


def recompute_node_class_multiset(
    measurements: list[dict[str, float | str]],
) -> str:
    classes = sorted([
        (str(row["uname_machine"]), str(row["cpu_model"]),
         int(row["logical_cpus"]))
        for row in measurements
    ])
    require(len(classes) == 12, "node-class multiset requires 12 repetitions")
    return json.dumps(classes, separators=(",", ":"))


def conservative_selection_key(row: dict[str, object]) -> tuple[object, ...]:
    return (
        row["timeout_seconds"], row["projected_seconds"], row["batch"],
        {"8": 0, "16": 1, "auto": 2}[str(row["batch_request"])],
    )


def recompute_timing_inversion(
    summaries: list[dict[str, float | str]],
) -> tuple[str, str]:
    by_cell = {(int(row["probes"]), str(row["temperature"])): row
               for row in summaries}
    inverted = [temperature for temperature in ("cold", "warm")
                if float(by_cell[(40, temperature)]["command_seconds"]) <
                float(by_cell[(20, temperature)]["command_seconds"])]
    if not inverted:
        return "NONE", "-"
    return "UNEXPLAINED_VARIABILITY", "|".join(inverted)


def validate_qacct(run_dir: Path, experiment: str, processors: int, *,
                   stage: str, bundle_sha: str, source_commit: str,
                   manifest_sha: str) -> dict[str, str]:
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
    parse_duration(result["ru_wallclock"])
    parse_duration(result["cpu"])
    require(re.fullmatch(r"[^\s,|]+", result["hostname"]) is not None and
            re.fullmatch(r"[^\s,|]+", result["qname"]) is not None,
            f"qacct {experiment}: invalid host/queue accounting")
    metadata_path = run_dir / "experiments" / experiment / "run.metadata.txt"
    tokens = read_text(metadata_path).strip().split()
    metadata: dict[str, str] = {}
    for token in tokens:
        key, separator, value = token.partition("=")
        require(separator == "=" and key not in metadata,
                f"invalid run metadata token: {experiment}")
        metadata[key] = value
    require(set(metadata) == {
        "experiment", "stage", "bundle", "source", "manifest", "job", "host", "slots",
    }, f"run metadata schema changed: {experiment}")
    expected_metadata = {
        "experiment": experiment, "stage": stage, "bundle": bundle_sha,
        "source": source_commit, "manifest": manifest_sha,
        "job": submitted.split(".", 1)[0], "slots": str(processors),
    }
    require(all(metadata[key] == value for key, value in expected_metadata.items()) and
            metadata["host"].split(".", 1)[0] ==
            result["hostname"].split(".", 1)[0],
            f"run metadata identity changed: {experiment}")
    if CALIBRATION_PATTERN.fullmatch(experiment) or stage in {"full", "stress2x"}:
        node = validate_node_characteristics(
            run_dir / "experiments" / experiment / "node_characteristics.txt",
            result["hostname"],
        )
        result.update({f"node_{key}": value for key, value in node.items()})
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


def read_key_value_certificate(path: Path, label: str) -> dict[str, str]:
    values: dict[str, str] = {}
    for line in read_text(path).splitlines():
        key, separator, value = line.partition("=")
        require(separator == "=" and key != "" and key not in values,
                f"invalid {label} certificate row: {path}")
        values[key] = value
    require(values, f"empty {label} certificate: {path}")
    return values


def expected_timeout(row: dict[str, str],
                     selection: dict[str, str] | None,
                     run_dir: Path | None = None) -> int:
    stage = row["stage"]
    fixed = {
        "bundle_smoke": 600, "install_auto": 600, "install_cmg": 600,
        "license": 600, "hierarchy_stress": 1800,
        "sample_compare": 1800, "cz18_preflight": 2100,
        "selector": 900, "calibration_selector": 900,
        "prepare": 3600, "fixed": 3600, "calibration": 5400,
        "full": MAXIMUM_TIMEOUT_SECONDS,
        "stress2x": MAXIMUM_TIMEOUT_SECONDS,
    }
    timeout = fixed[stage]
    if row["phase"] == "production":
        require(selection is not None, "production timeout selected too late")
        field = ("stress_calibration_timeout_seconds"
                 if stage == "stress2x" else "timeout_seconds")
        timeout = int(float(selection[field]))
    if row["phase"] == "stress":
        require(run_dir is not None,
                "stress timeout requires its pre-submission projection certificate")
        certificate = read_key_value_certificate(
            run_dir / "validation/stress_projection.txt",
            "stress projection",
        )
        require(certificate.get("timeout_seconds", "").isdigit(),
                "invalid stress projection timeout")
        timeout = int(certificate["timeout_seconds"])
        require(1800 <= timeout <=
                MAXIMUM_TIMEOUT_SECONDS,
                "stress projection timeout exceeds the SCC envelope")
    return timeout


def validate_prepare(run_dir: Path, plan_row: dict[str, str], source_commit: str,
                     bundle_sha: str, manifest_sha: str,
                     data: dict[str, str]) -> str:
    experiment = plan_row["experiment_id"]
    root = run_dir / "experiments" / experiment
    validate_resource(root / "resources.txt")
    require("FEVC SEPARATIONS PREPARE PASS" in read_text(root / "application.txt"),
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
        validate_correction_timing(row, experiment)
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
            require(rhs_rows and tuple(rhs_rows[0]) == RHS_FIELDS,
                    f"RHS certificate schema changed: {experiment}")
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


def calibration_candidate_evidence(
    run_dir: Path, route: str, batch_request: str, *, bundle_sha: str,
    source_commit: str, manifest_sha: str,
) -> tuple[dict[tuple[int, str, int], dict[str, str]],
           list[dict[str, float | str]], list[dict[str, float | str]]]:
    rows: dict[tuple[int, str, int], dict[str, str]] = {}
    accounting: dict[tuple[int, str, int], dict[str, str]] = {}
    for probes in (20, 40):
        for temperature in ("cold", "warm"):
            for repetition in CALIBRATION_REPETITIONS:
                experiment = calibration_experiment_id(
                    route, batch_request, probes, temperature, repetition)
                result = read_rows(
                    run_dir / "experiments" / experiment / f"prod_{experiment}.csv")
                require(len(result) == 1, f"invalid calibration result: {experiment}")
                rows[(probes, temperature, repetition)] = result[0]
                accounting[(probes, temperature, repetition)] = validate_qacct(
                    run_dir, experiment, 4, stage="calibration",
                    bundle_sha=bundle_sha, source_commit=source_commit,
                    manifest_sha=manifest_sha)

    summaries: list[dict[str, float | str]] = []
    raw: list[dict[str, float | str]] = []
    for probes in (20, 40):
        for temperature in ("cold", "warm"):
            cell_rows = [rows[(probes, temperature, repetition)]
                         for repetition in CALIBRATION_REPETITIONS]
            cell_accounting = [accounting[(probes, temperature, repetition)]
                               for repetition in CALIBRATION_REPETITIONS]
            commands = [finite(item, "command_seconds") for item in cell_rows]
            corrections = [finite(item, "correction_seconds") for item in cell_rows]
            walls = [parse_duration(item["ru_wallclock"])
                     for item in cell_accounting]
            summaries.append({
                "probes": probes,
                "temperature": temperature,
                "command_seconds": statistics.median(commands),
                "command_min_seconds": min(commands),
                "command_max_seconds": max(commands),
                "command_range_seconds": max(commands) - min(commands),
                "correction_seconds": statistics.median(corrections),
                "correction_min_seconds": min(corrections),
                "correction_max_seconds": max(corrections),
                "qacct_wall_seconds": statistics.median(walls),
                "qacct_wall_min_seconds": min(walls),
                "qacct_wall_max_seconds": max(walls),
                "hostnames": "|".join(sorted(
                    {item["hostname"] for item in cell_accounting})),
                "qnames": "|".join(sorted(
                    {item["qname"] for item in cell_accounting})),
                "uname_machines": "|".join(sorted(
                    {item["node_uname_machine"] for item in cell_accounting})),
                "cpu_models": "|".join(sorted(
                    {item["node_cpu_model"] for item in cell_accounting})),
                "logical_cpus": "|".join(sorted(
                    {item["node_logical_cpus"] for item in cell_accounting})),
            })
            for repetition, result, qacct in zip(
                    CALIBRATION_REPETITIONS, cell_rows, cell_accounting, strict=False):
                raw.append({
                    "probes": probes,
                    "temperature": temperature,
                    "repetition": repetition,
                    "command_seconds": finite(result, "command_seconds"),
                    "correction_seconds": finite(result, "correction_seconds"),
                    "qacct_wall_seconds": parse_duration(qacct["ru_wallclock"]),
                    "hostname": qacct["hostname"],
                    "qname": qacct["qname"],
                    "cpu_seconds": parse_duration(qacct["cpu"]),
                    "maxvmem_bytes": parse_memory(qacct["maxvmem"]),
                    "uname_machine": qacct["node_uname_machine"],
                    "cpu_model": qacct["node_cpu_model"],
                    "logical_cpus": qacct["node_logical_cpus"],
                })
    return rows, summaries, raw


def validate_auto_forced_equality(
    run_dir: Path, automatic_id: str, forced_id: str,
    automatic: dict[str, str], forced: dict[str, str],
) -> None:
    label = f"{automatic_id}/{forced_id}"
    require(file_sha256(run_dir / "experiments" / automatic_id /
                        "retained_matches.csv") ==
            file_sha256(run_dir / "experiments" / forced_id /
                        "retained_matches.csv"),
            f"auto/forced retained sample changed: {label}")
    for field in (
        "plugin_worker", "plugin_firm", "plugin_covariance", "plugin_total",
        "corrected_worker", "corrected_firm", "corrected_covariance", "corrected_total",
    ):
        require(close(finite(automatic, field), finite(forced, field), 2e-9),
                f"auto/forced target changed: {label} {field}")
    require(automatic["preconditioner_selected"] ==
            forced["preconditioner_selected"] == "cmg",
            f"auto/forced selected route changed: {label}")
    for field in (
        "selected_batch", "rng_state_reproducible", "seed", "tolerance",
        "N_retained", "worker_levels", "firm_levels", "deletion_units",
        "route_hybrid_vertices", "route_hybrid_edges", "route_hierarchy_levels",
        "route_terminal_vertices", "route_planned_rhs", "solver_iterations",
        "solver_max_residual",
    ):
        require(close(finite(automatic, field), finite(forced, field), 1e-10),
                f"auto/forced route/graph changed: {label} {field}")
    auto_rhs = read_rows(run_dir / "experiments" / automatic_id /
                         "rhs_repetition_1.csv")
    forced_rhs = read_rows(run_dir / "experiments" / forced_id /
                           "rhs_repetition_1.csv")
    require(len(auto_rhs) == len(forced_rhs), f"auto/forced RHS count changed: {label}")
    for left, right in zip(auto_rhs, forced_rhs, strict=False):
        for field in ("repetition", "stage", "batch_start", "rhs", "iterations",
                      "converged"):
            require(left[field] == right[field],
                    f"auto/forced RHS certificate changed: {label} {field}")
        require(close(finite(left, "relative_residual"),
                      finite(right, "relative_residual"), 1e-10),
                f"auto/forced RHS residual changed: {label}")


def validate_selection(run_dir: Path, source_commit: str, bundle_sha: str,
                       manifest_sha: str, prepared_sha: str, wage_sha: str) -> dict[str, str]:
    path = run_dir / "experiments/calibration_selector/calibration_selection.csv"
    rows = read_rows(path)
    require(len(rows) == 1, "calibration selector must emit one row")
    row = rows[0]
    require(row["bundle_sha256"] == bundle_sha and row["source_commit"] == source_commit,
            "selection source identity mismatch")
    require(row["data_manifest_sha256"] == manifest_sha and
            row["prepared_sha256"] == prepared_sha and
            row["wage_input_sha256"] == wage_sha,
            "selection input identity mismatch")
    require(row["formula"] == FORMULA and int(finite(row, "full_probes")) == FULL_PROBES,
            "selection projection contract changed")
    require(int(finite(row, "maximum_process_timeout_seconds")) ==
            MAXIMUM_TIMEOUT_SECONDS and
            int(finite(row, "wrapper_runtime_margin_seconds")) ==
            WRAPPER_RUNTIME_MARGIN_SECONDS and
            int(finite(row, "scc_hard_runtime_ceiling_seconds")) ==
            SCC_HARD_RUNTIME_CEILING_SECONDS and
            MAXIMUM_TIMEOUT_SECONDS + WRAPPER_RUNTIME_MARGIN_SECONDS ==
            SCC_HARD_RUNTIME_CEILING_SECONDS,
            "selection SCC runtime envelope changed")
    require(row["selected_preconditioner"] == "cmg" and
            row["preconditioner"] == "auto",
            "CZ18 production selection must use public auto selecting CMG")
    require(row["batch_request"] in {"8", "16", "auto"} and
            int(finite(row, "batch")) in FEASIBLE_EXPLICIT_BATCHES,
            "invalid selected batch request/result")
    require(int(finite(row, "calibration_repetitions_per_cell")) == 3 and
            row["timing_summary"] == "median_of_3_no_trimming" and
            row["hostname_policy"] == "DIFFERENT_HOSTS_ALLOWED_NOT_CAUSAL",
            "calibration repetition/host policy changed")

    admissible: list[dict[str, object]] = []
    all_raw: list[dict[str, float | str]] = []
    candidate_rows_by_route: dict[
        tuple[str, str], dict[tuple[int, str, int], dict[str, str]]
    ] = {}
    selected_summaries: list[dict[str, float | str]] | None = None
    selected_raw: list[dict[str, float | str]] | None = None
    for route in ("auto", "cmg"):
        for batch_request in ("8", "16", "auto"):
            candidate_rows, summaries, raw = calibration_candidate_evidence(
                run_dir, route, batch_request, bundle_sha=bundle_sha,
                source_commit=source_commit, manifest_sha=manifest_sha)
            candidate_rows_by_route[(route, batch_request)] = candidate_rows
            all_raw.extend(raw)
            selected_batches = {int(finite(value, "selected_batch"))
                                for value in candidate_rows.values()}
            selected_routes = {value["preconditioner_selected"]
                               for value in candidate_rows.values()}
            typical_measurements = [{
                "probes": int(summary["probes"]),
                "temperature": str(summary["temperature"]),
                "command_seconds": float(summary["command_seconds"]),
                "correction_seconds": float(summary["correction_seconds"]),
                "qacct_wall_seconds": float(summary["qacct_wall_seconds"]),
            } for summary in summaries]
            typical_alpha, typical_beta, typical_headroom, typical_projected, \
                typical_timeout = recompute_calibration_projection(typical_measurements)
            alpha, beta, headroom, projected, timeout, slope_policy = \
                recompute_conservative_calibration_projection(raw)
            if (len(selected_batches) != 1 or selected_routes != {"cmg"} or
                    timeout > MAXIMUM_TIMEOUT_SECONDS):
                continue
            candidate = {
                "preconditioner": route,
                "batch_request": batch_request,
                "batch": next(iter(selected_batches)),
                "typical_projected_seconds": typical_projected,
                "typical_timeout_seconds": typical_timeout,
                "projected_seconds": projected,
                "timeout_seconds": timeout,
                "typical_alpha_seconds": typical_alpha,
                "typical_beta_seconds_per_probe": typical_beta,
                "typical_headroom_seconds": typical_headroom,
                "alpha_seconds": alpha,
                "beta_seconds_per_probe": beta,
                "headroom_seconds": headroom,
                "slope_pairing_policy": slope_policy,
                "same_host_coincidences": recompute_same_host_coincidences(raw),
                "node_class_multiset": recompute_node_class_multiset(raw),
            }
            admissible.append(candidate)
            if route == row["preconditioner"] and batch_request == row["batch_request"]:
                selected_summaries, selected_raw = summaries, raw

    for batch_request in ("8", "16", "auto"):
        for probes in (20, 40):
            for temperature in ("cold", "warm"):
                for repetition in CALIBRATION_REPETITIONS:
                    automatic_id = calibration_experiment_id(
                        "auto", batch_request, probes, temperature, repetition)
                    forced_id = calibration_experiment_id(
                        "cmg", batch_request, probes, temperature, repetition)
                    validate_auto_forced_equality(
                        run_dir, automatic_id, forced_id,
                        candidate_rows_by_route[("auto", batch_request)][
                            (probes, temperature, repetition)],
                        candidate_rows_by_route[("cmg", batch_request)][
                            (probes, temperature, repetition)],
                    )

    require(admissible and int(finite(row, "candidate_count")) == len(admissible),
            "admissible candidate count changed")
    admissible_auto = [candidate for candidate in admissible
                       if candidate["preconditioner"] == "auto"]
    require(admissible_auto and
            int(finite(row, "automatic_candidate_count")) == len(admissible_auto),
            "admissible automatic candidate count changed")
    require(int(finite(row, "measured_candidate_count")) == 6 and
            int(finite(row, "measured_job_count")) == 72,
            "selection measurement accounting changed")
    node_characteristics_identical = len({
        str(candidate["node_class_multiset"]) for candidate in admissible_auto
    }) == 1
    if node_characteristics_identical:
        ranking_policy = "MEDIAN_TYPICAL_IDENTICAL_NODE_CLASS_MULTISET"
        preferred = min(admissible_auto, key=candidate_sort_key)
    else:
        ranking_policy = "CONSERVATIVE_HIGH_HETEROGENEOUS_NODE_CLASS_MULTISET"
        preferred = min(admissible_auto, key=conservative_selection_key)
    require(row["ranking_policy"] == ranking_policy and
            int(finite(row, "auto_node_characteristics_identical")) ==
            int(node_characteristics_identical),
            "node-comparability ranking policy changed")
    stress_calibration_candidates = [
        candidate for candidate in admissible_auto
        if candidate["batch_request"] == "8"
    ]
    require(len(stress_calibration_candidates) == 1,
            "automatic batch-8 evidence cannot admit stress calibration")
    stress_calibration_candidate = stress_calibration_candidates[0]
    stress_calibration_timeout = min(
        MAXIMUM_TIMEOUT_SECONDS,
        max(1800, 2 * int(stress_calibration_candidate["timeout_seconds"])),
    )
    require(int(finite(row, "auto_batch8_timeout_seconds")) ==
            int(stress_calibration_candidate["timeout_seconds"]) and
            row["stress_calibration_timeout_formula"] ==
            "min(42600,max(1800,2*auto_batch8_timeout_seconds))" and
            int(finite(row, "stress_calibration_timeout_seconds")) ==
            stress_calibration_timeout,
            "stress calibration timeout is not bound to the conservative "
            "automatic batch-8 envelope")
    expected_auto_multisets = json.dumps({
        str(candidate["batch_request"]): json.loads(
            str(candidate["node_class_multiset"]))
        for candidate in admissible_auto
    }, sort_keys=True, separators=(",", ":"))
    require(row["auto_node_class_multisets"] == expected_auto_multisets,
            "automatic candidate node-class multisets changed")
    for field in (
        "preconditioner", "batch_request", "batch", "typical_alpha_seconds",
        "typical_beta_seconds_per_probe", "typical_headroom_seconds",
        "typical_projected_seconds", "typical_timeout_seconds", "alpha_seconds",
        "beta_seconds_per_probe", "headroom_seconds", "projected_seconds",
        "timeout_seconds", "slope_pairing_policy",
        "same_host_coincidences",
        "node_class_multiset",
    ):
        if field.endswith("seconds") and field not in {"slope_pairing_policy"}:
            require(close(finite(row, field), float(preferred[field]), 1e-9),
                    f"selector timing changed: {field}")
        else:
            require(str(row[field]) == str(preferred[field]),
                    f"selector preference changed: {field}")
    require(finite(row, "timeout_seconds") <= MAXIMUM_TIMEOUT_SECONDS,
            "calibrated timeout inadmissible")
    require(selected_summaries is not None and selected_raw is not None,
            "selected candidate evidence absent")
    require(close(finite(row, "t20_seconds"), max(
                float(item["command_seconds"]) for item in selected_summaries
                if item["probes"] == 20), 1e-12) and
            close(finite(row, "t40_seconds"), max(
                float(item["command_seconds"]) for item in selected_summaries
                if item["probes"] == 40), 1e-12),
            "selection median P20/P40 timings changed")
    inversion_status, inversion_temperatures = \
        recompute_timing_inversion(selected_summaries)
    require(row["timing_inversion_status"] == inversion_status and
            row["timing_inversion_temperatures"] == inversion_temperatures,
            "timing inversion classification changed")

    for summary in selected_summaries:
        prefix = f"p{summary['probes']}_{summary['temperature']}"
        expected_ids = "|".join(
            calibration_experiment_id(row["preconditioner"], row["batch_request"],
                                      int(summary["probes"]),
                                      str(summary["temperature"]), repetition)
            for repetition in CALIBRATION_REPETITIONS
        )
        require(row[f"{prefix}_experiments"] == expected_ids,
                f"selected repetition IDs changed: {prefix}")
        for field in (
            "command_seconds", "command_min_seconds", "command_max_seconds",
            "command_range_seconds", "correction_seconds",
            "correction_min_seconds", "correction_max_seconds",
            "qacct_wall_seconds", "qacct_wall_min_seconds",
            "qacct_wall_max_seconds",
        ):
            require(close(finite(row, f"{prefix}_{field}"),
                          float(summary[field]), 1e-12),
                    f"cell timing summary changed: {prefix} {field}")
        for field in ("hostnames", "qnames", "uname_machines",
                      "cpu_models", "logical_cpus"):
            require(row[f"{prefix}_{field}"] == str(summary[field]),
                    f"cell node/accounting summary changed: {prefix} {field}")

    selected_hosts = "|".join(sorted({str(item["hostname"]) for item in selected_raw}))
    selected_qnames = "|".join(sorted({str(item["qname"]) for item in selected_raw}))
    require(row["selected_hostnames"] == selected_hosts and
            row["selected_qnames"] == selected_qnames and
            row["selected_uname_machines"] == "|".join(sorted(
                {str(item["uname_machine"]) for item in selected_raw})) and
            row["selected_cpu_models"] == "|".join(sorted(
                {str(item["cpu_model"]) for item in selected_raw})) and
            row["selected_logical_cpus"] == "|".join(sorted(
                {str(item["logical_cpus"]) for item in selected_raw})),
            "selected node characteristics changed")
    numeric_accounting = {
        "selected_qacct_wall_min_seconds": min(
            float(item["qacct_wall_seconds"]) for item in selected_raw),
        "selected_qacct_wall_max_seconds": max(
            float(item["qacct_wall_seconds"]) for item in selected_raw),
        "selected_qacct_cpu_min_seconds": min(
            float(item["cpu_seconds"]) for item in selected_raw),
        "selected_qacct_cpu_max_seconds": max(
            float(item["cpu_seconds"]) for item in selected_raw),
        "selected_qacct_maxvmem_bytes": max(
            float(item["maxvmem_bytes"]) for item in selected_raw),
    }
    for field, value in numeric_accounting.items():
        require(close(finite(row, field), value, 1e-12),
                f"selected qacct summary changed: {field}")
    require(row["all_calibration_hostnames"] == "|".join(sorted(
                {str(item["hostname"]) for item in all_raw})) and
            row["all_calibration_qnames"] == "|".join(sorted(
                {str(item["qname"]) for item in all_raw})) and
            row["all_calibration_uname_machines"] == "|".join(sorted(
                {str(item["uname_machine"]) for item in all_raw})) and
            row["all_calibration_cpu_models"] == "|".join(sorted(
                {str(item["cpu_model"]) for item in all_raw})) and
            row["all_calibration_logical_cpus"] == "|".join(sorted(
                {str(item["logical_cpus"]) for item in all_raw})),
            "all-calibration node/accounting summary changed")

    require(close(finite(row, "batch_memory_fraction"), BATCH_MEMORY_FRACTION, 1e-15) and
            int(finite(row, "batch_memory_budget_bytes")) == batch_memory_budget_bytes(),
            "batch memory policy changed")
    require(int(finite(row, "batch8_scratch_forecast_bytes")) ==
            BATCH_BASE_SCRATCH_BYTES, "batch-8 forecast changed")
    for width in (16, 32, 64, 128):
        require(int(finite(row, f"batch{width}_scratch_forecast_bytes")) ==
                forecast_batch_bytes(width), f"batch-{width} forecast changed")
    require(row["feasible_batch_widths"] == "8|16|auto" and
            row["infeasible_batch_widths"] == "32|64|128" and
            all(forecast_batch_bytes(width) <= batch_memory_budget_bytes()
                for width in FEASIBLE_EXPLICIT_BATCHES) and
            all(forecast_batch_bytes(width) > batch_memory_budget_bytes()
                for width in INFEASIBLE_BATCHES),
            "registered batch feasibility classification is inconsistent")

    calibration_ids = sorted(
        calibration_experiment_id(route, batch, probes, temperature, repetition)
        for route in ("auto", "cmg") for batch in ("8", "16", "auto")
        for probes in (20, 40) for temperature in ("cold", "warm")
        for repetition in CALIBRATION_REPETITIONS
    )
    require(row["calibration_evidence_sha256"] ==
            evidence_digest(run_dir, calibration_ids),
            "calibration evidence changed after selection")
    return row


def recompute_stress_projection(
    rows: dict[str, dict[str, str]],
    accounting: dict[str, dict[str, str]],
) -> dict[str, float | int]:
    require(set(rows) == set(STRESS_CALIBRATION_IDS),
            "stress projection requires all three independent calibrations")
    alpha_values: list[float] = []
    beta_values: list[float] = []
    wall_values: list[float] = []
    command_values: list[float] = []
    correction_values: list[float] = []
    for experiment in STRESS_CALIBRATION_IDS:
        row = rows[experiment]
        qacct = accounting.get(experiment)
        require(qacct is not None and qacct.get("node_schema") ==
                "kss_prod_node_v1",
                f"stress calibration lacks validated qacct/node evidence: "
                f"{experiment}")
        probes = int(finite(row, "requested_probes"))
        require(probes == 20, f"stress calibration probe count changed: {experiment}")
        command = finite(row, "command_seconds")
        correction = finite(row, "correction_seconds")
        wall = parse_duration(qacct["ru_wallclock"])
        require(correction <= command + 1.0,
                f"stress correction timer exceeds command timer: {experiment}")
        require(command <= wall + 1.0,
                f"stress command timer exceeds qacct wall time: {experiment}")
        alpha_values.append(max(0.0, wall - correction))
        beta_values.append(correction / probes)
        wall_values.append(wall)
        command_values.append(command)
        correction_values.append(correction)
    alpha = max(alpha_values)
    beta = max(beta_values)
    projected = math.ceil(max(
        float(CALIBRATION_MINIMUM_TIMEOUT_SECONDS),
        CALIBRATION_SETUP_SAFETY_FACTOR * alpha +
        CALIBRATION_MARGINAL_SAFETY_FACTOR * FULL_PROBES * beta +
        float(FIXED_HEADROOM_SECONDS),
    ))
    timeout = max(1800, projected)
    return {
        "alpha_seconds": alpha,
        "beta_seconds_per_probe": beta,
        "projected_seconds": projected,
        "timeout_seconds": timeout,
        "median_wall_seconds": statistics.median(wall_values),
        "median_command_seconds": statistics.median(command_values),
        "median_correction_seconds": statistics.median(correction_values),
        "wall_min_seconds": min(wall_values),
        "wall_max_seconds": max(wall_values),
        "correction_min_seconds": min(correction_values),
        "correction_max_seconds": max(correction_values),
    }


def require_larger_stress_dimensions(
    stress: dict[str, str], full_cz18: dict[str, str], experiment: str,
) -> None:
    for field, strict in (
        ("N_retained", False), ("worker_levels", True),
        ("firm_levels", False), ("deletion_units", False),
        ("route_hybrid_vertices", False), ("route_hybrid_edges", False),
    ):
        observed = finite(stress, field)
        boundary = 2 * finite(full_cz18, field)
        require(observed > boundary if strict else observed >= boundary,
                f"{experiment}: {field} does not satisfy the registered "
                "larger-than-CZ18 stress dimensions")


def validate_stress_calibration_matrix(
    run_dir: Path,
    outputs: dict[str, list[dict[str, str]]],
    accounting: dict[str, dict[str, str]],
    full_cz18: dict[str, str],
) -> tuple[dict[str, str], dict[str, float | int]]:
    require(all(experiment in outputs for experiment in STRESS_CALIBRATION_IDS),
            "stress calibration repetition matrix is incomplete")
    require(len({read_text(run_dir / "submissions" / f"{experiment}.job_id").strip()
                 for experiment in STRESS_CALIBRATION_IDS}) ==
            len(STRESS_CALIBRATION_IDS),
            "stress calibration repetitions reused one process")
    reference_id = STRESS_CALIBRATION_IDS[0]
    reference = outputs[reference_id][0]
    reference_rhs = read_rows(
        run_dir / "experiments" / reference_id / "rhs_repetition_1.csv"
    )
    require(len(reference_rhs) == 3 * 20 + 1,
            "stress calibration RHS certificate count changed")
    for experiment in STRESS_CALIBRATION_IDS:
        candidate = outputs[experiment][0]
        require_larger_stress_dimensions(candidate, full_cz18, experiment)
        require(candidate["preconditioner_selected"] == "cmg" and
                finite(candidate, "route_hierarchy_levels") > 1 and
                1 <= finite(candidate, "route_terminal_vertices") <= 6144,
                f"{experiment}: larger stress case did not use bounded "
                "multilevel CMG")
        require(candidate["_retained_sha256"] == reference["_retained_sha256"],
                f"{experiment}: stress repetition changed retained sample")
        for field in (
            "prepared_sha256", "estimator_input_sha256", "wage_input_sha256",
            "batch_requested", "preconditioner_requested",
            "preconditioner_selected", "algorithm_requested",
            "algorithm_selected", "fallback_status", "routing_reason",
        ):
            require(candidate[field] == reference[field],
                    f"{experiment}: stress repetition config changed: {field}")
        for field in (
            "requested_processors", "actual_processors", "declared_memory_gib",
            "requested_probes", "seed", "tolerance", "selected_batch",
            "N_retained", "worker_levels", "firm_levels", "deletion_units",
            "route_hybrid_vertices", "route_hybrid_edges",
            "route_hierarchy_levels", "route_terminal_vertices",
            "route_planned_rhs", "solver_iterations", "solver_max_residual",
        ):
            require(finite(candidate, field) == finite(reference, field),
                    f"{experiment}: stress repetition graph/result changed: {field}")
        for field in (
            "plugin_worker", "plugin_firm", "plugin_covariance", "plugin_total",
            "corrected_worker", "corrected_firm", "corrected_covariance",
            "corrected_total",
        ):
            require(close(finite(candidate, field), finite(reference, field), 1e-10),
                    f"{experiment}: stress repetition scientific result changed: "
                    f"{field}")
        candidate_rhs = read_rows(
            run_dir / "experiments" / experiment / "rhs_repetition_1.csv"
        )
        require(len(candidate_rhs) == len(reference_rhs),
                f"{experiment}: stress repetition RHS count changed")
        for left, right in zip(reference_rhs, candidate_rhs, strict=False):
            for field in (
                "repetition", "stage", "batch_start", "rhs", "iterations",
                "converged",
            ):
                require(left[field] == right[field],
                        f"{experiment}: stress repetition RHS changed: {field}")
            require(close(finite(left, "relative_residual"),
                          finite(right, "relative_residual"), 1e-10),
                    f"{experiment}: stress repetition RHS residual changed")
    projection_rows = {
        experiment: outputs[experiment][0]
        for experiment in STRESS_CALIBRATION_IDS
    }
    projection = recompute_stress_projection(projection_rows, accounting)
    require(int(projection["timeout_seconds"]) <= MAXIMUM_TIMEOUT_SECONDS,
            "stress calibration projection exceeds the SCC runtime ceiling")
    return reference, projection


def validate_full_stress_configuration(
    stress_full: dict[str, str], stress_calibration: dict[str, str],
    full_cz18: dict[str, str],
) -> None:
    experiment = "cz18_stress2x_full200"
    require_larger_stress_dimensions(stress_full, full_cz18, experiment)
    require(stress_full["preconditioner_selected"] == "cmg" and
            finite(stress_full, "route_hierarchy_levels") > 1 and
            1 <= finite(stress_full, "route_terminal_vertices") <= 6144,
            "full larger stress case did not use bounded multilevel CMG")
    require(stress_full["_retained_sha256"] ==
            stress_calibration["_retained_sha256"],
            "stress calibration and full run retained different samples")
    for field in (
        "prepared_sha256", "estimator_input_sha256", "wage_input_sha256",
        "batch_requested", "preconditioner_requested", "preconditioner_selected",
        "algorithm_requested", "algorithm_selected", "fallback_status",
    ):
        require(stress_full[field] == stress_calibration[field],
                f"stress calibration/full configuration changed: {field}")
    for field in (
        "requested_processors", "actual_processors", "declared_memory_gib",
        "seed", "tolerance", "selected_batch", "N_retained", "worker_levels",
        "firm_levels", "deletion_units", "route_hybrid_vertices",
        "route_hybrid_edges", "route_hierarchy_levels",
        "route_terminal_vertices",
    ):
        require(finite(stress_full, field) == finite(stress_calibration, field),
                f"stress calibration/full graph changed: {field}")
    for field in (
        "plugin_worker", "plugin_firm", "plugin_covariance", "plugin_total",
    ):
        require(close(finite(stress_full, field),
                      finite(stress_calibration, field), 1e-10),
                f"stress calibration/full plug-in changed: {field}")


def validate_stress_projection_certificate(
    run_dir: Path, bundle_sha: str, source_commit: str, manifest_sha: str,
    stress_calibration: dict[str, str],
    projection: dict[str, float | int],
    accounting: dict[str, dict[str, str]],
) -> dict[str, str]:
    path = run_dir / "validation/stress_projection.txt"
    values = read_key_value_certificate(path, "stress projection")
    base_fields = (
        "schema", "status", "source_commit", "bundle_sha256",
        "data_manifest_sha256", "cz18_full_result_sha256",
        "retained_sha_file_sha256", "estimator_input_sha256",
        "calibration_count", "calibration_experiment_ids", "processors",
        "declared_memory_gib", "selected_batch", "node_class_multiset_json",
        "node_characteristics_identical", "qacct_hostnames_json",
        "qacct_qnames_json", "hostname_policy",
    )
    per_repetition_suffixes = (
        "experiment_id", "csv_sha256", "qacct_sha256",
        "job_id_file_sha256", "rhs_sha256",
        "node_characteristics_sha256", "job_id", "hostname", "qname",
        "cpu_seconds", "qacct_wall_seconds", "qacct_maxvmem_bytes",
        "command_seconds", "leverage_seconds", "target_seconds",
        "correction_seconds", "alpha_seconds", "beta_seconds_per_probe",
        "rhs_max_residual",
    )
    tail_fields = (
        "median_qacct_wall_seconds", "median_correction_seconds",
        "alpha_seconds", "beta_seconds_per_probe", "setup_safety_factor",
        "marginal_safety_factor", "headroom_seconds", "calibration_probes",
        "full_probes", "projected_seconds", "timeout_floor_seconds",
        "timeout_seconds", "formula",
    )
    expected_fields = base_fields + tuple(
        f"calibration_r{index}_{suffix}"
        for index in CALIBRATION_REPETITIONS
        for suffix in per_repetition_suffixes
    ) + tail_fields
    require(tuple(values) == expected_fields and len(values) == 88,
            "stress projection certificate schema changed")
    require(values["schema"] == "kss_stress_projection_v2" and
            values["status"] == "PASS",
            "stress projection certificate status changed")
    require(values["source_commit"] == source_commit and
            values["bundle_sha256"] == bundle_sha and
            values["data_manifest_sha256"] == manifest_sha,
            "stress projection source identity changed")
    full_result = run_dir / "experiments/cz18_full200/prod_cz18_full200.csv"
    retained_sha_file = run_dir / "experiments/cz18_full200/retained_sample.sha256"
    retained_sha = read_text(retained_sha_file).strip()
    require(values["cz18_full_result_sha256"] == file_sha256(full_result) and
            values["retained_sha_file_sha256"] == file_sha256(retained_sha_file) and
            values["estimator_input_sha256"] == retained_sha ==
            stress_calibration["estimator_input_sha256"],
            "stress projection parent-input evidence changed")
    require(values["calibration_count"] == "3" and
            values["calibration_experiment_ids"] ==
            "|".join(STRESS_CALIBRATION_IDS),
            "stress projection calibration repetition set changed")
    require(int(float(values["processors"])) ==
            int(finite(stress_calibration, "requested_processors")) and
            int(float(values["declared_memory_gib"])) == 56 and
            int(float(values["selected_batch"])) ==
            int(finite(stress_calibration, "selected_batch")),
            "stress projection processor/memory/batch binding changed")

    node_classes: list[tuple[str, str, int]] = []
    hostnames: list[str] = []
    qnames: list[str] = []
    walls: list[float] = []
    corrections: list[float] = []
    for index, experiment in zip(CALIBRATION_REPETITIONS,
                                 STRESS_CALIBRATION_IDS, strict=False):
        root = run_dir / "experiments" / experiment
        result_path = root / f"prod_{experiment}.csv"
        qacct_path = run_dir / "qacct" / f"{experiment}.txt"
        job_path = run_dir / "submissions" / f"{experiment}.job_id"
        rhs_path = root / "rhs_repetition_1.csv"
        node_path = root / "node_characteristics.txt"
        qacct = accounting[experiment]
        row = read_rows(result_path)[0]
        prefix = f"calibration_r{index}_"
        require(values[prefix + "experiment_id"] == experiment and
                values[prefix + "csv_sha256"] == file_sha256(result_path) and
                values[prefix + "qacct_sha256"] == file_sha256(qacct_path) and
                values[prefix + "job_id_file_sha256"] == file_sha256(job_path) and
                values[prefix + "rhs_sha256"] == file_sha256(rhs_path) and
                values[prefix + "node_characteristics_sha256"] ==
                file_sha256(node_path),
                f"stress projection evidence hash changed: {experiment}")
        job_id = read_text(job_path).strip()
        require(values[prefix + "job_id"] == job_id and
                values[prefix + "hostname"] == qacct["hostname"] and
                values[prefix + "qname"] == qacct["qname"],
                f"stress projection qacct identity changed: {experiment}")
        wall = parse_duration(qacct["ru_wallclock"])
        cpu = parse_duration(qacct["cpu"])
        maxvmem = parse_memory(qacct["maxvmem"])
        correction = finite(row, "correction_seconds")
        alpha = max(0.0, wall - correction)
        beta = correction / 20
        rhs_rows = read_rows(rhs_path)
        rhs_residual = max(finite(rhs, "relative_residual") for rhs in rhs_rows)
        expected_numeric = {
            "cpu_seconds": cpu,
            "qacct_wall_seconds": wall,
            "qacct_maxvmem_bytes": maxvmem,
            "command_seconds": finite(row, "command_seconds"),
            "leverage_seconds": finite(row, "leverage_seconds"),
            "target_seconds": finite(row, "target_seconds"),
            "correction_seconds": correction,
            "alpha_seconds": alpha,
            "beta_seconds_per_probe": beta,
            "rhs_max_residual": rhs_residual,
        }
        for suffix, expected in expected_numeric.items():
            require(close(float(values[prefix + suffix]), expected, 1e-12),
                    f"stress projection numeric evidence changed: "
                    f"{experiment}: {suffix}")
        node_classes.append((qacct["node_uname_machine"],
                             qacct["node_cpu_model"],
                             int(qacct["node_logical_cpus"])))
        hostnames.append(qacct["hostname"])
        qnames.append(qacct["qname"])
        walls.append(wall)
        corrections.append(correction)
    expected_node_classes = json.dumps(sorted(node_classes), separators=(",", ":"))
    require(values["node_class_multiset_json"] == expected_node_classes and
            int(float(values["node_characteristics_identical"])) ==
            int(len(set(node_classes)) == 1),
            "stress projection node-class description changed")
    require(values["qacct_hostnames_json"] ==
            json.dumps(hostnames, separators=(",", ":")) and
            values["qacct_qnames_json"] ==
            json.dumps(qnames, separators=(",", ":")) and
            values["hostname_policy"] == STRESS_NODE_POLICY,
            "stress projection noncausal node/accounting policy changed")

    expected_tail = {
        "median_qacct_wall_seconds": statistics.median(walls),
        "median_correction_seconds": statistics.median(corrections),
        "alpha_seconds": float(projection["alpha_seconds"]),
        "beta_seconds_per_probe": float(projection["beta_seconds_per_probe"]),
        "setup_safety_factor": CALIBRATION_SETUP_SAFETY_FACTOR,
        "marginal_safety_factor": CALIBRATION_MARGINAL_SAFETY_FACTOR,
        "headroom_seconds": float(FIXED_HEADROOM_SECONDS),
        "calibration_probes": 20.0,
        "full_probes": float(FULL_PROBES),
        "projected_seconds": float(projection["projected_seconds"]),
        "timeout_floor_seconds": 1800.0,
        "timeout_seconds": float(projection["timeout_seconds"]),
    }
    for field, expected in expected_tail.items():
        require(close(float(values[field]), expected, 1e-12),
                f"stress projection summary changed: {field}")
    require(values["formula"] == STRESS_PROJECTION_FORMULA and
            int(float(values["timeout_seconds"])) <= MAXIMUM_TIMEOUT_SECONDS,
            "stress projection formula/runtime envelope changed")
    full_copy = run_dir / "experiments/cz18_stress2x_full200/stress_projection.txt"
    require(full_copy.is_file() and file_sha256(full_copy) == file_sha256(path),
            "full stress job did not preserve the byte-identical admission certificate")
    return values


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
    if phase == "stress":
        paths.append(run_dir / "validation/stress_projection.txt")
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
    parser.add_argument(
        "--phase", choices=("preflight", "calibration", "production", "stress")
    )
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
    if args.phase in {"calibration", "production", "stress"}:
        validate_phase_pass(args.run_dir, plan, "preflight", args.source_commit,
                            args.bundle_sha, manifest_sha)
    if args.phase in {"production", "stress"}:
        calibration_pass = validate_phase_pass(
            args.run_dir, plan, "calibration", args.source_commit,
            args.bundle_sha, manifest_sha)
        selection_path = args.run_dir / "experiments/calibration_selector/calibration_selection.csv"
        require(calibration_pass.get("selection_sha256") == file_sha256(selection_path),
                "selection changed after calibration acceptance")
    if args.phase == "stress":
        production_pass = validate_phase_pass(
            args.run_dir, plan, "production", args.source_commit,
            args.bundle_sha, manifest_sha)
        require(production_pass.get("selection_sha256") ==
                file_sha256(selection_path),
                "selection changed after production acceptance")

    phases = {"preflight"}
    if args.phase in {"calibration", "production", "stress"}:
        phases.add("calibration")
    if args.phase in {"production", "stress"}:
        phases.add("production")
    if args.phase == "stress":
        phases.add("stress")
    prepared_hashes: dict[str, str] = {}
    outputs: dict[str, list[dict[str, str]]] = {}
    accounting: dict[str, dict[str, str]] = {}
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
            int(spec["memory_gib"]), expected_timeout(row, selection, args.run_dir),
            args.bundle_sha, manifest_sha, ledger, row["depends"],
            phase_by_experiment)
        accounting[row["experiment_id"]] = validate_qacct(
            args.run_dir, row["experiment_id"], int(spec["processors"]),
            stage=row["stage"], bundle_sha=args.bundle_sha,
            source_commit=args.source_commit, manifest_sha=manifest_sha,
        )
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

    if args.phase in {"calibration", "production", "stress"}:
        for route in ("auto", "cmg"):
            for batch in ("8", "16", "auto"):
                for probes in (20, 40):
                    for temperature in ("cold", "warm"):
                        names = [calibration_experiment_id(
                            route, batch, probes, temperature, repetition)
                            for repetition in CALIBRATION_REPETITIONS]
                        require(all(name in outputs for name in names),
                                "calibration repetition matrix incomplete")
                        require(len({read_text(
                            args.run_dir / "submissions" / f"{name}.job_id").strip()
                            for name in names}) == len(CALIBRATION_REPETITIONS),
                                "calibration repetitions reused one process")
                        reference = outputs[names[0]][0]
                        reference_rhs = read_rows(
                            args.run_dir / "experiments" / names[0] /
                            "rhs_repetition_1.csv")
                        for name in names[1:]:
                            candidate = outputs[name][0]
                            require(reference["_retained_sha256"] ==
                                    candidate["_retained_sha256"],
                                    "repetition changed retained sample")
                            for field in (
                                "prepared_sha256", "estimator_input_sha256",
                                "wage_input_sha256", "batch_requested",
                                "preconditioner_requested", "preconditioner_selected",
                                "algorithm_requested", "algorithm_selected",
                                "fallback_status", "routing_reason",
                            ):
                                require(reference[field] == candidate[field],
                                        f"cross-repetition config changed: {field}")
                            for field in (
                                "requested_processors", "actual_processors",
                                "declared_memory_gib", "requested_probes", "seed",
                                "tolerance", "selected_batch", "N_retained",
                                "worker_levels", "firm_levels", "deletion_units",
                                "route_hybrid_vertices", "route_hybrid_edges",
                                "route_hierarchy_levels", "route_planned_rhs",
                                "solver_iterations", "solver_max_residual",
                            ):
                                require(finite(reference, field) == finite(candidate, field),
                                        f"cross-repetition resource/graph changed: {field}")
                            for field in (
                                "plugin_worker", "plugin_firm", "plugin_covariance",
                                "plugin_total", "corrected_worker", "corrected_firm",
                                "corrected_covariance", "corrected_total",
                            ):
                                require(close(finite(reference, field),
                                              finite(candidate, field), 1e-10),
                                        f"cross-repetition scientific result changed: {field}")
                            candidate_rhs = read_rows(
                                args.run_dir / "experiments" / name /
                                "rhs_repetition_1.csv")
                            require(len(candidate_rhs) == len(reference_rhs),
                                    "cross-repetition RHS count changed")
                            for left_rhs, right_rhs in zip(reference_rhs, candidate_rhs, strict=False):
                                for field in ("repetition", "stage", "batch_start", "rhs",
                                              "iterations", "converged"):
                                    require(left_rhs[field] == right_rhs[field],
                                            f"cross-repetition RHS certificate changed: {field}")
                                require(close(finite(left_rhs, "relative_residual"),
                                              finite(right_rhs, "relative_residual"), 1e-10),
                                        "cross-repetition RHS residual changed")

                for probes in (20, 40):
                    for repetition in CALIBRATION_REPETITIONS:
                        cold_name = calibration_experiment_id(
                            route, batch, probes, "cold", repetition)
                        warm_name = calibration_experiment_id(
                            route, batch, probes, "warm", repetition)
                        cold, warm = outputs[cold_name][0], outputs[warm_name][0]
                        require(cold["_retained_sha256"] == warm["_retained_sha256"],
                                "cold/warm retained sample changed")
                        for field in (
                            "plugin_worker", "plugin_firm", "plugin_covariance",
                            "plugin_total", "corrected_worker", "corrected_firm",
                            "corrected_covariance", "corrected_total",
                        ):
                            require(close(finite(cold, field), finite(warm, field), 1e-10),
                                    f"cold/warm result changed: {cold_name} {field}")

                for temperature in ("cold", "warm"):
                    for repetition in CALIBRATION_REPETITIONS:
                        low = outputs[calibration_experiment_id(
                            route, batch, 20, temperature, repetition)][0]
                        high = outputs[calibration_experiment_id(
                            route, batch, 40, temperature, repetition)][0]
                        require(low["_retained_sha256"] == high["_retained_sha256"],
                                "P20/P40 retained sample changed")
                        for field in ("plugin_worker", "plugin_firm",
                                      "plugin_covariance", "plugin_total"):
                            require(close(finite(low, field), finite(high, field), 1e-10),
                                    f"P20/P40 plug-in changed: {route} {batch} {field}")
                        for field in (
                            "selected_batch", "N_retained", "worker_levels",
                            "firm_levels", "deletion_units", "route_hybrid_vertices",
                            "route_hybrid_edges", "route_hierarchy_levels",
                        ):
                            require(finite(low, field) == finite(high, field),
                                    f"P20/P40 graph or route changed: {route} {batch} {field}")

        for route in ("auto", "cmg"):
            for probes in (20, 40):
                for temperature in ("cold", "warm"):
                    for repetition in CALIBRATION_REPETITIONS:
                        names = [calibration_experiment_id(
                            route, batch, probes, temperature, repetition)
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
                            require(reference["_retained_sha256"] ==
                                    candidate["_retained_sha256"],
                                    "batch changed retained sample")
                            for field in (
                                "plugin_worker", "plugin_firm", "plugin_covariance",
                                "plugin_total", "corrected_worker", "corrected_firm",
                                "corrected_covariance", "corrected_total",
                            ):
                                require(close(finite(reference, field),
                                              finite(candidate, field), 2e-9),
                                        f"batch invariance failed: {route} P{probes} "
                                        f"{temperature} r{repetition} {field}")
    if args.phase in {"production", "stress"}:
        full = outputs["cz18_full200"][0]
        require(full["preconditioner_selected"] == "cmg",
                "full CZ18 estimator did not use qualified CMG")
        require(finite(full, "route_hybrid_vertices") > 6144 and
                finite(full, "route_hierarchy_levels") > 1,
                "full CZ18 did not exercise the large-hybrid multilevel path")
        stress_cal, stress_projection = validate_stress_calibration_matrix(
            args.run_dir, outputs, accounting, full
        )
    if args.phase == "stress":
        stress_full = outputs["cz18_stress2x_full200"][0]
        validate_full_stress_configuration(stress_full, stress_cal, full)
        validate_stress_projection_certificate(
            args.run_dir, args.bundle_sha, args.source_commit, manifest_sha,
            stress_cal, stress_projection, accounting,
        )

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

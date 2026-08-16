#!/usr/bin/env python3
"""Select a CZ18 CMG configuration from paired setup/slope calibrations."""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import re
import statistics
from pathlib import Path


FULL_PROBES = 200
CALIBRATION_PROBES = (20, 40)
SAFETY_FACTOR = 1.5  # Retained for the separate larger-stress projection.
SETUP_SAFETY_FACTOR = 1.25
MARGINAL_SAFETY_FACTOR = 1.5
FIXED_HEADROOM_SECONDS = 120
FULL_RETAINED_SAVE_ALLOWANCE_SECONDS = 120
MINIMUM_TIMEOUT_SECONDS = 300
# Leave 600 seconds for wrapper/accounting work while preserving SCC's broad
# <=12-hour eligibility envelope.  The selected request remains the measured
# conservative projection, not this operational ceiling.
SCC_HARD_RUNTIME_CEILING_SECONDS = 12 * 60 * 60
WRAPPER_RUNTIME_MARGIN_SECONDS = 600
MAXIMUM_TIMEOUT_SECONDS = (
    SCC_HARD_RUNTIME_CEILING_SECONDS - WRAPPER_RUNTIME_MARGIN_SECONDS
)
REQUESTED_TOLERANCE = 1e-10
COMPLETE_RESIDUAL_GATE = max(1e-11, 10 * REQUESTED_TOLERANCE)
# Aggregate Stata CSVs preserve about eight significant decimal digits for
# these separately exported timers.  This tolerance applies only to the
# accounting identity correction_seconds = leverage_seconds + target_seconds;
# it does not alter estimator, solver, or complete-residual tolerances.
CSV_TIMING_IDENTITY_TOLERANCE = 2e-7
FORMULA = (
    "beta_hi=max(0,cross_host_upper_envelope_slopes,"
    "all_correction_seconds/probes);"
    "alpha_hi=max(0,all_command_seconds-probes*beta_hi);"
    "headroom=max(120,max_all_cold(qacct_wall-command_seconds)+120_save);"
    "ceil(max(300,1.25*alpha_hi+1.5*200*beta_hi+headroom))"
)

CALIBRATION_MEMORY_GIB = 56
BATCH_MEMORY_FRACTION = 0.35
BATCH_BASE_WIDTH = 8
BATCH_BASE_SCRATCH_BYTES = 7_447_296_000
FEASIBLE_EXPLICIT_BATCHES = (8, 16)
INFEASIBLE_BATCHES = (32, 64, 128)
CALIBRATION_ROUTES = ("auto", "cmg")
CALIBRATION_BATCH_REQUESTS = ("8", "16", "auto")
CALIBRATION_REPETITIONS = (1, 2, 3)
CALIBRATION_PATTERN = re.compile(
    r"cal_(auto|cmg)_b(8|16|auto)_p(20|40)_(cold|warm)_r([123])"
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def read_one(path: Path) -> dict[str, str]:
    require(path.is_file(), f"missing calibration output: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    require(len(rows) == 1, f"calibration output must have one row: {path}")
    return rows[0]


def read_plan(path: Path) -> dict[str, dict[str, str]]:
    require(path.is_file(), f"missing experiment plan: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle, delimiter="\t"))
    return {row["experiment_id"]: row for row in rows}


def finite(row: dict[str, str], field: str) -> float:
    try:
        value = float(row[field])
    except (KeyError, ValueError) as exc:
        raise ValueError(f"invalid {field}") from exc
    require(math.isfinite(value), f"nonfinite {field}")
    return value


def timing_identity_after_csv(correction: float, leverage: float,
                              target: float) -> bool:
    return abs(correction - leverage - target) <= (
        CSV_TIMING_IDENTITY_TOLERANCE * (1 + abs(correction))
    )


def timing_identity_diagnostic(correction: float, leverage: float,
                               target: float) -> str:
    difference = abs(correction - leverage - target)
    bound = CSV_TIMING_IDENTITY_TOLERANCE * (1 + abs(correction))
    return (f"correction={correction:.17g} leverage={leverage:.17g} "
            f"target={target:.17g} difference={difference:.17g} "
            f"bound={bound:.17g}")


def identity(row: dict[str, str], prefix: str) -> None:
    total = finite(row, f"{prefix}_total")
    parts = finite(row, f"{prefix}_worker") + finite(row, f"{prefix}_firm")
    parts += 2 * finite(row, f"{prefix}_covariance")
    require(abs(total - parts) <= 1e-8 * (1 + abs(parts)), f"{prefix} identity failed")


def memory_bytes(value: str) -> float:
    match = re.fullmatch(r"([0-9]+(?:\.[0-9]+)?)([KMGTP]?)", value.strip(), re.I)
    require(match is not None, f"unparseable memory value: {value}")
    scale = {"": 1, "K": 1024, "M": 1024**2, "G": 1024**3,
             "T": 1024**4, "P": 1024**5}[match.group(2).upper()]
    return float(match.group(1)) * scale


def batch_memory_budget_bytes() -> int:
    return math.floor(BATCH_MEMORY_FRACTION * CALIBRATION_MEMORY_GIB * 1024**3)


def forecast_batch_bytes(width: int) -> int:
    require(width > 0 and width % BATCH_BASE_WIDTH == 0, "invalid forecast batch width")
    return BATCH_BASE_SCRATCH_BYTES * width // BATCH_BASE_WIDTH


def calibration_id(route: str, batch: str, probes: int,
                   temperature: str, repetition: int) -> str:
    return f"cal_{route}_b{batch}_p{probes}_{temperature}_r{repetition}"


def duration_seconds(value: str) -> float:
    """Parse the numeric or HH:MM:SS duration forms emitted by qacct."""

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


def projection_from_measurements(
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
        require(probes in CALIBRATION_PROBES and temperature in {"cold", "warm"},
                "invalid projection cell")
        require((probes, temperature) not in by_cell, "duplicate projection cell")
        require(all(math.isfinite(value) for value in (command, correction, wall)) and
                command >= 0 and correction >= 0 and wall >= 0,
                "invalid projection timing")
        require(correction <= command + 1.0,
                "correction timer exceeds enclosing command timer")
        by_cell[(probes, temperature)] = row
    require(set(by_cell) == {(probes, temperature)
                             for probes in CALIBRATION_PROBES
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
    cold_overhead = max(
        0.0,
        *(float(row["qacct_wall_seconds"]) - float(row["command_seconds"])
          for row in measurements if row["temperature"] == "cold"),
    )
    headroom = max(
        float(FIXED_HEADROOM_SECONDS),
        cold_overhead + FULL_RETAINED_SAVE_ALLOWANCE_SECONDS,
    )
    projected = (SETUP_SAFETY_FACTOR * alpha +
                 MARGINAL_SAFETY_FACTOR * FULL_PROBES * beta +
                 headroom)
    timeout = math.ceil(max(MINIMUM_TIMEOUT_SECONDS, projected))
    return alpha, beta, headroom, projected, timeout


def conservative_projection_from_repetitions(
    measurements: list[dict[str, float | str]],
) -> tuple[float, float, float, float, int, str]:
    """Upper-envelope admission projection over all independent repetitions.

    The independently scattered qsub jobs have no intentionally matched host
    blocks. Command slopes therefore always use the conservative cross-host
    upper envelope max(C40)-min(C20); incidental hostname overlap is not a
    pairing and never changes the projection.
    """

    expected = len(CALIBRATION_PROBES) * 2 * len(CALIBRATION_REPETITIONS)
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
        require(probes in CALIBRATION_PROBES and temperature in {"cold", "warm"},
                "invalid conservative projection cell")
        require(hostname != "" and all(math.isfinite(value)
                for value in (command, correction, wall)) and
                command >= 0 and correction >= 0 and wall >= 0,
                "invalid conservative projection timing/accounting")
        require(correction <= command + 1.0,
                "correction timer exceeds enclosing command timer")
        cells.setdefault((probes, temperature), []).append(row)
    expected_cells = {(probes, temperature)
                      for probes in CALIBRATION_PROBES
                      for temperature in ("cold", "warm")}
    require(set(cells) == expected_cells and
            all(len(rows) == len(CALIBRATION_REPETITIONS)
                for rows in cells.values()),
            "conservative repetition matrix is incomplete")

    slope_candidates: list[float] = []
    slope_modes: list[str] = []
    for temperature in ("cold", "warm"):
        low = cells[(20, temperature)]
        high = cells[(40, temperature)]
        slope = ((max(float(row["command_seconds"]) for row in high) -
                  min(float(row["command_seconds"]) for row in low)) / 20.0)
        slope_modes.append(f"{temperature}=cross_host_upper_envelope")
        slope_candidates.append(slope)

    beta_correction = max(
        float(row["correction_seconds"]) / int(row["probes"])
        for row in measurements
    )
    beta = max(0.0, beta_correction, *slope_candidates)
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
    projected = (SETUP_SAFETY_FACTOR * alpha +
                 MARGINAL_SAFETY_FACTOR * FULL_PROBES * beta + headroom)
    timeout = math.ceil(max(MINIMUM_TIMEOUT_SECONDS, projected))
    return alpha, beta, headroom, projected, timeout, "|".join(slope_modes)


def same_host_coincidence_diagnostic(
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


def timing_inversion_status(
    summaries: list[dict[str, float | str]],
) -> tuple[str, str]:
    """Classify median P40<P20 timing as unexplained, never as host failure."""

    by_cell = {(int(row["probes"]), str(row["temperature"])): row
               for row in summaries}
    inverted = [temperature for temperature in ("cold", "warm")
                if float(by_cell[(40, temperature)]["command_seconds"]) <
                float(by_cell[(20, temperature)]["command_seconds"])]
    if not inverted:
        return "NONE", "-"
    return "UNEXPLAINED_VARIABILITY", "|".join(inverted)


def candidate_sort_key(row: dict[str, object]) -> tuple[object, ...]:
    # Median summaries rank typical performance. Admission and production
    # timeout remain bound to the all-repetition upper envelope.
    return (
        row["typical_timeout_seconds"],
        0 if row["preconditioner"] == "auto" else 1,
        row["typical_projected_seconds"],
        row["timeout_seconds"],
        row["batch"],
    )


def conservative_candidate_sort_key(row: dict[str, object]) -> tuple[object, ...]:
    batch_order = {"8": 0, "16": 1, "auto": 2}
    return (
        row["timeout_seconds"],
        row["projected_seconds"],
        row["batch"],
        batch_order[str(row["batch_request"])],
    )


def node_class_multiset(measurements: list[dict[str, float | str]]) -> str:
    classes = sorted([
        (str(row["uname_machine"]), str(row["cpu_model"]),
         int(row["logical_cpus"]))
        for row in measurements
    ])
    require(len(classes) == 12, "node-class multiset requires 12 repetitions")
    return json.dumps(classes, separators=(",", ":"))


def select_public_automatic_candidate(
    candidates: list[dict[str, object]],
) -> tuple[dict[str, object], str, bool]:
    automatic = [candidate for candidate in candidates
                 if candidate["preconditioner"] == "auto"]
    require(automatic,
            "no public automatic-route calibration projects inside the SCC runtime envelope")
    identical = len({str(candidate["node_class_multiset"])
                     for candidate in automatic}) == 1
    if identical:
        return (min(automatic, key=candidate_sort_key),
                "MEDIAN_TYPICAL_IDENTICAL_NODE_CLASS_MULTISET", True)
    return (min(automatic, key=conservative_candidate_sort_key),
            "CONSERVATIVE_HIGH_HETEROGENEOUS_NODE_CLASS_MULTISET", False)


def qacct_pass(path: Path, processors: int,
               expected_job_id: str) -> dict[str, float | str]:
    require(path.is_file(), f"missing calibration qacct: {path}")
    text = path.read_text(encoding="utf-8", errors="replace")
    def field(name: str) -> str:
        matches = re.findall(rf"(?m)^{name}\s+(\S+)", text)
        require(len(matches) == 1, f"qacct missing/duplicate {name}: {path}")
        return matches[0]
    require(field("jobnumber") == expected_job_id.split(".", 1)[0],
            f"qacct job number mismatch: {path}")
    require(field("failed") == "0" and field("exit_status") == "0", f"failed qacct: {path}")
    require(int(field("slots")) == processors, f"qacct slot mismatch: {path}")
    maxvmem = memory_bytes(field("maxvmem"))
    require(maxvmem <= 60 * 1024**3,
            f"qacct memory exceeded policy: {path}")
    wall = duration_seconds(field("ru_wallclock"))
    cpu = duration_seconds(field("cpu"))
    hostname = field("hostname")
    qname = field("qname")
    require(re.fullmatch(r"[^\s,|]+", hostname) is not None and
            re.fullmatch(r"[^\s,|]+", qname) is not None,
            f"invalid qacct host/queue accounting: {path}")
    return {
        "wall_seconds": wall,
        "cpu_seconds": cpu,
        "hostname": hostname,
        "qname": qname,
        "maxvmem_bytes": maxvmem,
    }


def run_metadata_pass(path: Path, *, experiment: str, stage: str,
                      bundle_sha: str, source_commit: str, manifest_sha: str,
                      expected_job_id: str, processors: int,
                      qacct_hostname: str) -> None:
    require(path.is_file(), f"missing run metadata: {path}")
    tokens = path.read_text(encoding="utf-8", errors="replace").strip().split()
    values: dict[str, str] = {}
    for token in tokens:
        key, separator, value = token.partition("=")
        require(separator == "=" and key not in values,
                f"invalid run metadata token: {path}")
        values[key] = value
    require(set(values) == {
        "experiment", "stage", "bundle", "source", "manifest", "job", "host", "slots",
    }, f"run metadata schema changed: {path}")
    expected = {
        "experiment": experiment, "stage": stage, "bundle": bundle_sha,
        "source": source_commit, "manifest": manifest_sha,
        "job": expected_job_id.split(".", 1)[0], "slots": str(processors),
    }
    require(all(values[key] == value for key, value in expected.items()) and
            values["host"].split(".", 1)[0] == qacct_hostname.split(".", 1)[0],
            f"run metadata identity changed: {path}")


def resource_pass(path: Path) -> None:
    require(path.is_file(), f"missing resource evidence: {path}")
    text = path.read_text(encoding="utf-8", errors="replace")
    match = re.search(r"Maximum resident set size \(kbytes\):\s*(\d+)", text)
    require(match is not None and 0 < int(match.group(1)) <= 60 * 1024**2,
            f"resource RSS exceeded policy or is absent: {path}")


def node_characteristics_pass(
    path: Path, accounting: dict[str, float | str],
) -> dict[str, str]:
    require(path.is_file(), f"missing node characteristics: {path}")
    values: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
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
            str(accounting["hostname"]).split(".", 1)[0],
            f"node and qacct host disagree: {path}")
    return values


def validate_rhs(path: Path, expected: int) -> tuple[tuple[str, ...], ...]:
    require(path.is_file(), f"missing RHS evidence: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    require(len(rows) == expected, f"RHS evidence count mismatch: {path}")
    require(all(finite(row, "converged") == 1 for row in rows), f"unaccepted RHS: {path}")
    require(all(finite(row, "relative_residual") >= 0 for row in rows),
            f"negative RHS residual: {path}")
    residual = max(finite(row, "relative_residual") for row in rows)
    require(residual <= COMPLETE_RESIDUAL_GATE * (1 + 1e-10), f"RHS residual gate failed: {path}")
    fields = ("repetition", "stage", "batch_start", "rhs", "iterations",
              "relative_residual", "converged")
    return tuple(tuple(row[field] for field in fields) for row in rows)


def evidence_digest(run_dir: Path, experiment_ids: list[str]) -> str:
    digest = hashlib.sha256()
    relative_paths: list[Path] = []
    for experiment in experiment_ids:
        root = Path("experiments") / experiment
        relative_paths.extend((
            root / f"prod_{experiment}.csv",
            root / "retained_matches.csv",
            root / "resources.txt",
            root / "node_characteristics.txt",
            root / "wrapper.pass",
            root / "rhs_repetition_1.csv",
            Path("qacct") / f"{experiment}.txt",
        ))
    for relative in sorted(relative_paths, key=str):
        path = run_dir / relative
        require(path.is_file(), f"missing calibration evidence: {path}")
        digest.update(str(relative).encode() + b"\0")
        digest.update(path.read_bytes())
    return digest.hexdigest()


def validate_row(
    args: argparse.Namespace,
    plan: dict[str, dict[str, str]],
    experiment: str,
) -> dict[str, str]:
    spec = plan.get(experiment)
    require(spec is not None and spec["stage"] == "calibration", "unplanned calibration")
    root = args.run_dir / "experiments" / experiment
    row = read_one(root / f"prod_{experiment}.csv")
    require(row["experiment_id"] == experiment and finite(row, "repetition") == 1,
            "calibration experiment/repetition identity changed")
    require(spec["repetitions"] == "1" and spec["depends"] == "cz18_preflight",
            "calibration job is not an independent single repetition")
    require(row["bundle_sha256"] == args.bundle_sha, "bundle changed in calibration")
    require(row["source_commit"] == args.source_commit, "source changed in calibration")
    require(row["data_manifest_sha256"] == args.data_manifest_sha,
            "data manifest changed in calibration")
    require(row["prepared_sha256"] == args.prepared_sha, "prepared sample changed")
    require(row["estimator_input_sha256"] == args.prepared_sha, "estimator input changed")
    require(row["wage_input_sha256"] == args.wage_sha, "wage input changed")
    require(row["temperature"] == spec["temperature"], "calibration temperature changed")
    require(row["estimator_status"] == "KSS_POINT_ESTIMATES_ONLY", "calibration failed")
    require(row["algorithm_requested"] == row["algorithm_selected"] == "jla",
            "calibration did not exercise JLA")
    require(finite(row, "command_rc") == 0 and finite(row, "command_seconds") >= 0,
            "calibration command failed or has invalid timing")
    leverage_seconds = finite(row, "leverage_seconds")
    target_seconds = finite(row, "target_seconds")
    correction_seconds = finite(row, "correction_seconds")
    require(leverage_seconds >= 0 and target_seconds >= 0 and correction_seconds >= 0,
            "calibration stage timing is negative")
    require(timing_identity_after_csv(
                correction_seconds, leverage_seconds, target_seconds),
            f"calibration correction timing identity failed: {experiment}: "
            f"{timing_identity_diagnostic(correction_seconds, leverage_seconds, target_seconds)}")
    require(correction_seconds <= finite(row, "command_seconds") + 1.0,
            "calibration correction timer exceeds command timer")
    require(abs(finite(row, "tolerance") - REQUESTED_TOLERANCE) <= 1e-20,
            "calibration tolerance changed")
    require(0 <= finite(row, "solver_max_residual") <=
            COMPLETE_RESIDUAL_GATE * (1 + 1e-10), "calibration residual failed")
    require(int(finite(row, "requested_probes")) == int(spec["probes"]),
            "calibration probes changed")
    require(int(finite(row, "seed")) == 8675309, "calibration seed changed")
    require(int(finite(row, "declared_memory_gib")) == CALIBRATION_MEMORY_GIB,
            "calibration memory changed")
    require(int(finite(row, "requested_processors")) == int(spec["processors"]) and
            int(finite(row, "actual_processors")) == int(spec["processors"]),
            "calibration processor count was not bound exactly")
    require(row["batch_requested"] == spec["batch"], "calibration batch changed")
    selected_batch = int(finite(row, "selected_batch"))
    require(selected_batch in FEASIBLE_EXPLICIT_BATCHES, "calibration selected infeasible batch")
    if spec["batch"] != "auto":
        require(selected_batch == int(spec["batch"]), "explicit calibration batch changed")
    require(int(finite(row, "batch_scratch_forecast_bytes")) ==
            forecast_batch_bytes(selected_batch), "batch scratch forecast changed")
    require(finite(row, "batch_scratch_forecast_bytes") <= batch_memory_budget_bytes(),
            "calibration batch exceeds registered scratch budget")
    require(row["preconditioner_requested"] == spec["preconditioner"],
            "calibration route changed")
    require(finite(row, "rng_state_reproducible") == 1,
            "calibration terminal RNG state is not reproducible")
    require(finite(row, "route_planned_rhs") == 3 * finite(row, "requested_probes") + 1,
            "calibration RHS plan changed")
    identity(row, "plugin")
    identity(row, "corrected")
    expected_job_id = (args.run_dir / "submissions" /
                       f"{experiment}.job_id").read_text(encoding="utf-8").strip()
    row["_qacct"] = qacct_pass(
        args.run_dir / "qacct" / f"{experiment}.txt",
        int(spec["processors"]), expected_job_id,
    )  # type: ignore[assignment]
    run_metadata_pass(
        root / "run.metadata.txt", experiment=experiment, stage="calibration",
        bundle_sha=args.bundle_sha, source_commit=args.source_commit,
        manifest_sha=args.data_manifest_sha, expected_job_id=expected_job_id,
        processors=int(spec["processors"]),
        qacct_hostname=str(row["_qacct"]["hostname"]),
    )
    row["_node"] = node_characteristics_pass(
        root / "node_characteristics.txt", row["_qacct"]
    )  # type: ignore[assignment]
    resource_pass(root / "resources.txt")
    row["_rhs_signature"] = validate_rhs(
        root / "rhs_repetition_1.csv", int(finite(row, "route_planned_rhs"))
    )  # type: ignore[assignment]
    wrapper = root / "wrapper.pass"
    expected = (f"KSS_PROD_WRAPPER_PASS {experiment} {args.bundle_sha} "
                f"{args.source_commit} {args.data_manifest_sha}\n")
    require(wrapper.read_text(encoding="utf-8") == expected, "calibration wrapper mismatch")
    row["_retained_bytes"] = (root / "retained_matches.csv").read_bytes()  # type: ignore[assignment]
    return row


def compare_rhs_signatures(left_rhs: tuple[tuple[str, ...], ...],
                           right_rhs: tuple[tuple[str, ...], ...],
                           label: str) -> None:
    require(len(left_rhs) == len(right_rhs), f"{label} RHS count changed")
    for left_item, right_item in zip(left_rhs, right_rhs):
        require(left_item[:5] == right_item[:5] and left_item[6] == right_item[6],
                f"{label} RHS discrete certificate changed")
        left_residual, right_residual = float(left_item[5]), float(right_item[5])
        require(abs(left_residual - right_residual) <=
                1e-10 * (1 + abs(left_residual)),
                f"{label} RHS residual changed")


def compare_scientific_rows(left: dict[str, str], right: dict[str, str],
                            label: str, corrected: bool) -> None:
    fields = ["plugin_worker", "plugin_firm", "plugin_covariance", "plugin_total"]
    if corrected:
        fields.extend(("corrected_worker", "corrected_firm",
                       "corrected_covariance", "corrected_total"))
    for field in fields:
        left_value, right_value = finite(left, field), finite(right, field)
        require(abs(left_value - right_value) <= 1e-10 * (1 + abs(left_value)),
                f"{label} {field} changed")
    require(left["_retained_bytes"] == right["_retained_bytes"],
            f"{label} retained sample changed")
    compare_rhs_signatures(left["_rhs_signature"], right["_rhs_signature"], label)
    for field in ("selected_batch", "N_retained", "worker_levels", "firm_levels",
                  "deletion_units", "route_hybrid_vertices", "route_hybrid_edges",
                  "route_hierarchy_levels", "route_planned_rhs"):
        require(finite(left, field) == finite(right, field),
                f"{label} {field} changed")
    for field in ("algorithm_requested", "algorithm_selected", "batch_requested",
                  "preconditioner_requested", "preconditioner_selected",
                  "fallback_status", "routing_reason"):
        require(left[field] == right[field], f"{label} {field} changed")


def compare_auto_forced_rows(automatic: dict[str, str], forced: dict[str, str],
                             label: str) -> None:
    for field in ("plugin_worker", "plugin_firm", "plugin_covariance", "plugin_total",
                  "corrected_worker", "corrected_firm", "corrected_covariance",
                  "corrected_total"):
        left, right = finite(automatic, field), finite(forced, field)
        require(abs(left - right) <= 2e-9 * (1 + abs(left)),
                f"{label} auto/forced target changed: {field}")
    require(automatic["_retained_bytes"] == forced["_retained_bytes"],
            f"{label} auto/forced retained sample changed")
    require(automatic["preconditioner_selected"] ==
            forced["preconditioner_selected"] == "cmg",
            f"{label} auto/forced selected route changed")
    for field in ("selected_batch", "rng_state_reproducible", "seed", "tolerance",
                  "N_retained", "worker_levels", "firm_levels", "deletion_units",
                  "route_hybrid_vertices", "route_hybrid_edges",
                  "route_hierarchy_levels", "route_terminal_vertices",
                  "route_planned_rhs", "solver_iterations", "solver_max_residual"):
        left, right = finite(automatic, field), finite(forced, field)
        require(abs(left - right) <= 1e-10 * (1 + abs(left)),
                f"{label} auto/forced route/graph changed: {field}")
    compare_rhs_signatures(automatic["_rhs_signature"], forced["_rhs_signature"],
                           f"{label} auto/forced")


def summarize_cell(rows: list[dict[str, str]], probes: int,
                   temperature: str) -> dict[str, float | str]:
    require(len(rows) == len(CALIBRATION_REPETITIONS),
            "calibration cell does not have three independent repetitions")
    reference = rows[0]
    for row in rows[1:]:
        compare_scientific_rows(reference, row, "cross-repetition", corrected=True)
    accounting = [row["_qacct"] for row in rows]
    nodes = [row["_node"] for row in rows]
    commands = [finite(row, "command_seconds") for row in rows]
    corrections = [finite(row, "correction_seconds") for row in rows]
    walls = [float(item["wall_seconds"]) for item in accounting]
    return {
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
        "hostnames": "|".join(sorted({str(item["hostname"]) for item in accounting})),
        "qnames": "|".join(sorted({str(item["qname"]) for item in accounting})),
        "uname_machines": "|".join(sorted(
            {str(item["uname_machine"]) for item in nodes})),
        "cpu_models": "|".join(sorted({str(item["cpu_model"]) for item in nodes})),
        "logical_cpus": "|".join(sorted({str(item["logical_cpus"]) for item in nodes})),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--run-dir", required=True, type=Path)
    parser.add_argument("--plan", required=True, type=Path)
    parser.add_argument("--bundle-sha", required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--data-manifest-sha", required=True)
    parser.add_argument("--prepared-sha", required=True)
    parser.add_argument("--wage-sha", required=True)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    require(re.fullmatch(r"[0-9a-f]{64}", args.bundle_sha) is not None, "invalid bundle SHA")
    require(re.fullmatch(r"[0-9a-f]{40}", args.source_commit) is not None, "invalid source commit")
    for value in (args.data_manifest_sha, args.prepared_sha, args.wage_sha):
        require(re.fullmatch(r"[0-9a-f]{64}", value) is not None, "invalid input SHA")

    plan = read_plan(args.plan)
    calibration_ids = sorted(
        experiment for experiment, spec in plan.items() if spec["stage"] == "calibration"
    )
    require(len(calibration_ids) == 72 and
            all(CALIBRATION_PATTERN.fullmatch(experiment) for experiment in calibration_ids),
            "calibration plan is not the registered 72-job repetition matrix")
    evidence = {experiment: validate_row(args, plan, experiment)
                for experiment in calibration_ids}
    for batch_request in CALIBRATION_BATCH_REQUESTS:
        for probes in CALIBRATION_PROBES:
            for temperature in ("cold", "warm"):
                for repetition in CALIBRATION_REPETITIONS:
                    automatic = evidence[calibration_id(
                        "auto", batch_request, probes, temperature, repetition)]
                    forced = evidence[calibration_id(
                        "cmg", batch_request, probes, temperature, repetition)]
                    compare_auto_forced_rows(
                        automatic, forced,
                        f"b{batch_request} P{probes} {temperature} r{repetition}",
                    )
    candidates: list[dict[str, object]] = []

    for route in CALIBRATION_ROUTES:
        for batch_request in CALIBRATION_BATCH_REQUESTS:
            cells: dict[tuple[int, str], list[dict[str, str]]] = {}
            summaries: list[dict[str, float | str]] = []
            for probes in CALIBRATION_PROBES:
                for temperature in ("cold", "warm"):
                    ids = [calibration_id(route, batch_request, probes,
                                          temperature, repetition)
                           for repetition in CALIBRATION_REPETITIONS]
                    cell_rows = [evidence[experiment] for experiment in ids]
                    cells[(probes, temperature)] = cell_rows
                    summaries.append(summarize_cell(cell_rows, probes, temperature))
                compare_scientific_rows(
                    cells[(probes, "cold")][0], cells[(probes, "warm")][0],
                    "cold/warm", corrected=True,
                )

            for temperature in ("cold", "warm"):
                low = cells[(20, temperature)][0]
                high = cells[(40, temperature)][0]
                require(low["_retained_bytes"] == high["_retained_bytes"],
                        "P20/P40 retained sample changed")
                for field in ("plugin_worker", "plugin_firm", "plugin_covariance", "plugin_total"):
                    require(abs(finite(low, field) - finite(high, field)) <=
                            1e-10 * (1 + abs(finite(low, field))),
                            f"P20/P40 plug-in {field} changed")
                for field in ("selected_batch", "N_retained", "worker_levels", "firm_levels",
                              "deletion_units", "route_hybrid_vertices", "route_hybrid_edges",
                              "route_hierarchy_levels"):
                    require(finite(low, field) == finite(high, field),
                            f"P20/P40 {field} changed")
                require(low["preconditioner_selected"] == high["preconditioner_selected"],
                        "P20/P40 selected route changed")

            selected_batches = {int(finite(row, "selected_batch"))
                                for cell in cells.values() for row in cell}
            selected_routes = {row["preconditioner_selected"]
                               for cell in cells.values() for row in cell}
            typical_measurements = [
                {
                    "probes": int(summary["probes"]),
                    "temperature": str(summary["temperature"]),
                    "command_seconds": float(summary["command_seconds"]),
                    "correction_seconds": float(summary["correction_seconds"]),
                    "qacct_wall_seconds": float(summary["qacct_wall_seconds"]),
                }
                for summary in summaries
            ]
            raw_measurements = [
                {
                    "probes": probes,
                    "temperature": temperature,
                    "command_seconds": finite(row, "command_seconds"),
                    "correction_seconds": finite(row, "correction_seconds"),
                    "qacct_wall_seconds": float(row["_qacct"]["wall_seconds"]),
                    "hostname": str(row["_qacct"]["hostname"]),
                    "qname": str(row["_qacct"]["qname"]),
                    "uname_machine": str(row["_node"]["uname_machine"]),
                    "cpu_model": str(row["_node"]["cpu_model"]),
                    "logical_cpus": int(row["_node"]["logical_cpus"]),
                }
                for (probes, temperature), cell in cells.items()
                for row in cell
            ]
            typical_alpha, typical_beta, typical_headroom, \
                typical_projected, typical_timeout = \
                projection_from_measurements(typical_measurements)
            alpha, beta, headroom, projected, timeout, slope_policy = \
                conservative_projection_from_repetitions(raw_measurements)
            if (len(selected_batches) != 1 or selected_routes != {"cmg"} or
                    timeout > MAXIMUM_TIMEOUT_SECONDS):
                continue
            selected_batch = selected_batches.pop()
            inversion_status, inversion_temperatures = timing_inversion_status(summaries)
            candidate: dict[str, object] = {
                "preconditioner": route,
                "selected_preconditioner": "cmg",
                "batch_request": batch_request,
                "batch": selected_batch,
                "processors": 4,
                "actual_processors": 4,
                "memory_gib": CALIBRATION_MEMORY_GIB,
                "calibration_repetitions_per_cell": len(CALIBRATION_REPETITIONS),
                "timing_summary": "median_of_3_no_trimming",
                "t20_seconds": max(float(row["command_seconds"])
                                    for row in typical_measurements
                                    if row["probes"] == 20),
                "t40_seconds": max(float(row["command_seconds"])
                                    for row in typical_measurements
                                    if row["probes"] == 40),
                "typical_alpha_seconds": typical_alpha,
                "typical_beta_seconds_per_probe": typical_beta,
                "typical_headroom_seconds": typical_headroom,
                "typical_projected_seconds": typical_projected,
                "typical_timeout_seconds": typical_timeout,
                "alpha_seconds": alpha,
                "beta_seconds_per_probe": beta,
                "headroom_seconds": headroom,
                "projected_seconds": projected,
                "timeout_seconds": timeout,
                "slope_pairing_policy": slope_policy,
                "same_host_coincidences": same_host_coincidence_diagnostic(
                    raw_measurements),
                "timing_inversion_status": inversion_status,
                "timing_inversion_temperatures": inversion_temperatures,
                "hostname_policy": "DIFFERENT_HOSTS_ALLOWED_NOT_CAUSAL",
                "node_class_multiset": node_class_multiset(raw_measurements),
            }
            for summary in summaries:
                prefix = f"p{summary['probes']}_{summary['temperature']}"
                candidate[f"{prefix}_experiments"] = "|".join(
                    calibration_id(route, batch_request, int(summary["probes"]),
                                   str(summary["temperature"]), repetition)
                    for repetition in CALIBRATION_REPETITIONS
                )
                for field in (
                    "command_seconds", "command_min_seconds", "command_max_seconds",
                    "command_range_seconds", "correction_seconds",
                    "correction_min_seconds", "correction_max_seconds",
                    "qacct_wall_seconds", "qacct_wall_min_seconds",
                    "qacct_wall_max_seconds", "hostnames", "qnames",
                    "uname_machines", "cpu_models", "logical_cpus",
                ):
                    candidate[f"{prefix}_{field}"] = summary[field]
            selected_accounting = [row["_qacct"]
                                   for cell in cells.values() for row in cell]
            selected_nodes = [row["_node"]
                              for cell in cells.values() for row in cell]
            candidate.update({
                "selected_hostnames": "|".join(sorted(
                    {str(item["hostname"]) for item in selected_accounting})),
                "selected_qnames": "|".join(sorted(
                    {str(item["qname"]) for item in selected_accounting})),
                "selected_uname_machines": "|".join(sorted(
                    {str(item["uname_machine"]) for item in selected_nodes})),
                "selected_cpu_models": "|".join(sorted(
                    {str(item["cpu_model"]) for item in selected_nodes})),
                "selected_logical_cpus": "|".join(sorted(
                    {str(item["logical_cpus"]) for item in selected_nodes})),
                "selected_qacct_wall_min_seconds": min(
                    float(item["wall_seconds"]) for item in selected_accounting),
                "selected_qacct_wall_max_seconds": max(
                    float(item["wall_seconds"]) for item in selected_accounting),
                "selected_qacct_cpu_min_seconds": min(
                    float(item["cpu_seconds"]) for item in selected_accounting),
                "selected_qacct_cpu_max_seconds": max(
                    float(item["cpu_seconds"]) for item in selected_accounting),
                "selected_qacct_maxvmem_bytes": max(
                    float(item["maxvmem_bytes"]) for item in selected_accounting),
            })
            candidates.append(candidate)

    require(candidates,
            "no paired CMG calibration projects inside the SCC runtime envelope")
    automatic_candidates = [candidate for candidate in candidates
                            if candidate["preconditioner"] == "auto"]
    selected, ranking_policy, node_characteristics_identical = \
        select_public_automatic_candidate(candidates)
    budget = batch_memory_budget_bytes()
    selected.update({
        "bundle_sha256": args.bundle_sha,
        "source_commit": args.source_commit,
        "data_manifest_sha256": args.data_manifest_sha,
        "prepared_sha256": args.prepared_sha,
        "wage_input_sha256": args.wage_sha,
        "calibration_evidence_sha256": evidence_digest(args.run_dir, calibration_ids),
        "full_probes": FULL_PROBES,
        "formula": FORMULA,
        "maximum_process_timeout_seconds": MAXIMUM_TIMEOUT_SECONDS,
        "wrapper_runtime_margin_seconds": WRAPPER_RUNTIME_MARGIN_SECONDS,
        "scc_hard_runtime_ceiling_seconds": SCC_HARD_RUNTIME_CEILING_SECONDS,
        "candidate_count": len(candidates),
        "automatic_candidate_count": len(automatic_candidates),
        "ranking_policy": ranking_policy,
        "auto_node_characteristics_identical": int(node_characteristics_identical),
        "auto_node_class_multisets": json.dumps({
            str(candidate["batch_request"]): json.loads(
                str(candidate["node_class_multiset"]))
            for candidate in automatic_candidates
        }, sort_keys=True, separators=(",", ":")),
        "measured_candidate_count": 6,
        "measured_job_count": len(calibration_ids),
        "all_calibration_hostnames": "|".join(sorted({
            str(row["_qacct"]["hostname"]) for row in evidence.values()
        })),
        "all_calibration_qnames": "|".join(sorted({
            str(row["_qacct"]["qname"]) for row in evidence.values()
        })),
        "all_calibration_uname_machines": "|".join(sorted({
            str(row["_node"]["uname_machine"]) for row in evidence.values()
        })),
        "all_calibration_cpu_models": "|".join(sorted({
            str(row["_node"]["cpu_model"]) for row in evidence.values()
        })),
        "all_calibration_logical_cpus": "|".join(sorted({
            str(row["_node"]["logical_cpus"]) for row in evidence.values()
        })),
        "batch_memory_fraction": BATCH_MEMORY_FRACTION,
        "batch_memory_budget_bytes": budget,
        "batch8_scratch_forecast_bytes": forecast_batch_bytes(8),
        "batch16_scratch_forecast_bytes": forecast_batch_bytes(16),
        "batch32_scratch_forecast_bytes": forecast_batch_bytes(32),
        "batch64_scratch_forecast_bytes": forecast_batch_bytes(64),
        "batch128_scratch_forecast_bytes": forecast_batch_bytes(128),
        "feasible_batch_widths": "8|16|auto",
        "infeasible_batch_widths": "32|64|128",
    })
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(selected))
        writer.writeheader()
        writer.writerow(selected)
    print("KSS_PROD CALIBRATION SELECTOR PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

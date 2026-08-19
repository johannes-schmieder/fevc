"""Shared validation primitives for PREP-BND-1 benchmark evidence."""

from __future__ import annotations

import csv
import hashlib
import math
import re
from pathlib import Path

MARKER_LOCAL = "KSS_PREP_BND1_LOCAL_PASS"
MARKER_SCALE = "KSS_PREP_BND1_SCC_PASS"
MARKER_CZ18 = "KSS_PREP_BND1_CZ18_PASS"
INPUT_CZ18_SHA256 = "1748ca2a6a46f248e05c0329407e7e7708ec7628c1ffce5f0e06ee264bdf0575"
PREP_RHS_SCHEMA = "PREP-RHS-PERF-V1"
PREP_RHS_METRICS = (
    "selection_other",
    "graph_prune",
    "semantic_order",
    "compression_prepare",
    "lifecycle_transition",
    "lifecycle_restore",
    "observed_total",
)
PREP_BND_SCHEMA = "PREP-BND-PERF-V1"
PREP_BND_METRICS = (
    "mark_validate",
    "initial_group",
    "runtime_setup",
    "graph_setup_io",
    "graph_prune",
    "retained_map",
    "semantic_order",
    "compression_prepare",
    "lifecycle_transition",
    "lifecycle_restore",
    "observed_total",
)
PREP_BND_COUNTS_SCHEMA = "PREP-BND-COUNTS-V1"
PREP_BND_COUNT_METRICS = (
    "initial_id_group_calls",
    "deletion_group_calls",
    "retained_id_group_calls",
    "semantic_group_calls",
    "stata_sort_calls",
    "graph_import_columns",
    "graph_import_rows",
    "retained_map_columns",
    "retained_map_rows",
    "compression_import_columns",
    "compression_import_rows",
)
TIMING_FIELDS = (
    "command_s",
    "selection_s",
    "graph_s",
    "semantic_s",
    "compression_s",
    "prep_observed_s",
    "transition_s",
    "work_s",
    "restore_s",
    "setup_s",
    "fit_s",
    "leverage_s",
    "target_s",
    "correction_s",
    "rng_s",
    "schur_s",
    "precond_s",
    "pcg_s",
)
EXACT_FIELDS = (
    "processors",
    "probes",
    "seed",
    "input_sha256",
    "selected_batch",
    "route",
    "engine",
    "estimator_status",
    "iterations",
    "schur_actions",
    "schur_batches",
    "precond_apps",
    "precond_batches",
    "acceptance",
    "n_rows",
    "n_retained",
    "cells",
    "units",
    "strata",
    "workers",
    "firms",
    "sample_count",
    "sample_signature",
    "life_sample_restored",
    "data_restored",
    "rng_restored",
    "sort_rng_restored",
    "sort_restored",
    "fe_applicable",
    "fe_workspace_builds",
    "fe_buffered_batches",
    "fe_legacy_batches",
    "fe_buffered_columns",
    "fe_legacy_columns",
    "fe_fallback_batches",
    "fe_max_width",
    "fe_workspace_bytes",
    "fe_avoided_bytes",
)
SCIENTIFIC_FIELDS = (
    "max_residual",
    "identity_residual",
    "r11",
    "r21",
    "r31",
    "r41",
    "r12",
    "r22",
    "r32",
    "r42",
    "r13",
    "r23",
    "r33",
    "r43",
    "r14",
    "r24",
    "r34",
    "r44",
)
SCIENTIFIC_ABS_REL_TOLERANCE = 2e-12
HEX40 = re.compile(r"^[0-9a-f]{40}$")
HEX64 = re.compile(r"^[0-9a-f]{64}$")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def sha256(path: Path) -> str:
    require(path.is_file(), f"missing file: {path}")
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def read_rows(path: Path, role: str, commit: str, repetitions: int) -> list[dict[str, str]]:
    require(HEX40.fullmatch(commit) is not None, f"invalid {role} commit")
    require(path.is_file(), f"missing summary CSV: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    require(len(rows) == repetitions, f"{path}: expected {repetitions} rows")
    for run, row in enumerate(rows, 1):
        require(row["source_label"] == role, f"{path}: role binding failed")
        require(row["source_commit"] == commit, f"{path}: commit binding failed")
        require(int(row["run"]) == run, f"{path}: noncanonical run order")
        expected_temperature = "cold" if run == 1 else "warm"
        require(row["temperature"] == expected_temperature, f"{path}: bad temperature")
        require(float(row["result_mreldif"]) <= 2e-9, f"{path}: repeated result drift")
        require(int(float(row["processors"])) == 4, f"{path}: not four processors")
        require(
            float(row["max_residual"]) <= float(row["acceptance"]),
            f"{path}: residual acceptance failed",
        )
        for field in TIMING_FIELDS:
            value = float(row[field])
            require(math.isfinite(value) and value >= 0, f"{path}: invalid {field}")
        for field in (
            "life_sample_restored",
            "data_restored",
            "rng_restored",
            "sort_rng_restored",
            "sort_restored",
        ):
            require(int(float(row[field])) == 1, f"{path}: caller state failed: {field}")
    return rows


def read_profiles(
    path: Path, role: str, commit: str, repetitions: int
) -> list[dict[str, str]]:
    require(path.is_file(), f"missing profile CSV: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    for row in rows:
        require(row["source_label"] == role, f"{path}: profile role binding failed")
        require(row["source_commit"] == commit, f"{path}: profile commit binding failed")
        require(1 <= int(row["run"]) <= repetitions, f"{path}: profile run invalid")
        value = float(row["value"])
        require(math.isfinite(value) and value >= 0, f"{path}: profile value invalid")
    for run in range(1, repetitions + 1):
        prep = [row for row in rows if int(row["run"]) == run and row["matrix_name"] == "prep_profile"]
        require({row["metric"] for row in prep} == set(PREP_RHS_METRICS), f"{path}: PREP-RHS metrics differ")
        require({row["schema"] for row in prep} == {PREP_RHS_SCHEMA}, f"{path}: PREP-RHS schema differs")
        boundary = [row for row in rows if int(row["run"]) == run and row["matrix_name"] == "prep_boundary_profile"]
        if boundary:
            require({row["metric"] for row in boundary} == set(PREP_BND_METRICS), f"{path}: PREP-BND metrics differ")
            require({row["schema"] for row in boundary} == {PREP_BND_SCHEMA}, f"{path}: PREP-BND schema differs")
        counts = [row for row in rows if int(row["run"]) == run and row["matrix_name"] == "prep_boundary_counts"]
        if counts:
            require({row["metric"] for row in counts} == set(PREP_BND_COUNT_METRICS), f"{path}: PREP-BND count metrics differ")
            require({row["schema"] for row in counts} == {PREP_BND_COUNTS_SCHEMA}, f"{path}: PREP-BND count schema differs")
    return rows


def compare_scientific_contract(
    baseline: list[dict[str, str]], candidate: list[dict[str, str]], label: str
) -> None:
    require(len(baseline) == len(candidate), f"{label}: repetition count differs")
    for run, (left, right) in enumerate(zip(baseline, candidate, strict=True), 1):
        for field in EXACT_FIELDS:
            require(left[field] == right[field], f"{label} run {run}: mismatch {field}")
        for field in SCIENTIFIC_FIELDS:
            left_value = float(left[field])
            right_value = float(right[field])
            require(
                math.isfinite(left_value) and math.isfinite(right_value),
                f"{label} run {run}: nonfinite {field}",
            )
            require(
                math.isclose(
                    left_value,
                    right_value,
                    rel_tol=SCIENTIFIC_ABS_REL_TOLERANCE,
                    abs_tol=SCIENTIFIC_ABS_REL_TOLERANCE,
                ),
                f"{label} run {run}: numerical mismatch {field}",
            )


def profile_map(rows: list[dict[str, str]], matrix_name: str, run: int) -> dict[str, float]:
    return {
        row["metric"]: float(row["value"])
        for row in rows
        if row["matrix_name"] == matrix_name and int(row["run"]) == run
    }


def validate_causal_transition(
    baseline_profiles: list[dict[str, str]],
    candidate_profiles: list[dict[str, str]],
    candidate_rows: list[dict[str, str]],
) -> list[dict[str, object]]:
    require(len(candidate_rows) > 0, "missing candidate rows")
    transitions: list[dict[str, object]] = []
    for run, row in enumerate(candidate_rows, 1):
        baseline = profile_map(baseline_profiles, "prep_boundary_counts", run)
        candidate = profile_map(candidate_profiles, "prep_boundary_counts", run)
        require(set(baseline) == set(PREP_BND_COUNT_METRICS), "baseline PREP-BND counts absent")
        require(set(candidate) == set(PREP_BND_COUNT_METRICS), "candidate PREP-BND counts absent")
        require(baseline["retained_id_group_calls"] == 2, "baseline retained grouping exposure differs")
        require(candidate["retained_id_group_calls"] == 0, "candidate retained grouping was not eliminated")
        require(baseline["semantic_group_calls"] == 1, "baseline semantic grouping exposure differs")
        require(baseline["stata_sort_calls"] == 2, "baseline Stata sort exposure differs")
        candidate_semantic = candidate["semantic_group_calls"]
        candidate_sorts = candidate["stata_sort_calls"]
        require(candidate_semantic in (0, 1), "candidate semantic grouping exposure is invalid")
        require(
            candidate_sorts == candidate_semantic + 1,
            "candidate semantic grouping and sort exposures are inconsistent",
        )
        unchanged = [
            metric
            for metric in PREP_BND_COUNT_METRICS
            if metric
            not in ("retained_id_group_calls", "semantic_group_calls", "stata_sort_calls")
        ]
        for metric in unchanged:
            require(baseline[metric] == candidate[metric], f"causal count changed: {metric}")
        retained = float(row["n_retained"])
        require(candidate["retained_map_columns"] == 2, "candidate retained map width differs")
        require(candidate["retained_map_rows"] == retained, "candidate retained map row count differs")
        require(candidate["compression_import_rows"] == retained, "candidate compression import row count differs")
        transitions.append(
            {
                "run": run,
                "baseline_retained_id_group_calls": 2,
                "candidate_retained_id_group_calls": 0,
                "baseline_semantic_group_calls": 1,
                "candidate_semantic_group_calls": int(candidate_semantic),
                "baseline_stata_sort_calls": 2,
                "candidate_stata_sort_calls": int(candidate_sorts),
                "unchanged_count_metrics": unchanged,
                "candidate_retained_map_columns": 2,
                "candidate_retained_map_rows": int(candidate["retained_map_rows"]),
            }
        )
    return transitions


def read_key_value(path: Path) -> dict[str, str]:
    require(path.is_file(), f"missing receipt: {path}")
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.reader(handle, delimiter="\t"))
    require(rows and rows[0] == ["key", "value"], f"invalid receipt header: {path}")
    out: dict[str, str] = {}
    for row in rows[1:]:
        require(len(row) == 2 and row[0] not in out, f"invalid receipt row: {path}")
        out[row[0]] = row[1]
    return out


def parse_qacct(path: Path, expected_job_id: str, expected_slots: int) -> dict[str, str]:
    require(path.is_file(), f"missing qacct: {path}")
    fields: dict[str, list[str]] = {}
    for raw in path.read_text(encoding="utf-8").splitlines():
        parts = raw.split(None, 1)
        if len(parts) == 2:
            fields.setdefault(parts[0], []).append(parts[1].strip())
    for name in ("jobnumber", "failed", "exit_status", "hostname", "qname", "slots", "ru_wallclock", "cpu", "maxvmem"):
        require(len(fields.get(name, [])) == 1, f"{path}: missing/duplicate {name}")
    one = {name: values[0] for name, values in fields.items()}
    require(one["jobnumber"] == expected_job_id, f"{path}: job number mismatch")
    require(one["failed"] == "0", f"{path}: SGE failed")
    require(one["exit_status"] == "0", f"{path}: nonzero exit status")
    require(int(one["slots"]) == expected_slots, f"{path}: slot request mismatch")
    require(float(one["ru_wallclock"]) >= 0 and float(one["cpu"]) >= 0, f"{path}: invalid accounting time")
    require(one["maxvmem"] not in ("", "0", "0.000"), f"{path}: missing maxvmem")
    return one

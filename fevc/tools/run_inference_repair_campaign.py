#!/usr/bin/env python3
"""Run fresh source-bound match inference repair development and confirmation."""

from __future__ import annotations

import argparse
import csv
import hashlib
import importlib.util
import io
import json
import math
import statistics
import subprocess
import sys
from pathlib import Path
from typing import Any, Sequence


BASE_PATH = Path(__file__).with_name("run_match_inference_q0_campaign.py")
BASE_SPEC = importlib.util.spec_from_file_location("fevc_match_q0_campaign_base", BASE_PATH)
assert BASE_SPEC is not None and BASE_SPEC.loader is not None
BASE = importlib.util.module_from_spec(BASE_SPEC)
sys.modules[BASE_SPEC.name] = BASE
BASE_SPEC.loader.exec_module(BASE)

CampaignError = BASE.CampaignError
_binary_hash = BASE._binary_hash
_git_identity = BASE._git_identity
_is_sha256 = BASE._is_sha256
_json_bytes = BASE._json_bytes
_label_hash = BASE._label_hash
_sha256 = BASE._sha256
_splitmix64 = BASE._splitmix64
_write_new = BASE._write_new

MANIFEST_SCHEMA = "fevc-inference-repair-match-campaign-manifest-v1"
PREFLIGHT_SCHEMA = "fevc-match-inference-q1-design-preflight-v1"
PREFLIGHT_RECEIPT_SCHEMA = "fevc-inference-repair-match-preflight-receipt-v1"
ROW_SCHEMA = "fevc-inference-repair-match-row-v1"
TASK_RECEIPT_SCHEMA = "fevc-inference-repair-match-task-receipt-v1"
BUILD_RECEIPT_SCHEMA = "fevc-inference-repair-match-scc-build-v1"
SUMMARY_SCHEMA = "fevc-inference-repair-match-summary-v1"
REGISTRATION_SCHEMA = "FEVC_INFERENCE_REPAIR_MATCH_CAMPAIGN_V1"
REGISTRATION_PATH = Path("fevc/docs/inference_repair_match_campaign_v1.json")
TARGETS = ("worker", "firm", "covariance", "total")
ONE_MODE_TARGETS = ("worker", "firm", "total")
MASTER_SEED = 0x4D61_7463_6851_31A1
CONFIRMATION_SEED = 0xB734_90D2_6A18_CE55
FIXED_FOLD_SEED = 0x862A_D51C_3F07_B994
FAILURE_STATUSES = {
    "backend_failure",
    "point_invariance_failed",
    "covariance_failed",
    "q1_result_failed",
}
COMMON_NUMERIC_FIELDS = (
    "truth",
    "point_estimate",
    "point_error",
    "estimated_variance",
    "estimated_sd",
    "confidence_lower",
    "confidence_upper",
    "effective_match_count",
    "largest_match_mass_share",
    "largest_match_leverage",
    "smallest_maker_denominator",
    "maximum_influence_share",
    "leading_eigenvalue",
    "second_eigenvalue",
    "leading_share",
    "remainder_share",
    "maximum_mode_weight",
    "leading_residual",
    "second_residual",
    "variance_floor_share",
    "boundary_share",
    "maximum_boundary_excess",
    "maximum_prediction_leverage",
    "minimum_fitted_rcond",
    "sensitivity_median_log_ratio",
    "sensitivity_p90_log_ratio",
    "sensitivity_maximum_log_ratio",
    "sensitivity_log_variance_correlation",
    "maximum_complete_residual",
    "full_residual_tolerance",
    "maximum_trace_mcse",
    "psd_cleanup",
    "smallest_covariance_eigenvalue",
    "largest_covariance_eigenvalue",
    "point_correction_identity_error",
    "aggregate_variance_mean",
)
Q1_NUMERIC_FIELDS = (
    "leading_score",
    "raw_leading_variance_correction",
    "leading_variance",
    "leading_recentered_component",
    "remainder_estimate",
    "remainder_identity_error",
    "leading_remainder_covariance",
    "remainder_variance",
    "remainder_trace_mcse",
    "curvature",
    "critical_value",
    "leading_f_statistic",
    "remainder_influence_concentration",
    "q1_point_identity_error",
)
THRESHOLDS = {
    "correct_success_rate": 0.98,
    "bias_mcse_multiplier": 4.0,
    "coverage_absolute_tolerance": 0.03,
    "coverage_mcse_multiplier": 3.0,
    "correct_se_ratio_lower": 0.82,
    "correct_se_ratio_upper": 1.18,
    "mild_success_rate": 0.97,
    "mild_coverage_floor": 0.85,
    "mild_coverage_drop": 0.07,
    "severe_success_rate": 0.85,
}


def _cell(
    name: str,
    gate: str,
    match_mass: str,
    within_match_dependence: str,
    variance_dgp: str,
    variance_model: str,
    target_shape: str,
    signal: str,
    controls: bool,
    solver: str,
    reference_distribution: str,
    coverage_targets: Sequence[str],
) -> dict[str, Any]:
    return {
        "cell": name,
        "gate": gate,
        "match_mass": match_mass,
        "within_match_dependence": within_match_dependence,
        "variance_dgp": variance_dgp,
        "variance_model": variance_model,
        "target_shape": target_shape,
        "signal": signal,
        "controls": controls,
        "solver": solver,
        "reference_distribution": reference_distribution,
        "coverage_targets": list(coverage_targets),
    }


def _one_mode(
    name: str,
    gate: str,
    mass: str,
    dependence: str,
    variance_dgp: str = "correct",
    variance_model: str = "structured_common",
    signal: str = "regular",
    controls: bool = False,
    solver: str = "diagonal",
) -> dict[str, Any]:
    eligible = ONE_MODE_TARGETS if signal == "regular" and not controls and gate != "solver_diagnostic" else ()
    return _cell(
        name,
        gate,
        mass,
        dependence,
        variance_dgp,
        variance_model,
        "one_mode",
        signal,
        controls,
        solver,
        "q1",
        eligible,
    )


CELLS = (
    _one_mode("one_mode_equal_independent", "correct", "equal", "independent"),
    _one_mode("one_mode_equal_independent_cmg", "solver_diagnostic", "equal", "independent", solver="cmg"),
    _one_mode("one_mode_unequal_independent", "correct", "highly_unequal", "independent"),
    _one_mode("one_mode_unequal_common_shock", "correct", "highly_unequal", "common_shock"),
    _one_mode("one_mode_unequal_serial", "correct", "highly_unequal", "serial_ar1"),
    _one_mode("one_mode_equal_aggregate_shapes", "correct", "highly_unequal", "equal_aggregate_mixed_shapes"),
    _one_mode("one_mode_leverage_sensitivity", "correct", "equal", "independent", variance_model="structured_leverage"),
    _one_mode("one_mode_mild_omitted", "mild", "highly_unequal", "independent", variance_dgp="mild_omitted"),
    _one_mode("one_mode_severe_omitted", "descriptive", "highly_unequal", "independent", variance_dgp="severe_omitted"),
    _cell("diffuse_q0_comparator", "q0_comparator", "equal", "independent", "correct", "structured_common", "diffuse", "regular", False, "diagonal", "q0", TARGETS),
    _cell("multi_mode_diagnostic", "multi_mode_diagnostic", "equal", "common_shock", "correct", "structured_common", "multi_mode", "regular", False, "diagonal", "q1", ()),
    _one_mode("weak_signal", "failure_diagnostic", "highly_unequal", "independent", signal="weak"),
    _one_mode("null_signal", "failure_diagnostic", "highly_unequal", "independent", signal="null"),
    _one_mode("controls_varying_fixedoffset", "conditioning_diagnostic", "highly_unequal", "serial_ar1", controls=True),
)
CELL_BY_NAME = {cell["cell"]: cell for cell in CELLS}
TASK_SETTINGS = {
    "estimator_probes": 256,
    "covariance_probes": 512,
    "spectrum_probes": 128,
    "spectrum_iterations": 256,
    "critical_simulations": 4_000,
}
PROFILE_DEFAULTS = {
    "tiny": {
        "cells": CELLS,
        "k": 20,
        "replications": 1,
        "shard_size": 1,
        "settings": TASK_SETTINGS,
        "preflight_spectrum_probes": 4_096,
    },
    "smoke": {
        "cells": (CELL_BY_NAME["controls_varying_fixedoffset"],),
        "k": 20,
        "replications": 2,
        "shard_size": 2,
        "settings": TASK_SETTINGS,
        "preflight_spectrum_probes": 4_096,
    },
    "development": {
        "cells": CELLS,
        "k": 20,
        "replications": 400,
        "shard_size": 20,
        "settings": TASK_SETTINGS,
        "preflight_spectrum_probes": 4_096,
    },
}


PROFILE_DEFAULTS["confirmation"] = {
    **PROFILE_DEFAULTS["development"], "replications": 2500, "shard_size": 50,
}


def _semantic_seed(cell: str, k: int, replication: int, master: int = MASTER_SEED) -> int:
    return _splitmix64(
        master
        ^ _splitmix64(_label_hash(cell))
        ^ _splitmix64(k)
        ^ _splitmix64(replication)
    )


def _finite(value: Any, label: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value):
        raise CampaignError(f"nonfinite or missing {label}")
    return float(value)


def _registration_identity(root: Path) -> dict[str, Any]:
    path = root / REGISTRATION_PATH
    try:
        payload = path.read_bytes()
        registration = json.loads(payload)
    except (OSError, json.JSONDecodeError) as error:
        raise CampaignError(f"cannot read grouped q1 campaign registration: {error}") from error
    if registration.get("schema") != REGISTRATION_SCHEMA or registration.get("status") != "REPAIR_REGISTERED":
        raise CampaignError("grouped q1 campaign registration schema or status mismatch")
    frozen = registration.get("source_binding", {}).get("frozen_file_sha256")
    if not isinstance(frozen, dict) or not frozen:
        raise CampaignError("grouped q1 campaign registration has no frozen file inventory")
    for relative, expected in frozen.items():
        candidate = root / relative
        if not candidate.is_file() or _sha256(candidate.read_bytes()) != expected:
            raise CampaignError(f"grouped q1 frozen file hash mismatch: {relative}")
    return {
        "path": REGISTRATION_PATH.as_posix(),
        "schema": REGISTRATION_SCHEMA,
        "sha256": _sha256(payload),
    }


def _settings_arguments(settings: dict[str, int], *, preflight_spectrum_probes: int | None = None) -> list[str]:
    spectrum = settings["spectrum_probes"] if preflight_spectrum_probes is None else preflight_spectrum_probes
    return [
        str(settings["estimator_probes"]),
        str(settings["covariance_probes"]),
        str(spectrum),
        str(settings["spectrum_iterations"]),
        str(settings["critical_simulations"]),
    ]


def _coverage_eligible(spec: dict[str, Any], target: str) -> bool:
    return target in spec["coverage_targets"]


def _preflight_failures(rows: Sequence[dict[str, Any]], cells: Sequence[dict[str, Any]], k: int) -> list[str]:
    by_key = {(row["cell"], row["target"]): row for row in rows}
    failures = []
    for cell in cells:
        for target in TARGETS:
            row = by_key[(cell["cell"], target)]
            label = f"{cell['cell']}/{target}"
            if row["independent_matches"] != k * k:
                failures.append(f"{label}: independent-match count")
            if not row["nuisance_uncertainty_conditioned_away"]:
                failures.append(f"{label}: conditioning flag")
            if row["coverage_eligible"] != _coverage_eligible(cell, target):
                failures.append(f"{label}: target-specific coverage eligibility")
            for field in (
                "largest_match_mass_share",
                "largest_match_leverage",
                "maximum_mode_weight",
                "maximum_influence_share",
                "leading_share",
                "remainder_share",
            ):
                if not 0.0 <= row[field] <= 1.0:
                    failures.append(f"{label}: invalid {field}")
            if row["smallest_maker_denominator"] <= 0.0:
                failures.append(f"{label}: nonpositive maker denominator")
            if row["leading_residual"] > 0.02 or row["second_residual"] > 0.02:
                failures.append(f"{label}: spectral residual")
    names = {cell["cell"] for cell in cells}
    if {"one_mode_equal_independent", "one_mode_unequal_independent"} <= names:
        equal = by_key[("one_mode_equal_independent", "worker")]
        unequal = by_key[("one_mode_unequal_independent", "worker")]
        if equal["effective_match_count"] < 0.99 * k * k:
            failures.append("equal match mass does not realize full effective count")
        if unequal["effective_match_count"] >= equal["effective_match_count"]:
            failures.append("unequal match mass does not reduce effective count")
    for cell in cells:
        if cell["target_shape"] == "one_mode":
            for target in ONE_MODE_TARGETS:
                row = by_key[(cell["cell"], target)]
                if row["leading_share"] <= 0.75 or row["remainder_share"] >= 0.15:
                    failures.append(f"{cell['cell']}/{target}: one-mode regime")
            if by_key[(cell["cell"], "covariance")]["remainder_share"] <= 0.50:
                failures.append(f"{cell['cell']}/covariance: missing multi-mode limitation")
        elif cell["target_shape"] == "diffuse":
            for target in ONE_MODE_TARGETS:
                if by_key[(cell["cell"], target)]["leading_share"] >= 0.12:
                    failures.append(f"{cell['cell']}/{target}: diffuse leading concentration")
            if by_key[(cell["cell"], "covariance")]["leading_share"] >= 0.25:
                failures.append(f"{cell['cell']}/covariance: diffuse leading concentration")
        elif cell["target_shape"] == "multi_mode":
            for target in TARGETS:
                if by_key[(cell["cell"], target)]["remainder_share"] <= 0.35:
                    failures.append(f"{cell['cell']}/{target}: missing concentrated remainder")
    return failures


def _validate_preflight_row(row: dict[str, Any], cell: dict[str, Any], k: int) -> None:
    key = (cell["cell"], row.get("target"))
    if (
        row.get("schema") != PREFLIGHT_SCHEMA
        or row.get("cell") != cell["cell"]
        or row.get("k") != k
        or row.get("target") not in TARGETS
        or row.get("target_shape") != cell["target_shape"]
        or row.get("reference_distribution") != cell["reference_distribution"]
    ):
        raise CampaignError(f"preflight {key} metadata mismatch")
    for field in (
        "effective_match_count",
        "largest_match_mass_share",
        "largest_match_leverage",
        "smallest_maker_denominator",
        "leading_eigenvalue",
        "second_eigenvalue",
        "leading_share",
        "remainder_share",
        "maximum_mode_weight",
        "maximum_influence_share",
        "leading_residual",
        "second_residual",
    ):
        row[field] = _finite(row.get(field), f"preflight {key} {field}")
    for field in ("coverage_eligible", "nuisance_uncertainty_conditioned_away"):
        if not isinstance(row.get(field), bool):
            raise CampaignError(f"preflight {key} malformed {field}")
    if not isinstance(row.get("independent_matches"), int):
        raise CampaignError(f"preflight {key} malformed independent-match count")


def run_preflight(root: Path, profile: str, output_dir: Path, binary: Path) -> dict[str, Any]:
    defaults = PROFILE_DEFAULTS[profile]
    source = _git_identity(root)
    registration = _registration_identity(root)
    rows = []
    commands = []
    for cell in defaults["cells"]:
        command = [
            str(binary),
            "design-preflight-v1",
            cell["cell"],
            str(defaults["k"]),
            *_settings_arguments(
                defaults["settings"],
                preflight_spectrum_probes=defaults["preflight_spectrum_probes"],
            ),
        ]
        completed = subprocess.run(command, capture_output=True, text=True)
        if completed.returncode:
            raise CampaignError(f"preflight {cell['cell']} exited {completed.returncode}: {completed.stderr[-2000:]}")
        commands.append(command)
        seen = set()
        for line in completed.stdout.splitlines():
            try:
                row = json.loads(line)
            except json.JSONDecodeError as error:
                raise CampaignError(f"preflight {cell['cell']} emitted non-JSON output") from error
            _validate_preflight_row(row, cell, defaults["k"])
            key = (row["cell"], row["target"])
            if key in seen:
                raise CampaignError(f"preflight {cell['cell']} duplicate target")
            seen.add(key)
            rows.append(row)
        expected = {(cell["cell"], target) for target in TARGETS}
        if seen != expected:
            raise CampaignError(f"preflight {cell['cell']} target inventory mismatch")
    failures = _preflight_failures(rows, defaults["cells"], defaults["k"])
    if failures:
        raise CampaignError("design preflight failed: " + "; ".join(failures))
    rows.sort(key=lambda row: (row["cell"], row["target"]))
    payload = b"".join(json.dumps(row, sort_keys=True, separators=(",", ":")).encode() + b"\n" for row in rows)
    _write_new(output_dir / "preflight.jsonl", payload)
    receipt = {
        "schema": PREFLIGHT_RECEIPT_SCHEMA,
        "status": "PASS",
        "decision": "outcome_free_q1_design_regimes_realized",
        "profile": profile,
        "source": source,
        "registration": registration,
        "binary_sha256": _binary_hash(binary),
        "commands": commands,
        "preflight_spectrum_probes": defaults["preflight_spectrum_probes"],
        "row_count": len(rows),
        "output_sha256": _sha256(payload),
        "failures": [],
    }
    _write_new(output_dir / "receipt.json", _json_bytes(receipt))
    return receipt


def _read_preflight_receipt(path: Path, source: dict[str, Any], registration: dict[str, Any], profile: str) -> dict[str, Any]:
    try:
        receipt_payload = path.read_bytes()
        receipt = json.loads(receipt_payload)
        output_payload = path.with_name("preflight.jsonl").read_bytes()
    except (OSError, json.JSONDecodeError) as error:
        raise CampaignError(f"cannot read q1 preflight receipt: {error}") from error
    defaults = PROFILE_DEFAULTS[profile]
    if (
        receipt.get("schema") != PREFLIGHT_RECEIPT_SCHEMA
        or receipt.get("status") != "PASS"
        or receipt.get("profile") != profile
        or receipt.get("source") != source
        or receipt.get("registration") != registration
        or receipt.get("preflight_spectrum_probes") != defaults["preflight_spectrum_probes"]
        or receipt.get("failures") != []
    ):
        raise CampaignError("q1 preflight receipt does not bind this source, registration, and profile")
    if not _is_sha256(receipt.get("binary_sha256")) or not _is_sha256(receipt.get("output_sha256")):
        raise CampaignError("q1 preflight receipt has malformed artifact hashes")
    if _sha256(output_payload) != receipt["output_sha256"]:
        raise CampaignError("q1 preflight payload does not match its receipt")
    expected = {(cell["cell"], target): cell for cell in defaults["cells"] for target in TARGETS}
    seen = set()
    rows = []
    for line in output_payload.splitlines():
        try:
            row = json.loads(line)
        except json.JSONDecodeError as error:
            raise CampaignError("q1 preflight payload contains malformed JSON") from error
        key = (row.get("cell"), row.get("target"))
        if key not in expected or key in seen:
            raise CampaignError("q1 preflight payload has an invalid or duplicate target key")
        _validate_preflight_row(row, expected[key], defaults["k"])
        seen.add(key)
        rows.append(row)
    if seen != set(expected) or receipt.get("row_count") != len(rows):
        raise CampaignError("q1 preflight payload inventory does not match its receipt")
    failures = _preflight_failures(rows, defaults["cells"], defaults["k"])
    if failures:
        raise CampaignError("q1 preflight payload fails registered gates: " + "; ".join(failures))
    return {
        "path": "preflight/receipt.json",
        "sha256": _sha256(receipt_payload),
        "binary_sha256": receipt["binary_sha256"],
        "output_sha256": receipt["output_sha256"],
    }


def create_manifest(root: Path, profile: str, output: Path, preflight_receipt: Path, shard_size: int | None) -> dict[str, Any]:
    defaults = PROFILE_DEFAULTS[profile]
    shard = shard_size or defaults["shard_size"]
    replications = defaults["replications"]
    if shard <= 0 or replications % shard:
        raise CampaignError("shard size must divide the replication count")
    source = _git_identity(root)
    registration = _registration_identity(root)
    if profile in {"smoke", "development", "confirmation"} and source["dirty"]:
        raise CampaignError(f"{profile} manifests require a clean source tree")
    preflight = _read_preflight_receipt(preflight_receipt, source, registration, profile)
    tasks = []
    for cell in defaults["cells"]:
        for start in range(0, replications, shard):
            task_id = len(tasks) + 1
            tasks.append({
                "task_id": task_id,
                "cell": cell["cell"],
                "k": defaults["k"],
                "replication_start": start,
                "replications": shard,
                "output": f"task-{task_id:05d}.jsonl",
                "receipt": f"task-{task_id:05d}.receipt.json",
            })
    manifest = {
        "schema": MANIFEST_SCHEMA,
        "status": "FROZEN_REPAIR_TASKS",
        "profile": profile,
        "source": source,
        "registration": registration,
        "preflight": preflight,
        "scientific_contract": {
            "deletion": "match",
            "nuisance": "fixedoffset",
            "reference_distribution": "target_specific_q0_or_q1_without_automatic_selection",
            "inference_units": "independent_declared_matches",
            "within_match_dependence": "unrestricted_through_scalar_aggregate_variance",
            "cross_match_dependence": "independent",
            "nuisance_uncertainty": "ignored_fixed_offset_approximation",
            "master_seed": CONFIRMATION_SEED if profile == "confirmation" else MASTER_SEED,
            "fixed_fold_seed": FIXED_FOLD_SEED,
            "critical_reference": "deterministic_quadrature_with_separate_counter_validation",
            "q1_coverage_regime": "one_removed_mode_and_diffuse_remainder_target_by_target",
            "cells": list(defaults["cells"]),
            "replications_per_cell": replications,
            "settings": defaults["settings"],
            "preflight_spectrum_probes": defaults["preflight_spectrum_probes"],
            "thresholds": THRESHOLDS,
        },
        "task_count": len(tasks),
        "expected_row_count": len(defaults["cells"]) * replications * len(TARGETS),
        "tasks": tasks,
    }
    _validate_manifest(manifest)
    _write_new(output, _json_bytes(manifest))
    return manifest


def _validate_manifest(manifest: dict[str, Any]) -> None:
    profile = manifest.get("profile")
    if manifest.get("schema") != MANIFEST_SCHEMA or manifest.get("status") != "FROZEN_REPAIR_TASKS" or profile not in PROFILE_DEFAULTS:
        raise CampaignError("q1 manifest schema, status, or profile mismatch")
    defaults = PROFILE_DEFAULTS[profile]
    tasks = manifest.get("tasks")
    contract = manifest.get("scientific_contract")
    if not isinstance(tasks, list) or not isinstance(contract, dict) or manifest.get("task_count") != len(tasks):
        raise CampaignError("q1 manifest task inventory is malformed")
    cells = contract.get("cells")
    replications = contract.get("replications_per_cell")
    if (
        cells != list(defaults["cells"])
        or replications != defaults["replications"]
        or contract.get("settings") != defaults["settings"]
        or contract.get("preflight_spectrum_probes") != defaults["preflight_spectrum_probes"]
        or contract.get("thresholds") != THRESHOLDS
        or contract.get("master_seed") != (CONFIRMATION_SEED if profile == "confirmation" else MASTER_SEED)
        or contract.get("fixed_fold_seed") != FIXED_FOLD_SEED
        or contract.get("nuisance_uncertainty") != "ignored_fixed_offset_approximation"
        or contract.get("critical_reference") != "deterministic_quadrature_with_separate_counter_validation"
    ):
        raise CampaignError("q1 manifest scientific contract is not the registered profile")
    coverage: dict[str, set[int]] = {cell["cell"]: set() for cell in cells}
    for expected_task_id, task in enumerate(tasks, 1):
        required = {"task_id", "cell", "k", "replication_start", "replications", "output", "receipt"}
        if (set(task) != required or task["task_id"] != expected_task_id or task["cell"] not in coverage or task["k"] != defaults["k"]
            or task["output"] != f"task-{expected_task_id:05d}.jsonl"
            or task["receipt"] != f"task-{expected_task_id:05d}.receipt.json"):
            raise CampaignError("q1 manifest task metadata is malformed")
        start = task["replication_start"]
        count = task["replications"]
        if not isinstance(start, int) or not isinstance(count, int) or start < 0 or count <= 0:
            raise CampaignError("q1 manifest task range is malformed")
        values = set(range(start, start + count))
        if coverage[task["cell"]] & values:
            raise CampaignError("q1 manifest has overlapping replication ranges")
        coverage[task["cell"]].update(values)
    expected = set(range(replications))
    if any(values != expected for values in coverage.values()):
        raise CampaignError("q1 manifest has partial replication coverage")
    if manifest.get("expected_row_count") != len(cells) * replications * len(TARGETS):
        raise CampaignError("q1 manifest expected row count is inconsistent")


def _read_manifest(path: Path) -> dict[str, Any]:
    try:
        manifest = json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as error:
        raise CampaignError(f"cannot read q1 campaign manifest: {error}") from error
    _validate_manifest(manifest)
    return manifest


def _task(manifest: dict[str, Any], task_id: int) -> dict[str, Any]:
    matches = [task for task in manifest["tasks"] if task["task_id"] == task_id]
    if len(matches) != 1:
        raise CampaignError(f"task {task_id} is not uniquely registered")
    return matches[0]


def _cell_spec(manifest: dict[str, Any], task: dict[str, Any]) -> dict[str, Any]:
    matches = [cell for cell in manifest["scientific_contract"]["cells"] if cell["cell"] == task["cell"]]
    if len(matches) != 1:
        raise CampaignError(f"task {task['task_id']} has no unique cell specification")
    return {**matches[0], "master_seed": manifest["scientific_contract"]["master_seed"]}


def _validate_row(row: dict[str, Any], task: dict[str, Any], spec: dict[str, Any], critical_simulations: int) -> tuple[str, int, int, str]:
    key = (row.get("cell"), row.get("k"), row.get("replication"), row.get("target"))
    if row.get("schema") != ROW_SCHEMA or row.get("cell") != task["cell"] or row.get("k") != task["k"] or row.get("target") not in TARGETS:
        raise CampaignError(f"{key}: q1 row identity mismatch")
    replication = row.get("replication")
    if not isinstance(replication, int) or not task["replication_start"] <= replication < task["replication_start"] + task["replications"]:
        raise CampaignError(f"{key}: replication outside task range")
    if row.get("semantic_seed") != _semantic_seed(task["cell"], task["k"], replication, spec["master_seed"]):
        raise CampaignError(f"{key}: semantic seed mismatch")
    for field in (
        "gate", "match_mass", "within_match_dependence", "variance_dgp", "variance_model",
        "target_shape", "signal", "controls", "solver", "reference_distribution",
    ):
        if row.get(field) != spec[field]:
            raise CampaignError(f"{key}: metadata mismatch for {field}")
    target = row["target"]
    if row.get("coverage_eligible") != _coverage_eligible(spec, target):
        raise CampaignError(f"{key}: target-specific coverage eligibility mismatch")
    status = row.get("status")
    if status in {"success", "target_unavailable"}:
        unavailable = status == "target_unavailable"
        if type(row.get("outer_fold_fingerprint")) is not int or not 0 <= row["outer_fold_fingerprint"] < 2**64:
            raise CampaignError(f"{key}: missing fixed-fold fingerprint")
        if unavailable and (spec["reference_distribution"] != "q1" or row.get("q1_status") not in {1, 2, 3, 6}):
            raise CampaignError(f"{key}: untyped target unavailability")
        for field in COMMON_NUMERIC_FIELDS:
            if unavailable and field in {"confidence_lower", "confidence_upper"}:
                if row.get(field) is not None:
                    raise CampaignError(f"{key}: unavailable target has an interval")
                continue
            if row.get("q1_status") == 6 and field in {
                "leading_eigenvalue", "second_eigenvalue", "leading_share", "remainder_share",
                "maximum_mode_weight", "leading_residual", "second_residual",
            } and row.get(field) is None:
                continue
            _finite(row.get(field), f"{key} {field}")
        for field in ("covered", "lower_miss", "upper_miss", "q1_diagnostics_applicable", "nuisance_uncertainty_conditioned_away"):
            if not isinstance(row.get(field), bool):
                raise CampaignError(f"{key}: malformed {field}")
        if sum(bool(row[field]) for field in ("covered", "lower_miss", "upper_miss")) != (0 if unavailable else 1):
            raise CampaignError(f"{key}: incoherent interval outcome")
        for field in ("independent_matches", "minimum_active_terms", "minimum_training_matches", "variance_floor_count", "critical_simulations"):
            if not isinstance(row.get(field), int) or row[field] < 0:
                raise CampaignError(f"{key}: malformed {field}")
        if (
            row["independent_matches"] != task["k"] ** 2
            or row["estimated_variance"] <= 0.0
            or row["estimated_sd"] <= 0.0
            or (not unavailable and row["confidence_lower"] > row["confidence_upper"])
            or row["smallest_maker_denominator"] <= 0.0
            or not row["nuisance_uncertainty_conditioned_away"]
        ):
            raise CampaignError(f"{key}: invalid grouped q1 numerical contract")
        if row["maximum_complete_residual"] > row["full_residual_tolerance"] * (1.0 + 1e-8):
            raise CampaignError(f"{key}: solver residual exceeds receipt tolerance")
        if row["minimum_active_terms"] <= 0 or row["minimum_training_matches"] <= 0:
            raise CampaignError(f"{key}: structured support diagnostics are empty")
        for field in (
            "largest_match_mass_share", "largest_match_leverage", "maximum_influence_share",
            "leading_share", "remainder_share", "maximum_mode_weight", "variance_floor_share", "boundary_share",
        ):
            if row.get("q1_status") == 6 and row.get(field) is None:
                continue
            if not 0.0 <= row[field] <= 1.0:
                raise CampaignError(f"{key}: invalid share {field}")
        if spec["reference_distribution"] == "q1":
            expected_draws = critical_simulations if row.get("q1_status") in {0, 3} else 0
            if (not row["q1_diagnostics_applicable"] or row["critical_simulations"] != expected_draws
                or row.get("quadrature_critical") is not True
                or (not unavailable and row.get("q1_status") != 0)):
                raise CampaignError(f"{key}: q1 accounting mismatch")
            for field in Q1_NUMERIC_FIELDS:
                if unavailable and field in {"curvature", "critical_value", "leading_f_statistic"} and row.get(field) is None:
                    continue
                _finite(row.get(field), f"{key} {field}")
            for field in ("remainder_influence_variance", "remainder_trace_variance"):
                _finite(row.get(field), f"{key} {field}")
            if not math.isclose(row["remainder_influence_variance"] - row["remainder_trace_variance"], row["remainder_variance"], rel_tol=1e-10, abs_tol=1e-12):
                raise CampaignError(f"{key}: remainder covariance decomposition mismatch")
            if not unavailable:
                determinant = _finite(row.get("standardized_determinant"), f"{key} standardized determinant")
                if row["leading_variance"] <= 0.0 or row["remainder_variance"] <= 0.0 or determinant <= 1e-12 or row["critical_value"] <= 0.0:
                    raise CampaignError(f"{key}: q1 covariance or critical value is invalid")
                expected_curvature = 2 * abs(row["leading_eigenvalue"]) * row["leading_variance"] / math.sqrt(row["remainder_variance"] * determinant)
                if not math.isclose(row["curvature"], expected_curvature, rel_tol=1e-9, abs_tol=1e-12):
                    raise CampaignError(f"{key}: corrected curvature identity failed")
            if row["remainder_identity_error"] > max(1e-12, 1e-9 * max(abs(row["point_estimate"]), 1.0)):
                raise CampaignError(f"{key}: q1 remainder identity failed")
            if row["q1_point_identity_error"] > 1e-10 * max(abs(row["point_estimate"]), 1.0):
                raise CampaignError(f"{key}: q1 point identity failed")
            if not 0.0 <= row["remainder_influence_concentration"] <= 1.0:
                raise CampaignError(f"{key}: invalid q1 remainder influence concentration")
        else:
            if row["q1_diagnostics_applicable"] or row["critical_simulations"] != 0:
                raise CampaignError(f"{key}: q0 comparator has q1 accounting")
            if any(row.get(field) is not None for field in Q1_NUMERIC_FIELDS):
                raise CampaignError(f"{key}: q0 comparator has q1 diagnostics")
    elif status in FAILURE_STATUSES:
        if not isinstance(row.get("error_code"), str) or not isinstance(row.get("error_phase"), str):
            raise CampaignError(f"{key}: failure lacks typed error metadata")
    else:
        raise CampaignError(f"{key}: unknown status {status!r}")
    return key


def run_task(manifest_path: Path, task_id: int, output_dir: Path, binary: Path) -> dict[str, Any]:
    manifest = _read_manifest(manifest_path)
    task = _task(manifest, task_id)
    spec = _cell_spec(manifest, task)
    settings = manifest["scientific_contract"]["settings"]
    command = [
        str(binary), "repair-confirmation-v1" if manifest["profile"] == "confirmation" else "repair-development-v1", task["cell"], str(task["k"]),
        str(task["replication_start"]), str(task["replications"]), *_settings_arguments(settings),
    ]
    completed = subprocess.run(command, capture_output=True, text=True)
    if completed.returncode:
        raise CampaignError(f"task {task_id} exited {completed.returncode}: {completed.stderr[-2000:]}")
    rows = []
    seen = set()
    for line in completed.stdout.splitlines():
        if not line.strip():
            continue
        try:
            row = json.loads(line)
        except json.JSONDecodeError as error:
            raise CampaignError(f"task {task_id} emitted non-JSON output") from error
        key = _validate_row(row, task, spec, settings["critical_simulations"])
        if key in seen:
            raise CampaignError(f"task {task_id} emitted duplicate key {key}")
        seen.add(key)
        rows.append((key, row))
    expected = {
        (task["cell"], task["k"], replication, target)
        for replication in range(task["replication_start"], task["replication_start"] + task["replications"])
        for target in TARGETS
    }
    if seen != expected:
        raise CampaignError(f"task {task_id} inventory mismatch: missing={sorted(expected-seen)[:4]} extra={sorted(seen-expected)[:4]}")
    rows.sort(key=lambda item: item[0])
    payload = b"".join(json.dumps(row, sort_keys=True, separators=(",", ":")).encode() + b"\n" for _, row in rows)
    _write_new(output_dir / task["output"], payload)
    receipt = {
        "schema": TASK_RECEIPT_SCHEMA,
        "status": "success",
        "runtime": {"python": sys.version.split()[0], "executable": sys.executable},
        "task": task,
        "manifest_sha256": _sha256(manifest_path.read_bytes()),
        "source": manifest["source"],
        "registration": manifest["registration"],
        "preflight": manifest["preflight"],
        "command": command,
        "binary_sha256": _binary_hash(binary),
        "row_count": len(rows),
        "output_sha256": _sha256(payload),
        "stderr": completed.stderr.splitlines(),
    }
    _write_new(output_dir / task["receipt"], _json_bytes(receipt))
    return receipt


def _read_task_rows(path: Path, task: dict[str, Any], spec: dict[str, Any], critical_simulations: int) -> tuple[list[dict[str, Any]], str]:
    payload = path.read_bytes()
    rows = []
    seen = set()
    for line in payload.splitlines():
        try:
            row = json.loads(line)
        except json.JSONDecodeError as error:
            raise CampaignError(f"{path}: malformed JSON") from error
        key = _validate_row(row, task, spec, critical_simulations)
        if key in seen:
            raise CampaignError(f"{path}: duplicate key {key}")
        seen.add(key)
        rows.append(row)
    expected = {
        (task["cell"], task["k"], replication, target)
        for replication in range(task["replication_start"], task["replication_start"] + task["replications"])
        for target in TARGETS
    }
    if seen != expected:
        raise CampaignError(f"{path}: partial task inventory")
    return rows, _sha256(payload)


def _mean(rows: Sequence[dict[str, Any]], field: str) -> float | None:
    values = [row[field] for row in rows if row.get(field) is not None]
    return statistics.fmean(values) if values else None


def _summary(rows: Sequence[dict[str, Any]], attempts: int) -> dict[str, Any]:
    successful = [row for row in rows if row.get("status") == "success"]
    point_rows = [row for row in rows if row.get("point_error") is not None]
    errors = [row["point_error"] for row in point_rows]
    empirical_sd = statistics.stdev(errors) if len(errors) >= 2 else None
    mean_se = _mean(successful, "estimated_sd")
    coverage = _mean(successful, "covered")
    output = {
        "attempts": attempts,
        "successes": len(successful),
        "success_rate": len(successful) / attempts,
        "bias": statistics.fmean(errors) if errors else None,
        "point_estimates": len(point_rows),
        "bias_mcse": empirical_sd / math.sqrt(len(point_rows)) if empirical_sd is not None else None,
        "coverage_denominator": len(successful),
        "coverage_among_all_attempts": sum(row.get("covered", False) for row in successful) / attempts,
        "coverage": coverage,
        "coverage_mcse": math.sqrt(coverage * (1.0 - coverage) / len(successful)) if coverage is not None else None,
        "lower_miss": _mean(successful, "lower_miss"),
        "upper_miss": _mean(successful, "upper_miss"),
        "empirical_sd": empirical_sd,
        "mean_se": mean_se,
        "se_ratio": empirical_sd / mean_se if empirical_sd is not None and mean_se is not None and mean_se > 0 else None,
        "failure_counts": {},
    }
    for field in COMMON_NUMERIC_FIELDS[7:] + Q1_NUMERIC_FIELDS:
        output[f"mean_{field}"] = _mean(successful, field)
    for row in rows:
        if row.get("status") != "success":
            label = f"{row['status']}:{row.get('q1_status', row.get('error_code', 'NONE'))}:{row.get('error_phase', 'component_inference_q1')}"
            output["failure_counts"][label] = output["failure_counts"].get(label, 0) + 1
    return output


def _gate_correct(label: str, summary: dict[str, Any]) -> list[str]:
    failures = []
    if summary["success_rate"] < THRESHOLDS["correct_success_rate"]:
        failures.append(f"{label}: success rate")
    required = ("bias", "bias_mcse", "coverage", "coverage_mcse", "se_ratio")
    if summary["successes"] < 2 or any(summary[name] is None for name in required):
        failures.append(f"{label}: insufficient successful replications")
        return failures
    if abs(summary["bias"]) > max(1e-12, THRESHOLDS["bias_mcse_multiplier"] * summary["bias_mcse"]):
        failures.append(f"{label}: bias")
    bound = max(THRESHOLDS["coverage_absolute_tolerance"], THRESHOLDS["coverage_mcse_multiplier"] * summary["coverage_mcse"])
    if abs(summary["coverage"] - 0.95) > bound:
        failures.append(f"{label}: coverage")
    if not THRESHOLDS["correct_se_ratio_lower"] <= summary["se_ratio"] <= THRESHOLDS["correct_se_ratio_upper"]:
        failures.append(f"{label}: standard-error ratio")
    return failures


def _scientific_failures(summaries: Sequence[dict[str, Any]], profile: str) -> tuple[list[str], dict[str, Any]]:
    if profile not in {"development", "confirmation"}:
        return [], {"visibly_misspecified": None, "rows": []}
    by_key = {(row["cell"], row["target"]): row for row in summaries}
    failures = []
    for row in summaries:
        label = f"{row['cell']}/{row['target']}"
        eligible = row["target"] in row["coverage_targets"]
        if row["gate"] in {"correct", "q0_comparator"} and eligible:
            failures.extend(_gate_correct(label, row))
        elif row["gate"] == "solver_diagnostic" and row["target"] in ONE_MODE_TARGETS and row["success_rate"] < THRESHOLDS["correct_success_rate"]:
            failures.append(f"{label}: diagnostic success rate")
    for target in ONE_MODE_TARGETS:
        row = by_key[("one_mode_mild_omitted", target)]
        label = f"one_mode_mild_omitted/{target}"
        if row["success_rate"] < THRESHOLDS["mild_success_rate"]:
            failures.append(f"{label}: mild success rate")
        if row["coverage"] is None or row["coverage"] < THRESHOLDS["mild_coverage_floor"]:
            failures.append(f"{label}: mild coverage floor")
        else:
            base = by_key[("one_mode_unequal_independent", target)]
            if base["coverage"] is None or base["coverage_mcse"] is None or row["coverage_mcse"] is None:
                failures.append(f"{label}: mild comparison unavailable")
            else:
                allowance = THRESHOLDS["mild_coverage_drop"] + 3.0 * math.hypot(base["coverage_mcse"], row["coverage_mcse"])
                if base["coverage"] - row["coverage"] > allowance:
                    failures.append(f"{label}: mild coverage degradation")
    severe_rows = []
    for target in ONE_MODE_TARGETS:
        row = by_key[("one_mode_severe_omitted", target)]
        label = f"one_mode_severe_omitted/{target}"
        if row["success_rate"] < THRESHOLDS["severe_success_rate"]:
            failures.append(f"{label}: severe success rate")
        severe_rows.append({"row": label, "coverage": row["coverage"], "se_ratio": row["se_ratio"]})
    visible = any(
        row["coverage"] is not None and row["se_ratio"] is not None
        and (row["coverage"] < 0.88 or row["coverage"] > 0.99 or row["se_ratio"] < 0.78 or row["se_ratio"] > 1.22)
        for row in severe_rows
    )
    if not visible:
        failures.append("severe omitted-driver cells do not expose the structured-model limitation")
    return failures, {"visibly_misspecified": visible, "rows": severe_rows}


def _summaries_csv(summaries: Sequence[dict[str, Any]]) -> bytes:
    stream = io.StringIO(newline="")
    writer = csv.DictWriter(stream, fieldnames=list(summaries[0]), lineterminator="\n")
    writer.writeheader()
    for row in summaries:
        value = dict(row)
        value["coverage_targets"] = json.dumps(value["coverage_targets"], separators=(",", ":"))
        value["failure_counts"] = json.dumps(value["failure_counts"], sort_keys=True, separators=(",", ":"))
        writer.writerow(value)
    return stream.getvalue().encode()


def aggregate(manifest_path: Path, task_dir: Path, output_dir: Path, build_receipt_path: Path | None = None) -> dict[str, Any]:
    manifest = _read_manifest(manifest_path)
    manifest_hash = _sha256(manifest_path.read_bytes())
    settings = manifest["scientific_contract"]["settings"]
    all_rows = []
    task_hashes = {}
    errors = []
    binary_hashes = set()
    for task in manifest["tasks"]:
        try:
            spec = _cell_spec(manifest, task)
            rows, digest = _read_task_rows(task_dir / task["output"], task, spec, settings["critical_simulations"])
            receipt = json.loads((task_dir / task["receipt"]).read_text())
            if (
                receipt.get("schema") != TASK_RECEIPT_SCHEMA or receipt.get("status") != "success"
                or receipt.get("task") != task or receipt.get("manifest_sha256") != manifest_hash
                or receipt.get("source") != manifest["source"] or receipt.get("registration") != manifest["registration"]
                or receipt.get("preflight") != manifest["preflight"] or receipt.get("row_count") != len(rows)
                or receipt.get("output_sha256") != digest
            ):
                raise CampaignError("receipt does not bind q1 task output")
            binary_hashes.add(receipt.get("binary_sha256"))
        except (OSError, json.JSONDecodeError, CampaignError) as error:
            errors.append(f"task {task['task_id']}: {error}")
            continue
        all_rows.extend(rows)
        task_hashes[task["output"]] = digest
    if errors:
        raise CampaignError("; ".join(errors))
    if len(binary_hashes) != 1 or not _is_sha256(next(iter(binary_hashes))):
        raise CampaignError("q1 task receipts do not share one valid binary")
    binary_hash = next(iter(binary_hashes))
    build_receipt_hash = None
    if build_receipt_path is not None:
        try:
            build_payload = build_receipt_path.read_bytes()
            build = json.loads(build_payload)
        except (OSError, json.JSONDecodeError) as error:
            raise CampaignError(f"cannot read q1 build receipt: {error}") from error
        if build.get("schema") != BUILD_RECEIPT_SCHEMA or build.get("status") != "success" or build.get("binary_sha256") != binary_hash or build.get("source_commit") != manifest["source"]["commit"]:
            raise CampaignError("q1 build receipt does not bind manifest source and task binary")
        build_receipt_hash = _sha256(build_payload)
    elif binary_hash != manifest["preflight"]["binary_sha256"]:
        raise CampaignError("local q1 tasks do not use the preflight binary")
    all_rows.sort(key=lambda row: (row["cell"], row["k"], row["replication"], row["target"]))
    if len(all_rows) != manifest["expected_row_count"]:
        raise CampaignError("q1 aggregate row count does not match manifest")
    grouped: dict[tuple[str, str], list[dict[str, Any]]] = {}
    fold_fingerprints: dict[str, set[int]] = {}
    for row in all_rows:
        grouped.setdefault((row["cell"], row["target"]), []).append(row)
        if row.get("outer_fold_fingerprint") is not None:
            fold_fingerprints.setdefault(row["cell"], set()).add(row["outer_fold_fingerprint"])
    if any(len(values) != 1 for values in fold_fingerprints.values()):
        raise CampaignError("outcome-free folds changed across replications")
    summaries = []
    attempts = manifest["scientific_contract"]["replications_per_cell"]
    for spec in manifest["scientific_contract"]["cells"]:
        for target in TARGETS:
            summary = _summary(grouped.get((spec["cell"], target), []), attempts)
            summary.update(spec)
            summary["target"] = target
            summary["coverage_eligible"] = _coverage_eligible(spec, target)
            summaries.append(summary)
    scientific_failures, severe = _scientific_failures(summaries, manifest["profile"])
    status = "PASS" if manifest["profile"] in {"development", "confirmation"} and not scientific_failures else "FAIL" if manifest["profile"] in {"development", "confirmation"} else "COMPLETE"
    aggregate_payload = b"".join(json.dumps(row, sort_keys=True, separators=(",", ":")).encode() + b"\n" for row in all_rows)
    summaries_payload = _summaries_csv(summaries)
    _write_new(output_dir / "aggregate.jsonl", aggregate_payload)
    _write_new(output_dir / "summaries.csv", summaries_payload)
    receipt = {
        "schema": SUMMARY_SCHEMA,
        "status": status,
        "decision": "match_inference_repair_pass" if status == "PASS" else "match_inference_repair_failed" if status == "FAIL" else "pipeline_complete_non_evidentiary",
        "profile": manifest["profile"],
        "runtime": {"python": sys.version.split()[0], "executable": sys.executable},
        "manifest_sha256": manifest_hash,
        "source": manifest["source"],
        "registration": manifest["registration"],
        "preflight": manifest["preflight"],
        "binary_sha256": binary_hash,
        "build_receipt_sha256": build_receipt_hash,
        "row_count": len(all_rows),
        "task_output_sha256": task_hashes,
        "aggregate_sha256": _sha256(aggregate_payload),
        "summaries_sha256": _sha256(summaries_payload),
        "scientific_failures": scientific_failures,
        "severe_misspecification": severe,
        "thresholds": THRESHOLDS,
        "summaries": summaries,
        "interpretation": (
            "Internal suggestive match-cluster inference treating the full-sample nuisance-control offset as fixed and ignoring its estimation uncertainty; "
            "declared matches are independent, dependence within each match is unrestricted through its scalar aggregate variance, "
            "and aggregate match variances follow a named structured model. Q1 removes one leading target-specific mode and requires "
            "a diffuse remainder. This is neither joint-nuisance nor unrestricted-KSS inference."
        ),
    }
    _write_new(output_dir / "receipt.json", _json_bytes(receipt))
    return receipt


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    preflight = subparsers.add_parser("run-preflight")
    preflight.add_argument("--profile", choices=tuple(PROFILE_DEFAULTS), required=True)
    preflight.add_argument("--root", type=Path, required=True)
    preflight.add_argument("--output-dir", type=Path, required=True)
    preflight.add_argument("--binary", type=Path, required=True)
    create = subparsers.add_parser("create-manifest")
    create.add_argument("--profile", choices=tuple(PROFILE_DEFAULTS), required=True)
    create.add_argument("--root", type=Path, required=True)
    create.add_argument("--output", type=Path, required=True)
    create.add_argument("--preflight-receipt", type=Path, required=True)
    create.add_argument("--shard-size", type=int)
    task = subparsers.add_parser("run-task")
    task.add_argument("manifest", type=Path)
    task.add_argument("task_id", type=int)
    task.add_argument("output_dir", type=Path)
    task.add_argument("--binary", type=Path, required=True)
    collect = subparsers.add_parser("aggregate")
    collect.add_argument("manifest", type=Path)
    collect.add_argument("task_dir", type=Path)
    collect.add_argument("output_dir", type=Path)
    collect.add_argument("--build-receipt", type=Path)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    arguments = _parser().parse_args(argv)
    try:
        if arguments.command == "run-preflight":
            result = run_preflight(arguments.root.resolve(), arguments.profile, arguments.output_dir, arguments.binary)
        elif arguments.command == "create-manifest":
            result = create_manifest(arguments.root.resolve(), arguments.profile, arguments.output, arguments.preflight_receipt, arguments.shard_size)
        elif arguments.command == "run-task":
            result = run_task(arguments.manifest, arguments.task_id, arguments.output_dir, arguments.binary)
        else:
            result = aggregate(arguments.manifest, arguments.task_dir, arguments.output_dir, arguments.build_receipt)
    except (CampaignError, OSError, subprocess.SubprocessError, ValueError) as error:
        print(json.dumps({"status": "failure", "error": str(error)}, sort_keys=True))
        return 1
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

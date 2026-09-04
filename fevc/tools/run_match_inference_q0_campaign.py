#!/usr/bin/env python3
"""Run the source-bound fixed-offset collapsed-match q=0 development pipeline."""

from __future__ import annotations

import argparse
import csv
import hashlib
import io
import json
import math
import os
import statistics
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Sequence


MANIFEST_SCHEMA = "fevc-match-inference-q0-campaign-manifest-v1"
PREFLIGHT_SCHEMA = "fevc-match-inference-q0-design-preflight-v1"
PREFLIGHT_RECEIPT_SCHEMA = "fevc-match-inference-q0-preflight-receipt-v1"
ROW_SCHEMA = "fevc-match-inference-q0-development-row-v1"
TASK_RECEIPT_SCHEMA = "fevc-match-inference-q0-task-receipt-v1"
BUILD_RECEIPT_SCHEMA = "fevc-match-inference-q0-scc-build-v1"
SUMMARY_SCHEMA = "fevc-match-inference-q0-summary-v1"
REGISTRATION_SCHEMA = "FEVC_MATCH_INFERENCE_Q0_CAMPAIGN_V1"
REGISTRATION_PATH = Path("fevc/docs/match_inference_q0_campaign_v1.json")
REGISTRATION_AMENDMENT_SCHEMA = "FEVC_MATCH_INFERENCE_Q0_CAMPAIGN_V1_AMENDMENT1"
REGISTRATION_AMENDMENT_PATH = Path(
    "fevc/docs/match_inference_q0_campaign_v1_amendment1.json"
)
TARGETS = ("worker", "firm", "covariance", "total")
MASTER_SEED = 0x98D3_407A_F651_2CBE
FAILURE_STATUSES = {"backend_failure", "point_invariance_failed", "q0_covariance_failed"}
NUMERIC_FIELDS = (
    "truth",
    "point_estimate",
    "point_error",
    "estimated_variance",
    "estimated_sd",
    "effective_match_count",
    "largest_match_mass_share",
    "largest_match_leverage",
    "smallest_maker_denominator",
    "maximum_influence_share",
    "leading_share",
    "remainder_share",
    "maximum_mode_weight",
    "leading_residual",
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
        "coverage_eligible": not controls,
    }


CELLS = (
    _cell("diffuse_equal_independent", "correct", "equal", "independent", "correct", "structured_common", "diffuse", "regular", False, "diagonal"),
    _cell("diffuse_equal_independent_cmg", "solver_diagnostic", "equal", "independent", "correct", "structured_common", "diffuse", "regular", False, "cmg"),
    _cell("diffuse_unequal_independent", "correct", "highly_unequal", "independent", "correct", "structured_common", "diffuse", "regular", False, "diagonal"),
    _cell("diffuse_unequal_common_shock", "correct", "highly_unequal", "common_shock", "correct", "structured_common", "diffuse", "regular", False, "diagonal"),
    _cell("diffuse_unequal_serial", "correct", "highly_unequal", "serial_ar1", "correct", "structured_common", "diffuse", "regular", False, "diagonal"),
    _cell("diffuse_equal_aggregate_shapes", "correct", "highly_unequal", "equal_aggregate_mixed_shapes", "correct", "structured_common", "diffuse", "regular", False, "diagonal"),
    _cell("diffuse_leverage_sensitivity", "correct", "equal", "independent", "correct", "structured_leverage", "diffuse", "regular", False, "diagonal"),
    _cell("diffuse_mild_omitted", "mild", "highly_unequal", "independent", "mild_omitted", "structured_common", "diffuse", "regular", False, "diagonal"),
    _cell("diffuse_severe_omitted", "descriptive", "highly_unequal", "independent", "severe_omitted", "structured_common", "diffuse", "regular", False, "diagonal"),
    _cell("one_mode_diagnostic", "spectral_diagnostic", "equal", "common_shock", "correct", "structured_common", "one_mode", "regular", False, "diagonal"),
    _cell("multi_mode_diagnostic", "spectral_diagnostic", "equal", "common_shock", "correct", "structured_common", "multi_mode", "regular", False, "diagonal"),
    _cell("weak_signal", "failure_diagnostic", "highly_unequal", "independent", "correct", "structured_common", "diffuse", "weak", False, "diagonal"),
    _cell("null_signal", "failure_diagnostic", "highly_unequal", "independent", "correct", "structured_common", "diffuse", "null", False, "diagonal"),
    _cell("controls_varying_fixedoffset", "conditioning_diagnostic", "highly_unequal", "serial_ar1", "correct", "structured_common", "diffuse", "regular", True, "diagonal"),
)
CELL_BY_NAME = {cell["cell"]: cell for cell in CELLS}
PROFILE_DEFAULTS = {
    "tiny": {
        "cells": CELLS,
        "k": 20,
        "replications": 1,
        "shard_size": 1,
        "settings": {"estimator_probes": 256, "covariance_probes": 512, "spectrum_probes": 128, "spectrum_iterations": 256},
    },
    "smoke": {
        "cells": (CELL_BY_NAME["controls_varying_fixedoffset"],),
        "k": 20,
        "replications": 2,
        "shard_size": 2,
        "settings": {"estimator_probes": 96, "covariance_probes": 192, "spectrum_probes": 128, "spectrum_iterations": 256},
    },
    "development": {
        "cells": CELLS,
        "k": 20,
        "replications": 400,
        "shard_size": 20,
        "settings": {"estimator_probes": 256, "covariance_probes": 512, "spectrum_probes": 128, "spectrum_iterations": 256},
    },
}


class CampaignError(RuntimeError):
    """The registered grouped-match campaign contract or evidence is invalid."""


def _json_bytes(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def _sha256(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def _splitmix64(value: int) -> int:
    mask = (1 << 64) - 1
    value = (value + 0x9E37_79B9_7F4A_7C15) & mask
    value = ((value ^ (value >> 30)) * 0xBF58_476D_1CE4_E5B9) & mask
    value = ((value ^ (value >> 27)) * 0x94D0_49BB_1331_11EB) & mask
    return (value ^ (value >> 31)) & mask


def _label_hash(value: str) -> int:
    state = 0xCBF2_9CE4_8422_2325
    for byte in value.encode():
        state = ((state ^ byte) * 0x1000_0000_01B3) & ((1 << 64) - 1)
    return state


def _semantic_seed(cell: str, k: int, replication: int) -> int:
    return _splitmix64(MASTER_SEED ^ _splitmix64(_label_hash(cell)) ^ _splitmix64(k) ^ _splitmix64(replication))


def _write_new(path: Path, payload: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        raise CampaignError(f"refusing to replace existing output {path}")
    descriptor, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(payload)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
    except BaseException:
        Path(temporary).unlink(missing_ok=True)
        raise


def _git_identity(root: Path) -> dict[str, Any]:
    commit = subprocess.run(["git", "rev-parse", "HEAD"], cwd=root, check=True, capture_output=True, text=True).stdout.strip()
    status = subprocess.run(["git", "status", "--porcelain=v1"], cwd=root, check=True, capture_output=True, text=True).stdout
    diff = subprocess.run(["git", "diff", "--binary", "HEAD"], cwd=root, check=True, capture_output=True).stdout
    source_lines = []
    listed = subprocess.run(
        ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
        cwd=root,
        check=True,
        capture_output=True,
    ).stdout.split(b"\0")
    for encoded in listed:
        if not encoded:
            continue
        relative = encoded.decode()
        path = root / relative
        if not path.is_file() or path.is_symlink():
            raise CampaignError(f"source manifest entry is not a regular file: {relative}")
        source_lines.append(f"{_sha256(path.read_bytes())}  {relative}\n")
    return {
        "commit": commit,
        "dirty": bool(status),
        "status": status.splitlines(),
        "worktree_diff_sha256": _sha256(diff),
        "source_file_count": len(source_lines),
        "source_manifest_sha256": _sha256("".join(source_lines).encode()),
    }


def _registration_identity(root: Path) -> dict[str, Any]:
    path = root / REGISTRATION_PATH
    amendment_path = root / REGISTRATION_AMENDMENT_PATH
    try:
        payload = path.read_bytes()
        registration = json.loads(payload)
        amendment_payload = amendment_path.read_bytes()
        amendment = json.loads(amendment_payload)
    except (OSError, json.JSONDecodeError) as error:
        raise CampaignError(f"cannot read grouped q0 registration: {error}") from error
    if registration.get("schema") != REGISTRATION_SCHEMA or registration.get("status") != "DEVELOPMENT_REGISTERED":
        raise CampaignError("grouped q0 registration schema or status mismatch")
    if (
        amendment.get("schema") != REGISTRATION_AMENDMENT_SCHEMA
        or amendment.get("status") != "DEVELOPMENT_REGISTERED_AMENDMENT"
    ):
        raise CampaignError("grouped q0 registration amendment schema or status mismatch")
    amends = amendment.get("amends")
    if (
        not isinstance(amends, dict)
        or amends.get("path") != REGISTRATION_PATH.as_posix()
        or amends.get("sha256") != _sha256(payload)
    ):
        raise CampaignError("grouped q0 registration amendment does not bind the original")
    frozen = registration.get("source_binding", {}).get("frozen_file_sha256")
    if not isinstance(frozen, dict) or not frozen:
        raise CampaignError("grouped q0 registration has no frozen file inventory")
    overrides = amendment.get("source_binding", {}).get("frozen_file_sha256")
    if not isinstance(overrides, dict) or not overrides or not set(overrides) <= set(frozen):
        raise CampaignError("grouped q0 registration amendment has an invalid frozen override")
    effective = {**frozen, **overrides}
    for relative, expected in effective.items():
        candidate = root / relative
        if not candidate.is_file() or _sha256(candidate.read_bytes()) != expected:
            raise CampaignError(f"grouped q0 frozen file hash mismatch: {relative}")
    return {
        "path": REGISTRATION_PATH.as_posix(),
        "schema": REGISTRATION_SCHEMA,
        "sha256": _sha256(payload),
        "amendment": {
            "path": REGISTRATION_AMENDMENT_PATH.as_posix(),
            "schema": REGISTRATION_AMENDMENT_SCHEMA,
            "sha256": _sha256(amendment_payload),
        },
    }


def _settings_arguments(settings: dict[str, int]) -> list[str]:
    return [str(settings[name]) for name in ("estimator_probes", "covariance_probes", "spectrum_probes", "spectrum_iterations")]


def _finite(value: Any, label: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value):
        raise CampaignError(f"nonfinite or missing {label}")
    return float(value)


def _binary_hash(path: Path) -> str:
    if not path.is_file():
        raise CampaignError(f"campaign binary does not exist: {path}")
    return _sha256(path.read_bytes())


def _is_sha256(value: Any) -> bool:
    return (
        isinstance(value, str)
        and len(value) == 64
        and all(character in "0123456789abcdef" for character in value)
    )


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
            for field in ("largest_match_mass_share", "largest_match_leverage", "maximum_mode_weight", "maximum_influence_share"):
                if not 0.0 <= row[field] < 1.0:
                    failures.append(f"{label}: invalid {field}")
            if row["smallest_maker_denominator"] <= 0.0:
                failures.append(f"{label}: nonpositive maker denominator")
    if {cell["cell"] for cell in cells} >= {"diffuse_equal_independent", "diffuse_unequal_independent"}:
        equal = by_key[("diffuse_equal_independent", "worker")]
        unequal = by_key[("diffuse_unequal_independent", "worker")]
        if equal["effective_match_count"] < 0.99 * k * k:
            failures.append("equal match mass does not realize full effective count")
        if unequal["effective_match_count"] >= equal["effective_match_count"]:
            failures.append("unequal match mass does not reduce effective count")
    if "diffuse_equal_independent" in {cell["cell"] for cell in cells}:
        for target in ("worker", "firm", "total"):
            if by_key[("diffuse_equal_independent", target)]["leading_share"] >= 0.12:
                failures.append(f"diffuse_equal_independent/{target}: leading concentration")
        if by_key[("diffuse_equal_independent", "covariance")]["leading_share"] >= 0.25:
            failures.append("diffuse_equal_independent/covariance: leading concentration")
    if "one_mode_diagnostic" in {cell["cell"] for cell in cells}:
        for target in ("worker", "firm", "total"):
            row = by_key[("one_mode_diagnostic", target)]
            if row["leading_share"] <= 0.75 or row["remainder_share"] >= 0.15:
                failures.append(f"one_mode_diagnostic/{target}: one-mode regime")
        if by_key[("one_mode_diagnostic", "covariance")]["remainder_share"] <= 0.50:
            failures.append("one_mode_diagnostic/covariance: missing multi-mode limitation")
    if "multi_mode_diagnostic" in {cell["cell"] for cell in cells}:
        for target in TARGETS:
            if by_key[("multi_mode_diagnostic", target)]["remainder_share"] <= 0.35:
                failures.append(f"multi_mode_diagnostic/{target}: missing concentrated remainder")
    return failures


def run_preflight(root: Path, profile: str, output_dir: Path, binary: Path) -> dict[str, Any]:
    defaults = PROFILE_DEFAULTS[profile]
    source = _git_identity(root)
    registration = _registration_identity(root)
    rows = []
    commands = []
    for cell in defaults["cells"]:
        command = [str(binary), "design-preflight-v1", cell["cell"], str(defaults["k"]), *_settings_arguments(defaults["settings"])]
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
            key = (row.get("cell"), row.get("target"))
            if row.get("schema") != PREFLIGHT_SCHEMA or row.get("cell") != cell["cell"] or row.get("k") != defaults["k"] or row.get("target_shape") != cell["target_shape"] or row.get("target") not in TARGETS:
                raise CampaignError(f"preflight {cell['cell']} metadata mismatch")
            if key in seen:
                raise CampaignError(f"preflight {cell['cell']} duplicate target")
            seen.add(key)
            for field in ("effective_match_count", "largest_match_mass_share", "largest_match_leverage", "smallest_maker_denominator", "leading_share", "remainder_share", "maximum_mode_weight", "maximum_influence_share"):
                row[field] = _finite(row.get(field), f"preflight {cell['cell']} {field}")
            if not isinstance(row.get("independent_matches"), int) or not isinstance(row.get("nuisance_uncertainty_conditioned_away"), bool):
                raise CampaignError(f"preflight {cell['cell']} malformed count or conditioning flag")
            rows.append(row)
        if seen != {(cell["cell"], target) for target in TARGETS}:
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
        "decision": "outcome_free_design_regimes_realized",
        "profile": profile,
        "source": source,
        "registration": registration,
        "binary_sha256": _binary_hash(binary),
        "commands": commands,
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
        raise CampaignError(f"cannot read preflight receipt: {error}") from error
    if receipt.get("schema") != PREFLIGHT_RECEIPT_SCHEMA or receipt.get("status") != "PASS" or receipt.get("profile") != profile or receipt.get("source") != source or receipt.get("registration") != registration or receipt.get("failures") != []:
        raise CampaignError("preflight receipt does not bind this source, registration, and profile")
    if not _is_sha256(receipt.get("binary_sha256")) or not _is_sha256(receipt.get("output_sha256")):
        raise CampaignError("preflight receipt has malformed artifact hashes")
    if _sha256(output_payload) != receipt["output_sha256"]:
        raise CampaignError("preflight payload does not match its receipt")
    defaults = PROFILE_DEFAULTS[profile]
    expected = {
        (cell["cell"], target): (cell, target)
        for cell in defaults["cells"]
        for target in TARGETS
    }
    seen = set()
    rows = []
    for line in output_payload.splitlines():
        try:
            row = json.loads(line)
        except json.JSONDecodeError as error:
            raise CampaignError("preflight payload contains malformed JSON") from error
        key = (row.get("cell"), row.get("target"))
        if key not in expected or key in seen:
            raise CampaignError("preflight payload has an invalid or duplicate target key")
        cell, _ = expected[key]
        if row.get("schema") != PREFLIGHT_SCHEMA or row.get("k") != defaults["k"] or row.get("target_shape") != cell["target_shape"]:
            raise CampaignError("preflight payload metadata mismatch")
        for field in ("effective_match_count", "largest_match_mass_share", "largest_match_leverage", "smallest_maker_denominator", "leading_share", "remainder_share", "maximum_mode_weight", "maximum_influence_share"):
            row[field] = _finite(row.get(field), f"preflight payload {key} {field}")
        if not isinstance(row.get("independent_matches"), int) or not isinstance(row.get("nuisance_uncertainty_conditioned_away"), bool):
            raise CampaignError("preflight payload has malformed count or conditioning flag")
        seen.add(key)
        rows.append(row)
    if seen != set(expected) or receipt.get("row_count") != len(rows):
        raise CampaignError("preflight payload inventory does not match its receipt")
    failures = _preflight_failures(rows, defaults["cells"], defaults["k"])
    if failures:
        raise CampaignError("preflight payload fails registered gates: " + "; ".join(failures))
    return {"path": "preflight/receipt.json", "sha256": _sha256(receipt_payload), "binary_sha256": receipt["binary_sha256"], "output_sha256": receipt["output_sha256"]}


def create_manifest(root: Path, profile: str, output: Path, preflight_receipt: Path, shard_size: int | None) -> dict[str, Any]:
    defaults = PROFILE_DEFAULTS[profile]
    shard = shard_size or defaults["shard_size"]
    replications = defaults["replications"]
    if shard <= 0 or replications % shard:
        raise CampaignError("shard size must divide the replication count")
    source = _git_identity(root)
    registration = _registration_identity(root)
    if profile in {"smoke", "development"} and source["dirty"]:
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
        "status": "FROZEN_DEVELOPMENT_TASKS",
        "profile": profile,
        "source": source,
        "registration": registration,
        "preflight": preflight,
        "scientific_contract": {
            "deletion": "match",
            "nuisance": "fixedoffset",
            "reference_distribution": "q0",
            "inference_units": "independent_declared_matches",
            "within_match_dependence": "unrestricted_through_scalar_aggregate_variance",
            "cross_match_dependence": "independent",
            "nuisance_uncertainty": "conditioned_away",
            "cells": list(defaults["cells"]),
            "replications_per_cell": replications,
            "settings": defaults["settings"],
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
    if manifest.get("schema") != MANIFEST_SCHEMA or manifest.get("status") != "FROZEN_DEVELOPMENT_TASKS" or manifest.get("profile") not in PROFILE_DEFAULTS:
        raise CampaignError("manifest schema, status, or profile mismatch")
    tasks = manifest.get("tasks")
    contract = manifest.get("scientific_contract")
    if not isinstance(tasks, list) or not isinstance(contract, dict) or manifest.get("task_count") != len(tasks):
        raise CampaignError("manifest task inventory is malformed")
    cells = contract.get("cells")
    replications = contract.get("replications_per_cell")
    if not isinstance(cells, list) or not isinstance(replications, int) or replications <= 0:
        raise CampaignError("manifest scientific contract is malformed")
    expected_task_id = 1
    coverage: dict[str, set[int]] = {cell["cell"]: set() for cell in cells}
    for task in tasks:
        required = {"task_id", "cell", "k", "replication_start", "replications", "output", "receipt"}
        if set(task) != required or task["task_id"] != expected_task_id or task["cell"] not in coverage:
            raise CampaignError("manifest task metadata is malformed")
        expected_task_id += 1
        start = task["replication_start"]
        count = task["replications"]
        if not isinstance(start, int) or not isinstance(count, int) or start < 0 or count <= 0:
            raise CampaignError("manifest task range is malformed")
        values = set(range(start, start + count))
        if coverage[task["cell"]] & values:
            raise CampaignError("manifest has overlapping replication ranges")
        coverage[task["cell"]].update(values)
    expected = set(range(replications))
    if any(values != expected for values in coverage.values()):
        raise CampaignError("manifest has partial replication coverage")
    if manifest.get("expected_row_count") != len(cells) * replications * len(TARGETS):
        raise CampaignError("manifest expected row count is inconsistent")


def _read_manifest(path: Path) -> dict[str, Any]:
    try:
        manifest = json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as error:
        raise CampaignError(f"cannot read campaign manifest: {error}") from error
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
    return matches[0]


def _validate_row(row: dict[str, Any], task: dict[str, Any], spec: dict[str, Any]) -> tuple[str, int, int, str]:
    key = (row.get("cell"), row.get("k"), row.get("replication"), row.get("target"))
    if row.get("schema") != ROW_SCHEMA or row.get("cell") != task["cell"] or row.get("k") != task["k"] or row.get("target") not in TARGETS:
        raise CampaignError(f"{key}: row identity mismatch")
    replication = row.get("replication")
    if not isinstance(replication, int) or not task["replication_start"] <= replication < task["replication_start"] + task["replications"]:
        raise CampaignError(f"{key}: replication outside task range")
    if row.get("semantic_seed") != _semantic_seed(task["cell"], task["k"], replication):
        raise CampaignError(f"{key}: semantic seed mismatch")
    for field in ("gate", "match_mass", "within_match_dependence", "variance_dgp", "variance_model", "target_shape", "signal", "controls", "solver", "coverage_eligible"):
        if row.get(field) != spec[field]:
            raise CampaignError(f"{key}: metadata mismatch for {field}")
    status = row.get("status")
    if status == "success":
        for field in NUMERIC_FIELDS:
            _finite(row.get(field), f"{key} {field}")
        for field in ("covered", "lower_miss", "upper_miss", "nuisance_uncertainty_conditioned_away"):
            if not isinstance(row.get(field), bool):
                raise CampaignError(f"{key}: malformed {field}")
        if sum(bool(row[field]) for field in ("covered", "lower_miss", "upper_miss")) != 1:
            raise CampaignError(f"{key}: incoherent interval outcome")
        for field in ("independent_matches", "minimum_active_terms", "minimum_training_matches", "variance_floor_count"):
            if not isinstance(row.get(field), int) or row[field] < 0:
                raise CampaignError(f"{key}: malformed {field}")
        if row["independent_matches"] != task["k"] ** 2 or row["estimated_variance"] <= 0.0 or row["estimated_sd"] <= 0.0 or row["smallest_maker_denominator"] <= 0.0 or not row["nuisance_uncertainty_conditioned_away"]:
            raise CampaignError(f"{key}: invalid grouped q0 numerical contract")
        if row["maximum_complete_residual"] > row["full_residual_tolerance"] * (1.0 + 1e-8):
            raise CampaignError(f"{key}: solver residual exceeds receipt tolerance")
        if row["minimum_active_terms"] <= 0 or row["minimum_training_matches"] <= 0:
            raise CampaignError(f"{key}: structured support diagnostics are empty")
        for field in ("largest_match_mass_share", "largest_match_leverage", "maximum_influence_share", "leading_share", "remainder_share", "maximum_mode_weight", "variance_floor_share", "boundary_share"):
            if not 0.0 <= row[field] <= 1.0:
                raise CampaignError(f"{key}: invalid share {field}")
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
    command = [
        str(binary), "q0-matrix-v1", task["cell"], str(task["k"]), str(task["replication_start"]), str(task["replications"]),
        *_settings_arguments(manifest["scientific_contract"]["settings"]),
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
        key = _validate_row(row, task, spec)
        if key in seen:
            raise CampaignError(f"task {task_id} emitted duplicate key {key}")
        seen.add(key)
        rows.append((key, row))
    expected = {(task["cell"], task["k"], replication, target) for replication in range(task["replication_start"], task["replication_start"] + task["replications"]) for target in TARGETS}
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


def _read_task_rows(path: Path, task: dict[str, Any], spec: dict[str, Any]) -> tuple[list[dict[str, Any]], str]:
    payload = path.read_bytes()
    rows = []
    seen = set()
    for line in payload.splitlines():
        try:
            row = json.loads(line)
        except json.JSONDecodeError as error:
            raise CampaignError(f"{path}: malformed JSON") from error
        key = _validate_row(row, task, spec)
        if key in seen:
            raise CampaignError(f"{path}: duplicate key {key}")
        seen.add(key)
        rows.append(row)
    expected = {(task["cell"], task["k"], replication, target) for replication in range(task["replication_start"], task["replication_start"] + task["replications"]) for target in TARGETS}
    if seen != expected:
        raise CampaignError(f"{path}: partial task inventory")
    return rows, _sha256(payload)


def _mean(rows: Sequence[dict[str, Any]], field: str) -> float | None:
    return statistics.fmean(row[field] for row in rows) if rows else None


def _summary(rows: Sequence[dict[str, Any]], attempts: int) -> dict[str, Any]:
    successful = [row for row in rows if row.get("status") == "success"]
    errors = [row["point_error"] for row in successful]
    empirical_sd = statistics.stdev(errors) if len(errors) >= 2 else None
    mean_se = _mean(successful, "estimated_sd")
    coverage = _mean(successful, "covered")
    output = {
        "attempts": attempts,
        "successes": len(successful),
        "success_rate": len(successful) / attempts,
        "bias": statistics.fmean(errors) if errors else None,
        "bias_mcse": empirical_sd / math.sqrt(len(successful)) if empirical_sd is not None else None,
        "coverage": coverage,
        "coverage_mcse": math.sqrt(coverage * (1.0 - coverage) / len(successful)) if coverage is not None else None,
        "lower_miss": _mean(successful, "lower_miss"),
        "upper_miss": _mean(successful, "upper_miss"),
        "empirical_sd": empirical_sd,
        "mean_se": mean_se,
        "se_ratio": empirical_sd / mean_se if empirical_sd is not None and mean_se is not None and mean_se > 0 else None,
        "failure_counts": {},
    }
    for field in NUMERIC_FIELDS[5:]:
        output[f"mean_{field}"] = _mean(successful, field)
    for row in rows:
        if row.get("status") != "success":
            label = f"{row['status']}:{row.get('error_code', 'NONE')}:{row.get('error_phase', 'none')}"
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
    if profile != "development":
        return [], {"visibly_misspecified": None, "rows": []}
    by_key = {(row["cell"], row["target"]): row for row in summaries}
    failures = []
    for row in summaries:
        label = f"{row['cell']}/{row['target']}"
        if row["gate"] == "correct":
            failures.extend(_gate_correct(label, row))
        elif row["gate"] in {"solver_diagnostic", "spectral_diagnostic", "conditioning_diagnostic"} and row["success_rate"] < THRESHOLDS["correct_success_rate"]:
            failures.append(f"{label}: diagnostic success rate")
    for target in TARGETS:
        row = by_key[("diffuse_mild_omitted", target)]
        label = f"diffuse_mild_omitted/{target}"
        if row["success_rate"] < THRESHOLDS["mild_success_rate"]:
            failures.append(f"{label}: mild success rate")
        if row["coverage"] is None or row["coverage"] < THRESHOLDS["mild_coverage_floor"]:
            failures.append(f"{label}: mild coverage floor")
        else:
            base = by_key[("diffuse_unequal_independent", target)]
            if base["coverage"] is None or base["coverage_mcse"] is None or row["coverage_mcse"] is None:
                failures.append(f"{label}: mild comparison unavailable")
            else:
                allowance = THRESHOLDS["mild_coverage_drop"] + 3.0 * math.hypot(base["coverage_mcse"], row["coverage_mcse"])
                if base["coverage"] - row["coverage"] > allowance:
                    failures.append(f"{label}: mild coverage degradation")
    severe_rows = []
    for target in TARGETS:
        row = by_key[("diffuse_severe_omitted", target)]
        label = f"diffuse_severe_omitted/{target}"
        if row["success_rate"] < THRESHOLDS["severe_success_rate"]:
            failures.append(f"{label}: severe success rate")
        severe_rows.append({"row": label, "coverage": row["coverage"], "se_ratio": row["se_ratio"]})
    visible = any(row["coverage"] is not None and row["se_ratio"] is not None and (row["coverage"] < 0.88 or row["coverage"] > 0.99 or row["se_ratio"] < 0.78 or row["se_ratio"] > 1.22) for row in severe_rows)
    if not visible:
        failures.append("severe omitted-driver cells do not expose the structured-model limitation")
    return failures, {"visibly_misspecified": visible, "rows": severe_rows}


def _summaries_csv(summaries: Sequence[dict[str, Any]]) -> bytes:
    stream = io.StringIO(newline="")
    writer = csv.DictWriter(stream, fieldnames=list(summaries[0]), lineterminator="\n")
    writer.writeheader()
    for row in summaries:
        value = dict(row)
        value["failure_counts"] = json.dumps(value["failure_counts"], sort_keys=True, separators=(",", ":"))
        writer.writerow(value)
    return stream.getvalue().encode()


def aggregate(manifest_path: Path, task_dir: Path, output_dir: Path, build_receipt_path: Path | None = None) -> dict[str, Any]:
    manifest = _read_manifest(manifest_path)
    manifest_hash = _sha256(manifest_path.read_bytes())
    all_rows = []
    task_hashes = {}
    errors = []
    binary_hashes = set()
    for task in manifest["tasks"]:
        try:
            spec = _cell_spec(manifest, task)
            rows, digest = _read_task_rows(task_dir / task["output"], task, spec)
            receipt = json.loads((task_dir / task["receipt"]).read_text())
            if receipt.get("schema") != TASK_RECEIPT_SCHEMA or receipt.get("status") != "success" or receipt.get("task") != task or receipt.get("manifest_sha256") != manifest_hash or receipt.get("source") != manifest["source"] or receipt.get("registration") != manifest["registration"] or receipt.get("preflight") != manifest["preflight"] or receipt.get("row_count") != len(rows) or receipt.get("output_sha256") != digest:
                raise CampaignError("receipt does not bind task output")
            binary_hashes.add(receipt.get("binary_sha256"))
        except (OSError, json.JSONDecodeError, CampaignError) as error:
            errors.append(f"task {task['task_id']}: {error}")
            continue
        all_rows.extend(rows)
        task_hashes[task["output"]] = digest
    if errors:
        raise CampaignError("; ".join(errors))
    if len(binary_hashes) != 1 or not _is_sha256(next(iter(binary_hashes))):
        raise CampaignError("task receipts do not share one valid binary")
    binary_hash = next(iter(binary_hashes))
    build_receipt_hash = None
    if build_receipt_path is not None:
        try:
            build_payload = build_receipt_path.read_bytes()
            build = json.loads(build_payload)
        except (OSError, json.JSONDecodeError) as error:
            raise CampaignError(f"cannot read build receipt: {error}") from error
        if build.get("schema") != BUILD_RECEIPT_SCHEMA or build.get("status") != "success" or build.get("binary_sha256") != binary_hash or build.get("source_commit") != manifest["source"]["commit"]:
            raise CampaignError("build receipt does not bind manifest source and task binary")
        build_receipt_hash = _sha256(build_payload)
    elif binary_hash != manifest["preflight"]["binary_sha256"]:
        raise CampaignError("local tasks do not use the preflight binary")
    all_rows.sort(key=lambda row: (row["cell"], row["k"], row["replication"], row["target"]))
    if len(all_rows) != manifest["expected_row_count"]:
        raise CampaignError("aggregate row count does not match manifest")
    grouped: dict[tuple[str, str], list[dict[str, Any]]] = {}
    for row in all_rows:
        grouped.setdefault((row["cell"], row["target"]), []).append(row)
    summaries = []
    attempts = manifest["scientific_contract"]["replications_per_cell"]
    for spec in manifest["scientific_contract"]["cells"]:
        for target in TARGETS:
            summary = _summary(grouped.get((spec["cell"], target), []), attempts)
            summary.update(spec)
            summary["target"] = target
            summaries.append(summary)
    scientific_failures, severe = _scientific_failures(summaries, manifest["profile"])
    status = "PASS" if manifest["profile"] == "development" and not scientific_failures else "FAIL" if manifest["profile"] == "development" else "COMPLETE"
    aggregate_payload = b"".join(json.dumps(row, sort_keys=True, separators=(",", ":")).encode() + b"\n" for row in all_rows)
    summaries_payload = _summaries_csv(summaries)
    _write_new(output_dir / "aggregate.jsonl", aggregate_payload)
    _write_new(output_dir / "summaries.csv", summaries_payload)
    receipt = {
        "schema": SUMMARY_SCHEMA,
        "status": status,
        "decision": "grouped_match_q0_development_pass" if status == "PASS" else "grouped_match_q0_development_failed" if status == "FAIL" else "pipeline_complete_non_evidentiary",
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
            "Internal suggestive match-cluster inference conditional on the full-sample fixed nuisance-control offset; "
            "declared matches are independent, dependence within each match is unrestricted through its scalar aggregate variance, "
            "and the aggregate variances follow a named structured model. This is neither joint-nuisance nor unrestricted-KSS inference."
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

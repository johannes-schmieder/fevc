#!/usr/bin/env python3
"""Fresh corrected-source observation confirmation with immutable V5 gates."""

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


MANIFEST_SCHEMA = "fevc-rc-observation-inference-manifest-v1"
ROW_SCHEMA = "fevc-rc-observation-inference-row-v1"
TASK_RECEIPT_SCHEMA = "fevc-rc-observation-inference-task-v1"
SUMMARY_SCHEMA = "fevc-rc-observation-inference-summary-v1"
REGISTRATION_SCHEMA = "FEVC_RC_OBSERVATION_INFERENCE_V1"
REGISTRATION_PATH = Path("fevc/docs/rc_observation_inference_v1.json")
TARGETS = ("worker", "firm", "covariance", "total")
FAILURE_STATUSES = {
    "variance_fit_failed",
    "q0_variance_failed",
    "q1_failed",
    "q1_interval_failed",
}
NUMERIC_FIELDS = (
    "point_error",
    "estimated_sd",
    "interval_width",
    "leading_variance",
    "remainder_variance",
    "leading_remainder_covariance",
    "remainder_identity_error",
    "critical",
    "leading_share",
    "remainder_share",
    "floor_share",
    "boundary_share",
    "maximum_boundary_excess",
    "maximum_prediction_leverage",
    "minimum_fitted_rcond",
    "leading_eigenvalue",
    "curvature",
)
THRESHOLDS = {
    "bias_mcse_multiplier": 4.0,
    "coverage_absolute_tolerance": 0.015,
    "coverage_mcse_multiplier": 3.0,
    "correct_success_rate": 0.99,
    "q1_nonprimary_success_rate": 0.95,
    "correct_se_ratio_lower": 0.90,
    "correct_se_ratio_upper": 1.10,
    "mild_success_rate": 0.98,
    "mild_coverage_floor": 0.88,
    "mild_coverage_drop": 0.04,
    "severe_success_rate": 0.90,
}
V5_CONFIRMATION_SEED = 0xC247_F908_3AE1_6D5B
PIPELINE_SEED = 0x9816_AC5D_72B0_F3E4


def _cell(
    name: str,
    k: int,
    controls: bool,
    dominant: bool,
    variance_model: str,
    variance_dgp: str,
    error_dgp: str,
    reference: str,
    beta: str,
    gate: str,
) -> dict[str, Any]:
    return {
        "cell": name,
        "k": k,
        "controls": controls,
        "dominant": dominant,
        "variance_model": variance_model,
        "variance_dgp": variance_dgp,
        "error_dgp": error_dgp,
        "reference": reference,
        "beta": beta,
        "gate": gate,
    }


BASE = (
    _cell("diffuse_common", 12, False, False, "structured_common", "Common", "Gaussian", "Q0", "nonzero", "correct"),
    _cell("dominant_common", 12, False, True, "structured_common", "Common", "Gaussian", "Q1", "nonzero", "correct"),
    _cell("diffuse_common", 16, False, False, "structured_common", "Common", "Gaussian", "Q0", "nonzero", "correct"),
    _cell("dominant_common", 16, False, True, "structured_common", "Common", "Gaussian", "Q1", "nonzero", "correct"),
)
EXTRA = (
    _cell("diffuse_homoskedastic", 16, False, False, "structured_common", "Homoskedastic", "Gaussian", "Q0", "nonzero", "correct"),
    _cell("diffuse_leverage", 16, False, False, "structured_leverage", "Leverage", "Gaussian", "Q0", "nonzero", "correct"),
    _cell("dominant_leverage", 16, False, True, "structured_leverage", "Leverage", "Gaussian", "Q1", "nonzero", "correct"),
    _cell("diffuse_common_t8", 16, False, False, "structured_common", "Common", "StudentT8", "Q0", "nonzero", "correct"),
    _cell("dominant_common_t8", 16, False, True, "structured_common", "Common", "StudentT8", "Q1", "nonzero", "correct"),
    _cell("diffuse_common_controls", 16, True, False, "structured_common", "Common", "Gaussian", "Q0", "nonzero", "correct"),
    _cell("dominant_common_controls", 16, True, True, "structured_common", "Common", "Gaussian", "Q1", "nonzero", "correct"),
    _cell("diffuse_mild_functional", 16, False, False, "structured_common", "MildFunctional", "Gaussian", "Q0", "nonzero", "mild"),
    _cell("dominant_mild_functional", 16, False, True, "structured_common", "MildFunctional", "Gaussian", "Q1", "nonzero", "mild"),
    _cell("diffuse_mild_omitted", 16, False, False, "structured_common", "MildOmitted", "Gaussian", "Q0", "nonzero", "mild"),
    _cell("dominant_mild_omitted", 16, False, True, "structured_common", "MildOmitted", "Gaussian", "Q1", "nonzero", "mild"),
    _cell("diffuse_severe_omitted", 16, False, False, "structured_common", "SevereOmitted", "Gaussian", "Q0", "nonzero", "descriptive"),
    _cell("dominant_severe_omitted", 16, False, True, "structured_common", "SevereOmitted", "Gaussian", "Q1", "nonzero", "descriptive"),
    _cell("diffuse_common_null", 16, False, False, "structured_common", "Common", "Gaussian", "Q0", "zero", "diagnostic"),
    _cell("dominant_common_null_q0", 16, False, True, "structured_common", "Common", "Gaussian", "Q0", "zero", "diagnostic"),
    _cell("dominant_common_null", 16, False, True, "structured_common", "Common", "Gaussian", "Q1", "zero", "diagnostic"),
)
ALL_CELLS = BASE + EXTRA
SMOKE_KEYS = {
    ("diffuse_common", 16),
    ("dominant_common", 16),
    ("dominant_leverage", 16),
    ("dominant_common_t8", 16),
    ("dominant_common_controls", 16),
    ("diffuse_mild_omitted", 16),
    ("dominant_severe_omitted", 16),
}
PROFILE_DEFAULTS = {
    "tiny": {"cells": ALL_CELLS, "replications": 2, "shard_size": 2},
    "smoke": {
        "cells": tuple(cell for cell in ALL_CELLS if (cell["cell"], cell["k"]) == ("dominant_common_controls", 16)),
        "replications": 4,
        "shard_size": 4,
    },
    "confirmation": {"cells": ALL_CELLS, "replications": 2_500, "shard_size": 500},
}


class ConfirmationError(RuntimeError):
    """The frozen confirmation contract or evidence is invalid."""


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
    for byte in value.encode("utf-8"):
        state = ((state ^ byte) * 0x1000_0000_01B3) & ((1 << 64) - 1)
    return state


def _semantic_seed(cell: str, k: int, replication: int, master: int = V5_CONFIRMATION_SEED) -> int:
    return _splitmix64(
        master
        ^ _splitmix64(_label_hash(cell))
        ^ _splitmix64(k)
        ^ _splitmix64(replication)
    )


def _write_new(path: Path, payload: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        raise ConfirmationError(f"refusing to replace existing output {path}")
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
    commit = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=root, check=True, capture_output=True, text=True
    ).stdout.strip()
    status = subprocess.run(
        ["git", "status", "--porcelain=v1"], cwd=root, check=True, capture_output=True, text=True
    ).stdout
    diff = subprocess.run(
        ["git", "diff", "--binary", "HEAD"], cwd=root, check=True, capture_output=True
    ).stdout
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
        relative = encoded.decode("utf-8")
        path = root / relative
        if not path.is_file() or path.is_symlink():
            raise ConfirmationError(f"source manifest entry is not a regular file: {relative}")
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
    try:
        payload = path.read_bytes()
        registration = json.loads(payload)
    except (OSError, json.JSONDecodeError) as error:
        raise ConfirmationError(f"cannot read V5 registration: {error}") from error
    if registration.get("schema") != REGISTRATION_SCHEMA or registration.get("status") != "CONFIRMATION_REGISTERED":
        raise ConfirmationError("V5 registration schema or status mismatch")
    frozen = registration.get("source_binding", {}).get("frozen_file_sha256")
    if not isinstance(frozen, dict) or not frozen:
        raise ConfirmationError("V5 registration has no frozen file inventory")
    for relative, expected in frozen.items():
        candidate = root / relative
        if not candidate.is_file() or _sha256(candidate.read_bytes()) != expected:
            raise ConfirmationError(f"V5 frozen file hash mismatch: {relative}")
    return {
        "path": REGISTRATION_PATH.as_posix(),
        "schema": REGISTRATION_SCHEMA,
        "sha256": _sha256(payload),
    }



def _validate_preflight(rows):
    expected = {(cell["cell"], cell["k"], target) for cell in ALL_CELLS for target in TARGETS}
    by_key = {}
    for row in rows:
        key = (row.get("cell"), row.get("k"), row.get("target"))
        if key not in expected or key in by_key or row.get("schema") != "fevc-rc-observation-preflight-v1":
            raise ConfirmationError("preflight schema or inventory mismatch")
        if (type(row.get("observations")) is not int or type(row.get("parameters")) is not int
                or not 0 < row["parameters"] < row["observations"]
                or row.get("variance_fold_seed") != 8675309):
            raise ConfirmationError("preflight dimensions or fixed fold seed mismatch")
        for field in ("maker_minimum", "variance_minimum", "mean_leading_share", "mean_remainder_share"):
            if not isinstance(row.get(field), (int, float)) or not math.isfinite(row[field]):
                raise ConfirmationError("nonfinite preflight geometry")
        if row["maker_minimum"] <= 0 or row["variance_minimum"] <= 0:
            raise ConfirmationError("nonpositive preflight support")
        if not all(0 <= row[field] <= 1 for field in ("mean_leading_share", "mean_remainder_share")):
            raise ConfirmationError("invalid preflight spectral share")
        by_key[key] = row
    if set(by_key) != expected:
        raise ConfirmationError("partial preflight inventory")
    failures = _spectral_failures(by_key)
    if failures:
        raise ConfirmationError("preflight design regime: " + "; ".join(failures))


def run_preflight(root: Path, output_dir: Path, binary: Path) -> dict:
    source = _git_identity(root)
    registration = _registration_identity(root)
    completed = subprocess.run([str(binary), "design-preflight-rc-v1"], capture_output=True, text=True)
    if completed.returncode:
        raise ConfirmationError(f"preflight exited {completed.returncode}: {completed.stderr[-2000:]}")
    rows = [json.loads(line) for line in completed.stdout.splitlines()]
    _validate_preflight(rows)
    rows.sort(key=lambda row: (row["cell"], row["k"], row["target"]))
    payload = b"".join(json.dumps(row, sort_keys=True).encode() + b"\n" for row in rows)
    receipt = {
        "schema": "fevc-rc-observation-preflight-receipt-v1", "status": "PASS",
        "source": source, "registration": registration, "row_count": len(rows),
        "binary_sha256": _binary_hash(binary), "output_sha256": _sha256(payload),
    }
    _write_new(output_dir / "preflight.jsonl", payload)
    _write_new(output_dir / "receipt.json", _json_bytes(receipt))
    return receipt


def _read_preflight(path: Path, source: dict, registration: dict) -> dict:
    receipt_bytes = path.read_bytes()
    receipt = json.loads(receipt_bytes)
    payload = path.with_name("preflight.jsonl").read_bytes()
    if (receipt.get("schema") != "fevc-rc-observation-preflight-receipt-v1"
            or receipt.get("status") != "PASS" or receipt.get("source") != source
            or receipt.get("registration") != registration or receipt.get("row_count") != 80
            or receipt.get("output_sha256") != _sha256(payload)):
        raise ConfirmationError("preflight source, registration, or payload mismatch")
    _validate_preflight([json.loads(line) for line in payload.splitlines()])
    return {"sha256": _sha256(receipt_bytes), "binary_sha256": receipt["binary_sha256"],
            "output_sha256": receipt["output_sha256"]}


def create_manifest(root: Path, profile: str, output: Path, shard_size: int | None, preflight_receipt: Path) -> dict[str, Any]:
    defaults = PROFILE_DEFAULTS[profile]
    shard = shard_size or defaults["shard_size"]
    replications = defaults["replications"]
    if shard <= 0 or replications % shard:
        raise ConfirmationError("shard size must divide the replication count")
    source = _git_identity(root)
    registration = _registration_identity(root)
    if profile != "tiny" and source["dirty"]:
        raise ConfirmationError("confirmation manifests require a clean source tree")
    preflight = _read_preflight(preflight_receipt, source, registration)
    tasks = []
    for cell in defaults["cells"]:
        for start in range(0, replications, shard):
            task_id = len(tasks) + 1
            tasks.append(
                {
                    "task_id": task_id,
                    "cell": cell["cell"],
                    "k": cell["k"],
                    "replication_start": start,
                    "replications": shard,
                    "output": f"task-{task_id:05d}.jsonl",
                    "receipt": f"task-{task_id:05d}.receipt.json",
                }
            )
    expected_rows = 4 * sum(task["replications"] for task in tasks)
    manifest = {
        "schema": MANIFEST_SCHEMA,
        "profile": profile,
        "source": source,
        "registration": registration,
        "preflight": preflight,
        "scientific_contract": {
            "deletion": "observation",
            "weights": "unit_frequency",
            "population": "movers_only",
            "independence": "independent_observations",
            "variance_models": ["structured_common", "structured_leverage"],
            "strict_unrestricted_kss": False,
            "cells": list(defaults["cells"]),
            "targets": list(TARGETS),
            "replications_per_cell": replications,
            "outcome_seed_hex": hex(V5_CONFIRMATION_SEED if profile == "confirmation" else PIPELINE_SEED),
            "outcome_seed_keys": ["cell", "dimension", "replication"],
            "variance_fold_seed": 8_675_309,
            "q1_curvature": "2_abs_lambda_times_leading_variance_over_conditional_remainder_sd",
            "q1_recenter": "raw_observation_leaveout_mode_variance_product",
            "q1_critical": "deterministic_32_point_gauss_legendre_inversion",
            "q1_claim": "uniform_at_least_nominal_under_the_registered_approximation",
            "thresholds": THRESHOLDS,
        },
        "expected_output_inventory": ["aggregate.jsonl", "summaries.csv", "receipt.json"],
        "task_count": len(tasks),
        "expected_row_count": expected_rows,
        "tasks": tasks,
    }
    _write_new(output, _json_bytes(manifest))
    return manifest


def _read_manifest(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ConfirmationError(f"cannot read manifest: {error}") from error
    tasks = value.get("tasks")
    if value.get("schema") != MANIFEST_SCHEMA or not isinstance(tasks, list):
        raise ConfirmationError("manifest schema or task inventory mismatch")
    if value.get("task_count") != len(tasks):
        raise ConfirmationError("manifest task count is inconsistent")
    if [task.get("task_id") for task in tasks] != list(range(1, len(tasks) + 1)):
        raise ConfirmationError("manifest task identifiers are not canonical")
    outputs = [task.get("output") for task in tasks]
    receipts = [task.get("receipt") for task in tasks]
    if len(set(outputs)) != len(outputs) or len(set(receipts)) != len(receipts):
        raise ConfirmationError("manifest task paths are not unique")
    contract = value.get("scientific_contract", {})
    cells = contract.get("cells")
    replications = contract.get("replications_per_cell")
    if not isinstance(cells, list) or not isinstance(replications, int) or replications <= 0:
        raise ConfirmationError("manifest scientific inventory is malformed")
    expected_ranges: dict[tuple[str, int], list[tuple[int, int]]] = {
        (cell["cell"], cell["k"]): [] for cell in cells
    }
    for task in tasks:
        key = (task.get("cell"), task.get("k"))
        start = task.get("replication_start")
        count = task.get("replications")
        if key not in expected_ranges or not isinstance(start, int) or not isinstance(count, int) or start < 0 or count <= 0:
            raise ConfirmationError("manifest task range is malformed")
        expected_ranges[key].append((start, start + count))
    for key, ranges in expected_ranges.items():
        ranges.sort()
        if not ranges or ranges[0][0] != 0 or ranges[-1][1] != replications:
            raise ConfirmationError(f"manifest has a partial replication inventory for {key}")
        if any(left[1] != right[0] for left, right in zip(ranges, ranges[1:])):
            raise ConfirmationError(f"manifest has overlapping or missing ranges for {key}")
    if value.get("expected_row_count") != 4 * sum(task["replications"] for task in tasks):
        raise ConfirmationError("manifest expected row count is inconsistent")
    if value.get("profile") == "confirmation" and value.get("source", {}).get("dirty"):
        raise ConfirmationError("confirmation manifest records a dirty source")
    profile = value.get("profile")
    if profile not in PROFILE_DEFAULTS:
        raise ConfirmationError("unknown registered profile")
    defaults = PROFILE_DEFAULTS[profile]
    if (contract.get("cells") != list(defaults["cells"])
            or contract.get("replications_per_cell") != defaults["replications"]
            or contract.get("thresholds") != THRESHOLDS
            or contract.get("outcome_seed_hex") != hex(V5_CONFIRMATION_SEED if profile == "confirmation" else PIPELINE_SEED)
            or contract.get("variance_fold_seed") != 8675309
            or contract.get("deletion") != "observation"
            or contract.get("population") != "movers_only"
            or contract.get("weights") != "unit_frequency"
            or contract.get("targets") != list(TARGETS)):
        raise ConfirmationError("scientific contract differs from registered profile")
    for task in tasks:
        if (task["output"] != f"task-{task['task_id']:05d}.jsonl"
                or task["receipt"] != f"task-{task['task_id']:05d}.receipt.json"):
            raise ConfirmationError("noncanonical task output paths")
    preflight = value.get("preflight")
    if not isinstance(preflight, dict) or any(
            not isinstance(preflight.get(key), str) or len(preflight[key]) != 64
            or any(char not in "0123456789abcdef" for char in preflight[key])
            for key in ("sha256", "binary_sha256", "output_sha256")):
        raise ConfirmationError("missing or malformed preflight identity")
    return value


def _task(manifest: dict[str, Any], task_id: int) -> dict[str, Any]:
    if task_id < 1 or task_id > manifest["task_count"]:
        raise ConfirmationError(f"task id {task_id} is outside the manifest")
    return manifest["tasks"][task_id - 1]


def _cell_spec(manifest: dict[str, Any], task: dict[str, Any]) -> dict[str, Any]:
    matches = [
        cell
        for cell in manifest["scientific_contract"]["cells"]
        if cell["cell"] == task["cell"] and cell["k"] == task["k"]
    ]
    if len(matches) != 1:
        raise ConfirmationError("task does not identify one registered cell")
    return {**matches[0], "master_seed": V5_CONFIRMATION_SEED if manifest["profile"] == "confirmation" else PIPELINE_SEED}


def _validate_row(row: Any, task: dict[str, Any], spec: dict[str, Any]) -> tuple[str, int, int, str]:
    if not isinstance(row, dict) or row.get("schema") != ROW_SCHEMA:
        raise ConfirmationError("task emitted a malformed row")
    key = (row.get("cell"), row.get("k"), row.get("replication"), row.get("target"))
    if key[0] != task["cell"] or key[1] != task["k"] or key[3] not in TARGETS:
        raise ConfirmationError(f"task emitted an unknown semantic key {key}")
    if not isinstance(key[2], int) or not (
        task["replication_start"] <= key[2] < task["replication_start"] + task["replications"]
    ):
        raise ConfirmationError(f"task emitted an out-of-range semantic key {key}")
    status = row.get("status")
    if row.get("semantic_seed") != _semantic_seed(task["cell"], task["k"], key[2], spec["master_seed"]):
        raise ConfirmationError(f"{key}: semantic seed mismatch")
    if status == "success":
        if type(row.get("outer_fold_fingerprint")) is not int or not 0 <= row["outer_fold_fingerprint"] < 2**64:
            raise ConfirmationError(f"{key}: missing fixed-fold fingerprint")
        for name, expected in spec.items():
            if name in {"cell", "k", "master_seed"}:
                continue
            if row.get(name) != expected:
                raise ConfirmationError(f"{key}: metadata mismatch for {name}")
        for field in NUMERIC_FIELDS:
            value = row.get(field)
            if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value):
                raise ConfirmationError(f"{key}: nonfinite or missing {field}")
        if row["estimated_sd"] <= 0.0 or row["interval_width"] < 0.0:
            raise ConfirmationError(f"{key}: invalid interval scale")
        if row["reference"] == "Q1":
            leading, remainder, cross = (row[field] for field in (
                "leading_variance", "remainder_variance", "leading_remainder_covariance"))
            if leading <= 0 or remainder <= 0 or 1 - cross * cross / (leading * remainder) <= 1e-12:
                raise ConfirmationError(f"{key}: invalid standardized q1 covariance")
            expected = 2 * abs(row["leading_eigenvalue"]) * leading / math.sqrt(remainder - cross * cross / leading)
            if not math.isclose(row["curvature"], expected, rel_tol=1e-9, abs_tol=1e-12):
                raise ConfirmationError(f"{key}: corrected curvature identity")
        for field in ("covered", "lower_miss", "upper_miss"):
            if not isinstance(row.get(field), bool):
                raise ConfirmationError(f"{key}: malformed {field}")
        if sum(bool(row[field]) for field in ("covered", "lower_miss", "upper_miss")) != 1:
            raise ConfirmationError(f"{key}: incoherent interval outcome")
    elif status not in FAILURE_STATUSES:
        raise ConfirmationError(f"{key}: unknown failure status {status!r}")
    return key


def _binary_hash(path: Path) -> str:
    if not path.is_file():
        raise ConfirmationError(f"qualification binary does not exist: {path}")
    return _sha256(path.read_bytes())


def run_task(manifest_path: Path, task_id: int, output_dir: Path, binary: Path) -> None:
    manifest = _read_manifest(manifest_path)
    task = _task(manifest, task_id)
    spec = _cell_spec(manifest, task)
    command = [
        str(binary),
        "confirmation-matrix-v5" if manifest["profile"] == "confirmation" else "pipeline-matrix-rc-v1",
        task["cell"],
        str(task["k"]),
        str(task["replication_start"]),
        str(task["replications"]),
    ]
    completed = subprocess.run(command, capture_output=True, text=True)
    if completed.returncode:
        raise ConfirmationError(
            f"task {task_id} exited {completed.returncode}: {completed.stderr[-2000:]}"
        )
    rows = []
    seen = set()
    for line in completed.stdout.splitlines():
        if not line.strip():
            continue
        try:
            row = json.loads(line)
        except json.JSONDecodeError as error:
            raise ConfirmationError(f"task {task_id} emitted non-JSON output") from error
        key = _validate_row(row, task, spec)
        if key in seen:
            raise ConfirmationError(f"task {task_id} emitted duplicate key {key}")
        seen.add(key)
        rows.append((key, row))
    expected = {
        (task["cell"], task["k"], replication, target)
        for replication in range(task["replication_start"], task["replication_start"] + task["replications"])
        for target in TARGETS
    }
    if seen != expected:
        raise ConfirmationError(
            f"task {task_id} inventory mismatch: missing={sorted(expected-seen)[:4]} extra={sorted(seen-expected)[:4]}"
        )
    rows.sort(key=lambda item: item[0])
    payload = b"".join(
        json.dumps(row, sort_keys=True, separators=(",", ":")).encode() + b"\n"
        for _, row in rows
    )
    _write_new(output_dir / task["output"], payload)
    receipt = {
        "schema": TASK_RECEIPT_SCHEMA,
        "status": "success",
        "runtime": {"python": sys.version.split()[0], "executable": sys.executable},
        "task": task,
        "manifest_sha256": _sha256(manifest_path.read_bytes()),
        "source": manifest["source"],
        "registration": manifest["registration"],
        "command": command,
        "binary_sha256": _binary_hash(binary),
        "row_count": len(rows),
        "output_sha256": _sha256(payload),
        "stderr": completed.stderr.splitlines(),
    }
    _write_new(output_dir / task["receipt"], _json_bytes(receipt))


def _sample_sd(values: Sequence[float]) -> float | None:
    return statistics.stdev(values) if len(values) >= 2 else None


def _mean(rows: Sequence[dict[str, Any]], field: str) -> float | None:
    return statistics.fmean(row[field] for row in rows) if rows else None


def _summary(rows: Sequence[dict[str, Any]], attempts: int) -> dict[str, Any]:
    successful = [row for row in rows if row.get("status") == "success"]
    errors = [row["point_error"] for row in successful]
    empirical_sd = _sample_sd(errors)
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
        "se_ratio": empirical_sd / mean_se if empirical_sd is not None and mean_se is not None and mean_se > 0.0 else None,
        "failure_counts": {},
    }
    for field in NUMERIC_FIELDS[2:]:
        output[f"mean_{field}"] = _mean(successful, field)
    for row in rows:
        if row.get("status") != "success":
            status = row["status"]
            output["failure_counts"][status] = output["failure_counts"].get(status, 0) + 1
    return output


def _gate_common(label: str, summary: dict[str, Any], success_rate: float) -> list[str]:
    failures = []
    if summary["success_rate"] < success_rate:
        failures.append(f"{label}: success rate")
    return failures


def _gate_correct(label: str, summary: dict[str, Any]) -> list[str]:
    failures = _gate_common(label, summary, THRESHOLDS["correct_success_rate"])
    required = ("bias", "bias_mcse", "coverage", "coverage_mcse", "se_ratio")
    if summary["successes"] < 2 or any(summary[name] is None for name in required):
        failures.append(f"{label}: insufficient successful replications")
        return failures
    if abs(summary["bias"]) > max(1.0e-12, THRESHOLDS["bias_mcse_multiplier"] * summary["bias_mcse"]):
        failures.append(f"{label}: bias")
    bound = max(
        THRESHOLDS["coverage_absolute_tolerance"],
        THRESHOLDS["coverage_mcse_multiplier"] * summary["coverage_mcse"],
    )
    if abs(summary["coverage"] - 0.95) > bound:
        failures.append(f"{label}: coverage")
    if not THRESHOLDS["correct_se_ratio_lower"] <= summary["se_ratio"] <= THRESHOLDS["correct_se_ratio_upper"]:
        failures.append(f"{label}: standard-error ratio")
    return failures


def _read_task_rows(path: Path, task: dict[str, Any], spec: dict[str, Any]) -> tuple[list[dict[str, Any]], str]:
    payload = path.read_bytes()
    rows = []
    seen = set()
    for line in payload.splitlines():
        try:
            row = json.loads(line)
        except json.JSONDecodeError as error:
            raise ConfirmationError(f"{path}: malformed JSON") from error
        key = _validate_row(row, task, spec)
        if key in seen:
            raise ConfirmationError(f"{path}: duplicate key {key}")
        seen.add(key)
        rows.append(row)
    expected = {
        (task["cell"], task["k"], replication, target)
        for replication in range(task["replication_start"], task["replication_start"] + task["replications"])
        for target in TARGETS
    }
    if seen != expected:
        raise ConfirmationError(f"{path}: partial task inventory")
    return rows, _sha256(payload)


def _spectral_failures(by_key: dict[tuple[str, int, str], dict[str, Any]]) -> list[str]:
    failures = []
    for target in ("worker", "firm"):
        diffuse = by_key[("diffuse_common", 16, target)]
        dominant = by_key[("dominant_common", 16, target)]
        if diffuse["mean_leading_share"] is None or diffuse["mean_leading_share"] >= 0.11:
            failures.append(f"diffuse_common/16/{target}: diffuse spectrum")
        if dominant["mean_leading_share"] is None or dominant["mean_leading_share"] <= 0.75:
            failures.append(f"dominant_common/16/{target}: missing leading mode")
        if dominant["mean_remainder_share"] is None or dominant["mean_remainder_share"] >= 0.15:
            failures.append(f"dominant_common/16/{target}: concentrated q1 remainder")
    covariance = by_key[("dominant_common", 16, "covariance")]
    if covariance["mean_leading_share"] is None or covariance["mean_leading_share"] <= 0.95:
        failures.append("dominant_common/16/covariance: missing q>1 leading concentration")
    if covariance["mean_remainder_share"] is None or covariance["mean_remainder_share"] <= 0.50:
        failures.append("dominant_common/16/covariance: missing q>1 remainder concentration")
    unreliable = by_key[("dominant_common_null_q0", 16, "worker")]
    if unreliable["mean_leading_share"] is None or unreliable["mean_leading_share"] <= 0.75:
        failures.append("dominant_common_null_q0/16/worker: q0 stress not concentrated")
    for target in ("worker", "firm", "total"):
        first = by_key[("diffuse_common", 12, target)]["mean_leading_share"]
        second = by_key[("diffuse_common", 16, target)]["mean_leading_share"]
        if first is None or second is None or second >= first:
            failures.append(f"diffuse_common/{target}: concentration did not decline")
    return failures


def _scientific_failures(
    summaries: list[dict[str, Any]], profile: str
) -> tuple[list[str], dict[str, Any]]:
    if profile != "confirmation":
        return [], {"visibly_misspecified": None, "rows": []}
    by_key = {(row["cell"], row["k"], row["target"]): row for row in summaries}
    failures = _spectral_failures(by_key)
    for row in summaries:
        label = f"{row['cell']}/{row['k']}/{row['target']}"
        if row["gate"] == "correct" and (row["reference"] == "Q0" or row["target"] != "covariance"):
            failures.extend(_gate_correct(label, row))
        if row["gate"] == "correct" and row["reference"] == "Q1" and row["target"] == "covariance":
            failures.extend(_gate_common(label, row, THRESHOLDS["q1_nonprimary_success_rate"]))
    for shape in ("diffuse", "dominant"):
        for suffix in ("mild_functional", "mild_omitted"):
            cell = f"{shape}_{suffix}"
            base = f"{shape}_common"
            for target in TARGETS:
                row = by_key[(cell, 16, target)]
                if row["reference"] == "Q1" and target == "covariance":
                    continue
                label = f"{cell}/16/{target}"
                failures.extend(_gate_common(label, row, THRESHOLDS["mild_success_rate"]))
                if row["coverage"] is None or row["coverage"] < THRESHOLDS["mild_coverage_floor"]:
                    failures.append(f"{label}: mild coverage floor")
                    continue
                comparison = by_key[(base, 16, target)]
                if comparison["coverage"] is None or comparison["coverage_mcse"] is None or row["coverage_mcse"] is None:
                    failures.append(f"{label}: mild comparison unavailable")
                    continue
                allowed = THRESHOLDS["mild_coverage_drop"] + 3.0 * math.hypot(
                    row["coverage_mcse"], comparison["coverage_mcse"]
                )
                if comparison["coverage"] - row["coverage"] > allowed:
                    failures.append(f"{label}: mild coverage degradation")
    severe_rows = []
    for shape in ("diffuse", "dominant"):
        for target in TARGETS:
            row = by_key[(f"{shape}_severe_omitted", 16, target)]
            label = f"{row['cell']}/16/{target}"
            failures.extend(_gate_common(label, row, THRESHOLDS["severe_success_rate"]))
            severe_rows.append(
                {"row": label, "coverage": row["coverage"], "se_ratio": row["se_ratio"]}
            )
    visible = any(
        row["coverage"] is not None
        and row["se_ratio"] is not None
        and (row["coverage"] < 0.90 or row["coverage"] > 0.985 or row["se_ratio"] < 0.85 or row["se_ratio"] > 1.15)
        for row in severe_rows
    )
    if not visible:
        failures.append("severe omitted-driver cells do not expose model limitation")
    return failures, {"visibly_misspecified": visible, "rows": severe_rows}


def _summaries_csv(summaries: Sequence[dict[str, Any]]) -> bytes:
    fieldnames = list(summaries[0])
    stream = io.StringIO(newline="")
    writer = csv.DictWriter(stream, fieldnames=fieldnames, lineterminator="\n")
    writer.writeheader()
    for row in summaries:
        output = dict(row)
        output["failure_counts"] = json.dumps(output["failure_counts"], sort_keys=True, separators=(",", ":"))
        writer.writerow(output)
    return stream.getvalue().encode()


def aggregate(
    manifest_path: Path,
    task_dir: Path,
    output_dir: Path,
    build_receipt_path: Path | None = None,
) -> dict[str, Any]:
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
            receipt = json.loads((task_dir / task["receipt"]).read_text(encoding="utf-8"))
            if (
                receipt.get("schema") != TASK_RECEIPT_SCHEMA
                or receipt.get("status") != "success"
                or receipt.get("task") != task
                or receipt.get("manifest_sha256") != manifest_hash
                or receipt.get("source") != manifest["source"]
                or receipt.get("registration") != manifest["registration"]
                or receipt.get("row_count") != len(rows)
                or receipt.get("output_sha256") != digest
            ):
                raise ConfirmationError("receipt does not bind task output")
            binary = receipt.get("binary_sha256")
            if not isinstance(binary, str) or len(binary) != 64:
                raise ConfirmationError("task receipt has invalid binary hash")
            binary_hashes.add(binary)
        except (OSError, json.JSONDecodeError, ConfirmationError) as error:
            errors.append(f"task {task['task_id']}: {error}")
            continue
        all_rows.extend(rows)
        task_hashes[task["output"]] = digest
    if errors:
        raise ConfirmationError("; ".join(errors))
    if len(binary_hashes) != 1:
        raise ConfirmationError("task receipts do not share one binary")
    binary_hash = next(iter(binary_hashes))
    build_receipt_hash = None
    if build_receipt_path is not None:
        try:
            build_payload = build_receipt_path.read_bytes()
            build = json.loads(build_payload)
        except (OSError, json.JSONDecodeError) as error:
            raise ConfirmationError(f"cannot read build receipt: {error}") from error
        if build.get("schema") != "fevc-rc-observation-inference-scc-build-v1" or build.get("status") != "success" or build.get("binary_sha256") != binary_hash:
            raise ConfirmationError("build receipt does not bind task binary")
        if build.get("source_commit") != manifest["source"]["commit"]:
            raise ConfirmationError("build receipt does not bind manifest source")
        build_receipt_hash = _sha256(build_payload)
    elif binary_hash != manifest["preflight"]["binary_sha256"]:
        raise ConfirmationError("local task binary differs from preflight")
    all_rows.sort(key=lambda row: (row["cell"], row["k"], row["replication"], row["target"]))
    if len(all_rows) != manifest["expected_row_count"]:
        raise ConfirmationError("aggregate row count does not match manifest")
    grouped: dict[tuple[str, int, str], list[dict[str, Any]]] = {}
    fold_fingerprints = {}
    for row in all_rows:
        grouped.setdefault((row["cell"], row["k"], row["target"]), []).append(row)
        if row.get("outer_fold_fingerprint") is not None:
            fold_fingerprints.setdefault((row["cell"], row["k"]), set()).add(row["outer_fold_fingerprint"])
    if any(len(values) != 1 for values in fold_fingerprints.values()):
        raise ConfirmationError("outcome-free folds changed across replications")
    summaries = []
    attempts = manifest["scientific_contract"]["replications_per_cell"]
    for spec in manifest["scientific_contract"]["cells"]:
        for target in TARGETS:
            key = (spec["cell"], spec["k"], target)
            summary = _summary(grouped.get(key, []), attempts)
            summary.update(spec)
            summary["target"] = target
            summaries.append(summary)
    scientific_failures, severe = _scientific_failures(summaries, manifest["profile"])
    status = "PASS" if manifest["profile"] == "confirmation" and not scientific_failures else (
        "FAIL" if manifest["profile"] == "confirmation" else "COMPLETE"
    )
    aggregate_payload = b"".join(
        json.dumps(row, sort_keys=True, separators=(",", ":")).encode() + b"\n" for row in all_rows
    )
    summaries_payload = _summaries_csv(summaries)
    _write_new(output_dir / "aggregate.jsonl", aggregate_payload)
    _write_new(output_dir / "summaries.csv", summaries_payload)
    receipt = {
        "schema": SUMMARY_SCHEMA,
        "status": status,
        "decision": (
            "structured_observation_q0_q1_confirmation_pass"
            if status == "PASS"
            else "structured_observation_q0_q1_confirmation_failed"
            if status == "FAIL"
            else "pipeline_complete_non_evidentiary"
        ),
        "profile": manifest["profile"],
        "runtime": {"python": sys.version.split()[0], "executable": sys.executable},
        "manifest_sha256": manifest_hash,
        "source": manifest["source"],
        "registration": manifest["registration"],
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
            "Assumption-conditional evidence for FEVC structured observation-level variance models; "
            "not unrestricted-heteroskedastic KSS variance-product evidence. Q1 uses the KSS/Andrews-"
            "Mikusheva curvature construction, whose claim is uniform at-least-nominal coverage."
        ),
    }
    _write_new(output_dir / "receipt.json", _json_bytes(receipt))
    return receipt


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    preflight = subparsers.add_parser("run-preflight")
    preflight.add_argument("--root", type=Path, required=True)
    preflight.add_argument("--output-dir", type=Path, required=True)
    preflight.add_argument("--binary", type=Path, required=True)
    create = subparsers.add_parser("create-manifest")
    create.add_argument("--profile", choices=tuple(PROFILE_DEFAULTS), required=True)
    create.add_argument("--root", type=Path, required=True)
    create.add_argument("--output", type=Path, required=True)
    create.add_argument("--shard-size", type=int)
    create.add_argument("--preflight-receipt", type=Path, required=True)
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
            result = run_preflight(arguments.root.resolve(), arguments.output_dir, arguments.binary)
        elif arguments.command == "create-manifest":
            result = create_manifest(arguments.root.resolve(), arguments.profile, arguments.output, arguments.shard_size, arguments.preflight_receipt)
        elif arguments.command == "run-task":
            run_task(arguments.manifest, arguments.task_id, arguments.output_dir, arguments.binary)
            result = {"status": "success", "task_id": arguments.task_id}
        else:
            result = aggregate(arguments.manifest, arguments.task_dir, arguments.output_dir, arguments.build_receipt)
    except (ConfirmationError, OSError, subprocess.SubprocessError, ValueError) as error:
        print(json.dumps({"status": "failure", "error": str(error)}, sort_keys=True))
        return 1
    print(json.dumps(result, indent=2, sort_keys=True))
    return 1 if result.get("status") == "FAIL" else 0


if __name__ == "__main__":
    raise SystemExit(main())

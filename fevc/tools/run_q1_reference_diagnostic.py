#!/usr/bin/env python3
"""Create, execute, validate, and aggregate the registered q=1 V4 diagnostic."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import os
import statistics
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Iterable, Sequence
from xml.sax.saxutils import escape


MANIFEST_SCHEMA = "fevc-q1-reference-campaign-v4"
ROW_SCHEMA = "fevc-q1-reference-diagnostic-v4"
TASK_RECEIPT_SCHEMA = "fevc-q1-reference-task-v4"
SUMMARY_SCHEMA = "fevc-q1-reference-summary-v4"
REGISTRATION_SCHEMA = "FEVC_STRUCTURED_INFERENCE_QUALIFICATION_V4"
REGISTRATION = Path("fevc/docs/structured_inference_qualification_v4.json")
SAMPLES = ("calibration", "evaluation")
OUTCOME_DGPS = ("gaussian", "student_t8")
KINDS = (("outcome", "gaussian"), ("outcome", "student_t8"), ("reference", "gaussian_reference"))
VARIANTS = (
    "production_q1",
    "fixed_population_q1",
    "leading_population_q1",
    "remainder_population_q1",
    "cross_population_q1",
    "production_q0",
    "fixed_population_q0",
)
OUTCOME_SEEDS = {
    "calibration": 0x6F9A31C2074D85E1,
    "evaluation": 0xBD427619A05E3CF8,
}
REFERENCE_SEEDS = {
    "calibration": 0x293E8CB754A1F602,
    "evaluation": 0xE51740AD9B6328C4,
}
PROFILE_DEFAULTS = {
    "tiny": {
        "dimensions": (8,),
        "calibration_replications": 2,
        "evaluation_replications": 2,
        "shard_size": 2,
    },
    "smoke": {
        "dimensions": (16,),
        "calibration_replications": 8,
        "evaluation_replications": 8,
        "shard_size": 8,
    },
    "diagnostic": {
        "dimensions": (64,),
        "calibration_replications": 20_000,
        "evaluation_replications": 10_000,
        "shard_size": 500,
    },
}
COVERAGE_TARGET = 0.95
COVERAGE_ABSOLUTE_TOLERANCE = 0.015
COVERAGE_MCSE_MULTIPLIER = 3.0
SUCCESS_RATE_MINIMUM = 0.99
OUTCOME_NUMERIC = (
    "truth",
    "point_error",
    "score_error",
    "remainder_error",
    "leading_variance_estimated",
    "leading_variance_population",
    "remainder_variance_estimated",
    "remainder_variance_population",
    "cross_covariance_estimated",
    "cross_covariance_population",
    "full_variance_estimated",
    "full_variance_population",
    "leading_variance_correction",
    "remainder_identity_error",
    "leading_share",
    "remainder_share",
    "maximum_mode_share",
    "maximum_full_influence_share",
    "maximum_remainder_influence_share",
    "required_radius_population",
)
REFERENCE_NUMERIC = (
    "truth",
    "score_error",
    "remainder_error",
    "leading_variance_population",
    "remainder_variance_population",
    "cross_covariance_population",
    "curvature_population",
    "theoretical_critical",
    "required_radius_population",
)


class DiagnosticError(RuntimeError):
    """The V4 campaign inputs or outputs violate the frozen contract."""


def _json_bytes(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def _sha256(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def _write_new(path: Path, payload: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        raise DiagnosticError(f"refusing to replace existing output {path}")
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
        ["git", "status", "--porcelain=v1"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    diff = subprocess.run(
        ["git", "diff", "--binary", "HEAD"], cwd=root, check=True, capture_output=True
    ).stdout
    entries = subprocess.run(
        ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
        cwd=root,
        check=True,
        capture_output=True,
    ).stdout.split(b"\0")
    source_lines = []
    for encoded in entries:
        if not encoded:
            continue
        relative = encoded.decode()
        path = root / relative
        if not path.is_file() or path.is_symlink():
            raise DiagnosticError(f"source manifest entry is not a regular file: {relative}")
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
    path = root / REGISTRATION
    try:
        payload = path.read_bytes()
        value = json.loads(payload)
    except (OSError, json.JSONDecodeError) as error:
        raise DiagnosticError(f"cannot read V4 registration: {error}") from error
    if value.get("schema") != REGISTRATION_SCHEMA:
        raise DiagnosticError("V4 registration schema mismatch")
    return {"path": str(REGISTRATION), "sha256": _sha256(payload)}


def create_manifest(
    root: Path, profile: str, output: Path, shard_size: int | None
) -> dict[str, Any]:
    defaults = PROFILE_DEFAULTS[profile]
    shard = shard_size or defaults["shard_size"]
    source = _git_identity(root)
    if profile != "tiny" and source["dirty"]:
        raise DiagnosticError(f"{profile} manifests require a clean committed source")
    tasks = []
    task_id = 1
    for dimension in defaults["dimensions"]:
        for sample in SAMPLES:
            replications = defaults[f"{sample}_replications"]
            if shard <= 0 or replications % shard:
                raise DiagnosticError(
                    "shard size must divide both calibration and evaluation replications"
                )
            for start in range(0, replications, shard):
                tasks.append(
                    {
                        "task_id": task_id,
                        "sample": sample,
                        "dimension": dimension,
                        "replication_start": start,
                        "replications": shard,
                        "output": f"task-{task_id:05d}.jsonl",
                        "receipt": f"task-{task_id:05d}.receipt.json",
                    }
                )
                task_id += 1
    manifest = {
        "schema": MANIFEST_SCHEMA,
        "profile": profile,
        "source": source,
        "registration": _registration_identity(root),
        "scientific_contract": {
            "design": "dominant_common_v4",
            "target": "firm_variance",
            "dimensions": list(defaults["dimensions"]),
            "variance": "true_observation_variance",
            "error_dgps": list(OUTCOME_DGPS),
            "reference_self_test": "exact_joint_gaussian",
            "calibration_replications": defaults["calibration_replications"],
            "evaluation_replications": defaults["evaluation_replications"],
            "outcome_seeds": OUTCOME_SEEDS,
            "reference_seeds": REFERENCE_SEEDS,
            "common_random_numbers": "Gaussian numerator shared by paired Gaussian and standardized-t8 observations",
            "population_covariance": "analytic fixed-design zero-signal covariance",
            "empirical_radius_role": "diagnostic_only_from_calibration_sample",
            "coverage_rule": {
                "target": COVERAGE_TARGET,
                "tolerance": "max(0.015,3*coverage_mcse)",
            },
            "variants": list(VARIANTS),
        },
        "task_count": len(tasks),
        "tasks": tasks,
    }
    _write_new(output, _json_bytes(manifest))
    return manifest


def _read_manifest(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as error:
        raise DiagnosticError(f"cannot read manifest: {error}") from error
    if value.get("schema") != MANIFEST_SCHEMA:
        raise DiagnosticError("manifest schema mismatch")
    tasks = value.get("tasks")
    if not isinstance(tasks, list) or value.get("task_count") != len(tasks):
        raise DiagnosticError("manifest task inventory is malformed")
    if [task.get("task_id") for task in tasks] != list(range(1, len(tasks) + 1)):
        raise DiagnosticError("manifest task identifiers are not canonical")
    if value.get("profile") != "tiny" and value.get("source", {}).get("dirty"):
        raise DiagnosticError("SCC manifest records a dirty source")
    contract = value.get("scientific_contract", {})
    if contract.get("outcome_seeds") != OUTCOME_SEEDS or contract.get("reference_seeds") != REFERENCE_SEEDS:
        raise DiagnosticError("manifest semantic RNG domains do not match V4")
    if set(contract.get("dimensions", ())) != {task["dimension"] for task in tasks}:
        raise DiagnosticError("manifest dimensions and task inventory disagree")
    return value


def _task(manifest: dict[str, Any], task_id: int) -> dict[str, Any]:
    if task_id < 1 or task_id > manifest["task_count"]:
        raise DiagnosticError(f"task id {task_id} is outside the manifest")
    return manifest["tasks"][task_id - 1]


def _splitmix64(value: int) -> int:
    mask = (1 << 64) - 1
    value = (value + 0x9E3779B97F4A7C15) & mask
    value = ((value ^ (value >> 30)) * 0xBF58476D1CE4E5B9) & mask
    value = ((value ^ (value >> 27)) * 0x94D049BB133111EB) & mask
    return (value ^ (value >> 31)) & mask


def _hash_label(value: str) -> int:
    state = 0xCBF29CE484222325
    for byte in value.encode():
        state = ((state ^ byte) * 0x100000001B3) & ((1 << 64) - 1)
    return state


def _semantic_seed(master: int, label: str, dimension: int, replication: int) -> int:
    return _splitmix64(
        master
        ^ _splitmix64(_hash_label(label))
        ^ _splitmix64(dimension)
        ^ _splitmix64(replication)
    )


def _finite_number(row: dict[str, Any], field: str, key: tuple[Any, ...]) -> float:
    value = row.get(field)
    if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value):
        raise DiagnosticError(f"{key}: nonfinite or missing {field}")
    return float(value)


def _validate_interval(row: dict[str, Any], prefix: str, key: tuple[Any, ...]) -> None:
    lower = _finite_number(row, f"{prefix}_lower", key)
    upper = _finite_number(row, f"{prefix}_upper", key)
    width = _finite_number(row, f"{prefix}_width", key)
    critical = _finite_number(row, f"{prefix}_critical", key)
    if upper < lower or critical <= 0.0 or abs(width - (upper - lower)) > 1.0e-10 * max(1.0, abs(width)):
        raise DiagnosticError(f"{key}: incoherent {prefix} interval")
    flags = []
    for suffix in ("covered", "lower_miss", "upper_miss"):
        value = row.get(f"{prefix}_{suffix}")
        if not isinstance(value, bool):
            raise DiagnosticError(f"{key}: malformed {prefix}_{suffix}")
        flags.append(value)
    if sum(flags) != 1:
        raise DiagnosticError(f"{key}: incoherent {prefix} coverage flags")


def _validate_row(row: Any, task: dict[str, Any]) -> tuple[str, str, str, int, int]:
    if not isinstance(row, dict) or row.get("schema") != ROW_SCHEMA:
        raise DiagnosticError("task emitted a malformed row")
    key = (
        row.get("sample"),
        row.get("kind"),
        row.get("error_dgp"),
        row.get("k"),
        row.get("replication"),
    )
    if key[0] != task["sample"] or (key[1], key[2]) not in KINDS:
        raise DiagnosticError(f"task emitted an unknown semantic key {key}")
    if key[3] != task["dimension"] or not isinstance(key[4], int) or not (
        task["replication_start"]
        <= key[4]
        < task["replication_start"] + task["replications"]
    ):
        raise DiagnosticError(f"task emitted an out-of-range semantic key {key}")
    seed = row.get("semantic_seed")
    if not isinstance(seed, int):
        raise DiagnosticError(f"{key}: malformed semantic seed")
    if key[1] == "outcome":
        expected_seed = _semantic_seed(
            OUTCOME_SEEDS[key[0]], "dominant_common_v4", key[3], key[4]
        )
    else:
        expected_seed = _semantic_seed(
            REFERENCE_SEEDS[key[0]], "q1_reference_v4", key[3], key[4]
        )
    if seed != expected_seed:
        raise DiagnosticError(f"{key}: mixed or incorrect semantic seed")
    if row.get("status") != "success":
        if not isinstance(row.get("status"), str) or row["status"] == "":
            raise DiagnosticError(f"{key}: malformed failure status")
        return key
    numeric = OUTCOME_NUMERIC if key[1] == "outcome" else REFERENCE_NUMERIC
    for field in numeric:
        _finite_number(row, field, key)
    if key[1] == "outcome":
        for field in (
            "leading_variance_estimated",
            "leading_variance_population",
            "remainder_variance_estimated",
            "remainder_variance_population",
            "full_variance_estimated",
            "full_variance_population",
        ):
            if row[field] <= 0.0:
                raise DiagnosticError(f"{key}: nonpositive {field}")
        if row["remainder_identity_error"] > 1.0e-8 * max(1.0, abs(row["point_error"])):
            raise DiagnosticError(f"{key}: material remainder identity error")
        if key[0] == "evaluation":
            for variant in VARIANTS:
                _validate_interval(row, variant, key)
            for suffix in ("lower", "upper", "width", "critical"):
                left = row[f"production_q1_{suffix}"]
                right = row[f"leading_population_q1_{suffix}"]
                if abs(left - right) > 1.0e-11 * max(1.0, abs(left), abs(right)):
                    raise DiagnosticError(
                        f"{key}: leading-population hybrid is not production-identical"
                    )
            implied = row["required_radius_population"] <= row["fixed_population_q1_critical"]
            if implied != row["fixed_population_q1_covered"]:
                raise DiagnosticError(f"{key}: fixed interval and required radius disagree")
    elif key[0] == "evaluation":
        _validate_interval(row, "reference_q1", key)
        implied = row["required_radius_population"] <= row["reference_q1_critical"]
        if implied != row["reference_q1_covered"]:
            raise DiagnosticError(f"{key}: reference interval and required radius disagree")
    return key


def _expected_keys(task: dict[str, Any]) -> set[tuple[str, str, str, int, int]]:
    return {
        (task["sample"], kind, dgp, task["dimension"], replication)
        for replication in range(
            task["replication_start"],
            task["replication_start"] + task["replications"],
        )
        for kind, dgp in KINDS
    }


def _binary_sha256(binary: Path) -> str:
    return _sha256(binary.read_bytes())


def run_task(manifest_path: Path, task_id: int, output_dir: Path, binary: Path) -> None:
    manifest = _read_manifest(manifest_path)
    task = _task(manifest, task_id)
    command = [
        str(binary),
        "diagnostic-q1-v4",
        task["sample"],
        str(task["dimension"]),
        str(task["replication_start"]),
        str(task["replications"]),
    ]
    completed = subprocess.run(command, capture_output=True, text=True)
    if completed.returncode:
        raise DiagnosticError(
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
            raise DiagnosticError(f"task {task_id} emitted non-JSON output") from error
        key = _validate_row(row, task)
        if key in seen:
            raise DiagnosticError(f"task {task_id} emitted duplicate key {key}")
        seen.add(key)
        rows.append((key, row))
    expected = _expected_keys(task)
    if seen != expected:
        raise DiagnosticError(
            f"task {task_id} inventory mismatch: missing={sorted(expected-seen)[:4]} "
            f"extra={sorted(seen-expected)[:4]}"
        )
    rows.sort(key=lambda item: item[0])
    payload = b"".join(
        json.dumps(row, sort_keys=True, separators=(",", ":")).encode() + b"\n"
        for _, row in rows
    )
    output_path = output_dir / task["output"]
    _write_new(output_path, payload)
    receipt = {
        "schema": TASK_RECEIPT_SCHEMA,
        "status": "success",
        "runtime": {"python": sys.version.split()[0], "executable": sys.executable},
        "task": task,
        "manifest_sha256": _sha256(manifest_path.read_bytes()),
        "source": manifest["source"],
        "registration": manifest["registration"],
        "command": command,
        "binary_sha256": _binary_sha256(binary),
        "row_count": len(rows),
        "output_sha256": _sha256(payload),
        "stderr": completed.stderr.splitlines(),
    }
    _write_new(output_dir / task["receipt"], _json_bytes(receipt))


def _task_rows(path: Path, task: dict[str, Any]) -> tuple[list[dict[str, Any]], str]:
    payload = path.read_bytes()
    rows = []
    seen = set()
    for line in payload.splitlines():
        try:
            row = json.loads(line)
        except json.JSONDecodeError as error:
            raise DiagnosticError(f"{path}: malformed JSON row") from error
        key = _validate_row(row, task)
        if key in seen:
            raise DiagnosticError(f"{path}: duplicate key {key}")
        seen.add(key)
        rows.append(row)
    expected = _expected_keys(task)
    if seen != expected:
        raise DiagnosticError(
            f"{path}: partial inventory missing={sorted(expected-seen)[:4]} "
            f"extra={sorted(seen-expected)[:4]}"
        )
    return rows, _sha256(payload)


def _quantile(values: Sequence[float], probability: float) -> float | None:
    if not values:
        return None
    ordered = sorted(values)
    position = probability * (len(ordered) - 1)
    lower = math.floor(position)
    upper = math.ceil(position)
    if lower == upper:
        return ordered[lower]
    weight = position - lower
    return ordered[lower] * (1.0 - weight) + ordered[upper] * weight


def _nearest_rank(values: Sequence[float], probability: float) -> float:
    if not values:
        raise DiagnosticError("cannot calibrate a radius without successful replications")
    ordered = sorted(values)
    return ordered[min(len(ordered), max(1, math.ceil(probability * len(ordered)))) - 1]


def _moments(values: Sequence[float]) -> dict[str, Any]:
    mean = statistics.fmean(values) if values else None
    centered = [value - mean for value in values] if mean is not None else []
    variance = (
        sum(value * value for value in centered) / (len(values) - 1)
        if len(values) >= 2
        else None
    )
    second = (
        sum(value * value for value in centered) / len(values) if values else None
    )
    skewness = (
        sum(value**3 for value in centered) / len(values) / second**1.5
        if len(values) >= 3 and second is not None and second > 0.0
        else None
    )
    excess = (
        sum(value**4 for value in centered) / len(values) / second**2 - 3.0
        if len(values) >= 4 and second is not None and second > 0.0
        else None
    )
    output = {
        "mean": mean,
        "variance": variance,
        "skewness": skewness,
        "excess_kurtosis": excess,
    }
    for probability in (0.005, 0.01, 0.025, 0.05, 0.5, 0.95, 0.975, 0.99, 0.995):
        output[f"q{probability:g}"] = _quantile(values, probability)
    return output


def _sample_covariance(left: Sequence[float], right: Sequence[float]) -> float | None:
    if len(left) < 2 or len(left) != len(right):
        return None
    left_mean = statistics.fmean(left)
    right_mean = statistics.fmean(right)
    return sum(
        (left_value - left_mean) * (right_value - right_mean)
        for left_value, right_value in zip(left, right)
    ) / (len(left) - 1)


def _coverage_summary(flags: Sequence[bool], lower: Sequence[bool] | None = None, upper: Sequence[bool] | None = None) -> dict[str, Any]:
    if not flags:
        return {"coverage": None, "coverage_mcse": None, "lower_miss": None, "upper_miss": None}
    coverage = statistics.fmean(float(value) for value in flags)
    return {
        "coverage": coverage,
        "coverage_mcse": math.sqrt(coverage * (1.0 - coverage) / len(flags)),
        "lower_miss": statistics.fmean(float(value) for value in lower) if lower is not None else None,
        "upper_miss": statistics.fmean(float(value) for value in upper) if upper is not None else None,
    }


def _coverage_pass(summary: dict[str, Any]) -> bool:
    if summary["coverage"] is None or summary["coverage_mcse"] is None:
        return False
    tolerance = max(
        COVERAGE_ABSOLUTE_TOLERANCE,
        COVERAGE_MCSE_MULTIPLIER * summary["coverage_mcse"],
    )
    return abs(summary["coverage"] - COVERAGE_TARGET) <= tolerance


def _normal_cdf(value: float) -> float:
    return 0.5 * (1.0 + math.erf(value / math.sqrt(2.0)))


_GL_NODE = (
    0.04830766568773832, 0.1444719615827965, 0.23928736225213707,
    0.33186860228212767, 0.42135127613063533, 0.5068999089322294,
    0.5877157572407623, 0.6630442669302152, 0.7321821187402897,
    0.7944837959679424, 0.84936761373257, 0.8963211557660521,
    0.9349060759377397, 0.9647622555875064, 0.9856115115452684,
    0.9972638618494816,
)
_GL_WEIGHT = (
    0.0965400885147278, 0.09563872007927486, 0.09384439908080457,
    0.09117387869576388, 0.08765209300440381, 0.08331192422694685,
    0.07819389578707031, 0.0723457941088485, 0.06582222277636185,
    0.05868409347853555, 0.05099805926237618, 0.04283589802222668,
    0.03427386291302143, 0.02539206530926206, 0.01627439473090567,
    0.007018610009470097,
)


def _reference_cdf(distance: float, curvature: float) -> float:
    if distance <= 0.0:
        return 0.0
    if curvature <= 1.0e-10:
        return 2.0 * _normal_cdf(distance) - 1.0
    inverse = 1.0 / curvature

    def transformed(value: float) -> float:
        x_value = distance * (1.0 - value * value)
        y_square = max(
            0.0,
            (distance - x_value) * (2.0 * inverse + distance + x_value),
        )
        half_density = math.sqrt(2.0 / math.pi) * math.exp(-0.5 * x_value * x_value)
        half_cdf = min(1.0, max(0.0, 2.0 * _normal_cdf(math.sqrt(y_square)) - 1.0))
        return 2.0 * distance * value * half_density * half_cdf

    integral = 0.0
    for node, weight in zip(_GL_NODE, _GL_WEIGHT):
        integral += 0.5 * weight * (
            transformed(0.5 * (1.0 - node)) + transformed(0.5 * (1.0 + node))
        )
    return min(1.0, max(0.0, integral))


def _reference_quantile(curvature: float, probability: float) -> float:
    lower = 0.0
    upper = 2.0
    while _reference_cdf(upper, curvature) < probability:
        upper *= 2.0
    for _ in range(64):
        midpoint = 0.5 * (lower + upper)
        if _reference_cdf(midpoint, curvature) < probability:
            lower = midpoint
        else:
            upper = midpoint
    return 0.5 * (lower + upper)


def _radius_summary(rows: Sequence[dict[str, Any]], calibration_critical: float) -> dict[str, Any]:
    values = [float(row["required_radius_population"]) for row in rows]
    output = {
        "count": len(values),
        "calibration_critical": calibration_critical,
        "coverage_at_calibration_critical": statistics.fmean(
            float(value <= calibration_critical) for value in values
        ) if values else None,
    }
    for probability in (0.5, 0.9, 0.95, 0.975, 0.99):
        output[f"empirical_q{probability:g}"] = _quantile(values, probability)
    return output


def _distribution_summary(rows: Sequence[dict[str, Any]]) -> dict[str, Any]:
    successful = [row for row in rows if row.get("status") == "success"]
    if not successful:
        return {"attempts": len(rows), "successes": 0, "success_rate": 0.0}
    leading_z = [
        row["score_error"] / math.sqrt(row["leading_variance_population"])
        for row in successful
    ]
    remainder_z = [
        row["remainder_error"] / math.sqrt(row["remainder_variance_population"])
        for row in successful
    ]
    point_errors = [row["point_error"] for row in successful if "point_error" in row]
    empirical_leading = _sample_covariance(
        [row["score_error"] for row in successful],
        [row["score_error"] for row in successful],
    )
    empirical_remainder = _sample_covariance(
        [row["remainder_error"] for row in successful],
        [row["remainder_error"] for row in successful],
    )
    empirical_cross = _sample_covariance(
        [row["score_error"] for row in successful],
        [row["remainder_error"] for row in successful],
    )
    leading_population = statistics.fmean(row["leading_variance_population"] for row in successful)
    remainder_population = statistics.fmean(row["remainder_variance_population"] for row in successful)
    cross_population = statistics.fmean(row["cross_covariance_population"] for row in successful)
    output = {
        "attempts": len(rows),
        "successes": len(successful),
        "success_rate": len(successful) / len(rows),
        "failure_counts": {},
        "bias": statistics.fmean(point_errors) if point_errors else None,
        "bias_mcse": (
            statistics.stdev(point_errors) / math.sqrt(len(point_errors))
            if len(point_errors) >= 2
            else None
        ),
        "empirical_leading_variance": empirical_leading,
        "mean_estimated_leading_variance": statistics.fmean(
            row.get("leading_variance_estimated", row["leading_variance_population"])
            for row in successful
        ),
        "population_leading_variance": leading_population,
        "empirical_remainder_variance": empirical_remainder,
        "mean_estimated_remainder_variance": statistics.fmean(
            row.get("remainder_variance_estimated", row["remainder_variance_population"])
            for row in successful
        ),
        "population_remainder_variance": remainder_population,
        "empirical_cross_covariance": empirical_cross,
        "mean_estimated_cross_covariance": statistics.fmean(
            row.get("cross_covariance_estimated", row["cross_covariance_population"])
            for row in successful
        ),
        "population_cross_covariance": cross_population,
        "leading": _moments(leading_z),
        "remainder": _moments(remainder_z),
    }
    for row in rows:
        if row.get("status") != "success":
            output["failure_counts"][row["status"]] = output["failure_counts"].get(row["status"], 0) + 1
    standardized_covariance = _sample_covariance(leading_z, remainder_z)
    leading_sd = statistics.stdev(leading_z) if len(leading_z) >= 2 else None
    remainder_sd = statistics.stdev(remainder_z) if len(remainder_z) >= 2 else None
    correlation = (
        standardized_covariance / (leading_sd * remainder_sd)
        if standardized_covariance is not None
        and leading_sd is not None
        and remainder_sd is not None
        and leading_sd > 0.0
        and remainder_sd > 0.0
        else None
    )
    square_covariance = _sample_covariance(
        [value * value for value in leading_z],
        [value * value for value in remainder_z],
    )
    output["joint"] = {
        "correlation": correlation,
        "population_standardized_covariance": cross_population
        / math.sqrt(leading_population * remainder_population),
        "estimated_standardized_covariance": output["mean_estimated_cross_covariance"]
        / math.sqrt(
            output["mean_estimated_leading_variance"]
            * output["mean_estimated_remainder_variance"]
        ),
        "standardized_covariance_error": (
            empirical_cross - output["mean_estimated_cross_covariance"]
        )
        / math.sqrt(
            output["mean_estimated_leading_variance"]
            * output["mean_estimated_remainder_variance"]
        ),
        "squared_dependence_covariance": square_covariance,
        "absolute_1_96_coexceedance": statistics.fmean(
            float(abs(left) > 1.959963984540054 and abs(right) > 1.959963984540054)
            for left, right in zip(leading_z, remainder_z)
        ),
        "same_sign_rate": statistics.fmean(
            float(left * right >= 0.0) for left, right in zip(leading_z, remainder_z)
        ),
    }
    if "maximum_mode_share" in successful[0]:
        for field in (
            "leading_share",
            "remainder_share",
            "maximum_mode_share",
            "maximum_full_influence_share",
            "maximum_remainder_influence_share",
        ):
            output[f"mean_{field}"] = statistics.fmean(row[field] for row in successful)
        output["maximum_remainder_identity_error"] = max(
            row["remainder_identity_error"] for row in successful
        )
    output["empirical_to_estimated_leading_variance_ratio"] = (
        empirical_leading / output["mean_estimated_leading_variance"]
    )
    output["empirical_to_estimated_remainder_variance_ratio"] = (
        empirical_remainder / output["mean_estimated_remainder_variance"]
    )
    return output


def _variant_summaries(rows: Sequence[dict[str, Any]], empirical_critical: float) -> dict[str, Any]:
    successful = [row for row in rows if row.get("status") == "success"]
    output = {}
    for variant in VARIANTS:
        summary = _coverage_summary(
            [row[f"{variant}_covered"] for row in successful],
            [row[f"{variant}_lower_miss"] for row in successful],
            [row[f"{variant}_upper_miss"] for row in successful],
        )
        summary["mean_width"] = statistics.fmean(row[f"{variant}_width"] for row in successful) if successful else None
        if variant.endswith("q1") and variant != "production_q1":
            summary["mean_lower_change_from_production"] = statistics.fmean(
                row[f"{variant}_lower"] - row["production_q1_lower"] for row in successful
            ) if successful else None
            summary["mean_upper_change_from_production"] = statistics.fmean(
                row[f"{variant}_upper"] - row["production_q1_upper"] for row in successful
            ) if successful else None
        summary["passes_v3_coverage_rule"] = _coverage_pass(summary)
        output[variant] = summary
    empirical = _coverage_summary(
        [row["required_radius_population"] <= empirical_critical for row in successful]
    )
    empirical["critical"] = empirical_critical
    empirical["passes_v3_coverage_rule"] = _coverage_pass(empirical)
    output["empirical_calibrated_fixed_q1"] = empirical
    return output


def _reference_variant_summary(rows: Sequence[dict[str, Any]], empirical_critical: float) -> dict[str, Any]:
    successful = [row for row in rows if row.get("status") == "success"]
    reference = _coverage_summary(
        [row["reference_q1_covered"] for row in successful],
        [row["reference_q1_lower_miss"] for row in successful],
        [row["reference_q1_upper_miss"] for row in successful],
    )
    reference["mean_width"] = statistics.fmean(row["reference_q1_width"] for row in successful) if successful else None
    reference["passes_v3_coverage_rule"] = _coverage_pass(reference)
    empirical = _coverage_summary(
        [row["required_radius_population"] <= empirical_critical for row in successful]
    )
    empirical["critical"] = empirical_critical
    empirical["passes_v3_coverage_rule"] = _coverage_pass(empirical)
    return {"reference_q1": reference, "empirical_calibrated_fixed_q1": empirical}


def _classify(summaries: dict[str, Any]) -> dict[str, Any]:
    reference = summaries["gaussian_reference"]["variants"]["reference_q1"]
    gaussian = summaries["gaussian"]["variants"]
    student = summaries["student_t8"]["variants"]
    repairs = [
        variant
        for variant in (
            "leading_population_q1",
            "remainder_population_q1",
            "cross_population_q1",
        )
        if not student["production_q1"]["passes_v3_coverage_rule"]
        and student[variant]["passes_v3_coverage_rule"]
    ]
    if not reference["passes_v3_coverage_rule"]:
        classification = "critical_radius_or_ellipse_image_problem"
    elif (
        gaussian["fixed_population_q1"]["passes_v3_coverage_rule"]
        and not student["fixed_population_q1"]["passes_v3_coverage_rule"]
    ):
        classification = "finite_sample_non_gaussian_reference_law_problem"
    elif (
        student["fixed_population_q1"]["passes_v3_coverage_rule"]
        and not student["production_q1"]["passes_v3_coverage_rule"]
    ):
        classification = "covariance_studentization_problem"
    elif (
        not gaussian["fixed_population_q1"]["passes_v3_coverage_rule"]
        and not student["fixed_population_q1"]["passes_v3_coverage_rule"]
    ):
        classification = "q1_decomposition_or_reference_mapping_problem"
    elif (
        student["fixed_population_q0"]["passes_v3_coverage_rule"]
        and not student["production_q1"]["passes_v3_coverage_rule"]
    ):
        classification = "q1_transition_region_problem"
    elif student["production_q1"]["passes_v3_coverage_rule"]:
        classification = "v4_primary_cell_passes"
    else:
        classification = "unresolved"
    return {
        "classification": classification,
        "covariance_component_repairs": repairs,
        "correction_authorized": False,
        "promotion_authorized": False,
    }


def _csv_bytes(rows: Iterable[Sequence[Any]]) -> bytes:
    with tempfile.SpooledTemporaryFile(mode="w+", newline="", encoding="utf-8") as stream:
        writer = csv.writer(stream, lineterminator="\n")
        writer.writerows(rows)
        stream.seek(0)
        return stream.read().encode()


def _svg_polyline(
    title: str,
    series: Sequence[tuple[str, Sequence[tuple[float, float]], str]],
    x_label: str,
    y_label: str,
) -> bytes:
    width, height = 760, 500
    left, right, top, bottom = 78, 24, 48, 66
    points = [point for _, values, _ in series for point in values]
    x_min = min(point[0] for point in points)
    x_max = max(point[0] for point in points)
    y_min = min(point[1] for point in points)
    y_max = max(point[1] for point in points)
    if x_max == x_min:
        x_max += 1.0
    if y_max == y_min:
        y_max += 1.0
    x_scale = (width - left - right) / (x_max - x_min)
    y_scale = (height - top - bottom) / (y_max - y_min)
    body = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
        '<rect width="100%" height="100%" fill="white"/>',
        f'<text x="{width/2}" y="28" text-anchor="middle" font-family="sans-serif" font-size="18">{escape(title)}</text>',
        f'<line x1="{left}" y1="{height-bottom}" x2="{width-right}" y2="{height-bottom}" stroke="#222"/>',
        f'<line x1="{left}" y1="{top}" x2="{left}" y2="{height-bottom}" stroke="#222"/>',
    ]
    for index, (label, values, color) in enumerate(series):
        rendered = " ".join(
            f"{left + (x-x_min)*x_scale:.2f},{height-bottom-(y-y_min)*y_scale:.2f}"
            for x, y in values
        )
        body.append(f'<polyline points="{rendered}" fill="none" stroke="{color}" stroke-width="2"/>')
        body.append(f'<text x="{left + 12 + index*180}" y="{height-18}" font-family="sans-serif" font-size="12" fill="{color}">{escape(label)}</text>')
    body.extend(
        [
            f'<text x="{width/2}" y="{height-42}" text-anchor="middle" font-family="sans-serif" font-size="13">{escape(x_label)}</text>',
            f'<text transform="translate(18 {height/2}) rotate(-90)" text-anchor="middle" font-family="sans-serif" font-size="13">{escape(y_label)}</text>',
            f'<text x="{left}" y="{height-bottom+18}" font-family="sans-serif" font-size="11">{x_min:.3g}</text>',
            f'<text x="{width-right}" y="{height-bottom+18}" text-anchor="end" font-family="sans-serif" font-size="11">{x_max:.3g}</text>',
            f'<text x="{left-8}" y="{height-bottom}" text-anchor="end" font-family="sans-serif" font-size="11">{y_min:.3g}</text>',
            f'<text x="{left-8}" y="{top+4}" text-anchor="end" font-family="sans-serif" font-size="11">{y_max:.3g}</text>',
            "</svg>",
        ]
    )
    return ("\n".join(body) + "\n").encode()


def _coverage_svg(
    title: str, summaries: dict[str, dict[str, dict[str, Any]]]
) -> bytes:
    variants = list(summaries["gaussian"])
    width, height = 900, 520
    left, right, top, bottom = 72, 24, 50, 120
    y_min, y_max = 0.90, 1.00
    x_step = (width - left - right) / len(variants)
    y_scale = (height - top - bottom) / (y_max - y_min)
    colors = {"gaussian": "#1f77b4", "student_t8": "#d62728"}
    body = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
        '<rect width="100%" height="100%" fill="white"/>',
        f'<text x="{width/2}" y="28" text-anchor="middle" font-family="sans-serif" font-size="18">{escape(title)}</text>',
        f'<line x1="{left}" y1="{height-bottom}" x2="{width-right}" y2="{height-bottom}" stroke="#222"/>',
        f'<line x1="{left}" y1="{top}" x2="{left}" y2="{height-bottom}" stroke="#222"/>',
    ]
    nominal_y = height - bottom - (0.95 - y_min) * y_scale
    body.append(
        f'<line x1="{left}" y1="{nominal_y:.2f}" x2="{width-right}" y2="{nominal_y:.2f}" stroke="#555" stroke-dasharray="5 4"/>'
    )
    for index, variant in enumerate(variants):
        x_center = left + (index + 0.5) * x_step
        body.append(
            f'<text transform="translate({x_center:.2f} {height-bottom+14}) rotate(50)" font-family="sans-serif" font-size="10">{escape(variant)}</text>'
        )
        for offset, dgp in ((-6.0, "gaussian"), (6.0, "student_t8")):
            value = summaries[dgp][variant]
            coverage = value["coverage"]
            mcse = value["coverage_mcse"]
            if coverage is None or mcse is None:
                continue
            x_value = x_center + offset
            y_value = height - bottom - (coverage - y_min) * y_scale
            low = height - bottom - (min(y_max, coverage + 1.96 * mcse) - y_min) * y_scale
            high = height - bottom - (max(y_min, coverage - 1.96 * mcse) - y_min) * y_scale
            color = colors[dgp]
            body.extend(
                [
                    f'<line x1="{x_value:.2f}" y1="{low:.2f}" x2="{x_value:.2f}" y2="{high:.2f}" stroke="{color}"/>',
                    f'<circle cx="{x_value:.2f}" cy="{y_value:.2f}" r="3.5" fill="{color}"/>',
                ]
            )
    for value in (0.90, 0.925, 0.95, 0.975, 1.0):
        y_value = height - bottom - (value - y_min) * y_scale
        body.append(
            f'<text x="{left-8}" y="{y_value+4:.2f}" text-anchor="end" font-family="sans-serif" font-size="11">{value:.3f}</text>'
        )
    body.extend(
        [
            f'<text x="{left+20}" y="{height-18}" font-family="sans-serif" font-size="12" fill="{colors["gaussian"]}">Gaussian</text>',
            f'<text x="{left+120}" y="{height-18}" font-family="sans-serif" font-size="12" fill="{colors["student_t8"]}">standardized t8</text>',
            f'<text transform="translate(18 {height/2}) rotate(-90)" text-anchor="middle" font-family="sans-serif" font-size="13">coverage (95% MC bars)</text>',
            "</svg>",
        ]
    )
    return ("\n".join(body) + "\n").encode()


def aggregate(manifest_path: Path, task_dir: Path, output_dir: Path) -> dict[str, Any]:
    manifest = _read_manifest(manifest_path)
    all_rows = []
    global_keys = set()
    task_hashes = {}
    task_receipt_hashes = {}
    binary_hashes = set()
    failures = []
    manifest_hash = _sha256(manifest_path.read_bytes())
    for task in manifest["tasks"]:
        output_path = task_dir / task["output"]
        receipt_path = task_dir / task["receipt"]
        try:
            receipt_payload = receipt_path.read_bytes()
            receipt = json.loads(receipt_payload)
            rows, digest = _task_rows(output_path, task)
            if (
                receipt.get("schema") != TASK_RECEIPT_SCHEMA
                or receipt.get("status") != "success"
                or receipt.get("task") != task
                or receipt.get("output_sha256") != digest
                or receipt.get("manifest_sha256") != manifest_hash
                or receipt.get("source") != manifest["source"]
                or receipt.get("registration") != manifest["registration"]
            ):
                raise DiagnosticError("receipt does not bind the task output")
            for row in rows:
                key = (row["sample"], row["kind"], row["error_dgp"], row["k"], row["replication"])
                if key in global_keys:
                    raise DiagnosticError(f"duplicate campaign key {key}")
                global_keys.add(key)
            binary_hashes.add(receipt.get("binary_sha256"))
        except (OSError, json.JSONDecodeError, DiagnosticError) as error:
            failures.append(f"task {task['task_id']}: {error}")
            continue
        all_rows.extend(rows)
        task_hashes[task["output"]] = digest
        task_receipt_hashes[task["receipt"]] = _sha256(receipt_payload)
    if failures:
        raise DiagnosticError("; ".join(failures))
    if len(binary_hashes) != 1 or None in binary_hashes:
        raise DiagnosticError("task receipts do not share one binary identity")
    expected_count = sum(task["replications"] for task in manifest["tasks"]) * len(KINDS)
    if len(all_rows) != expected_count:
        raise DiagnosticError("aggregate row count does not match the manifest")

    dimensions = manifest["scientific_contract"]["dimensions"]
    summary_by_dimension = {}
    coverage_table = [["k", "error_dgp", "variant", "coverage", "coverage_mcse", "lower_miss", "upper_miss", "mean_width", "passes_v3_rule"]]
    distribution_table = [["k", "error_dgp", "score", "mean", "variance", "skewness", "excess_kurtosis", "q0.025", "q0.5", "q0.975"]]
    radius_table = [["k", "error_dgp", "probability", "empirical_cdf", "theoretical_cdf"]]
    plot_rows: dict[int, dict[str, list[dict[str, Any]]]] = {}
    for dimension in dimensions:
        dimension_summary = {}
        plot_rows[dimension] = {}
        calibration_criticals = {}
        for dgp in (*OUTCOME_DGPS, "gaussian_reference"):
            calibration = [
                row for row in all_rows
                if row["k"] == dimension and row["sample"] == "calibration" and row["error_dgp"] == dgp and row.get("status") == "success"
            ]
            required_successes = manifest["scientific_contract"]["calibration_replications"]
            if len(calibration) / required_successes < SUCCESS_RATE_MINIMUM:
                raise DiagnosticError(f"{dimension}/{dgp}: calibration success rate below V4 minimum")
            calibration_criticals[dgp] = _nearest_rank(
                [row["required_radius_population"] for row in calibration], COVERAGE_TARGET
            )
        reference_rows = [
            row for row in all_rows
            if row["k"] == dimension and row["sample"] == "evaluation" and row["error_dgp"] == "gaussian_reference"
        ]
        reference_success = [row for row in reference_rows if row.get("status") == "success"]
        if not reference_success:
            raise DiagnosticError(f"{dimension}: no successful reference evaluation rows")
        curvature = reference_success[0]["curvature_population"]
        theoretical_critical = reference_success[0]["theoretical_critical"]
        for row in reference_success:
            if abs(row["curvature_population"] - curvature) > 1.0e-12 or abs(row["theoretical_critical"] - theoretical_critical) > 1.0e-12:
                raise DiagnosticError(f"{dimension}: reference covariance is not fixed")
        for dgp in (*OUTCOME_DGPS, "gaussian_reference"):
            evaluation = [
                row for row in all_rows
                if row["k"] == dimension and row["sample"] == "evaluation" and row["error_dgp"] == dgp
            ]
            expected = manifest["scientific_contract"]["evaluation_replications"]
            successful = [row for row in evaluation if row.get("status") == "success"]
            if len(successful) / expected < SUCCESS_RATE_MINIMUM:
                raise DiagnosticError(f"{dimension}/{dgp}: evaluation success rate below V4 minimum")
            distribution = _distribution_summary(evaluation)
            radius = _radius_summary(successful, calibration_criticals[dgp])
            radius["theoretical_critical"] = theoretical_critical
            radius["theoretical_cdf_at_critical"] = _reference_cdf(theoretical_critical, curvature)
            ordered_radius = sorted(row["required_radius_population"] for row in successful)
            radius["kolmogorov_distance_from_q1_reference"] = max(
                max(
                    abs(_reference_cdf(value, curvature) - index / len(ordered_radius)),
                    abs(_reference_cdf(value, curvature) - (index - 1) / len(ordered_radius)),
                )
                for index, value in enumerate(ordered_radius, start=1)
            )
            radius["theoretical_quantiles"] = {
                f"q{probability:g}": _reference_quantile(curvature, probability)
                for probability in (0.5, 0.9, 0.95, 0.975, 0.99)
            }
            variants = (
                _reference_variant_summary(successful, calibration_criticals[dgp])
                if dgp == "gaussian_reference"
                else _variant_summaries(successful, calibration_criticals[dgp])
            )
            dimension_summary[dgp] = {
                "distribution": distribution,
                "required_radius": radius,
                "variants": variants,
            }
            plot_rows[dimension][dgp] = successful
            for score in ("leading", "remainder"):
                moment = distribution[score]
                distribution_table.append([
                    dimension, dgp, score, moment["mean"], moment["variance"], moment["skewness"], moment["excess_kurtosis"], moment["q0.025"], moment["q0.5"], moment["q0.975"],
                ])
            for variant, value in variants.items():
                coverage_table.append([
                    dimension, dgp, variant, value["coverage"], value["coverage_mcse"], value.get("lower_miss"), value.get("upper_miss"), value.get("mean_width"), value["passes_v3_coverage_rule"],
                ])
            grid_max = max(3.0, _quantile(ordered_radius, 0.995) or 3.0)
            for index in range(101):
                point = grid_max * index / 100.0
                empirical = sum(value <= point for value in ordered_radius) / len(ordered_radius)
                theoretical = _reference_cdf(point, curvature)
                radius_table.append([dimension, dgp, point, empirical, theoretical])
        summary_by_dimension[str(dimension)] = dimension_summary

    classification = _classify(summary_by_dimension[str(dimensions[-1])])
    diagnostics = {
        "schema": SUMMARY_SCHEMA,
        "status": "COMPLETE",
        "profile": manifest["profile"],
        "source": manifest["source"],
        "registration": manifest["registration"],
        "manifest_sha256": manifest_hash,
        "binary_sha256": next(iter(binary_hashes)),
        "row_count": len(all_rows),
        "task_count": manifest["task_count"],
        "summaries": summary_by_dimension,
        "decision": classification,
        "limitations": [
            "development diagnostic only; it does not authorize confirmation or promotion",
            "true observation variances and the registered zero-signal firm target only",
            "empirical radii are held-out diagnostic benchmarks and are not a public method",
            "this is not unrestricted-heteroskedastic KSS variance-product evidence",
        ],
    }
    output_hashes = {}
    for name, payload in (
        ("diagnostics.json", _json_bytes(diagnostics)),
        ("coverage.csv", _csv_bytes(coverage_table)),
        ("distribution.csv", _csv_bytes(distribution_table)),
        ("required_radius_cdf.csv", _csv_bytes(radius_table)),
    ):
        _write_new(output_dir / name, payload)
        output_hashes[name] = _sha256(payload)

    primary = dimensions[-1]
    cdf_series = []
    colors = {"gaussian": "#1f77b4", "student_t8": "#d62728", "gaussian_reference": "#2ca02c"}
    for dgp in (*OUTCOME_DGPS, "gaussian_reference"):
        values = sorted(row["required_radius_population"] for row in plot_rows[primary][dgp])
        step = max(1, len(values) // 400)
        points = [(value, (index + 1) / len(values)) for index, value in enumerate(values) if index % step == 0]
        cdf_series.append((dgp, points, colors[dgp]))
    primary_curvature = plot_rows[primary]["gaussian_reference"][0][
        "curvature_population"
    ]
    theoretical_points = [
        (
            3.0 * index / 400.0,
            _reference_cdf(3.0 * index / 400.0, primary_curvature),
        )
        for index in range(401)
    ]
    cdf_series.append(("theoretical", theoretical_points, "#111111"))
    plots = {
        "required_radius_cdf.svg": _svg_polyline(
            f"q=1 required-radius CDF, k={primary}", cdf_series, "required radius", "CDF"
        )
    }
    normal = statistics.NormalDist()
    for score in ("leading", "remainder"):
        series = []
        for dgp in OUTCOME_DGPS:
            rows = plot_rows[primary][dgp]
            field = "score_error" if score == "leading" else "remainder_error"
            variance_field = f"{score}_variance_population"
            values = sorted(row[field] / math.sqrt(row[variance_field]) for row in rows)
            step = max(1, len(values) // 400)
            points = [
                (normal.inv_cdf((index + 0.5) / len(values)), value)
                for index, value in enumerate(values)
                if index % step == 0
            ]
            series.append((dgp, points, colors[dgp]))
        plots[f"{score}_qq.svg"] = _svg_polyline(
            f"Standard-normal QQ: {score}, k={primary}", series, "normal quantile", "empirical quantile"
        )
    plots["coverage_by_variant.svg"] = _coverage_svg(
        f"Coverage by interval variant, k={primary}",
        {
            dgp: summary_by_dimension[str(primary)][dgp]["variants"]
            for dgp in OUTCOME_DGPS
        },
    )
    for name, payload in plots.items():
        _write_new(output_dir / name, payload)
        output_hashes[name] = _sha256(payload)

    receipt = {
        "schema": SUMMARY_SCHEMA,
        "status": "COMPLETE",
        "runtime": {"python": sys.version.split()[0], "executable": sys.executable},
        "profile": manifest["profile"],
        "source": manifest["source"],
        "registration": manifest["registration"],
        "manifest_sha256": manifest_hash,
        "binary_sha256": next(iter(binary_hashes)),
        "task_count": manifest["task_count"],
        "row_count": len(all_rows),
        "task_output_sha256": task_hashes,
        "task_receipt_sha256": task_receipt_hashes,
        "aggregate_output_sha256": output_hashes,
        "decision": classification,
    }
    _write_new(output_dir / "receipt.json", _json_bytes(receipt))
    return receipt


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    create = commands.add_parser("create-manifest")
    create.add_argument("--profile", choices=tuple(PROFILE_DEFAULTS), required=True)
    create.add_argument("--root", type=Path, required=True)
    create.add_argument("--output", type=Path, required=True)
    create.add_argument("--shard-size", type=int)
    task = commands.add_parser("run-task")
    task.add_argument("manifest", type=Path)
    task.add_argument("task_id", type=int)
    task.add_argument("output_dir", type=Path)
    task.add_argument("--binary", type=Path, required=True)
    collect = commands.add_parser("aggregate")
    collect.add_argument("manifest", type=Path)
    collect.add_argument("task_dir", type=Path)
    collect.add_argument("output_dir", type=Path)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    arguments = _parser().parse_args(argv)
    try:
        if arguments.command == "create-manifest":
            result = create_manifest(
                arguments.root.resolve(), arguments.profile, arguments.output, arguments.shard_size
            )
        elif arguments.command == "run-task":
            run_task(arguments.manifest, arguments.task_id, arguments.output_dir, arguments.binary)
            result = {"status": "success", "task_id": arguments.task_id}
        else:
            result = aggregate(arguments.manifest, arguments.task_dir, arguments.output_dir)
    except (DiagnosticError, OSError, subprocess.SubprocessError, ValueError) as error:
        print(json.dumps({"status": "failure", "error": str(error)}, sort_keys=True))
        return 1
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Create, execute, aggregate, and validate the q=1 diagnostic campaign."""

from __future__ import annotations

import argparse
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


MANIFEST_SCHEMA = "fevc-structured-inference-campaign-v2"
ROW_SCHEMA = "fevc-q1-diagnostic-v2"
RECEIPT_SCHEMA = "fevc-structured-inference-task-v2"
SUMMARY_SCHEMA = "fevc-structured-inference-summary-v2"
CELLS = ("dominant_leverage", "dominant_common_t8")
SOURCES = ("oracle", "fitted")
PROFILE_DEFAULTS = {
    "smoke": {"dimensions": (8,), "replications": 2, "shard_size": 2},
    "diagnostic": {
        "dimensions": (16, 24, 32, 48, 64),
        "replications": 1_000,
        "shard_size": 100,
    },
    "confirmation": {
        "dimensions": (32, 48, 64),
        "replications": 5_000,
        "shard_size": 100,
    },
}
THRESHOLDS = {
    "bias_mcse_multiplier": 4.0,
    "coverage_absolute_tolerance": 0.015,
    "coverage_mcse_multiplier": 3.0,
    "success_rate": 0.99,
    "se_ratio_lower": 0.90,
    "se_ratio_upper": 1.10,
}
NUMERIC_FIELDS = (
    "point_error",
    "estimated_sd",
    "leading_variance",
    "remainder_variance",
    "leading_remainder_covariance",
    "curvature",
    "interval_width",
    "score_error",
    "remainder_error",
    "leading_share",
    "remainder_share",
    "maximum_mode_share",
    "remainder_influence_concentration",
    "floor_share",
    "boundary_share",
)


class CampaignError(RuntimeError):
    """The campaign inputs or outputs violate the frozen contract."""


def _json_bytes(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode()


def _sha256(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


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
    commit = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    status = subprocess.run(
        ["git", "status", "--porcelain=v1"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    diff = subprocess.run(
        ["git", "diff", "--binary", "HEAD"],
        cwd=root,
        check=True,
        capture_output=True,
    ).stdout
    listed = subprocess.run(
        ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
        cwd=root,
        check=True,
        capture_output=True,
    ).stdout.split(b"\0")
    source_lines = []
    for encoded in listed:
        if not encoded:
            continue
        relative = encoded.decode("utf-8")
        path = root / relative
        if not path.is_file() or path.is_symlink():
            raise CampaignError(f"source manifest entry is not a regular file: {relative}")
        source_lines.append(f"{_sha256(path.read_bytes())}  {relative}\n")
    source_manifest = "".join(source_lines).encode()
    return {
        "commit": commit,
        "dirty": bool(status),
        "status": status.splitlines(),
        "worktree_diff_sha256": _sha256(diff),
        "source_file_count": len(source_lines),
        "source_manifest_sha256": _sha256(source_manifest),
    }


def create_manifest(
    root: Path,
    profile: str,
    output: Path,
    shard_size: int | None,
) -> dict[str, Any]:
    defaults = PROFILE_DEFAULTS[profile]
    shard = shard_size or defaults["shard_size"]
    if shard <= 0 or defaults["replications"] % shard:
        raise CampaignError("shard size must divide the profile replication count")
    source = _git_identity(root)
    if profile == "confirmation" and source["dirty"]:
        raise CampaignError("confirmation manifests require a clean source tree")
    tasks = []
    task_id = 1
    for dimension in defaults["dimensions"]:
        for start in range(0, defaults["replications"], shard):
            tasks.append(
                {
                    "task_id": task_id,
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
        "scientific_contract": {
            "cells": list(CELLS),
            "variance_sources": list(SOURCES),
            "target": "firm",
            "dimensions": list(defaults["dimensions"]),
            "replications_per_cell_dimension": defaults["replications"],
            "reference": "q1",
            "error_variance": "oracle_and_cross_fitted_structured",
            "interval_simulations": 2_000,
            "thresholds": THRESHOLDS,
            "k16_role": "finite_sample_diagnostic",
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
        raise CampaignError(f"cannot read manifest: {error}") from error
    if value.get("schema") != MANIFEST_SCHEMA:
        raise CampaignError("manifest schema mismatch")
    tasks = value.get("tasks")
    if not isinstance(tasks, list) or value.get("task_count") != len(tasks):
        raise CampaignError("manifest task inventory is malformed")
    ids = [task.get("task_id") for task in tasks]
    if ids != list(range(1, len(tasks) + 1)):
        raise CampaignError("manifest task identifiers are not canonical")
    if value.get("profile") == "confirmation" and value.get("source", {}).get("dirty"):
        raise CampaignError("confirmation manifest records a dirty source")
    return value


def _task(manifest: dict[str, Any], task_id: int) -> dict[str, Any]:
    if task_id < 1 or task_id > manifest["task_count"]:
        raise CampaignError(f"task id {task_id} is outside the manifest")
    return manifest["tasks"][task_id - 1]


def _validate_row(row: Any, task: dict[str, Any]) -> tuple[str, int, int, str]:
    if not isinstance(row, dict) or row.get("schema") != ROW_SCHEMA:
        raise CampaignError("task emitted a malformed row")
    key = (
        row.get("cell"),
        row.get("k"),
        row.get("replication"),
        row.get("variance_source"),
    )
    if key[0] not in CELLS or key[3] not in SOURCES:
        raise CampaignError(f"task emitted an unknown semantic key {key}")
    if key[1] != task["dimension"] or not (
        task["replication_start"]
        <= key[2]
        < task["replication_start"] + task["replications"]
    ):
        raise CampaignError(f"task emitted an out-of-range semantic key {key}")
    if row.get("status") == "success":
        for field in NUMERIC_FIELDS:
            value = row.get(field)
            if not isinstance(value, (int, float)) or not math.isfinite(value):
                raise CampaignError(f"{key}: nonfinite or missing {field}")
        for field in ("covered", "lower_miss", "upper_miss"):
            if not isinstance(row.get(field), bool):
                raise CampaignError(f"{key}: malformed {field}")
        if sum(bool(row[field]) for field in ("covered", "lower_miss", "upper_miss")) != 1:
            raise CampaignError(f"{key}: interval outcome is incoherent")
    return key


def run_task(manifest_path: Path, task_id: int, output_dir: Path, binary: Path) -> None:
    manifest = _read_manifest(manifest_path)
    task = _task(manifest, task_id)
    command = [
        str(binary),
        "diagnostic-q1",
        str(task["dimension"]),
        str(task["replication_start"]),
        str(task["replications"]),
    ]
    completed = subprocess.run(command, capture_output=True, text=True)
    if completed.returncode:
        raise CampaignError(
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
            raise CampaignError(f"task {task_id} emitted non-JSON output") from error
        key = _validate_row(row, task)
        if key in seen:
            raise CampaignError(f"task {task_id} emitted duplicate key {key}")
        seen.add(key)
        rows.append((key, row))
    expected = {
        (cell, task["dimension"], replication, source)
        for cell in CELLS
        for replication in range(
            task["replication_start"],
            task["replication_start"] + task["replications"],
        )
        for source in SOURCES
    }
    if seen != expected:
        raise CampaignError(
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
        "schema": RECEIPT_SCHEMA,
        "status": "success",
        "runtime": {"python": sys.version.split()[0], "executable": sys.executable},
        "task": task,
        "manifest_sha256": _sha256(manifest_path.read_bytes()),
        "source": manifest["source"],
        "command": command,
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
    coverage = (
        statistics.fmean(float(row["covered"]) for row in successful)
        if successful
        else None
    )
    empirical_sd = _sample_sd(errors)
    mean_se = _mean(successful, "estimated_sd")
    output = {
        "attempts": attempts,
        "successes": len(successful),
        "success_rate": len(successful) / attempts,
        "bias": statistics.fmean(errors) if errors else None,
        "bias_mcse": (
            empirical_sd / math.sqrt(len(successful))
            if empirical_sd is not None
            else None
        ),
        "coverage": coverage,
        "coverage_mcse": (
            math.sqrt(coverage * (1.0 - coverage) / len(successful))
            if coverage is not None
            else None
        ),
        "lower_miss": (
            statistics.fmean(float(row["lower_miss"]) for row in successful)
            if successful
            else None
        ),
        "upper_miss": (
            statistics.fmean(float(row["upper_miss"]) for row in successful)
            if successful
            else None
        ),
        "empirical_sd": empirical_sd,
        "mean_se": mean_se,
        "se_ratio": (
            empirical_sd / mean_se
            if empirical_sd is not None and mean_se is not None and mean_se > 0.0
            else None
        ),
    }
    for field in NUMERIC_FIELDS[2:]:
        output[f"mean_{field}"] = _mean(successful, field)
    failures: dict[str, int] = {}
    for row in rows:
        if row.get("status") != "success":
            failures[row["status"]] = failures.get(row["status"], 0) + 1
    output["failure_counts"] = failures
    return output


def _gate(label: str, summary: dict[str, Any]) -> list[str]:
    failures = []
    if summary["success_rate"] < THRESHOLDS["success_rate"]:
        failures.append(f"{label}: success rate")
    required = ("bias", "bias_mcse", "coverage", "coverage_mcse", "se_ratio")
    if summary["successes"] < 2 or any(summary[field] is None for field in required):
        failures.append(f"{label}: insufficient successful replications")
        return failures
    if abs(summary["bias"]) > max(
        1.0e-12, THRESHOLDS["bias_mcse_multiplier"] * summary["bias_mcse"]
    ):
        failures.append(f"{label}: bias")
    coverage_bound = max(
        THRESHOLDS["coverage_absolute_tolerance"],
        THRESHOLDS["coverage_mcse_multiplier"] * summary["coverage_mcse"],
    )
    if abs(summary["coverage"] - 0.95) > coverage_bound:
        failures.append(f"{label}: coverage")
    if not THRESHOLDS["se_ratio_lower"] <= summary["se_ratio"] <= THRESHOLDS["se_ratio_upper"]:
        failures.append(f"{label}: standard-error ratio")
    return failures


def _task_rows(path: Path, task: dict[str, Any]) -> tuple[list[dict[str, Any]], str]:
    payload = path.read_bytes()
    rows = []
    seen = set()
    for line in payload.splitlines():
        row = json.loads(line)
        key = _validate_row(row, task)
        if key in seen:
            raise CampaignError(f"{path}: duplicate key {key}")
        seen.add(key)
        rows.append(row)
    return rows, _sha256(payload)


def aggregate(manifest_path: Path, task_dir: Path, output_dir: Path) -> dict[str, Any]:
    manifest = _read_manifest(manifest_path)
    all_rows = []
    output_hashes = {}
    failures = []
    for task in manifest["tasks"]:
        output_path = task_dir / task["output"]
        receipt_path = task_dir / task["receipt"]
        try:
            receipt = json.loads(receipt_path.read_text())
            rows, digest = _task_rows(output_path, task)
            if (
                receipt.get("schema") != RECEIPT_SCHEMA
                or receipt.get("status") != "success"
                or receipt.get("output_sha256") != digest
                or receipt.get("manifest_sha256") != _sha256(manifest_path.read_bytes())
            ):
                raise CampaignError("receipt does not bind the task output")
        except (OSError, json.JSONDecodeError, CampaignError) as error:
            failures.append(f"task {task['task_id']}: {error}")
            continue
        all_rows.extend(rows)
        output_hashes[task["output"]] = digest
    if failures:
        raise CampaignError("; ".join(failures))
    all_rows.sort(key=lambda row: (row["cell"], row["k"], row["replication"], row["variance_source"]))
    expected_count = 2 * 2 * sum(task["replications"] for task in manifest["tasks"])
    if len(all_rows) != expected_count:
        raise CampaignError("aggregate row count does not match the manifest")
    summaries = []
    gate_failures = []
    attempts = manifest["scientific_contract"]["replications_per_cell_dimension"]
    for cell in CELLS:
        for dimension in manifest["scientific_contract"]["dimensions"]:
            for source in SOURCES:
                selected = [
                    row
                    for row in all_rows
                    if row["cell"] == cell
                    and row["k"] == dimension
                    and row["variance_source"] == source
                ]
                summary = _summary(selected, attempts)
                summary.update({"cell": cell, "k": dimension, "variance_source": source})
                summaries.append(summary)
                if dimension in (32, 48, 64):
                    gate_failures.extend(_gate(f"{cell}/{dimension}/{source}", summary))
    profile = manifest["profile"]
    status = "PASS" if profile == "confirmation" and not gate_failures else "COMPLETE"
    decision = (
        "finite_sample_convergence_supported"
        if not gate_failures and all(dimension in manifest["scientific_contract"]["dimensions"] for dimension in (32, 48, 64))
        else "correction_or_additional_evidence_required"
    )
    receipt = {
        "schema": SUMMARY_SCHEMA,
        "status": status,
        "runtime": {"python": sys.version.split()[0], "executable": sys.executable},
        "profile": profile,
        "decision": decision,
        "manifest_sha256": _sha256(manifest_path.read_bytes()),
        "row_count": len(all_rows),
        "task_output_sha256": output_hashes,
        "gate_failures": gate_failures,
        "thresholds": THRESHOLDS,
        "summaries": summaries,
        "interpretation": (
            "Assumption-conditional evidence for FEVC's structured variance q=1 mode; "
            "not unrestricted-heteroskedastic KSS variance-product evidence."
        ),
    }
    aggregate_payload = b"".join(
        json.dumps(row, sort_keys=True, separators=(",", ":")).encode() + b"\n"
        for row in all_rows
    )
    _write_new(output_dir / "aggregate.jsonl", aggregate_payload)
    _write_new(output_dir / "receipt.json", _json_bytes(receipt))
    return receipt


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    create = subparsers.add_parser("create-manifest")
    create.add_argument("--profile", choices=tuple(PROFILE_DEFAULTS), required=True)
    create.add_argument("--root", type=Path, required=True)
    create.add_argument("--output", type=Path, required=True)
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
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    arguments = _parser().parse_args(argv)
    try:
        if arguments.command == "create-manifest":
            result = create_manifest(
                arguments.root.resolve(),
                arguments.profile,
                arguments.output,
                arguments.shard_size,
            )
        elif arguments.command == "run-task":
            run_task(arguments.manifest, arguments.task_id, arguments.output_dir, arguments.binary)
            result = {"status": "success", "task_id": arguments.task_id}
        else:
            result = aggregate(arguments.manifest, arguments.task_dir, arguments.output_dir)
    except (CampaignError, OSError, subprocess.SubprocessError, ValueError) as error:
        print(json.dumps({"status": "failure", "error": str(error)}, sort_keys=True))
        return 1
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

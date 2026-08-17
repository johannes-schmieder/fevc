#!/usr/bin/env python3
"""Shared validation and receipt helpers for the MATLAB scale benchmark."""

import csv
import hashlib
import json
import math
import os
import re
import tempfile
from pathlib import Path

HEX40 = re.compile(r"^[0-9a-f]{40}$")
HEX64 = re.compile(r"^[0-9a-f]{64}$")
LABEL = re.compile(r"^[A-Za-z0-9._-]+$")


class BenchmarkError(RuntimeError):
    """Typed benchmark validation failure."""


def require(condition, message):
    if not condition:
        raise BenchmarkError(message)


def sha256_file(path):
    digest = hashlib.sha256()
    with Path(path).open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def sha256_inventory(root, relative_paths):
    """Match ``sha256sum files | sha256sum`` using POSIX relative names."""
    digest = hashlib.sha256()
    root = Path(root)
    for relative in relative_paths:
        item = root / relative
        require(item.is_file(), f"missing maintained source: {relative}")
        line = f"{sha256_file(item)}  {relative}\n"
        digest.update(line.encode("utf-8"))
    return digest.hexdigest()


def load_json(path):
    try:
        with Path(path).open("r", encoding="utf-8") as handle:
            value = json.load(handle)
    except (OSError, ValueError) as exc:
        raise BenchmarkError(f"cannot read JSON {path}: {exc}") from exc
    require(isinstance(value, dict), f"JSON root must be an object: {path}")
    return value


def atomic_write_json(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(
        prefix=path.name + ".", suffix=".tmp", dir=str(path.parent)
    )
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as handle:
            json.dump(value, handle, indent=2, sort_keys=True, allow_nan=False)
            handle.write("\n")
        os.replace(temporary, str(path))
    except Exception:
        try:
            os.unlink(temporary)
        except OSError:
            pass
        raise


def integer(value, label, minimum=0):
    require(not isinstance(value, bool), f"{label} must be an integer")
    try:
        parsed = int(value)
    except (TypeError, ValueError) as exc:
        raise BenchmarkError(f"{label} must be an integer") from exc
    require(parsed == value and parsed >= minimum, f"{label} must be an integer at least {minimum}")
    return parsed


def finite(value, label):
    try:
        parsed = float(value)
    except (TypeError, ValueError) as exc:
        raise BenchmarkError(f"{label} must be finite") from exc
    require(math.isfinite(parsed), f"{label} must be finite")
    return parsed


def hash_value(value, label, length=64):
    pattern = HEX64 if length == 64 else HEX40
    require(
        isinstance(value, str) and pattern.fullmatch(value) is not None,
        f"{label} is not a lowercase hexadecimal digest",
    )
    return value


def target_identity(values):
    worker = finite(values["worker"], "target.worker")
    firm = finite(values["firm"], "target.firm")
    covariance = finite(values["covariance"], "target.covariance")
    total = finite(values["total"], "target.total")
    expected = worker + firm + 2.0 * covariance
    scaled_error = abs(total - expected) / (1.0 + abs(total) + abs(expected))
    return scaled_error


def matrix_relative_difference(left, right):
    numerator = math.sqrt(sum((a - b) ** 2 for a, b in zip(left, right, strict=False)))
    denominator = 1.0 + math.sqrt(sum(a * a for a in left))
    return numerator / denominator


def validate_case(binding, contract):
    require(binding.get("schema") == "kss_matlab_scale_case_v1", "case schema changed")
    require(
        contract.get("schema") == "kss_matlab_scale_source_v1", "source contract schema changed"
    )
    label = binding.get("label")
    require(isinstance(label, str) and LABEL.fullmatch(label) is not None, "invalid case label")
    scale = integer(binding.get("scale"), "scale", 1)
    require(scale in contract["supported_scales"], "unsupported scale")
    topology = binding.get("topology")
    require(topology in contract["supported_topologies"], "unsupported topology")
    sample_mode = binding.get("sample_mode")
    require(sample_mode in contract["supported_sample_modes"], "unsupported sample mode")
    require(integer(binding.get("seed"), "seed", 0) <= 4294967295, "seed exceeds uint32")
    require(
        integer(binding.get("probes"), "probes", 1) == contract["required_probes"],
        "probe count changed",
    )
    warm_repetitions = integer(binding.get("warm_repetitions"), "warm_repetitions", 1)
    require(warm_repetitions >= contract["minimum_warm_repetitions"], "too few warm repetitions")

    source = binding.get("source")
    require(isinstance(source, dict), "case source must be an object")
    hash_value(source.get("source_commit"), "source.source_commit", 40)
    hash_value(source.get("bundle_sha256"), "source.bundle_sha256")
    require(
        source.get("matlab_upstream_commit") == contract["maintained_upstream_commit"],
        "maintained upstream commit changed",
    )
    require(
        source.get("matlab_runtime_tree_sha256") == contract["runtime_tree"]["sha256"],
        "maintained runtime tree changed",
    )
    require(
        source.get("matlab_core_sha256") == contract["core"]["sha256"], "maintained core changed"
    )
    for field in (
        "matlab_cmg_entry_sha256",
        "matlab_hierarchy_sha256",
        "matlab_solver_sha256",
        "benchmark_contract_sha256",
    ):
        hash_value(source.get(field), f"source.{field}")

    input_record = binding.get("input")
    require(isinstance(input_record, dict), "case input must be an object")
    hash_value(input_record.get("sha256"), "input.sha256")
    for field in ("rows", "workers", "firms", "matches"):
        integer(input_record.get(field), f"input.{field}", 1)
    require(
        input_record.get("id_contract") == contract["input_id_contract"],
        "input ID contract changed",
    )
    require(
        input_record.get("order_contract") == contract["input_order_contract"],
        "input order contract changed",
    )

    reference = binding.get("reference_sample")
    require(isinstance(reference, dict), "reference_sample must be an object")
    require(
        reference.get("kind") in ("stata_kss_bc", "fixture_oracle", "audited_external"),
        "unsupported reference sample kind",
    )
    hash_value(reference.get("receipt_sha256"), "reference_sample.receipt_sha256")
    hash_value(reference.get("retained_key_sha256"), "reference_sample.retained_key_sha256")
    for field in ("rows", "workers", "firms", "matches"):
        integer(reference.get(field), f"reference_sample.{field}", 1)
    plugin = reference.get("plugin")
    require(isinstance(plugin, dict), "reference plug-in must be an object")
    require(
        set(plugin) == {"worker", "firm", "covariance", "total"},
        "reference plug-in target set changed",
    )
    require(target_identity(plugin) <= 1e-12, "reference plug-in accounting identity failed")
    if sample_mode == "fixed":
        for field in ("rows", "workers", "firms", "matches"):
            require(
                input_record[field] == reference[field], f"fixed input/reference {field} mismatch"
            )
    return binding


def parse_elapsed(value):
    fields = value.strip().split(":")
    require(1 <= len(fields) <= 3, "malformed GNU-time elapsed value")
    try:
        numbers = [float(item) for item in fields]
    except ValueError as exc:
        raise BenchmarkError("malformed GNU-time elapsed value") from exc
    if len(numbers) == 3:
        return numbers[0] * 3600.0 + numbers[1] * 60.0 + numbers[2]
    if len(numbers) == 2:
        return numbers[0] * 60.0 + numbers[1]
    return numbers[0]


def parse_gnu_time(path):
    result = {}
    path = Path(path)
    if not path.is_file():
        return result
    values = {}
    with path.open("r", encoding="utf-8", errors="replace") as handle:
        for raw in handle:
            stripped = raw.strip()
            signal_match = re.fullmatch(r"Command terminated by signal ([0-9]+)", stripped)
            if signal_match is not None:
                result["termination_signal"] = int(signal_match.group(1))
                continue
            if stripped.startswith("Elapsed (wall clock) time") and ": " in stripped:
                key, value = stripped.rsplit(": ", 1)
                values[key] = value
                continue
            if ":" not in stripped:
                continue
            key, value = stripped.split(":", 1)
            values[key.strip()] = value.strip()
    mappings = {
        "User time (seconds)": "user_cpu_seconds",
        "System time (seconds)": "system_cpu_seconds",
        "Maximum resident set size (kbytes)": "peak_rss_kib",
        "Exit status": "time_exit_status",
    }
    for source, target in mappings.items():
        if source in values:
            try:
                result[target] = float(values[source])
            except ValueError:
                pass
    elapsed_key = next(
        (key for key in values if key.startswith("Elapsed (wall clock) time")),
        None,
    )
    if elapsed_key is not None:
        try:
            result["process_wall_seconds"] = parse_elapsed(values[elapsed_key])
        except BenchmarkError:
            pass
    if "user_cpu_seconds" in result and "system_cpu_seconds" in result:
        result["cpu_seconds"] = result["user_cpu_seconds"] + result["system_cpu_seconds"]
    return result


def read_one_row(path):
    path = Path(path)
    if path.suffix.lower() == ".json":
        return load_json(path)
    try:
        with path.open("r", encoding="utf-8", newline="") as handle:
            rows = list(csv.DictReader(handle))
    except OSError as exc:
        raise BenchmarkError(f"cannot read candidate {path}: {exc}") from exc
    require(len(rows) == 1, "candidate aggregate must contain one row")
    return rows[0]

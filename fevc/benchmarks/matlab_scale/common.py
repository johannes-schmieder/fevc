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
CASE_SCHEMA = "kss_matlab_scale_case_v2"
PREPARATION_SCHEMA = "kss_matlab_scale_preparation_v1"
REFERENCE_SCHEMA = "kss_matlab_scale_reference_v1"
ACCEPTANCE_SCHEMA = "kss_matlab_scale_scc_acceptance_v1"
PROCESS_IDENTITY_SCHEMA = "kss_matlab_scale_process_identity_v1"
PROCESS_TREE_SCHEMA = "kss_matlab_scale_process_tree_rss_v1"
PROCESS_PID_APIS = frozenset(("matlabProcessID_R2025a", "feature_getpid"))
DIMENSION_FIELDS = ("rows", "workers", "firms", "matches")
FIXED_PREPARATION_CASES = (
    (1, "well"),
    (2, "well_connected"),
    (2, "ring"),
    (4, "well_connected"),
)
REFERENCE_ESTIMATOR = {
    "algorithm": "jla",
    "deletion": "match",
    "controls": "none",
    "frequency_semantics": "literal_physical_rows_v1",
    "target_weight_semantics": "uniform_stored_rows_v1",
}
REFERENCE_KSS_OPTIONS = {
    "option_contract": "KSS-STREAMLINE-OPTIONS-V1",
    "frequency_var": "-",
    "target_var": "-",
    "deletion_var": "-",
    "deletion_mode": "match",
}
REFERENCE_PROVENANCE_HASH_FIELDS = (
    "kss_admission_receipt_sha256",
    "kss_certificate_sha256",
    "kss_driver_sha256",
    "kss_job_id_file_sha256",
    "kss_node_receipt_sha256",
    "kss_qacct_sha256",
    "kss_reservation_sha256",
    "kss_scheduler_request_sha256",
    "kss_source_manifest_sha256",
    "kss_summary_sha256",
    "kss_validator_sha256",
    "preparation_acceptance_sha256",
    "preparation_receipt_sha256",
    "prepared_input_sha256",
    "retained_key_sha256",
    "source_input_sha256",
)
REFERENCE_PROVENANCE_FIELDS = {
    "producer",
    "experiment_id",
    "fixture",
    "scale_factor",
    "job_id",
    *REFERENCE_KSS_OPTIONS,
    *REFERENCE_PROVENANCE_HASH_FIELDS,
}


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


def record_field(record, names, label):
    """Return the first present, nonempty field from a flat receipt."""
    for name in names:
        if name in record and record[name] not in (None, ""):
            return record[name]
    raise BenchmarkError(f"receipt lacks {label}")


def validate_fixed_preparation_case(scale, topology):
    """Reject scale/topology pairs outside the admitted fixed-sample ladder."""
    scale = integer(scale, "preparation scale", 1)
    require(
        isinstance(topology, str) and (scale, topology) in FIXED_PREPARATION_CASES,
        "preparation scale/topology pair is unsupported",
    )
    return scale, topology


def validate_process_identity(
    record, *, expected_mode=None, expected_label=None, expected_case_sha256=None
):
    """Validate MATLAB-authored client and process-pool PID identities."""
    required = {
        "schema",
        "status",
        "pid_api",
        "mode",
        "label",
        "case_sha256",
        "expected_pool_workers",
        "client_pid",
        "worker_indices",
        "worker_pids",
    }
    require(isinstance(record, dict) and set(record) == required, "process identity fields changed")
    require(record.get("schema") == PROCESS_IDENTITY_SCHEMA, "process identity schema changed")
    require(record.get("status") == "PASS", "process identity did not pass")
    require(
        record.get("pid_api") in PROCESS_PID_APIS,
        "process identity PID API changed",
    )
    require(record.get("mode") in ("cold", "warm"), "process identity mode changed")
    require(
        isinstance(record.get("label"), str) and LABEL.fullmatch(record["label"]) is not None,
        "process identity label is invalid",
    )
    hash_value(record.get("case_sha256"), "process identity case SHA-256")
    expected_workers = integer(
        record.get("expected_pool_workers"), "process identity expected pool workers", 1
    )
    require(expected_workers == 4, "process identity expected pool size changed")
    indices = record.get("worker_indices")
    pids = record.get("worker_pids")
    require(isinstance(indices, list) and indices == [1, 2, 3, 4], "worker indices changed")
    require(isinstance(pids, list) and len(pids) == expected_workers, "worker PID count changed")
    worker_pids = [integer(pid, "MATLAB worker PID", 2) for pid in pids]
    client_pid = integer(record.get("client_pid"), "MATLAB client PID", 2)
    require(
        len(set(worker_pids)) == expected_workers and client_pid not in worker_pids,
        "MATLAB client and worker PIDs are not distinct",
    )
    if expected_mode is not None:
        require(record["mode"] == expected_mode, "process identity mode differs from job")
    if expected_label is not None:
        require(record["label"] == expected_label, "process identity label differs from job")
    if expected_case_sha256 is not None:
        require(
            record["case_sha256"] == expected_case_sha256,
            "process identity case differs from job",
        )
    return {
        "pid_api": record["pid_api"],
        "mode": record["mode"],
        "label": record["label"],
        "case_sha256": record["case_sha256"],
        "expected_pool_workers": expected_workers,
        "client_pid": client_pid,
        "worker_indices": indices,
        "worker_pids": worker_pids,
        "named_pids": [client_pid, *worker_pids],
    }


def validate_process_tree_record(record, identity, identity_sha256, *, identity_path=None):
    """Validate monitor proof that all MATLAB PIDs were sampled as descendants."""
    hash_value(identity_sha256, "process identity SHA-256")
    require(record.get("schema") == PROCESS_TREE_SCHEMA, "process-tree schema changed")
    require(record.get("status") == "PASS", "process-tree monitor did not pass")
    require(record.get("expected_pool_workers") == 4, "process-tree pool size changed")
    require(record.get("minimum_expected_process_count") == 5, "process-tree minimum changed")
    require(
        record.get("process_identity_sha256") == identity_sha256,
        "process-tree identity hash changed",
    )
    if identity_path is not None:
        require(
            Path(record.get("process_identity_path", "")).absolute()
            == Path(identity_path).absolute(),
            "process-tree identity path changed",
        )
    require(record.get("client_pid") == identity["client_pid"], "monitored client PID changed")
    require(record.get("worker_pids") == identity["worker_pids"], "monitored worker PIDs changed")
    require(record.get("named_pid_count") == 5, "monitored named-PID count changed")
    require(
        record.get("all_named_pids_observed_as_descendants") is True,
        "not all MATLAB PIDs were observed as descendants",
    )
    sample_count = integer(record.get("sample_count"), "process-tree sample count", 1)
    identity_samples = integer(
        record.get("identity_observation_count"), "identity observation count", 1
    )
    require(identity_samples <= sample_count, "identity observations exceed monitor samples")
    peak_rss = finite(record.get("peak_rss_kib"), "process-tree peak RSS")
    identity_peak_rss = finite(record.get("identity_peak_rss_kib"), "named-process RSS")
    peak_process_count = integer(
        record.get("peak_process_count"), "process-tree peak process count", 1
    )
    identity_peak_process_count = integer(
        record.get("identity_peak_process_count"), "named-process sample count", 5
    )
    require(
        0 < identity_peak_rss <= peak_rss
        and 5 <= identity_peak_process_count <= peak_process_count,
        "named-process RSS sample omitted a MATLAB PID or exceeded the full peak",
    )
    return {
        "sample_count": sample_count,
        "identity_observation_count": identity_samples,
        "peak_rss_kib": peak_rss,
        "identity_peak_rss_kib": identity_peak_rss,
        "peak_process_count": peak_process_count,
        "identity_peak_process_count": identity_peak_process_count,
    }


def validate_preparation_receipt(receipt):
    """Validate the immutable fields consumed by every fixed MATLAB case."""
    require(receipt.get("schema") == PREPARATION_SCHEMA, "preparation receipt schema changed")
    require(receipt.get("status") == "PASS", "preparation receipt did not pass")
    label = receipt.get("label")
    require(isinstance(label, str) and LABEL.fullmatch(label) is not None, "invalid preparation label")
    source_commit = hash_value(receipt.get("source_commit"), "preparation source commit", 40)
    bundle_sha = hash_value(receipt.get("bundle_sha256"), "preparation bundle SHA-256")
    source_input_sha = hash_value(
        receipt.get("source_input_sha256"), "preparation source input SHA-256"
    )
    prepared_input_sha = hash_value(
        receipt.get("prepared_input_sha256"), "preparation output SHA-256"
    )
    retained_key_sha = hash_value(
        receipt.get("retained_key_sha256"), "preparation retained-key SHA-256"
    )
    require(receipt.get("sample_mode") == "fixed_retained", "preparation is not fixed-retained")
    topology = receipt.get("topology")
    scale, topology = validate_fixed_preparation_case(receipt.get("scale"), topology)
    dimensions = receipt.get("dimensions")
    require(isinstance(dimensions, dict), "preparation dimensions are missing")
    dimensions = {
        field: integer(dimensions.get(field), f"preparation dimensions.{field}", 1)
        for field in DIMENSION_FIELDS
    }
    source_dimensions = receipt.get("source_dimensions")
    require(isinstance(source_dimensions, dict), "preparation source dimensions are missing")
    source_dimensions = {
        field: integer(source_dimensions.get(field), f"preparation source dimensions.{field}", 1)
        for field in DIMENSION_FIELDS
    }
    artifacts = receipt.get("artifacts")
    require(isinstance(artifacts, dict), "preparation artifacts are missing")
    require(
        artifacts.get("input_csv", {}).get("sha256") == prepared_input_sha,
        "preparation input artifact is not bound",
    )
    require(
        artifacts.get("retained_keys", {}).get("sha256") == retained_key_sha,
        "preparation retained-key artifact is not bound",
    )
    return {
        "receipt_schema": PREPARATION_SCHEMA,
        "label": label,
        "scale": scale,
        "topology": topology,
        "source_commit": source_commit,
        "bundle_sha256": bundle_sha,
        "source_input_sha256": source_input_sha,
        "prepared_input_sha256": prepared_input_sha,
        "retained_key_sha256": retained_key_sha,
        "dimensions": dimensions,
        "source_dimensions": source_dimensions,
    }


def validate_preparation_acceptance(acceptance, preparation_sha256, preparation):
    """Require a three-layer SCC acceptance receipt for fixed preparation."""
    require(acceptance.get("schema") == ACCEPTANCE_SCHEMA, "preparation acceptance schema changed")
    require(acceptance.get("status") == "PASS", "preparation SCC acceptance did not pass")
    require(acceptance.get("stage") == "prepare", "preparation acceptance stage changed")
    require(acceptance.get("label") == preparation["label"], "preparation acceptance label changed")
    require(
        acceptance.get("wrapper_sha256") == preparation_sha256,
        "preparation acceptance does not bind the wrapper receipt",
    )
    require(
        acceptance.get("source_commit") == preparation["source_commit"],
        "preparation acceptance source commit changed",
    )
    require(
        acceptance.get("bundle_sha256") == preparation["bundle_sha256"],
        "preparation acceptance bundle changed",
    )
    evidence = acceptance.get("evidence")
    require(isinstance(evidence, dict), "preparation acceptance evidence map is missing")
    require(
        set(evidence)
        == {
            "request",
            "submission",
            "scheduler_request",
            "job_id_file",
            "qacct",
            "wrapper",
        },
        "preparation acceptance evidence set changed",
    )
    top_level_hashes = {
        "request": "request_sha256",
        "submission": "submission_sha256",
        "scheduler_request": "scheduler_request_sha256",
        "job_id_file": "job_id_file_sha256",
        "qacct": "qacct_sha256",
        "wrapper": "wrapper_sha256",
    }
    for name, digest_field in top_level_hashes.items():
        item = evidence.get(name)
        require(isinstance(item, dict), f"preparation acceptance evidence is malformed: {name}")
        require(
            isinstance(item.get("path"), str) and Path(item["path"]).is_absolute(),
            f"preparation acceptance evidence path is invalid: {name}",
        )
        digest = hash_value(
            item.get("sha256"), f"preparation acceptance evidence {name} SHA-256"
        )
        require(
            acceptance.get(digest_field) == digest,
            f"preparation acceptance evidence hash changed: {name}",
        )
    for layer in ("scheduler", "application", "output"):
        require(
            acceptance.get("layers", {}).get(layer) == "PASS",
            f"preparation acceptance {layer} layer did not pass",
        )
    return acceptance


def validate_reference_record(record, preparation, *, probes, seed):
    """Normalize a converter-produced, source-bound KSS reference receipt.

    Manual hashes supplied beside an otherwise unbound aggregate are not
    evidence.  The receipt bytes themselves must name the retained key, at
    least one input digest, dimensions, and estimator semantics.
    """
    require(record.get("schema") == REFERENCE_SCHEMA, "reference receipt schema changed")
    require(record.get("status") == "PASS", "reference receipt did not pass")
    require(record.get("kind") == "stata_vckss", "reference kind was not derived from KSS")
    require(record.get("label") == preparation["label"], "reference label changed")
    require(record.get("sample_mode") == "fixed_retained", "reference sample mode changed")
    require(
        hash_value(record.get("source_commit"), "reference source commit", 40)
        == preparation["source_commit"],
        "reference source commit changed",
    )
    require(
        hash_value(record.get("bundle_sha256"), "reference bundle SHA-256")
        == preparation["bundle_sha256"],
        "reference bundle changed",
    )
    retained_key_sha = hash_value(
        record.get("retained_key_sha256"), "reference retained-key SHA-256"
    )
    require(
        retained_key_sha == preparation["retained_key_sha256"],
        "reference retained key differs from preparation",
    )
    input_bindings = {}
    for field, expected in (
        ("prepared_input_sha256", preparation["prepared_input_sha256"]),
        ("source_input_sha256", preparation["source_input_sha256"]),
    ):
        if record.get(field) not in (None, ""):
            observed = hash_value(record[field], f"reference {field}")
            require(observed == expected, f"reference {field} changed")
            input_bindings[field] = observed
    require(input_bindings, "reference receipt binds neither prepared nor source input")

    provenance = record.get("provenance")
    require(isinstance(provenance, dict), "reference KSS provenance is missing")
    require(set(provenance) == REFERENCE_PROVENANCE_FIELDS, "reference provenance fields changed")
    require(
        provenance.get("producer") == "kss_matlab_scale_reference_converter_v1",
        "reference producer changed",
    )
    require(
        isinstance(provenance.get("experiment_id"), str)
        and LABEL.fullmatch(provenance["experiment_id"]) is not None,
        "reference experiment ID is invalid",
    )
    require(
        provenance.get("fixture") in {
            "local",
            "cz24",
            "cz25",
            "cz18",
            "well_connected",
            "ring",
        },
        "reference fixture is invalid",
    )
    integer(provenance.get("scale_factor"), "reference scale factor", 1)
    require(
        isinstance(provenance.get("job_id"), str)
        and provenance["job_id"].isdigit(),
        "reference KSS job ID is invalid",
    )
    for field, expected in REFERENCE_KSS_OPTIONS.items():
        require(
            provenance.get(field) == expected,
            f"reference KSS option changed: {field}",
        )
    for field in REFERENCE_PROVENANCE_HASH_FIELDS:
        hash_value(provenance.get(field), f"reference provenance {field}")
    for field in (
        "source_input_sha256",
        "prepared_input_sha256",
        "retained_key_sha256",
    ):
        require(
            provenance[field] == preparation[field],
            f"reference provenance {field} changed",
        )

    dimensions = {
        "rows": integer(
            float(record_field(record, ("N_retained", "stored_rows", "rows"), "retained rows")),
            "reference rows",
            1,
        ),
        "workers": integer(
            float(record_field(record, ("worker_levels", "workers"), "workers")),
            "reference workers",
            1,
        ),
        "firms": integer(
            float(record_field(record, ("firm_levels", "firms"), "firms")),
            "reference firms",
            1,
        ),
        "matches": integer(
            float(record_field(record, ("deletion_units", "matches"), "matches")),
            "reference matches",
            1,
        ),
    }
    require(dimensions == preparation["dimensions"], "reference dimensions differ from preparation")

    algorithm_requested = str(record_field(record, ("algorithm_requested",), "algorithm_requested"))
    algorithm_selected = str(record_field(record, ("algorithm_selected",), "algorithm_selected"))
    require(
        algorithm_requested.lower() == algorithm_selected.lower() == REFERENCE_ESTIMATOR["algorithm"],
        "reference algorithm changed",
    )
    for field in (
        "deletion",
        "controls",
        "frequency_semantics",
        "target_weight_semantics",
    ):
        observed = str(record_field(record, (field,), field)).lower()
        require(observed == REFERENCE_ESTIMATOR[field], f"reference {field} changed")
    observed_probes = integer(
        float(record_field(record, ("requested_probes", "probes"), "probes")),
        "reference probes",
        1,
    )
    observed_seed = integer(float(record_field(record, ("seed",), "seed")), "reference seed", 0)
    require(observed_probes == probes, "reference probe count changed")
    require(observed_seed == seed, "reference seed changed")

    plugin = {
        target: finite(record_field(record, ("plugin_" + target,), "plugin_" + target), target)
        for target in ("worker", "firm", "covariance", "total")
    }
    require(target_identity(plugin) <= 1e-12, "reference plug-in accounting identity failed")
    return {
        "receipt_schema": REFERENCE_SCHEMA,
        "kind": "stata_vckss",
        "label": preparation["label"],
        "source_commit": preparation["source_commit"],
        "bundle_sha256": preparation["bundle_sha256"],
        "input_bindings": input_bindings,
        "retained_key_sha256": retained_key_sha,
        "dimensions": dimensions,
        "estimator": {
            **REFERENCE_ESTIMATOR,
            "probes": observed_probes,
            "seed": observed_seed,
        },
        "plugin": plugin,
        "provenance": provenance,
    }


def validate_case(binding, contract):
    require(binding.get("schema") == CASE_SCHEMA, "case schema changed")
    require(
        contract.get("schema") == "kss_matlab_scale_source_v1", "source contract schema changed"
    )
    require(contract.get("case_schema") == CASE_SCHEMA, "registered case schema changed")
    require(
        contract.get("preparation_schema") == PREPARATION_SCHEMA,
        "registered preparation schema changed",
    )
    require(contract.get("reference_schema") == REFERENCE_SCHEMA, "registered reference schema changed")
    require(contract.get("prepared_submission_modes") == ["fixed"], "prepared submission modes changed")
    registered_cases = contract.get("fixed_preparation_cases")
    require(
        isinstance(registered_cases, list)
        and [
            (integer(item.get("scale"), "registered preparation scale", 1), item.get("topology"))
            for item in registered_cases
            if isinstance(item, dict)
        ]
        == list(FIXED_PREPARATION_CASES)
        and len(registered_cases) == len(FIXED_PREPARATION_CASES),
        "registered fixed preparation cases changed",
    )
    label = binding.get("label")
    require(isinstance(label, str) and LABEL.fullmatch(label) is not None, "invalid case label")
    scale = integer(binding.get("scale"), "scale", 1)
    require(scale in contract["supported_scales"], "unsupported scale")
    topology = binding.get("topology")
    require(topology in contract["supported_topologies"], "unsupported topology")
    validate_fixed_preparation_case(scale, topology)
    sample_mode = binding.get("sample_mode")
    require(sample_mode in contract["supported_sample_modes"], "unsupported sample mode")
    require(sample_mode == "fixed", "prepared-case schema accepts fixed samples only")
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

    preparation = binding.get("preparation")
    require(isinstance(preparation, dict), "case preparation must be an object")
    hash_value(preparation.get("receipt_sha256"), "preparation.receipt_sha256")
    hash_value(preparation.get("acceptance_sha256"), "preparation.acceptance_sha256")
    require(
        preparation.get("receipt_schema") == PREPARATION_SCHEMA,
        "case preparation schema changed",
    )
    require(preparation.get("label") == label, "case/preparation label mismatch")
    require(preparation.get("scale") == scale, "case/preparation scale mismatch")
    require(preparation.get("topology") == topology, "case/preparation topology mismatch")
    require(
        preparation.get("source_commit") == source["source_commit"],
        "case/preparation source commit mismatch",
    )
    require(
        preparation.get("bundle_sha256") == source["bundle_sha256"],
        "case/preparation bundle mismatch",
    )
    for field in ("source_input_sha256", "prepared_input_sha256", "retained_key_sha256"):
        hash_value(preparation.get(field), f"preparation.{field}")
    require(
        preparation["prepared_input_sha256"] == input_record["sha256"],
        "case input does not match preparation",
    )
    preparation_dimensions = preparation.get("dimensions")
    require(isinstance(preparation_dimensions, dict), "preparation dimensions are missing")
    for field in DIMENSION_FIELDS:
        require(
            integer(preparation_dimensions.get(field), f"preparation.{field}", 1)
            == input_record[field],
            f"case input/preparation {field} mismatch",
        )
    preparation_source_dimensions = preparation.get("source_dimensions")
    require(
        isinstance(preparation_source_dimensions, dict),
        "preparation source dimensions are missing",
    )
    for field in DIMENSION_FIELDS:
        integer(preparation_source_dimensions.get(field), f"preparation source {field}", 1)

    reference = binding.get("reference_sample")
    require(isinstance(reference, dict), "reference_sample must be an object")
    require(
        reference.get("kind") in ("stata_vckss", "fixture_oracle", "audited_external"),
        "unsupported reference sample kind",
    )
    require(reference.get("kind") == "stata_vckss", "case reference kind is not KSS-derived")
    hash_value(reference.get("receipt_sha256"), "reference_sample.receipt_sha256")
    require(reference.get("receipt_schema") == REFERENCE_SCHEMA, "reference receipt schema changed")
    require(reference.get("label") == label, "case/reference label mismatch")
    require(
        reference.get("source_commit") == source["source_commit"],
        "case/reference source commit mismatch",
    )
    require(
        reference.get("bundle_sha256") == source["bundle_sha256"],
        "case/reference bundle mismatch",
    )
    hash_value(reference.get("retained_key_sha256"), "reference_sample.retained_key_sha256")
    require(
        reference["retained_key_sha256"] == preparation["retained_key_sha256"],
        "case reference/preparation retained-key mismatch",
    )
    input_bindings = reference.get("input_bindings")
    require(isinstance(input_bindings, dict) and input_bindings, "reference input binding missing")
    allowed_input_bindings = {"prepared_input_sha256", "source_input_sha256"}
    require(set(input_bindings) <= allowed_input_bindings, "unknown reference input binding")
    for field, digest in input_bindings.items():
        hash_value(digest, f"reference_sample.input_bindings.{field}")
        require(digest == preparation[field], f"reference {field} mismatch")
    for field in DIMENSION_FIELDS:
        integer(reference.get(field), f"reference_sample.{field}", 1)
        require(reference[field] == preparation_dimensions[field], f"reference {field} mismatch")
    estimator = reference.get("estimator")
    require(isinstance(estimator, dict), "reference estimator binding missing")
    for field, expected in REFERENCE_ESTIMATOR.items():
        require(estimator.get(field) == expected, f"reference estimator {field} changed")
    require(estimator.get("probes") == binding["probes"], "reference estimator probes changed")
    require(estimator.get("seed") == binding["seed"], "reference estimator seed changed")
    plugin = reference.get("plugin")
    require(isinstance(plugin, dict), "reference plug-in must be an object")
    require(
        set(plugin) == {"worker", "firm", "covariance", "total"},
        "reference plug-in target set changed",
    )
    require(target_identity(plugin) <= 1e-12, "reference plug-in accounting identity failed")
    provenance = reference.get("provenance")
    require(isinstance(provenance, dict), "case reference provenance is missing")
    require(set(provenance) == REFERENCE_PROVENANCE_FIELDS, "case reference provenance changed")
    for field in REFERENCE_PROVENANCE_HASH_FIELDS:
        hash_value(provenance.get(field), f"reference provenance {field}")
    require(
        provenance.get("producer") == "kss_matlab_scale_reference_converter_v1",
        "case reference producer changed",
    )
    for field, expected in REFERENCE_KSS_OPTIONS.items():
        require(provenance.get(field) == expected, f"case reference KSS option changed: {field}")
    require(
        provenance["preparation_receipt_sha256"] == preparation["receipt_sha256"]
        and provenance["preparation_acceptance_sha256"] == preparation["acceptance_sha256"]
        and provenance["source_input_sha256"] == preparation["source_input_sha256"]
        and provenance["prepared_input_sha256"] == preparation["prepared_input_sha256"]
        and provenance["retained_key_sha256"] == preparation["retained_key_sha256"],
        "case reference provenance differs from preparation",
    )
    for field in DIMENSION_FIELDS:
        require(input_record[field] == reference[field], f"fixed input/reference {field} mismatch")
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

#!/usr/bin/env python3
"""Create a checksum-bound MATLAB scale case without reading row data."""

import argparse
import sys
from pathlib import Path

from common import (
    BenchmarkError,
    atomic_write_json,
    finite,
    hash_value,
    integer,
    load_json,
    read_one_row,
    require,
    sha256_file,
    target_identity,
    validate_case,
)
from verify_case import inventory_hash, runtime_tree

TARGETS = ("worker", "firm", "covariance", "total")


def source_identity(matlab_root, contract):
    matlab_root = Path(matlab_root)
    core = matlab_root / contract["core"]["relative_path"]
    cmg = matlab_root / contract["cmg_entry"]["relative_path"]
    hierarchy_paths = contract["hierarchy_sources"]
    hierarchy_display = [Path(item).name for item in hierarchy_paths]
    solver_paths = contract["solver_sources"]
    solver_display = [item[len("CMG/") :] for item in solver_paths]
    runtime_paths, runtime_sha = runtime_tree(matlab_root, contract["runtime_tree"]["roots"])
    identity = {
        "matlab_upstream_commit": contract["maintained_upstream_commit"],
        "matlab_runtime_tree_sha256": runtime_sha,
        "matlab_core_sha256": sha256_file(core),
        "matlab_cmg_entry_sha256": sha256_file(cmg),
        "matlab_hierarchy_sha256": inventory_hash(matlab_root, hierarchy_paths, hierarchy_display),
        "matlab_solver_sha256": inventory_hash(matlab_root, solver_paths, solver_display),
    }
    require(
        len(runtime_paths) == contract["runtime_tree"]["file_count"],
        "maintained runtime file count changed",
    )
    require(
        identity["matlab_runtime_tree_sha256"] == contract["runtime_tree"]["sha256"],
        "maintained runtime tree changed",
    )
    require(identity["matlab_core_sha256"] == contract["core"]["sha256"], "maintained core changed")
    return identity


def field(row, names, label):
    for name in names:
        if name in row and row[name] not in (None, ""):
            return row[name]
    raise BenchmarkError(f"reference aggregate lacks {label}")


def build(args):
    contract_path = Path(args.contract)
    input_path = Path(args.input)
    reference_path = Path(args.reference_aggregate)
    require(contract_path.is_file(), "source contract is missing")
    require(input_path.is_file(), "input CSV is missing")
    require(reference_path.is_file(), "reference aggregate is missing")
    contract = load_json(contract_path)
    reference = read_one_row(reference_path)
    source = source_identity(args.matlab_root, contract)
    source.update(
        {
            "source_commit": hash_value(args.source_commit, "source commit", 40),
            "bundle_sha256": hash_value(args.bundle_sha256, "bundle SHA-256"),
            "benchmark_contract_sha256": sha256_file(contract_path),
        }
    )
    plugin = {
        target: finite(
            field(reference, ("plugin_" + target,), "plugin_" + target), "plugin_" + target
        )
        for target in TARGETS
    }
    require(target_identity(plugin) <= 1e-12, "reference plug-in accounting identity failed")
    reference_sample = {
        "kind": args.reference_kind,
        "receipt_sha256": sha256_file(reference_path),
        "retained_key_sha256": hash_value(
            args.reference_key_sha256, "reference retained-key SHA-256"
        ),
        "rows": integer(
            float(field(reference, ("N_retained", "stored_rows"), "retained rows")),
            "reference rows",
            1,
        ),
        "workers": integer(
            float(field(reference, ("worker_levels", "workers"), "workers")), "reference workers", 1
        ),
        "firms": integer(
            float(field(reference, ("firm_levels", "firms"), "firms")), "reference firms", 1
        ),
        "matches": integer(
            float(field(reference, ("deletion_units", "matches"), "matches")),
            "reference matches",
            1,
        ),
        "plugin": plugin,
    }
    case = {
        "schema": "kss_matlab_scale_case_v1",
        "label": args.label,
        "scale": args.scale,
        "topology": args.topology,
        "sample_mode": args.sample_mode,
        "seed": args.seed,
        "probes": args.probes,
        "warm_repetitions": args.warm_repetitions,
        "source": source,
        "input": {
            "sha256": sha256_file(input_path),
            "rows": args.input_rows,
            "workers": args.input_workers,
            "firms": args.input_firms,
            "matches": args.input_matches,
            "id_contract": contract["input_id_contract"],
            "order_contract": contract["input_order_contract"],
        },
        "reference_sample": reference_sample,
    }
    validate_case(case, contract)
    atomic_write_json(args.output, case)
    print(sha256_file(args.output))
    return 0


def parse_args(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument("--contract", required=True)
    parser.add_argument("--matlab-root", required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--bundle-sha256", required=True)
    parser.add_argument("--label", required=True)
    parser.add_argument("--scale", type=int, required=True)
    parser.add_argument("--topology", required=True)
    parser.add_argument("--sample-mode", choices=("fixed", "selection"), required=True)
    parser.add_argument("--seed", type=int, default=8675309)
    parser.add_argument("--probes", type=int, default=200)
    parser.add_argument("--warm-repetitions", type=int, default=3)
    parser.add_argument("--input", required=True)
    parser.add_argument("--input-rows", type=int, required=True)
    parser.add_argument("--input-workers", type=int, required=True)
    parser.add_argument("--input-firms", type=int, required=True)
    parser.add_argument("--input-matches", type=int, required=True)
    parser.add_argument("--reference-aggregate", required=True)
    parser.add_argument(
        "--reference-kind",
        default="stata_kss_bc",
        choices=("stata_kss_bc", "fixture_oracle", "audited_external"),
    )
    parser.add_argument("--reference-key-sha256", required=True)
    parser.add_argument("--output", required=True)
    return parser.parse_args(argv)


if __name__ == "__main__":
    try:
        sys.exit(build(parse_args()))
    except (BenchmarkError, KeyError, OSError, ValueError) as exc:
        print(f"KSS MATLAB SCALE CASE FAIL: {exc}", file=sys.stderr)
        sys.exit(2)

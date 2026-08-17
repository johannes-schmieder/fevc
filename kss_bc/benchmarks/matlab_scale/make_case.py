#!/usr/bin/env python3
"""Create a checksum-bound MATLAB scale case without reading row data."""

import argparse
import sys
from pathlib import Path

from common import (
    CASE_SCHEMA,
    BenchmarkError,
    atomic_write_json,
    hash_value,
    load_json,
    read_one_row,
    require,
    sha256_file,
    validate_case,
    validate_preparation_acceptance,
    validate_preparation_receipt,
    validate_reference_record,
)
from validate_scc_job import revalidate_acceptance_sources
from verify_case import inventory_hash, runtime_tree


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


def build(args):
    contract_path = Path(args.contract)
    input_path = Path(args.input)
    reference_path = Path(args.reference_aggregate)
    preparation_path = Path(args.preparation_receipt)
    preparation_acceptance_path = Path(args.preparation_acceptance)
    require(contract_path.is_file(), "source contract is missing")
    require(input_path.is_file(), "input CSV is missing")
    require(reference_path.is_file(), "reference aggregate is missing")
    require(preparation_path.is_file(), "preparation wrapper receipt is missing")
    require(preparation_acceptance_path.is_file(), "preparation SCC acceptance is missing")
    contract = load_json(contract_path)
    preparation_sha = sha256_file(preparation_path)
    preparation = validate_preparation_receipt(load_json(preparation_path))
    preparation_acceptance = load_json(preparation_acceptance_path)
    validate_preparation_acceptance(preparation_acceptance, preparation_sha, preparation)
    revalidate_acceptance_sources(
        preparation_acceptance_path,
        stage="prepare",
        job_dir=preparation_path.parent,
    )
    require(args.sample_mode == "fixed", "prepared-case builder accepts fixed samples only")
    require(args.label == preparation["label"], "case label differs from preparation")
    require(args.scale == preparation["scale"], "case scale differs from preparation")
    require(args.topology == preparation["topology"], "case topology differs from preparation")
    require(args.source_commit == preparation["source_commit"], "source commit differs from preparation")
    require(args.bundle_sha256 == preparation["bundle_sha256"], "bundle differs from preparation")
    require(
        sha256_file(input_path) == preparation["prepared_input_sha256"],
        "input CSV differs from preparation",
    )
    reference = read_one_row(reference_path)
    reference_binding = validate_reference_record(
        reference,
        preparation,
        probes=args.probes,
        seed=args.seed,
    )
    require(
        reference_binding["provenance"]["preparation_receipt_sha256"] == preparation_sha
        and reference_binding["provenance"]["preparation_acceptance_sha256"]
        == sha256_file(preparation_acceptance_path),
        "reference receipt does not bind the accepted preparation bytes",
    )
    source = source_identity(args.matlab_root, contract)
    source.update(
        {
            "source_commit": hash_value(args.source_commit, "source commit", 40),
            "bundle_sha256": hash_value(args.bundle_sha256, "bundle SHA-256"),
            "benchmark_contract_sha256": sha256_file(contract_path),
        }
    )
    reference_sample = {
        "kind": reference_binding["kind"],
        "receipt_sha256": sha256_file(reference_path),
        "receipt_schema": reference_binding["receipt_schema"],
        "label": reference_binding["label"],
        "source_commit": reference_binding["source_commit"],
        "bundle_sha256": reference_binding["bundle_sha256"],
        "input_bindings": reference_binding["input_bindings"],
        "retained_key_sha256": reference_binding["retained_key_sha256"],
        **reference_binding["dimensions"],
        "estimator": reference_binding["estimator"],
        "plugin": reference_binding["plugin"],
        "provenance": reference_binding["provenance"],
    }
    case = {
        "schema": CASE_SCHEMA,
        "label": args.label,
        "scale": args.scale,
        "topology": args.topology,
        "sample_mode": args.sample_mode,
        "seed": args.seed,
        "probes": args.probes,
        "warm_repetitions": args.warm_repetitions,
        "source": source,
        "input": {
            "sha256": preparation["prepared_input_sha256"],
            **preparation["dimensions"],
            "id_contract": contract["input_id_contract"],
            "order_contract": contract["input_order_contract"],
        },
        "preparation": {
            "receipt_sha256": preparation_sha,
            "acceptance_sha256": sha256_file(preparation_acceptance_path),
            **preparation,
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
    parser.add_argument("--preparation-receipt", required=True)
    parser.add_argument("--preparation-acceptance", required=True)
    parser.add_argument("--reference-aggregate", required=True)
    parser.add_argument("--output", required=True)
    return parser.parse_args(argv)


if __name__ == "__main__":
    try:
        sys.exit(build(parse_args()))
    except (BenchmarkError, KeyError, OSError, TypeError, ValueError) as exc:
        print(f"KSS MATLAB SCALE CASE FAIL: {exc}", file=sys.stderr)
        sys.exit(2)

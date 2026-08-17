#!/usr/bin/env python3
"""Verify a source-bound MATLAB scale case before launching MATLAB."""

import argparse
import hashlib
import sys
import time
from pathlib import Path

from common import (
    BenchmarkError,
    atomic_write_json,
    hash_value,
    load_json,
    require,
    sha256_file,
    validate_case,
    validate_preparation_acceptance,
    validate_preparation_receipt,
)
from validate_scc_job import revalidate_acceptance_sources


def inventory_hash(root, paths, display_paths=None):
    root = Path(root)
    if display_paths is None:
        display_paths = paths
    require(len(paths) == len(display_paths), "inventory path mismatch")
    digest = hashlib.sha256()
    for relative, display in zip(paths, display_paths, strict=False):
        item = root / relative
        require(item.is_file(), f"missing maintained source: {relative}")
        digest.update((f"{sha256_file(item)}  {display}\n").encode())
    return digest.hexdigest()


def runtime_tree(root, roots):
    root = Path(root)
    paths = []
    for subtree in roots:
        base = root / subtree
        require(base.is_dir(), f"missing maintained subtree: {subtree}")
        for item in base.rglob("*"):
            require(not item.is_symlink(), f"maintained runtime tree contains a symlink: {item}")
            if item.is_file():
                paths.append(item.relative_to(root).as_posix())
    paths.sort()
    return paths, inventory_hash(root, paths)


def verify(args):
    started = time.monotonic()
    contract_path = Path(args.contract)
    case_path = Path(args.case)
    input_path = Path(args.input)
    preparation_path = Path(args.preparation_receipt)
    preparation_acceptance_path = Path(args.preparation_acceptance)
    matlab_root = Path(args.matlab_root)
    record = {
        "schema": "kss_matlab_scale_identity_v1",
        "status": "FAIL",
        "failure_code": "KSS_MATLAB_SCALE_IDENTITY_INCOMPLETE",
        "failure_message": "identity verification did not finish",
        "case_path": str(case_path),
        "input_path": str(input_path),
        "preparation_receipt_path": str(preparation_path),
        "preparation_acceptance_path": str(preparation_acceptance_path),
        "matlab_root": str(matlab_root),
    }
    try:
        require(contract_path.is_file(), "source contract is missing")
        require(case_path.is_file(), "case binding is missing")
        require(input_path.is_file(), "benchmark input is missing")
        require(preparation_path.is_file(), "preparation wrapper receipt is missing")
        require(preparation_acceptance_path.is_file(), "preparation acceptance is missing")
        require(matlab_root.is_dir(), "maintained MATLAB root is missing")
        contract_sha = sha256_file(contract_path)
        case_sha = sha256_file(case_path)
        preparation_sha = sha256_file(preparation_path)
        preparation_acceptance_sha = sha256_file(preparation_acceptance_path)
        if args.measured_input_sha256:
            require(
                len(args.measured_input_sha256) == 64
                and all(item in "0123456789abcdef" for item in args.measured_input_sha256),
                "invalid wrapper-measured input checksum",
            )
            input_sha = args.measured_input_sha256
            input_hash_source = "wrapper_sha256sum"
        else:
            input_sha = sha256_file(input_path)
            input_hash_source = "python_sha256"
        require(contract_sha == args.contract_sha256, "source contract checksum mismatch")
        require(case_sha == args.case_sha256, "case binding checksum mismatch")
        contract = load_json(contract_path)
        case = validate_case(load_json(case_path), contract)
        preparation = validate_preparation_receipt(load_json(preparation_path))
        preparation_acceptance = load_json(preparation_acceptance_path)
        validate_preparation_acceptance(preparation_acceptance, preparation_sha, preparation)
        revalidate_acceptance_sources(
            args.preparation_acceptance_source,
            stage="prepare",
            job_dir=Path(args.preparation_acceptance_source).parent,
            expected_acceptance_sha256=preparation_acceptance_sha,
        )
        source = case["source"]
        require(
            source["benchmark_contract_sha256"] == contract_sha,
            "case does not bind the source contract",
        )
        require(case["input"]["sha256"] == input_sha, "input checksum mismatch")
        require(
            case["preparation"]["receipt_sha256"] == preparation_sha
            and case["preparation"]["acceptance_sha256"] == preparation_acceptance_sha,
            "case does not bind the preparation evidence",
        )
        for field in (
            "receipt_schema",
            "label",
            "scale",
            "topology",
            "source_commit",
            "bundle_sha256",
            "source_input_sha256",
            "prepared_input_sha256",
            "retained_key_sha256",
            "dimensions",
            "source_dimensions",
        ):
            require(
                case["preparation"].get(field) == preparation[field],
                f"case/preparation field changed: {field}",
            )
        executing_source_commit = hash_value(
            args.executing_source_commit, "executing source commit", 40
        )
        executing_bundle_sha = hash_value(
            args.executing_bundle_sha256, "executing bundle SHA-256"
        )
        require(
            source["source_commit"] == executing_source_commit,
            "executing source commit differs from case",
        )
        require(
            source["bundle_sha256"] == executing_bundle_sha,
            "executing bundle differs from case",
        )

        core_path = matlab_root / contract["core"]["relative_path"]
        cmg_path = matlab_root / contract["cmg_entry"]["relative_path"]
        core_sha = sha256_file(core_path)
        cmg_sha = sha256_file(cmg_path)
        newline_bytes = core_path.read_bytes().count(b"\n")
        hierarchy_paths = contract["hierarchy_sources"]
        hierarchy_display = [Path(item).name for item in hierarchy_paths]
        hierarchy_sha = inventory_hash(matlab_root, hierarchy_paths, hierarchy_display)
        solver_paths = contract["solver_sources"]
        solver_display = [
            item[len("CMG/") :] if item.startswith("CMG/") else item for item in solver_paths
        ]
        solver_sha = inventory_hash(matlab_root, solver_paths, solver_display)
        runtime_paths, runtime_sha = runtime_tree(matlab_root, contract["runtime_tree"]["roots"])

        require(core_sha == source["matlab_core_sha256"], "maintained core checksum mismatch")
        require(
            newline_bytes == contract["core"]["newline_bytes"],
            "maintained core newline count changed",
        )
        require(
            cmg_sha == source["matlab_cmg_entry_sha256"], "maintained CMG entry checksum mismatch"
        )
        require(
            hierarchy_sha == source["matlab_hierarchy_sha256"],
            "maintained hierarchy checksum mismatch",
        )
        require(solver_sha == source["matlab_solver_sha256"], "maintained solver checksum mismatch")
        require(
            len(runtime_paths) == contract["runtime_tree"]["file_count"],
            "maintained runtime file count changed",
        )
        require(
            runtime_sha == source["matlab_runtime_tree_sha256"],
            "maintained runtime tree checksum mismatch",
        )

        record.update(
            {
                "status": "PASS",
                "failure_code": "NONE",
                "failure_message": "NONE",
                "label": case["label"],
                "scale": case["scale"],
                "topology": case["topology"],
                "sample_mode": case["sample_mode"],
                "case_sha256": case_sha,
                "contract_sha256": contract_sha,
                "input_sha256": input_sha,
                "input_hash_source": input_hash_source,
                "preparation_receipt_sha256": preparation_sha,
                "preparation_acceptance_sha256": preparation_acceptance_sha,
                "preparation_retained_key_sha256": preparation["retained_key_sha256"],
                "source_input_sha256": preparation["source_input_sha256"],
                "executing_source_commit": executing_source_commit,
                "executing_bundle_sha256": executing_bundle_sha,
                "matlab_core_sha256": core_sha,
                "matlab_core_newline_bytes": newline_bytes,
                "matlab_cmg_entry_sha256": cmg_sha,
                "matlab_hierarchy_sha256": hierarchy_sha,
                "matlab_solver_sha256": solver_sha,
                "matlab_runtime_tree_sha256": runtime_sha,
                "matlab_runtime_file_count": len(runtime_paths),
            }
        )
    except (BenchmarkError, KeyError, OSError, TypeError, ValueError) as exc:
        record["failure_code"] = "KSS_MATLAB_SCALE_IDENTITY_REJECTED"
        record["failure_message"] = str(exc)
    record["verification_seconds"] = time.monotonic() - started
    atomic_write_json(args.output, record)
    return 0 if record["status"] == "PASS" else 2


def parse_args(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument("--case", required=True)
    parser.add_argument("--case-sha256", required=True)
    parser.add_argument("--contract", required=True)
    parser.add_argument("--contract-sha256", required=True)
    parser.add_argument("--input", required=True)
    parser.add_argument("--measured-input-sha256")
    parser.add_argument("--preparation-receipt", required=True)
    parser.add_argument("--preparation-acceptance", required=True)
    parser.add_argument("--preparation-acceptance-source", required=True)
    parser.add_argument("--executing-source-commit", required=True)
    parser.add_argument("--executing-bundle-sha256", required=True)
    parser.add_argument("--matlab-root", required=True)
    parser.add_argument("--output", required=True)
    return parser.parse_args(argv)


if __name__ == "__main__":
    sys.exit(verify(parse_args()))

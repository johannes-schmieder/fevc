#!/usr/bin/env python3
"""Fail-closed pre-qsub gate for the fixed-retained MATLAB benchmark."""

import argparse
import sys
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


def verify(args):
    record = {
        "schema": "kss_matlab_scale_submission_gate_v1",
        "status": "FAIL",
        "failure_code": "KSS_MATLAB_SCALE_SUBMISSION_GATE_INCOMPLETE",
        "failure_message": "submission gate did not finish",
    }
    try:
        case_path = Path(args.case)
        contract_path = Path(args.contract)
        input_path = Path(args.input)
        preparation_path = Path(args.preparation_receipt)
        acceptance_path = Path(args.preparation_acceptance)
        for path, label in (
            (case_path, "case"),
            (contract_path, "source contract"),
            (input_path, "prepared input"),
            (preparation_path, "preparation wrapper receipt"),
            (acceptance_path, "preparation SCC acceptance"),
        ):
            require(path.is_file(), f"{label} is missing")

        case_sha = sha256_file(case_path)
        contract_sha = sha256_file(contract_path)
        input_sha = sha256_file(input_path)
        preparation_sha = sha256_file(preparation_path)
        acceptance_sha = sha256_file(acceptance_path)
        require(case_sha == args.case_sha256, "case checksum mismatch")
        require(contract_sha == args.contract_sha256, "source contract checksum mismatch")
        require(input_sha == args.input_sha256, "prepared input checksum mismatch")
        contract = load_json(contract_path)
        case = validate_case(load_json(case_path), contract)
        require(case["sample_mode"] == "fixed", "fixed submitter rejects selection cases")
        require(case["label"] == args.label, "case label differs from submission")
        require(case["input"]["sha256"] == input_sha, "case input checksum mismatch")
        require(
            case["source"]["benchmark_contract_sha256"] == contract_sha,
            "case source-contract binding changed",
        )

        preparation = validate_preparation_receipt(load_json(preparation_path))
        acceptance = load_json(acceptance_path)
        validate_preparation_acceptance(acceptance, preparation_sha, preparation)
        revalidate_acceptance_sources(
            acceptance_path,
            stage="prepare",
            job_dir=preparation_path.parent,
        )
        require(
            case["preparation"]["receipt_sha256"] == preparation_sha
            and case["preparation"]["acceptance_sha256"] == acceptance_sha,
            "case does not bind preparation receipts",
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
            case["source"]["source_commit"] == executing_source_commit,
            "executing source commit differs from case",
        )
        require(
            case["source"]["bundle_sha256"] == executing_bundle_sha,
            "executing bundle differs from case",
        )
        record.update(
            {
                "status": "PASS",
                "failure_code": "NONE",
                "failure_message": "NONE",
                "label": case["label"],
                "sample_mode": case["sample_mode"],
                "case_sha256": case_sha,
                "contract_sha256": contract_sha,
                "input_sha256": input_sha,
                "preparation_receipt_sha256": preparation_sha,
                "preparation_acceptance_sha256": acceptance_sha,
                "source_commit": executing_source_commit,
                "bundle_sha256": executing_bundle_sha,
            }
        )
    except (BenchmarkError, KeyError, OSError, TypeError, ValueError) as exc:
        record["failure_code"] = "KSS_MATLAB_SCALE_SUBMISSION_GATE_REJECTED"
        record["failure_message"] = str(exc)
    atomic_write_json(args.output, record)
    return 0 if record["status"] == "PASS" else 2


def parse_args(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument("--case", required=True)
    parser.add_argument("--case-sha256", required=True)
    parser.add_argument("--contract", required=True)
    parser.add_argument("--contract-sha256", required=True)
    parser.add_argument("--input", required=True)
    parser.add_argument("--input-sha256", required=True)
    parser.add_argument("--preparation-receipt", required=True)
    parser.add_argument("--preparation-acceptance", required=True)
    parser.add_argument("--executing-source-commit", required=True)
    parser.add_argument("--executing-bundle-sha256", required=True)
    parser.add_argument("--label", required=True)
    parser.add_argument("--output", required=True)
    return parser.parse_args(argv)


if __name__ == "__main__":
    sys.exit(verify(parse_args()))

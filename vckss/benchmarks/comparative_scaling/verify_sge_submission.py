#!/usr/bin/env python3
"""Validate and record the effective SCC/SGE job specification."""

import argparse
import hashlib
import json
from pathlib import Path


class SubmissionError(RuntimeError):
    pass


def fields(text):
    parsed = {}
    for line in text.splitlines():
        if ":" not in line:
            continue
        key, value = line.split(":", 1)
        parsed[key.strip()] = value.strip()
    return parsed


def resources(value):
    parsed = {}
    for item in filter(None, (part.strip() for part in value.split(","))):
        key, separator, resource_value = item.partition("=")
        parsed[key] = resource_value if separator else "TRUE"
    return parsed


def validate(qstat_path, expected_slots, require_binding, binding_script=None):
    raw = qstat_path.read_bytes()
    text = raw.decode("utf-8")
    values = fields(text)
    hard = resources(values.get("hard resource_list", ""))
    soft = resources(values.get("soft resource_list", ""))

    for key in ("hard queue_list", "master hard queue_list"):
        if values.get(key):
            raise SubmissionError(f"unexpected queue restriction: {key}={values[key]}")
    forbidden = {"buyin", "exclusive", "cpu_type", "cpu_arch", "arch", "hostname"}
    bad_hard = sorted(forbidden.intersection(hard))
    if bad_hard:
        raise SubmissionError(f"unexpected hard resources: {bad_hard}")
    expected_hard = ({"h_rt": "7200", "mem_per_core": "4G", "no_gpu": "TRUE"}
                     if expected_slots == 4 else
                     {"h_rt": "43200", "mem_per_core": "8G", "no_gpu": "TRUE"})
    if hard != expected_hard:
        raise SubmissionError(
            f"expected exact hard resources {expected_hard}; observed {hard}"
        )
    if soft != {"buyin": "TRUE"}:
        raise SubmissionError(
            "expected only SCC global-JSV soft buyin=TRUE injection; "
            f"observed {soft}"
        )

    pe = values.get("parallel environment", "")
    accepted_pes = {
        f"omp range: {expected_slots}",
        f"omp{expected_slots} range: {expected_slots}",
    }
    if pe not in accepted_pes:
        raise SubmissionError(f"expected omp {expected_slots}; observed {pe!r}")
    binding = values.get("binding", "")
    binding_request = None
    binding_evidence = None
    binding_script_sha256 = None
    if require_binding:
        binding_request = f"linear:{expected_slots}"
        if binding:
            if binding_request not in binding:
                raise SubmissionError(
                    f"expected {binding_request} binding; observed {binding!r}"
                )
            binding_evidence = "QSTAT"
        else:
            if binding_script is None:
                raise SubmissionError(
                    "SCC qstat omitted binding and no immutable binding script was supplied"
                )
            script_raw = binding_script.read_bytes()
            directive = f"#$ -binding {binding_request}"
            if script_raw.decode("utf-8").splitlines().count(directive) != 1:
                raise SubmissionError(
                    f"immutable job script does not contain exactly one {directive!r}"
                )
            if values.get("script_file") != str(binding_script):
                raise SubmissionError("qstat script path does not match binding script")
            binding_script_sha256 = hashlib.sha256(script_raw).hexdigest()
            binding_evidence = "SOURCE_DIRECTIVE_RUNTIME_AFFINITY_REQUIRED"

    return {
        "schema": "VCKSS-SGE-EFFECTIVE-SUBMISSION-V1",
        "status": "PASS",
        "qstat_sha256": hashlib.sha256(raw).hexdigest(),
        "parallel_environment": pe,
        "parallel_environment_request": f"omp {expected_slots}",
        "parallel_environment_resolution": (
            "SCC_SLOT_SPECIFIC_ALIAS" if pe.startswith(f"omp{expected_slots} ")
            else "REQUESTED_PE"
        ),
        "binding": binding or None,
        "binding_request": binding_request,
        "binding_evidence": binding_evidence,
        "binding_script_sha256": binding_script_sha256,
        "runtime_affinity_gate_required": require_binding,
        "hard_resources": hard,
        "soft_resources": soft,
        "queue_constraint": None,
        "buyin_requested_by_harness": False,
        "soft_buyin_injection": "SCC_GLOBAL_JSV_MANDATORY",
        "scheduler_eligibility": "ALL_HOSTS_MATCHING_HARD_RESOURCES",
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--qstat", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--expected-slots", type=int, required=True)
    parser.add_argument("--require-binding", action="store_true")
    parser.add_argument("--binding-script", type=Path)
    args = parser.parse_args()
    receipt = validate(
        args.qstat,
        args.expected_slots,
        args.require_binding,
        args.binding_script,
    )
    args.output.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()

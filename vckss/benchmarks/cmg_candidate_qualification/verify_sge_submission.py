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


def validate(qstat_path, expected_slots, require_binding):
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
    if soft != {"buyin": "TRUE"}:
        raise SubmissionError(
            "expected only SCC global-JSV soft buyin=TRUE injection; "
            f"observed {soft}"
        )

    pe = values.get("parallel environment", "")
    if f"omp range: {expected_slots}" not in pe:
        raise SubmissionError(f"expected omp {expected_slots}; observed {pe!r}")
    binding = values.get("binding", "")
    if require_binding and f"linear:{expected_slots}" not in binding:
        raise SubmissionError(
            f"expected linear:{expected_slots} binding; observed {binding!r}"
        )

    return {
        "schema": "VCKSS-SGE-EFFECTIVE-SUBMISSION-V1",
        "status": "PASS",
        "qstat_sha256": hashlib.sha256(raw).hexdigest(),
        "parallel_environment": pe,
        "binding": binding or None,
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
    args = parser.parse_args()
    receipt = validate(args.qstat, args.expected_slots, args.require_binding)
    args.output.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()

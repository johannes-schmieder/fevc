"""Strict, bounded evidence contract for full-CMG Windows qualification."""
from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path
import re
from statistics import median

CMG = "e5613e81fb3b03c78de98658b372646fa802aabb"
COMMON = ("pe_import_export", "clean_install", "lifecycle", "point_mean",
          "projection_mean", "registry_idle")
LEGACY = ("observation_component", "individual_component", "match_component",
          "pooled_deletion", "mcse_modes", "mcse_attachments", "centering_mean",
          "centering_exact", "centering_jla", "centering_options")
GROUPS = {
    "smoke": ("cmg_boundary",),
    "legacy": LEGACY,
    "cmg_point": ("cmg_boundary", "cmg_point_1", "cmg_point_2",
                  "cmg_controlled_1", "cmg_controlled_2", "cmg_pooled",
                  "cmg_mcse_1", "cmg_mcse_2", "cmg_v2"),
    "cmg_attachments": ("cmg_attachments_1", "cmg_attachments_2",
                        'cmg_pooled_common_1', 'cmg_pooled_common_2', 'cmg_pooled_leverage_1', 'cmg_pooled_leverage_2'),
    "cmg_lifecycle": ("cmg_lifecycle_1", "cmg_lifecycle_2"),
    "performance": ("complete_command_measurements",),
    "installation": ("replacement_install", "cmg_boundary"),
}
REQUIRED = ("smoke", "legacy", "cmg_point", "cmg_attachments",
            "cmg_lifecycle", "installation")
IDENTITY_KEYS = {"production_commit", "harness_sha256", "candidate_sha256",
                 "qualification_group", "campaign_sha256", "performance_arm",
                 "performance_block"}
CHECK_KEYS = {"schema", "gates", "stata_version", "stata_flavor", "runtime_profile",
              "production_commit", "harness_sha256", "binary_sha256", "status",
              "qualification_group", "campaign_sha256", "performance_arm",
              "performance_block", "completed_tests", "measurements"}


def strict_json(data: bytes | str):
    def pairs(items):
        value = {}
        for key, item in items:
            if key in value:
                raise ValueError("duplicate JSON key")
            value[key] = item
        return value
    def invalid(_):
        raise ValueError("nonfinite JSON")
    return json.loads(data, object_pairs_hook=pairs, parse_constant=invalid)


def hex_value(value, length=64):
    return isinstance(value, str) and re.fullmatch("[0-9a-f]{" + str(length) + "}", value) is not None


def validate_selection(identity):
    group = identity.get("qualification_group")
    if group not in GROUPS:
        raise ValueError("qualification group")
    if not hex_value(identity.get("campaign_sha256")):
        raise ValueError("campaign identity")
    arm = identity.get("performance_arm")
    block = identity.get("performance_block")
    if arm not in ("candidate", "incumbent") or type(block) is not int:
        raise ValueError("performance identity")
    if group == "performance":
        if not 0 <= block < 6:
            raise ValueError("performance block")
    elif arm != "candidate" or block != 0:
        raise ValueError("non-performance selection")
    if identity["runtime_profile"] != ("smoke" if group == "smoke" else "full"):
        raise ValueError("group/profile mismatch")
    return group


def validate_measurements(rows, group):
    if group != "performance":
        if rows != []:
            raise ValueError("unexpected measurements")
        return
    # Twelve complete-command observations: six frozen cases, one/two workers.
    if not isinstance(rows, list) or len(rows) != 12:
        raise ValueError("measurement count")
    seen = set()
    for row in rows:
        if not isinstance(row, list) or len(row) != 14:
            raise ValueError("measurement width")
        case, workers, elapsed, *values = row
        if type(case) is not int or not 0 <= case < 6 or type(workers) is not int or workers not in (1, 2):
            raise ValueError("measurement cell")
        if (case, workers) in seen:
            raise ValueError("duplicate measurement")
        seen.add((case, workers))
        if any(type(x) not in (int, float) or not math.isfinite(x) for x in row):
            raise ValueError("nonfinite measurement")
        if elapsed < 0 or any(x < 0 for x in row[7:11]):
            raise ValueError("negative time/MCSE")
        residual, limit, retained = row[11:]
        if not 0 <= residual <= limit or limit <= 0 or retained <= 0 or retained != int(retained):
            raise ValueError("scientific measurement gate")


def validate_checks(checks, identity, binary_sha256, rust_scope):
    group = validate_selection(identity)
    if not hex_value(identity.get("production_commit"), 40) or not hex_value(identity.get("harness_sha256")) or not hex_value(binary_sha256):
        raise ValueError("source/binary identity")
    scopes = {"HOSTED_CI_PASS", "PRIVATE_BUILD_AND_TEST_PASS"}
    if group == "smoke":
        scopes.add("PRIVATE_BUILD_ONLY_SMOKE")
    if rust_scope not in scopes:
        raise ValueError("Rust qualification scope")
    if not isinstance(checks, dict) or set(checks) != CHECK_KEYS:
        raise ValueError("checks schema")
    completed = list(COMMON + GROUPS[group])
    expected_gates = {name: "PASS" for name in completed}
    expected_gates.update({name: rust_scope for name in ("rust_workspace", "rust_backend")})
    expected = {
        "schema": "FEVC-WINDOWS-CHECKS-V4", "gates": expected_gates,
        "stata_version": "19", "stata_flavor": "MP",
        "runtime_profile": identity["runtime_profile"],
        "production_commit": identity["production_commit"],
        "harness_sha256": identity["harness_sha256"],
        "binary_sha256": binary_sha256, "status": "PASS",
        "qualification_group": group,
        "campaign_sha256": identity["campaign_sha256"],
        "performance_arm": identity["performance_arm"],
        "performance_block": identity["performance_block"],
        "completed_tests": completed,
    }
    if any(checks[key] != value for key, value in expected.items()):
        raise ValueError("checks coverage/identity")
    validate_measurements(checks["measurements"], group)
    return checks


def aggregate(records):
    if not records:
        raise ValueError("missing qualification")
    for record in records:
        validate_checks(record, record, record["binary_sha256"], record["gates"]["rust_workspace"])
    if len({record["harness_sha256"] for record in records}) != 1:
        raise ValueError("mixed harnesses")
    campaigns = {record["campaign_sha256"] for record in records}
    if len(campaigns) != 1:
        raise ValueError("mixed campaigns")
    keys = [(r["qualification_group"], r["performance_arm"], r["performance_block"]) for r in records]
    if len(keys) != len(set(keys)):
        raise ValueError("duplicate qualification")
    required = {(group, "candidate", 0) for group in REQUIRED}
    required.update(("performance", arm, block) for arm in ("candidate", "incumbent") for block in range(6))
    if set(keys) != required:
        raise ValueError("incomplete campaign")
    for arm in ("candidate", "incumbent"):
        selected = [r for r in records if r["performance_arm"] == arm]
        if len({r["binary_sha256"] for r in selected}) != 1:
            raise ValueError("mixed binaries")
        if len({r["production_commit"] for r in selected}) != 1:
            raise ValueError("mixed production sources")
    indexed = {(r["performance_arm"], r["performance_block"]): {
        (row[0], row[1]): row for row in r["measurements"]
    } for r in records if r["qualification_group"] == "performance"}
    cells = []
    for case in range(6):
        for workers in (1, 2):
            pairs = [(indexed[("incumbent", block)][(case, workers)],
                      indexed[("candidate", block)][(case, workers)])
                     for block in range(6)]
            science = True
            worst_scaled_difference = 0.0
            for incumbent, candidate in pairs:
                science &= incumbent[13] == candidate[13]
                for target in range(4):
                    before, after = incumbent[3 + target], candidate[3 + target]
                    scale = max(1.0, abs(before), abs(after))
                    bound = max(1e-8 * scale, 0.1 * max(incumbent[7 + target], candidate[7 + target]))
                    difference = abs(after - before)
                    science &= difference <= bound
                    worst_scaled_difference = max(worst_scaled_difference, difference / bound)
            before_ms = median(pair[0][2] for pair in pairs)
            after_ms = median(pair[1][2] for pair in pairs)
            allowance_ms = max(0.05 * before_ms, 1.0)
            cells.append({"case": case, "workers": workers,
                          "incumbent_median_ms": before_ms,
                          "candidate_median_ms": after_ms,
                          "allowance_ms": allowance_ms,
                          "performance": "PASS" if after_ms <= before_ms + allowance_ms else "FAIL",
                          "science": "PASS" if science else "FAIL",
                          "largest_difference_over_scientific_bound": worst_scaled_difference,
                          "incumbent_ms": [pair[0][2] for pair in pairs],
                          "candidate_ms": [pair[1][2] for pair in pairs]})
    passed = all(cell["performance"] == cell["science"] == "PASS" for cell in cells)
    return {"status": "PASS" if passed else "FAIL", "groups": len(records),
            "campaign_sha256": campaigns.pop(), "cells": cells,
            "comparison": "six fixed paired blocks; common-draw registered scientific bound; complete-command medians",
            "retries_or_exclusions": 0}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("receipts", nargs="+", type=Path)
    args = parser.parse_args()
    records = [strict_json(path.read_bytes()) for path in args.receipts]
    # Collector validation precedes aggregation; repeat coverage/shape validation.
    for record in records:
        identity = {**record, "candidate_sha256": record["binary_sha256"]}
        validate_checks(record, identity, record["binary_sha256"], record["gates"]["rust_workspace"])
    result = aggregate(records)
    print(json.dumps(result, sort_keys=True))
    raise SystemExit(0 if result["status"] == "PASS" else 1)


if __name__ == "__main__":
    main()

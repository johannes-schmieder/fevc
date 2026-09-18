#!/usr/bin/env python3
"""Validate numerical parity and the frozen performance gate after completion."""

from __future__ import annotations

import argparse
import csv
import json
import math
import statistics
from collections import defaultdict
from pathlib import Path

from campaign import PROFILES, SEEDS


def relative(left: float, right: float) -> float:
    return abs(left - right) / max(abs(left), abs(right), 1.0)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run_dir", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    if args.output.exists():
        parser.error("output is write-once")
    manifest = json.loads((args.run_dir / "input/development/development-manifest.json").read_text())
    if manifest["status"] != "FROZEN_DEVELOPMENT_READY":
        raise SystemExit("development manifest is not frozen")
    rows, timed, warmups = [], 0, 0
    for task in range(1, 17):
        task_dir = args.run_dir / f"campaign/tasks/task-{task:02d}"
        if not (task_dir / "task.tsv").is_file():
            raise SystemExit(f"missing task receipt {task}")
        for result in sorted(task_dir.glob("*/result.tsv")):
            parsed = list(csv.DictReader(result.open(), delimiter="\t"))
            if len(parsed) != 1:
                raise SystemExit(f"invalid result row count: {result}")
            row = parsed[0]
            if row["schema"] != "FEVC-OPTIMIZATION-DEVELOPMENT-CALL-V1":
                raise SystemExit(f"invalid result schema: {result}")
            profile = row["profile"]
            if row["input_sha256"] != manifest["input_hashes"][profile]:
                raise SystemExit(f"input hash mismatch: {result}")
            if int(row["warmup"]):
                warmups += 1
            else:
                timed += 1
            if float(row["max_residual"]) > float(row["acceptance"]):
                raise SystemExit(f"residual gate failed: {result}")
            if abs(float(row["target_identity"])) > 1e-10:
                raise SystemExit(f"target identity failed: {result}")
            if row["variant"] == "candidate":
                point = PROFILES.index(profile) < 5
                valid_mode = (
                    row["execution_mode"] in {"direct_attachments", "legacy"}
                    if point
                    else row["execution_mode"] == "diagonal_queue"
                )
                valid_route = row["route"] == ("CMG" if point else "DIAGONAL")
                if not valid_mode or not valid_route:
                    raise SystemExit(f"candidate route mismatch: {result}")
            row["_path"] = str(result)
            rows.append(row)
    if (timed, warmups, len(rows)) != (96, 32, 128):
        raise SystemExit(f"call accounting mismatch: timed={timed} warmups={warmups} rows={len(rows)}")

    paired = defaultdict(dict)
    for row in rows:
        if int(row["warmup"]):
            continue
        key = (row["profile"], int(row["threads"]), int(row["seed"]))
        if int(row["seed"]) not in SEEDS or row["variant"] in paired[key]:
            raise SystemExit(f"invalid paired key: {row['_path']}")
        paired[key][row["variant"]] = row
    ratios, profile_ratios = [], defaultdict(list)
    for key, pair in paired.items():
        if set(pair) != {"baseline", "candidate"}:
            raise SystemExit(f"unpaired call: {key}")
        for row_index in range(1, 5):
            for column in range(1, 5):
                field = f"result_{row_index}_{column}"
                if relative(float(pair["baseline"][field]), float(pair["candidate"][field])) > 1e-7:
                    raise SystemExit(f"numerical parity failed: {key} {field}")
        ratio = float(pair["candidate"]["command_seconds"]) / float(pair["baseline"]["command_seconds"])
        profile_ratios[key[0]].append(ratio)
        if PROFILES.index(key[0]) < 5:
            ratios.append(ratio)
    geometric_ratio = math.exp(statistics.fmean(math.log(value) for value in ratios))
    medians = {profile: statistics.median(values) for profile, values in profile_ratios.items()}
    gate_pass = geometric_ratio <= 0.97 and all(value <= 1.05 for value in medians.values())
    report = {
        "schema": "FEVC-OPTIMIZATION-DEVELOPMENT-DECISION-V1",
        "status": "PASS" if gate_pass else "FAIL",
        "timed_calls": timed,
        "warmups": warmups,
        "point_geometric_candidate_over_baseline": geometric_ratio,
        "point_geometric_improvement": 1.0 - geometric_ratio,
        "profile_median_candidate_over_baseline": medians,
        "required_point_improvement": 0.03,
        "maximum_profile_regression": 0.05,
    }
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    print(json.dumps(report, sort_keys=True))
    return 0 if gate_pass else 1


if __name__ == "__main__":
    raise SystemExit(main())

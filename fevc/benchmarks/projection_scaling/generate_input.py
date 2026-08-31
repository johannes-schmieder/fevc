#!/usr/bin/env python3
"""Generate one deterministic all-mover projection-scaling input."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
from pathlib import Path


SIZES = (6000, 24000, 96000)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("rows", type=int, choices=SIZES)
    parser.add_argument("output", type=Path)
    parser.add_argument("receipt", type=Path)
    args = parser.parse_args()

    workers = args.rows // 6
    firms = workers // 2
    if workers * 6 != args.rows or firms * 2 != workers:
        raise ValueError("registered dimensions changed")

    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle, lineterminator="\n")
        writer.writerow(("observation_key", "worker", "firm", "period", "y", "z1", "z2"))
        observation = 0
        for worker in range(1, workers + 1):
            alpha = math.sin(worker * 0.017) + 0.5 * math.cos(worker * 0.031)
            for period in range(6):
                observation += 1
                firm = ((worker - 1 + period // 2) % firms) + 1
                psi = 0.7 * math.cos(firm * 0.023) - 0.2 * math.sin(firm * 0.041)
                noise = 0.2 * math.sin(observation * 0.37) + 0.05 * math.cos(worker + 3 * period)
                y = 1.0 + alpha + psi + noise
                z1 = math.sin(worker / 5.0) + math.cos(firm / 3.0)
                z2 = math.cos(worker / 7.0) - math.sin(firm / 4.0)
                writer.writerow((
                    observation,
                    worker,
                    firm,
                    period + 1,
                    f"{y:.17g}",
                    f"{z1:.17g}",
                    f"{z2:.17g}",
                ))

    digest = hashlib.sha256(args.output.read_bytes()).hexdigest()
    receipt = {
        "schema": "FEVC-PROJECTION-SCALING-INPUT-V1",
        "status": "PASS",
        "rows": args.rows,
        "workers": workers,
        "firms": firms,
        "spells_per_worker": 6,
        "firms_per_worker": 3,
        "all_movers": True,
        "sha256": digest,
    }
    args.receipt.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(receipt, sort_keys=True))


if __name__ == "__main__":
    main()

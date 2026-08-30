#!/usr/bin/env python3
"""Generate a deterministic six-spell, three-firm-per-worker AKM fixture."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
from pathlib import Path

from common import PROBES


def generate(rows: int, output: Path, receipt_path: Path) -> dict[str, object]:
    if rows not in PROBES:
        raise ValueError("unregistered row count")
    workers = rows // 6
    firms = workers // 2
    if workers * 6 != rows or firms * 2 != workers:
        raise ValueError("registered dimensions changed")
    output.parent.mkdir(parents=True, exist_ok=True)
    digest = hashlib.sha256()
    with output.open("wb") as raw:
        class DigestWriter:
            def write(self, value: str) -> int:
                encoded = value.encode("utf-8")
                digest.update(encoded)
                return raw.write(encoded)

        writer = csv.writer(DigestWriter(), lineterminator="\n")
        writer.writerow(("observation_key", "worker", "firm", "period", "y", "z1", "z2"))
        observation = 0
        for worker in range(1, workers + 1):
            alpha = math.sin(worker * 0.017) + 0.5 * math.cos(worker * 0.031)
            for period in range(6):
                observation += 1
                firm = ((worker - 1 + period // 2) % firms) + 1
                psi = 0.7 * math.cos(firm * 0.023) - 0.2 * math.sin(firm * 0.041)
                noise = 0.2 * math.sin(observation * 0.37) + 0.05 * math.cos(worker + 3 * period)
                writer.writerow((
                    observation, worker, firm, period + 1,
                    f"{1.0 + alpha + psi + noise:.17g}",
                    f"{math.sin(worker / 5.0) + math.cos(firm / 3.0):.17g}",
                    f"{math.cos(worker / 7.0) - math.sin(firm / 4.0):.17g}",
                ))
    receipt = {
        "schema": "VCKSS-PROJECTION-AKM-INPUT-V1",
        "status": "PASS",
        "rows": rows,
        "workers": workers,
        "firms": firms,
        "spells_per_worker": 6,
        "firms_per_worker": 3,
        "all_movers": True,
        "sha256": digest.hexdigest(),
    }
    receipt_path.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return receipt


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("rows", type=int, choices=tuple(PROBES))
    parser.add_argument("output", type=Path)
    parser.add_argument("receipt", type=Path)
    args = parser.parse_args()
    print(json.dumps(generate(args.rows, args.output, args.receipt), sort_keys=True))


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Generate the deterministic strong_d3 benchmark panel without dependencies."""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import os
from pathlib import Path


def generate(output: Path, receipt: Path, n: int) -> dict[str, object]:
    if n not in (960, 7_680, 30_720, 122_880, 491_520):
        raise ValueError("row count outside benchmark grid")
    workers, firms, degree = n // 3, n // 120, 3
    if workers != 40 * firms:
        raise ValueError("strong_d3 dimensions changed")
    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = output.with_suffix(output.suffix + ".tmp")
    digest = hashlib.sha256()
    with temporary.open("w", encoding="utf-8", newline="") as raw:
        class HashingWriter:
            def write(self, value: str) -> int:
                encoded = value.encode("utf-8")
                digest.update(encoded)
                return raw.write(value)
        writer = csv.writer(HashingWriter(), lineterminator="\n")
        writer.writerow(("observation_key", "worker", "firm", "period", "match", "y"))
        for observation in range(1, n + 1):
            worker = (observation - 1) // degree + 1
            period = (observation - 1) % degree + 1
            layer = (worker - 1) // firms
            base = (worker - 1) % firms
            if period == 1:
                offset = 0
            elif period == 2:
                offset = 1 + layer % (firms // 4 - 1)
            else:
                offset = (firms + 2) // 3 + (97 * layer) % (firms // 4)
            firm = (base + offset) % firms + 1
            outcome = ((worker % 257) / 16 + (firm % 127) / 32 +
                       period / 64 + (observation % 13) / 128)
            writer.writerow((observation, worker, firm, period, observation,
                             format(outcome, ".17g")))
    os.replace(temporary, output)
    value = {
        "schema": "FEVC-FIVE-WAY-INPUT-V1", "status": "PASS",
        "rows": n, "workers": workers, "firms": firms,
        "degree": degree, "structure": "strong_d3",
        "unique_worker_firm": True, "all_workers_move": True,
        "sample_contract": "same_literal_match_rows_v2",
        "sha256": digest.hexdigest(),
    }
    receipt_tmp = receipt.with_suffix(receipt.suffix + ".tmp")
    receipt_tmp.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n",
                           encoding="utf-8")
    os.replace(receipt_tmp, receipt)
    return value


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--receipt", type=Path, required=True)
    parser.add_argument("--rows", type=int, required=True)
    args = parser.parse_args()
    value = generate(args.output, args.receipt, args.rows)
    print(f"FEVC_FIVE_WAY_INPUT_PASS rows={value['rows']} sha256={value['sha256']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

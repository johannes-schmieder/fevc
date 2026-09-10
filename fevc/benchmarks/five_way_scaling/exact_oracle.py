#!/usr/bin/env python3
"""Independent dense population-N exact KSS oracle for the N=960 fixture."""

from __future__ import annotations

import argparse
import csv
import json
import os
from pathlib import Path

import numpy as np


def target(a: np.ndarray, b: np.ndarray, coefficients: np.ndarray,
           x: np.ndarray, inv_xx: np.ndarray, sigma: np.ndarray) -> float:
    left, right = a @ coefficients, b @ coefficients
    left -= left.mean(); right -= right.mean()
    plugin = float(left @ right / len(left))
    ac, bc = a - a.mean(axis=0), b - b.mean(axis=0)
    q = ac.T @ bc
    xh = x @ inv_xx
    bii = np.einsum("ij,jk,ik->i", xh, q, xh, optimize=True)
    return plugin - float(bii @ sigma / len(left))


def oracle(input_path: Path) -> dict[str, object]:
    with input_path.open(encoding="utf-8", newline="") as handle:
        data = list(csv.DictReader(handle))
    n = len(data)
    pairs = {(row["worker"], row["firm"]) for row in data}
    if n != 960 or len(pairs) != n:
        raise ValueError("oracle accepts only the frozen N=960 unique-match fixture")
    worker = np.array([int(row["worker"]) for row in data], dtype=np.int64) - 1
    firm = np.array([int(row["firm"]) for row in data], dtype=np.int64) - 1
    y = np.array([float(row["y"]) for row in data], dtype=np.float64)
    nw, nf = n // 3, n // 120
    d = np.zeros((n, nw)); d[np.arange(n), worker] = 1
    f = np.zeros((n, nf - 1)); selected = firm < nf - 1
    f[np.arange(n)[selected], firm[selected]] = -1
    x = np.column_stack((d, f))
    xx = x.T @ x
    inv_xx = np.linalg.inv(xx)
    coefficients = inv_xx @ x.T @ y
    residual = y - x @ coefficients
    leverage = np.einsum("ij,jk,ik->i", x, inv_xx, x, optimize=True)
    if not np.all((leverage >= 0) & (leverage < 1 - 1e-10)):
        raise ValueError("fixture is not leave-one-match estimable")
    # The KSS block-deletion identity uses the original outcome multiplied by
    # the deleted-fit residual.  Centering belongs in Q, not in this score.
    sigma = y * residual / (1 - leverage)
    zeros_d = np.zeros_like(d); zeros_f = np.zeros_like(f)
    aw = np.column_stack((d, zeros_f)); af = np.column_stack((zeros_d, f))
    values = {
        "worker": target(aw, aw, coefficients, x, inv_xx, sigma),
        "firm": target(af, af, coefficients, x, inv_xx, sigma),
        "covariance": target(aw, af, coefficients, x, inv_xx, sigma),
    }
    values["total"] = values["worker"] + values["firm"] + 2 * values["covariance"]
    return {"schema":"FEVC-FIVE-WAY-EXACT-ORACLE-V1","status":"PASS",
            "rows":n,"workers":nw,"firms":nf,"denominator":"population_N",
            "implementation":"independent_dense_numpy_normal_equations",
            "maximum_leverage":float(leverage.max()),"targets":values}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    value = oracle(args.input)
    temporary = args.output.with_suffix(args.output.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    os.replace(temporary,args.output)
    print("FEVC_FIVE_WAY_EXACT_ORACLE_PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

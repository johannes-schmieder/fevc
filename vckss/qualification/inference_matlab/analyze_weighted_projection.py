#!/usr/bin/env python3
"""Validate compressed fweights against expansion and maintained MATLAB."""

from __future__ import annotations

import argparse
import csv
import json
import math
from pathlib import Path


PROJECTIONS = ("firm_frequency", "worker_target")
TERMS = ("_cons", "z1", "z2")


def read_rows(path: Path) -> list[dict[str, str | float]]:
    with path.open(newline="", encoding="utf-8") as handle:
        rows: list[dict[str, str | float]] = []
        for row in csv.DictReader(handle):
            clean: dict[str, str | float] = {
                key: value.strip() for key, value in row.items()
            }
            clean["value"] = float(row["value"])
            rows.append(clean)
        return rows


def lookup(rows, route, projection, kind, row="", col="") -> float:
    values = [
        item["value"]
        for item in rows
        if item["route"] == route
        and item["projection"] == projection
        and item["kind"] == kind
        and item["row"] == row
        and item["col"] == col
    ]
    if len(values) != 1:
        raise ValueError(
            f"expected one {route}/{projection}/{kind}/{row}/{col}, "
            f"found {len(values)}"
        )
    return float(values[0])


def discrepancy(left: float, right: float) -> dict[str, float]:
    absolute = abs(left - right)
    return {
        "absolute": absolute,
        "relative": absolute / max(abs(left), abs(right), 1e-30),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("vckss_csv", type=Path)
    parser.add_argument("matlab_csv", type=Path)
    parser.add_argument("output_json", type=Path)
    args = parser.parse_args()

    vckss = read_rows(args.vckss_csv)
    matlab = read_rows(args.matlab_csv)
    comparisons = []
    maxima = {
        "compressed_expanded_absolute": 0.0,
        "compressed_matlab_same_formula_absolute": 0.0,
        "compressed_matlab_lincom_coefficient_relative": 0.0,
        "compressed_matlab_lincom_se_relative": 0.0,
        "rust_exact_coefficient_relative": 0.0,
        "rust_exact_covariance_relative": 0.0,
    }

    for projection in PROJECTIONS:
        for kind in ("projection_b", "projection_V", "projection_V_naive"):
            rows = ("",) if kind == "projection_b" else TERMS
            for row in rows:
                for col in TERMS:
                    exact = lookup(vckss, "exact", projection, kind, row, col)
                    expanded = lookup(vckss, "expanded", projection, kind, row, col)
                    same_formula = lookup(
                        matlab, "same_formula", projection, kind, row, col
                    )
                    expansion_diff = discrepancy(exact, expanded)
                    matlab_diff = discrepancy(exact, same_formula)
                    maxima["compressed_expanded_absolute"] = max(
                        maxima["compressed_expanded_absolute"],
                        expansion_diff["absolute"],
                    )
                    maxima["compressed_matlab_same_formula_absolute"] = max(
                        maxima["compressed_matlab_same_formula_absolute"],
                        matlab_diff["absolute"],
                    )
                    comparisons.append(
                        {
                            "projection": projection,
                            "kind": kind,
                            "row": row,
                            "col": col,
                            "vckss_exact": exact,
                            "vckss_expanded": expanded,
                            "matlab_same_formula": same_formula,
                            "expansion": expansion_diff,
                            "matlab": matlab_diff,
                        }
                    )

        for term in TERMS:
            exact_b = lookup(vckss, "exact", projection, "projection_b", "", term)
            rust_b = lookup(vckss, "rust", projection, "projection_b", "", term)
            maxima["rust_exact_coefficient_relative"] = max(
                maxima["rust_exact_coefficient_relative"],
                discrepancy(exact_b, rust_b)["relative"],
            )
            for second_term in TERMS:
                exact_v = lookup(
                    vckss, "exact", projection, "projection_V", term, second_term
                )
                rust_v = lookup(
                    vckss, "rust", projection, "projection_V", term, second_term
                )
                maxima["rust_exact_covariance_relative"] = max(
                    maxima["rust_exact_covariance_relative"],
                    discrepancy(exact_v, rust_v)["relative"],
                )

        for term in ("z1", "z2"):
            exact_b = lookup(vckss, "exact", projection, "projection_b", "", term)
            matlab_b = lookup(
                matlab, "lincom_KSS", projection, "projection_b", "", term
            )
            exact_se = math.sqrt(
                lookup(vckss, "exact", projection, "projection_V", term, term)
            )
            matlab_se = lookup(
                matlab, "lincom_KSS", projection, "projection_se", "", term
            )
            maxima["compressed_matlab_lincom_coefficient_relative"] = max(
                maxima["compressed_matlab_lincom_coefficient_relative"],
                discrepancy(exact_b, matlab_b)["relative"],
            )
            maxima["compressed_matlab_lincom_se_relative"] = max(
                maxima["compressed_matlab_lincom_se_relative"],
                discrepancy(exact_se, matlab_se)["relative"],
            )

    passed = (
        maxima["compressed_expanded_absolute"] <= 1e-10
        and maxima["compressed_matlab_same_formula_absolute"] <= 1e-7
        and maxima["compressed_matlab_lincom_coefficient_relative"] <= 1e-6
        and maxima["compressed_matlab_lincom_se_relative"] <= 1e-5
        and maxima["rust_exact_coefficient_relative"] <= 1e-10
        and maxima["rust_exact_covariance_relative"] <= 0.005
    )
    result = {
        "schema": "VCKSS-MATLAB-WEIGHTED-PROJECTION-V1",
        "status": "pass" if passed else "fail",
        "frequency_contract": "positive integer literal physical copies",
        "target_contract": "stored-row projection mass, not frequency multiplied",
        "maxima": maxima,
        "comparisons": comparisons,
    }
    args.output_json.write_text(
        json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(json.dumps({"status": result["status"], **maxima}, indent=2))
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()

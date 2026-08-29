#!/usr/bin/env python3
import argparse
import csv
import json
import math
import statistics
from pathlib import Path


COMPONENTS = (
    "worker_variance",
    "firm_variance",
    "worker_firm_covariance",
)
PROJECTIONS = ("_cons", "z1", "z2")


def read_rows(path):
    with Path(path).open(newline="", encoding="utf-8") as handle:
        rows = []
        for raw in csv.DictReader(handle):
            row = {key: value.strip() for key, value in raw.items()}
            row["seed"] = int(row["seed"]) if row["seed"] else None
            row["value"] = float(row["value"])
            rows.append(row)
        return rows


def lookup(rows, kind, seed=None, row="", col=""):
    values = [
        item["value"]
        for item in rows
        if item["kind"] == kind
        and item["seed"] == seed
        and item["row"] == row
        and item["col"] == col
    ]
    if len(values) != 1:
        raise ValueError(
            f"Expected one {kind}/{seed}/{row}/{col} value, found {len(values)}"
        )
    return values[0]


def difference(left, right):
    absolute = abs(left - right)
    scale = max(abs(left), abs(right), 1e-30)
    return absolute, absolute / scale


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("vckss_csv")
    parser.add_argument("matlab_csv")
    parser.add_argument("output_json")
    parser.add_argument("output_csv")
    parser.add_argument("output_covariance_csv")
    args = parser.parse_args()

    vckss = read_rows(args.vckss_csv)
    matlab = read_rows(args.matlab_csv)
    seeds = (101, 202, 303, 404, 505)

    exact_rows = []
    for kind in ("projection_b", "projection_V", "projection_V_naive"):
        row_names = ("",) if kind == "projection_b" else PROJECTIONS
        for row_name in row_names:
            for col_name in PROJECTIONS:
                left = lookup(vckss, kind, 101, row_name, col_name)
                right = lookup(matlab, kind, None, row_name, col_name)
                absolute, relative = difference(left, right)
                exact_rows.append(
                    {
                        "comparison": kind,
                        "row": row_name,
                        "col": col_name,
                        "vckss": left,
                        "matlab": right,
                        "absolute_difference": absolute,
                        "relative_difference": relative,
                    }
                )

    lincom_rows = []
    for name in ("z1", "z2"):
        b_vckss = lookup(vckss, "projection_b", 101, "", name)
        b_matlab = lookup(matlab, "lincom_b", None, "", name)
        se_vckss = math.sqrt(lookup(vckss, "projection_V", 101, name, name))
        se_matlab = lookup(matlab, "lincom_se", None, "", name)
        b_abs, b_rel = difference(b_vckss, b_matlab)
        se_abs, se_rel = difference(se_vckss, se_matlab)
        lincom_rows.append(
            {
                "term": name,
                "vckss_coefficient": b_vckss,
                "matlab_coefficient": b_matlab,
                "coefficient_absolute_difference": b_abs,
                "coefficient_relative_difference": b_rel,
                "vckss_kss_se": se_vckss,
                "matlab_kss_se": se_matlab,
                "se_absolute_difference": se_abs,
                "se_relative_difference": se_rel,
            }
        )

    component_point_rows = []
    component_se_rows = []
    for component in COMPONENTS:
        vckss_points = [lookup(vckss, "component_b", seed, "", component) for seed in seeds]
        matlab_points = [lookup(matlab, "component_b", seed, "", component) for seed in seeds]
        for seed, left, right in zip(seeds, vckss_points, matlab_points):
            absolute, relative = difference(left, right)
            component_point_rows.append(
                {
                    "component": component,
                    "seed": seed,
                    "vckss": left,
                    "matlab": right,
                    "absolute_difference": absolute,
                    "relative_difference": relative,
                }
            )

        vckss_ses = [
            math.sqrt(lookup(vckss, "component_V", seed, component, component))
            for seed in seeds
        ]
        matlab_ses = [lookup(matlab, "component_se", seed, "", component) for seed in seeds]
        ratios = [left / right for left, right in zip(vckss_ses, matlab_ses)]
        component_se_rows.append(
            {
                "component": component,
                "vckss_mean_variance": statistics.mean(value * value for value in vckss_ses),
                "vckss_mean_se": statistics.mean(vckss_ses),
                "vckss_min_se": min(vckss_ses),
                "vckss_max_se": max(vckss_ses),
                "matlab_mean_variance": statistics.mean(value * value for value in matlab_ses),
                "matlab_mean_se": statistics.mean(matlab_ses),
                "matlab_min_se": min(matlab_ses),
                "matlab_max_se": max(matlab_ses),
                "mean_seedwise_vckss_matlab_ratio": statistics.mean(ratios),
                "min_seedwise_vckss_matlab_ratio": min(ratios),
                "max_seedwise_vckss_matlab_ratio": max(ratios),
            }
        )

    covariance_rows = []
    component_names = COMPONENTS + ("total_variance",)
    for row_name in component_names:
        for col_name in component_names:
            values = [
                lookup(vckss, "component_V", seed, row_name, col_name)
                for seed in seeds
            ]
            covariance_rows.append(
                {
                    "row": row_name,
                    "col": col_name,
                    "vckss_mean": statistics.mean(values),
                    "vckss_min": min(values),
                    "vckss_max": max(values),
                    "matlab": (
                        statistics.mean(
                            lookup(matlab, "component_se", seed, "", row_name) ** 2
                            for seed in seeds
                        )
                        if row_name == col_name and row_name in COMPONENTS
                        else None
                    ),
                }
            )

    max_exact_abs = max(row["absolute_difference"] for row in exact_rows)
    max_exact_rel = max(row["relative_difference"] for row in exact_rows)
    max_lincom_abs = max(
        max(row["coefficient_absolute_difference"], row["se_absolute_difference"])
        for row in lincom_rows
    )
    max_component_point_abs = max(
        row["absolute_difference"] for row in component_point_rows
    )
    max_component_point_rel = max(
        row["relative_difference"] for row in component_point_rows
    )
    max_lincom_coefficient_rel = max(
        row["coefficient_relative_difference"] for row in lincom_rows
    )
    max_lincom_se_rel = max(row["se_relative_difference"] for row in lincom_rows)
    exact_pass = (
        max_exact_abs <= 1e-10
        and max_lincom_coefficient_rel <= 1e-8
        and max_lincom_se_rel <= 1e-6
    )

    result = {
        "schema": "VCKSS-MATLAB-INFERENCE-COMPARISON-V1",
        "status": "pass" if exact_pass else "fail",
        "comparison_boundary": {
            "projection_matrix": "exact same-formula comparison",
            "lincom": "official maintained lincom_KSS comparison",
            "component_point_estimates": "descriptive official maintained leave_out_COMPLETE comparison; MATLAB uses iterative solves",
            "component_marginal_standard_errors": "descriptive because VCkss uses 16 fail-closed bins while MATLAB uses its 1000-grid smoother and retains negative fitted variances; random streams also differ",
            "component_off_diagonal_covariances": "not returned by maintained MATLAB and therefore not directly comparable",
            "total_component_variance": "not returned by maintained MATLAB and therefore not directly comparable",
        },
        "max_exact_projection_absolute_difference": max_exact_abs,
        "max_exact_projection_relative_difference": max_exact_rel,
        "max_lincom_absolute_difference": max_lincom_abs,
        "max_lincom_coefficient_relative_difference": max_lincom_coefficient_rel,
        "max_lincom_se_relative_difference": max_lincom_se_rel,
        "max_component_point_absolute_difference": max_component_point_abs,
        "max_component_point_relative_difference": max_component_point_rel,
        "projection_rows": exact_rows,
        "lincom_rows": lincom_rows,
        "component_point_rows": component_point_rows,
        "component_marginal_se_summary": component_se_rows,
        "component_covariance_summary": covariance_rows,
    }
    Path(args.output_json).write_text(
        json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    with Path(args.output_csv).open("w", newline="", encoding="utf-8") as handle:
        fields = list(component_se_rows[0])
        writer = csv.DictWriter(handle, fieldnames=fields, lineterminator="\n")
        writer.writeheader()
        writer.writerows(component_se_rows)
    with Path(args.output_covariance_csv).open("w", newline="", encoding="utf-8") as handle:
        fields = list(covariance_rows[0])
        writer = csv.DictWriter(handle, fieldnames=fields, lineterminator="\n")
        writer.writeheader()
        writer.writerows(covariance_rows)
    print(json.dumps({key: result[key] for key in (
        "status",
        "max_exact_projection_absolute_difference",
        "max_exact_projection_relative_difference",
        "max_lincom_absolute_difference",
        "max_lincom_coefficient_relative_difference",
        "max_lincom_se_relative_difference",
        "max_component_point_absolute_difference",
        "max_component_point_relative_difference",
    )}, indent=2))
    if not exact_pass:
        raise SystemExit(1)


if __name__ == "__main__":
    main()

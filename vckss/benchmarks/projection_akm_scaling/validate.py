#!/usr/bin/env python3
"""Validate a captured AKM projection pair without discarding failures."""

from __future__ import annotations

import argparse
import csv
import json
import math
import re
from pathlib import Path
from typing import Any

from common import (
    LEAVE_OUT_SHA256,
    LINCOM_SHA256,
    UPSTREAM_COMMIT,
    read_manifest,
)


def key_values(path: Path) -> dict[str, str]:
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle, delimiter="\t"))
    if not rows or list(rows[0]) != ["key", "value"]:
        raise ValueError(f"invalid key/value receipt: {path}")
    result = {row["key"]: row["value"] for row in rows}
    if len(result) != len(rows):
        raise ValueError(f"duplicate key in {path}")
    return result


def csv_row(path: Path) -> dict[str, str]:
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    if len(rows) != 1:
        raise ValueError(f"expected one row in {path}")
    return rows[0]


def unit_scaled(left: float, right: float) -> float:
    return abs(left - right) / max(1.0, abs(left), abs(right))


def relative(left: float, right: float) -> float:
    return abs(left - right) / max(1e-300, abs(left), abs(right))


def optional_int(value: str) -> int | None:
    """Read a Stata numeric that may be exported as an empty missing value."""

    if value.strip() == "":
        return None
    parsed = float(value)
    if not math.isfinite(parsed) or parsed != math.floor(parsed):
        raise ValueError(f"invalid optional integer: {value}")
    return int(parsed)


def matrix_relative(left: dict[str, str], right: dict[str, str]) -> float:
    differences: list[float] = []
    scale = 1e-300
    for i in range(1, 4):
        for j in range(1, 4):
            lvalue = float(left[f"V_{i}_{j}"])
            rvalue = float(right[f"V_{i}_{j}"])
            differences.append(abs(lvalue - rvalue))
            scale = max(scale, abs(lvalue), abs(rvalue))
    return max(differences) / scale


def validate_process(role_dir: Path, outcome: str, cap: int) -> dict[str, Any]:
    tree = json.loads((role_dir / "process_tree.json").read_text(encoding="utf-8"))
    peak = int(tree.get("whole_peak_rss_kib", 0))
    if peak <= 0 or not (role_dir / "resources.txt").is_file():
        raise ValueError(f"missing process/resource evidence in {role_dir}")
    wall = float((role_dir / "whole_wall_seconds.txt").read_text(encoding="utf-8"))
    if not math.isfinite(wall) or wall <= 0:
        raise ValueError(f"invalid whole wall in {role_dir}")
    if outcome == "PASS" and tree.get("status") != "PASS":
        raise ValueError(f"passing application has failed monitor in {role_dir}")
    if outcome == "RIGHT_CENSORED" and not (cap <= wall <= cap + 180):
        raise ValueError(f"censor wall is inconsistent in {role_dir}: {wall}")
    return {"whole_peak_rss_kib": peak, "whole_wall_seconds": wall, "tree": tree}


def validate_role(
    pair_dir: Path, role: str, expected: dict[str, str]
) -> dict[str, Any]:
    role_dir = pair_dir / role
    status = key_values(role_dir / "status.tsv")
    if status.get("schema") != "VCKSS-PROJECTION-AKM-ROLE-STATUS-V1":
        raise ValueError(f"role status schema changed for {role}")
    for key in ("source_commit", "input_sha256", "task_sha256"):
        if status.get(key) != expected[key]:
            raise ValueError(f"{role} {key} changed")
    if int(status["active_cores"]) != int(expected["cores"]):
        raise ValueError(f"{role} core identity changed")
    if int(status["stata_processors"]) != min(4, int(expected["cores"])):
        raise ValueError(f"{role} Stata processor identity changed")
    outcome = status["outcome"]
    if outcome not in ("PASS", "FAIL", "RIGHT_CENSORED"):
        raise ValueError(f"invalid role outcome for {role}")
    app_rc = int(status["application_exit_status"])
    monitor_rc = int(status["monitor_exit_status"])
    valid = int(status["application_receipt_valid"])
    censored = int(status["right_censored"])
    cap = int(expected["role_cap_seconds"])
    if int(status["role_cap_seconds"]) != cap:
        raise ValueError(f"{role} cap changed")
    if outcome == "PASS" and (app_rc != 0 or monitor_rc != 0 or valid != 1 or censored != 0):
        raise ValueError(f"inconsistent passing status for {role}")
    if outcome == "RIGHT_CENSORED" and (app_rc != 124 or valid != 0 or censored != 1):
        raise ValueError(f"inconsistent censor status for {role}")
    if outcome == "FAIL" and (app_rc == 0 and monitor_rc == 0 and valid == 1):
        raise ValueError(f"inconsistent failed status for {role}")
    process = validate_process(role_dir, outcome, cap)
    return {**status, **process}


def validate_pair(
    pair_dir: Path,
    manifest: Path,
    task_id: int,
    *,
    gate: bool,
) -> dict[str, Any]:
    expected = read_manifest(manifest)[task_id - 1]
    receipt = key_values(pair_dir / "pair_receipt.tsv")
    if receipt.get("schema") != "VCKSS-PROJECTION-AKM-PAIR-V1" or receipt.get("status") != "CAPTURED":
        raise ValueError("pair receipt did not capture")
    for key in (
        "stage", "task_id", "rows", "workers", "firms", "cores", "replicate",
        "probes", "seed", "order", "role_cap_seconds", "memory_gib",
        "task_sha256",
    ):
        if receipt.get(key) != expected[key]:
            raise ValueError(f"pair receipt {key} changed")
    if not re.fullmatch(r"[0-9a-f]{40}", receipt.get("source_commit", "")):
        raise ValueError("invalid pair source identity")
    if not re.fullmatch(r"[0-9a-f]{64}", receipt.get("input_sha256", "")):
        raise ValueError("invalid pair input identity")
    expected = {
        **expected,
        "source_commit": receipt["source_commit"],
        "input_sha256": receipt["input_sha256"],
    }
    if receipt.get("scheduler_slots") != ("4" if gate else "16"):
        raise ValueError("scheduler slot contract changed")
    if int(receipt.get("stata_processors", -1)) != min(4, int(expected["cores"])):
        raise ValueError("pair Stata processor contract changed")
    baseline = int((pair_dir / "stata_baseline_rss_kib.txt").read_text().strip())
    if baseline <= 0:
        raise ValueError("invalid empty-Stata baseline")
    roles = {
        role: validate_role(pair_dir, role, expected)
        for role in ("rust", "matlab")
    }
    outcomes = {role: value["outcome"] for role, value in roles.items()}
    result: dict[str, Any] = {
        "schema": "VCKSS-PROJECTION-AKM-VALIDATION-V1",
        "status": "PASS" if set(outcomes.values()) == {"PASS"} else (
            "RIGHT_CENSORED" if "RIGHT_CENSORED" in outcomes.values() else "FAIL"
        ),
        **{key: expected[key] for key in (
            "stage", "task_id", "rows", "workers", "firms", "cores", "replicate",
            "probes", "seed", "order", "role_cap_seconds", "memory_gib",
            "task_sha256",
        )},
        "source_commit": expected["source_commit"],
        "input_sha256": expected["input_sha256"],
        "role_outcomes": outcomes,
        "pair_whole_wall_seconds": sum(value["whole_wall_seconds"] for value in roles.values()),
        "rust_whole_peak_rss_kib": roles["rust"]["whole_peak_rss_kib"],
        "matlab_whole_peak_rss_kib": roles["matlab"]["whole_peak_rss_kib"],
        "stata_baseline_rss_kib": baseline,
        "rust_incremental_rss_kib": roles["rust"]["whole_peak_rss_kib"] - baseline,
    }
    if result["status"] == "PASS":
        rust = csv_row(pair_dir / "rust" / "result.csv")
        matlab = json.loads((pair_dir / "matlab" / "result.json").read_text(encoding="utf-8"))
        if rust.get("status") != "PASS" or matlab.get("status") != "PASS":
            raise ValueError("passing role result status changed")
        for key in ("source_commit", "input_sha256"):
            if rust[key] != expected[key] or matlab[key] != expected[key]:
                raise ValueError(f"result {key} changed")
        for key in ("rows", "workers", "firms", "probes", "seed", "active_cores"):
            if int(float(rust[key])) != int(expected["cores" if key == "active_cores" else key]):
                raise ValueError(f"Rust result {key} changed")
            if int(matlab[key]) != int(expected["cores" if key == "active_cores" else key]):
                raise ValueError(f"MATLAB result {key} changed")
        if int(float(rust["stata_processors"])) != min(4, int(expected["cores"])):
            raise ValueError("Rust result Stata processor identity changed")
        if matlab["upstream_commit"] != UPSTREAM_COMMIT or matlab["leave_out_sha256"] != LEAVE_OUT_SHA256 or matlab["lincom_sha256"] != LINCOM_SHA256:
            raise ValueError("maintained MATLAB identity changed")
        coefficient_relative = max(
            unit_scaled(float(rust[f"b_{term}"]), float(matlab[f"b_{term}"]))
            for term in ("cons", "z1", "z2")
        )
        se_relative = max(
            relative(math.sqrt(float(rust[f"V_{index}_{index}"])), float(matlab[f"se_{term}"]))
            for index, term in ((2, "z1"), (3, "z2"))
        )
        covariance_relative = max(
            relative(float(rust[f"V_{index}_{index}"]), float(matlab[f"V_{term}_{term}"]))
            for index, term in ((2, "z1"), (3, "z2"))
        )
        science_failures: list[str] = []
        if coefficient_relative > 1e-8:
            science_failures.append(f"coefficient={coefficient_relative}")
        if se_relative > 0.01:
            science_failures.append(f"se={se_relative}")
        if covariance_relative > 0.01:
            science_failures.append(f"covariance_diagonal={covariance_relative}")
        if float(rust["projection_complete_residual"]) > float(rust["residual_tolerance"]):
            science_failures.append("projection complete residual")
        if float(rust["solver_complete_residual"]) > float(rust["residual_tolerance"]):
            science_failures.append("model complete residual")
        if float(rust["covariance_min"]) < -1e-8 * max(1.0, float(rust["covariance_max"])):
            science_failures.append("PSD")
        if float(rust["memory_forecast_bytes"]) > float(rust["memory_limit_bytes"]):
            science_failures.append("memory forecast")
        if float(rust["projection_gram_rcond"]) <= 0:
            science_failures.append("projection conditioning")
        route_codes = {
            key: int(float(rust[key]))
            for key in (
                "route_code", "rust_requested_route", "rust_selected_route",
                "rust_solver_fallback", "rust_solver_fallback_error",
            )
        }
        if (
            route_codes["route_code"] != 3
            or route_codes["rust_requested_route"] != 3
            or route_codes["rust_selected_route"] != 3
            or route_codes["rust_solver_fallback"] != 0
            or route_codes["rust_solver_fallback_error"] != 0
        ):
            science_failures.append("forced CMG route identity")
        route_levels = optional_int(rust["route_hierarchy_levels"])
        route_terminal = optional_int(rust["route_terminal_vertices"])
        if route_levels is not None and route_levels < 1:
            science_failures.append("CMG hierarchy levels")
        if route_terminal is not None and not 1 <= route_terminal <= 6144:
            science_failures.append("CMG terminal vertices")
        result.update({
            "coefficient_max_unit_scaled_difference": coefficient_relative,
            "se_max_relative_difference": se_relative,
            "covariance_diagonal_max_relative_difference": covariance_relative,
            "rust_command_seconds": float(rust["command_seconds"]),
            "matlab_command_seconds": float(matlab["command_seconds"]),
            "rust_projection_complete_residual": float(rust["projection_complete_residual"]),
            "rust_model_complete_residual": float(rust["solver_complete_residual"]),
            "rust_residual_tolerance": float(rust["residual_tolerance"]),
            "rust_projection_solver_iterations": int(float(rust["projection_solver_iterations"])),
            "rust_route_hierarchy_levels": route_levels,
            "rust_route_terminal_vertices": route_terminal,
            **{f"rust_{key}": value for key, value in route_codes.items()},
            "rust_projection_gram_rcond": float(rust["projection_gram_rcond"]),
            "rust_projection_psd_cleanup": float(rust["psd_cleanup"]),
            "rust_memory_forecast_bytes": int(float(rust["memory_forecast_bytes"])),
            "matlab_fit_relative_residual": float(matlab["fit_relative_residual"]),
            "matlab_fit_iterations": int(matlab["fit_iterations"]),
            "matlab_complete_fit_residual": float(matlab["complete_fit_residual"]),
            "science_failures": science_failures,
        })
        if science_failures:
            result["status"] = "FAIL"
    if gate:
        exact_status = validate_role(pair_dir, "exact", expected)
        result["exact_outcome"] = exact_status["outcome"]
        if exact_status["outcome"] != "PASS" or result["status"] != "PASS":
            result["status"] = "FAIL"
        else:
            exact = csv_row(pair_dir / "exact" / "result.csv")
            rust = csv_row(pair_dir / "rust" / "result.csv")
            matlab = json.loads((pair_dir / "matlab" / "result.json").read_text())
            exact_coefficient = max(
                unit_scaled(float(exact[f"b_{term}"]), float(rust[f"b_{term}"]))
                for term in ("cons", "z1", "z2")
            )
            exact_covariance = matrix_relative(exact, rust)
            exact_matlab_se = max(
                relative(math.sqrt(float(exact[f"V_{index}_{index}"])), float(matlab[f"se_{term}"]))
                for index, term in ((2, "z1"), (3, "z2"))
            )
            result.update({
                "exact_rust_coefficient_max_unit_scaled_difference": exact_coefficient,
                "exact_rust_covariance_matrix_relative_difference": exact_covariance,
                "exact_matlab_se_max_relative_difference": exact_matlab_se,
            })
            if max(exact_coefficient / 1e-8, exact_covariance / 0.01, exact_matlab_se / 0.01) > 1:
                result["status"] = "FAIL"
                result.setdefault("science_failures", []).append("exact oracle gate")
    return result


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("pair_dir", type=Path)
    parser.add_argument("manifest", type=Path)
    parser.add_argument("task_id", type=int)
    parser.add_argument("output", type=Path)
    parser.add_argument("--gate", action="store_true")
    parser.add_argument("--require-pass", action="store_true")
    args = parser.parse_args()
    result = validate_pair(args.pair_dir, args.manifest, args.task_id, gate=args.gate)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(result, indent=2, sort_keys=True))
    if args.require_pass and result["status"] != "PASS":
        raise SystemExit(2)


if __name__ == "__main__":
    main()

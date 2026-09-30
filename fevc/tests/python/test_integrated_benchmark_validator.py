from __future__ import annotations

import csv
import importlib.util
from pathlib import Path

import pytest


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "vckss_integrated_checks", ROOT / "tools/run_checks.py"
)
assert SPEC is not None and SPEC.loader is not None
CHECKS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKS)


def receipt(backend: str) -> dict[str, str]:
    row = {
        "status": "KSS_POINT_ESTIMATES_ONLY",
        "backend_selected": backend,
        "command_seconds": "0.1",
        "total_seconds": "0.2",
        "N_stored": "40",
        "N_physical": "60",
        "N_retained": "40",
        "worker_levels": "10",
        "firm_levels": "4",
        "deletion_units": "20",
        "requested_probes": "10",
        "probes": "10",
        "mcse_mode": "all",
        "mcse_method": "crossfit_if_v1",
        "mcse_status": "ok_local",
        "mcse_available": "1",
    }
    covariance = [[1, 0, 0, 1], [0, 1, 0, 1], [0, 0, 1, 2], [1, 1, 2, 6]]
    for i in range(4):
        for j in range(4):
            row[f"mcse_cov_raw_{i+1}{j+1}"] = str(1e-4 * covariance[i][j])
    for prefix in ("plugin", "correction", "corrected", "mcse"):
        for target in ("worker", "firm", "covariance", "total"):
            row[f"{prefix}_{target}"] = (
                "-0.01" if target == "covariance" and prefix != "mcse" else "0.01"
            )
    row["mcse_total"] = str(0.01 * 6**0.5)
    if backend == "rust":
        row.update(
            {
                "rust_deletion_rank_gap": "0.4",
                "complete_residual_max": "1e-12",
                "target_identity_residual": "1e-13",
                "rust_maker_relres": "2e-13",
                "rust_actual_accounting_residual": "3e-14",
                "rust_plan_solve_peak_bytes": "4096",
                "rust_plan_nonbatched_peak_bytes": "2048",
                "rust_counter_plan_complete": "1",
                "rust_pre_rng_hi": "0",
                "rust_pre_rng_lo": "0",
            }
        )
    else:
        row["deletion_rank_gap"] = "0.4"
        for field in (
            "graph_seconds",
            "fit_seconds",
            "preconditioner_seconds",
            "leverage_seconds",
            "target_seconds",
            "correction_seconds",
        ):
            row[field] = "0.01"
    return row


def write_receipt(directory: Path, row: dict[str, str]) -> None:
    (directory / "smoke.stata.pass").write_text("PASS\n", encoding="utf-8")
    with (directory / "smoke.csv").open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(row))
        writer.writeheader()
        writer.writerow(row)


def test_native_smoke_accepts_explicitly_unavailable_phase_timings(
    tmp_path: Path,
) -> None:
    write_receipt(tmp_path, receipt("rust"))
    CHECKS.validate_benchmark(tmp_path)


def test_native_smoke_requires_native_scientific_receipts(tmp_path: Path) -> None:
    row = receipt("rust")
    row["rust_deletion_rank_gap"] = ""
    write_receipt(tmp_path, row)
    with pytest.raises(RuntimeError, match="rust_deletion_rank_gap"):
        CHECKS.validate_benchmark(tmp_path)


def test_mata_smoke_still_requires_mata_phase_timings(tmp_path: Path) -> None:
    row = receipt("mata")
    row["graph_seconds"] = ""
    write_receipt(tmp_path, row)
    with pytest.raises(RuntimeError, match="graph_seconds"):
        CHECKS.validate_benchmark(tmp_path)


def test_withheld_mcse_retains_points_and_raw_covariance(tmp_path: Path) -> None:
    row = receipt("mata")
    row.update(mcse_status="unstable_nonpsd", mcse_available="0")
    for target in ("worker", "firm", "covariance", "total"):
        row[f"mcse_{target}"] = ""
    for field in row:
        if field.startswith("mcse_cov_raw_"):
            row[field] = str(-float(row[field]))
    write_receipt(tmp_path, row)
    CHECKS.validate_benchmark(tmp_path)


@pytest.mark.parametrize("field,value", [
    ("mcse_mode", "conditional"), ("mcse_method", ""),
    ("mcse_status", "unknown"), ("mcse_available", "0"),
    ("mcse_worker", ""), ("mcse_firm", "-1"),
    ("mcse_cov_raw_11", ""), ("mcse_cov_raw_12", "2"),
    ("mcse_cov_raw_44", "2"), ("mcse_cov_raw_33", "nan"),
])
def test_mcse_export_rejects_inconsistent_or_partial_receipts(
    tmp_path: Path, field: str, value: str,
) -> None:
    row = receipt("mata")
    row[field] = value
    write_receipt(tmp_path, row)
    with pytest.raises(RuntimeError, match="MCSE|mcse"):
        CHECKS.validate_benchmark(tmp_path)


def test_withheld_mcse_cannot_export_a_conditional_substitute(tmp_path: Path) -> None:
    row = receipt("mata")
    row.update(mcse_status="unstable_nonpsd", mcse_available="0")
    write_receipt(tmp_path, row)
    with pytest.raises(RuntimeError, match="must leave"):
        CHECKS.validate_benchmark(tmp_path)

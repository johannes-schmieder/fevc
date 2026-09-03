from __future__ import annotations

import csv
import importlib.util
import sys
from pathlib import Path

import pytest


ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "fevc" / "tools" / "validate_structured_inference_qualification.py"
SPEC = importlib.util.spec_from_file_location("structured_qualification", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


def _cell_metadata(cell: str) -> tuple[str, str, str, str, str, str]:
    dominant = cell.startswith("dominant")
    reference = "Q0" if not dominant or cell.endswith("_q0") else "Q1"
    if "mild" in cell:
        gate = "mild"
    elif "severe" in cell:
        gate = "descriptive"
    elif "null" in cell:
        gate = "diagnostic"
    else:
        gate = "correct"
    model = "structured_leverage" if "leverage" in cell else "structured_common"
    variance = (
        "SevereOmitted"
        if "severe" in cell
        else "MildOmitted"
        if "mild_omitted" in cell
        else "MildFunctional"
        if "mild_functional" in cell
        else "Leverage"
        if "leverage" in cell
        else "Homoskedastic"
        if "homoskedastic" in cell
        else "Common"
    )
    error = "StudentT8" if cell.endswith("_t8") else "Gaussian"
    beta = "zero" if "null" in cell else "nonzero"
    return gate, model, variance, error, reference, beta


def _write_fixture(path: Path, profile: str, bad_coverage: bool = False) -> None:
    keys = sorted(MODULE._expected_keys(profile))
    rows = []
    for cell, k, target in keys:
        gate, model, variance, error, reference, beta = _cell_metadata(cell)
        dominant = cell.startswith("dominant")
        if dominant and target in {"worker", "firm"}:
            leading, remainder = 0.80, 0.10
        elif dominant and target == "covariance":
            leading, remainder = 0.98, 0.60
        elif target == "total":
            leading, remainder = (0.05 if k == 12 else 0.04), 0.05
        elif target == "covariance":
            leading, remainder = 0.15, 0.20
        else:
            leading = 0.09 if k == 12 else 0.07
            remainder = 0.10
        coverage = 0.95
        se_ratio = 1.0
        if "severe" in cell and target == "worker":
            coverage, se_ratio = 0.80, 1.30
        if bad_coverage and cell == "diffuse_common" and k == 16 and target == "worker":
            coverage = 0.80
        replications = 100 if profile == "development" else 2_500
        rows.append(
            {
                "schema": "1",
                "profile": profile,
                "cell": cell,
                "gate": gate,
                "k": str(k),
                "controls": str(int("controls" in cell)),
                "dominant": str(int(dominant)),
                "variance_model": model,
                "variance_dgp": variance,
                "error_dgp": error,
                "reference": reference,
                "beta": beta,
                "target": target,
                "replications": str(replications),
                "successes": str(replications),
                "bias": "0",
                "bias_mcse": "0.001",
                "coverage": str(coverage),
                "coverage_mcse": "0.004",
                "empirical_sd": "1",
                "mean_se": "1",
                "se_ratio": str(se_ratio),
                "mean_estimated_variance": "1",
                "leading_share": str(leading),
                "remainder_share": str(remainder),
                "mean_floor_share": "0",
                "mean_boundary_share": "0.01",
            }
        )
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=sorted(MODULE.REQUIRED_COLUMNS))
        writer.writeheader()
        writer.writerows(rows)


def test_registered_cell_matrices_are_complete() -> None:
    assert len(MODULE._expected_keys("development")) == 72
    assert len(MODULE._expected_keys("confirmation")) == 80


def test_development_receipt_is_not_promotion_evidence(tmp_path: Path) -> None:
    evidence = tmp_path / "development.csv"
    _write_fixture(evidence, "development")
    receipt = MODULE.validate(evidence, "development")
    assert receipt["status"] == "PASS"
    assert receipt["decision"] == "development_pass"
    assert receipt["severe_misspecification"]["visibly_misspecified"]


def test_confirmation_fails_closed_on_material_undercoverage(tmp_path: Path) -> None:
    evidence = tmp_path / "confirmation.csv"
    _write_fixture(evidence, "confirmation", bad_coverage=True)
    with pytest.raises(MODULE.QualificationError, match="coverage misses"):
        MODULE.validate(evidence, "confirmation")


def test_repeat_must_be_byte_identical(tmp_path: Path) -> None:
    evidence = tmp_path / "development.csv"
    repeated = tmp_path / "repeat.csv"
    _write_fixture(evidence, "development")
    _write_fixture(repeated, "development")
    MODULE.validate(evidence, "development", repeated)
    repeated.write_text(repeated.read_text(encoding="utf-8") + "\n", encoding="utf-8")
    with pytest.raises(MODULE.QualificationError, match="byte-for-byte"):
        MODULE.validate(evidence, "development", repeated)

from __future__ import annotations

import math
from pathlib import Path

from kss_bc.benchmarks.model_numopt2 import (
    EXPECTED_BUNDLE_SHA256,
    EXPECTED_SOURCE_COMMIT,
    GIB,
    REQUIRED_EXPERIMENTS,
    TARGET_RHS,
    TRAIN_RHS,
    affine_fit,
    choose_batch,
    compressed_memory,
    parse_memory,
    raw_basis,
    through_origin_fit,
)

ROOT = Path(__file__).resolve().parents[3]


def test_affine_scale_fit_recovers_intercept_and_slope() -> None:
    fit = affine_fit((1 / 64, 1 / 32, 1 / 16), (12.5, 15.0, 20.0))
    assert math.isclose(fit.intercept, 10.0, rel_tol=1e-12)
    assert math.isclose(fit.slope, 160.0, rel_tol=1e-12)
    assert math.isclose(fit.predict(1), 170.0, rel_tol=1e-12)
    assert fit.train_max_relative_error < 1e-12


def test_density_four_registers_one_completed_rung_when_larger_runs_censor() -> None:
    assert "strong_f64_d4" in REQUIRED_EXPERIMENTS
    assert "strong_f32_d4" not in REQUIRED_EXPERIMENTS
    assert "strong_f16_d4" not in REQUIRED_EXPERIMENTS


def test_raw_increment_fit_is_nonnegative_and_through_origin() -> None:
    slope, error = through_origin_fit((10, 20, 40), (30, 60, 120))
    assert math.isclose(slope, 3.0, rel_tol=1e-12)
    assert error < 1e-12
    clipped_slope, _ = through_origin_fit((1, 2), (-1, -2))
    assert clipped_slope == 0


def test_raw_compression_basis_accounts_for_sort_growth() -> None:
    cells = 1_000
    linear = raw_basis("import_selection_seconds", 8 * cells, cells)
    sorting = raw_basis("compression_transition_seconds", 8 * cells, cells)
    assert linear == 7 * cells
    assert sorting > linear


def test_model_uses_complete_rhs_scaling_and_binary_memory_units() -> None:
    assert TRAIN_RHS == 61
    assert TARGET_RHS == 601
    assert parse_memory("1.5G") == round(1.5 * 1024**3)
    assert len(EXPECTED_SOURCE_COMMIT) == 40
    assert len(EXPECTED_BUNDLE_SHA256) == 64


def test_model_contract_separates_raw_and_numerical_stages() -> None:
    source = (ROOT / "kss_bc/benchmarks/model_numopt2.py").read_text(
        encoding="utf-8"
    )
    assert '"compression_transition_seconds"' in source
    assert '"numerical_p200_seconds"' in source
    assert "minimum_rpc_26_3_error_fraction" in source
    assert "admission_headroom_fraction_after_model_high" in source


def test_structural_memory_matches_api8_overlap_schedule() -> None:
    result = compressed_memory(
        workers=100,
        firms=10,
        density=3,
        rows_per_cell=8,
        probes=200,
        batch=4,
        raw_bytes_per_row=120,
        cmg_hierarchy_bytes=123_456,
    )
    cells = 300
    parameters = 109
    expected_cell = 8 * (9 * cells + 3 * 100 + 4 * 10)
    expected_phase = 8 * 4 * (2 * cells + 8 * cells + 6 * parameters)
    assert result["cell_bytes"] == expected_cell
    assert result["phase_scratch_bytes"] == expected_phase
    assert result["numerical_peak_bytes"] >= result["transition_peak_bytes"]


def test_batch_selection_reduces_scratch_to_meet_hard_memory_cap() -> None:
    selected, forecast, admitted = choose_batch(
        workers=40_000_000,
        firms=1_000_000,
        density=2,
        rows_per_cell=1,
        probes=200,
        raw_bytes_per_row=0,
        cmg_hierarchy_bytes=0,
        memory_error=0.20,
    )
    assert admitted
    assert selected < 16
    assert float(forecast["peak_bytes"]) * 1.2 * 1.2 <= 128 * GIB

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
SOURCE = (ROOT / "kss_bc_scale.mata").read_text(encoding="utf-8")
RUNTIME_SOURCE = (ROOT / "kss_bc_scale_runtime.mata").read_text(
    encoding="utf-8"
)
ADO_SOURCE = (ROOT / "kss_bc.ado").read_text(encoding="utf-8")


@dataclass(frozen=True)
class Fixture:
    worker: np.ndarray
    firm: np.ndarray
    deletion: np.ndarray
    frequency: np.ndarray
    outcome: np.ndarray
    target: np.ndarray


def fixture() -> Fixture:
    worker = np.repeat(np.arange(3), 4)
    firm = np.array([0, 0, 1, 1, 0, 0, 2, 2, 1, 1, 2, 2])
    deletion = np.array([10, 11, 12, 12, 20, 20, 21, 22, 30, 30, 31, 31])
    frequency = np.array([2, 1, 1, 3, 2, 2, 1, 4, 1, 2, 3, 1], dtype=np.int64)
    outcome = np.array(
        [0.2, -0.1, 0.8, 1.1, -0.4, 0.3, 1.4, 1.0, -0.2, 0.5, 0.9, 1.3]
    )
    target = frequency.astype(float)
    target[[1, 7]] /= 2
    target[11] = np.nextafter(target[11], np.inf)
    return Fixture(worker, firm, deletion, frequency, outcome, target)


def canonical_groups(*keys: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    key = np.column_stack(keys)
    unique, inverse = np.unique(key, axis=0, return_inverse=True)
    return unique, inverse


def compensated_sum(values: np.ndarray) -> float:
    subtotal = 0.0
    correction = 0.0
    for value in values:
        updated = subtotal + float(value)
        if abs(subtotal) >= abs(value):
            correction += (subtotal - updated) + float(value)
        else:
            correction += (float(value) - updated) + subtotal
        subtotal = updated
    return subtotal + correction


def compressed_fixture(data: Fixture) -> dict[str, np.ndarray]:
    cells, row_to_cell = canonical_groups(data.worker, data.firm)
    units, row_to_unit = np.unique(data.deletion, return_inverse=True)
    unit_cell = np.empty(units.size, dtype=np.int64)
    for unit in range(units.size):
        member_cells = np.unique(row_to_cell[row_to_unit == unit])
        if member_cells.size != 1:
            raise ValueError("deletion unit spans coefficient cells")
        unit_cell[unit] = member_cells[0]

    scale = data.target / data.frequency
    strata_key, row_to_stratum = canonical_groups(row_to_cell, scale)

    cell_frequency = np.bincount(row_to_cell, weights=data.frequency)
    cell_outcome_sum = np.array(
        [
            compensated_sum(data.frequency[row_to_cell == cell] * data.outcome[row_to_cell == cell])
            for cell in range(cells.shape[0])
        ]
    )
    cell_mean = cell_outcome_sum / cell_frequency
    cell_centered_ss = np.array(
        [
            compensated_sum(
                data.frequency[row_to_cell == cell]
                * (data.outcome[row_to_cell == cell] - cell_mean[cell]) ** 2
            )
            for cell in range(cells.shape[0])
        ]
    )
    return {
        "cells": cells,
        "row_to_cell": row_to_cell,
        "units": units,
        "row_to_unit": row_to_unit,
        "unit_cell": unit_cell,
        "strata_key": strata_key,
        "row_to_stratum": row_to_stratum,
        "cell_frequency": cell_frequency,
        "cell_outcome_sum": cell_outcome_sum,
        "cell_mean": cell_mean,
        "cell_centered_ss": cell_centered_ss,
    }


def dense_design(data: Fixture) -> np.ndarray:
    workers = int(data.worker.max() + 1)
    firms = int(data.firm.max() + 1)
    x = np.zeros((data.worker.size, workers + firms - 1))
    x[np.arange(data.worker.size), data.worker] = 1
    keep_firm = data.firm < firms - 1
    x[np.flatnonzero(keep_firm), workers + data.firm[keep_firm]] = 1
    return x


def test_mata_source_exposes_standalone_compressed_api() -> None:
    required = (
        "struct kssbc_scale_design",
        "struct kssbc_scale_diagnostic",
        "struct kssbc_scale_target_strata",
        "kssbc_scale__diagnose(",
        "kssbc_scale__prepare(",
        "kssbc_scale__compact(",
        "kssbc_scale__fe_transpose(",
        "kssbc_scale__fe_schur_action(",
        "kssbc_scale__fe_worker(",
        "kssbc_scale__fe_predict(",
        "kssbc_scale__fe_full_residual(",
        "kssbc_scale__weighted_rss(",
        "kssbc_scale__strata_prepare(",
        "kssbc_scale__strata_to_cells(",
        "unit_cell_order",
        "unit_cell_panel",
    )
    for name in required:
        assert name in SOURCE
    stable_helper = SOURCE[
        SOURCE.index("kssbc_scale__stable_groupsum(") :
        SOURCE.index("kssbc_scale__integer_group_sum(")
    ]
    assert "panelsum(" not in stable_helper
    assert "for (group=" not in stable_helper
    assert "correction[active,.]" in stable_helper
    target_helper = SOURCE[
        SOURCE.index("kssbc_scale__strata_prepare(") :
        SOURCE.index("kssbc_scale__diagnose(")
    ]
    assert "tolerance" not in target_helper


def test_command_constructs_and_cleans_one_cached_compressed_state() -> None:
    assert ADO_SOURCE.count("kssbc_srt__prepare(") == 1
    assert "kssbc_scale__stata_diag(" not in ADO_SOURCE
    assert "kssbc_scale__diagnose(" not in RUNTIME_SOURCE
    assert "kssbc_scale__prepare(" not in RUNTIME_SOURCE
    assert "KSSBC_SCALE_RUNTIME = kssbc_srt__empty()" not in RUNTIME_SOURCE

    prepare = SOURCE[
        SOURCE.index("void kssbc_srt__prepare(") :
        SOURCE.index("void kssbc_scale__stata_diag(")
    ]
    assert prepare.count("kssbc_scale__prepare(") == 1
    assert "kssbc_scale__diagnose(" not in prepare
    assert "diagnostic = design.diagnostic" in prepare

    wrapper = ADO_SOURCE[: ADO_SOURCE.index("program define _kss_bc_impl")]
    assert wrapper.count("kssbc_scale_runtime__reset()") == 2
    command_prepare = ADO_SOURCE.index("capture noisily mata: kssbc_srt__prepare(")
    for known_rejection in (
        '"`engine_requested\'" == "generic"',
        '"`deletion\'" != "match"',
        "`control_count' > 0",
    ):
        assert ADO_SOURCE.index(known_rejection) < command_prepare
    assert ADO_SOURCE.index("_kss_bc_lifecycle_memory, stage(selection)") < command_prepare
    assert ADO_SOURCE.index('kssbc_scale_runtime__status() == "PREPARED"') > command_prepare


def test_cells_deletion_units_and_target_strata_are_independent() -> None:
    data = fixture()
    compressed = compressed_fixture(data)
    assert compressed["cells"].shape[0] == 6
    assert compressed["units"].size == 8
    assert np.count_nonzero(compressed["unit_cell"] == 0) == 2
    assert compressed["strata_key"].shape[0] == 9
    assert np.array_equal(
        np.unique(compressed["row_to_cell"]), np.arange(6)
    )
    assert np.array_equal(
        np.unique(compressed["row_to_unit"]), np.arange(8)
    )


def test_compressed_fe_algebra_equals_row_algebra() -> None:
    data = fixture()
    compressed = compressed_fixture(data)
    x = dense_design(data)
    weights = data.frequency.astype(float)
    cells = compressed["cells"]
    workers = int(data.worker.max() + 1)
    firms = int(data.firm.max() + 1)

    x_cell = np.zeros((cells.shape[0], workers + firms - 1))
    x_cell[np.arange(cells.shape[0]), cells[:, 0]] = 1
    keep = cells[:, 1] < firms - 1
    x_cell[np.flatnonzero(keep), workers + cells[keep, 1]] = 1
    h_row = x.T @ (weights[:, None] * x)
    h_cell = x_cell.T @ (compressed["cell_frequency"][:, None] * x_cell)
    assert np.array_equal(h_row, h_cell)

    values = np.column_stack((data.outcome, np.sin(np.arange(data.outcome.size))))
    row_rhs = x.T @ values
    cell_values = np.vstack(
        [
            [compensated_sum(values[compressed["row_to_cell"] == cell, column]) for column in range(2)]
            for cell in range(cells.shape[0])
        ]
    )
    assert np.allclose(row_rhs, x_cell.T @ cell_values, atol=2e-15, rtol=2e-15)

    gamma = np.array([[0.4, -0.1], [-0.2, 0.3], [0.0, 0.0]])
    worker_mass = np.bincount(cells[:, 0], weights=compressed["cell_frequency"])
    worker_mean = np.vstack(
        [
            np.sum(
                compressed["cell_frequency"][cells[:, 0] == worker, None]
                * gamma[cells[cells[:, 0] == worker, 1]],
                axis=0,
            )
            / worker_mass[worker]
            for worker in range(workers)
        ]
    )
    cell_schur = np.vstack(
        [
            np.sum(
                compressed["cell_frequency"][cells[:, 1] == firm, None]
                * (
                    gamma[firm]
                    - worker_mean[cells[cells[:, 1] == firm, 0]]
                ),
                axis=0,
            )
            for firm in range(firms)
        ]
    )
    projection = np.eye(data.worker.size) - x[:, :workers] @ np.diag(
        1 / np.bincount(data.worker, weights=weights)
    ) @ (x[:, :workers].T * weights)
    row_firm = np.zeros((data.worker.size, firms))
    row_firm[np.arange(data.worker.size), data.firm] = 1
    row_schur = row_firm.T @ (weights[:, None] * (projection @ row_firm @ gamma))
    assert np.allclose(cell_schur, row_schur, atol=3e-15, rtol=3e-15)


def test_compressed_rss_uses_centered_moments_exactly() -> None:
    data = fixture()
    compressed = compressed_fixture(data)
    prediction = np.linspace(-0.3, 0.7, compressed["cells"].shape[0])
    row_prediction = prediction[compressed["row_to_cell"]]
    raw = np.sum(data.frequency * (data.outcome - row_prediction) ** 2)
    compressed_rss = np.sum(
        compressed["cell_centered_ss"]
        + compressed["cell_frequency"]
        * (compressed["cell_mean"] - prediction) ** 2
    )
    assert np.isclose(raw, compressed_rss, atol=2e-15, rtol=2e-15)


def test_exact_target_scale_grouping_does_not_merge_neighbors() -> None:
    data = fixture()
    compressed = compressed_fixture(data)
    last_cell = compressed["row_to_cell"][-1]
    same_cell = compressed["row_to_cell"] == last_cell
    scales = data.target[same_cell] / data.frequency[same_cell]
    assert scales[0] == 1.0
    assert scales[1] == np.nextafter(1.0, np.inf)
    assert np.unique(compressed["row_to_stratum"][same_cell]).size == 2


def test_compensated_sum_retains_cancellation_sensitive_addend() -> None:
    values = np.array([1.0e16, 1.0, -1.0e16])
    assert float(np.sum(values)) == 0.0
    assert compensated_sum(values) == 1.0


def test_cross_cell_deletion_id_is_rejected_by_independent_oracle() -> None:
    data = fixture()
    invalid = Fixture(
        data.worker,
        data.firm,
        data.deletion.copy(),
        data.frequency,
        data.outcome,
        data.target,
    )
    invalid.deletion[4] = invalid.deletion[0]
    try:
        compressed_fixture(invalid)
    except ValueError as error:
        assert "spans coefficient cells" in str(error)
    else:
        raise AssertionError("cross-cell deletion ID was accepted")

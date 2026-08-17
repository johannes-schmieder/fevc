"""Independent oracles for the KSS-SCALE no-control match correction.

The dense side of these tests reproduces the existing match-block calculation
from its mathematical definition.  It deliberately does not import the Mata
implementation or a future compressed-kernel helper.  The compressed side
uses only the proposed deletion-unit and coefficient-cell sufficient
statistics.
"""

from __future__ import annotations

from dataclasses import dataclass
from math import fsum

import numpy as np
import pytest
import sympy as sp


class NonestimableDeletion(ValueError):
    """The rank-one residual maker failed the registered positivity gate."""


@dataclass(frozen=True)
class UnitFixture:
    cell: int
    frequency: np.ndarray
    residual: np.ndarray
    outcome: np.ndarray
    projection_share: float
    finite_bias: float
    finite_variance: float


def _stable_sum(values: np.ndarray | list[float]) -> float:
    """Accumulate one-dimensional binary64 values without orderwise drift."""

    return fsum(float(value) for value in np.asarray(values).ravel())


def _stable_dot(left: np.ndarray, right: np.ndarray) -> float:
    return fsum(
        float(one) * float(two)
        for one, two in zip(np.asarray(left), np.asarray(right), strict=True)
    )


def _maximum_column_relres(residual: np.ndarray, rhs: np.ndarray) -> float:
    relative = []
    for column in range(rhs.shape[1]):
        scale = np.linalg.norm(rhs[:, column])
        value = np.linalg.norm(residual[:, column])
        relative.append(value if scale == 0 else value / scale)
    return max(relative, default=0.0)


def _dense_current_block(
    unit: UnitFixture,
    *,
    block_tolerance: float = 1e-10,
) -> tuple[float, float]:
    """Directly invert the current no-control low-rank match maker.

    Returns the frequency-weighted deleted-residual contraction and the
    complete two-RHS inverse residual.  This follows the current Mata path:
    invert ``I - p uu'`` on both ``sqrt(f) e`` and ``u``, then apply the
    coefficient-one bias and variance terms before contracting by
    ``sqrt(f)``.
    """

    frequency = np.asarray(unit.frequency, dtype=float)
    residual = np.asarray(unit.residual, dtype=float)
    if frequency.ndim != 1 or residual.shape != frequency.shape:
        raise ValueError("invalid deletion-unit vectors")
    if np.any(frequency <= 0) or np.any(frequency != np.floor(frequency)):
        raise ValueError("frequencies must be positive integers")

    square_root_frequency = np.sqrt(frequency)
    common = square_root_frequency / np.sqrt(_stable_sum(frequency))
    factor = np.sqrt(unit.projection_share) * common
    maker = np.eye(frequency.size) - np.outer(factor, factor)
    maker = 0.5 * (maker + maker.T)
    minimum = float(np.linalg.eigvalsh(maker).min())
    if minimum <= block_tolerance:
        raise NonestimableDeletion("dense match residual block is singular")

    transformed_residual = square_root_frequency * residual
    rhs = np.column_stack((transformed_residual, common))
    actions = np.linalg.solve(maker, rhs)
    inverse_residual = maker @ actions - rhs
    relres = _maximum_column_relres(inverse_residual, rhs)

    transformed = actions[:, 0]
    inverse_common = actions[:, 1]
    common_transformed = float(common @ transformed)
    common_inverse = float(common @ inverse_common)
    deleted_adjusted = (
        transformed
        + unit.finite_bias
        * inverse_common
        * common_transformed
        - unit.finite_variance
        * inverse_common
        * common_inverse
        * common_transformed
    )
    return float(square_root_frequency @ deleted_adjusted), relres


def _compressed_unit(
    unit: UnitFixture,
    *,
    block_tolerance: float = 1e-10,
) -> tuple[float, float, float]:
    """Evaluate the proposed exact deletion-unit sufficient-statistic form."""

    residual_mass = _stable_dot(unit.frequency, unit.residual)
    residual_share = 1.0 - unit.projection_share
    if residual_share <= block_tolerance:
        raise NonestimableDeletion("compressed match residual block is singular")
    reciprocal_residual = abs(residual_share * (1.0 / residual_share) - 1.0)
    multiplier = (
        1.0 / residual_share
        + unit.finite_bias / residual_share**2
        - unit.finite_variance / residual_share**3
    )
    return residual_mass * multiplier, residual_mass, reciprocal_residual


def _expand_literal_copies(unit: UnitFixture) -> UnitFixture:
    repeats = np.asarray(unit.frequency, dtype=int)
    return UnitFixture(
        cell=unit.cell,
        frequency=np.ones(int(repeats.sum()), dtype=int),
        residual=np.repeat(unit.residual, repeats),
        outcome=np.repeat(unit.outcome, repeats),
        projection_share=unit.projection_share,
        finite_bias=unit.finite_bias,
        finite_variance=unit.finite_variance,
    )


def _dense_target_draws(
    units: list[UnitFixture], cell_predictions: np.ndarray
) -> np.ndarray:
    terms: list[list[float]] = [[] for _ in range(cell_predictions.shape[1])]
    for unit in units:
        deleted, _ = _dense_current_block(unit)
        outcome_mass = _stable_dot(unit.frequency, unit.outcome)
        for probe, prediction in enumerate(cell_predictions[unit.cell]):
            terms[probe].append(outcome_mass * deleted * float(prediction) ** 2)
    return np.array([fsum(probe_terms) for probe_terms in terms])


def _compressed_target_draws(
    units: list[UnitFixture], cell_predictions: np.ndarray
) -> tuple[np.ndarray, np.ndarray]:
    cell_terms: list[list[float]] = [
        [] for _ in range(cell_predictions.shape[0])
    ]
    for unit in units:
        deleted, _, _ = _compressed_unit(unit)
        outcome_mass = _stable_dot(unit.frequency, unit.outcome)
        cell_terms[unit.cell].append(outcome_mass * deleted)
    cell_weight = np.array([fsum(terms) for terms in cell_terms])
    draws = np.array(
        [
            fsum(
                float(cell_weight[cell])
                * float(cell_predictions[cell, probe]) ** 2
                for cell in range(cell_predictions.shape[0])
            )
            for probe in range(cell_predictions.shape[1])
        ]
    )
    return draws, cell_weight


def test_symbolic_dense_block_reduces_to_registered_scalar_formula() -> None:
    # Perfect-square frequencies keep this direct dense inversion rational,
    # so the identity is exact rather than a floating-point comparison.
    frequency = sp.Matrix([1, 4, 4])
    square_root_frequency = sp.Matrix([1, 2, 2])
    physical_mass = sum(frequency)
    common = square_root_frequency / sp.sqrt(physical_mass)
    residual = sp.Matrix(
        [sp.Rational(7, 5), sp.Rational(-2, 3), sp.Rational(5, 7)]
    )
    projection_share = sp.Rational(2, 5)
    finite_bias = sp.Rational(-3, 125)
    finite_variance = sp.Rational(1, 500)
    maker = sp.eye(3) - projection_share * common * common.T
    transformed = maker.inv() * sp.diag(1, 2, 2) * residual
    inverse_common = maker.inv() * common
    deleted_adjusted = (
        transformed
        + finite_bias * inverse_common * (common.T * transformed)[0]
        - finite_variance
        * inverse_common
        * (common.T * inverse_common)[0]
        * (common.T * transformed)[0]
    )
    dense = (square_root_frequency.T * deleted_adjusted)[0]

    residual_mass = (frequency.T * residual)[0]
    residual_share = 1 - projection_share
    compressed = residual_mass * (
        residual_share**-1
        + finite_bias * residual_share**-2
        - finite_variance * residual_share**-3
    )
    assert sp.simplify(dense - compressed) == 0


@pytest.mark.parametrize("seed", [7, 8675309, 20260816, 20260823])
def test_seeded_dense_units_and_cell_contractions_match(seed: int) -> None:
    rng = np.random.default_rng(seed)
    cells = np.array([0, 0, 1, 2, 2, 3, 1])
    units: list[UnitFixture] = []
    for cell in cells:
        rows = int(rng.integers(2, 7))
        units.append(
            UnitFixture(
                cell=int(cell),
                frequency=rng.integers(1, 6, size=rows),
                residual=rng.normal(size=rows),
                outcome=rng.normal(loc=0.4, scale=2.0, size=rows),
                projection_share=float(rng.uniform(0.02, 0.92)),
                finite_bias=float(rng.normal(scale=2e-3)),
                finite_variance=float(rng.uniform(0, 5e-4)),
            )
        )

    predictions = rng.normal(size=(4, 9))
    dense_draws = _dense_target_draws(units, predictions)
    compressed_draws, cell_weight = _compressed_target_draws(units, predictions)
    assert np.allclose(compressed_draws, dense_draws, rtol=3e-12, atol=3e-12)
    assert np.all(np.isfinite(cell_weight))

    for unit in units:
        dense, inverse_relres = _dense_current_block(unit)
        compressed, _, reciprocal_relres = _compressed_unit(unit)
        assert np.isclose(compressed, dense, rtol=3e-12, atol=3e-12)
        assert inverse_relres < 3e-15
        assert reciprocal_relres < 3e-16


def test_literal_frequencies_match_physical_copy_expansion() -> None:
    units = [
        UnitFixture(
            cell=0,
            frequency=np.array([3, 1, 4]),
            residual=np.array([0.75, -1.25, 0.375]),
            outcome=np.array([1.0, -0.5, 2.25]),
            projection_share=0.37,
            finite_bias=-0.004,
            finite_variance=0.0007,
        ),
        UnitFixture(
            cell=0,
            frequency=np.array([2, 5]),
            residual=np.array([-0.4, 0.9]),
            outcome=np.array([3.0, -1.0]),
            projection_share=0.61,
            finite_bias=0.0025,
            finite_variance=0.0004,
        ),
        UnitFixture(
            cell=1,
            frequency=np.array([1, 2, 3]),
            residual=np.array([1.2, -0.1, -0.3]),
            outcome=np.array([-2.0, 4.0, 0.5]),
            projection_share=0.23,
            finite_bias=-0.001,
            finite_variance=0.0002,
        ),
    ]
    expanded = [_expand_literal_copies(unit) for unit in units]
    predictions = np.array([[0.2, -0.7, 1.1], [1.4, 0.5, -0.3]])

    for stored, copies in zip(units, expanded, strict=True):
        dense_stored, _ = _dense_current_block(stored)
        dense_copies, _ = _dense_current_block(copies)
        compressed_stored, residual_stored, _ = _compressed_unit(stored)
        compressed_copies, residual_copies, _ = _compressed_unit(copies)
        assert np.isclose(dense_stored, dense_copies, rtol=2e-13, atol=2e-13)
        # Regrouping is algebraically exact but need not preserve the last
        # binary64 bit because multiplication by a frequency and literal
        # repeated addition follow different floating-point paths.
        assert np.isclose(
            compressed_stored, compressed_copies, rtol=2e-15, atol=2e-15
        )
        assert np.isclose(
            residual_stored, residual_copies, rtol=2e-15, atol=2e-15
        )

    dense_stored = _dense_target_draws(units, predictions)
    dense_expanded = _dense_target_draws(expanded, predictions)
    compressed_stored, _ = _compressed_target_draws(units, predictions)
    assert np.allclose(dense_stored, dense_expanded, rtol=3e-13, atol=3e-13)
    assert np.allclose(compressed_stored, dense_expanded, rtol=3e-13, atol=3e-13)


def test_parallel_deletion_ids_remain_separate_inside_one_cell() -> None:
    first = UnitFixture(
        cell=0,
        frequency=np.array([2, 1]),
        residual=np.array([1.0, -0.25]),
        outcome=np.array([2.0, -1.0]),
        projection_share=0.21,
        finite_bias=-0.003,
        finite_variance=0.0002,
    )
    second = UnitFixture(
        cell=0,
        frequency=np.array([1, 3, 2]),
        residual=np.array([-0.5, 0.75, 1.25]),
        outcome=np.array([0.5, 3.0, -2.0]),
        projection_share=0.73,
        finite_bias=0.006,
        finite_variance=0.0008,
    )
    third = UnitFixture(
        cell=1,
        frequency=np.array([1, 1]),
        residual=np.array([0.2, -0.4]),
        outcome=np.array([1.0, 2.0]),
        projection_share=0.12,
        finite_bias=-0.001,
        finite_variance=0.0001,
    )
    units = [first, second, third]
    predictions = np.array([[0.3, -1.2], [0.8, 0.25]])
    draws, cell_weight = _compressed_target_draws(units, predictions)
    dense = _dense_target_draws(units, predictions)

    first_deleted, _, _ = _compressed_unit(first)
    second_deleted, _, _ = _compressed_unit(second)
    expected_first_cell = fsum(
        [
            _stable_dot(first.frequency, first.outcome) * first_deleted,
            _stable_dot(second.frequency, second.outcome) * second_deleted,
        ]
    )
    assert len(units) == 3 and len(cell_weight) == 2
    assert first.projection_share != second.projection_share
    assert cell_weight[0] == expected_first_cell
    assert np.allclose(draws, dense, rtol=2e-13, atol=2e-13)


def test_nonestimability_boundary_matches_dense_maker_gate() -> None:
    tolerance = 1e-8
    base = dict(
        cell=0,
        frequency=np.array([1, 3, 2]),
        residual=np.array([0.4, -0.2, 0.7]),
        outcome=np.array([1.0, 2.0, -1.0]),
        finite_bias=2e-10,
        finite_variance=1e-18,
    )

    accepted = UnitFixture(projection_share=1 - 2 * tolerance, **base)
    dense, relres = _dense_current_block(accepted, block_tolerance=tolerance)
    compressed, _, reciprocal_relres = _compressed_unit(
        accepted, block_tolerance=tolerance
    )
    assert np.isclose(compressed, dense, rtol=2e-8, atol=2e-8)
    assert relres < 2e-8
    assert reciprocal_relres < 3e-16

    # Stay on the rejected side of the boundary by more than a few ulps.
    # At the decimal expression ``1-tolerance`` itself, forming ``1-p`` and
    # the dense eigenvalue can round to opposite sides of the threshold.
    for residual_share in (0.9 * tolerance, 0.5 * tolerance):
        rejected = UnitFixture(projection_share=1 - residual_share, **base)
        with pytest.raises(NonestimableDeletion):
            _compressed_unit(rejected, block_tolerance=tolerance)
        with pytest.raises(NonestimableDeletion):
            _dense_current_block(rejected, block_tolerance=tolerance)


def test_compensated_unit_and_cell_sums_survive_adversarial_cancellation() -> None:
    residual_terms = np.array([1e16, 1.0, -1e16])
    assert float(np.sum(residual_terms)) == 0.0
    assert _stable_sum(residual_terms) == 1.0
    assert _stable_sum(residual_terms[::-1]) == 1.0

    unit = UnitFixture(
        cell=0,
        frequency=np.ones(3, dtype=int),
        residual=residual_terms,
        outcome=np.ones(3),
        projection_share=0.25,
        finite_bias=0.01,
        finite_variance=0.001,
    )
    deleted, residual_mass, _ = _compressed_unit(unit)
    multiplier = 1 / 0.75 + 0.01 / 0.75**2 - 0.001 / 0.75**3
    assert residual_mass == 1.0
    assert deleted == multiplier

    cell_terms = np.array([1e16, 1.0, -1e16])
    assert float(np.sum(cell_terms)) == 0.0
    assert _stable_sum(cell_terms) == 1.0
    assert _stable_sum(cell_terms[[2, 0, 1]]) == 1.0

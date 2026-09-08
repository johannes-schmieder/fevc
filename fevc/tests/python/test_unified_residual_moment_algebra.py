"""Independent dense oracles for the unified residual-moment match adapter.

These tests use explicit physical rows, matrices, and NumPy linear algebra.
They do not import production code, its projection helpers, or its fitters.
"""

from dataclasses import dataclass

import numpy as np
from numpy.testing import assert_allclose
import pytest


@dataclass(frozen=True)
class Design:
    coordinates: tuple
    frequency: np.ndarray
    stored_group: np.ndarray
    stored_design: np.ndarray
    physical_design: np.ndarray
    expansion: np.ndarray
    collapse: np.ndarray
    mass: np.ndarray
    target_mass: np.ndarray
    controls: np.ndarray
    outcome: np.ndarray
    known_offset: np.ndarray

    @property
    def groups(self):
        return len(self.coordinates)

    @property
    def collapsed_design(self):
        return self.collapse @ self.physical_design


def _projection(design):
    # A grounded coordinate is only a test representation of the full FE span.
    return design @ np.linalg.solve(design.T @ design, design.T)


@pytest.fixture
def design():
    # Distinct declared matches may share one worker--firm coefficient cell.
    coordinates = tuple((w, f) for w in range(3) for f in range(4)) + (
        (0, 0),
        (2, 3),
    )
    stored_design, stored_group, frequency, target_mass, controls = [], [], [], [], []
    for group, (worker, firm) in enumerate(coordinates):
        frequencies = [1] if group == 0 else [1 + group % 3, 1 + (2 * group + 1) % 4]
        for within, mass in enumerate(frequencies):
            row = np.zeros(6)
            row[worker] = 1.0
            if firm < 3:
                row[3 + firm] = 1.0
            stored_design.append(row)
            stored_group.append(group)
            frequency.append(mass)
            target_mass.append(0.3 + ((group + 3 * within) * 7 % 13) / 8.0)
            controls.append([
                0.2 * worker - 0.1 * firm + 0.17 * within,
                np.sin(group + 0.8 * within),
            ])
    stored_design = np.asarray(stored_design)
    stored_group = np.asarray(stored_group)
    frequency = np.asarray(frequency)
    expansion = np.repeat(np.eye(len(frequency)), frequency, axis=0)
    physical_design = expansion @ stored_design
    physical_group = np.repeat(stored_group, frequency)
    mass = np.bincount(stored_group, weights=frequency)
    collapse = np.zeros((len(coordinates), len(physical_group)))
    collapse[physical_group, np.arange(len(physical_group))] = 1.0 / np.sqrt(mass[physical_group])
    controls = expansion @ np.asarray(controls)
    known_offset = controls @ np.array([0.7, -0.4])
    error = expansion @ np.cos(0.73 * np.arange(len(frequency)))
    outcome = physical_design @ (0.2 * np.arange(1, 7)) + known_offset + error
    return Design(
        coordinates, frequency, stored_group, stored_design, physical_design,
        expansion, collapse, mass, np.asarray(target_mass), controls, outcome,
        known_offset,
    )


def _basis(groups):
    # The moment identity holds for any fixed full-rank variance basis.
    return np.column_stack([
        np.ones(groups), np.linspace(-1.0, 1.0, groups), np.cos(np.arange(groups)),
    ])


def _block_covariance(design, variance, kind):
    covariance = np.zeros((len(design.outcome), len(design.outcome)))
    for group in range(design.groups):
        indices = np.flatnonzero(design.collapse[group])
        size = len(indices)
        position = np.arange(size)
        unit = np.ones(size) / np.sqrt(size)
        if kind == "independent":
            block = np.diag(1.0 + 0.1 * position)
        elif kind == "common":
            block = np.ones((size, size)) + 0.2 * np.eye(size)
        elif kind == "serial":
            block = 0.65 ** np.abs(position[:, None] - position[None, :])
        elif kind == "contrast":
            average = np.outer(unit, unit)
            block = average + 4.0 * (np.eye(size) - average)
        else:
            raise AssertionError(f"Unknown covariance block: {kind}")
        assert np.linalg.eigvalsh(block).min() > 0.0
        covariance[np.ix_(indices, indices)] = block * variance[group] / (unit @ block @ unit)
    return covariance


def _target(design, target_mass):
    firm = design.copy()
    firm[:, :3] = 0.0
    weight = target_mass / target_mass.sum()
    centered = firm - weight @ firm
    return centered.T @ (weight[:, None] * centered)


def test_weighted_collapse_preserves_fit_projection_and_residuals(design):
    physical = design.physical_design
    collapsed = design.collapsed_design
    projection = _projection(physical)
    collapsed_projection = _projection(collapsed)
    maker = np.eye(len(physical)) - projection
    collapsed_maker = np.eye(design.groups) - collapsed_projection
    adjusted = design.outcome - design.known_offset
    coefficients = np.linalg.lstsq(physical, adjusted, rcond=None)[0]
    collapsed_coefficients = np.linalg.lstsq(collapsed, design.collapse @ adjusted, rcond=None)[0]
    assert_allclose(collapsed.T @ collapsed, physical.T @ physical, atol=2e-14, rtol=2e-14)
    assert_allclose(coefficients, collapsed_coefficients, atol=2e-14, rtol=2e-14)
    assert_allclose(projection, design.collapse.T @ collapsed_projection @ design.collapse, atol=2e-14)
    assert_allclose(design.collapse @ maker, collapsed_maker @ design.collapse, atol=2e-14)
    assert_allclose(design.collapse @ maker @ adjusted, collapsed_maker @ design.collapse @ adjusted, atol=2e-14)
    # Squaring first retains within-match residual contrasts and is not the fitter response.
    physical_residual = maker @ adjusted
    correct = (design.collapse @ physical_residual) ** 2
    incorrect = (design.collapse ** 2) @ (physical_residual ** 2)
    assert np.max(np.abs(correct - incorrect)) > 0.01


@pytest.mark.parametrize("kind", ["independent", "common", "serial", "contrast"])
def test_exact_residual_moments_recover_aggregate_variance_model(design, kind):
    basis = _basis(design.groups)
    coefficients = np.array([1.5, 0.3, 0.2])
    variance = basis @ coefficients
    covariance = _block_covariance(design, variance, kind)
    physical_maker = np.eye(len(design.outcome)) - _projection(design.physical_design)
    maker = np.eye(design.groups) - _projection(design.collapsed_design)
    aggregate_covariance = design.collapse @ covariance @ design.collapse.T
    residual_covariance = design.collapse @ physical_maker @ covariance @ physical_maker.T @ design.collapse.T
    information = basis.T @ (maker * maker) @ basis
    expected_moment = basis.T @ np.diag(residual_covariance)
    assert_allclose(aggregate_covariance, np.diag(variance), atol=2e-14)
    assert_allclose(residual_covariance, maker @ np.diag(variance) @ maker.T, atol=2e-14)
    assert_allclose(expected_moment, information @ coefficients, atol=2e-13)
    assert_allclose(np.linalg.solve(information, expected_moment), coefficients, atol=2e-13)


def test_direct_probe_gram_agrees_before_and_after_weighted_collapse(design):
    rng = np.random.default_rng(20260908)
    gaussian = rng.standard_normal((len(design.outcome), 512))
    physical_maker = np.eye(len(design.outcome)) - _projection(design.physical_design)
    maker = np.eye(design.groups) - _projection(design.collapsed_design)
    physical_residual = design.collapse @ physical_maker @ gaussian
    collapsed_residual = maker @ (design.collapse @ gaussian)
    basis = _basis(design.groups)
    first = (basis.T @ (physical_residual ** 2)).T
    second = (basis.T @ (collapsed_residual ** 2)).T
    assert_allclose(first, second, rtol=2e-14, atol=2e-13)
    assert_allclose(np.cov(first, rowvar=False, ddof=1) / 2,
                    np.cov(second, rowvar=False, ddof=1) / 2, rtol=2e-14, atol=2e-13)


def test_frequency_copies_match_stored_row_covariance_contraction(design):
    stored_covariance = np.zeros((len(design.frequency), len(design.frequency)))
    expected = np.zeros(design.groups)
    for group in range(design.groups):
        indices = np.flatnonzero(design.stored_group == group)
        position = np.arange(len(indices))
        block = 0.4 ** np.abs(position[:, None] - position[None, :])
        block += np.diag(0.2 + 0.1 * position)
        stored_covariance[np.ix_(indices, indices)] = block
        frequency = design.frequency[indices]
        expected[group] = frequency @ block @ frequency / design.mass[group]
    # Literal copies of a stored random variable retain its covariance, rather
    # than becoming independently sampled errors when the row is expanded.
    physical_covariance = design.expansion @ stored_covariance @ design.expansion.T
    actual = design.collapse @ physical_covariance @ design.collapse.T
    assert_allclose(actual, np.diag(expected), atol=2e-14)


def test_target_weights_are_distinct_from_regression_frequency(design):
    physical = design.physical_design
    collapsed = design.collapsed_design
    stored_target = _target(design.stored_design, design.target_mass)
    # Each literal frequency copy receives an equal share of stored target mass.
    physical_target_mass = design.expansion @ (design.target_mass / design.frequency)
    physical_target = _target(physical, physical_target_mass)
    inverse = np.linalg.inv(physical.T @ physical)
    physical_kernel = physical @ inverse @ stored_target @ inverse @ physical.T
    collapsed_kernel = collapsed @ inverse @ stored_target @ inverse @ collapsed.T
    assert_allclose(stored_target, physical_target, atol=2e-14)
    assert_allclose(design.collapse @ physical_kernel @ design.collapse.T, collapsed_kernel, atol=2e-14)
    block_trace = np.array([
        np.diag(physical_kernel)[design.collapse[group] != 0].sum()
        for group in range(design.groups)
    ])
    assert_allclose(block_trace, np.diag(collapsed_kernel), atol=2e-14)
    wrong_target = _target(physical, design.expansion @ design.target_mass)
    assert np.max(np.abs(wrong_target - stored_target)) > 0.01


def test_independent_matches_are_not_coefficient_cells_or_frequency_copies(design):
    assert design.groups == 14
    assert len(set(design.coordinates)) == 12
    assert len(design.frequency) > design.groups
    assert len(design.outcome) > len(design.frequency)
    assert design.mass[0] == 1
    assert design.coordinates[0] == design.coordinates[12]
    # This contrast between two independent matches sharing a cell is a real
    # residual direction. Merging those matches would erase it.
    contrast = np.zeros(design.groups)
    contrast[0] = np.sqrt(design.mass[12])
    contrast[12] = -np.sqrt(design.mass[0])
    contrast /= np.linalg.norm(contrast)
    maker = np.eye(design.groups) - _projection(design.collapsed_design)
    assert_allclose(design.collapsed_design.T @ contrast, 0.0, atol=2e-14)
    assert_allclose(maker @ contrast, contrast, atol=2e-14)
    assert_allclose(np.diag(design.collapse @ design.collapse.T), 1.0, atol=2e-14)


def test_within_match_contrast_covariance_disappears_after_collapse(design):
    variance = _basis(design.groups) @ np.array([1.5, 0.3, 0.2])
    independent = _block_covariance(design, variance, "independent")
    contrasting = _block_covariance(design, variance, "contrast")
    physical_maker = np.eye(len(design.outcome)) - _projection(design.physical_design)
    delta = contrasting - independent
    assert np.max(np.abs(delta)) > 1.0
    assert_allclose(design.collapse @ delta @ design.collapse.T, 0.0, atol=2e-14)
    assert_allclose(design.collapse @ physical_maker @ delta @ physical_maker.T @ design.collapse.T, 0.0, atol=2e-14)
    # The discarded physical contrasts still have positive sampling variation.
    contrasts = np.eye(len(design.outcome)) - design.collapse.T @ design.collapse
    assert np.trace(contrasts @ contrasting @ contrasts.T) > 1.0


def test_known_and_same_sample_estimated_offsets_have_different_moments(design):
    physical = design.physical_design
    controls = design.controls
    full_design = np.column_stack([physical, controls])
    physical_maker = np.eye(len(physical)) - _projection(physical)
    full_maker = np.eye(len(physical)) - _projection(full_design)
    maker = np.eye(design.groups) - _projection(design.collapsed_design)
    full_coefficients = np.linalg.lstsq(full_design, design.outcome, rcond=None)[0]
    estimated_offset = controls @ full_coefficients[-controls.shape[1]:]
    actual_residual = design.collapse @ full_maker @ design.outcome
    assert_allclose(actual_residual, maker @ design.collapse @ (design.outcome - estimated_offset), atol=2e-13)
    known_residual = design.collapse @ physical_maker @ (design.outcome - design.known_offset)
    assert np.max(np.abs(actual_residual - known_residual)) > 0.01

    basis = _basis(design.groups)
    variance = basis @ np.array([1.5, 0.3, 0.2])
    covariance = _block_covariance(design, variance, "serial")
    known_covariance = design.collapse @ physical_maker @ covariance @ physical_maker.T @ design.collapse.T
    estimated_covariance = design.collapse @ full_maker @ covariance @ full_maker.T @ design.collapse.T
    naive = maker @ np.diag(variance) @ maker.T
    assert_allclose(known_covariance, naive, atol=2e-13)
    assert np.max(np.abs(estimated_covariance - naive)) > 0.001
    assert np.max(np.abs(basis.T @ np.diag(estimated_covariance - naive))) > 0.001

    # Offset estimation alone couples originally independent aggregate errors.
    offset_estimator = np.linalg.solve(full_design.T @ full_design, full_design.T)[-controls.shape[1]:]
    offset_adjustment = np.eye(len(physical)) - controls @ offset_estimator
    adjusted_covariance = design.collapse @ offset_adjustment @ covariance @ offset_adjustment.T @ design.collapse.T
    off_diagonal = adjusted_covariance - np.diag(np.diag(adjusted_covariance))
    assert np.max(np.abs(off_diagonal)) > 0.001
    assert_allclose(estimated_covariance, maker @ adjusted_covariance @ maker.T, atol=2e-13)


def test_constant_variance_fit_is_collapsed_rss_over_match_residual_df(design):
    projection = _projection(design.collapsed_design)
    maker = np.eye(design.groups) - projection
    ones = np.ones(design.groups)
    degrees_of_freedom = design.groups - np.linalg.matrix_rank(design.collapsed_design)
    information = ones @ (maker * maker) @ ones
    residual = maker @ design.collapse @ (design.outcome - design.known_offset)
    estimate = (ones @ residual ** 2) / information
    assert_allclose(information, degrees_of_freedom, atol=2e-14)
    assert_allclose(estimate, (residual @ residual) / degrees_of_freedom, atol=2e-14)
    assert degrees_of_freedom == 8
    wrong_df = len(design.outcome) - np.linalg.matrix_rank(design.physical_design)
    assert abs(estimate - (residual @ residual) / wrong_df) > 0.01
    covariance = _block_covariance(design, np.full(design.groups, 1.7), "contrast")
    expected_rss = np.trace(maker @ design.collapse @ covariance @ design.collapse.T @ maker.T)
    assert_allclose(expected_rss / degrees_of_freedom, 1.7, atol=2e-14)


def test_gaussian_projected_moment_covariance_gives_exact_information(design):
    projection = _projection(design.collapsed_design)
    maker = np.eye(design.groups) - projection
    basis = _basis(design.groups)
    # For independent standard Gaussian probes, covariance of quadratic forms
    # is 2 tr(A B); this dense calculation uses no sampled probe approximation.
    quadratic = [projection @ np.diag(column) @ projection for column in basis.T]
    covariance = np.array([[2.0 * np.trace(left @ right) for right in quadratic] for left in quadratic])
    direct = basis.T @ np.diag(1.0 - 2.0 * np.diag(projection)) @ basis
    information = basis.T @ (maker * maker) @ basis
    assert_allclose(direct + 0.5 * covariance, information, atol=2e-13)

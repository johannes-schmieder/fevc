from __future__ import annotations

import itertools

import numpy as np


def physical_directions(copies: int) -> np.ndarray:
    return np.asarray(
        list(itertools.product((-1.0, 1.0), repeat=copies)),
        dtype=float,
    )


def stored_sums(
    directions: np.ndarray,
    frequency: np.ndarray,
) -> tuple[np.ndarray, list[np.ndarray]]:
    groups: list[np.ndarray] = []
    begin = 0
    for count in frequency:
        groups.append(np.arange(begin, begin + count))
        begin += count
    return (
        np.column_stack([directions[:, group].sum(axis=1) for group in groups]),
        groups,
    )


def test_observation_probe_moments_equal_literal_copy_aggregation() -> None:
    frequency = np.array([2, 1, 2], dtype=int)
    stored_design = np.array([[1.0, 0.0], [1.0, 1.0], [1.0, 2.0]])
    expanded_design = np.repeat(stored_design, frequency, axis=0)
    projection = expanded_design @ np.linalg.inv(
        expanded_design.T @ expanded_design
    ) @ expanded_design.T
    residual_maker = np.eye(frequency.sum()) - projection
    directions = physical_directions(int(frequency.sum()))
    sums, groups = stored_sums(directions, frequency)

    inverse = np.linalg.inv(
        stored_design.T @ (frequency[:, None] * stored_design)
    )
    projected = sums @ (stored_design @ inverse @ stored_design.T).T

    for stored_row, group in enumerate(groups):
        p = projected[:, stored_row]
        average_sign = sums[:, stored_row] / frequency[stored_row]
        collapsed_m2 = 1 + p**2 - 2 * p * average_sign
        collapsed_m4 = (
            1
            + 6 * p**2
            + p**4
            - 4 * p * (1 + p**2) * average_sign
        )
        collapsed_mixed = p**2 * collapsed_m2

        physical_p = (directions @ projection.T)[:, group]
        physical_m = (directions @ residual_maker.T)[:, group]
        assert np.allclose(physical_p, p[:, None], atol=2e-15, rtol=0)
        assert np.allclose(
            collapsed_m2,
            np.mean(physical_m**2, axis=1),
            atol=3e-15,
            rtol=0,
        )
        assert np.allclose(
            collapsed_m4,
            np.mean(physical_m**4, axis=1),
            atol=8e-15,
            rtol=0,
        )
        assert np.allclose(
            collapsed_mixed,
            np.mean(physical_p**2 * physical_m**2, axis=1),
            atol=5e-15,
            rtol=0,
        )


def test_match_probe_contraction_equals_literal_copy_direction() -> None:
    frequency = np.array([2, 1, 2], dtype=int)
    # The first two stored rows form one match and share an FE design row.
    stored_design = np.array([[1.0, 0.0], [1.0, 0.0], [1.0, 1.0]])
    expanded_design = np.repeat(stored_design, frequency, axis=0)
    projection = expanded_design @ np.linalg.inv(
        expanded_design.T @ expanded_design
    ) @ expanded_design.T
    residual_maker = np.eye(frequency.sum()) - projection
    directions = physical_directions(int(frequency.sum()))
    sums, groups = stored_sums(directions, frequency)
    match = np.concatenate((groups[0], groups[1]))
    match_frequency = frequency[0] + frequency[1]
    match_direction = np.ones(match_frequency) / np.sqrt(match_frequency)

    physical_projection = (directions @ projection.T)[:, match]
    physical_residual = (directions @ residual_maker.T)[:, match]
    literal_p = physical_projection @ match_direction
    literal_m = physical_residual @ match_direction

    inverse = np.linalg.inv(
        stored_design.T @ (frequency[:, None] * stored_design)
    )
    projected = sums @ (stored_design @ inverse @ stored_design.T).T
    collapsed_p = (
        frequency[0] * projected[:, 0]
        + frequency[1] * projected[:, 1]
    ) / np.sqrt(match_frequency)
    collapsed_m = (sums[:, 0] + sums[:, 1]) / np.sqrt(
        match_frequency
    ) - collapsed_p

    assert np.allclose(collapsed_p, literal_p, atol=2e-15, rtol=0)
    assert np.allclose(collapsed_m, literal_m, atol=2e-15, rtol=0)
    for power_p, power_m in ((2, 0), (0, 2), (4, 0), (0, 4), (2, 2)):
        assert np.allclose(
            collapsed_p**power_p * collapsed_m**power_m,
            literal_p**power_p * literal_m**power_m,
            atol=2e-14,
            rtol=0,
        )


def test_target_probe_centering_equals_literal_copy_construction() -> None:
    frequency = np.array([2, 1, 2], dtype=int)
    target_mass = np.array([0.8, 1.3, 0.5])
    directions = physical_directions(int(frequency.sum()))
    sums, groups = stored_sums(directions, frequency)
    total = target_mass.sum()

    copy_mass = np.repeat(target_mass / frequency, frequency) / total
    literal_uncentered = np.sqrt(copy_mass)[None, :] * directions
    literal = literal_uncentered - copy_mass[None, :] * literal_uncentered.sum(
        axis=1, keepdims=True
    )
    literal_stored = np.column_stack(
        [literal[:, group].sum(axis=1) for group in groups]
    )

    first = np.sqrt(target_mass / (frequency * total))[None, :] * sums
    collapsed = first - (target_mass / total)[None, :] * first.sum(
        axis=1, keepdims=True
    )
    assert np.allclose(collapsed, literal_stored, atol=3e-16, rtol=0)

    covariance = collapsed.T @ collapsed / directions.shape[0]
    share = target_mass / total
    assert np.allclose(
        covariance,
        np.diag(share) - np.outer(share, share),
        atol=3e-15,
        rtol=0,
    )


def test_two_one_frequency_mapping_has_physical_fourth_moments() -> None:
    frequency = np.array([2, 1], dtype=int)
    directions = physical_directions(int(frequency.sum()))
    sums, groups = stored_sums(directions, frequency)
    compressed = sums / np.sqrt(frequency)[None, :]
    assert np.allclose(
        np.mean(compressed**4, axis=0),
        3 - 2 / frequency,
        atol=2e-15,
        rtol=0,
    )

    stored_design = np.sqrt(frequency)[:, None]
    weighted_projection = stored_design @ np.linalg.inv(
        stored_design.T @ stored_design
    ) @ stored_design.T
    expanded_design = np.ones((frequency.sum(), 1))
    physical_projection = expanded_design @ np.linalg.inv(
        expanded_design.T @ expanded_design
    ) @ expanded_design.T
    for row, group in enumerate(groups):
        assert np.allclose(
            np.diag(physical_projection)[group],
            weighted_projection[row, row] / frequency[row],
            atol=2e-15,
            rtol=0,
        )

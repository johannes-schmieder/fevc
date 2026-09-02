from __future__ import annotations

import numpy as np
import pytest

from fevc.tests.python.oracle import (
    OracleError,
    build_expanded_design,
    deleted_residuals_by_refit,
    exact_kss,
    exact_projection,
)


def fixture() -> dict[str, np.ndarray]:
    worker = np.repeat(np.arange(6), 4)
    firm = np.array(
        [0, 0, 1, 1, 0, 2, 2, 1, 1, 2, 3, 3, 2, 3, 0, 0, 3, 1, 1, 2, 3, 3, 2, 0]
    )
    time = np.tile(np.arange(4), 6)
    controls = np.column_stack((time - time.mean(), (time == 2).astype(float)))
    y = 1.5 + 0.3 * worker - 0.2 * firm + controls @ np.array([0.4, -0.15])
    y = y + np.array(
        [0.2, -0.1, 0.1, -0.2, -0.2, 0.3, -0.1, 0.1, 0.1, -0.2, 0.2, -0.1,
         -0.1, 0.2, -0.2, 0.1, 0.3, -0.2, 0.1, -0.2, -0.2, 0.1, 0.2, -0.1]
    )
    match = np.array(
        [10, 10, 11, 11, 20, 21, 21, 22, 30, 31, 32, 32, 40, 41, 42, 42, 50, 51, 51, 52, 60, 60, 61, 62]
    )
    return {"y": y, "worker": worker, "firm": firm, "controls": controls, "match": match}


def model_data(data: dict[str, np.ndarray]) -> dict[str, np.ndarray]:
    return {key: data[key] for key in ("y", "worker", "firm", "controls")}


def test_observation_and_match_oracles_account_exactly() -> None:
    data = fixture()
    observation = exact_kss(**model_data(data), deletion="observation")
    match = exact_kss(**model_data(data), deletion="match", deletion_id=data["match"])
    for result in (observation, match):
        assert np.isfinite(result.corrected).all()
        assert np.isclose(result.plugin[3], result.plugin[0] + result.plugin[1] + 2 * result.plugin[2])
        assert np.isclose(
            result.correction[3],
            result.correction[0] + result.correction[1] + 2 * result.correction[2],
        )
    assert match.deletion_units < observation.deletion_units


def test_fixedoffset_matches_registered_dense_fixture() -> None:
    data = fixture()
    result = exact_kss(
        **model_data(data),
        deletion="match",
        deletion_id=data["match"],
        nuisance="fixedoffset",
    )
    expected_plugin = np.array(
        [
            0.23886949630417761,
            0.02942901912684345,
            -0.02694262626105621,
            0.21441326290890872,
        ]
    )
    expected_correction = np.array(
        [
            -0.00467505214166742,
            0.00105611329461117,
            -0.00744004087996870,
            -0.01849902060699364,
        ]
    )
    assert np.allclose(result.plugin, expected_plugin, atol=2e-13, rtol=2e-13)
    assert np.allclose(result.correction, expected_correction, atol=2e-13, rtol=2e-13)


def test_block_woodbury_residual_equals_direct_refit() -> None:
    data = fixture()
    design = build_expanded_design(
        **model_data(data), deletion="match", deletion_id=data["match"]
    )
    inverse = np.linalg.inv(design.x.T @ design.x)
    beta = inverse @ design.x.T @ design.y
    residual = design.y - design.x @ beta
    for indices, direct in deleted_residuals_by_refit(design):
        xg = design.x[indices]
        pg = xg @ inverse @ xg.T
        identity = np.linalg.solve(np.eye(indices.size) - pg, residual[indices])
        assert np.allclose(identity, direct, atol=2e-10, rtol=2e-10)


def test_frequency_weights_equal_literal_expansion() -> None:
    data = fixture()
    frequency = np.where(np.arange(data["y"].size) % 3 == 0, 2, 1)
    target = 0.5 + np.arange(data["y"].size) / data["y"].size
    weighted = exact_kss(
        **model_data(data),
        frequency=frequency,
        target_weight=target,
        deletion="match",
        deletion_id=data["match"],
    )

    index = np.repeat(np.arange(data["y"].size), frequency)
    expanded_target = np.concatenate(
        [np.repeat(target[row] / frequency[row], frequency[row]) for row in range(target.size)]
    )
    expanded = exact_kss(
        data["y"][index],
        data["worker"][index],
        data["firm"][index],
        controls=data["controls"][index],
        target_weight=expanded_target,
        deletion="match",
        deletion_id=data["match"][index],
    )
    assert np.allclose(weighted.plugin, expanded.plugin, atol=1e-11)
    assert np.allclose(weighted.correction, expanded.correction, atol=2e-10)


def test_observation_frequency_deletes_one_physical_copy() -> None:
    data = fixture()
    frequency = np.where(np.arange(data["y"].size) % 3 == 0, 2, 1)
    target = 0.5 + np.arange(data["y"].size) / data["y"].size
    weighted = exact_kss(
        **model_data(data),
        frequency=frequency,
        target_weight=target,
        deletion="observation",
    )
    index = np.repeat(np.arange(data["y"].size), frequency)
    expanded_target = np.concatenate(
        [np.repeat(target[row] / frequency[row], frequency[row]) for row in range(target.size)]
    )
    expanded = exact_kss(
        data["y"][index],
        data["worker"][index],
        data["firm"][index],
        controls=data["controls"][index],
        target_weight=expanded_target,
        deletion="observation",
    )
    assert np.allclose(weighted.plugin, expanded.plugin, atol=1e-11)
    assert np.allclose(weighted.correction, expanded.correction, atol=2e-10)


def test_actual_matches_with_same_coordinates_remain_separate() -> None:
    data = fixture()
    split = data["match"].copy()
    split[1] = 99
    result = exact_kss(**model_data(data), deletion="match", deletion_id=split)
    cell = exact_kss(
        **model_data(data), deletion="match"
    )
    assert result.deletion_units != cell.deletion_units


def test_cross_coordinate_match_id_is_rejected() -> None:
    data = fixture()
    invalid = data["match"].copy()
    invalid[4] = invalid[0]
    with pytest.raises(OracleError, match="one worker-firm coordinate"):
        exact_kss(**model_data(data), deletion="match", deletion_id=invalid)


def test_invalid_frequency_is_rejected() -> None:
    data = fixture()
    frequency = np.ones(data["y"].size)
    frequency[0] = 1.5
    with pytest.raises(OracleError, match="positive integers"):
        exact_kss(**model_data(data), frequency=frequency)


def test_seven_edge_projection_regression_rejects_centered_proxy() -> None:
    """Pin the referee counterexample without using production code."""

    edges = [(2, 2), (1, 1), (0, 2), (2, 1), (1, 0), (0, 1), (2, 0)]
    x = np.zeros((7, 5))
    for row, (worker, firm) in enumerate(edges):
        x[row, worker] = 1
        if firm < 2:  # firm 2 is the grounded level
            x[row, 3 + firm] = 1
    inverse = np.linalg.inv(x.T @ x)
    hat = x @ inverse @ x.T
    leverage = np.diag(hat)
    assert np.allclose(leverage[:3], 11 / 15)
    assert np.isclose(leverage[3], 3 / 5)
    assert np.allclose(leverage[4:], 11 / 15)

    # Regress the three grounded firm effects at coordinates (0,1,2) on a
    # constant and slope. The slope loading is -.5 on firm 0 and zero on
    # firm 1 because firm 2 is grounded at zero.
    loading = np.array([0.0, 0.0, 0.0, -0.5, 0.0])
    score = x @ inverse @ loading
    sigma = np.array([1.0] * 6 + [4.0])
    true_variance = float(np.sum(sigma * score**2))

    residual_maker = np.eye(7) - hat
    centered_expectation = sigma - (residual_maker @ sigma) / (7 * (1 - leverage))
    old_centered = float(np.sum(centered_expectation * score**2))
    corrected_uncentered = float(np.sum(sigma * score**2))
    assert np.isclose(true_variance, 2 / 3, atol=2e-15)
    assert np.isclose(corrected_uncentered, 2 / 3, atol=2e-15)
    assert np.isclose(old_centered, 53 / 84, atol=2e-15)
    assert np.isclose(old_centered - true_variance, -1 / 28, atol=2e-15)


def test_projection_frequency_compression_matches_literal_copies() -> None:
    data = fixture()
    project = np.column_stack(
        (np.sin(np.arange(data["y"].size) / 3), data["firm"] / 5)
    )
    frequency = np.where(np.arange(data["y"].size) % 3 == 0, 2, 1)
    target = 0.5 + np.arange(data["y"].size) / data["y"].size
    compressed = exact_projection(
        **model_data(data),
        project=project,
        frequency=frequency,
        target_weight=target,
        deletion="match",
        deletion_id=data["match"],
        effect="firm",
        project_weight="target",
    )
    index = np.repeat(np.arange(data["y"].size), frequency)
    expanded_target = np.concatenate(
        [np.repeat(target[row] / frequency[row], frequency[row]) for row in range(target.size)]
    )
    expanded = exact_projection(
        data["y"][index],
        data["worker"][index],
        data["firm"][index],
        project[index],
        controls=data["controls"][index],
        target_weight=expanded_target,
        deletion="match",
        deletion_id=data["match"][index],
        effect="firm",
        project_weight="target",
    )
    assert np.allclose(compressed.coefficients, expanded.coefficients, atol=2e-11)
    assert np.allclose(compressed.covariance, expanded.covariance, atol=3e-10)
    assert np.allclose(compressed.naive_covariance, expanded.naive_covariance, atol=3e-10)


def test_projection_joint_controls_and_eligible_stayer_use_mixed_blocks() -> None:
    data = fixture()
    y = np.append(data["y"], 2.1)
    worker = np.append(data["worker"], 99)
    firm = np.append(data["firm"], 0)
    controls = np.vstack((data["controls"], [0.25, 1.0]))
    match = np.append(data["match"], 999)
    project = np.sin(np.arange(y.size) / 4) + firm / 7
    frequency = np.append(np.ones(data["y"].size, dtype=int), 2)
    stayer = np.append(np.zeros(data["y"].size), 1)
    result = exact_projection(
        y,
        worker,
        firm,
        project,
        controls=controls,
        frequency=frequency,
        deletion="match",
        deletion_id=match,
        stayer=stayer,
        nuisance="joint",
        effect="worker",
    )
    mover_units = np.unique(data["match"]).size
    assert result.deletion_units == mover_units + 2
    assert np.isfinite(result.coefficients).all()
    assert np.isfinite(result.covariance).all()
    assert np.allclose(result.covariance, result.covariance.T, atol=1e-13)


def test_projection_slopes_are_location_invariant_but_intercept_is_not() -> None:
    data = fixture()
    z = np.column_stack((data["firm"] / 3, np.sin(np.arange(data["y"].size))))
    mass = np.ones(data["y"].size)
    design = np.column_stack((np.ones(data["y"].size), z))
    gram_inverse = np.linalg.inv(design.T @ (mass[:, None] * design))
    effect = 0.2 * data["firm"] + np.cos(data["firm"])
    base = gram_inverse @ design.T @ (mass * effect)
    shift = 1.75
    shifted = gram_inverse @ design.T @ (mass * (effect - shift))
    assert np.isclose(shifted[0], base[0] - shift, atol=2e-14)
    assert np.allclose(shifted[1:], base[1:], atol=2e-14)


def test_block_projection_monte_carlo_tracks_covariance_and_interval_behavior() -> None:
    """Exercise many independent correlated match blocks at a fixed design."""

    workers = 40
    periods = 20
    n = workers * periods
    worker = np.repeat(np.arange(workers), periods)
    time = np.tile(np.arange(periods), workers)
    firm = (worker + np.floor(time / 2).astype(int)) % 20
    match = worker * (periods // 2) + np.floor(time / 2).astype(int)
    project = np.sin(worker / 5) + np.cos(firm / 3) + time / 20
    baseline_y = np.sin(np.arange(n))
    oracle = exact_projection(
        baseline_y,
        worker,
        firm,
        project,
        deletion="match",
        deletion_id=match,
        effect="firm",
    )
    expanded = build_expanded_design(
        baseline_y, worker, firm, deletion="match", deletion_id=match
    )
    x = expanded.x
    score = oracle.score
    blocks = [
        np.flatnonzero(expanded.deletion_id == code)
        for code in np.unique(expanded.deletion_id)
    ]
    replications = 3_000
    rng = np.random.default_rng(20260902)
    errors = np.zeros((replications, n))
    true_covariance = np.zeros((2, 2))
    for block in blocks:
        standard_deviation = 0.4 + 0.0002 * block
        sigma = np.outer(standard_deviation, standard_deviation) * 0.45
        np.fill_diagonal(sigma, standard_deviation**2)
        errors[:, block] = rng.multivariate_normal(
            np.zeros(block.size), sigma, size=replications
        )
        true_covariance += score[block].T @ sigma @ score[block]

    beta = np.arange(x.shape[1]) * 0.03
    outcomes = x @ beta + errors
    estimated = np.zeros((replications, 2, 2))
    information = x.T @ x
    all_rows = np.arange(n)
    for block in blocks:
        keep = np.setdiff1d(all_rows, block)
        deleted_projection = x[block] @ np.linalg.solve(
            information - x[block].T @ x[block], x[keep].T
        )
        deleted_residual = outcomes[:, block] - outcomes[:, keep] @ deleted_projection.T
        score_y = outcomes[:, block] @ score[block]
        score_deleted = deleted_residual @ score[block]
        estimated += 0.5 * (
            score_y[:, :, None] * score_deleted[:, None, :]
            + score_deleted[:, :, None] * score_y[:, None, :]
        )

    mean_covariance = estimated.mean(axis=0)
    assert np.allclose(mean_covariance, true_covariance, rtol=0.025, atol=2e-5)
    slope_error = errors @ score[:, 1]
    valid = estimated[:, 1, 1] > 0
    covered = valid & (
        np.abs(slope_error) <= 1.959963984540054 * np.sqrt(np.maximum(estimated[:, 1, 1], 0))
    )
    assert valid.mean() >= 0.995
    assert 0.925 <= covered.mean() <= 0.96

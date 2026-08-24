from __future__ import annotations

import numpy as np
import pytest

from vckss.tests.python.oracle import (
    OracleError,
    build_expanded_design,
    deleted_residuals_by_refit,
    exact_kss,
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

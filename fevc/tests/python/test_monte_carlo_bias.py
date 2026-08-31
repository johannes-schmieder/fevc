from __future__ import annotations

import numpy as np

from fevc.tests.python.oracle import build_expanded_design, exact_kss, target_matrices
from fevc.tests.python.test_dense_oracle import fixture


def test_exact_kss_removes_fixed_design_observation_bias() -> None:
    data = fixture()
    template = build_expanded_design(
        data["y"],
        data["worker"],
        data["firm"],
        controls=data["controls"],
        deletion="observation",
    )
    beta = np.array(
        [-0.8, -0.45, -0.1, 0.25, 0.65, 1.05, 0.35, -0.4, 0.55, 0.3, -0.2]
    )
    signal = template.x @ beta
    truth = np.array([beta @ target @ beta for target in target_matrices(template)])

    rng = np.random.default_rng(20260816)
    repetitions = 1_000
    plugin = np.empty((repetitions, 4))
    corrected = np.empty((repetitions, 4))
    for repetition in range(repetitions):
        outcome = signal + rng.normal(scale=0.7, size=signal.size)
        result = exact_kss(
            outcome,
            data["worker"],
            data["firm"],
            controls=data["controls"],
            deletion="observation",
        )
        plugin[repetition] = result.plugin
        corrected[repetition] = result.corrected

    plugin_bias = plugin.mean(axis=0) - truth
    corrected_bias = corrected.mean(axis=0) - truth
    # The registered seed leaves a clear finite-sample gap for every target.
    assert np.all(np.abs(corrected_bias) < 0.25 * np.abs(plugin_bias))
    assert np.allclose(
        corrected.mean(axis=0),
        truth,
        atol=4 * corrected.std(axis=0, ddof=1) / np.sqrt(repetitions),
        rtol=0,
    )


def test_exact_kss_removes_bias_with_independent_match_blocks() -> None:
    data = fixture()
    template = build_expanded_design(
        data["y"],
        data["worker"],
        data["firm"],
        controls=data["controls"],
        deletion="match",
        deletion_id=data["match"],
    )
    beta = np.array(
        [-0.8, -0.45, -0.1, 0.25, 0.65, 1.05, 0.35, -0.4, 0.55, 0.3, -0.2]
    )
    signal = template.x @ beta
    truth = np.array([beta @ target @ beta for target in target_matrices(template)])
    match_codes = np.unique(data["match"], return_inverse=True)[1]

    rng = np.random.default_rng(20260817)
    repetitions = 1_000
    plugin = np.empty((repetitions, 4))
    corrected = np.empty((repetitions, 4))
    for repetition in range(repetitions):
        block_shock = rng.normal(scale=0.6, size=match_codes.max() + 1)
        error = block_shock[match_codes] + rng.normal(scale=0.25, size=signal.size)
        result = exact_kss(
            signal + error,
            data["worker"],
            data["firm"],
            controls=data["controls"],
            deletion="match",
            deletion_id=data["match"],
        )
        plugin[repetition] = result.plugin
        corrected[repetition] = result.corrected

    plugin_bias = plugin.mean(axis=0) - truth
    corrected_bias = corrected.mean(axis=0) - truth
    assert np.all(np.abs(corrected_bias) < 0.35 * np.abs(plugin_bias))
    assert np.allclose(
        corrected.mean(axis=0),
        truth,
        atol=4 * corrected.std(axis=0, ddof=1) / np.sqrt(repetitions),
        rtol=0,
    )

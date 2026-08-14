from __future__ import annotations

import itertools

import numpy as np
import sympy as sp

from kss_bc.tests.python.oracle import jla_bias_terms


def test_delta_method_mixed_coefficient_is_one() -> None:
    p, m, p2, m2, pm, probes = sp.symbols("p m p2 m2 pm probes", positive=True)
    f = m / (p + m)
    hessian = sp.hessian(f, (p, m)).subs(m, 1 - p)
    covariance = sp.Matrix(
        [
            [p2 - p**2, pm - p * (1 - p)],
            [pm - p * (1 - p), m2 - (1 - p) ** 2],
        ]
    ) / probes
    bias = sp.factor(sp.trace(hessian * covariance) / 2)
    target = ((1 - p) * p2 - p * m2 + (1 - 2 * p) * pm) / probes
    assert sp.simplify(bias - target) == 0


def test_rank_one_projection_selects_coefficient_one() -> None:
    vector = np.sqrt(np.array([0.2, 0.3, 0.2, 0.2, 0.1]))
    projection = np.outer(vector, vector)
    residual_maker = np.eye(vector.size) - projection
    directions = np.array(list(itertools.product([-1.0, 1.0], repeat=vector.size)))
    projected = directions @ projection[0]
    residual = directions @ residual_maker[0]
    p = projection[0, 0]
    m = residual_maker[0, 0]
    p2 = np.mean(projected**4)
    m2 = np.mean(residual**4)
    mixed = np.mean(projected**2 * residual**2)

    _, coefficient_one, _ = jla_bias_terms(p, m, p2, m2, mixed, 1)
    _, coefficient_two, _ = jla_bias_terms(
        p, m, p2, m2, mixed, 1, mixed_coefficient=2
    )
    assert np.isclose(coefficient_one, -0.07872, atol=1e-12)
    assert np.isclose(coefficient_two, -0.02208, atol=1e-12)


def test_fixed_seed_simulation_agrees_with_coefficient_one() -> None:
    rng = np.random.default_rng(20260814)
    vector = np.sqrt(np.array([0.2, 0.3, 0.2, 0.2, 0.1]))
    projection = np.outer(vector, vector)
    residual_maker = np.eye(vector.size) - projection
    directions = np.array(list(itertools.product([-1.0, 1.0], repeat=vector.size)))
    projected = (directions @ projection[0]) ** 2
    residual = (directions @ residual_maker[0]) ** 2
    reps = 25
    experiments = 600_000
    counts = rng.multinomial(reps, np.full(directions.shape[0], 1 / directions.shape[0]), size=experiments)
    p_hat = counts @ projected / reps
    m_hat = counts @ residual / reps
    scaled_draw = reps * (m_hat / (p_hat + m_hat) - residual_maker[0, 0])
    scaled_bias = np.mean(scaled_draw)
    simulation_mcse = np.std(scaled_draw, ddof=1) / np.sqrt(experiments)
    assert np.isclose(scaled_bias, -0.07895562372081338, atol=1e-10)
    assert np.isclose(simulation_mcse, 0.001859863253553029, atol=1e-12)
    assert abs(scaled_bias - (-0.07872)) < 0.008
    assert abs(scaled_bias - (-0.07872)) < abs(scaled_bias - (-0.02208))


def test_finite_probe_gate_events_do_not_imply_conditional_unbiasedness() -> None:
    projection = np.full((2, 2), 0.5)
    residual_maker = np.eye(2) - projection
    directions = np.array(list(itertools.product([-1.0, 1.0], repeat=2)))
    projected = directions @ projection[0]
    residual = directions @ residual_maker[0]
    assert np.count_nonzero(residual == 0) == 2

    # At R=2, one quarter of all probe pairs have zero estimated residual
    # leverage despite exact leave-one-out estimability.
    pairs = np.array(list(itertools.product(range(4), repeat=2)))
    residual_mean = np.mean(residual[pairs] ** 2, axis=1)
    assert np.count_nonzero(residual_mean == 0) == 4
    design = np.ones((2, 1))
    assert np.linalg.matrix_rank(design[1:]) == design.shape[1]


def test_estimable_match_can_have_zero_probe_ratio_denominator() -> None:
    projection = np.full((4, 4), 0.25)
    residual_maker = np.eye(4) - projection
    block = np.array([0, 1])
    block_direction = np.ones(2) / np.sqrt(2)
    maker_block = residual_maker[np.ix_(block, block)]
    assert np.allclose(np.linalg.eigvalsh(maker_block), [0.5, 1.0])

    directions = np.array(list(itertools.product([-1.0, 1.0], repeat=4)))
    projected = (directions @ projection[block].T) @ block_direction
    residual = (directions @ residual_maker[block].T) @ block_direction
    denominator = projected**2 + residual**2
    assert np.count_nonzero(denominator == 0) == 4

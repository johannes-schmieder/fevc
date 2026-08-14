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
    experiments = 150_000
    counts = rng.multinomial(reps, np.full(directions.shape[0], 1 / directions.shape[0]), size=experiments)
    p_hat = counts @ projected / reps
    m_hat = counts @ residual / reps
    scaled_bias = reps * np.mean(m_hat / (p_hat + m_hat) - residual_maker[0, 0])
    assert abs(scaled_bias - (-0.07872)) < 0.008
    assert abs(scaled_bias - (-0.07872)) < abs(scaled_bias - (-0.02208))

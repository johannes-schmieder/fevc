from __future__ import annotations

import itertools

import numpy as np

from vckss.tests.python.oracle import build_expanded_design
from vckss.tests.python.test_dense_oracle import fixture, model_data


def within_cell_scatter(
    controls: np.ndarray,
    worker: np.ndarray,
    firm: np.ndarray,
    keep: np.ndarray,
) -> np.ndarray:
    scatter = np.zeros((controls.shape[1], controls.shape[1]))
    cells = np.column_stack((worker, firm))
    for cell in np.unique(cells, axis=0):
        use = keep & np.all(cells == cell, axis=1)
        if np.any(use):
            centered = controls[use] - controls[use].mean(axis=0)
            scatter += centered.T @ centered
    return scatter


def test_rank_one_block_inverse_derivatives_have_registered_signs() -> None:
    control_loading = np.array([[0.20, -0.05], [-0.10, 0.25], [0.15, 0.10]])
    control_projection = control_loading @ control_loading.T
    direction = np.sqrt(np.array([0.2, 0.3, 0.5]))
    residual = np.array([0.7, -0.25, 0.4])
    leverage = 0.27

    def inverse_residual(value: float) -> np.ndarray:
        maker = (
            np.eye(direction.size)
            - control_projection
            - value * np.outer(direction, direction)
        )
        return np.linalg.solve(maker, residual)

    maker = (
        np.eye(direction.size)
        - control_projection
        - leverage * np.outer(direction, direction)
    )
    inverse = np.linalg.inv(maker)
    inverse_direction = inverse @ direction
    first = direction @ inverse @ residual
    second = direction @ inverse_direction
    analytic_first = inverse_direction * first
    analytic_half_second = inverse_direction * second * first

    step = 1e-5
    center = inverse_residual(leverage)
    finite_first = (
        inverse_residual(leverage + step)
        - inverse_residual(leverage - step)
    ) / (2 * step)
    finite_half_second = (
        inverse_residual(leverage + step)
        - 2 * center
        + inverse_residual(leverage - step)
    ) / (2 * step**2)
    assert np.allclose(finite_first, analytic_first, atol=2e-9, rtol=2e-9)
    assert np.allclose(
        finite_half_second, analytic_half_second, atol=3e-6, rtol=3e-6
    )


def test_joint_projection_splits_into_fe_and_residualized_controls() -> None:
    data = fixture()
    design = build_expanded_design(
        **model_data(data), deletion="match", deletion_id=data["match"]
    )
    fe_columns = np.any(design.worker_rows != 0, axis=0) | np.any(
        design.firm_rows != 0, axis=0
    )
    x_fe = design.x[:, fe_columns]
    controls = design.x[:, ~fe_columns]
    p_fe = x_fe @ np.linalg.inv(x_fe.T @ x_fe) @ x_fe.T
    residualized_controls = controls - p_fe @ controls
    control_projection = residualized_controls @ np.linalg.inv(
        residualized_controls.T @ residualized_controls
    ) @ residualized_controls.T
    p_full = design.x @ np.linalg.inv(design.x.T @ design.x) @ design.x.T
    assert np.allclose(p_full, p_fe + control_projection, atol=2e-12, rtol=2e-12)
    assert np.allclose(p_fe @ control_projection, 0, atol=2e-12, rtol=2e-12)

    for block in np.unique(design.deletion_id):
        use = design.deletion_id == block
        frequency_direction = np.ones(use.sum()) / np.sqrt(use.sum())
        p_block = p_fe[np.ix_(use, use)]
        leverage = float(frequency_direction @ p_block @ frequency_direction)
        assert np.allclose(
            p_block,
            leverage * np.outer(frequency_direction, frequency_direction),
            atol=2e-12,
            rtol=2e-12,
        )


def test_general_block_delta_adjustment_reduces_inverse_bias() -> None:
    block = np.array([0, 1])
    direction = np.sqrt(np.array([0.4, 0.6]))
    leverage = 0.30
    base_vector = np.r_[np.sqrt(leverage) * direction, np.repeat(np.sqrt(0.7 / 4), 4)]
    base_vector /= np.linalg.norm(base_vector)

    candidate = np.array([0.3, -0.2, 0.5, -0.4, 0.1, 0.6])
    control_vector = candidate - base_vector * (candidate @ base_vector)
    control_vector /= np.linalg.norm(control_vector)
    base_projection = np.outer(base_vector, base_vector)
    control_projection = np.outer(control_vector, control_vector)
    block_control = control_projection[np.ix_(block, block)]
    outcome_residual = np.array([0.7, -0.25])
    rank_one = np.outer(direction, direction)
    assert np.linalg.norm(block_control @ rank_one-rank_one @ block_control) > 1e-3

    directions = np.array(list(itertools.product([-1.0, 1.0], repeat=6)))
    projected = (directions @ base_projection[block].T) @ direction
    residual = (directions @ (np.eye(6) - base_projection)[block].T) @ direction
    projected_sq = projected**2
    residual_sq = residual**2
    assert np.isclose(projected_sq.mean(), leverage, atol=2e-14)
    assert np.isclose(residual_sq.mean(), 1 - leverage, atol=2e-14)

    probes = 25
    experiments = 80_000
    rng = np.random.default_rng(20260815)
    counts = rng.multinomial(
        probes,
        np.full(directions.shape[0], 1 / directions.shape[0]),
        size=experiments,
    )
    p_mean = counts @ projected_sq / probes
    m_mean = counts @ residual_sq / probes
    p_fourth = counts @ (projected_sq**2) / probes
    m_fourth = counts @ (residual_sq**2) / probes
    mixed = counts @ (projected_sq * residual_sq) / probes
    total = p_mean + m_mean
    p_bar = p_mean / total
    m_bar = m_mean / total
    finite_variance = (
        m_bar**2 * p_fourth
        + p_bar**2 * m_fourth
        - 2 * p_bar * m_bar * mixed
    ) / probes
    finite_bias_m = (
        m_bar * p_fourth
        - p_bar * m_fourth
        + (m_bar - p_bar) * mixed
    ) / probes

    truth_maker = (
        np.eye(2)
        - block_control
        - leverage * np.outer(direction, direction)
    )
    truth = np.linalg.solve(truth_maker, outcome_residual)
    naive = np.empty((experiments, 2))
    adjusted = np.empty((experiments, 2))
    for experiment in range(experiments):
        maker = (
            np.eye(2)
            - block_control
            - p_bar[experiment] * np.outer(direction, direction)
        )
        inverse = np.linalg.inv(maker)
        base = inverse @ outcome_residual
        inverse_direction = inverse @ direction
        first = direction @ base
        second = direction @ inverse_direction
        naive[experiment] = base
        adjusted[experiment] = (
            base
            + finite_bias_m[experiment] * inverse_direction * first
            - finite_variance[experiment] * inverse_direction * second * first
        )

    naive_bias = np.linalg.norm(naive.mean(axis=0) - truth)
    adjusted_bias = np.linalg.norm(adjusted.mean(axis=0) - truth)
    assert adjusted_bias < 0.35 * naive_bias


def test_within_cell_trace_bound_certifies_every_control_deletion() -> None:
    data = fixture()
    keep = np.ones(data["y"].size, dtype=bool)
    scatter = within_cell_scatter(
        data["controls"], data["worker"], data["firm"], keep
    )
    inverse = np.linalg.inv(scatter)
    trace_losses = []
    minimum_deleted_eigenvalue = np.inf
    for match in np.unique(data["match"]):
        retained = data["match"] != match
        deleted_scatter = within_cell_scatter(
            data["controls"], data["worker"], data["firm"], retained
        )
        loss = scatter - deleted_scatter
        assert np.linalg.eigvalsh(loss).min() > -2e-15
        trace_losses.append(np.trace(inverse @ loss))
        minimum_deleted_eigenvalue = min(
            minimum_deleted_eigenvalue,
            np.linalg.eigvalsh(deleted_scatter).min(),
        )
    gap = 1 - max(trace_losses)
    assert np.isclose(gap, 0.5882352941176471, atol=2e-14)
    assert minimum_deleted_eigenvalue > 0

    # The generalized trace loss, hence the certificate, is invariant to an
    # invertible reparameterization of the control columns.
    transform = np.array([[1.3, -0.4], [0.2, 0.8]])
    transformed = data["controls"] @ transform
    transformed_scatter = within_cell_scatter(
        transformed, data["worker"], data["firm"], keep
    )
    transformed_inverse = np.linalg.inv(transformed_scatter)
    transformed_losses = []
    for match in np.unique(data["match"]):
        retained = data["match"] != match
        deleted_scatter = within_cell_scatter(
            transformed, data["worker"], data["firm"], retained
        )
        transformed_losses.append(
            np.trace(transformed_inverse @ (transformed_scatter-deleted_scatter))
        )
    assert np.allclose(trace_losses, transformed_losses, atol=3e-15, rtol=0)


def test_block_only_control_has_no_deterministic_rank_certificate() -> None:
    data = fixture()
    block_only = (data["match"] == 10).astype(float)[:, None]
    keep = np.ones(data["y"].size, dtype=bool)
    scatter = within_cell_scatter(
        block_only, data["worker"], data["firm"], keep
    )
    assert np.array_equal(scatter, np.zeros((1, 1)))

    design = build_expanded_design(
        data["y"], data["worker"], data["firm"], controls=block_only,
        deletion="match", deletion_id=data["match"],
    )
    assert np.linalg.matrix_rank(design.x) == design.x.shape[1]
    retained = design.deletion_id != design.deletion_id[0]
    assert np.linalg.matrix_rank(design.x[retained]) < design.x.shape[1]


def test_rank_gap_must_subtract_measured_whitening_error() -> None:
    # This scalar construction lies inside the old whitening-residual gate.
    # Its unbuffered trace gap is positive and exceeded the old rank margin,
    # even though the deleted scatter is negative.  Subtracting the measured
    # whitening residual restores a valid lower bound.
    information = np.array([[1.0]])
    whitener = np.array([[np.sqrt(1 - 5e-8)]])
    loss = np.array([[1 + 2e-8]])
    whitened_information = whitener.T @ information @ whitener
    whitened_loss = whitener.T @ loss @ whitener
    whitening_error = np.linalg.norm(
        whitened_information-np.eye(1), ord="fro"
    )
    raw_gap = 1-np.trace(whitened_loss)
    buffered_gap = raw_gap-whitening_error
    actual = np.linalg.eigvalsh(
        whitener.T @ (information-loss) @ whitener
    ).min()

    assert raw_gap > 1e-8
    assert actual < 0
    assert buffered_gap < 0
    assert actual >= buffered_gap-2e-16


def test_aggregate_batch_residual_can_mask_one_failed_column() -> None:
    residual = np.array([[1.0, 0.0]])
    right_hand_side = np.array([[0.0, 1e12]])
    aggregate = np.linalg.norm(residual) / (1+np.linalg.norm(right_hand_side))
    per_column = np.array(
        [
            np.linalg.norm(residual[:, column])
            / (1+np.linalg.norm(right_hand_side[:, column]))
            for column in range(residual.shape[1])
        ]
    )
    assert aggregate < 1e-10
    assert per_column.max() == 1


def test_rank_scatter_requires_explicit_cell_centering() -> None:
    controls = np.array([1e9+1, 1e9-1, 1e9+2, 1e9-2])[:, None]
    frequency = np.ones(4)
    unstable = (
        controls.T @ (frequency[:, None]*controls)
        - (frequency @ controls)[:, None]
        @ (frequency @ controls)[None, :]
        / frequency.sum()
    )
    centered = controls-controls.mean(axis=0)
    stable = centered.T @ (frequency[:, None]*centered)
    assert unstable[0, 0] == 0
    assert stable[0, 0] == 10

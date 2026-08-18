"""Independent dense KSS point-estimation oracle.

This module deliberately shares no implementation code with the Stata/Mata
package.  It expands integer frequency weights literally and evaluates the
general block leave-out identity with dense linear algebra.  It is a
development and falsification oracle, not a production dependency.
"""

from __future__ import annotations

from collections.abc import Iterable
from dataclasses import dataclass
from typing import Literal

import numpy as np
from numpy.typing import ArrayLike, NDArray

FloatArray = NDArray[np.float64]
IntArray = NDArray[np.int64]
Deletion = Literal["observation", "match"]
Nuisance = Literal["joint", "fixedoffset"]


class OracleError(ValueError):
    """Raised when the requested dense oracle calculation is undefined."""


@dataclass(frozen=True)
class ExpandedDesign:
    y: FloatArray
    x: FloatArray
    worker_rows: FloatArray
    firm_rows: FloatArray
    stored_index: IntArray
    deletion_id: IntArray
    target_weight_stored: FloatArray
    worker_codes_stored: IntArray
    firm_codes_stored: IntArray


@dataclass(frozen=True)
class OracleResult:
    plugin: FloatArray
    correction: FloatArray
    corrected: FloatArray
    beta: FloatArray
    residual: FloatArray
    leverage_max: float
    deletion_units: int
    target_weight_sum: float


def _as_1d(values: ArrayLike, name: str, dtype: np.dtype) -> NDArray:
    out = np.asarray(values, dtype=dtype)
    if out.ndim != 1:
        raise OracleError(f"{name} must be one-dimensional")
    return out


def _encode(values: ArrayLike, name: str) -> IntArray:
    raw = np.asarray(values)
    if raw.ndim != 1:
        raise OracleError(f"{name} must be one-dimensional")
    if raw.dtype.kind in "fc" and not np.isfinite(raw.astype(float)).all():
        raise OracleError(f"{name} contains a missing or nonfinite value")
    _, encoded = np.unique(raw, return_inverse=True)
    return encoded.astype(np.int64)


def _controls_matrix(controls: ArrayLike | None, n: int) -> FloatArray:
    if controls is None:
        return np.empty((n, 0), dtype=float)
    out = np.asarray(controls, dtype=float)
    if out.ndim == 1:
        out = out[:, None]
    if out.ndim != 2 or out.shape[0] != n:
        raise OracleError("controls must have one row per stored observation")
    if not np.isfinite(out).all():
        raise OracleError("controls contain a missing or nonfinite value")
    return out


def build_expanded_design(
    y: ArrayLike,
    worker: ArrayLike,
    firm: ArrayLike,
    *,
    controls: ArrayLike | None = None,
    frequency: ArrayLike | None = None,
    target_weight: ArrayLike | None = None,
    deletion: Deletion = "match",
    deletion_id: ArrayLike | None = None,
) -> ExpandedDesign:
    """Build a full-rank worker/all-but-one-firm/control quotient.

    Frequency weights are expanded into literal copies.  Explicit target
    weights are total stored-row mass, so they remain stored-row objects.
    """

    y_stored = _as_1d(y, "y", np.dtype(float))
    if not np.isfinite(y_stored).all():
        raise OracleError("y contains a missing or nonfinite value")
    n = y_stored.size
    worker_code = _encode(worker, "worker")
    firm_code = _encode(firm, "firm")
    if worker_code.size != n or firm_code.size != n:
        raise OracleError("worker and firm must have one value per observation")
    z = _controls_matrix(controls, n)

    if frequency is None:
        freq = np.ones(n, dtype=np.int64)
    else:
        freq_raw = _as_1d(frequency, "frequency", np.dtype(float))
        if freq_raw.size != n or not np.isfinite(freq_raw).all():
            raise OracleError("frequency must be finite and match y")
        if np.any(freq_raw <= 0) or np.any(freq_raw != np.floor(freq_raw)):
            raise OracleError("frequency must contain positive integers")
        freq = freq_raw.astype(np.int64)

    if target_weight is None:
        target = freq.astype(float)
    else:
        target = _as_1d(target_weight, "target_weight", np.dtype(float))
        if target.size != n or not np.isfinite(target).all() or np.any(target < 0):
            raise OracleError("target weights must be finite and nonnegative")
    if not float(target.sum()) > 0:
        raise OracleError("target weights must have positive total mass")

    if deletion == "observation":
        deletion_stored = np.arange(n, dtype=np.int64)
    elif deletion == "match":
        if deletion_id is None:
            pair = np.column_stack((worker_code, firm_code))
            _, deletion_stored = np.unique(pair, axis=0, return_inverse=True)
            deletion_stored = deletion_stored.astype(np.int64)
        else:
            deletion_stored = _encode(deletion_id, "deletion_id")
            if deletion_stored.size != n:
                raise OracleError("deletion_id must match y")
            for code in np.unique(deletion_stored):
                use = deletion_stored == code
                if np.unique(worker_code[use]).size != 1 or np.unique(firm_code[use]).size != 1:
                    raise OracleError(
                        "each match deletion ID must remain within one worker-firm coordinate"
                    )
    else:
        raise OracleError("deletion must be observation or match")

    stored_index = np.repeat(np.arange(n, dtype=np.int64), freq)
    y_expanded = y_stored[stored_index]
    w_expanded = worker_code[stored_index]
    f_expanded = firm_code[stored_index]
    z_expanded = z[stored_index]

    workers = int(worker_code.max()) + 1
    firms = int(firm_code.max()) + 1
    d = np.eye(workers, dtype=float)[w_expanded]
    f_all = np.eye(firms, dtype=float)[f_expanded]
    f_quotient = f_all[:, : max(firms - 1, 0)]
    x = np.column_stack((d, f_quotient, z_expanded))

    worker_rows = np.column_stack(
        (d, np.zeros((d.shape[0], f_quotient.shape[1] + z.shape[1])))
    )
    firm_rows = np.column_stack(
        (np.zeros((d.shape[0], d.shape[1])), f_quotient, np.zeros((d.shape[0], z.shape[1])))
    )

    if deletion == "observation":
        deletion_expanded = np.arange(stored_index.size, dtype=np.int64)
    else:
        deletion_expanded = deletion_stored[stored_index]

    return ExpandedDesign(
        y=y_expanded,
        x=x,
        worker_rows=worker_rows,
        firm_rows=firm_rows,
        stored_index=stored_index,
        deletion_id=deletion_expanded,
        target_weight_stored=target,
        worker_codes_stored=worker_code,
        firm_codes_stored=firm_code,
    )


def target_matrices(design: ExpandedDesign) -> tuple[FloatArray, ...]:
    """Return worker, firm, covariance, and total target matrices."""

    n_stored = design.target_weight_stored.size
    representative = np.empty(n_stored, dtype=np.int64)
    for row in range(n_stored):
        representative[row] = int(np.flatnonzero(design.stored_index == row)[0])
    aw = design.worker_rows[representative]
    af = design.firm_rows[representative]
    mass = design.target_weight_stored.astype(float)
    share = mass / mass.sum()
    center = np.diag(share) - np.outer(share, share)
    qw = aw.T @ center @ aw
    qf = af.T @ center @ af
    qc = 0.5 * (aw.T @ center @ af + af.T @ center @ aw)
    qt = qw + qf + 2.0 * qc
    return qw, qf, qc, qt


def exact_kss(
    y: ArrayLike,
    worker: ArrayLike,
    firm: ArrayLike,
    *,
    controls: ArrayLike | None = None,
    frequency: ArrayLike | None = None,
    target_weight: ArrayLike | None = None,
    deletion: Deletion = "match",
    deletion_id: ArrayLike | None = None,
    nuisance: Nuisance = "joint",
    rank_tolerance: float = 1e-11,
    block_tolerance: float = 1e-10,
) -> OracleResult:
    """Evaluate exact dense KSS correction for arbitrary deletion blocks."""

    design = build_expanded_design(
        y,
        worker,
        firm,
        controls=controls,
        frequency=frequency,
        target_weight=target_weight,
        deletion=deletion,
        deletion_id=deletion_id,
    )
    if nuisance not in ("joint", "fixedoffset"):
        raise OracleError("nuisance must be joint or fixedoffset")

    full_x = design.x
    full_info = full_x.T @ full_x
    full_rank = np.linalg.matrix_rank(full_info, tol=rank_tolerance)
    if full_rank != full_info.shape[0]:
        raise OracleError(
            f"identified information is singular: rank {full_rank} of {full_info.shape[0]}"
        )
    full_inverse = np.linalg.inv(full_info)
    full_beta = full_inverse @ (full_x.T @ design.y)
    qs_full = target_matrices(design)

    if nuisance == "fixedoffset":
        fe_columns = np.any(design.worker_rows != 0, axis=0) | np.any(
            design.firm_rows != 0, axis=0
        )
        nuisance_columns = ~fe_columns
        working_y = design.y - full_x[:, nuisance_columns] @ full_beta[nuisance_columns]
        x = full_x[:, fe_columns]
        qs = tuple(q[np.ix_(fe_columns, fe_columns)] for q in qs_full)
    else:
        working_y = design.y
        x = full_x
        qs = qs_full

    info = x.T @ x
    rank = np.linalg.matrix_rank(info, tol=rank_tolerance)
    if rank != info.shape[0]:
        raise OracleError(f"identified information is singular: rank {rank} of {info.shape[0]}")
    inverse = np.linalg.inv(info)
    inverse_residual = np.linalg.norm(info @ inverse - np.eye(info.shape[0]), ord=np.inf)
    if inverse_residual > 1e-8:
        raise OracleError(f"information inverse residual is too large: {inverse_residual}")

    beta = inverse @ (x.T @ working_y)
    residual = working_y - x @ beta
    plugin = np.array([float(beta @ q @ beta) for q in qs])
    correction = np.zeros(4, dtype=float)
    leverage_max = 0.0

    for block in np.unique(design.deletion_id):
        use = design.deletion_id == block
        xg = x[use]
        yg = working_y[use]
        eg = residual[use]
        pg = xg @ inverse @ xg.T
        leverage_max = max(leverage_max, float(np.linalg.eigvalsh(pg).max()))
        maker = np.eye(pg.shape[0]) - pg
        smallest = float(np.linalg.eigvalsh(maker).min())
        if smallest <= block_tolerance:
            raise OracleError(
                f"deletion block is not leave-out estimable: min residual eigenvalue {smallest}"
            )
        deleted_residual = np.linalg.solve(maker, eg)
        for target_index, q in enumerate(qs):
            bg = xg @ inverse @ q @ inverse @ xg.T
            correction[target_index] += float(yg @ bg @ deleted_residual)

    corrected = plugin - correction
    if not np.allclose(plugin[3], plugin[0] + plugin[1] + 2 * plugin[2], atol=1e-11):
        raise OracleError("plug-in accounting identity failed")
    if not np.allclose(
        correction[3], correction[0] + correction[1] + 2 * correction[2], atol=1e-10
    ):
        raise OracleError("correction accounting identity failed")

    return OracleResult(
        plugin=plugin,
        correction=correction,
        corrected=corrected,
        beta=beta,
        residual=residual,
        leverage_max=leverage_max,
        deletion_units=int(np.unique(design.deletion_id).size),
        target_weight_sum=float(design.target_weight_stored.sum()),
    )


def deleted_residuals_by_refit(design: ExpandedDesign) -> Iterable[tuple[IntArray, FloatArray]]:
    """Yield block indices and direct deleted-fit residuals for falsification."""

    for block in np.unique(design.deletion_id):
        use = design.deletion_id == block
        keep = ~use
        x_keep = design.x[keep]
        beta_deleted, residuals, rank, _ = np.linalg.lstsq(x_keep, design.y[keep], rcond=None)
        if rank != design.x.shape[1]:
            raise OracleError("direct deleted fit lost rank")
        del residuals
        yield np.flatnonzero(use), design.y[use] - design.x[use] @ beta_deleted


def jla_bias_terms(
    p: ArrayLike,
    m: ArrayLike,
    p_second: ArrayLike,
    m_second: ArrayLike,
    mixed_second: ArrayLike,
    probes: int,
    *,
    mixed_coefficient: float = 1.0,
) -> tuple[FloatArray, FloatArray, FloatArray]:
    """Return finite-projection variance, bias, and multiplier.

    `mixed_coefficient=2` exists only so tests can pin the legacy MATLAB
    discrepancy.  Production code must always use the default value one.
    """

    if probes <= 0:
        raise OracleError("probes must be positive")
    p_arr = np.asarray(p, dtype=float)
    m_arr = np.asarray(m, dtype=float)
    p2 = np.asarray(p_second, dtype=float)
    m2 = np.asarray(m_second, dtype=float)
    pm = np.asarray(mixed_second, dtype=float)
    variance = (m_arr**2 * p2 + p_arr**2 * m2 - 2 * p_arr * m_arr * pm) / probes
    bias = (
        m_arr * p2
        - p_arr * m2
        + mixed_coefficient * (m_arr - p_arr) * pm
    ) / probes
    multiplier = 1 - variance / m_arr**2 + bias / m_arr
    return variance, bias, multiplier

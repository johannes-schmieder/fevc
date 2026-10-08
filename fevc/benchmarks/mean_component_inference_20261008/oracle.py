"""Independent dense Gaussian audit; observation-square matrices are test-only.

The production estimator is never imported. Independent units are physical
observations or scalar whole-match sufficient rows. ``direction`` maps a
constant outcome shift to this (possibly sqrt-mass-scaled) geometry.
"""
from __future__ import annotations

from dataclasses import dataclass
import hashlib
import json

import numpy as np
from scipy import linalg
from scipy.stats import rankdata

TARGETS = ("worker", "firm", "covariance", "total")


def seed(*key: object) -> int:
    return 1 + int.from_bytes(hashlib.sha256(json.dumps(key, separators=(",", ":")).encode()).digest()[:8], "big") % 2147483646


@dataclass
class Design:
    rows: dict[str, np.ndarray]
    x: np.ndarray
    direction: np.ndarray
    mean: np.ndarray
    variance: np.ndarray
    kernels: np.ndarray
    plugin: np.ndarray
    modes: np.ndarray
    eigenvalues: np.ndarray
    remainders: np.ndarray
    truth: np.ndarray
    diagnostics: dict

    @property
    def c0(self):
        return float(self.direction @ self.mean / (self.direction @ self.direction))

    def draw(self, domain: str, cell: str, replication: int, error="gaussian"):
        rng = np.random.default_rng(seed("mean-component-v1", domain, cell, replication))
        noise = rng.standard_normal(len(self.mean)) if error == "gaussian" else rng.standard_t(8, len(self.mean)) * np.sqrt(6 / 8)
        return self.mean + np.sqrt(self.variance) * noise

    def physical_outcome(self, scaled):
        return np.asarray(scaled)[self.rows["unit"].astype(int)] / self.direction[self.rows["unit"].astype(int)]


def symmetric(matrix):
    return (matrix + matrix.T) / 2


def gaussian_covariance(kernels, mean, variance):
    """All cross-covariances of Gaussian quadratic forms, independently."""
    kernels = np.asarray(kernels)
    means = np.einsum("i,tij,j->t", mean, kernels, mean) + np.einsum("tii,i->t", kernels, variance)
    whitened = kernels * np.sqrt(variance)[None, :, None] * np.sqrt(variance)[None, None, :]
    influences = (kernels @ mean) * np.sqrt(variance)
    covariance = 2 * np.einsum("tij,sij->ts", whitened, whitened) + 4 * influences @ influences.T
    return means, covariance


def mean_kernel(kernels, direction):
    """A' C A via low-rank arithmetic, independently of production code."""
    a = direction / (direction @ direction)
    cd = kernels @ direction
    return (kernels - cd[:, :, None] * a[None, None, :]
            - a[None, :, None] * cd[:, None, :]
            + np.einsum("i,ti->t", direction, cd)[:, None, None] * np.outer(a, a))


def build_design(k: int, route: str, reference: str, diagnostic="primary") -> Design:
    if k < 6 or route not in ("mata", "observation", "match") or reference not in ("highrank", "q1"):
        raise ValueError("invalid design specification")
    workers, firms, frequencies, groups, targets = [], [], [], [], []
    for w in range(k):
        for f in range(k):
            multiplicity = 2 + ((w * 13 + f * 7 + 1) % 3)
            if route == "match":
                copies = [1 + (w + 2 * f) % 3, 1 + (2 * w + f) % 2]
                if diagnostic == "concentrated_mass" and w == 0 and f == 0:
                    copies = [20 * k, 10 * k]
            else:
                copies = [1] * multiplicity
            base = .85 + ((w * 19 + f * 11 + 3) % 31) / 100
            multiplier = 1.
            if reference == "q1" and w == f == 0:
                multiplier = 1000.
            if diagnostic == "multimode" and w == f and w < 3:
                multiplier = (900., 600., 300.)[w]
            for frequency in copies:
                workers.append(w); firms.append(f); frequencies.append(frequency)
                groups.append(w * k + f if route == "match" else len(groups))
                targets.append(base * multiplier / len(copies))
    worker, firm, freq, unit, tw = map(np.asarray, (workers, firms, frequencies, groups, targets))
    nu = int(unit.max()) + 1
    mass = np.bincount(unit, weights=freq, minlength=nu)
    # Within a whole match all rows have the same FE design. Physical copies
    # contribute regression mass, never extra independent Gaussian errors.
    first = np.unique(unit, return_index=True)[1]
    wu, fu = worker[first], firm[first]
    xu = np.column_stack((np.eye(k)[wu], np.eye(k)[fu, :-1]))
    direction = np.sqrt(mass)
    x = direction[:, None] * xu
    h_inv = linalg.inv(x.T @ x)
    projection = x @ h_inv @ x.T
    residual = np.eye(nu) - projection
    leverage = np.diag(projection)
    if np.min(1 - leverage) <= 1e-10:
        raise ValueError("unidentified deletion")
    xd = np.column_stack((np.eye(k)[worker], np.zeros((len(worker), k - 1))))
    xf = np.column_stack((np.zeros((len(worker), k)), np.eye(k)[firm, :-1]))
    normalized = tw / tw.sum()
    xd -= normalized @ xd
    xf -= normalized @ xf
    q = [xd.T @ (normalized[:, None] * xd), xf.T @ (normalized[:, None] * xf)]
    q.append(symmetric(xd.T @ (normalized[:, None] * xf)))
    q.append(q[0] + q[1] + 2 * q[2])
    ah = x @ h_inv
    plugin = np.asarray([ah @ target @ ah.T for target in q])
    ratio = np.diagonal(plugin, axis1=1, axis2=2) / (1 - leverage)
    kernels = np.asarray([b - symmetric(r[:, None] * residual) for b, r in zip(plugin, ratio)])
    chol = linalg.cholesky(x.T @ x, lower=True)
    ichol = linalg.solve_triangular(chol, np.eye(len(chol)), lower=True)
    modes, eigenvalues, remainders, geometry = [], [], [], []
    for b, target, kernel in zip(plugin, q, kernels):
        ev, vec = linalg.eigh(ichol @ target @ ichol.T)
        order = np.argsort(-np.abs(ev))
        ev = ev[order]
        v = x @ linalg.solve_triangular(chol.T, vec[:, order[0]], lower=False)
        v /= np.linalg.norm(v)
        # Fix sign only for deterministic receipts; all scientific quantities
        # are invariant to jointly reversing the leading score and covariance.
        if v[np.argmax(np.abs(v))] < 0: v = -v
        leading_kernel = np.outer(v, v) - symmetric((v * v / (1 - leverage))[:, None] * residual)
        modes.append(v); eigenvalues.append(ev[0]); remainders.append(kernel - ev[0] * leading_kernel)
        trace = float(ev @ ev)
        remainder_trace = max(0., trace - ev[0] ** 2)
        geometry.append(dict(leading_share=float(ev[0] ** 2 / trace),
                             remainder_share=float(ev[1] ** 2 / remainder_trace) if remainder_trace else 0.,
                             maximum_mode_weight=float(np.max(v * v))))
    # This variance function belongs to both currently offered native bases.
    ranks = (rankdata(leverage, method="average") - .5) / nu
    variance = .7 + .3 * ranks + .15 * ranks ** 2
    if diagnostic == "homoskedastic": variance = np.ones(nu)
    hidden = np.sin((wu + 1) * (fu + 2) * .71) ** 2
    if diagnostic == "mild": variance *= .85 + .3 * hidden
    if diagnostic == "severe": variance *= .1 + 4.9 * hidden ** 3
    scale = {"weak": .04, "null": 0.}.get(diagnostic, 1.)
    alpha = scale * (2 * np.sin((np.arange(k) + 1) * .73) + np.arange(k) / k)
    psi = scale * (-1.4 * np.cos((np.arange(k) + 1) * 1.07))
    mean = direction * (3. + alpha[wu] + psi[fu])
    truth = np.asarray([np.sum(normalized * (alpha[worker] - normalized @ alpha[worker]) ** 2),
                        np.sum(normalized * (psi[firm] - normalized @ psi[firm]) ** 2),
                        np.sum(normalized * (alpha[worker] - normalized @ alpha[worker]) * (psi[firm] - normalized @ psi[firm]))])
    truth = np.append(truth, truth[0] + truth[1] + 2 * truth[2])
    diagnostics = dict(k=k, route=route, reference=reference, diagnostic=diagnostic,
        stored_rows=len(worker), independent_units=nu, physical_rows=int(freq.sum()),
        rank=len(chol), smallest_maker=float(np.min(1 - leverage)),
        mean_effective_units=float(mass.sum() ** 2 / (mass @ mass)),
        largest_mean_weight=float(mass.max() / mass.sum()),
        variance_min=float(variance.min()), variance_max=float(variance.max()),
        maximum_kernel_diagonal=float(np.max(np.abs(np.diagonal(kernels, axis1=1, axis2=2)))),
        geometry=dict(zip(TARGETS, geometry)))
    return Design(dict(worker=worker, firm=firm, frequency=freq, unit=unit, target=tw),
                  x, direction, mean, variance, kernels, plugin, np.asarray(modes),
                  np.asarray(eigenvalues), np.asarray(remainders), truth, diagnostics)


def moments(design: Design):
    d = design
    z0 = d.mean - d.c0 * d.direction
    actual = mean_kernel(d.kernels, d.direction)
    fixed_mean, fixed_cov = gaussian_covariance(d.kernels, z0, d.variance)
    actual_mean, actual_cov = gaussian_covariance(actual, d.mean, d.variance)
    delta_mean, delta_cov = gaussian_covariance(actual - d.kernels, z0, d.variance)
    rem_actual = mean_kernel(d.remainders, d.direction)
    rem_fixed_mean, rem_fixed_cov = gaussian_covariance(d.remainders, z0, d.variance)
    rem_mean, rem_cov = gaussian_covariance(rem_actual, d.mean, d.variance)
    score_cov_fixed = 2 * np.einsum("ti,i,ti->t", d.modes, d.variance, d.remainders @ z0)
    score_cov_actual = 2 * np.einsum("ti,i,ti->t", d.modes, d.variance, rem_actual @ d.mean)
    score_variance = np.sum(d.modes * d.modes * d.variance, axis=1)
    conditional_remainder = np.diag(rem_fixed_cov) - score_cov_fixed ** 2 / score_variance
    rem_delta_mean, rem_delta_cov = gaussian_covariance(rem_actual - d.remainders, z0, d.variance)
    # Expected feasible covariance when the true variances are supplied but
    # the observed sample mean is frozen inside the influence calculation.
    a = d.direction / (d.direction @ d.direction)
    ca = d.kernels - (d.kernels @ d.direction)[:, :, None] * a[None, None, :]
    whitened_ca = ca * np.sqrt(d.variance)[None, :, None] * np.sqrt(d.variance)[None, None, :]
    whitened_c = d.kernels * np.sqrt(d.variance)[None, :, None] * np.sqrt(d.variance)[None, None, :]
    influence = (d.kernels @ z0) * np.sqrt(d.variance)
    expected_feasible = (4 * influence @ influence.T
        + 4 * np.einsum("tij,sij->ts", whitened_ca, whitened_ca)
        - 2 * np.einsum("tij,sij->ts", whitened_c, whitened_c))
    return dict(diagnostics=d.diagnostics, c0=d.c0, truth=d.truth.tolist(),
        fixed_c0=dict(expectation=fixed_mean.tolist(), covariance=fixed_cov.tolist()),
        actual_mean=dict(expectation=actual_mean.tolist(), bias=(actual_mean - d.truth).tolist(), covariance=actual_cov.tolist(),
                         expected_true_variance_feasible_covariance=expected_feasible.tolist()),
        difference=dict(expectation=delta_mean.tolist(), covariance=delta_cov.tolist(),
                        rms_over_actual_sd=np.sqrt((np.diag(delta_cov) + delta_mean ** 2) / np.diag(actual_cov)).tolist()),
        q1=dict(fixed_remainder_expectation=rem_fixed_mean.tolist(), actual_remainder_expectation=rem_mean.tolist(),
                fixed_remainder_covariance=rem_fixed_cov.tolist(), actual_remainder_covariance=rem_cov.tolist(),
                fixed_score_remainder_covariance=score_cov_fixed.tolist(), actual_score_remainder_covariance=score_cov_actual.tolist(),
                leading_score_variance=score_variance.tolist(),fixed_conditional_remainder_variance=conditional_remainder.tolist(),
                mean_discrepancy_rms_over_conditional_remainder_sd=[float(np.sqrt((v+b*b)/s)) if s>0 else None
                    for v,b,s in zip(np.diag(rem_delta_cov),rem_delta_mean,conditional_remainder)]))


def evaluate(design: Design, outcome):
    d = design
    c = float(d.direction @ outcome / (d.direction @ d.direction))
    z = outcome - c * d.direction
    z0 = outcome - d.c0 * d.direction
    point = np.einsum("i,tij,j->t", z, d.kernels, z)
    fixed = np.einsum("i,tij,j->t", z0, d.kernels, z0)
    trace = 2 * np.einsum("tij,sij,i,j->ts", d.kernels, d.kernels, d.variance, d.variance)
    influence = (d.kernels @ z) * np.sqrt(d.variance)
    fixed_influence = (d.kernels @ z0) * np.sqrt(d.variance)
    return dict(mean=c, point=point.tolist(), fixed_point=fixed.tolist(),
                oracle_feasible_covariance=(4 * influence @ influence.T - trace).tolist(),
                fixed_oracle_feasible_covariance=(4 * fixed_influence @ fixed_influence.T - trace).tolist(),
                q1_score=(d.modes @ z).tolist(),
                q1_remainder=np.einsum("i,tij,j->t", z, d.remainders, z).tolist())

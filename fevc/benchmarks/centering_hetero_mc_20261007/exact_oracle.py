"""Independent dense Gaussian audit for small, unweighted AKM fixtures.

Observation-square matrices are deliberately confined to this audit oracle.
The construction does not invoke fevc or its production implementation.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np
import pandas as pd

TARGETS = ["Worker variance", "Firm variance", "Worker–firm covariance", "Variance of sum"]


def gaussian_moments(matrix, signal, variance):
    """Moments of y'Fy for independent y_i ~ N(signal_i, variance_i)."""
    w = (matrix + matrix.T) / 2
    expectation = signal @ w @ signal + np.diag(w) @ variance
    sampling_variance = (2 * np.sum(w * w * variance[:, None] * variance[None, :])
                         + 4 * np.sum(variance * (w @ signal) ** 2))
    return float(expectation), float(np.sqrt(max(0., sampling_variance)))


def build_matrices(data):
    n = len(data)
    D = pd.get_dummies(data.workerid, dtype=float).to_numpy()
    F = pd.get_dummies(data.firmid, dtype=float).to_numpy()[:, 1:]
    X = np.column_stack([D, F])
    if np.linalg.matrix_rank(X) != X.shape[1]:
        raise ValueError("fixture design is not connected/identified")
    A = np.linalg.inv(X.T @ X)
    M = np.eye(n) - X @ A @ X.T
    C = np.eye(n) - np.ones((n, n)) / n
    U = C @ np.column_stack([D, np.zeros_like(F)])
    V = C @ np.column_stack([np.zeros_like(D), F])
    Q = [U.T @ U / n, V.T @ V / n, (U.T @ V + V.T @ U) / (2 * n)]
    Q.append(Q[0] + Q[1] + 2 * Q[2])
    movers = data.groupby("workerid").firmid.transform("nunique").to_numpy() > 1
    worker, firm = data.workerid.to_numpy(), data.firmid.to_numpy()
    groups = [np.flatnonzero((worker == w) & (firm == f))
              for w, f in data[movers].groupby(["workerid", "firmid"]).groups]
    groups += [np.array([i]) for i in np.flatnonzero(~movers)]
    Z = np.zeros((n, len(groups)))
    R = np.zeros((n, n))
    masses = np.array([len(g) for g in groups])
    a = np.empty(len(groups))
    minimum_eigenvalue = np.inf
    for j, g in enumerate(groups):
        Z[g, j] = 1
        Mgg = M[np.ix_(g, g)]
        minimum_eigenvalue = min(minimum_eigenvalue, np.linalg.eigvalsh(Mgg).min())
        if minimum_eigenvalue <= 1e-12:
            raise ValueError("fixture fails leave-out identification")
        R[g, :] = np.linalg.solve(Mgg, M[g, :])
        a[j] = Mgg.sum() / len(g)
    K = n * np.diag(a) - Z.T @ M @ Z
    sizes, group_sums = Z @ masses, Z @ Z.T
    if np.max(np.abs(M @ data.signal.to_numpy())) > 1e-9:
        raise ValueError("signal is not in the fitted AKM span")
    matrices = {}
    for target, q in enumerate(Q, 1):
        B = X @ A @ q @ A @ X.T
        block = np.zeros_like(B)
        for g in groups:
            block[np.ix_(g, g)] = B[np.ix_(g, g)]
        correction = block @ R
        ell = block @ np.ones(n)
        k = np.linalg.solve(K.T, (Z.T @ ell) / masses)
        x = -ell.copy()
        for j, g in enumerate(groups):
            x[g] += n * k[j] * M[np.ix_(g, g)].sum(axis=1)
        v = -M @ Z @ k
        delta = C @ (((x / sizes + v)[:, None] * group_sums
                      - group_sums * (x / sizes)[None, :]) @ R)
        matrices[target] = {"plugin": B, "none": B - correction,
                            "mean": B - C @ correction,
                            "corrected": B - C @ correction + delta}
    diagnostics = dict(n=n, workers=D.shape[1], firms=F.shape[1] + 1,
        rank=X.shape[1], deletion_units=len(groups), stayer_rows=int(sum(~movers)),
        minimum_deletion_maker_eigenvalue=float(minimum_eigenvalue))
    return matrices, diagnostics, M


def oracle(fixture, draws=None, *, variance=None, sampling_reps=2000):
    data = fixture.copy() if isinstance(fixture, pd.DataFrame) else pd.read_csv(fixture)
    if variance is None:
        if "sigma2_true" not in data:
            raise ValueError("fixture must expose conditional sigma2_true")
        variance = data.sigma2_true.to_numpy()
    variance = np.asarray(variance, dtype=float)
    if variance.shape != (len(data),) or not np.isfinite(variance).all() or np.any(variance < 0):
        raise ValueError("conditional variances must be finite, nonnegative and row aligned")
    if sampling_reps < 1:
        raise ValueError("sampling_reps must be positive")
    matrices, diagnostics, M = build_matrices(data)
    signal = data.signal.to_numpy()
    alpha, psi = data.alpha_true.to_numpy(), data.psi_true.to_numpy()
    truth = np.array([np.var(alpha), np.var(psi),
        np.mean((alpha-alpha.mean())*(psi-psi.mean())), np.var(alpha+psi)])
    yy = pd.read_csv(draws).to_numpy() if isinstance(draws, (str, Path)) else draws
    if yy is not None and (yy.ndim != 2 or yy.shape[0] != len(data)):
        raise ValueError("draws must have one fixture-aligned row per observation")
    moments, paired = [], []
    for target, modes in matrices.items():
        for mode, matrix in modes.items():
            expectation, sd = gaussian_moments(matrix, signal, variance)
            row = dict(target=target, target_name=TARGETS[target-1], mode=mode,
                truth=float(truth[target-1]), expectation=expectation,
                bias=expectation-float(truth[target-1]), sd=sd,
                simulation_se=sd/np.sqrt(sampling_reps))
            if yy is not None:
                row["points"] = np.einsum("ij,ij->j", yy, matrix @ yy).tolist()
            moments.append(row)
        # Pairing removes almost all outcome uncertainty for the tiny correction.
        expectation, sd = gaussian_moments(modes["mean"]-modes["corrected"], signal, variance)
        paired.append(dict(target=target, target_name=TARGETS[target-1],
            expected_mean_minus_corrected=expectation, sd_mean_minus_corrected=sd,
            simulation_se=sd/np.sqrt(sampling_reps),
            expected_z_at_sampling_reps=expectation/(sd/np.sqrt(sampling_reps)) if sd else 0.))
    diagnostics.update(true_targets=truth.tolist(),
        positive_true_covariance=bool(truth[2] > 0),
        noise_variance_mean=float(variance.mean()), noise_variance_min=float(variance.min()),
        noise_variance_max=float(variance.max()),
        noise_variance_ratio=float(variance.max()/variance.min()) if variance.min() > 0 else None,
        residualized_variance_rms=float(np.sqrt(np.mean((M @ variance)**2))),
        positive_variance_rows=int(np.sum(variance > 0)), sampling_reps=int(sampling_reps))
    return dict(diagnostics=diagnostics, moments=moments, paired_centering=paired)


def main():
    p = argparse.ArgumentParser()
    p.add_argument("fixture", type=Path)
    p.add_argument("output", type=Path)
    p.add_argument("--draws", type=Path)
    p.add_argument("--sampling-reps", type=int, default=2000)
    a = p.parse_args()
    result = oracle(a.fixture, a.draws, sampling_reps=a.sampling_reps)
    a.output.parent.mkdir(parents=True, exist_ok=True)
    a.output.write_text(json.dumps(result, indent=2) + "\n")
    print(pd.DataFrame(result["moments"]).drop(columns="points", errors="ignore").to_string(index=False))
    print(pd.DataFrame(result["paired_centering"]).to_string(index=False))


if __name__ == "__main__":
    main()

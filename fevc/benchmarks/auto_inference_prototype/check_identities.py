"""Numerical check of the expectations the proposal relies on.

Compares pseudo-noise averages from the matrix-free code with closed forms
from explicit kernels on a small design, and the data statistic with its
stated expectation.  Exits non-zero if any relative error exceeds its bound.

Usage:
    python check_identities.py
"""

import sys

import numpy as np
from akm_design import make_design
from dense_mc import DenseOracle, true_effects, variance_scenarios
from sparse_auto import AutoSE


def main():
    d = make_design(120, 420, 60, 17)
    o = DenseOracle(d)
    a, p = true_effects(d, np.random.default_rng(18))
    mu = d.X(a, p)
    v = variance_scenarios(d, o)["mass"]
    A = AutoSE(d, exact_diag=(o.Pd, np.array(o.Bd)))
    w = 1.0 / o.Md
    n = d.n
    print("design n=%d workers=%d firms=%d" % (n, d.nW, d.nJ))

    # closed forms under the variance vector v
    tau = np.array([[v @ (o.C[t] * o.C[s]) @ v for s in range(3)] for t in range(3)])
    EV = o.MM @ v
    kvm = [(o.C[t] * o.M) @ v for t in range(3)]
    noise = np.array(
        [
            [np.sum(w * (EV * ((o.C[t] * o.C[s]) @ v) + 2 * kvm[t] * kvm[s])) for s in range(3)]
            for t in range(3)
        ]
    )
    lam = [
        np.linalg.eigvalsh((np.sqrt(v)[:, None] * o.C[t]) * np.sqrt(v)[None, :]) for t in range(3)
    ]
    reff = np.array([(ev**2).sum() ** 2 / (ev**4).sum() for ev in lam])

    # pseudo-noise averages from the matrix-free operators
    rng = np.random.default_rng(19)
    R = 4000
    tau_hat = np.zeros((3, 3))
    noise_hat = np.zeros((3, 3))
    diag = np.zeros(3)
    off = np.zeros(3)
    pairs = 0
    for _ in range(R // 100):
        zeta = (np.sqrt(v) * rng.standard_normal((n, 100)).T).T
        mz, kz = A.kernel(zeta)
        for t in range(3):
            for s in range(3):
                tau_hat[t, s] += ((v * kz[t].T).T * kz[s]).sum() / R
                noise_hat[t, s] += ((w * (mz**2).T).T * kz[t] * kz[s]).sum() / R
            U = (np.sqrt(v) * kz[t].T).T
            G = U.T @ U
            iu = np.triu_indices(100, 1)
            diag[t] += np.trace(G)
            off[t] += (G[iu] ** 2).sum()
        pairs += 100 * 99 // 2
    reff_hat = (diag / R) ** 2 / (off / pairs)

    # expectation of the data statistic over outcomes
    M = 20000
    Y = mu[:, None] + np.sqrt(v)[:, None] * rng.standard_normal((n, M))
    E = o.M @ Y
    stat = np.zeros((3, 3))
    G = [o.C[t] @ Y for t in range(3)]
    for t in range(3):
        for s in range(3):
            stat[t, s] = np.einsum("ir,ir,ir->r", G[t], E * E * w[:, None], G[s]).mean()
    m = [o.C[t] @ mu for t in range(3)]
    expect = np.array([[np.sum(w * EV * m[t] * m[s]) for s in range(3)] for t in range(3)]) + noise

    checks = [
        ("Hutchinson trace: tr(K_t V K_s V)", tau_hat, tau, 0.02),
        ("noise statistic N_ts(V)", noise_hat, noise, 0.03),
        ("effective rank from pairs of draws", reff_hat, reff, 0.05),
        ("E[sum_a h_a g_ta g_sa] = signal + N_ts", stat, expect, 0.02),
    ]
    bad = 0
    for name, got, want, tol in checks:
        scale = np.sqrt(np.outer(np.diag(want), np.diag(want))) if want.ndim == 2 else np.abs(want)
        err = float(np.max(np.abs(got - want) / scale))
        ok = err <= tol
        bad += not ok
        print(
            "%-42s max scaled error %.4f (bound %.2f) %s" % (name, err, tol, "ok" if ok else "FAIL")
        )
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())

"""Exact dense Monte Carlo for comparing variance-model strategies.

Everything is computed with explicit n x n kernels, so the true sampling
variance of the leave-out estimator is known in closed form for Gaussian
errors.  This isolates the statistical questions (which variance fit, which
truncation) from the numerical ones (JLA probes, trace draws).
"""

from __future__ import annotations

import warnings

import numpy as np
from scipy.linalg import qr
from scipy.stats import rankdata

TARGETS = ["worker", "firm", "cov", "total"]
T34 = np.array([[1, 0, 0], [0, 1, 0], [0, 0, 1], [1, 1, 2.0]])


def midrank(x):
    x = np.asarray(x, float)
    scale = np.max(np.abs(x))
    if scale == 0:
        return np.zeros_like(x)
    x = np.round(x / scale * 2.0**40)  # outcome-free tie grid
    if np.all(x == x[0]):
        return np.zeros_like(x)
    return 2.0 * (rankdata(x, method="average") - 0.5) / len(x) - 1.0


def quad_basis(cols, interactions=True):
    cols = [c for c in cols if np.ptp(c) > 0]
    Z = [np.ones_like(cols[0])] + cols + [c * c for c in cols]
    if interactions:
        for i in range(len(cols)):
            for j in range(i + 1, len(cols)):
                Z.append(cols[i] * cols[j])
    return np.column_stack(Z)


def reduce_basis(Z, tol=1e-10):
    """Drop numerically redundant columns (outcome-free)."""
    _, r, piv = qr(Z, mode="economic", pivoting=True)
    d = np.abs(np.diag(r))
    keep = np.sort(piv[d > tol * d[0]])
    return Z[:, keep]


def features(P, Bd, mass, stay, kind="common"):
    """Type-specific quadratic-in-midrank basis, as in structured_common."""
    n = len(P)
    blocks = []
    for typ in (False, True):
        idx = np.flatnonzero(stay == typ)
        if len(idx) == 0:
            continue
        if kind == "homo":
            cols = None
        elif kind == "lean":
            cols = [midrank(P[idx])] + ([] if typ else [midrank(mass[idx])])
        elif kind == "leverage":
            cols = [midrank(P[idx])]
        else:
            cols = [midrank(P[idx])] + [midrank(Bd[t][idx]) for t in range(3)]
            if not typ:
                cols.append(midrank(mass[idx]))
            else:  # structural zeros for stayers
                cols = [cols[0], cols[1]]
        if cols is None:
            Zt = np.ones((len(idx), 1))
        else:
            Zt = quad_basis(cols, interactions=(kind == "common"))
        Zt = reduce_basis(Zt)
        Zfull = np.zeros((n, Zt.shape[1]))
        Zfull[idx] = Zt
        blocks.append(Zfull)
    return np.column_stack(blocks)


class DenseOracle:
    def __init__(self, d, center=True):
        self.d = d
        X = d.dense_X()
        n = d.n
        S = X.T @ X
        Si = np.linalg.inv(S)
        XSi = X @ Si
        self.P = XSi @ X.T
        self.M = np.eye(n) - self.P
        self.Pd = np.diag(self.P).copy()
        self.Md = 1.0 - self.Pd
        # coefficient-space target matrices via the Design operators
        k = d.nW + d.nJ - 1
        E = np.eye(k)
        a = E[: d.nW]
        p = np.vstack([E[d.nW :], np.zeros((1, k))])
        self.B = []
        self.Bd = []
        self.C = []
        dvec = d.sF / np.linalg.norm(d.sF)
        H = np.eye(n) - np.outer(dvec, dvec) if center else np.eye(n)
        self.H = H
        for t in range(3):
            aw, af = d.A(t, a, p)
            At = np.vstack([aw, af[:-1]])
            At = 0.5 * (At + At.T)
            Bt = XSi @ At @ XSi.T
            bd = np.diag(Bt).copy()
            dt = bd / self.Md
            Ct = Bt - 0.5 * (dt[:, None] * self.M + self.M * dt[None, :])
            self.B.append(Bt)
            self.Bd.append(bd)
            self.C.append(H @ Ct @ H)  # kernel of the mean-centred estimator
        self.CC = {(t, s): self.C[t] * self.C[s] for t in range(3) for s in range(t, 3)}
        self.MM = self.M * self.M
        self.KM = [self.C[t] * self.M for t in range(3)]

    def true_cov(self, mu, v):
        """Exact Gaussian covariance of the three primitive estimators."""
        g = [C @ mu for C in self.C]
        om = np.zeros((3, 3))
        for t in range(3):
            for s in range(t, 3):
                om[t, s] = om[s, t] = 4 * (g[t] * v) @ g[s] + 2 * v @ self.CC[(t, s)] @ v
        return om

    def trace(self, V):
        """2 tr(V C_t V C_s) for an n x R matrix of fitted variances."""
        R = V.shape[1]
        out = np.zeros((R, 3, 3))
        for (t, s), W in self.CC.items():
            out[:, t, s] = out[:, s, t] = 2 * np.einsum("ir,ir->r", V, W @ V)
        return out


def fit_variances(o, Y, Z, method, floor_rel=1e-8):
    """Fitted variances (n x R) and the share of floored rows per sample."""
    E = o.M @ Y
    if method == "moment":  # current: K eta = Z'e^2, exact Gram
        K = Z.T @ o.MM @ Z
        eta = np.linalg.solve(K, Z.T @ (E * E))
    elif method == "proxy":  # proposed: OLS of leave-out products
        s = (o.H @ Y) * E / o.Md[:, None]
        eta = np.linalg.solve(Z.T @ Z, Z.T @ s)
    else:
        raise ValueError(method)
    V = Z @ eta
    floor = floor_rel * np.median(E * E / o.Md[:, None], axis=0)
    floored = (V < floor).mean(axis=0)
    return np.maximum(V, floor), floored


def evaluate(o, mu, v, R, seed, specs, errors="gauss"):
    """Monte Carlo over outcomes for a fixed design.

    Each spec has ``fit`` in {oracle, moment, proxy}, a basis ``Z`` unless
    oracle, and ``lin`` in {model, robust, dr, naive} for the signal term.
    Noise terms are computed exactly from the fitted variances.
    """
    rng = np.random.default_rng(seed)
    n = o.d.n
    if errors == "gauss":
        eps = rng.standard_normal((n, R))
    elif errors == "t5":
        eps = rng.standard_t(5, (n, R)) / np.sqrt(5 / 3)
    elif errors == "skew":
        eps = (rng.chisquare(3, (n, R)) - 3) / np.sqrt(6)
    else:
        raise ValueError(errors)
    Y = mu[:, None] + np.sqrt(v)[:, None] * eps
    E = o.M @ Y
    G = [C @ Y for C in o.C]
    th3 = np.array([np.einsum("ir,ir->r", Y, G[t]) for t in range(3)])
    th = T34 @ th3
    th0 = T34 @ np.array([mu @ o.C[t] @ mu for t in range(3)])
    true = T34 @ o.true_cov(mu, v) @ T34.T
    res = {
        "sd_true": np.sqrt(np.diag(true)),
        "sd_emp": th.std(axis=1, ddof=1),
        "bias": th.mean(axis=1) - th0,
        "theta0": th0,
        "specs": {},
    }
    w = (1.0 / o.Md)[:, None]
    for name, spec in specs.items():
        if spec["fit"] == "oracle":
            V = np.repeat(v[:, None], R, axis=1)
            floored = np.zeros(R)
        else:
            V, floored = fit_variances(o, Y, spec["Z"], spec["fit"])
        mode = spec.get("lin", "model")
        two_tau = o.trace(V)  # 2 tr(V K_t V K_s)
        signal = np.zeros((R, 3, 3))  # estimate of 4 (K_t mu)' V (K_s mu)
        if mode in ("robust", "dr"):
            EV = o.MM @ V  # model-implied E[e^2]
            km = [o.KM[t] @ V for t in range(3)]  # (K_t V M)_aa
        for t in range(3):
            for s in range(t, 3):
                if mode == "model":
                    stat = np.einsum("ir,ir,ir->r", G[t], V, G[s])
                    noise = 0.5 * two_tau[:, t, s]
                elif mode == "naive":  # plug-in of raw leave-out products
                    stat = np.einsum("ir,ir,ir->r", G[t], (o.H @ Y) * E * w, G[s])
                    noise = 0.5 * two_tau[:, t, s]
                elif mode == "robust":  # HC2 residual variances
                    stat = np.einsum("ir,ir,ir->r", G[t], E * E * w, G[s])
                    c = o.CC[(t, s)] @ V  # (K_t V K_s)_aa
                    noise = (w * (EV * c + 2 * km[t] * km[s])).sum(0)
                    if spec.get("kurt"):  # fourth-cumulant part of the noise
                        noise = noise + spec["kurt"] * (w * ((o.CC[(t, s)] * o.MM) @ (V * V))).sum(
                            0
                        )
                elif mode == "dr":  # model value plus residual surprise
                    stat = np.einsum("ir,ir,ir->r", G[t], V + (E * E - EV) * w, G[s])
                    c = o.CC[(t, s)] @ V
                    noise = (V * c + 2 * w * km[t] * km[s]).sum(0)
                else:
                    raise ValueError(mode)
                signal[:, t, s] = signal[:, s, t] = 4 * (stat - noise)
        out = {}
        lam, U = np.linalg.eigh(signal)
        psd = np.einsum("rij,rj,rkj->rik", U, np.maximum(lam, 0), U)
        for label, om3 in (("raw", signal + two_tau), ("trunc", psd + two_tau)):
            var = np.einsum("raa->ra", np.einsum("ai,rij,bj->rab", T34, om3, T34)).T
            ok = var > 0
            se = np.sqrt(np.where(ok, var, np.nan))
            cover = np.where(ok, np.abs(th - th0[:, None]) <= 1.959964 * se, False)
            with np.errstate(all="ignore"), warnings.catch_warnings():
                warnings.simplefilter("ignore")
                out[label] = dict(
                    avail=ok.mean(1),
                    cover=cover.mean(1),
                    rms_ratio=np.sqrt(np.nanmean(se**2, axis=1)) / res["sd_emp"],
                    med_ratio=np.nanmedian(se, axis=1) / res["sd_emp"],
                )
        out["floor_share"] = float(floored.mean())
        res["specs"][name] = out
    return res


def variance_scenarios(d, o, sig2=0.05):
    """Unit variances of the scaled rows under several heteroskedasticity laws."""
    F = d.F
    st = d.stay
    psi_t = d.psi_rank[d.f] - 0.5
    w_t = d.w_rank[d.w] - 0.5
    lev = midrank(o.Pd)
    sc = {}
    sc["homo"] = np.full(d.n, sig2)
    # match random effect + idiosyncratic noise; stayers noisier per observation
    sc["mass"] = np.where(st, 1.3 * sig2, sig2 * (0.6 + 0.25 * F))
    # smooth in leverage (inside the leverage/common span up to approximation)
    sc["leverage"] = sig2 * (1.0 + 0.8 * lev + 0.5 * lev**2)
    # omitted driver: firm wage level, which also drives the firm-variance influence
    sc["firm_omit"] = sig2 * (0.25 + 9.0 * psi_t**2)
    # omitted driver: worker type
    sc["worker_omit"] = sig2 * np.exp(-2.2 * w_t)
    return sc


def true_effects(d, rng, sd_a=0.30, sd_p=0.18):
    from scipy.stats import norm

    a = sd_a * norm.ppf(np.clip(d.w_rank, 1e-3, 1 - 1e-3)) + 0.05 * rng.standard_normal(d.nW)
    p = sd_p * norm.ppf(np.clip(d.psi_rank, 1e-3, 1 - 1e-3)) + 0.03 * rng.standard_normal(d.nJ)
    p = p - p[-1]
    return a, p

"""Matrix-free prototype of the proposed automatic standard errors.

Pass 1 mirrors FEVC's existing JLA point estimate (leverage and target
diagonals from Rademacher probes).  Pass 2 adds three influence solves and a
handful of Gaussian pseudo-noise draws, each pushed through the same kernel.
"""

from __future__ import annotations

import time

import numpy as np
from dense_mc import T34, features


class AutoSE:
    def __init__(self, d, probes=200, seed=0, exact_diag=None):
        self.d = d
        self.solves = 0
        rng = np.random.default_rng(seed)
        n = d.n
        t0 = time.time()
        if exact_diag is None:
            a = np.zeros(n)
            b = np.zeros(n)
            Bd = np.zeros((3, n))
            step = 25
            for k0 in range(0, probes, step):
                kk = min(step, probes - k0)
                r = rng.choice([-1.0, 1.0], size=(n, kk))
                pr = self.P(r)
                a += (pr**2).sum(1)
                b += ((r - pr) ** 2).sum(1)
                r2 = rng.choice([-1.0, 1.0], size=(n, kk))
                ya = d.X(*d.solve(*d.Ah_t(0, r2)))
                yp = d.X(*d.solve(*d.Ah_t(1, r2)))
                self.solves += 2 * kk
                Bd[0] += (ya**2).sum(1)
                Bd[1] += (yp**2).sum(1)
                Bd[2] += (ya * yp).sum(1)
            self.Pd = a / (a + b)  # normalised leverage estimate
            self.Bd = Bd / probes
        else:
            self.Pd, self.Bd = exact_diag
        self.Md = 1.0 - self.Pd
        self.dt = self.Bd / self.Md
        self.t_pass1 = time.time() - t0
        self.solves_pass1 = self.solves
        self.Z = features(self.Pd, self.Bd, d.F, d.stay, "common")
        self.ZtZ = self.Z.T @ self.Z

    # ---- operators -------------------------------------------------------
    def beta(self, v):
        self.solves += 1 if v.ndim == 1 else v.shape[1]
        return self.d.solve(*self.d.Xt(v))

    def P(self, v):
        return self.d.X(*self.beta(v))

    def H(self, v):
        d = self.d
        return v - np.multiply.outer(d.sF, (d.sF @ v) / d.F.sum())

    def kernel(self, v, bv=None):
        """Return (M v, [K_t v for t]) for the mean-centred leave-out kernels."""
        d = self.d
        hv = self.H(v)
        a, p = self.beta(hv) if bv is None else bv
        mv = hv - d.X(a, p)
        out = []
        for t in range(3):
            dtv = (self.dt[t] * hv.T).T
            ra, rp = d.A(t, a, p)
            xa, xp = d.Xt(0.5 * dtv)
            self.solves += 1 if v.ndim == 1 else v.shape[1]
            c = d.X(*d.solve(ra + xa, rp + xp)) - 0.5 * (self.dt[t] * mv.T).T - 0.5 * dtv
            out.append(self.H(c))
        return mv, out

    # ---- estimation ------------------------------------------------------
    def fit(self, y, draws=8, seed=1, floor_rel=1e-8, model=True):
        d = self.d
        rng = np.random.default_rng(seed)
        s0 = self.solves
        t0 = time.time()
        z = self.H(y)
        a, p = self.beta(z)
        e, g = self.kernel(y, bv=(a, p))
        theta3 = d.theta(a, p) - (self.dt * z * e).sum(1)
        w = 1.0 / self.Md
        s = z * e * w  # leave-out products (KSS)
        if model:
            eta = np.linalg.solve(self.ZtZ, self.Z.T @ s)
            v = self.Z @ eta
        else:
            v = np.full(d.n, s.mean())
        hc2 = e * e * w
        floor = floor_rel * np.median(hc2)
        floored = float((v < floor).mean())
        v = np.maximum(v, floor)
        # pass 2: pseudo-noise through the same kernel
        xi = rng.standard_normal((d.n, draws))
        zeta = (np.sqrt(v) * xi.T).T
        mz, kz = self.kernel(zeta)
        tau = np.zeros((3, 3))
        noise = np.zeros((3, 3))
        stat_m = np.zeros((3, 3))
        stat_r = np.zeros((3, 3))
        tau_b = np.zeros((draws, 3, 3))
        q = np.array([(zeta * kz[t]).sum(0) for t in range(3)])  # z'K z draws
        for t in range(3):
            for u in range(t, 3):
                tb = ((v * kz[t].T).T * kz[u]).sum(0)
                tau_b[:, t, u] = tau_b[:, u, t] = tb
                tau[t, u] = tau[u, t] = tb.mean()
                nb = ((w * (mz**2).T).T * kz[t] * kz[u]).sum(0)
                noise[t, u] = noise[u, t] = nb.mean()
                stat_m[t, u] = stat_m[u, t] = (v * g[t] * g[u]).sum()
                stat_r[t, u] = stat_r[u, t] = (hc2 * g[t] * g[u]).sum()
        V_model = 4 * stat_m - 2 * tau
        V_rob = 4 * (stat_r - noise) + 2 * tau
        # spectral concentration from pairs of draws: tr(W^4) / tr(W^2)^2
        reff = np.full(3, np.nan)
        if draws >= 2:
            for t in range(3):
                U = (np.sqrt(v) * kz[t].T).T
                Gm = U.T @ U
                off = Gm[np.triu_indices(draws, 1)]
                reff[t] = np.mean(np.diag(Gm)) ** 2 / np.mean(off**2)
        infl = np.array([np.max(hc2 * g[t] ** 2) / np.sum(hc2 * g[t] ** 2) for t in range(3)])
        return dict(
            theta=T34 @ theta3,
            V_model=T34 @ V_model @ T34.T,
            V_rob=T34 @ V_rob @ T34.T,
            tau=tau,
            tau_b=tau_b,
            q=q,
            noise=noise,
            stat_m=stat_m,
            stat_r=stat_r,
            floored=floored,
            reff=reff,
            infl=infl,
            v=v,
            g=g,
            e=e,
            solves_pass2=self.solves - s0,
            t_pass2=time.time() - t0,
        )


def psd_floor(V3, tau):
    """Variance = signal + 2 tau with the signal part truncated to be PSD."""
    sig = V3 - 2 * tau
    w, U = np.linalg.eigh(0.5 * (sig + sig.T))
    return (U * np.maximum(w, 0)) @ U.T + 2 * tau


def true_cov(A, mu, v, draws=64, seed=99):
    """Exact-in-expectation covariance of the estimator with A's kernel, known mu and v."""
    rng = np.random.default_rng(seed)
    _, g = A.kernel(mu)
    zeta = (np.sqrt(v) * rng.standard_normal((A.d.n, draws)).T).T
    _, kz = A.kernel(zeta)
    om = np.zeros((3, 3))
    for t in range(3):
        for u in range(t, 3):
            tr = ((v * kz[t].T).T * kz[u]).sum(0).mean()
            om[t, u] = om[u, t] = 4 * (v * g[t] * g[u]).sum() + 2 * tr
    return T34 @ om @ T34.T

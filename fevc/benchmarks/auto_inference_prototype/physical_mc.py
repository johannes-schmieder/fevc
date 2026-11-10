"""Physical-row harness: controls, deletion units and centering conventions.

The collapsed harness in ``dense_mc.py`` has one row per independent unit and
no controls.  Here every physical observation is a row, so that

* controls can vary within a deletion unit and can be estimated jointly;
* deletion units can be matches, declared parts of a match, or observations;
* the corrected-centering point estimate can be formed.

Point estimators are computed exactly from dense kernels.  The standard error
is always computed in one geometry: deletion units collapsed to one row each
after subtracting the fitted control index, plus a first-order term for the
estimation of the control coefficients.
"""

from __future__ import annotations

import numpy as np

from akm_design import Design
from dense_mc import T34, features


def expand(d, deletion, rng):
    """Physical rows of a collapsed design and their deletion-unit labels."""
    mass = d.F.astype(int)
    unit0 = np.repeat(np.arange(d.n), mass)
    pos = np.concatenate([np.arange(k) for k in mass])
    if deletion == "match":
        unit = unit0
    elif deletion == "split":  # two declared units inside each longer mover match
        second = (~d.stay[unit0]) & (mass[unit0] >= 2) & (pos >= (mass[unit0] + 1) // 2)
        _, unit = np.unique(2 * unit0 + second, return_inverse=True)
    elif deletion == "observation":
        unit = np.arange(len(unit0))
    else:
        raise ValueError(deletion)
    return dict(
        unit0=unit0,
        pos=pos.astype(float),
        unit=unit,
        w=d.w[unit0],
        f=d.f[unit0],
        stay=d.stay[unit0],
    )


def make_controls(d, ph, k, rng):
    """Controls that vary within units and are correlated with firm type."""
    nP = len(ph["unit"])
    if k == 0:
        return np.zeros((nP, 0))
    ft = d.psi_rank[ph["f"]] - 0.5
    start = rng.normal(0, 2.0, d.nW)[ph["w"]] + rng.normal(0, 1.0, d.n)[ph["unit0"]]
    cols = [
        start + ph["pos"] + 0.3 * rng.standard_normal(nP),
        ft * (1.0 + ph["pos"])
        + 0.8 * ft * rng.standard_normal(d.n)[ph["unit0"]]
        + 0.3 * rng.standard_normal(nP),
    ]
    for j in range(2, k):  # further controls with weaker links to firm type
        cols.append(
            0.5 * ft * rng.standard_normal(d.n)[ph["unit0"]]
            + rng.standard_normal(nP)
            + 0.2 * j * ph["pos"]
        )
    Z = np.column_stack(cols[:k])
    return Z - Z.mean(0)


class PhysicalOracle:
    """Dense kernels for one design, deletion rule and control set."""

    def __init__(self, d, ph, Z, center=True):
        self.d, self.ph, self.Z = d, ph, Z
        unit = ph["unit"]
        nP = len(unit)
        nU = unit.max() + 1
        self.nP, self.nU, self.k = nP, nU, Z.shape[1]
        w, f = ph["w"], ph["f"]
        # --- units: one row per deletion unit -------------------------------
        first = np.zeros(nU, int)
        first[unit[::-1]] = np.arange(nP)[::-1]
        mU = np.bincount(unit).astype(float)
        self.du = Design(w[first], f[first], mU, ph["stay"][first])
        du = self.du
        self.T = np.zeros((nP, nU))
        self.T[np.arange(nP), unit] = 1.0 / np.sqrt(mU[unit])
        Xc = du.dense_X()
        Sci = np.linalg.inv(Xc.T @ Xc)
        self.Mc = np.eye(nU) - Xc @ Sci @ Xc.T
        self.Mcd = np.diag(self.Mc).copy()
        kfe = du.nW + du.nJ - 1
        eye = np.eye(kfe)
        aw = eye[: du.nW]
        pf = np.vstack([eye[du.nW :], np.zeros((1, kfe))])
        s = du.sF / np.linalg.norm(du.sF)
        self.Hc = np.eye(nU) - np.outer(s, s) if center else np.eye(nU)
        self.Kc, self.Bcd = [], []
        Acoef = []
        for t in range(3):
            ra, rp = du.A(t, aw, pf)
            At = np.vstack([ra, rp[:-1]])
            At = 0.5 * (At + At.T)
            Acoef.append(At)
            Bt = Xc @ Sci @ At @ Sci @ Xc.T
            dt = np.diag(Bt) / self.Mcd
            Ct = Bt - 0.5 * (dt[:, None] * self.Mc + self.Mc * dt[None, :])
            self.Kc.append(self.Hc @ Ct @ self.Hc)
            self.Bcd.append(np.diag(Bt).copy())
        self.KK = {(t, u): self.Kc[t] * self.Kc[u] for t in range(3) for u in range(t, 3)}
        self.KM = [self.Kc[t] * self.Mc for t in range(3)]
        self.MM = self.Mc * self.Mc
        # --- controls, residualised on the two-way effects -------------------
        Xp = self.T @ Xc
        if self.k:
            self.Zc = self.T.T @ Z
            self.Zt = Z - Xp @ (Sci @ (Xp.T @ Z))
            self.Lam = np.linalg.inv(self.Zt.T @ self.Zt)
            self.Ztc = self.Mc @ self.Zc
            self.Zw = Z - self.T @ self.Zc
        # --- joint leave-unit-out estimator on physical rows -----------------
        X = np.column_stack([Xp, Z])
        Si = np.linalg.inv(X.T @ X)
        XSi = X @ Si
        M = np.eye(nP) - XSi @ X.T
        self.gamma_map = Si[kfe:] @ X.T if self.k else np.zeros((0, nP))
        Hp = np.eye(nP) - 1.0 / nP if center else np.eye(nP)
        idx = [np.flatnonzero(unit == g) for g in range(nU)] if mU.max() > 1 else None
        self.KJ = []
        for t in range(3):
            A = np.zeros((kfe + self.k, kfe + self.k))
            A[:kfe, :kfe] = Acoef[t]
            B = XSi @ A @ XSi.T
            if idx is None:
                RM = (np.diag(B) / np.diag(M))[:, None] * M
            else:
                RM = np.zeros((nP, nP))
                for ix in idx:
                    RM[ix] = B[np.ix_(ix, ix)] @ np.linalg.solve(M[np.ix_(ix, ix)], M[ix])
            HRM = Hp @ RM
            self.KJ.append(B - 0.5 * (HRM + HRM.T))
        # --- corrected centering (two-way design after the offset) -----------
        n = float(nP)
        self.corr_map = np.linalg.solve(n * np.diag(self.Mcd / mU) - self.Mc, self.Mc)
        self.sqm = np.sqrt(mU)

    def joint_variance(self, mu, Gam):
        """Exact Gaussian covariance and mean of the joint estimator."""
        km = [K @ mu for K in self.KJ]
        kg = [K @ Gam for K in self.KJ]
        om = np.zeros((3, 3))
        for t in range(3):
            for u in range(3):
                om[t, u] = 4 * km[t] @ Gam @ km[u] + 2 * np.sum(kg[t] * kg[u].T)
        mean = np.array([mu @ km[t] + np.trace(kg[t]) for t in range(3)])
        return om, mean


def error_cov(d, ph, scenario, s_m2=0.02, s_u2=0.04):
    """Block covariance of physical errors; independence across deletion units."""
    unit = ph["unit"]
    nP = len(unit)
    ft = d.psi_rank[ph["f"]] - 0.5
    wt = d.w_rank[ph["w"]] - 0.5
    scale = dict(base=np.ones(nP), firm_omit=0.25 + 9.0 * ft**2, worker_omit=np.exp(-2.2 * wt))[
        scenario
    ]
    same = unit[:, None] == unit[None, :]
    root = np.sqrt(scale)
    Gam = (s_m2 * same + s_u2 * np.eye(nP)) * root[:, None] * root[None, :]
    return Gam


def evaluate(o, mu, Gam, reps, seed, model="lean", control_term=True, chunk=200):
    """Monte Carlo of joint, fixed-offset and corrected estimates with one SE."""
    rng = np.random.default_rng(seed)
    du = o.du
    nP = o.nP
    VJ, meanJ = o.joint_variance(mu, Gam)
    L = np.linalg.cholesky(Gam + 1e-12 * np.eye(nP))
    feats = features(1.0 - o.Mcd, o.Bcd, du.F, du.stay, model)
    ZtZi = np.linalg.inv(feats.T @ feats)
    w = 1.0 / o.Mcd
    out = dict(thJ=[], thF=[], thC=[], var=[], var_nc=[])
    for start in range(0, reps, chunk):
        R = min(chunk, reps - start)
        Y = mu[:, None] + L @ rng.standard_normal((nP, R))
        thJ = np.array([np.einsum("ir,ir->r", Y, K @ Y) for K in o.KJ])
        gam = o.gamma_map @ Y
        U = Y - o.Z @ gam if o.k else Y
        uc = o.T.T @ U
        zc = o.Hc @ uc
        ec = o.Mc @ uc
        G = [K @ uc for K in o.Kc]
        thF = np.array([np.einsum("ir,ir->r", uc, G[t]) for t in range(3)])
        s = zc * ec * w[:, None]
        # corrected centering: mean-centred estimate plus the documented increment
        kappa = (o.corr_map @ (o.sqm[:, None] * s)) / o.sqm[:, None]
        thC = thF - np.array([o.Bcd[t] @ kappa for t in range(3)])
        # working variance from leave-out products
        V = feats @ (ZtZi @ (feats.T @ s))
        floor = 1e-8 * np.median(ec * ec * w[:, None], axis=0)
        V = np.maximum(V, floor)
        et = ec * np.sqrt(w)[:, None]
        EV = o.MM @ V
        km = [o.KM[t] @ V for t in range(3)]
        if o.k:
            Uw = U - o.T @ uc
            W = np.stack([o.T.T @ (o.Zw[:, [j]] * Uw) * o.sqm[:, None] for j in range(o.k)])
        psi, psi0 = [], []
        for t in range(3):
            base = 2 * G[t] * et
            psi0.append(base)
            if o.k and control_term:
                lam = o.Lam @ (-2 * o.Zc.T @ G[t])
                base = (2 * G[t] + o.Ztc @ lam) * et + np.einsum("kur,kr->ur", W, lam)
            psi.append(base)
        var = np.zeros((R, 3, 3))
        var0 = np.zeros((R, 3, 3))
        two_tau = np.zeros((R, 3, 3))
        for t in range(3):
            for u in range(t, 3):
                c = o.KK[(t, u)] @ V
                tt = 2 * np.einsum("ir,ir->r", V, c)
                noise = (w[:, None] * (EV * c + 2 * km[t] * km[u])).sum(0)
                two_tau[:, t, u] = two_tau[:, u, t] = tt
                var[:, t, u] = var[:, u, t] = (psi[t] * psi[u]).sum(0) - 4 * noise
                var0[:, t, u] = var0[:, u, t] = (psi0[t] * psi0[u]).sum(0) - 4 * noise
        for sig, key in ((var, "var"), (var0, "var_nc")):
            lam_, Q = np.linalg.eigh(sig)
            psd = np.einsum("rij,rj,rkj->rik", Q, np.maximum(lam_, 0), Q) + two_tau
            out[key].append(np.einsum("raa->ra", np.einsum("ai,rij,bj->rab", T34, psd, T34)))
        out["thJ"].append(thJ)
        out["thF"].append(thF)
        out["thC"].append(thC)
    res = {
        k: (np.concatenate(v, axis=1) if k.startswith("th") else np.concatenate(v, axis=0).T)
        for k, v in out.items()
    }
    res = {k: (T34 @ v if k.startswith("th") else v) for k, v in res.items()}
    res["sdJ"] = np.sqrt(np.diag(T34 @ VJ @ T34.T))
    res["meanJ"] = T34 @ meanJ
    return res

"""Matrix-free prototype runs.

Usage:
    python run_sparse.py draws
        Numerical noise of the standard error against the number of draws.
    python run_sparse.py mc FIRMS MOVERS STAYERS SEED SCENARIOS REPS PROBES DRAWS [fast] [SD_PSI]
        Monte Carlo of the whole pipeline with random-projection leverages.
"""

import sys
import time

import numpy as np
from akm_design import make_design, make_design_fast
from dense_mc import T34, DenseOracle, true_effects, variance_scenarios
from sparse_auto import AutoSE, psd_floor, true_cov


def scen_v(d, P, name, sig2=0.05):
    F = d.F
    st = d.stay
    psi_t = d.psi_rank[d.f] - 0.5
    w_t = d.w_rank[d.w] - 0.5
    if name == "homo":
        return np.full(d.n, sig2)
    if name == "mass":
        return np.where(st, 1.3 * sig2, sig2 * (0.6 + 0.25 * F))
    if name == "firm_omit":
        return sig2 * (0.25 + 9.0 * psi_t**2)
    if name == "worker_omit":
        return sig2 * np.exp(-2.2 * w_t)
    raise ValueError(name)


def e1(seed=5):
    """Numerical noise of the SE from pseudo-draws, against exact dense traces."""
    d = make_design(500, 1700, 150, seed)
    o = DenseOracle(d)
    rng = np.random.default_rng(seed + 1)
    a, p = true_effects(d, rng)
    mu = d.X(a, p)
    v = variance_scenarios(d, o)["mass"]
    y = mu + np.sqrt(v) * rng.standard_normal(d.n)
    A = AutoSE(d, exact_diag=(o.Pd, np.array(o.Bd)))
    out = {}
    for R2 in (1, 2, 4, 8, 16, 64):
        se_r = []
        se_m = []
        tau = []
        noise = []
        reff = []
        for s in range(40):
            r = A.fit(y, draws=R2, seed=100 + s)
            se_r.append(np.sqrt(np.diag(r["V_rob"])))
            se_m.append(np.sqrt(np.diag(r["V_model"])))
            tau.append(np.diag(r["tau"]))
            noise.append(np.diag(r["noise"]))
            reff.append(r["reff"])
        se_r, se_m, tau, noise = map(np.array, (se_r, se_m, tau, noise))
        out[R2] = dict(
            cv_se_rob=(se_r.std(0) / se_r.mean(0)).round(4).tolist(),
            cv_se_model=(se_m.std(0) / se_m.mean(0)).round(4).tolist(),
            cv_tau=(tau.std(0) / tau.mean(0)).round(3).tolist(),
            cv_noise=(noise.std(0) / noise.mean(0)).round(3).tolist(),
            solves=r["solves_pass2"],
        )
        print(
            "R2=%2d solves %3d  CV(SE robust) %s  CV(SE model) %s  CV(tau) %s CV(noise) %s"
            % (
                R2,
                r["solves_pass2"],
                out[R2]["cv_se_rob"],
                out[R2]["cv_se_model"],
                out[R2]["cv_tau"],
                out[R2]["cv_noise"],
            ),
            flush=True,
        )
    # comparator: variance of z'Kz (the current trace estimator), same fitted variances
    r = A.fit(y, draws=2000, seed=7)
    q = r["q"]
    V = r["v"]
    tr_exact = np.array([V @ (o.C[t] * o.C[t]) @ V for t in range(3)])
    lam = [
        np.linalg.eigvalsh((np.sqrt(V)[:, None] * o.C[t]) * np.sqrt(V)[None, :]) for t in range(3)
    ]
    print(
        "exact effective rank (sum l^2)^2/sum l^4:",
        [round(float((ev**2).sum() ** 2 / (ev**4).sum()), 1) for ev in lam],
        " leading share l1^2/sum l^2:",
        [round(float((ev**2).max() / (ev**2).sum()), 4) for ev in lam],
    )
    print(
        "r_eff estimated from 8/64 draws:",
        A.fit(y, draws=8, seed=3)["reff"].round(1),
        A.fit(y, draws=64, seed=3)["reff"].round(1),
    )
    for R in (50, 200, 1000):
        est = np.array(
            [[q[t, k : k + R].var(ddof=1) / 2 for t in range(3)] for k in range(0, 2000 - R + 1, R)]
        )
        print(
            "variance-of-quadratic-form trace estimator, R=%4d solves: CV %s  mean/exact %s"
            % (R, (est.std(0) / est.mean(0)).round(3), (est.mean(0) / tr_exact).round(3))
        )
    hut = r["tau_b"][:, [0, 1, 2], [0, 1, 2]]
    print(
        "per-draw Hutchinson (4 solves): CV %s mean/exact %s"
        % ((hut.std(0) / hut.mean(0)).round(3), (hut.mean(0) / tr_exact).round(3))
    )


def mc(nf, nm, ns, seed, scen, reps, probes, draws, fast=False, sd_p=0.18, exact=False):
    """Monte Carlo of the complete matrix-free pipeline with JLA diagonals."""
    t0 = time.time()
    d = (make_design_fast if fast else make_design)(nf, nm, ns, seed)
    rng = np.random.default_rng(seed + 1)
    a, p = true_effects(d, rng, sd_p=sd_p)
    mu = d.X(a, p)
    th0 = T34 @ d.theta(a, p)
    if exact:
        o = DenseOracle(d)
        A = AutoSE(d, exact_diag=(o.Pd, np.array(o.Bd)))
    else:
        A = AutoSE(d, probes=probes, seed=seed + 2)
    v = scen_v(d, A.Pd, scen)
    print(
        "n=%d workers=%d firms=%d scen=%s probes=%s draws=%d  setup %.1fs pass1 %.1fs (%d solves)"
        % (
            d.n,
            d.nW,
            d.nJ,
            scen,
            "exact" if exact else probes,
            draws,
            time.time() - t0 - A.t_pass1,
            A.t_pass1,
            A.solves_pass1,
        ),
        flush=True,
    )
    th = []
    se = {"model": [], "robust": [], "model_t": [], "robust_t": []}
    t2 = []
    s2 = 0
    reff = []
    for r_ in range(reps):
        y = mu + np.sqrt(v) * rng.standard_normal(d.n)
        r = A.fit(y, draws=draws, seed=1000 + r_)
        th.append(r["theta"])
        t2.append(r["t_pass2"])
        s2 = r["solves_pass2"]
        reff.append(r["reff"])
        Vm3 = np.linalg.lstsq(T34, np.linalg.lstsq(T34, r["V_model"], rcond=None)[0].T, rcond=None)[
            0
        ]
        Vr3 = np.linalg.lstsq(T34, np.linalg.lstsq(T34, r["V_rob"], rcond=None)[0].T, rcond=None)[0]
        se["model"].append(np.diag(r["V_model"]))
        se["robust"].append(np.diag(r["V_rob"]))
        se["model_t"].append(np.diag(T34 @ psd_floor(Vm3, r["tau"]) @ T34.T))
        se["robust_t"].append(np.diag(T34 @ psd_floor(Vr3, r["tau"]) @ T34.T))
    th = np.array(th)
    sd = th.std(0, ddof=1)
    sd_true = np.sqrt(np.diag(true_cov(A, mu, v)))
    print(
        "  analytic SD of this estimator %s (Monte Carlo SD / analytic %s)"
        % (sd_true.round(5), (sd / sd_true).round(3))
    )
    print(
        "  theta0 %s  mean est %s  SD %s  bias/SD %s   pass2: %d solves, %.2fs per fit; r_eff med %s"
        % (
            th0.round(4),
            th.mean(0).round(4),
            sd.round(5),
            ((th.mean(0) - th0) / sd).round(2),
            s2,
            np.mean(t2),
            np.nanmedian(np.array(reff), 0).round(0),
        )
    )
    res = {}
    for k, val in se.items():
        val = np.array(val)
        ok = val > 0
        s = np.sqrt(np.where(ok, val, np.nan))
        cov = np.where(ok, np.abs(th - th0) <= 1.959964 * s, False).mean(0)
        covc = np.where(ok, np.abs(th - th.mean(0)) <= 1.959964 * s, False).mean(0)
        ratio = np.sqrt(np.nanmean(s**2, 0)) / sd_true
        print(
            "  %-9s avail %s  rmsSE/trueSD %s  cover(theta) %s  cover(centred) %s"
            % (k, ok.mean(0).round(3), ratio.round(3), cov.round(3), covc.round(3)),
            flush=True,
        )
        res[k] = dict(avail=ok.mean(0).tolist(), ratio=ratio.tolist(), cover=cov.tolist())
    return res


if __name__ == "__main__":
    which = sys.argv[1]
    if which == "draws":
        e1()
    elif which == "mc":
        nf, nm, ns, seed = map(int, sys.argv[2:6])
        scen = sys.argv[6]
        reps, probes, draws = map(int, sys.argv[7:10])
        fast = len(sys.argv) > 10 and sys.argv[10] == "fast"
        sd_p = float(sys.argv[11]) if len(sys.argv) > 11 else 0.18
        for s in scen.split(","):
            mc(nf, nm, ns, seed, s, reps, probes, draws, fast=fast, sd_p=sd_p)

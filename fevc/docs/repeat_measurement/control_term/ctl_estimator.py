"""Control coefficients in the repeat-measurement standard error: is the proposed ESTIMATOR right on average?

Same dense panel as ctl_theory.py (imported from it), no shared code with the package. For every draw:

  g-hat, the offset outcome y^c, m-hat = C y^c, J-hat = -2 Z^c' m-hat, c-hat = Lam J-hat, m-hat* = Pi m-hat,
  leave-out residuals e-tilde = e / M_ii, realised within-unit products w_a,
  unit variances: the sample variance of y^c within every repeat-measurement set (a worker-firm cell with two or more
  one-observation units), the TRUE variance for the other units (the fitted model is checked elsewhere),

  V-hat = sum_a [ v_a (2 m*_s)(2 m*_t) + 2 m*_s e-tilde w'c_t + 2 m*_t e-tilde w'c_s + (w'c_s)(w'c_t) ]
          - 4 tr(V Pi C_s V C_t Pi') - 4 tr(Om-hat Lam Z' C_s V C_t Z Lam) + 2 tr(C_s V C_t V) + pair term,

with exact traces (the package simulates them) and the pair term of the method without controls. Variants are
accumulated to show what each piece does. The mean of V-hat is compared with the Monte Carlo covariance of theta-hat.
"""
import sys
import numpy as np
import ctl_theory as T

rng = np.random.default_rng(777)
n, q, w, f, Z, Ztil, Lam, mu, gamma, sig2 = T.n, T.q, T.w, T.f, T.Z, T.Ztil, T.Lam, T.mu, T.gamma, T.sig2


def run(kind, R):
    groups, mass, uw, uf, agg, dev = T.structure(kind); m = len(groups)
    X = T.design(uw, uf, mass); C, Tm, Sinv = T.kernels(X, mass, float(n))
    P = X @ Sinv @ X.T; Md = 1 - np.diag(P); Mres = np.eye(m) - P
    Zc = agg @ Z; Ztc = agg @ Ztil; Pi = np.eye(m) - Ztc @ Lam @ Zc.T; v_true = agg ** 2 @ sig2
    cell = {}
    for a in range(m):
        if mass[a] == 1: cell.setdefault((uw[a], uf[a]), []).append(a)
    sets = [np.array(u) for u in cell.values() if len(u) >= 2]; size = np.array([len(u) for u in sets]); fac = (size - 2) * (size - 1) / (size + 1.0)
    cU = np.array([[C[t][u[0], u[1]] for u in sets] for t in range(3)])                 # the constant weight between two units of a set
    PiC = [Pi @ Ct for Ct in C]
    sqPi = [[PiC[s] * PiC[t] for t in range(3)] for s in range(3)]; sqC = [[C[s] * C[t] for t in range(3)] for s in range(3)]
    G = [Lam @ Zc.T @ Ct for Ct in C]                                                   # k x m: Lam Z' C_t
    names = ['none', 'unit part', '+ within', '+ cross', '+ noise of c-hat (full)']; acc = {k: np.zeros((3, 3)) for k in names}; sq = np.zeros((3, 3))
    th = np.zeros((R, 3)); sd = np.sqrt(sig2)
    for r in range(R):
        y = mu + Z @ gamma + sd * rng.standard_normal(n); gh = Lam @ (Ztil.T @ y); ystar = y - Z @ gh; yc = agg @ ystar
        mh = np.column_stack([Ct @ yc for Ct in C]); th[r] = yc @ mh
        e = Mres @ yc; et = e / Md
        v = v_true.copy(); qU = np.zeros(len(sets))
        for k, u in enumerate(sets): qU[k] = yc[u].var(ddof=1); v[u] = qU[k]
        wv = np.zeros((m, q))
        for a, g in enumerate(groups):
            if len(g) > 1: wv[a] = dev[g].T @ (ystar[g] - ystar[g].mean())
        ch = Lam @ (-2 * Zc.T @ mh); ms = mh + 0.5 * Ztc @ ch; wc = wv @ ch; Om = wv.T @ wv
        pair = np.array([[4 * (cU[s] * cU[t] * qU * qU * fac).sum() for t in range(3)] for s in range(3)])
        quad = np.array([[v @ sqC[s][t] @ v for t in range(3)] for s in range(3)])
        quadPi = np.array([[v @ sqPi[s][t] @ v for t in range(3)] for s in range(3)])
        noise_c = np.array([[4 * np.trace(Om @ (G[s] * v) @ G[t].T) for t in range(3)] for s in range(3)])
        base = 4 * (mh * v[:, None]).T @ mh - 2 * quad + pair                                   # the method without controls
        unit = 4 * (ms * v[:, None]).T @ ms - 4 * quadPi + 2 * quad + pair
        within = wc.T @ wc
        cross = 2 * ((ms * et[:, None]).T @ wc + wc.T @ (ms * et[:, None]))
        full = unit + within + cross - noise_c
        for k, val in zip(names, (base, unit, unit + within, unit + within + cross, full)): acc[k] += val
        sq += full ** 2
    mc = np.cov(th.T); T4 = np.array([[1, 0, 0], [0, 1, 0], [0, 0, 1], [1, 1, 2.0]]); comp = ['worker', 'firm', 'cov', 'sum']
    mean_full = acc[names[-1]] / R; se = np.sqrt(np.maximum(sq / R - mean_full ** 2, 0) / R)
    print(f'\n{kind} units: {m} units, {len(sets)} repeat-measurement sets, {R} draws; relative MC s.e. of a variance {np.sqrt(2 / (R - 1)):.3f}')
    print('  mean of V-hat / Monte Carlo variance of theta-hat')
    print('  component  ' + '  '.join(f'{k:>24s}' for k in names))
    for j in range(4):
        a = T4[j]; e_ = a @ mc @ a
        print(f'  {comp[j]:9s}  ' + '  '.join(f'{(a @ (acc[k] / R) @ a) / e_:24.3f}' for k in names))
    print('  off-diagonal entries of the full estimate / Monte Carlo covariance:', ' '.join(f'{mean_full[s, t] / mc[s, t]:.3f}' for s, t in ((0, 1), (0, 2), (1, 2))))
    print('  relative s.e. of the mean of the full estimate (diagonal):', np.round(np.diag(se) / np.diag(mean_full), 4))


if __name__ == '__main__':
    R = int(sys.argv[1]) if len(sys.argv) > 1 else 4000
    for kind in ('match', 'observation'): run(kind, R)

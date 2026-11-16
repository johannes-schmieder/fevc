"""The S formulation, checked against the EXACT variance (Gaussian errors, no Monte Carlo for the truth).

The noise of the control-adjusted unit outcome is v = Hc e with Hc = T - Zc Lam Ztil' (T aggregates person-years
into unit rows, e the person-year errors), so S = Var(v) = Hc Sig Hc'. For Gaussian errors

    Var(theta-hat_t) = 4 m_t' S m_t + 2 tr(C_t S C_t S),        E theta-hat_t - theta_t = tr(C_t S).

Estimators averaged over data draws (exact traces; unit variances: sample variance within every repeat-measurement
set, the true variance elsewhere):

  oracle   4 m-hat' S m-hat - 2 tr(C S C S) with the true S                      (must be unbiased: a check of the code)
  v1       linear - 4 tr(V Pi C V C Pi') - 4 tr(Om Lam Z' C V C Z Lam) + 2 tr(C V C V) + pair      (SPEC v1)
  v2       linear - 2 tr(C Sd C Sd) + pair,  Sd = Pi' V Pi + Z Lam Om Lam Z'               (S everywhere, no K in Sd)
  none     4 sum v m-hat^2 - 2 tr(C V C V) + pair                                         (controls taken as known)

with linear = sum_a [4 m*_a^2 v_a + 4 m*_a e-tilde_a w_a' c + (w_a' c)^2] and Om = sum_a w_a w_a'.
Reported: mean of each estimator / exact variance, for worker, firm, covariance and sum.
"""
import sys
import numpy as np
import ctl_theory as T

rng = np.random.default_rng(4242)
n, q, Z, Ztil, Lam, mu, gamma, sig2 = T.n, T.q, T.Z, T.Ztil, T.Lam, T.mu, T.gamma, T.sig2


def run(kind, R):
    groups, mass, uw, uf, agg, dev = T.structure(kind); m = len(groups)
    X = T.design(uw, uf, mass); C3, Tm, Sinv = T.kernels(X, mass, float(n)); C = C3 + [C3[0] + C3[1] + 2 * C3[2]]
    P = X @ Sinv @ X.T; Md = 1 - np.diag(P); Mres = np.eye(m) - P
    Zc = agg @ Z; Ztc = agg @ Ztil; Pi = np.eye(m) - Ztc @ Lam @ Zc.T; A = Zc @ Lam
    Hc = agg - Zc @ Lam @ Ztil.T; S = (Hc * sig2) @ Hc.T; muc = agg @ mu
    mtrue = [Ct @ muc for Ct in C]
    exact = np.array([4 * mtrue[t] @ S @ mtrue[t] + 2 * np.trace(C[t] @ S @ C[t] @ S) for t in range(4)])
    bias = np.array([np.trace(C[t] @ S) for t in range(4)]); known = np.array([4 * (agg ** 2 @ sig2) @ mtrue[t] ** 2 + 2 * (agg ** 2 @ sig2) @ (C[t] * C[t]) @ (agg ** 2 @ sig2) for t in range(4)])
    trS = np.array([np.trace(C[t] @ S @ C[t] @ S) for t in range(4)])
    v_true = agg ** 2 @ sig2
    cell = {}
    for a in range(m):
        if mass[a] == 1: cell.setdefault((uw[a], uf[a]), []).append(a)
    sets = [np.array(u) for u in cell.values() if len(u) >= 2]; size = np.array([len(u) for u in sets]); fac = (size - 2) * (size - 1) / (size + 1.0)
    cU = np.array([[C[t][u[0], u[1]] for u in sets] for t in range(4)])
    sqC = [Ct * Ct for Ct in C]; PiC = [Pi @ Ct for Ct in C]; sqPi = [G * G for G in PiC]; CA = [Ct @ A for Ct in C]; CZt = [Ct @ Ztc for Ct in C]
    acc = {k: np.zeros(4) for k in ('oracle', 'v1', 'v2', 'none')}; sd = np.sqrt(sig2); th = np.zeros((R, 4))
    for r in range(R):
        y = mu + Z @ gamma + sd * rng.standard_normal(n); gh = Lam @ (Ztil.T @ y); ystar = y - Z @ gh; yc = agg @ ystar
        e = Mres @ yc; et = e / Md
        v = v_true.copy(); qU = np.zeros(len(sets))
        for k, u in enumerate(sets): qU[k] = yc[u].var(ddof=1); v[u] = qU[k]
        wv = np.zeros((m, q))
        for a, g in enumerate(groups):
            if len(g) > 1: wv[a] = dev[g].T @ (ystar[g] - ystar[g].mean())
        Om = wv.T @ wv; Gam = (Ztc * v[:, None]).T @ Ztc + Om; N = np.block([[Gam, -np.eye(q)], [-np.eye(q), np.zeros((q, q))]])
        for t in range(4):
            mh = C[t] @ yc; th[r, t] = yc @ mh
            ch = Lam @ (-2 * Zc.T @ mh); ms = mh + 0.5 * Ztc @ ch; wc = wv @ ch
            pair = 4 * (cU[t] ** 2 * qU * qU * fac).sum()
            linear = 4 * (v * ms * ms).sum() + 4 * (ms * et * wc).sum() + (wc * wc).sum()
            trV = v @ sqC[t] @ v
            acc['oracle'][t] += 4 * mh @ S @ mh - 2 * trS[t]
            acc['none'][t] += 4 * (v * mh * mh).sum() - 2 * trV + pair
            acc['v1'][t] += linear - 4 * (v @ sqPi[t] @ v) - 4 * np.trace(Om @ (CA[t] * v[:, None]).T @ CA[t]) + 2 * trV + pair
            # tr(C Sd C Sd), Sd = V + F N F' with F = [A, V Ztil^c]
            CF = np.column_stack([CA[t], C[t] @ (Ztc * v[:, None])]); F = np.column_stack([A, Ztc * v[:, None]]); G = F.T @ CF
            trSd = trV + 2 * np.trace(N @ (CF * v[:, None]).T @ CF) + np.trace(N @ G @ N @ G)
            acc['v2'][t] += linear - 2 * trSd + pair
    comp = ['worker', 'firm', 'cov', 'sum']
    print(f'\n{kind} units ({m} units, {len(sets)} sets), {R} data draws. Exact variance by the Gaussian formula with S.')
    print('  component  exact sd    MC sd/exact  bias/sd   known-controls formula/exact   mean estimate / exact variance:  oracle     none       v1       v2')
    for t in range(4):
        print(f'  {comp[t]:9s}  {np.sqrt(exact[t]):.3e}   {th[:, t].std(ddof=1) / np.sqrt(exact[t]):.3f}       {bias[t] / np.sqrt(exact[t]):+.3f}     {known[t] / exact[t]:.3f}'
              + ' ' * 46 + '  '.join(f'{acc[k][t] / R / exact[t]:7.3f}' for k in ('oracle', 'none', 'v1', 'v2')))


if __name__ == '__main__':
    R = int(sys.argv[1]) if len(sys.argv) > 1 else 1500
    for kind in ('match', 'observation'): run(kind, R)

"""Control coefficients in the repeat-measurement standard error: is the first-order term right?

Dense check on a small panel; shares no code with the package. Person-year rows i with worker, firm and two
controls z_i that are correlated with the effects and vary inside every match; errors are independent across
person-years with variances that depend on the within-match deviation of a control (so that nothing cancels by
symmetry). Two unit structures:

  match        a mover's match is one unit (row sqrt(F_a)(e_w + e_f), outcome sum_i (y_i - z_i'g-hat)/sqrt(F_a)),
               every stayer observation its own unit;
  observation  every person-year its own unit.

Estimators: the fixed-offset estimate theta-tilde(g-hat) = y^c' C_t y^c (C_t the leave-out kernel of the unit design
without controls, g-hat the control coefficient of the full regression), and under observation units also the joint
estimate (controls inside every leave-out fit).

Formulas compared with the Monte Carlo covariance of the estimates, all with the true variances:

  V0 = 4 sum_a v_a g_sa g_ta + 2 sum_{a != b} C^s_ab C^t_ab v_a v_b                        (controls taken as known)
  V1 = sum_a v_a (2 Pi g_s)_a (2 Pi g_t)_a + sum_a [ 2 (Pi g_s)_a k_a' c_t + 2 (Pi g_t)_a k_a' c_s ]
       + sum_a c_s' Om_a c_t + the same quadratic part,

with g_t = C_t mu^c, Pi = I - Ztil^c Lam Z^c', c_t = Lam J_t, J_t = -2 Z^c' g_t, Lam = (sum_i ztil_i ztil_i')^{-1},
k_a = Cov(eps^c_a, om_a) = sum_{i in a} (z_i - zbar_a) sig_i^2 / sqrt(F_a), Om_a = Var(om_a) =
sum_{i in a} (z_i - zbar_a)(z_i - zbar_a)' sig_i^2, om_a = sum_{i in a} (z_i - zbar_a) eps_i.

The same for the score of a fixed direction d: b-hat = d' y^c has variance sum_a v_a (Pi d)_a^2 + 2 sum_a (Pi d)_a
k_a' c_b + sum_a c_b' Om_a c_b with c_b = -Lam Z^c' d.
"""
import sys
import numpy as np

rng = np.random.default_rng(20261114)
Wn, J, T = 260, 16, 6
rows = []
for k in range(Wn):
    if rng.random() < 0.45:
        j = int(rng.integers(J)); rows += [(k, j)] * int(rng.integers(2, T + 1))
    else:
        for j in rng.choice(J, size=int(rng.integers(2, 4)), replace=False): rows += [(k, int(j))] * int(rng.integers(1, 4))
w = np.array([r[0] for r in rows]); f = np.array([r[1] for r in rows]); n = len(w); p = Wn + J - 1
alpha = 0.4 * rng.standard_normal(Wn); psi = 0.25 * rng.standard_normal(J); psi[-1] = 0.0
u = 0.25 * rng.standard_normal(n)
Z = np.column_stack([0.4 * alpha[w] + 0.8 * psi[f] + u, 0.3 * psi[f] + 0.5 * rng.standard_normal(n)]); gamma = np.array([0.3, -0.2]); q = 2
sig2 = 0.03 * np.exp(0.6 * rng.standard_normal(n) + 2.0 * u)                     # depends on the within-match part of the first control
import os
if os.environ.get('CTL_VAR') == 'cell':                                           # one variance per worker-firm cell (the method's assumption for a set)
    _key = {}
    for _i in range(n): _key.setdefault((w[_i], f[_i]), 0.03 * np.exp(0.9 * rng.standard_normal()))
    sig2 = np.array([_key[(w[_i], f[_i])] for _i in range(n)])
mu = alpha[w] + psi[f]


def design(units_w, units_f, mass):
    m = len(units_w); X = np.zeros((m, p)); r = np.sqrt(mass)
    X[np.arange(m), units_w] = r
    for a in range(m):
        if units_f[a] < J - 1: X[a, Wn + units_f[a]] = r[a]
    return X


def kernels(X, mass, total):
    """Leave-out kernels of the worker variance, the firm variance and the covariance (person-year weights)."""
    Sinv = np.linalg.inv(X.T @ X); P = X @ Sinv @ X.T; Md = 1 - np.diag(P); assert Md.min() > 1e-8
    M = np.eye(len(X)) - P; r = np.sqrt(mass)
    Aa = np.zeros((len(X), p)); Aa[:, :Wn] = X[:, :Wn]; Af = np.zeros((len(X), p)); Af[:, Wn:] = X[:, Wn:]
    cen = lambda G: G - np.outer(r, (r @ G) / total)                              # person-year weighted centring in unit rows
    Tm = [cen(Aa).T @ cen(Aa) / total, cen(Af).T @ cen(Af) / total, 0.5 * (cen(Aa).T @ cen(Af) + cen(Af).T @ cen(Aa)) / total]
    C = []
    for t in range(3):
        B = X @ Sinv @ Tm[t] @ Sinv @ X.T; d = np.diag(B) / Md; Ct = B - 0.5 * (d[:, None] * M + M * d[None, :]); np.fill_diagonal(Ct, 0.0); C.append(Ct)
    return C, Tm, Sinv


# person-year design without controls (for residualising the controls) and with them
Xpy = design(w, f, np.ones(n)); Spy = np.linalg.inv(Xpy.T @ Xpy); Ztil = Z - Xpy @ (Spy @ (Xpy.T @ Z)); Lam = np.linalg.inv(Ztil.T @ Ztil)
nf = np.array([len(set(f[w == k])) for k in range(Wn)]); mover = nf[w] > 1


def structure(kind):
    if kind == 'observation': groups = [[i] for i in range(n)]
    else:
        cell = {}
        for i in range(n):
            key = (w[i], f[i]) if mover[i] else ('row', i)
            cell.setdefault(key, []).append(i)
        groups = list(cell.values())
    m = len(groups); mass = np.array([len(g) for g in groups], float); uw = np.array([w[g[0]] for g in groups]); uf = np.array([f[g[0]] for g in groups])
    agg = np.zeros((m, n))
    for a, g in enumerate(groups): agg[a, g] = 1 / np.sqrt(len(g))
    dev = np.zeros((n, q))
    for g in groups: dev[g] = Z[g] - Z[g].mean(0)
    return groups, mass, uw, uf, agg, dev


def run(kind, R):
    groups, mass, uw, uf, agg, dev = structure(kind); m = len(groups)
    X = design(uw, uf, mass); C, Tm, Sinv = kernels(X, mass, float(n))
    Zc = agg @ Z; Ztc = agg @ Ztil; v = agg ** 2 @ sig2; muc = agg @ mu
    kcov = np.zeros((m, q)); Om = np.zeros((m, q, q))
    for a, g in enumerate(groups):
        kcov[a] = (dev[g] * sig2[g][:, None]).sum(0) / np.sqrt(len(g)); Om[a] = (dev[g][:, :, None] * dev[g][:, None, :] * sig2[g][:, None, None]).sum(0)
    Pi = np.eye(m) - Ztc @ Lam @ Zc.T
    g = [Ct @ muc for Ct in C]; c = [Lam @ (-2 * Zc.T @ gt) for gt in g]
    quad = np.array([[2 * v @ (C[s] * C[t]) @ v for t in range(3)] for s in range(3)])
    V0 = np.array([[4 * (v * g[s] * g[t]).sum() for t in range(3)] for s in range(3)]) + quad
    lin = np.zeros((3, 3)); cross = np.zeros((3, 3)); within = np.zeros((3, 3))
    for s in range(3):
        for t in range(3):
            lin[s, t] = (v * (2 * Pi @ g[s]) * (2 * Pi @ g[t])).sum()
            cross[s, t] = (2 * (Pi @ g[s]) * (kcov @ c[t])).sum() + (2 * (Pi @ g[t]) * (kcov @ c[s])).sum()
            within[s, t] = np.einsum('i,aij,j->', c[s], Om, c[t])
    V1 = lin + cross + within + quad
    # a fixed direction: the leading eigenvector of the firm-variance form in the normal-matrix inner product
    L = np.linalg.cholesky(np.linalg.inv(Sinv)); Li = np.linalg.inv(L); ev, vec = np.linalg.eigh(Li @ Tm[1] @ Li.T); d = X @ (Li.T @ vec[:, -1]); d /= np.linalg.norm(d)
    cb = -Lam @ (Zc.T @ d); vb0 = (v * d * d).sum(); vb1 = (v * (Pi @ d) ** 2).sum() + 2 * ((Pi @ d) * (kcov @ cb)).sum() + np.einsum('i,aij,j->', cb, Om, cb)
    joint = None
    if kind == 'observation':                                                    # controls inside every leave-out fit
        Xf = np.column_stack([Xpy, Z]); Sf = np.linalg.inv(Xf.T @ Xf); Pf = Xf @ Sf @ Xf.T; Mdf = 1 - np.diag(Pf); Mf = np.eye(n) - Pf; joint = []
        for t in range(3):
            A = np.zeros((p + q, p + q)); A[:p, :p] = Tm[t]; B = Xf @ Sf @ A @ Sf @ Xf.T; dd = np.diag(B) / Mdf
            Ct = B - 0.5 * (dd[:, None] * Mf + Mf * dd[None, :]); np.fill_diagonal(Ct, 0.0); joint.append(Ct)
    th = np.zeros((R, 3)); thj = np.zeros((R, 3)); bh = np.zeros(R); sd = np.sqrt(sig2)
    for r in range(R):
        y = mu + Z @ gamma + sd * rng.standard_normal(n); gh = Lam @ (Ztil.T @ y); yc = agg @ (y - Z @ gh)
        th[r] = [yc @ (Ct @ yc) for Ct in C]; bh[r] = d @ yc
        if joint is not None: thj[r] = [y @ (Ct @ y) for Ct in joint]
    mc = np.cov(th.T); names = ['worker', 'firm', 'cov']; T4 = np.array([[1, 0, 0], [0, 1, 0], [0, 0, 1], [1, 1, 2.0]]); se = np.sqrt(2.0 / (R - 1))
    print(f'\n{kind} units: {m} units ({int((mass > 1).sum())} of several observations), {R} draws; relative MC s.e. of a variance {se:.3f}')
    print('  component   MC sd      known-controls formula / MC   with control term / MC   (linear, cross, within shares of the term)')
    for j, name in enumerate(names + ['sum']):
        a = T4[j]; e = a @ mc @ a; v0 = a @ V0 @ a; v1 = a @ V1 @ a
        print(f'  {name:9s} {np.sqrt(e):.3e}   {v0 / e:.3f}                          {v1 / e:.3f}                   '
              f'{(a @ (lin + quad - V0) @ a) / (v1 - v0 + 1e-300):+.2f} {(a @ cross @ a) / (v1 - v0 + 1e-300):+.2f} {(a @ within @ a) / (v1 - v0 + 1e-300):+.2f}')
    print(f'  score of a fixed direction: MC variance {bh.var(ddof=1):.3e}; known-controls / MC {vb0 / bh.var(ddof=1):.3f}; with control term / MC {vb1 / bh.var(ddof=1):.3f}')
    if joint is not None:
        mcj = np.cov(thj.T); dj = thj - th
        print('  joint estimate: variance / with control term:', ' '.join(f'{(T4[j] @ mcj @ T4[j]) / (T4[j] @ V1 @ T4[j]):.3f}' for j in range(4)),
              '; sd(joint - fixed offset) / sd:', ' '.join(f'{(dj @ T4[j]).std() / np.sqrt(T4[j] @ mc @ T4[j]):.3f}' for j in range(4)))


if __name__ == '__main__':
  R = int(sys.argv[1]) if len(sys.argv) > 1 else 20000
  print(f'person-years {n}, workers {Wn}, firms {J}; controls {q}; sd of g-hat: {np.sqrt(np.diag(Lam @ (Ztil.T * sig2) @ Ztil @ Lam)).round(4)}')
  for kind in ('match', 'observation'): run(kind, R)

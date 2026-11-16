"""Observation deletion: is the repeat-measurement variance estimator exactly unbiased when every worker-firm cell with
two or more observations is a repeat-measurement set?  Dense check on a small panel; shares no code with the package.

Rows are person-years. Sets U: cells (worker x firm) with |U| >= 2; within a set the regressors are identical, the errors
independent with a common variance. Single-observation cells keep their true variance here (the model is checked
elsewhere), so the check isolates the sets. For every set the weight between two of its rows is c_U = B_U / (1 - P_U),
and

  V-hat[s,t] = 4 sum_i m-hat_i^s m-hat_i^t v_i - 2 sum_{i != l} C^s_il C^t_il v_i v_l
               + 4 sum_U c^s_U c^t_U q_U^2 (|U|-2)(|U|-1)/(|U|+1).
"""
import sys
import numpy as np

rng = np.random.default_rng(20261113)
W, J, T = 220, 14, 6
rows = []
for k in range(W):
    kind = rng.random()
    if kind < 0.45:                                    # stayer with 2..T years
        j = rng.integers(J); rows += [(k, j)] * int(rng.integers(2, T + 1))
    else:                                              # mover: 2 or 3 spells of 1..3 years
        firms = rng.choice(J, size=int(rng.integers(2, 4)), replace=False)
        for j in firms: rows += [(k, int(j))] * int(rng.integers(1, 4))
w = np.array([r[0] for r in rows]); f = np.array([r[1] for r in rows]); n = len(w); p = W + J - 1
X = np.zeros((n, p)); X[np.arange(n), w] = 1.0
for i in range(n):
    if f[i] < J - 1: X[i, W + f[i]] = 1.0
Sinv = np.linalg.inv(X.T @ X); P = X @ Sinv @ X.T; lev = np.diag(P).copy(); Md = 1 - lev
assert lev.max() < 1 - 1e-8, 'a row cannot be deleted'
M = np.eye(n) - P
cen = lambda Z: Z - Z.mean(0)
Aa = np.zeros((n, p)); Aa[:, :W] = X[:, :W]; Af = np.zeros((n, p)); Af[:, W:] = X[:, W:]
Tm = [cen(Aa).T @ cen(Aa) / n, cen(Af).T @ cen(Af) / n, 0.5 * (cen(Aa).T @ cen(Af) + cen(Af).T @ cen(Aa)) / n]
C = []; Bd = []
for t in range(3):
    B = X @ Sinv @ Tm[t] @ Sinv @ X.T; d = np.diag(B) / Md; Ct = B - 0.5 * (d[:, None] * M + M * d[None, :]); np.fill_diagonal(Ct, 0.0)
    C.append(Ct); Bd.append(np.diag(B).copy())
cell = {}
for i in range(n): cell.setdefault((w[i], f[i]), []).append(i)
sets = [np.array(v) for v in cell.values() if len(v) >= 2]; single = np.array([v[0] for v in cell.values() if len(v) == 1])
nfirm = np.array([len({f[i] for i in np.flatnonzero(w == k)}) for k in range(W)]); mover_set = [nfirm[w[U[0]]] > 1 for U in sets]
print(f'rows {n}  sets {len(sets)} (of movers: {sum(mover_set)})  single-observation rows {len(single)}  max leverage {lev.max():.3f}')
# the weight between two rows of a set is constant and equals B_U / (1 - P_U)
worst = 0.0; cU = np.zeros((3, len(sets)))
for u, U in enumerate(sets):
    for t in range(3):
        blk = C[t][np.ix_(U, U)]; off = blk[~np.eye(len(U), dtype=bool)]; c = Bd[t][U[0]] / Md[U[0]]
        worst = max(worst, np.abs(off - c).max() / max(abs(c), 1e-300) if abs(c) > 1e-14 else np.abs(off).max()); cU[t, u] = c
print(f'largest relative deviation of C_il from B_U/(1-P_U) within a set: {worst:.2e}')
print('share of sets of movers with a nonzero firm-variance weight:', np.mean([abs(cU[1, u]) > 1e-12 for u in range(len(sets)) if mover_set[u]]))
# truth
alpha = 0.4 * rng.standard_normal(W); psi = 0.25 * rng.standard_normal(J); psi[-1] = 0.0; mu = alpha[w] + psi[f]
sig2 = np.zeros(n)
for U in sets: sig2[U] = 0.03 * np.exp(0.8 * rng.standard_normal() + 0.5 * alpha[w[U[0]]])       # heteroskedastic across sets
sig2[single] = 0.03 * np.exp(0.8 * rng.standard_normal(len(single)))
m = [Ct @ mu for Ct in C]; C2 = [[C[s] * C[t] for t in range(3)] for s in range(3)]
Vtrue = np.array([[4 * (m[s] * m[t] * sig2).sum() + 2 * sig2 @ C2[s][t] @ sig2 for t in range(3)] for s in range(3)])
theta0 = np.array([mu @ Ct @ mu for Ct in C])
R = int(sys.argv[1]) if len(sys.argv) > 1 else 20000
size = np.array([len(U) for U in sets]); fac = (size - 2) * (size - 1) / (size + 1.0)
acc = np.zeros((3, 3)); acc_nopair = np.zeros((3, 3)); th = np.zeros((R, 3)); sq = np.zeros((3, 3))
for r in range(R):
    y = mu + np.sqrt(sig2) * rng.standard_normal(n); v = sig2.copy(); q = np.zeros(len(sets))
    for u, U in enumerate(sets): q[u] = y[U].var(ddof=1); v[U] = q[u]
    mh = [Ct @ y for Ct in C]; th[r] = [y @ mh[t] for t in range(3)]
    Vh = np.array([[4 * (mh[s] * mh[t] * v).sum() - 2 * v @ C2[s][t] @ v for t in range(3)] for s in range(3)])
    pair = np.array([[4 * (cU[s] * cU[t] * q * q * fac).sum() for t in range(3)] for s in range(3)])
    acc_nopair += Vh; Vh = Vh + pair; acc += Vh; sq += Vh ** 2
mean = acc / R; se = np.sqrt(np.maximum(sq / R - mean ** 2, 0) / R); mc = np.cov(th.T)
names = ['person', 'firm', 'cov']
print(f'\n{R} draws. Entry: true V, mean of V-hat (its MC s.e.), ratio, z; without the pair term: ratio; MC variance of theta-hat / true V')
for s in range(3):
    for t in range(s, 3):
        print(f'  {names[s]:6s} {names[t]:6s} {Vtrue[s, t]:+.4e} {mean[s, t]:+.4e} ({se[s, t]:.1e}) ratio {mean[s, t] / Vtrue[s, t]:.4f} z {(mean[s, t] - Vtrue[s, t]) / se[s, t]:+.2f}   no pair term {acc_nopair[s, t] / R / Vtrue[s, t]:.4f}   MC/true {mc[s, t] / Vtrue[s, t]:.4f}')
print('bias of theta-hat in MC s.e.:', np.round((th.mean(0) - theta0) / (th.std(0) / np.sqrt(R)), 2))

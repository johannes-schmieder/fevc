"""Checks of lp_q1.py against dense exact computations on a tiny two-block design (exact diagonals)."""
import sys, numpy as np, scipy.linalg as sla
from lp6 import build_design, aggregate, T34
from lp_q1 import LPQ, interval
from lp_q1sim import two_block_panel, truth
cell = dict(nJ=12, nW=260, n_bridge=4, design_seed=3, gap=0.3, variance='profile')
P, blk, bridge = two_block_panel(cell['nJ'], cell['nW'], cell['n_bridge'], cell['design_seed']); D, info = build_design(P)
X = D.dense_X(); S = X.T @ X; Sinv = np.linalg.inv(S); k = D.nW + D.nJ - 1; n = D.n
def targ(t):
    M = np.zeros((k, k)); I = np.eye(k)
    for j in range(k):
        ra, rp = D.A(t, I[:D.nW, j], np.r_[I[D.nW:, j], 0.0]); M[:, j] = np.r_[ra, rp[:-1]]
    return M
At = [targ(t) for t in range(3)]; Pm = X @ Sinv @ X.T; Mm = np.eye(n) - Pm; Pd = np.diag(Pm).copy()
Bd = np.array([np.einsum('ij,jk,ik->i', X @ Sinv, At[t], X @ Sinv) for t in range(3)])
E = LPQ(D, exact_diag=(Pd, Bd)); h = D.sF; H = np.eye(n) - np.outer(h, h) / (h @ h)
alpha, psi, v_py = truth(P, blk, cell, 3); mu = D.X(alpha, psi); v_rows = aggregate(info, n, v_py * info['py_w']); rng = np.random.default_rng(1)
print('rows', n, 'movers rows', int((~D.stay).sum()), 'firms', D.nJ, 'workers', D.nW)
worst = dict(lam=0.0, w1=0.0, identity=0.0, mr=0.0, stayer_c=0.0)
for c in range(4):
    Ac = sum(T34[c, t] * At[t] for t in range(3)); Ac = 0.5 * (Ac + Ac.T); lam_all, V = sla.eigh(Ac, S); j = int(np.argmax(np.abs(lam_all))); lam_d = lam_all[j]; w_d = X @ V[:, j]
    lam, w1, _, steps = E.leading(c); sgn = np.sign(w1 @ w_d); gapratio = np.sort(np.abs(lam_all))[-2] / abs(lam_d)
    worst['lam'] = max(worst['lam'], abs(lam / lam_d - 1))
    if gapratio < 0.99: worst['w1'] = max(worst['w1'], np.abs(w1 - sgn * w_d).max())   # eigenvector only defined when the eigenvalue is simple
    lam_d, w_d, sgn = lam, w1, 1.0                                                    # dense formulas below use the engine's pair
    # dense remainder kernel (leave-out weights for A - lam S v v' S), centred
    def lo_kernel(B):
        dd = np.diag(B) / np.diag(Mm); C = B - 0.5 * (dd[:, None] * Mm + Mm * dd[None, :]); return C
    Bc = X @ Sinv @ Ac @ Sinv @ X.T; Cr = H @ lo_kernel(Bc - lam_d * np.outer(w_d, w_d)) @ H
    y = mu + np.sqrt(v_rows) * rng.standard_normal(n); q = E.q1(y, c, lam, w1, draws=8, vtrue=v_rows)
    worst['identity'] = max(worst['identity'], abs(q['theta_r'] - y @ Cr @ y) / abs(q['theta_r']))
    o = E.fit(y); mr = T34[c] @ o['g'] - lam * E.K1(w1, o['z'], o['e']); worst['mr'] = max(worst['mr'], np.abs(mr - Cr @ y).max() / np.abs(mr).max())
    # stayer pair weight of the remainder
    C0 = lo_kernel(Bc - lam_d * np.outer(w_d, w_d)); first = E.st_rows[np.r_[True, E.st_s[1:] != E.st_s[:-1]]]
    cr = ((T34[c] @ Bd)[first] - lam * w1[first] ** 2) * E.R / (E.R - 1.0)
    for s_i in range(0, len(E.sw), 7):
        rows = E.st_rows[E.st_s == s_i]; off = C0[np.ix_(rows, rows)][~np.eye(len(rows), dtype=bool)]
        worst['stayer_c'] = max(worst['stayer_c'], np.abs(off - cr[s_i]).max() / np.abs(C0).max())
    # exact covariance of (b1, theta_r) with the dense kernel against q1_truth
    mrt = Cr @ mu; Sex = np.array([[np.sum(w_d ** 2 * v_rows), 2 * sgn * np.sum(w_d * mrt * v_rows)], [0, 4 * np.sum(mrt ** 2 * v_rows) + 2 * v_rows @ (Cr * Cr) @ v_rows]]); Sex[1, 0] = Sex[0, 1]
    tr = E.q1_truth(mu, v_rows, c, lam, w1, draws=4096); St = np.array(tr['Sigma'])
    print(f"comp {c}: lam1 {lam:+.5f} (dense {lam_d:+.5f}), share lam1^2/sum {lam_d ** 2 / (lam_all ** 2).sum():.3f}, steps {steps}, max w1^2 {np.max(w1 ** 2):.3f}; "
          f"truth via draws / exact: {St[0, 0] / Sex[0, 0]:.4f} {St[0, 1] / Sex[0, 1]:.4f} {St[1, 1] / Sex[1, 1]:.4f}")
print('worst deviations', {k: f'{v:.2e}' for k, v in worst.items()})

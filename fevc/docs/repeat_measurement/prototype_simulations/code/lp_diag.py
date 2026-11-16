"""Convergence diagnostics on the six-year design (SCC; aggregates only).  Usage: lp_diag.py <design-dir> <out-dir>
D2: sqrt(mean squared off-diagonal entry of the Gram matrix of S^(1/2) K S^(1/2) zeta_d) / V-hat (cheap bound on
lambda_max^2 / V); D1: lambda_max^2 / V-hat by power iteration; largest share of V-hat carried by one stayer."""
import sys, os, json, time, numpy as np
from lp import Panel, build_design, aggregate, LP, T34
ddir, odir = sys.argv[1], sys.argv[2]; os.makedirs(odir, exist_ok=True); t0 = time.time()
z = np.load(ddir + '/panel.npz'); P = Panel(z['w'], z['f'], z['year'], z['y']); D, info = build_design(P); y = aggregate(info, D.n, P.y)
dz = np.load(ddir + '/diag_default_200.npz'); E = LP(D, exact_diag=(dz['Pd'], dz['Bd']))
f = E.fit(y); g4 = T34 @ f['g']; ss = E.stayer_stats(y, f['g']); Vx = np.maximum(E.stayer_var(f['va'], ss), 1e-300); sv = np.sqrt(Vx)
rng = np.random.default_rng(11); res = {}; G = np.zeros((4, 32, 32)); rown = np.zeros((4, D.n)); tau4 = np.zeros(4)
U = []
for b in range(2):
    zeta = (sv * rng.standard_normal((D.n, 16)).T).T; _, kz = E.A.kernel(zeta); k4 = np.einsum('ct,tnd->cnd', T34, np.array(kz))
    rown += (k4 ** 2).mean(2) / 2; U.append(k4 * sv[None, :, None])
U = np.concatenate(U, axis=2)                                           # (4, n, 32)
for c in range(4): G[c] = U[c].T @ U[c]
off = ~np.eye(32, dtype=bool); noise = np.array([np.trace(G[c]) / 32 for c in range(4)])
signal = (Vx * g4 ** 2).sum(1); Vhat = 4 * signal - 2 * noise - np.diag(T34 @ E.adjust(ss, 3.0) @ T34.T)  # corrected sign (was +2 noise in diag-v1)
res['Vhat'] = Vhat.tolist(); res['signal_share'] = (4 * signal / Vhat).tolist()
res['D2_32draws'] = (np.sqrt(np.array([(G[c][off] ** 2).mean() for c in range(4)])) / Vhat).tolist()
res['D2_first16'] = (np.sqrt(np.array([(G[c][:16, :16][off[:16, :16]] ** 2).mean() for c in range(4)])) / Vhat).tolist()
rowV = Vx * (4 * g4 ** 2 + 2 * rown)                                    # row contributions to V-hat
st = E.st_rows; per = np.array([np.bincount(E.st_s, rowV[c][st], len(E.sw)) for c in range(4)])
res['max_stayer_share'] = (per.max(1) / Vhat).tolist(); res['stayer_share_total'] = (per.sum(1) / Vhat).tolist()
res['max_mover_row_share'] = (rowV[:, ~D.stay].max(1) / Vhat).tolist()
def Bmul(c, x):
    _, kk = E.A.kernel(sv * x); return sv * (T34[c] @ np.array(kk))
lam2 = []
for c in range(4):
    x = rng.standard_normal(D.n); x /= np.linalg.norm(x)
    for it in range(25):
        yv = Bmul(c, Bmul(c, x)); l2 = np.linalg.norm(yv); x = yv / l2
    lam2.append(l2); print('component', c, 'lambda_max^2', l2, 'D1', l2 / Vhat[c], flush=True)
res['D1'] = (np.array(lam2) / Vhat).tolist(); res['seconds'] = time.time() - t0
tmp = odir + '/diag.json.tmp'; json.dump(res, open(tmp, 'w'), indent=1); os.replace(tmp, odir + '/diag.json'); print(json.dumps(res, indent=1))

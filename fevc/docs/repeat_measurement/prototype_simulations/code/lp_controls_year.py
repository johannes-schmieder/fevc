"""Size of the first-order term for estimated year effects on the six-year Veneto panel (SCC; aggregates only).
The adjusted wage in the panel file already has year effects and age terms removed; identifiers in the file with
age do not match, so only the year effects are treated here: Z = year dummies, outcome = adjusted wage.
Usage: lp_controls_year.py <design-dir> <out-dir>"""
import sys, os, json, time, numpy as np
from lp import Panel, build_design, aggregate, LP, T34
ddir, odir = sys.argv[1], sys.argv[2]; os.makedirs(odir, exist_ok=True); t0 = time.time(); res = {}
z = np.load(ddir + '/panel.npz'); P = Panel(z['w'], z['f'], z['year'], z['y'])
years = np.sort(np.unique(P.year)); Z = np.column_stack([(P.year == yv).astype(float) for yv in years[:-1]]); p = Z.shape[1]
D, info = build_design(P); dz = np.load(ddir + '/diag_default_200.npz'); E = LP(D, exact_diag=(dz['Pd'], dz['Bd']))
rw = np.column_stack([np.bincount(P.w, Z[:, j], P.nW) for j in range(p)]); rf = np.column_stack([np.bincount(P.f, Z[:, j], P.nJ) for j in range(p)])
aw, pf = D.solve(rw, rf); Zt = Z - (aw[P.w] + pf[P.f]); G = Zt.T @ Zt; Lam = np.linalg.inv(G)
res['normal_equation_check'] = float(np.abs(np.column_stack([np.bincount(P.w, Zt[:, j], P.nW) for j in range(p)])).max())
res['gamma_on_adjusted'] = (Lam @ (Zt.T @ P.y)).tolist()                      # should be ~0: year effects were already removed
y = aggregate(info, D.n, P.y); f = E.fit(y); g = f['g']; g4 = T34 @ g; e = f['e']; sr = np.sqrt(E.r)
ss = E.stayer_stats(y, g); Vx = E.stayer_var(f['va'], ss)
Zc = np.column_stack([aggregate(info, D.n, Z[:, j]) for j in range(p)]); Ztc = np.column_stack([aggregate(info, D.n, Zt[:, j]) for j in range(p)])
row = info['py_row']; F = D.F
zbar = np.column_stack([np.bincount(row, Z[:, j], D.n) / F for j in range(p)]); ybar = np.bincount(row, P.y, D.n) / F
W = np.column_stack([np.bincount(row, (Z[:, j] - zbar[row, j]) * (P.y - ybar[row]), D.n) for j in range(p)])
out = {}
for c, name in enumerate(('worker', 'firm', 'cov', 'sum')):
    J = -2 * (Zc.T @ g4[c]); lj = Lam @ J
    s0 = 2 * g4[c] * e * sr; s1 = (2 * g4[c] + Ztc @ lj) * e * sr + W @ lj
    m0 = 4 * (Vx * g4[c] ** 2).sum(); m1 = 4 * (Vx * (g4[c] + 0.5 * Ztc @ lj) ** 2).sum()
    out[name] = dict(score_ratio=float((s1 ** 2).sum() / (s0 ** 2).sum()), model_ratio=float(m1 / m0), model_signal_without=float(m0),
                     J=J.tolist(), J_norm=float(np.sqrt(J @ Lam @ J)), within_share=float(((W @ lj) ** 2).sum() / (s1 ** 2).sum()))
res['components'] = out; res['controls'] = p; res['seconds'] = time.time() - t0
tmp = odir + '/controls_year.json.tmp'; json.dump(res, open(tmp, 'w'), indent=1); os.replace(tmp, odir + '/controls_year.json'); print(json.dumps(res, indent=1))

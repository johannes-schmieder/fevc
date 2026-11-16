"""Size of the first-order term for estimated controls on the six-year Veneto panel (SCC; aggregates only).
Controls as in the package run on this panel: year effects and ((age-40)/40)^2, ^3, removed in a first step.
Usage: lp_controls.py <pooled-csv> <lcs-csv> <design-dir> <out-dir> [probes]
For each component reports the leading (signal) term with and without the control term, in the score form of
the package (docs/standard_errors, section Controls) and in the model form 4 sum V (m*)^2 against 4 sum V m^2."""
import sys, os, json, time, numpy as np, pandas as pd
from lp import Panel, build_design, aggregate, LP, T34
pooled, lcs, ddir, odir = sys.argv[1:5]; probes = int(sys.argv[5]) if len(sys.argv) > 5 else 200
os.makedirs(odir, exist_ok=True); t0 = time.time(); res = {}
a = pd.read_csv(pooled, usecols=['year', 'worker', 'firm', 'y_adjusted'])
b = pd.read_csv(lcs, usecols=['worker', 'firm', 'year', 'y', 'age'])
m = a.merge(b, on=['worker', 'firm', 'year'], how='left', validate='one_to_one')
res['merge'] = dict(pooled_rows=int(len(a)), matched=int(m['age'].notna().sum()))
print(res, flush=True)
if res['merge']['matched'] != len(a):
    json.dump(res, open(odir + '/controls.json', 'w'), indent=1); raise SystemExit('merge incomplete; stopping')
P = Panel(m['worker'].to_numpy(), m['firm'].to_numpy(), m['year'].to_numpy(), m['y_adjusted'].to_numpy(float))
years = np.sort(np.unique(P.year)); ag = (m['age'].to_numpy(float) - 40) / 40
Z = np.column_stack([(P.year == yv).astype(float) for yv in years[1:]] + [ag ** 2, ag ** 3]); p = Z.shape[1]
D, info = build_design(P)
dfile = ddir + f'/diag_default_{probes}.npz'
if os.path.exists(dfile):
    dz = np.load(dfile); E = LP(D, exact_diag=(dz['Pd'], dz['Bd']))
else:
    E = LP(D, probes=probes); np.savez(dfile, Pd=E.A.Pd, Bd=E.A.Bd)
# residualise the controls on worker and firm effects at the person-year level (same normal equations)
rw = np.column_stack([np.bincount(P.w, Z[:, j], P.nW) for j in range(p)]); rf = np.column_stack([np.bincount(P.f, Z[:, j], P.nJ) for j in range(p)])
aw, pf = D.solve(rw, rf); Zt = Z - (aw[P.w] + pf[P.f])
G = Zt.T @ Zt; Lam = np.linalg.inv(G)
res['normal_equation_check'] = float(np.abs(np.column_stack([np.bincount(P.w, Zt[:, j], P.nW) for j in range(p)])).max())
# consistency: y - y_adjusted should be Z gamma-hat plus a constant
dif = m['y'].to_numpy(float) - P.y; X1 = np.column_stack([np.ones(P.n), Z]); coef = np.linalg.lstsq(X1.T @ X1, X1.T @ dif, rcond=None)[0]
res['offset_check'] = dict(r2=float(1 - ((dif - X1 @ coef) ** 2).sum() / ((dif - dif.mean()) ** 2).sum()), gamma=coef[1:].tolist())
gam_ols = Lam @ (Zt.T @ m['y'].to_numpy(float)); res['gamma_joint'] = gam_ols.tolist()
# fit on the adjusted wage
y = aggregate(info, D.n, P.y); f = E.fit(y); g = f['g']; g4 = T34 @ g; e = f['e']; sr = np.sqrt(E.r)
ss = E.stayer_stats(y, g); Vx = E.stayer_var(f['va'], ss)
Zc = np.column_stack([aggregate(info, D.n, Z[:, j]) for j in range(p)]); Ztc = np.column_stack([aggregate(info, D.n, Zt[:, j]) for j in range(p)])
# within-row term: sum over the row's person-years of (z - zbar)(y* - ybar*)
row = info['py_row']; F = D.F
zbar = np.column_stack([np.bincount(row, Z[:, j], D.n) / F for j in range(p)]); ybar = np.bincount(row, P.y, D.n) / F
W = np.column_stack([np.bincount(row, (Z[:, j] - zbar[row, j]) * (P.y - ybar[row]), D.n) for j in range(p)])
out = {}
for c, name in enumerate(('worker', 'firm', 'cov', 'sum')):
    J = -2 * (Zc.T @ g4[c]); lj = Lam @ J
    s0 = 2 * g4[c] * e * sr; s1 = (2 * g4[c] + Ztc @ lj) * e * sr + W @ lj
    m0 = 4 * (Vx * g4[c] ** 2).sum(); m1 = 4 * (Vx * (g4[c] + 0.5 * Ztc @ lj) ** 2).sum()
    out[name] = dict(score_signal_without=float((s0 ** 2).sum()), score_signal_with=float((s1 ** 2).sum()), score_ratio=float((s1 ** 2).sum() / (s0 ** 2).sum()),
                     model_signal_without=float(m0), model_signal_with=float(m1), model_ratio=float(m1 / m0),
                     J_norm=float(np.sqrt(J @ Lam @ J)), within_share=float(((W @ lj) ** 2).sum() / (s1 ** 2).sum()))
res['components'] = out; res['controls'] = p; res['seconds'] = time.time() - t0
tmp = odir + '/controls.json.tmp'; json.dump(res, open(tmp, 'w'), indent=1); os.replace(tmp, odir + '/controls.json'); print(json.dumps(res, indent=1))

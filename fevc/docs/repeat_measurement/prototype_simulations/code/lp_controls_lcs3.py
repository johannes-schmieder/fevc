"""Size of the first-order term for estimated controls (year effects and ((age-40)/40)^2, ^3), measured on the
six-year file that carries age (SCC; aggregates only).  The file is the largest connected set; it is pruned here
to a leave-one-match-out connected sample (movers with >= 2 matches; stayers at retained firms), the controls are
estimated jointly with worker and firm effects on that sample, and the term is evaluated as in lp_controls.py.
Usage: lp_controls_lcs.py <lcs-csv> <out-dir> [probes]"""
import sys, os, json, time, numpy as np, pandas as pd
from lp6 import Panel, build_design, aggregate, LP, T34
from akm_design import prune
src, odir = sys.argv[1], sys.argv[2]; probes = int(sys.argv[3]) if len(sys.argv) > 3 else 200; names = sys.argv[4:] or ['twl']
os.makedirs(odir, exist_ok=True); t0 = time.time(); res = {}
b = pd.read_csv(src, usecols=['worker', 'firm', 'year', 'y', 'age'])
w, f, yr, y, age = (b[c].to_numpy() for c in ('worker', 'firm', 'year', 'y', 'age')); y = y.astype(float); age = age.astype(float)
for it in range(10):                                           # leave-one-match-out pruning, as in the simulation's synthetic panels
    P = Panel(w, f, yr); st_py = P.stay_w[P.w]; mv = ~P.stay_w[P.match_w]
    Fm = np.bincount(P.match, minlength=len(P.match_w)).astype(float)
    rw = np.r_[P.match_w[mv], P.w[st_py]]; rf = np.r_[P.match_f[mv], P.f[st_py]]
    rF = np.r_[Fm[mv], np.ones(st_py.sum())]; rs = np.r_[np.zeros(mv.sum(), bool), np.ones(st_py.sum(), bool)]
    _, _, _, _, keep, _ = prune(rw, rf, rF, rs)
    keep_match = np.zeros(len(P.match_w), bool); keep_match[np.flatnonzero(mv)[keep[:mv.sum()]]] = True
    keep_py = np.where(st_py, False, keep_match[P.match]); keep_py[np.flatnonzero(st_py)[keep[mv.sum():]]] = True
    cnt = np.bincount(P.w[keep_py], minlength=P.nW); keep_py &= cnt[P.w] >= 2
    print(f'prune round {it}: keep {keep_py.sum()} of {len(keep_py)} ({time.time() - t0:.0f}s)', flush=True)
    if keep_py.all(): break
    w, f, yr, y, age = w[keep_py], f[keep_py], yr[keep_py], y[keep_py], age[keep_py]
P = Panel(w, f, yr, y)
res['sample'] = dict(person_years=int(P.n), workers=int(P.nW), firms=int(P.nJ), stayers=int(P.stay_w.sum()))
years = np.sort(np.unique(P.year)); ag = (age - 40) / 40
Z = np.column_stack([(P.year == yv).astype(float) for yv in years[:-1]] + [ag ** 2, ag ** 3]); p = Z.shape[1]
D, info = build_design(P)
rw_ = np.column_stack([np.bincount(P.w, Z[:, j], P.nW) for j in range(p)]); rf_ = np.column_stack([np.bincount(P.f, Z[:, j], P.nJ) for j in range(p)])
aw, pf = D.solve(rw_, rf_); Zt = Z - (aw[P.w] + pf[P.f]); G = Zt.T @ Zt; Lam = np.linalg.inv(G)
gam = Lam @ (Zt.T @ P.y); ystar = P.y - Z @ gam
res['gamma'] = gam.tolist(); res['normal_equation_check'] = float(np.abs(np.column_stack([np.bincount(P.w, Zt[:, j], P.nW) for j in range(p)])).max())
E = LP(D, probes=probes)
yrow = aggregate(info, D.n, ystar); fit = E.fit(yrow); g = fit['g']; g4 = T34 @ g; e = fit['e']; sr = np.sqrt(E.r)
ss = E.stayer_stats(yrow, g); Vx = E.stayer_var(fit['va'], ss)
mods = E.round6_models(fit, ss); res['models'] = E.last_cells; oo = E.estimate(yrow, draws=16, dseed=1, which=('final', 'cells', 'only_round6'))
Vts = {k: E.stayer_var(mods[k], ss) for k in names}; V4s = {k: np.diag(T34 @ oo[k] @ T34.T) for k in names}; res['V'] = {k: v.tolist() for k, v in V4s.items()}
res['theta'] = fit['theta'].tolist()
Zc = np.column_stack([aggregate(info, D.n, Z[:, j]) for j in range(p)]); Ztc = np.column_stack([aggregate(info, D.n, Zt[:, j]) for j in range(p)])
row = info['py_row']; F = D.F
zbar = np.column_stack([np.bincount(row, Z[:, j], D.n) / F for j in range(p)]); ybar = np.bincount(row, ystar, D.n) / F
W = np.column_stack([np.bincount(row, (Z[:, j] - zbar[row, j]) * (ystar - ybar[row]), D.n) for j in range(p)])
out = {}
for sel, lab in ((slice(None), 'all'), (slice(0, p - 2), 'year_only'), (slice(p - 2, p), 'age_only')):
    for c, name in enumerate(('worker', 'firm', 'cov', 'sum')):
        J = -2 * (Zc[:, sel].T @ g4[c]); Ls = np.linalg.inv(G[sel, sel]); lj = Ls @ J
        s0 = 2 * g4[c] * e * sr; s1 = (2 * g4[c] + Ztc[:, sel] @ lj) * e * sr + W[:, sel] @ lj
        m0 = 4 * (Vx * g4[c] ** 2).sum(); m1 = 4 * (Vx * (g4[c] + 0.5 * Ztc[:, sel] @ lj) ** 2).sum()
        out[f'{lab}:{name}'] = dict(score_ratio=float((s1 ** 2).sum() / (s0 ** 2).sum()), model_ratio=float(m1 / m0))
        for k in names:
            t0_ = 4 * (Vts[k] * g4[c] ** 2).sum(); t1_ = 4 * (Vts[k] * (g4[c] + 0.5 * Ztc[:, sel] @ lj) ** 2).sum()
            out[f'{lab}:{name}'][k + '_se_ratio'] = float(np.sqrt(1 + (t1_ - t0_) / V4s[k][c]))
res['components'] = out; res['seconds'] = time.time() - t0
tmp = odir + '/controls_lcs3.json.tmp'; json.dump(res, open(tmp, 'w'), indent=1); os.replace(tmp, odir + '/controls_lcs3.json'); print(json.dumps(res, indent=1))

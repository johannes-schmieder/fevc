"""One-shot analysis of the six-year Veneto panel (SCC; aggregates only are written).
Usage: lp_real.py <design-dir> <out-dir> [probes]
Outputs real.json: point estimates, standard errors (shipped, model, proposed), shares of the leading term by
source, and diagnostics of stayer errors (gap curve, within-worker skewness and kurtosis, adjustment sizes)."""
import sys, json, time, os, numpy as np
from lp import Panel, build_design, aggregate, LP, T34
ddir, odir = sys.argv[1], sys.argv[2]; probes = int(sys.argv[3]) if len(sys.argv) > 3 else 200
os.makedirs(odir, exist_ok=True); t0 = time.time()
z = np.load(ddir + '/panel.npz'); P = Panel(z['w'], z['f'], z['year'], z['y'])
D, info = build_design(P); y = aggregate(info, D.n, P.y)
res = dict(rows=int(D.n), mover_rows=int((~D.stay).sum()), stayer_rows=int(D.stay.sum()), person_years=int(P.n), firms=int(D.nJ), workers=int(D.nW))
print(res, flush=True)
dfile = ddir + f'/diag_default_{probes}.npz'
if os.path.exists(dfile):
    dz = np.load(dfile); E = LP(D, exact_diag=(dz['Pd'], dz['Bd']), verbose=True)
else:
    E = LP(D, probes=probes, verbose=True); np.savez(dfile, Pd=E.A.Pd, Bd=E.A.Bd)
res['setup_seconds'] = time.time() - t0; res['pass1_solves'] = int(E.solves_pass1)
se = lambda V: np.sqrt(np.maximum(np.diag(T34 @ V @ T34.T), 0)).tolist()
runs = []
for ds in (1, 2):
    t1 = time.time(); o = E.estimate(y, draws=16, dseed=ds, kappas=(1.8, 3.0, 6.0, 18.4))
    runs.append({k: se(v) for k, v in o.items() if isinstance(v, np.ndarray) and v.shape == (3, 3)})
    runs[-1]['seconds'] = time.time() - t1; print('run', ds, runs[-1], flush=True)
res['theta'] = o['theta'].tolist(); res['se_runs'] = runs
f = o['fit']; ss = o['ss']; g = f['g']; g4 = T34 @ g
# plug-in components for reference
res['plugin'] = (T34 @ D.theta(f['alpha'], f['psi'])).tolist()
# shares of the leading term by source (proposed variances)
Vx = E.stayer_var(f['va'], ss); R_row = np.zeros(D.n, int); R_row[E.st_rows] = E.R[E.st_s].astype(int)
lead = Vx * g4 ** 2; tot = lead.sum(1)
src = {'movers': ~D.stay}
for r in range(2, 7): src[f'stayers_{r}y'] = R_row == r
res['share_leading'] = {k: (lead[:, m].sum(1) / tot).tolist() for k, m in src.items()}
res['share_rows'] = {k: float(m.mean()) for k, m in src.items()}
res['share_person_years'] = {k: float(D.F[m].sum() / D.F.sum()) for k, m in src.items()}
# adjustment pieces relative to the proposed variance (kappa = 3, raw)
tau_free = None
raw_noadj = np.array(runs[0]['prop_noadj']) ** 2
adj3 = np.diag(T34 @ E.adjust(ss, 3.0) @ T34.T); adjf = np.diag(T34 @ E.adjust(ss, 3.0, free=True) @ T34.T)
res['adjust_over_var'] = dict(kappa3=(adj3 / raw_noadj).tolist(), free=(adjf / raw_noadj).tolist())
# within-worker shape of stayer errors from k-statistics (R >= 4)
R = E.R; q = ss['q']; k3 = ss['k3']; k4 = ss['k4']; shape = {}
for lab, m in (('R>=4', R >= 4), ('R=4', R == 4), ('R=5', R == 5), ('R=6', R == 6)):
    s4 = (R[m] - 1) / (R[m] + 1) * (q[m] ** 2 - k4[m] / R[m])
    shape[lab] = dict(stayers=int(m.sum()), mean_var=float(q[m].mean()), kurtosis=float(3 + k4[m].sum() / s4.sum()),
                      skew_index=float(k3[m].mean() / q[m].mean() ** 1.5), cv_var=float(np.sqrt(max(s4.mean() / q[m].mean() ** 2 - 1, 0))))
res['stayer_shape'] = shape
res['stayer_var_by_years'] = {int(r): float(q[R == r].mean()) for r in range(2, 7)}
mT = g4[3]; mbarT = np.bincount(E.st_s, mT[E.st_rows], len(E.sw)) / R
hi = R >= 3
res['skew_vs_level'] = dict(corr_k3_mbar=float(np.corrcoef(k3[hi], mbarT[hi])[0, 1]),
                            mean_k3_by_mbar_quintile=[float(k3[hi][(mbarT[hi] >= a) & (mbarT[hi] <= b)].mean()) for a, b in zip(np.quantile(mbarT[hi], [0, .2, .4, .6, .8]), np.quantile(mbarT[hi], [.2, .4, .6, .8, 1]))],
                            mean_q_by_mbar_quintile=[float(q[(mbarT >= a) & (mbarT <= b)].mean()) for a, b in zip(np.quantile(mbarT, [0, .2, .4, .6, .8]), np.quantile(mbarT, [.2, .4, .6, .8, 1]))])
# gap curve: half mean squared difference between a worker's adjusted wages k years apart (same firm)
def gap_curve(mask_py):
    idx = np.flatnonzero(mask_py); o_ = idx[np.lexsort((P.year[idx], P.match[idx]))]
    mt, yr, yy = P.match[o_], P.year[o_].astype(int), P.y[o_]; out = {}
    for k in range(1, 6):
        num = 0.0; cnt = 0
        for j in range(1, 6):
            same = (mt[j:] == mt[:-j]) & (yr[j:] - yr[:-j] == k)
            num += (0.5 * (yy[j:] - yy[:-j]) ** 2)[same].sum(); cnt += int(same.sum())
        out[k] = dict(value=num / max(cnt, 1), pairs=cnt)
    # autocorrelation of first differences at lag 1 (consecutive years)
    d1 = (mt[1:] == mt[:-1]) & (yr[1:] - yr[:-1] == 1); dy = np.where(d1, yy[1:] - yy[:-1], np.nan)
    ok = ~np.isnan(dy[1:]) & ~np.isnan(dy[:-1]) & (mt[2:] == mt[:-2])
    a, b = dy[1:][ok], dy[:-1][ok]
    out['diff_autocorr'] = float(np.mean(a * b) / (0.5 * (np.mean(a * a) + np.mean(b * b)))); out['diff_pairs'] = int(ok.sum())
    return out
st_py = P.stay_w[P.w]; Tpy = P.T_w[P.w]
mlen = np.bincount(P.match)[P.match]
res['gap_curve'] = dict(stayers_all=gap_curve(st_py), stayers_6y=gap_curve(st_py & (Tpy == 6)), stayers_4y=gap_curve(st_py & (Tpy == 4)),
                        movers_within_match=gap_curve(~st_py & (mlen >= 2)))
res['total_seconds'] = time.time() - t0
tmp = odir + '/real.json.tmp'; json.dump(res, open(tmp, 'w'), indent=1); os.replace(tmp, odir + '/real.json')
print(json.dumps({k: v for k, v in res.items() if k not in ('se_runs',)}, indent=1))

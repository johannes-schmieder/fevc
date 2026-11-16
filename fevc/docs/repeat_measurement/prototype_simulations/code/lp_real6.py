"""Six-year Veneto panel with the round-6 movers' models (tw, twl, dq): standard errors, fitted variances of movers'
rows by decile of the estimated person effect, shares of the first term.  Aggregates only.
Usage: lp_real6.py <design-dir> <out-dir> <probes>"""
import sys, json, time, os, numpy as np
from lp6 import Panel, build_design, aggregate, LP, T34
ddir, odir, probes = sys.argv[1], sys.argv[2], int(sys.argv[3]); os.makedirs(odir, exist_ok=True); t0 = time.time()
z = np.load(ddir + '/panel.npz'); P = Panel(z['w'], z['f'], z['year'], z['y']); D, info = build_design(P); y = aggregate(info, D.n, P.y)
dz = np.load(ddir + f'/diag_default_{probes}.npz'); E = LP(D, exact_diag=(dz['Pd'], dz['Bd'])); res = dict(probes=probes, runs=[])
names = ('tw', 'twl', 'tlw', 'dq')
for ds, draws in ((1, 16), (2, 16), (3, 64)):
    t1 = time.time(); s0 = E.A.solves; o = E.estimate(y, draws=draws, dseed=ds, which=('final', 'cells', 'only_round6'))
    res['runs'].append(dict(draws=draws, seconds=time.time() - t1, solves=int(E.A.solves - s0), **{k: np.sqrt(np.maximum(np.diag(T34 @ o[k] @ T34.T), 0)).tolist() for k in names}))
    print(res['runs'][-1], flush=True)
res['theta'] = o['theta'].tolist(); f = o['fit']; ss = o['ss']; g4 = T34 @ f['g']; res['models'] = o['diag_cells']
mods = E.round6_models(f, ss); at, pt = E.std_effects(f); mv = np.flatnonzero(~D.stay); one = np.ones(len(mv)); w_st = E.sw
cut = np.quantile(at[w_st], np.arange(1, 10) / 10); dr = np.searchsorted(cut, at[D.w[mv]])
def by(dec, val, wt): return (np.bincount(dec, val * wt, 10) / np.bincount(dec, wt, 10)).tolist()
res['leave_out_by_decile'] = by(dr, f['s'][mv], one); res['rows_by_decile'] = np.bincount(dr, minlength=10).tolist()
res['share_capped'] = dict(person=float((np.abs(at[D.w[mv]]) >= 3.0).mean()), firm=float((np.abs(pt[D.f[mv]]) >= 3.0).mean()),
                           stayers_person=float((np.abs(at[w_st]) >= 3.0).mean()))
V4 = {k: np.array(res['runs'][-1][k]) ** 2 for k in names}
for k in names:
    Vx = E.stayer_var(mods[k], ss); lead = Vx * g4 ** 2
    res[k] = dict(fitted_by_decile=by(dr, mods[k][mv], one), fitted_mean=float(mods[k][mv].mean()),
                  mover_share_first_term=(lead[:, mv].sum(1) / lead.sum(1)).tolist(), first_term_over_V=(4 * lead.sum(1) / V4[k]).tolist(),
                  first_term_by_decile_sum=[float(lead[3, mv][dr == d].sum() / lead[3].sum()) for d in range(10)],
                  first_term_capped_rows=(lead[:, mv][:, np.abs(at[D.w[mv]]) >= 3.0].sum(1) / lead.sum(1)).tolist(),
                  adjust_over_V=(np.diag(T34 @ E.adjust(ss, 3.0) @ T34.T) / V4[k]).tolist())
res['seconds'] = time.time() - t0
tmp = odir + '/real6.json.tmp'; json.dump(res, open(tmp, 'w'), indent=1); os.replace(tmp, odir + '/real6.json'); print(json.dumps(res)[:2500])

"""Six-year Veneto panel: the transfer model (tr) next to the variant in which movers load on the stayers' profile
(tl); standard errors and the fitted variances of movers' rows by decile of the estimated person effect.
Aggregates only.  Usage: lp_real5.py <design-dir> <out-dir> <probes>"""
import sys, json, time, os, numpy as np
from lp5 import Panel, build_design, aggregate, LP, T34
ddir, odir, probes = sys.argv[1], sys.argv[2], int(sys.argv[3]); os.makedirs(odir, exist_ok=True); t0 = time.time()
z = np.load(ddir + '/panel.npz'); P = Panel(z['w'], z['f'], z['year'], z['y']); D, info = build_design(P); y = aggregate(info, D.n, P.y)
dz = np.load(ddir + f'/diag_default_{probes}.npz'); E = LP(D, exact_diag=(dz['Pd'], dz['Bd'])); res = dict(probes=probes, runs=[])
for ds in (1, 2):
    o = E.estimate(y, draws=16, dseed=ds, which=('final', 'load'))
    res['runs'].append({k: np.sqrt(np.maximum(np.diag(T34 @ o[k] @ T34.T), 0)).tolist() for k in ('tr', 'tl')}); print(res['runs'][-1], flush=True)
res['theta'] = o['theta'].tolist(); f = o['fit']; ss = o['ss']; g4 = T34 @ f['g']; a = f['alpha']
mv = np.flatnonzero(~D.stay); w_st = E.sw; one = np.ones(len(mv))
def by(dec, val, wt): return (np.bincount(dec, val * wt, 10) / np.bincount(dec, wt, 10)).tolist()
for name, load in (('tr', False), ('tl', True)):
    vt = E.working_transfer(f, ss, load=load); ti = dict(E.last_transfer); ma, sa, mp, sp = ti['std']
    cut = np.quantile((a[w_st] - ma) / sa, np.arange(1, 10) / 10); dr = np.searchsorted(cut, (a[D.w[mv]] - ma) / sa)
    Vx = E.stayer_var(vt, ss); lead = Vx * g4 ** 2
    res[name] = dict(transfer=ti, fitted_by_decile=by(dr, vt[mv], one), leave_out_by_decile=by(dr, f['s'][mv], one), fitted_mean=float(vt[mv].mean()),
                     first_term_by_decile=[(lead[:, mv][:, dr == d].sum(1) / lead.sum(1)).tolist() for d in range(10)],
                     mover_share_first_term=(lead[:, mv].sum(1) / lead.sum(1)).tolist())
res['seconds'] = time.time() - t0
tmp = odir + '/real5.json.tmp'; json.dump(res, open(tmp, 'w'), indent=1); os.replace(tmp, odir + '/real5.json'); print(json.dumps(res)[:3000])

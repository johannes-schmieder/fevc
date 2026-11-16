"""What do the decile averages of movers' leave-out variance estimates look like when the truth is known?
For chosen scenarios of a manifest, draws a few data sets on the fixed design and records, by decile of the
ESTIMATED standardised person effect (cut points from stayers): stayers' sample variance and fitted profile;
movers' rows: leave-out estimate, transfer-model fitted value and true variance.  Aggregates only.
Usage: lp_check.py <design-dir> <out-dir> <manifest> <reps> <scenario> [<scenario> ...]"""
import sys, os, json, time, numpy as np
from lp import aggregate, T34
import lp_mc
ddir, odir, mfile, reps = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4]); names = sys.argv[5:]
os.makedirs(odir, exist_ok=True); t0 = time.time(); man = json.load(open(mfile))
P = lp_mc.load_panel(ddir); D, info, E = lp_mc.engine(ddir, P, 'default', man.get('probes', 200))
tr = np.load(ddir + '/truth.npz'); mu = D.X(tr['alpha'], tr['psi'])
_, g = E.A.kernel(mu); gT = T34[3] @ np.array(g)
mbarT_w = np.bincount(D.w, gT * D.F, D.nW) / np.bincount(D.w, D.F, D.nW)
basis = lambda av, pv: np.column_stack([np.ones(len(av)), av, av ** 2, pv, pv ** 2, av * pv])
mv = np.flatnonzero(~D.stay); w_st = E.sw; f_st = np.zeros(len(w_st), np.int64); f_st[E.st_s] = D.f[E.st_rows]; df = E.R - 1.0
def by(dec, val, wt): return np.bincount(dec, val * wt, 10) / np.bincount(dec, wt, 10)
res = dict(reps=reps, probes=man.get('probes', 200), scenarios={})
for name in names:
    sc = dict(man['scenarios'][name]); v_py, sign_py = lp_mc.scenario_inputs(P, sc, tr, mbarT_w)
    v_rows = aggregate(info, D.n, v_py * info['py_w']); acc = {}
    for r in range(reps):
        rng = np.random.default_rng([sc['seed'], 3, r])
        y = mu + aggregate(info, D.n, lp_mc.draw_errors(P, sc, v_py, sign_py, None, rng))
        o = E.fit(y); ss = E.stayer_stats(y, o['g']); vt = E.working_transfer(o, ss); ti = E.last_transfer
        ma, sa, mp, sp = ti['std']; coef = np.array(ti['coef']); a, p = o['alpha'], o['psi']
        as_, ps_ = (a[w_st] - ma) / sa, (p[f_st] - mp) / sp; cut = np.quantile(as_, np.arange(1, 10) / 10)
        ds = np.searchsorted(cut, as_); ar, pr = (a[D.w[mv]] - ma) / sa, (p[D.f[mv]] - mp) / sp; dr = np.searchsorted(cut, ar); one = np.ones(len(mv))
        cur = dict(stayers_q=by(ds, ss['q'], df), stayers_fitted=by(ds, basis(as_, ps_) @ coef, df), stayers_true=by(ds, v_rows[E.st_rows][np.r_[True, D.w[E.st_rows][1:] != D.w[E.st_rows][:-1]]], df),
                   movers_leave_out=by(dr, o['s'][mv], one), movers_fitted=by(dr, vt[mv], one), movers_true=by(dr, v_rows[mv], one),
                   movers_rows=np.bincount(dr, minlength=10).astype(float), movers_mean_index=by(dr, ar, one),
                   overall=np.array([o['s'][mv].mean(), vt[mv].mean(), v_rows[mv].mean(), ti['profile_max'], ti['r2']]))
        for k, x in cur.items(): acc.setdefault(k, []).append(np.asarray(x, float))
        print(name, 'rep', r, f'{time.time() - t0:.0f}s', flush=True)
    res['scenarios'][name] = {k: np.mean(x, 0).tolist() for k, x in acc.items()}
    res['scenarios'][name]['sd_movers_leave_out'] = np.std(acc['movers_leave_out'], 0).tolist()
res['seconds'] = time.time() - t0
tmp = odir + '/check.json.tmp'; json.dump(res, open(tmp, 'w'), indent=1); os.replace(tmp, odir + '/check.json')
for name, v in res['scenarios'].items():
    print(name); [print('  ', k, [round(x, 4) for x in v[k]]) for k in ('stayers_q', 'stayers_fitted', 'movers_leave_out', 'movers_fitted', 'movers_true', 'overall')]

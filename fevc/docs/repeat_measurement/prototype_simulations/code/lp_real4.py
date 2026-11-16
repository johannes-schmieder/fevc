"""Six-year Veneto panel: the numbers reported in the note (SCC; aggregates only are written).
Usage: lp_real4.py <design-dir> <out-dir> <probes>
Writes real4.json: point estimates; standard errors with the transfer model (three runs of the simulated double
sum); the fitted transfer model; variance profiles by decile of the estimated effects for stayers (sample variance
of the stayer's years) and for movers (within-match sample variance, leave-out products, fitted values); shares
of the first term by source; and the concentration diagnostics."""
import sys, json, time, os, numpy as np
from lp import Panel, build_design, aggregate, LP, T34
ddir, odir, probes = sys.argv[1], sys.argv[2], int(sys.argv[3])
os.makedirs(odir, exist_ok=True); t0 = time.time()
z = np.load(ddir + '/panel.npz'); P = Panel(z['w'], z['f'], z['year'], z['y'])
D, info = build_design(P); y = aggregate(info, D.n, P.y)
res = dict(rows=int(D.n), mover_rows=int((~D.stay).sum()), stayer_rows=int(D.stay.sum()), person_years=int(P.n), firms=int(D.nJ),
           workers=int(D.nW), stayers=int(P.stay_w.sum()), movers=int((~P.stay_w).sum()), probes=probes)
dz = np.load(ddir + f'/diag_default_{probes}.npz'); E = LP(D, exact_diag=(dz['Pd'], dz['Bd']))
res['stayers_by_years'] = {int(r): int((E.R == r).sum()) for r in np.unique(E.R)}
Fm = D.F[~D.stay]; res['mover_rows_by_length'] = {int(r): int((Fm == r).sum()) for r in np.unique(Fm)}
res['leverage_movers'] = dict(mean=float(E.A.Pd[~D.stay].mean()), q50=float(np.median(E.A.Pd[~D.stay])), q99=float(np.quantile(E.A.Pd[~D.stay], .99)), max=float(E.A.Pd[~D.stay].max()))
runs = []
for ds, draws in ((1, 16), (2, 16), (3, 64)):
    t1 = time.time(); s0 = E.A.solves; o = E.estimate(y, draws=draws, dseed=ds, which=('final',))
    V4 = np.diag(T34 @ o['tr'] @ T34.T)
    runs.append(dict(draws=draws, seed=ds, V=V4.tolist(), se=np.sqrt(np.maximum(V4, 0)).tolist(), seconds=time.time() - t1, solves=int(E.A.solves - s0)))
    print('run', runs[-1], flush=True)
res['theta'] = o['theta'].tolist(); res['runs'] = runs; res['transfer'] = o['diag_tr']
f = o['fit']; ss = o['ss']; g = f['g']; g4 = T34 @ g; a, p = f['alpha'], f['psi']
res['plugin'] = (T34 @ D.theta(a, p)).tolist()
vt = E.working_transfer(f, ss); Vx = E.stayer_var(vt, ss); V4 = np.array(runs[-1]['V'])
# first term by source, adjustment
R_row = np.zeros(D.n, int); R_row[E.st_rows] = E.R[E.st_s].astype(int); lead = Vx * g4 ** 2; tot = lead.sum(1)
src = {'movers': ~D.stay}
for r in range(2, 7): src[f'stayers_{r}y'] = R_row == r
res['share_first_term'] = {k: (lead[:, m].sum(1) / tot).tolist() for k, m in src.items()}
res['share_person_years'] = {k: float(D.F[m].sum() / D.F.sum()) for k, m in src.items()}
res['first_term_over_V'] = (4 * tot / V4).tolist()
res['adjust_over_V'] = (np.diag(T34 @ E.adjust(ss, 3.0) @ T34.T) / V4).tolist()
# ---- variance profiles by decile of the standardised estimated effects (cut points from stayers)
ma, sa, mp, sp = res['transfer']['std']; coef = np.array(res['transfer']['coef'])
basis = lambda av, pv: np.column_stack([np.ones(len(av)), av, av ** 2, pv, pv ** 2, av * pv])
w_st = E.sw; f_st = np.zeros(len(w_st), np.int64); f_st[E.st_s] = D.f[E.st_rows]
as_, ps_ = (a[w_st] - ma) / sa, (p[f_st] - mp) / sp; q = ss['q']; df = E.R - 1.0
cut_a = np.quantile(as_, np.arange(1, 10) / 10); cut_p = np.quantile(ps_, np.arange(1, 10) / 10)
def by(dec, val, wt):
    return (np.bincount(dec, val * wt, 10) / np.bincount(dec, wt, 10)).tolist()
prof = {}
for lab, x, cut in (('alpha', as_, cut_a), ('psi', ps_, cut_p)):
    d = np.searchsorted(cut, x)
    prof['stayers_by_' + lab] = dict(cuts=cut.tolist(), n=np.bincount(d, minlength=10).tolist(), mean_index=by(d, x, df), sample_variance=by(d, q, df),
                                     fitted_profile=by(d, basis(as_, ps_) @ coef, df))
# movers: within-match sample variance of the person-years (matches of two or more years)
mv_py = ~P.stay_w[P.w]; nm = len(P.match_w); Fmat = np.bincount(P.match, minlength=nm).astype(float)
mean_m = np.bincount(P.match, P.y, nm) / Fmat; dev2 = np.bincount(P.match, (P.y - mean_m[P.match]) ** 2, nm)
sel = (~P.stay_w[P.match_w]) & (Fmat >= 2); dfm = Fmat[sel] - 1; s2m = dev2[sel] / dfm
am, pm = (a[P.match_w[sel]] - ma) / sa, (p[P.match_f[sel]] - mp) / sp
mv = np.flatnonzero(~D.stay); ar, pr = (a[D.w[mv]] - ma) / sa, (p[D.f[mv]] - mp) / sp
for lab, xm, xr, cut in (('alpha', am, ar, cut_a), ('psi', pm, pr, cut_p)):
    d = np.searchsorted(cut, xm); dr = np.searchsorted(cut, xr); one = np.ones(len(mv))
    prof['movers_by_' + lab] = dict(n_matches=np.bincount(d, minlength=10).tolist(), within_match_variance=by(d, s2m, dfm),
                                    mean_index_matches=by(d, xm, dfm), mean_index_rows=by(dr, xr, one),
                                    n_rows=np.bincount(dr, minlength=10).tolist(), leave_out_product=by(dr, f['s'][mv], one),
                                    fitted=by(dr, vt[mv], one), profile=by(dr, basis(ar, pr) @ coef, one))
res['within_match_overall'] = dict(matches=int(sel.sum()), mean=float((s2m * dfm).sum() / dfm.sum()), stayers_mean=float((q * df).sum() / df.sum()))
res['profiles'] = prof
res['movers_by_length'] = {int(r): dict(rows=int((Fm == r).sum()), leave_out_product=float(f['s'][mv][Fm == r].mean()), fitted=float(vt[mv][Fm == r].mean())) for r in np.unique(Fm)}
res['stayer_variance_by_years'] = {int(r): float(q[E.R == r].mean()) for r in np.unique(E.R)}
# ---- concentration diagnostics with the transfer variances
sv = np.sqrt(np.maximum(Vx, 1e-300)); rng = np.random.default_rng(11); rown = np.zeros((4, D.n)); U = []
for b in range(2):
    zeta = (sv * rng.standard_normal((D.n, 16)).T).T; _, kz = E.A.kernel(zeta); k4 = np.einsum('ct,tnd->cnd', T34, np.array(kz))
    rown += (k4 ** 2).mean(2) / 2; U.append(k4 * sv[None, :, None])
U = np.concatenate(U, axis=2); G = np.array([U[c].T @ U[c] for c in range(4)]); off = ~np.eye(32, dtype=bool)
rowV = Vx * (4 * g4 ** 2 + 2 * rown); st = E.st_rows; per = np.array([np.bincount(E.st_s, rowV[c][st], len(E.sw)) for c in range(4)])
dg = dict(double_sum_over_V=(2 * np.array([np.trace(G[c]) / 32 for c in range(4)]) / V4).tolist(),
          max_stayer_share=(per.max(1) / V4).tolist(), stayer_share_total=(per.sum(1) / V4).tolist(),
          max_mover_row_share=(rowV[:, ~D.stay].max(1) / V4).tolist(),
          D2=(np.sqrt(np.array([(G[c][off] ** 2).mean() for c in range(4)])) / V4).tolist())
def Kmul(c, x):
    _, kk = E.A.kernel(sv * x); return sv * (T34[c] @ np.array(kk))
lam2 = []
for c in range(4):
    x = rng.standard_normal(D.n); x /= np.linalg.norm(x)
    for it in range(20):
        yv = Kmul(c, Kmul(c, x)); l2 = np.linalg.norm(yv); x = yv / l2
    lam2.append(l2)
dg['lambda2_over_V'] = (np.array(lam2) / V4).tolist(); res['diagnostics'] = dg
res['total_seconds'] = time.time() - t0
tmp = odir + '/real4.json.tmp'; json.dump(res, open(tmp, 'w'), indent=1); os.replace(tmp, odir + '/real4.json')
print(json.dumps({k: v for k, v in res.items() if k != 'profiles'}, indent=1)); print(json.dumps(res['profiles'])[:1500])

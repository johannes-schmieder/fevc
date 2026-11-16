"""Six-year Veneto panel: the numbers reported in the note for one of the round-6 movers' models (SCC; aggregates
only).  Usage: lp_real7.py <design-dir> <out-dir> <probes> <model: tw|twl|dq>
Same content as lp_real4.py (estimates, standard errors from three runs of the simulated double sum, variance
profiles by decile, shares of the first term, concentration diagnostics), with the chosen model for movers."""
import sys, json, time, os, numpy as np
from lp6 import Panel, build_design, aggregate, LP, T34
ddir, odir, probes, name = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
os.makedirs(odir, exist_ok=True); t0 = time.time()
z = np.load(ddir + '/panel.npz'); P = Panel(z['w'], z['f'], z['year'], z['y'])
D, info = build_design(P); y = aggregate(info, D.n, P.y)
res = dict(model=name, rows=int(D.n), mover_rows=int((~D.stay).sum()), stayer_rows=int(D.stay.sum()), person_years=int(P.n), firms=int(D.nJ),
           workers=int(D.nW), stayers=int(P.stay_w.sum()), movers=int((~P.stay_w).sum()), probes=probes)
dz = np.load(ddir + f'/diag_default_{probes}.npz'); E = LP(D, exact_diag=(dz['Pd'], dz['Bd']))
runs = []
for ds, draws in ((1, 16), (2, 16), (3, 64)):
    t1 = time.time(); s0 = E.A.solves; o = E.estimate(y, draws=draws, dseed=ds, which=('final', 'cells', 'only_round6'))
    V4 = np.diag(T34 @ o[name] @ T34.T)
    runs.append(dict(draws=draws, seed=ds, V=V4.tolist(), se=np.sqrt(np.maximum(V4, 0)).tolist(), seconds=time.time() - t1, solves=int(E.A.solves - s0)))
    print('run', runs[-1], flush=True)
res['theta'] = o['theta'].tolist(); res['runs'] = runs; res['models'] = o['diag_cells']
f = o['fit']; ss = o['ss']; g = f['g']; g4 = T34 @ g; a, p = f['alpha'], f['psi']
res['plugin'] = (T34 @ D.theta(a, p)).tolist()
vt = E.round6_models(f, ss)[name]; Vx = E.stayer_var(vt, ss); V4 = np.array(runs[-1]['V']); at, pt = E.std_effects(f)
R_row = np.zeros(D.n, int); R_row[E.st_rows] = E.R[E.st_s].astype(int); lead = Vx * g4 ** 2; tot = lead.sum(1)
src = {'movers': ~D.stay}
for r in range(2, 7): src[f'stayers_{r}y'] = R_row == r
res['share_first_term'] = {k: (lead[:, m].sum(1) / tot).tolist() for k, m in src.items()}
res['share_person_years'] = {k: float(D.F[m].sum() / D.F.sum()) for k, m in src.items()}
res['first_term_over_V'] = (4 * tot / V4).tolist()
res['adjust_over_V'] = (np.diag(T34 @ E.adjust(ss, 3.0) @ T34.T) / V4).tolist()
# ---- profiles by decile of the standardised, winsorised estimated effects (cut points from stayers)
basis = lambda av, pv: np.column_stack([np.ones(len(av)), av, av ** 2, pv, pv ** 2, av * pv])
w_st = E.sw; f_st = np.zeros(len(w_st), np.int64); f_st[E.st_s] = D.f[E.st_rows]
as_, ps_ = at[w_st], pt[f_st]; q = ss['q']; df = E.R - 1.0
Bs = basis(as_, ps_); pis = np.linalg.lstsq(Bs.T @ (Bs * df[:, None]), Bs.T @ (df * q), rcond=None)[0]; qbar = float(np.average(q, weights=df))
res['stayer_profile'] = dict(coef_relative=(pis / qbar).tolist(), mean_variance=qbar, r2=float(1 - np.average((q - Bs @ pis) ** 2, weights=df) / np.average((q - qbar) ** 2, weights=df)),
                             share_capped_person=float((np.abs(as_) >= 3).mean()), share_capped_firm=float((np.abs(ps_) >= 3).mean()))
cut_a = np.quantile(as_, np.arange(1, 10) / 10); cut_p = np.quantile(ps_, np.arange(1, 10) / 10)
def by(dec, val, wt): return (np.bincount(dec, val * wt, 10) / np.bincount(dec, wt, 10)).tolist()
prof = {}
for lab, x, cut in (('alpha', as_, cut_a), ('psi', ps_, cut_p)):
    d = np.searchsorted(cut, x)
    prof['stayers_by_' + lab] = dict(cuts=cut.tolist(), n=np.bincount(d, minlength=10).tolist(), mean_index=by(d, x, df), sample_variance=by(d, q, df), fitted_profile=by(d, Bs @ pis, df))
mv_py = ~P.stay_w[P.w]; nm = len(P.match_w); Fmat = np.bincount(P.match, minlength=nm).astype(float)
mean_m = np.bincount(P.match, P.y, nm) / Fmat; dev2 = np.bincount(P.match, (P.y - mean_m[P.match]) ** 2, nm)
sel = (~P.stay_w[P.match_w]) & (Fmat >= 2); dfm = Fmat[sel] - 1; s2m = dev2[sel] / dfm
am, pm = at[P.match_w[sel]], pt[P.match_f[sel]]; mv = np.flatnonzero(~D.stay); ar, pr = at[D.w[mv]], pt[D.f[mv]]; one = np.ones(len(mv))
for lab, xm, xr, cut in (('alpha', am, ar, cut_a), ('psi', pm, pr, cut_p)):
    d = np.searchsorted(cut, xm); dr = np.searchsorted(cut, xr)
    prof['movers_by_' + lab] = dict(n_matches=np.bincount(d, minlength=10).tolist(), within_match_variance=by(d, s2m, dfm), mean_index_matches=by(d, xm, dfm),
                                    mean_index_rows=by(dr, xr, one), n_rows=np.bincount(dr, minlength=10).tolist(), leave_out_product=by(dr, f['s'][mv], one),
                                    fitted=by(dr, vt[mv], one), stayer_profile=by(dr, basis(ar, pr) @ pis / qbar, one))
res['within_match_overall'] = dict(matches=int(sel.sum()), mean=float((s2m * dfm).sum() / dfm.sum()), stayers_mean=float((q * df).sum() / df.sum()))
res['profiles'] = prof; Fm = D.F[mv]
res['movers_by_length'] = {int(r): dict(rows=int((Fm == r).sum()), leave_out_product=float(f['s'][mv][Fm == r].mean()), fitted=float(vt[mv][Fm == r].mean())) for r in np.unique(Fm)}
res['share_capped_mover_rows'] = dict(person=float((np.abs(ar) >= 3).mean()), firm=float((np.abs(pr) >= 3).mean()))
res['first_term_capped_rows'] = (lead[:, mv][:, np.abs(ar) >= 3].sum(1) / tot).tolist()
# ---- concentration diagnostics
sv = np.sqrt(np.maximum(Vx, 1e-300)); rng = np.random.default_rng(11); rown = np.zeros((4, D.n)); U = []
for b in range(2):
    zeta = (sv * rng.standard_normal((D.n, 16)).T).T; _, kz = E.A.kernel(zeta); k4 = np.einsum('ct,tnd->cnd', T34, np.array(kz))
    rown += (k4 ** 2).mean(2) / 2; U.append(k4 * sv[None, :, None])
U = np.concatenate(U, axis=2); G = np.array([U[c].T @ U[c] for c in range(4)]); off = ~np.eye(32, dtype=bool)
rowV = Vx * (4 * g4 ** 2 + 2 * rown); st = E.st_rows; per = np.array([np.bincount(E.st_s, rowV[c][st], len(E.sw)) for c in range(4)])
dg = dict(double_sum_over_V=(2 * np.array([np.trace(G[c]) / 32 for c in range(4)]) / V4).tolist(), max_stayer_share=(per.max(1) / V4).tolist(),
          stayer_share_total=(per.sum(1) / V4).tolist(), max_mover_row_share=(rowV[:, ~D.stay].max(1) / V4).tolist(),
          D2=(np.sqrt(np.array([(G[c][off] ** 2).mean() for c in range(4)])) / V4).tolist())
def Kmul(c, x):
    _, kk = E.A.kernel(sv * x); return sv * (T34[c] @ np.array(kk))
lam2 = []
for c in range(4):
    x = rng.standard_normal(D.n); x /= np.linalg.norm(x)
    for it in range(20):
        yv = Kmul(c, Kmul(c, x)); l2 = np.linalg.norm(yv); x = yv / l2
    lam2.append(l2)
dg['lambda2_over_V'] = (np.array(lam2) / V4).tolist(); res['diagnostics'] = dg; res['total_seconds'] = time.time() - t0
tmp = odir + '/real7.json.tmp'; json.dump(res, open(tmp, 'w'), indent=1); os.replace(tmp, odir + '/real7.json')
print(json.dumps({k: v for k, v in res.items() if k != 'profiles'})[:3000])

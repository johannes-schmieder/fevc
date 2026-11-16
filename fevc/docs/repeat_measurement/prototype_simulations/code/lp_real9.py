"""Six-year Veneto panel, the note's final estimator (lp7.LPF, model for movers 'dq'): one clean run of the standard
error exactly as the note describes it (SCC; aggregates only).  Records solves and seconds of the fit with the
influences and of the 16 draws; the 3x3 variance matrix (no projection) and its eigenvalues; the four standard
errors; the dominance bound sqrt(sum lambda^4)/V-hat and the effective rank from the SAME 16 draws; floors and
the coefficients of the model.  Usage: lp_real9.py <design-dir> <out-dir> <probes>"""
import sys, json, time, os, numpy as np
from lp6 import Panel, build_design, aggregate, T34
from lp7 import LPF
ddir, odir, probes = sys.argv[1], sys.argv[2], int(sys.argv[3]); ofile = odir + '/real9.json'
if os.path.exists(ofile): raise SystemExit(f'{ofile} exists; refusing to overwrite')
os.makedirs(odir, exist_ok=True); t00 = time.time()
z = np.load(ddir + '/panel.npz'); P = Panel(z['w'], z['f'], z['year'], z['y'])
D, info = build_design(P); y = aggregate(info, D.n, P.y)
dz = np.load(ddir + f'/diag_default_{probes}.npz'); E = LPF(D, exact_diag=(dz['Pd'], dz['Bd']))
res = dict(schema='LP-REAL9-V1', model='dq', rows=int(D.n), probes=probes, runs=[])
o = E.fit(y); t_first = time.time() - t00                          # includes the factorisation
for seed in (1, 2):
    t0 = time.time(); s0 = E.A.solves; o = E.fit(y); g = o['g']; ss = E.stayer_stats(y, g); t_fit = time.time() - t0; s_fit = E.A.solves - s0
    t1 = time.time(); mv, vm, minfo = E.movers_model(o); base = np.zeros(D.n); base[mv] = vm; Vx = E.stayer_var(base, ss); t_model = time.time() - t1
    t2 = time.time(); s2 = E.A.solves; rng = np.random.default_rng(seed + 7919); sv = np.sqrt(np.maximum(Vx, 1e-300))
    zeta = (sv * rng.standard_normal((D.n, 16)).T).T; _, kz = E.A.kernel(zeta); kz = np.array(kz)
    tau = np.array([[((Vx * kz[t].T).T * kz[u]).sum(0).mean() for u in range(3)] for t in range(3)])
    V3 = 4 * E.Ga(Vx, g) - 2 * tau - E.adjust(ss, 3.0); t_draws = time.time() - t2; s_draws = E.A.solves - s2
    V3 = 0.5 * (V3 + V3.T); w = np.linalg.eigvalsh(V3); V4 = np.diag(T34 @ V3 @ T34.T)
    k4 = np.einsum('ct,tnd->cnd', T34, kz); off = ~np.eye(16, dtype=bool); d2, reff, dsum = [], [], []
    for c in range(4):
        U = k4[c] * sv[:, None]; G = U.T @ U; m4 = (G[off] ** 2).mean(); t2c = np.trace(G) / 16
        d2.append(float(np.sqrt(m4) / V4[c])); reff.append(float(t2c ** 2 / m4)); dsum.append(float(2 * t2c / V4[c]))
    g4 = T34 @ g; lead = Vx * g4 ** 2; adj = np.diag(T34 @ E.adjust(ss, 3.0) @ T34.T)
    res['runs'].append(dict(seed=seed, draws=16, solves_fit=int(s_fit), seconds_fit=t_fit, seconds_model=t_model, solves_draws=int(s_draws),
                            seconds_draws=t_draws, V3=V3.tolist(), eigenvalues=w.tolist(), V4=V4.tolist(), se=np.sqrt(np.maximum(V4, 0)).tolist(),
                            bound_same_draws=d2, reff_same_draws=reff, double_sum_over_V=dsum, model=minfo,
                            first_term_over_V=(4 * lead.sum(1) / V4).tolist(), movers_share_first_term=(lead[:, mv].sum(1) / lead.sum(1)).tolist(),
                            adjust_over_V=(adj / V4).tolist()))
    print('run', res['runs'][-1], flush=True)
res['theta'] = o['theta'].tolist(); res['seconds_first_fit_with_factorisation'] = t_first; res['total_seconds'] = time.time() - t00
res['job_id'] = os.environ.get('JOB_ID', ''); res['host'] = os.environ.get('HOSTNAME', '')
tmp = ofile + '.tmp'; json.dump(res, open(tmp, 'w')); os.replace(tmp, ofile); print('wrote', ofile, flush=True)

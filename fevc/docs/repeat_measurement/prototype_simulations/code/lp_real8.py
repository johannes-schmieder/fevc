"""Six-year Veneto panel, the note's model for movers (twl) alone: one clean run of the standard error as the
note describes it (SCC; aggregates only).  Records
  - solves and seconds of the least-squares fit with the influences (beta-hat and m-hat) and of the 16 draws;
  - the 3x3 variance matrix of the base components BEFORE the projection onto positive semi-definite matrices
    that LP.estimate applies, its eigenvalues, and the four variances with and without the projection;
  - the dominance bound sqrt(sum lambda^4)/V-hat and the effective rank from the SAME 16 draws.
Usage: lp_real8.py <design-dir> <out-dir> <probes>"""
import sys, json, time, os, numpy as np
from lp6 import Panel, build_design, aggregate, LP, T34, psd
ddir, odir, probes = sys.argv[1], sys.argv[2], int(sys.argv[3])
ofile = odir + '/real8.json'
if os.path.exists(ofile): raise SystemExit(f'{ofile} exists; refusing to overwrite')
os.makedirs(odir, exist_ok=True); t00 = time.time()
z = np.load(ddir + '/panel.npz'); P = Panel(z['w'], z['f'], z['year'], z['y'])
D, info = build_design(P); y = aggregate(info, D.n, P.y)
dz = np.load(ddir + f'/diag_default_{probes}.npz'); E = LP(D, exact_diag=(dz['Pd'], dz['Bd']))
res = dict(schema='LP-REAL8-V1', model='twl', rows=int(D.n), probes=probes, runs=[])
o = E.fit(y); t_first = time.time() - t00                          # includes the factorisation
for seed in (1, 2):
    t0 = time.time(); s0 = E.A.solves; o = E.fit(y); g = o['g']; ss = E.stayer_stats(y, g); t_fit = time.time() - t0; s_fit = E.A.solves - s0
    t1 = time.time(); vt = E.round6_models(o, ss)['twl']; Vx = E.stayer_var(vt, ss); t_model = time.time() - t1
    t2 = time.time(); s2 = E.A.solves; rng = np.random.default_rng(seed + 7919); sv = np.sqrt(np.maximum(Vx, 1e-300))
    zeta = (sv * rng.standard_normal((D.n, 16)).T).T; _, kz = E.A.kernel(zeta); kz = np.array(kz)
    tau = np.array([[((Vx * kz[t].T).T * kz[u]).sum(0).mean() for u in range(3)] for t in range(3)])
    V3 = 4 * E.Ga(Vx, g) - 2 * tau - E.adjust(ss, 3.0); t_draws = time.time() - t2; s_draws = E.A.solves - s2
    V3 = 0.5 * (V3 + V3.T); w = np.linalg.eigvalsh(V3); V4 = np.diag(T34 @ V3 @ T34.T); V4p = np.diag(T34 @ psd(V3) @ T34.T)
    k4 = np.einsum('ct,tnd->cnd', T34, kz); off = ~np.eye(16, dtype=bool); d2, reff, dsum = [], [], []
    for c in range(4):
        U = k4[c] * sv[:, None]; G = U.T @ U; m4 = (G[off] ** 2).mean(); t2c = np.trace(G) / 16
        d2.append(float(np.sqrt(m4) / V4[c])); reff.append(float(t2c ** 2 / m4)); dsum.append(float(2 * t2c / V4[c]))
    res['runs'].append(dict(seed=seed, draws=16, solves_fit=int(s_fit), seconds_fit=t_fit, seconds_model=t_model, solves_draws=int(s_draws),
                            seconds_draws=t_draws, V3=V3.tolist(), eigenvalues=w.tolist(), V4_raw=V4.tolist(), V4_psd=V4p.tolist(),
                            se=np.sqrt(np.maximum(V4, 0)).tolist(), projection_changes=bool(w.min() < 0), bound_same_draws=d2,
                            reff_same_draws=reff, double_sum_over_V=dsum, load=E.last_cells.get('load'), twl_floored=E.last_cells.get('twl_floored')))
    print('run', res['runs'][-1], flush=True)
res['theta'] = o['theta'].tolist(); res['seconds_first_fit_with_factorisation'] = t_first; res['total_seconds'] = time.time() - t00
res['job_id'] = os.environ.get('JOB_ID', ''); res['host'] = os.environ.get('HOSTNAME', '')
tmp = ofile + '.tmp'; json.dump(res, open(tmp, 'w')); os.replace(tmp, ofile); print('wrote', ofile, flush=True)

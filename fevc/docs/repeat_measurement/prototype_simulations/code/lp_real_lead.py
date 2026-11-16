"""A real design: how many solves does the leading direction need by thick-restart Lanczos (lp_lead), and do the q = 1
diagnostics computed from an early-stopped direction agree with those from the converged one?  SCC; aggregates only.

For the person variance, the firm variance and the covariance: the Lanczos history (basis 8 with restarts, and without
restarts), the steps needed for residual tolerances 1e-2, 1e-3, 1e-4, 1e-6, a warm start from the previous component's
direction, and for the directions at 10 and 20 steps and at tolerances 1e-3 and 1e-6 the q = 1 pieces of the note (b1,
the covariance entries, the curvature, the interval, the share of the estimated variance carried by the direction).
Usage: lp_real_lead.py <design-dir> <out-dir> <probes> <cap>"""
import sys, json, time, os, numpy as np
from lp6 import Panel, build_design, aggregate, T34
from lp_q1 import LPQ, q1_interval
from lp_lead import lanczos_leading
ddir, odir, probes, cap = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4]); ofile = odir + '/real_lead.json'
if os.path.exists(ofile): raise SystemExit(f'{ofile} exists; refusing to overwrite')
os.makedirs(odir, exist_ok=True); Z = 1.959963984540054
z = np.load(ddir + '/panel.npz'); P = Panel(z['w'], z['f'], z['year'], z['y'])
D, info = build_design(P); y = aggregate(info, D.n, P.y)
dz = np.load(ddir + f'/diag_default_{probes}.npz'); E = LPQ(D, exact_diag=(dz['Pd'], dz['Bd']))
t0 = time.time(); s0 = E.A.solves; o = E.estimate_final(y, draws=16, dseed=1); V4 = np.diag(T34 @ o['dq'] @ T34.T)
res = dict(schema='LP-REAL-LEAD-V1', rows=int(D.n), mover_rows=int((~D.stay).sum()), firms=int(D.nJ), workers=int(D.nW), probes=probes, cap=cap,
           theta=np.asarray(o['theta']).tolist(), V=V4.tolist(), seconds_standard_error=time.time() - t0, solves_standard_error=int(E.A.solves - s0), comp={})
t1 = time.time(); a = np.random.default_rng(5).standard_normal(D.nW); p = np.random.default_rng(6).standard_normal(D.nJ); p[-1] = 0
for _ in range(5): D.solve(*D.Xt(D.X(a, p)))
res['seconds_per_solve'] = (time.time() - t1) / 5


def first(hist, tol): return next((i + 1 for i, h in enumerate(hist) if h[1] <= tol and i >= 1), None)


prev = None
for c, nm in enumerate(('person', 'firm', 'cov')):
    out = {}; t2 = time.time()
    r8 = lanczos_leading(D, c, m=8, cap=cap, tol=1e-6); rf = lanczos_leading(D, c, m=cap + 1, cap=cap, tol=1e-6)
    out['restart8'] = dict(hist=r8['hist'], steps=r8['steps'], converged=r8['converged'], lam=r8['lam'], steps_to={str(t): first(r8['hist'], t) for t in (1e-2, 1e-3, 1e-4, 1e-6)})
    out['full'] = dict(hist=rf['hist'], steps=rf['steps'], converged=rf['converged'], lam=rf['lam'], steps_to={str(t): first(rf['hist'], t) for t in (1e-2, 1e-3, 1e-4, 1e-6)})
    if prev is not None:
        rw = lanczos_leading(D, c, m=8, cap=cap, tol=1e-3, start=prev); out['warm_start_1e-3'] = dict(steps=rw['steps'], converged=rw['converged'], lam=rw['lam'])
    out['seconds_lanczos'] = time.time() - t2; out['variant'] = {}
    for label, kw in (('cap10', dict(cap=10, tol=1e-12)), ('cap20', dict(cap=20, tol=1e-12)), ('tol1e-3', dict(cap=cap, tol=1e-3)), ('tol1e-6', dict(cap=cap, tol=1e-6))):
        r = lanczos_leading(D, c, m=8, **kw); lam, w1 = r['lam'], r['w1']
        if label == 'tol1e-3': prev = r['coef']
        q = E.q1(y, c, lam, w1, draws=16, dseed=1); S = q['dq']; bf = q['dq_bbfit']; iv = q1_interval(lam, q['b1'], q['theta_r'], S, bf)
        w2 = np.sort(w1 ** 2)[::-1]; cs = np.cumsum(w2); sbb = max(S[0][0], bf); th = float(o['theta'][c]); se = float(np.sqrt(max(V4[c], 0)))
        pi1 = float((4 * lam ** 2 * max(q['b1'] ** 2 - sbb, 0) * sbb + 2 * lam ** 2 * sbb ** 2) / V4[c]) if V4[c] > 0 else None
        out['variant'][label] = dict(steps=r['steps'], converged=r['converged'], resid=r['resid'], lam=lam, lam2=r['lam2'], max_w2=float(w2[0]),
                                     rows_for_half_of_w2=int(np.searchsorted(cs, 0.5) + 1), b1=q['b1'], theta_r=q['theta_r'], Sigma_leave_out=S, Sbb_fitted=bf,
                                     variance_share=pi1, kappa_screen=(2 * abs(lam) * sbb / se if se > 0 else None),
                                     q1=None if iv is None else dict(lo=iv[0], hi=iv[1], kappa=iv[2], z=iv[3], larger=iv[4]), theta=th, se=se,
                                     normal=dict(lo=th - Z * se, hi=th + Z * se))
    res['comp'][nm] = out; print(nm, json.dumps({k: v for k, v in out.items() if k != 'variant'})[:600], flush=True)
    for k, v in out['variant'].items(): print('  ', k, json.dumps(v)[:500], flush=True)
res['seconds_total'] = time.time() - t0
tmp = ofile + '.tmp'; json.dump(res, open(tmp, 'w'), indent=1); os.replace(tmp, ofile); print('wrote', ofile)

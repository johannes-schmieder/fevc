"""A real design (six-year Veneto panel or a sub-panel): the q = 1 diagnostics and interval of the note
(lp_q1.LPQ, model for movers 'dq'). SCC; aggregates only.

For the person variance, the firm variance, the covariance and the variance of the sum: the leading eigenvalue and
its weights (power iteration, at most <max-steps> solves), the eigenvalue share lambda_1^2 / sum lambda^2 (the sum
by <trace-probes> random probes), max w^2 and how concentrated w^2 is, the three covariance entries, the curvature,
the q = 1 interval under the rule of the note and with the leave-out variance alone, the normal interval, and the
solves and seconds of each part.  Usage: lp_real_q1.py <design-dir> <out-dir> <probes> <max-steps> <trace-probes>"""
import sys, json, time, os, numpy as np
from lp6 import Panel, build_design, aggregate, T34
from lp_q1 import LPQ, q1_interval, interval
ddir, odir, probes, max_steps, tprobes = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4]), int(sys.argv[5]); ofile = odir + '/real_q1.json'
if os.path.exists(ofile): raise SystemExit(f'{ofile} exists; refusing to overwrite')
os.makedirs(odir, exist_ok=True); t00 = time.time(); Z = 1.959963984540054
z = np.load(ddir + '/panel.npz'); P = Panel(z['w'], z['f'], z['year'], z['y'])
D, info = build_design(P); y = aggregate(info, D.n, P.y)
dz = np.load(ddir + f'/diag_default_{probes}.npz'); E = LPQ(D, exact_diag=(dz['Pd'], dz['Bd']))
t0 = time.time(); s0 = E.A.solves; o = E.estimate_final(y, draws=16, dseed=1); V4 = np.diag(T34 @ o['dq'] @ T34.T)
res = dict(schema='LP-REAL-Q1-V1', model='dq', rows=int(D.n), mover_rows=int((~D.stay).sum()), firms=int(D.nJ), workers=int(D.nW), probes=probes,
           theta=np.asarray(o['theta']).tolist(), V=V4.tolist(), seconds_standard_error=time.time() - t0, solves_standard_error=int(E.A.solves - s0), comp={})
rng = np.random.default_rng(424242); R = (2.0 * (rng.random((D.n, tprobes)) < 0.5) - 1.0)        # Rademacher probes for trace(B^2) = sum lambda^2
bR = D.solve(*D.Xt(R)); E.A.solves += tprobes
for c, nm in enumerate(('person', 'firm', 'cov', 'sum')):
    t1 = time.time(); s1 = E.A.solves; lam, w1, _, steps = E.leading(c, iters=max_steps); t_lead = time.time() - t1; s_lead = E.A.solves - s1
    t2 = time.time(); s2 = E.A.solves; BR = D.X(*D.solve(*E.Acomp(c, *bR))); E.A.solves += tprobes
    sq = (BR ** 2).sum(0); tr2 = float(sq.mean()); tr2_se = float(sq.std(ddof=1) / np.sqrt(tprobes)); t_tr = time.time() - t2
    t3 = time.time(); s3 = E.A.solves; q = E.q1(y, c, lam, w1, draws=16, dseed=1); t_q = time.time() - t3; s_q = E.A.solves - s3
    S = q['dq']; bf = q['dq_bbfit']; iv = q1_interval(lam, q['b1'], q['theta_r'], S, bf); ivlo = interval(lam, q['b1'], q['theta_r'], S)
    w2 = np.sort(w1 ** 2)[::-1]; cs = np.cumsum(w2); th = float(o['theta'][c]); se = float(np.sqrt(max(V4[c], 0)))
    sbb = max(S[0][0], bf); kscreen = 2 * abs(lam) * sbb / se if se > 0 else None
    res['comp'][nm] = dict(lam1=float(lam), power_steps=int(steps), converged=bool(steps < max_steps), seconds_power=t_lead, solves_power=int(s_lead),
                           sum_lam_sq=tr2, sum_lam_sq_se=tr2_se, eigen_share=float(lam ** 2 / tr2), seconds_trace=t_tr,
                           max_w2=float(w2[0]), rows_for_half_of_w2=int(np.searchsorted(cs, 0.5) + 1), rows_for_90pct_of_w2=int(np.searchsorted(cs, 0.9) + 1),
                           w2_on_movers=float((w1[~D.stay] ** 2).sum()), inverse_sum_w4=float(1.0 / (w2 ** 2).sum()),
                           b1=q['b1'], theta_r=q['theta_r'], Sigma_leave_out=S, Sbb_fitted=bf, b1_over_se=float(q['b1'] / np.sqrt(sbb)),
                           var_share_of_square=float(2 * lam ** 2 * sbb ** 2 / V4[c]) if V4[c] > 0 else None, kappa_screen=kscreen,
                           q1=None if iv is None else dict(lo=iv[0], hi=iv[1], kappa=iv[2], z=iv[3], larger=iv[4]),
                           q1_leave_out=None if ivlo is None else dict(lo=ivlo[0], hi=ivlo[1], kappa=ivlo[2], z=ivlo[3]),
                           theta=th, se=se, normal=dict(lo=th - Z * se, hi=th + Z * se), seconds_q1=t_q, solves_q1=int(s_q))
    print(nm, json.dumps(res['comp'][nm]), flush=True)
res['total_seconds'] = time.time() - t00; res['job_id'] = os.environ.get('JOB_ID', ''); res['host'] = os.environ.get('HOSTNAME', '')
tmp = ofile + '.tmp'; json.dump(res, open(tmp, 'w')); os.replace(tmp, ofile); print('wrote', ofile, flush=True)

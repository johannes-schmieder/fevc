"""Development analysis (after the fact, on the stored q1-conf1 draws): how would an automatic choice between the
normal (high-rank) interval and the q = 1 interval have performed?  Reads only; writes one JSON next to this script."""
import json, glob, sys, os
import numpy as np
R = '<scratch>/runs_q1/q1-conf1'
COMP = ['person', 'firm', 'cov', 'sum']
THR = [0.05, 0.1, 0.2, 0.3, 0.5, 1.0]
cells = sorted(d for d in os.listdir(R) if os.path.isdir(f'{R}/{d}') and d != 'logs')
out = {}
for cell in cells:
    des = json.load(open(f'{R}/{cell}/design.json'))['comp']
    T = [json.load(open(f)) for f in sorted(glob.glob(f'{R}/{cell}/task-*.json'))]
    th0 = np.array(T[0]['theta_true']); lam = np.array(T[0]['lam1'])
    cat = lambda k: [x for t in T for x in t[k]]
    theta = np.array(cat('theta')); V = np.array(cat('V_dq')); b1 = np.array(cat('b1'))
    S = cat('S_dq'); bf = np.array(cat('bbfit_dq')); ci = cat('ci_dq'); n = len(theta)
    out[cell] = {}
    for c in range(4):
        v = V[:, c]; th = theta[:, c]; pos = v > 0
        hw = np.where(pos, 1.96 * np.sqrt(np.where(pos, v, 1.0)), np.nan)
        nlo, nhi = th - hw, th + hw
        qlo = np.array([np.nan if ci[r][c] is None else ci[r][c][0] for r in range(n)])
        qhi = np.array([np.nan if ci[r][c] is None else ci[r][c][1] for r in range(n)])
        kap = np.array([np.inf if ci[r][c] is None else ci[r][c][2] for r in range(n)])
        sbb = np.maximum(np.array([S[r][c][0][0] for r in range(n)]), bf[:, c])
        kscr = np.where(pos, 2 * abs(lam[c]) * sbb / np.sqrt(np.where(pos, v, 1.0)), np.inf)
        share = np.where(pos, 2 * lam[c] ** 2 * sbb ** 2 / np.where(pos, v, 1.0), np.inf)
        cov = lambda lo, hi: float(np.mean((lo <= th0[c]) & (th0[c] <= hi)))       # NaN bounds count as a miss
        res = dict(kappa_true=des[COMP[c]]['kappa'], share_true=des[COMP[c]]['lam_sq_Sbb_sq_over_V'], max_w2=des[COMP[c]]['max_w2'],
                   normal=cov(nlo, nhi), normal_none=float(np.mean(~pos)), q1=cov(qlo, qhi), q1_none=float(np.mean(np.isnan(qlo))),
                   len_q1_over_normal=float(np.nanmean((qhi - qlo)[pos]) / np.nanmean((nhi - nlo)[pos])) if pos.any() else None,
                   kappa_hat_med=float(np.median(kap)), kscreen_med=float(np.median(kscr)), share_med=float(np.median(share)))
        for name, stat in (('khat', kap), ('kscr', kscr), ('share', share)):
            for t in THR:
                use = (~pos) | (stat > t)                                            # q = 1 when no variance estimate or the trigger fires
                lo = np.where(use, qlo, nlo); hi = np.where(use, qhi, nhi)
                # where the q = 1 interval does not exist, fall back to the normal one if it does
                fb = use & np.isnan(lo) & pos; lo = np.where(fb, nlo, lo); hi = np.where(fb, nhi, hi)
                res[f'{name}>{t}'] = dict(cover=cov(lo, hi), q1_share=float(np.mean(use)), none=float(np.mean(np.isnan(lo))),
                                          len=float(np.nanmean(hi - lo)), len_q1=float(np.nanmean(qhi - qlo)))
        out[cell][COMP[c]] = res
json.dump(out, open(os.path.dirname(os.path.abspath(__file__)) + '/switch.json', 'w'), indent=1)
rows = [(cell, cp, out[cell][cp]) for cell in cells for cp in COMP[:3]]
print(f"{'design':22s} {'comp':6s} {'kap':>5s} {'share':>6s} {'normal':>6s} {'none':>5s} {'q1':>5s} | khat>.1 use | khat>.3 use | kscr>.1 use | kscr>.3 use | shr>.1 use")
for cell, cp, r in rows:
    f = lambda k: f"{r[k]['cover']:.3f} {r[k]['q1_share']:.2f}"
    print(f"{cell:22s} {cp:6s} {r['kappa_true']:5.2f} {r['share_true']:6.3f} {r['normal']:6.3f} {r['normal_none']:5.2f} {r['q1']:5.3f} | {f('khat>0.1')} | {f('khat>0.3')} | {f('kscr>0.1')} | {f('kscr>0.3')} | {f('share>0.1')}")
print()
for name in ('khat', 'kscr', 'share'):
    for t in THR:
        k = f'{name}>{t}'; cv = np.array([r[k]['cover'] for _, _, r in rows]); us = np.array([r[k]['q1_share'] for _, _, r in rows])
        ln = np.array([r[k]['len'] / r[k]['len_q1'] for _, _, r in rows])
        print(f"{k:10s} min cover {cv.min():.3f}  mean {cv.mean():.3f}  below 0.93: {(cv < 0.93).sum():2d}  below 0.94: {(cv < 0.94).sum():2d}   q1 used: mean {us.mean():.2f}   length vs always-q1: mean {ln.mean():.3f} min {ln.min():.3f}")
cv = np.array([r['q1'] for _, _, r in rows]); print(f"always q1  min cover {cv.min():.3f}  mean {cv.mean():.3f}")
cv = np.array([r['normal'] for _, _, r in rows]); print(f"always normal (no interval = miss) min cover {cv.min():.3f}  mean {cv.mean():.3f}  below 0.93: {(cv < 0.93).sum()}")

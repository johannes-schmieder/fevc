"""Development analysis on the stored q1-conf1 draws: a cheap trigger that needs only the leading eigenpair, the variance
of b1-hat and the high-rank variance estimate: pi1 = (4 lam^2 max(b1^2 - Sbb, 0) Sbb + 2 lam^2 Sbb^2) / V, the share of the
estimated variance carried by the leading direction (linear and squared part)."""
import json, glob, os
import numpy as np
R = '<scratch>/runs_q1/q1-conf1'
COMP = ['person', 'firm', 'cov', 'sum']; THR = [0.01, 0.02, 0.05, 0.1, 0.2]
cells = sorted(d for d in os.listdir(R) if os.path.isdir(f'{R}/{d}') and d != 'logs'); rows = []
for cell in cells:
    T = [json.load(open(f)) for f in sorted(glob.glob(f'{R}/{cell}/task-*.json'))]
    th0 = np.array(T[0]['theta_true']); lam = np.array(T[0]['lam1']); cat = lambda k: [x for t in T for x in t[k]]
    theta = np.array(cat('theta')); V = np.array(cat('V_dq')); b1 = np.array(cat('b1')); S = cat('S_dq'); bf = np.array(cat('bbfit_dq')); ci = cat('ci_dq'); n = len(theta)
    for c in range(3):
        v = V[:, c]; th = theta[:, c]; pos = v > 0; vs = np.where(pos, v, 1.0); hw = np.where(pos, 1.96 * np.sqrt(vs), np.nan); nlo, nhi = th - hw, th + hw
        qlo = np.array([np.nan if ci[r][c] is None else ci[r][c][0] for r in range(n)]); qhi = np.array([np.nan if ci[r][c] is None else ci[r][c][1] for r in range(n)])
        sbb = np.maximum(np.array([S[r][c][0][0] for r in range(n)]), bf[:, c]); l2 = lam[c] ** 2
        pi1 = np.where(pos, (4 * l2 * np.maximum(b1[:, c] ** 2 - sbb, 0) * sbb + 2 * l2 * sbb ** 2) / vs, np.inf)
        cov = lambda lo, hi: float(np.mean((lo <= th0[c]) & (th0[c] <= hi))); res = dict(cell=cell, comp=COMP[c], normal=cov(nlo, nhi), q1=cov(qlo, qhi), pi_med=float(np.median(pi1)))
        for t in THR:
            use = (~pos) | (pi1 > t); lo = np.where(use, qlo, nlo); hi = np.where(use, qhi, nhi); fb = use & np.isnan(lo) & pos; lo = np.where(fb, nlo, lo); hi = np.where(fb, nhi, hi)
            res[t] = (cov(lo, hi), float(np.mean(use)), float(np.nanmean(hi - lo) / np.nanmean(qhi - qlo)))
        rows.append(res)
for t in THR:
    cv = np.array([r[t][0] for r in rows]); us = np.array([r[t][1] for r in rows]); none = [r for r in rows if r['cell'].startswith('none')]
    worst = min(rows, key=lambda r: r[t][0])
    print(f"pi1>{t:<5} min cover {cv.min():.3f} ({worst['cell']} {worst['comp']})  mean {cv.mean():.3f}  below 0.93: {(cv < 0.93).sum():2d}  below 0.94: {(cv < 0.94).sum():2d}  q1 used: all designs {us.mean():.2f}, no-bottleneck designs {np.mean([r[t][1] for r in none]):.2f}")
print('median pi1, no-bottleneck designs:', ' '.join(f"{r['pi_med']:.3f}" for r in rows if r['cell'].startswith('none')))
print('median pi1, b100 designs:', ' '.join(f"{r['pi_med']:.3f}" for r in rows if r['cell'].startswith('b100')))

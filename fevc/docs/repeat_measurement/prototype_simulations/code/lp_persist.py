"""How much does the treatment of persistent stayer errors move the person-effect variance?  (SCC; aggregates only.)
Usage: lp_persist.py <design-dir> <out-dir>.  No linear solves: with exact stayer leverages the stayers' part of the
leave-out correction of the person-effect variance is sum_g A_gg * (estimate of the variance of the stayer's mean error),
A_gg = (T_g/N)(1 - T_g/N).  Three estimates of that variance:
  iid        s_g^2 / T_g                       (what the package assumes: independent errors across a stayer's years)
  transitory sigma_e^2 / T_g                   (random walk treated as part of the worker's wage; only transitory noise is error)
  walk       sigma_e^2/T + sigma_u^2 (T-1)(2T-1)/(6T)   (random walk from the first year treated as error)
with sigma_e^2 = -mean of products of adjacent first differences and sigma_u^2 = mean squared first difference - 2 sigma_e^2,
pooled within groups of stayers by number of years (consecutive years only)."""
import sys, os, json, numpy as np
from lp import Panel
ddir, odir = sys.argv[1], sys.argv[2]; os.makedirs(odir, exist_ok=True)
z = np.load(ddir + '/panel.npz'); P = Panel(z['w'], z['f'], z['year'], z['y']); N = P.n
st = P.stay_w[P.w]; idx = np.flatnonzero(st); o = idx[np.lexsort((P.year[idx], P.w[idx]))]
w, yr, y = P.w[o], P.year[o].astype(int), P.y[o]; T = P.T_w[w]
first = np.r_[True, w[1:] != w[:-1]]; sid = np.cumsum(first) - 1; nS = sid.max() + 1; Tg = np.bincount(sid)
mean = np.bincount(sid, y) / Tg; s2 = np.bincount(sid, (y - mean[sid]) ** 2) / (Tg - 1)
Agg = (Tg / N) * (1 - Tg / N)
d = np.where((w[1:] == w[:-1]) & (yr[1:] - yr[:-1] == 1), y[1:] - y[:-1], np.nan)            # first differences, consecutive years
prod = d[1:] * d[:-1]; okp = ~np.isnan(prod) & (w[2:] == w[:-2])
res = dict(person_years=int(N), stayers=int(nS), groups={})
corr = dict(iid=0.0, transitory=0.0, walk=0.0)
for t in range(2, 7):
    g = Tg == t; rows = Tg[sid] == t
    dd = d[rows[1:] & rows[:-1]]; dd = dd[~np.isnan(dd)]
    pp = prod[(rows[2:] & rows[:-2]) & okp]
    se2 = float(-pp.mean()) if len(pp) else float('nan'); su2 = float((dd ** 2).mean() - 2 * se2) if len(pp) else float('nan')
    res['groups'][t] = dict(stayers=int(g.sum()), mean_s2=float(s2[g].mean()), sigma_e2=se2, sigma_u2=su2, diff_pairs=int(len(pp)), mean_sq_diff=float((dd ** 2).mean()))
ref = res['groups'][6]                                                                      # two-year stayers have no product of differences
for t in range(2, 7):
    g = Tg == t; G = res['groups'][t]; se2 = G['sigma_e2'] if t >= 3 else float('nan'); su2 = G['sigma_u2'] if t >= 3 else float('nan')
    if t == 2:                                                                               # split the two-year variance in the six-year proportions
        ms = G['mean_sq_diff']; share = ref['sigma_u2'] / (ref['sigma_u2'] + 2 * ref['sigma_e2']); su2 = ms * share; se2 = ms * (1 - share) / 2
        G['sigma_e2'], G['sigma_u2'], G['imputed'] = se2, su2, True
    w_g = Agg[g].sum()
    corr['iid'] += float((Agg[g] * s2[g] / t).sum()); corr['transitory'] += w_g * se2 / t
    corr['walk'] += w_g * (se2 / t + su2 * (t - 1) * (2 * t - 1) / (6 * t))
res['stayer_correction'] = corr
res['shift_vs_iid'] = dict(transitory=corr['iid'] - corr['transitory'], walk=corr['iid'] - corr['walk'])   # change in the person-effect variance estimate
json.dump(res, open(odir + '/persist.json', 'w'), indent=1); print(json.dumps(res, indent=1))

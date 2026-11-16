"""Independent dense calculation of the repeat-measurement standard error under observation deletion on the panel
exported from Stata. Units are physical observations; a worker-firm cell with two or more observations is a
repeat-measurement set; the single observations of movers receive the fitted model, fitted on all observations of movers
when there are fewer than a hundred of them. With an argument 'x1' the control is partialled out by the full regression
first and its estimated index held fixed. Shares no code with the package."""
import sys, csv
import numpy as np
rows = list(csv.DictReader(open('panel_obs.csv'))); y = np.array([float(r['log_wage']) for r in rows]); w = np.array([int(r['worker_id']) for r in rows]); f = np.array([int(r['firm_id']) for r in rows])
uw, wi = np.unique(w, return_inverse=True); uf, fi = np.unique(f, return_inverse=True); W, J, n = len(uw), len(uf), len(y); p = W + J - 1
X = np.zeros((n, p)); X[np.arange(n), wi] = 1.0
for i in range(n):
    if fi[i] < J - 1: X[i, W + fi[i]] = 1.0
if len(sys.argv) > 1:
    z = np.array([float(r[sys.argv[1]]) for r in rows]); full = np.column_stack([X, z]); coef = np.linalg.lstsq(full, y, rcond=None)[0]; y = y - z * coef[-1]
    print('control coefficient %.15e' % coef[-1])
Aa = np.zeros((n, p)); Aa[:, :W] = X[:, :W]; Af = np.zeros((n, p)); Af[:, W:] = X[:, W:]
cen = lambda Z: Z - Z.mean(0)
T = [cen(Aa).T @ cen(Aa) / n, cen(Af).T @ cen(Af) / n, 0.5 * (cen(Aa).T @ cen(Af) + cen(Af).T @ cen(Aa)) / n]
Sinv = np.linalg.inv(X.T @ X); P = X @ Sinv @ X.T; M = np.eye(n) - P; lev = np.diag(P).copy(); Md = 1 - lev
assert lev.max() < 1 - 1e-9
H = np.eye(n) - np.ones((n, n)) / n; yc = H @ y; beta = Sinv @ X.T @ y; res = M @ y
K = []; Bd = []
for t in range(3):
    B = X @ Sinv @ T[t] @ Sinv @ X.T; d = np.diag(B) / Md; C = B - 0.5 * (d[:, None] * M + M * d[None, :]); K.append(H @ C @ H); Bd.append(np.diag(B).copy())
theta = np.array([yc @ K[t] @ yc for t in range(3)]); prod = yc * res / Md
cells = {}
for i in range(n): cells.setdefault((wi[i], fi[i]), []).append(i)
nfirm = np.zeros(W, int)
for (k, j) in cells: nfirm[k] += 1
mover = nfirm[wi] > 1
floor = 1e-8 * np.median(res ** 2 / Md); var = np.zeros(n); pair = np.zeros((3, 3)); single = []; nsets = 0; nset_units = 0
for (k, j), idx in sorted(cells.items()):
    idx = np.array(idx); Tg = len(idx)
    if Tg == 1:
        assert mover[idx[0]]; single.append(idx[0]); continue
    s2 = y[idx].var(ddof=1)
    c = np.array([(Bd[t][idx] / Md[idx]).mean() for t in range(3)]) if mover[idx[0]] else np.array([Bd[0][idx].mean() * Tg / (Tg - 1), 0.0, 0.0])
    pair += 4 * s2 ** 2 * (Tg - 2) * (Tg - 1) / (Tg + 1) * np.outer(c, c); var[idx] = max(s2, floor); nsets += 1; nset_units += Tg
single = np.array(sorted(single)); fit = single if len(single) >= 100 else np.flatnonzero(mover); m = len(fit)


def midrank(x):
    x = np.asarray(x, float); below = (x[None, :] < x[:, None]).sum(1); equal = (x[None, :] == x[:, None]).sum(1)
    return 2 * ((below + 0.5 * (equal + 1) - 0.5) / len(x)) - 1


ls = lambda Z, v: Z @ np.linalg.lstsq(Z, v, rcond=None)[0]
lr, mr = midrank(lev[fit]), midrank(np.ones(m)); perm = lambda k: max(1, min(k, m // 100))
Z = np.column_stack([np.ones(m), lr, mr, lr ** 2, mr ** 2])[:, :perm(5)]; scale = np.maximum(ls(Z, prod[fit]), 0.05 * prod[fit].mean())
ae = beta[wi]; pe = np.where(fi < J - 1, beta[np.minimum(W + fi, p - 1)], 0.0)
std = lambda v: np.clip((v - v.mean()) / np.sqrt(((v - v.mean()) ** 2).mean()), -3, 3)
a, q = std(ae)[fit], std(pe)[fit]
Bm = np.column_stack([np.ones(m), a, a ** 2, q, q ** 2, a * q])[:, :perm(6)] * scale[:, None]
fitted = np.maximum(ls(Bm, prod[fit]), floor); pos = {u: k for k, u in enumerate(fit)}
for u in single: var[u] = fitted[pos[u]]
infl = [K[t] @ yc for t in range(3)]
V3 = np.array([[4 * (var * infl[t] * infl[s]).sum() - 2 * np.trace(K[t] @ np.diag(var) @ K[s] @ np.diag(var)) for s in range(3)] for t in range(3)]) + pair
R = np.array([[1, 0, 0], [0, 1, 0], [0, 0, 1], [1, 1, 2.0]]); V4 = R @ V3 @ R.T
print('rows', n, 'sets', nsets, 'set units', nset_units, 'single', len(single), 'fitted on', m, 'scale columns', perm(5), 'profile columns', perm(6))
print('theta', ' '.join('%.15e' % v for v in theta))
print('pair', ' '.join('%.15e' % v for v in pair.ravel()))
print('se', ' '.join('%.15e' % v for v in np.sqrt(np.diag(V4))))
print('V4', ' '.join('%.15e' % v for v in V4.ravel()))

# --- leading directions and the q = 1 decomposition (dense, exact trace) ------------------------------------------
import scipy.linalg as sla
S = X.T @ X; names = ['worker', 'firm', 'cov', 'sum']; T4 = T + [T[0] + T[1] + 2 * T[2]]; K4 = K + [K[0] + K[1] + 2 * K[2]]; Bd4 = Bd + [Bd[0] + Bd[1] + 2 * Bd[2]]
rng = np.random.default_rng(1); zdraw = np.abs(rng.standard_normal((2, 4_000_000)))


def critical(kappa, level=0.95):
    a, b = zdraw
    if kappa <= 1e-10: d = a
    else: d = (a * a + b * b + 2 * a / kappa) / (np.sqrt(b * b + (a + 1 / kappa) ** 2) + 1 / kappa)
    return float(np.quantile(d, level))


V = np.diag(var); maker = 1 / Md
for t in range(4):
    lam_all, vec = sla.eigh(T4[t], S); k = int(np.argmax(np.abs(lam_all))); lam = float(lam_all[k]); v = vec[:, k]; mode = X @ v; mode = mode / np.sqrt(mode @ mode)
    second = float(np.sort(np.abs(lam_all))[-2])
    b1 = float(mode @ yc); lo = float((mode ** 2 * prod).sum()); fit_v = float((mode ** 2 * var).sum()); lead = max(lo, fit_v)
    d1 = mode ** 2 * maker; K1 = np.outer(mode, mode) - 0.5 * (d1[:, None] * M + M * d1[None, :]); Kr = K4[t] - lam * (H @ K1 @ H)
    mr = Kr @ yc; point = float(yc @ K4[t] @ yc); direct = float(yc @ mr); theta_r = point - lam * (b1 * b1 - lo)
    linear = float(4 * (var * mr * mr).sum()); cross = float(2 * (mode * var * mr).sum()); trace = float(np.trace(Kr @ V @ Kr @ V))
    pair_r = 0.0
    for (kk, j), idx in sorted(cells.items()):
        idx = np.array(idx); Tg = len(idx)
        if Tg < 2: continue
        s2 = y[idx].var(ddof=1)
        if mover[idx[0]]: c = ((Bd4[t][idx] - lam * mode[idx] ** 2) * maker[idx]).mean()
        else: c = ((Bd4[t][idx] if t in (0, 3) else 0.0) - lam * mode[idx] ** 2).sum() / (Tg - 1)
        pair_r += 4 * c * c * s2 * s2 * (Tg - 2) * (Tg - 1) / (Tg + 1)
    rr = linear - 2 * trace + pair_r; rho = cross / np.sqrt(lead * rr); kappa = 2 * abs(lam) * lead / np.sqrt(rr * (1 - rho * rho)); z = critical(kappa)
    L = np.linalg.cholesky(np.array([[lead, cross], [cross, rr]])); phi = np.linspace(0, 2 * np.pi, 2_000_001)
    f = lam * (b1 + z * L[0, 0] * np.cos(phi)) ** 2 + theta_r + z * (L[1, 0] * np.cos(phi) + L[1, 1] * np.sin(phi))
    V4tt = V4[t, t]; share = (4 * lam ** 2 * max(b1 * b1 - lead, 0) * lead + 2 * lam ** 2 * lead ** 2) / V4tt
    print(f"q1 {names[t]} lam {lam:.15e} second {second:.6e} maxw2 {float((mode ** 2).max()):.15e} b1 {abs(b1):.15e} lo {lo:.15e} fit {fit_v:.15e} theta_r {theta_r:.15e} identity {abs(direct - theta_r):.2e} "
          f"linear {linear:.15e} cross {abs(cross):.15e} trace2 {2 * trace:.15e} pair {pair_r:.15e} rr {rr:.15e} kappa {kappa:.15e} z {z:.6f} lb {f.min():.15e} ub {f.max():.15e} share {share:.15e} eigshare {(lam * fit_v) ** 2 / np.trace(K4[t] @ V @ K4[t] @ V):.15e}")

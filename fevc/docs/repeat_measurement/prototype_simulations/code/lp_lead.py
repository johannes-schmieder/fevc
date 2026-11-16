"""Leading direction of A_c v = lam S v by thick-restart Lanczos in the S inner product (one solve per step).

The prototype's power iteration (lp_q1.LPQ.leading) needs 90-200 solves per component on the Veneto designs, because
without a dominant direction the leading eigenvalues are close, and for the covariance they come with both signs.
Lanczos finds both ends of the spectrum at once; the direction of largest absolute eigenvalue is reported.

lanczos_leading(D, c, m, cap, tol, seed) -> dict(lam, w1, coef, steps, resid, lam2, hist, converged)
  m    basis size before a restart (m >= cap: no restart, full reorthogonalisation)
  cap  maximum number of solves
  tol  stop when the residual of the leading pair, ||S^-1 A v - lam v||_S, is at most tol * |lam|
  hist per step: (leading Ritz value, relative residual of the previous leading pair, second Ritz value by magnitude)
Rows are used for the inner product: <u, v>_S = (Xu)'(Xv)."""
import numpy as np
from lp6 import T34


def Acomp(D, c, a, p):
    ra = 0.0; rp = 0.0
    for t in range(3):
        if T34[c, t] != 0: x, yv = D.A(t, a, p); ra = ra + T34[c, t] * x; rp = rp + T34[c, t] * yv
    return ra, rp


def lanczos_leading(D, c, m=8, cap=40, tol=1e-3, seed=0, keep=3, start=None):
    rng = np.random.default_rng([seed, c])
    if start is None:
        a = rng.standard_normal(D.nW); p = rng.standard_normal(D.nJ); p[-1] = 0.0
    else: a, p = (np.array(v, float) for v in start)
    x = D.X(a, p); nrm = np.sqrt(x @ x); a, p, x = a / nrm, p / nrm, x / nrm
    Va, Vp, Vx = [a], [p], [x]; ga, gp = Acomp(D, c, a, p); Ga, Gp = [ga], [gp]
    H = np.array([[a @ ga + p @ gp]]); hist = []; steps = 0; best = None; conv = False
    dot = lambda j, ra, rp: Va[j] @ ra + Vp[j] @ rp
    while steps < cap:
        ua, up = D.solve(Ga[-1], Gp[-1]); steps += 1; ux = D.X(ua, up)
        for _ in range(2):                                       # S-orthogonalise against the basis, twice
            for j in range(len(Vx)):
                h = Vx[j] @ ux; ua = ua - h * Va[j]; up = up - h * Vp[j]; ux = ux - h * Vx[j]
        nrm = np.sqrt(ux @ ux); th, Y = np.linalg.eigh(H); order = np.argsort(-np.abs(th)); k = len(th)
        if nrm <= 1e-12 * max(1.0, abs(th[order[0]])):          # invariant subspace: exact
            lead = order[0]; res = 0.0
        else:
            ua, up, ux = ua / nrm, up / nrm, ux / nrm; ga, gp = Acomp(D, c, ua, up)
            b = np.array([dot(j, ga, gp) for j in range(k)]); lead = order[0]
            res = abs(b @ Y[:, lead]) / abs(th[lead])            # relative residual of the leading pair of the current basis
        lam = float(th[lead]); lam2 = float(th[order[1]]) if k > 1 else 0.0; hist.append((lam, float(res), lam2))
        best = (lam, Y[:, lead].copy(), [v.copy() for v in Va], [v.copy() for v in Vp], [v.copy() for v in Vx], lam2, float(res))
        if res <= tol and steps >= 2: conv = True; break
        if nrm <= 1e-12 * max(1.0, abs(lam)): conv = True; break
        if k >= m:                                               # thick restart: keep the Ritz vectors of largest magnitude
            sel = order[:keep]; Ys = Y[:, sel]
            comb = lambda L: [sum(Ys[j, i] * L[j] for j in range(k)) for i in range(len(sel))]
            Va, Vp, Vx, Ga, Gp = comb(Va), comb(Vp), comb(Vx), comb(Ga), comb(Gp)
            Hn = np.zeros((len(sel) + 1, len(sel) + 1)); Hn[:-1, :-1] = np.diag(th[sel]); bb = Ys.T @ b; Hn[:-1, -1] = bb; Hn[-1, :-1] = bb
            Hn[-1, -1] = ua @ ga + up @ gp; H = Hn
        else:
            Hn = np.zeros((k + 1, k + 1)); Hn[:-1, :-1] = H; Hn[:-1, -1] = b; Hn[-1, :-1] = b; Hn[-1, -1] = ua @ ga + up @ gp; H = Hn
        Va.append(ua); Vp.append(up); Vx.append(ux); Ga.append(ga); Gp.append(gp)
    lam, y, Ba, Bp, Bx, lam2, res = best
    a = sum(y[j] * Ba[j] for j in range(len(y))); p = sum(y[j] * Bp[j] for j in range(len(y))); x = sum(y[j] * Bx[j] for j in range(len(y)))
    nrm = np.sqrt(x @ x); a, p, x = a / nrm, p / nrm, x / nrm
    return dict(lam=float(T34[c] @ D.theta(a, p)), w1=x, coef=(a, p), steps=steps, resid=res, lam2=lam2, hist=hist, converged=conv)

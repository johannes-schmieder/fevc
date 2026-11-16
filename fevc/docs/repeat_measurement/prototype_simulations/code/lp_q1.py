"""q = 1 interval with the note's variance estimates (prototype; Section 7 of the note).

For a component c (0 person variance, 1 firm variance, 2 covariance, 3 variance of the sum):
  leading(c)      leading generalised eigenpair  A_c v = lam S v,  v'S v = 1  (power iteration on S^{-1} A_c),
                  w1 = X v
  q1(y, c, ...)   b1-hat = w1'y, the remainder theta_r-hat = theta-hat - lam (b1-hat^2 - sum w1^2 sigma-hat^2), and
                  the three entries of the covariance matrix of (b1-hat, theta_r-hat):
                    S_bb = sum w1^2 sigma-hat_i^2                       (leave-out estimates; no model)
                    S_br = 2 sum w1 m_r-hat sigma-tilde^2               (stayers: q_g; movers: model or given values)
                    S_rr = 4 sum m_r-hat^2 sigma-tilde^2 - 2 double sum(C_r) + stayer adjustment with c_g^r
  am_critical     critical value of Andrews and Mikusheva (2016) for curvature kappa (one nuisance direction)
  interval        smallest and largest value of lam b^2 + t over the ellipse
The outcome is centred at its person-year mean, as everywhere in the prototype (kernels are H C H)."""
import numpy as np
from lp6 import T34
from lp7 import LPF


def am_critical(kappa, alpha=0.05):
    """(1 - alpha) quantile of sqrt(Z1^2 + (|Z2| + 1/kappa)^2) - 1/kappa, Z1, Z2 independent standard normal."""
    from scipy.special import ndtr
    if kappa <= 1e-10: return 1.959963984540054 if abs(alpha - 0.05) < 1e-12 else float(np.sqrt(2) * _erfinv(1 - alpha))
    R = 1.0 / kappa; x, wq = np.polynomial.legendre.leggauss(200)
    def cdf(zc):                                                  # P(dist <= zc)
        t = 0.5 * zc * (x + 1); wt = 0.5 * zc * wq               # |Z2| = t in [0, zc]
        half = np.sqrt(np.maximum((zc + R) ** 2 - (t + R) ** 2, 0.0))
        return float((wt * 2 * np.exp(-0.5 * t * t) / np.sqrt(2 * np.pi) * (2 * ndtr(half) - 1)).sum())
    lo, hi = 1.0, 4.0
    for _ in range(60):
        mid = 0.5 * (lo + hi)
        if cdf(mid) < 1 - alpha: lo = mid
        else: hi = mid
    return 0.5 * (lo + hi)


def _erfinv(p):
    from statistics import NormalDist
    return NormalDist().inv_cdf(0.5 * (1 + p)) / np.sqrt(2)


def interval(lam, b1, th_r, S, alpha=0.05, crit=None):
    """KSS q = 1 interval. Returns (lo, hi, kappa, z) or None if S is not positive definite."""
    Sbb, Sbr, Srr = S[0][0], S[0][1], S[1][1]
    if not (Sbb > 0 and Srr > 0 and Sbb * Srr - Sbr * Sbr > 0): return None
    rho = Sbr / np.sqrt(Sbb * Srr); kappa = 2 * abs(lam) * Sbb / (np.sqrt(Srr) * np.sqrt(1 - rho * rho))
    z = am_critical(kappa, alpha) if crit is None else crit(kappa)
    L = np.linalg.cholesky(np.array([[Sbb, Sbr], [Sbr, Srr]]))
    f = lambda phi: lam * (b1 + z * (L[0, 0] * np.cos(phi))) ** 2 + th_r + z * (L[1, 0] * np.cos(phi) + L[1, 1] * np.sin(phi))
    phi = np.linspace(0, 2 * np.pi, 4001); v = f(phi); out = []
    for k, sgn in ((int(np.argmin(v)), 1.0), (int(np.argmax(v)), -1.0)):
        a, c = phi[k] - phi[1], phi[k] + phi[1]                  # golden-section refinement on the bracketing cell
        gr = (np.sqrt(5) - 1) / 2
        for _ in range(60):
            x1 = c - gr * (c - a); x2 = a + gr * (c - a)
            if sgn * f(x1) < sgn * f(x2): c = x2
            else: a = x1
        out.append(float(f(0.5 * (a + c))))
    return out[0], out[1], float(kappa), float(z)


def q1_interval(lam, b1, th_r, S, Sbb_fit, alpha=0.05):
    """The rule of the note. The variance of b1-hat is the larger of the leave-out estimate S[0][0] (unbiased, but noisy
    and possibly negative when few rows carry b1-hat) and the value from the fitted variances Sbb_fit (stayers' sample
    variances and the movers' model; positive). The bias correction inside theta-hat_r is not affected.
    Returns (lo, hi, kappa, z, which) with which = 'leave-out' or 'fitted', or None if the matrix is not positive
    definite."""
    which = 'leave-out' if S[0][0] >= Sbb_fit else 'fitted'
    iv = interval(lam, b1, th_r, [[max(S[0][0], Sbb_fit), S[0][1]], [S[0][1], S[1][1]]], alpha)
    return None if iv is None else (*iv, which)


class LPQ(LPF):
    def Acomp(self, c, a, p):
        ra = 0.0; rp = 0.0
        for t in range(3):
            if T34[c, t] != 0: x, yv = self.D.A(t, a, p); ra = ra + T34[c, t] * x; rp = rp + T34[c, t] * yv
        return ra, rp

    def leading(self, c, iters=200, tol=1e-9, seed=0):
        """Leading eigenpair (largest absolute value) of A_c v = lam S v with v'Sv = 1; returns lam, w1 = Xv, (a, p), steps."""
        D = self.D; rng = np.random.default_rng([seed, c]); a = rng.standard_normal(D.nW); p = rng.standard_normal(D.nJ); p[-1] = 0.0
        lam = 0.0
        for it in range(iters):
            a, p = D.solve(*self.Acomp(c, a, p)); self.A.solves += 1
            x = D.X(a, p); nrm = np.sqrt(x @ x); a, p = a / nrm, p / nrm
            new = float(T34[c] @ D.theta(a, p))
            if it > 3 and abs(new - lam) <= tol * abs(new): lam = new; break
            lam = new
        return lam, D.X(a, p), (a, p), it + 1

    def K1(self, w1, hv, mv):
        """Kernel of b1^2 - sum w1^2 sigma-hat^2 applied to a centred vector hv (mv = M hv): H[w1 (w1'hv) - D1 mv/2 - M(D1 hv)/2]."""
        A = self.A; d1 = w1 ** 2 / A.Md; dv = (d1 * hv.T).T
        c = np.multiply.outer(w1, w1 @ hv) - 0.5 * (d1 * mv.T).T - 0.5 * (dv - A.P(dv))
        return A.H(c)

    def q1(self, y, c, lam, w1, draws=16, dseed=1, vtrue=None):
        """Point pieces and the covariance entries for the model 'dq' and, if vtrue is given, for 'or' (movers' true variances)."""
        A = self.A; D = self.D; o = self.fit(y); g = o['g']; ss = self.stayer_stats(y, g); z, e, s = o['z'], o['e'], o['s']
        rng = np.random.default_rng(dseed + 104729)
        theta = float(T34[c] @ o['theta3']); b1 = float(w1 @ z); Sbb = float((w1 ** 2 * s).sum()); theta_r = theta - lam * (b1 * b1 - Sbb)
        mr = T34[c] @ g - lam * self.K1(w1, z, e)                 # influence of the remainder, C_r y
        mv, vm, info = self.movers_model(o); base = np.zeros(D.n); base[mv] = vm
        # stayers' pair weight for the remainder (exact diagonals): c_g^r = (B_ii - lam w_g^2) R/(R-1)
        first = self.st_rows[np.r_[True, self.st_s[1:] != self.st_s[:-1]]]
        cr = ((T34[c] @ A.Bd)[first] - lam * w1[first] ** 2) * self.R / (self.R - 1.0)
        adj = float((4 * cr ** 2 * ss['q'] ** 2 * (self.R - 2) * (self.R - 1) / (self.R + 1)).sum())
        out = dict(theta=theta, b1=b1, theta_r=theta_r, Sbb=Sbb, diag=info)
        for name, b in (('dq', base), ('or', vtrue)):
            if b is None: continue
            Vx = self.stayer_var(b, ss); sv = np.sqrt(np.maximum(Vx, 1e-300))
            zeta = (sv * rng.standard_normal((D.n, draws)).T).T; mz, kz = A.kernel(zeta)
            kr = np.einsum('t,tnd->nd', T34[c], np.array(kz)) - lam * self.K1(w1, A.H(zeta), mz)
            tau = float(((Vx * (kr ** 2).T).T).sum(0).mean())
            Sbr = float(2 * (w1 * mr * Vx).sum()); Srr = float(4 * (Vx * mr ** 2).sum() - 2 * tau + adj)
            out[name] = [[Sbb, Sbr], [Sbr, Srr]]; out[name + '_bbfit'] = float((w1 ** 2 * Vx).sum())
        return out

    def q1_truth(self, mu, v_rows, c, lam, w1, draws=512, seed=5):
        """True covariance matrix of (b1-hat, theta_r-hat) for independent errors with variances v_rows (projected weights)."""
        A = self.A; rng = np.random.default_rng(seed); hmu = A.H(mu); a, p = A.beta(hmu); e_mu, g = A.kernel(mu, bv=(a, p))
        mr = T34[c] @ np.array(g) - lam * self.K1(w1, hmu, e_mu); tau = 0.0; sv = np.sqrt(v_rows)
        for b in range(max(1, draws // 32)):
            zeta = (sv * rng.standard_normal((self.D.n, 32)).T).T; mz, kz = A.kernel(zeta)
            kr = np.einsum('t,tnd->nd', T34[c], np.array(kz)) - lam * self.K1(w1, A.H(zeta), mz)
            tau += float(((v_rows * (kr ** 2).T).T).sum(0).mean()) / max(1, draws // 32)
        Sbb = float((w1 ** 2 * v_rows).sum()); Sbr = float(2 * (w1 * mr * v_rows).sum()); Srr = float(4 * (v_rows * mr ** 2).sum() + 2 * tau)
        return dict(Sigma=[[Sbb, Sbr], [Sbr, Srr]], b1=float(w1 @ hmu), mr=mr)

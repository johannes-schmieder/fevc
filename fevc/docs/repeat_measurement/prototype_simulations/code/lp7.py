"""The estimator of the note in its final form (2026-11-12, owner's choice of the conservative model).

Movers' rows: scale x profile, both fitted to the movers' own leave-out estimates sigma-hat_i^2:
  step 1  scale_i   = fitted value of a least-squares regression on z_i = (1, p, f, p^2, f^2), p and f the ranks of
                      the leverage and of the match length (floored at 5% of the mean leave-out estimate);
  step 2  profile_i = b_i'd with b_i = (1, a, a^2, p, p^2, a p) in the standardised estimated person and firm
                      effects winsorised at +-3, d from the least-squares regression of sigma-hat_i^2 on scale_i b_i.
Stayers' rows: the stayer's sample variance.  The 3x3 variance matrix of the base components is returned as
computed: no projection onto positive semi-definite matrices (lp6.LP.estimate applied one).  Frozen for the
validation runs; lp6.py is unchanged."""
import numpy as np
from lp6 import LP, T34, midrank  # noqa: F401  (T34 re-exported)

CAP = 3.0


class LPF(LP):
    def movers_model(self, o, cap=CAP):
        D = self.D; mv = np.flatnonzero(~D.stay); s = o['s'][mv]; at, pt = self.std_effects(o, cap)
        ls = lambda X, yv: np.linalg.lstsq(X.T @ X, X.T @ yv, rcond=None)[0]
        lr = midrank(self.A.Pd[mv]); mr = midrank(D.F[mv]); Z = np.column_stack([np.ones(len(mv)), lr, mr, lr ** 2, mr ** 2])
        raw = Z @ ls(Z, s); fs = 0.05 * s.mean(); sc = np.maximum(raw, fs)
        a, p = at[D.w[mv]], pt[D.f[mv]]; Bm = np.column_stack([np.ones(len(mv)), a, a ** 2, p, p ** 2, a * p])
        X = sc[:, None] * Bm; d = ls(X, s); fit = X @ d
        info = dict(scale_floored=float((raw < fs).mean()), final_floored=float((fit < o['floor']).mean()),
                    mean_leave_out=float(s.mean()), profile_min=float((Bm @ d).min()), profile_max=float((Bm @ d).max()), coef=d.tolist())
        return mv, np.maximum(fit, o['floor']), info

    def estimate_final(self, y, draws=16, dseed=1, vtrue=None):
        """theta-hat and the 3x3 variance matrices 'dq' (the note's estimator) and, if vtrue is given, 'or' (movers'
        true variances).  eig_<name>: smallest over largest eigenvalue of the matrix (negative if not PSD)."""
        o = self.fit(y); g = o['g']; ss = self.stayer_stats(y, g); rng = np.random.default_rng(dseed + 7919)
        mv, vm, info = self.movers_model(o); base = np.zeros(self.D.n); base[mv] = vm
        out = dict(theta=o['theta'], fit=o, ss=ss, diag=info, movers=mv, movers_fitted=vm)
        for name, b in (('dq', base), ('or', vtrue)):
            if b is None: continue
            Vx = self.stayer_var(b, ss); tau, _ = self.pseudo(np.maximum(Vx, 1e-300), draws, rng)
            V3 = 4 * self.Ga(Vx, g) - 2 * tau - self.adjust(ss, 3.0); V3 = 0.5 * (V3 + V3.T)
            w = np.linalg.eigvalsh(V3); out[name] = V3; out['eig_' + name] = float(w[0] / w[-1])
        return out

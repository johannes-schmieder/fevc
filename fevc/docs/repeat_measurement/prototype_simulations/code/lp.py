"""Longer-panel prototype of the repeat-measurement standard error (stayer step).

Rows follow FEVC's pooled match/stayer design: one row per mover match (mass F = match length, scaled outcome
sqrt(F) * match mean) and, for stayers, either one row per person-year (the default) or two block rows of equal
length separated by a gap (the option for serially correlated stayer errors).  A stayer's rows have identical
regressors and equal mass, so their leverage and target diagonal are known exactly:
    P_ii = 1/R,  B_ii = F A_gg / n_g^2,  pair weight c = B_ii / (1 - P_ii),
with R rows of mass F, n_g = R F person-years, A_gg = (n_g/N)(1 - n_g/N) for the person-effect variance (zero
for the firm-effect variance and the covariance).  Movers' diagonals come from random projections.

Estimators (3x3 over the primitive components worker, firm, cov; map with T34):
  shipped  residual signal with simulated calibration (as the package ships today)
  model    working variance for every row
  prop     stayers: sample variance of the stayer's rows; movers: working variance; minus the adjustment
           sum_U T_U q_U^2 / (1 + nu_U) at stated kurtosis kappa (identical-row closed form)
  propfree as prop, but stayers with R >= 4 rows use the adjustment built from k-statistics, which needs
           neither a stated kurtosis nor zero skewness
"""
import time
import numpy as np
from akm_design import Design, prune
from sparse_auto import AutoSE
from dense_mc import T34


def midrank(x):
    o = np.argsort(x, kind='stable'); xs = x[o]; n = len(x)
    start = np.flatnonzero(np.r_[True, xs[1:] != xs[:-1]]); end = np.r_[start[1:], n]
    r = np.empty(n); r[o] = np.repeat(0.5 * (start + 1 + end), end - start)
    return 2 * ((r - 0.5) / n) - 1


def rust_working(proxy, lev, stay, mass):
    """Mirror of residual_noise::fit_working_variance (no floor)."""
    v = np.zeros(len(proxy))
    for typ in (False, True):
        idx = np.flatnonzero(stay == typ)
        if len(idx) == 0: continue
        lr = midrank(lev[idx])
        if not typ:
            mr = midrank(mass[idx]); B = np.column_stack([np.ones(len(idx)), lr, mr, lr ** 2, mr ** 2]); avail = 5
        else:
            B = np.column_stack([np.ones(len(idx)), lr, lr ** 2]); avail = 3
        k = int(np.clip(len(idx) // 100, 1, avail)); B = B[:, :k]
        keepc = []
        for c in range(k):
            Q = B[:, keepc + [c]]
            if np.linalg.matrix_rank(Q.T @ Q, tol=1e-10 * np.linalg.norm(Q.T @ Q)) == len(keepc) + 1: keepc.append(c)
        B = B[:, keepc]
        coef = np.linalg.lstsq(B.T @ B, B.T @ proxy[idx], rcond=None)[0]
        v[idx] = B @ coef
    return v


class SymDesign(Design):
    """Design whose firm system is factored once with a symmetric minimum-degree ordering (less fill)."""
    def factor(self):
        if self._lu is None:
            from scipy.sparse.linalg import splu
            t0 = time.time()
            self._lu = splu(self.L[:-1, :-1].tocsc(), permc_spec='MMD_AT_PLUS_A', diag_pivot_thresh=0.0, options=dict(SymmetricMode=True))
            if self.nJ > 20000: print(f'factor {time.time() - t0:.0f}s, nnz(L) {self._lu.L.nnz}', flush=True)
        return self._lu


def psd(S):
    w, U = np.linalg.eigh(0.5 * (S + S.T)); return (U * np.maximum(w, 0)) @ U.T


class Panel:
    """Person-year panel: worker, firm, year (integer codes), outcome; workers with one firm are stayers."""
    def __init__(self, worker, firm, year, y=None):
        _, self.w = np.unique(worker, return_inverse=True)
        _, self.f = np.unique(firm, return_inverse=True)
        self.year = np.asarray(year); self.y = None if y is None else np.asarray(y, float)
        self.nW = self.w.max() + 1; self.nJ = self.f.max() + 1; self.n = len(self.w)
        key = self.w.astype(np.int64) * self.nJ + self.f
        uk, self.match = np.unique(key, return_inverse=True)            # match id per person-year
        self.match_w = (uk // self.nJ).astype(np.int64); self.match_f = (uk % self.nJ).astype(np.int64)
        nfirm = np.bincount(self.match_w, minlength=self.nW)
        self.stay_w = nfirm == 1
        self.T_w = np.bincount(self.w, minlength=self.nW)


def build_design(P, gap=None):
    """Rows of the pooled design.  gap=None: stayer person-years.  gap=G>=0: two stayer blocks of F=max(1,(T-G)//2)
    years (first F, last F; middle dropped).  gap='firstlast': F=1.  Returns (Design, info) where info has
    py_row (row of each person-year, -1 if dropped), py_w (aggregation weight 1/sqrt(F_row)), row arrays."""
    st_py = P.stay_w[P.w]
    # movers: one row per match
    mv_match = ~P.stay_w[P.match_w]
    mrow = -np.ones(len(P.match_w), np.int64); mrow[mv_match] = np.arange(mv_match.sum())
    n_m = int(mv_match.sum())
    Fm = np.bincount(P.match[~st_py], minlength=len(P.match_w))[mv_match].astype(float)
    row_w = [P.match_w[mv_match]]; row_f = [P.match_f[mv_match]]; row_F = [Fm]; row_st = [np.zeros(n_m, bool)]
    py_row = -np.ones(P.n, np.int64); py_row[~st_py] = mrow[P.match[~st_py]]
    # stayers
    sidx = np.flatnonzero(st_py)
    order = sidx[np.lexsort((P.year[sidx], P.w[sidx]))]                    # by worker, then year
    ws = P.w[order]; T = P.T_w[ws]
    first = np.r_[True, ws[1:] != ws[:-1]]; pos = np.arange(len(ws)) - np.maximum.accumulate(np.where(first, np.arange(len(ws)), 0))
    if gap is None:
        srow = n_m + np.arange(len(order)); py_row[order] = srow
        row_w.append(ws); row_f.append(P.f[order]); row_F.append(np.ones(len(order))); row_st.append(np.ones(len(order), bool))
    else:
        Fb = np.ones(len(ws), np.int64) if gap == 'firstlast' else np.maximum(1, (T - int(gap)) // 2)
        blk = np.where(pos < Fb, 0, np.where(pos >= T - Fb, 1, -1))
        swork = np.cumsum(first) - 1                                         # stayer index 0..nS-1
        keep = blk >= 0
        py_row[order[keep]] = n_m + 2 * swork[keep] + blk[keep]
        nS = swork.max() + 1 if len(swork) else 0
        w_s = ws[first]; f_s = P.f[order][first]; F_s = Fb[first].astype(float)
        row_w.append(np.repeat(w_s, 2)); row_f.append(np.repeat(f_s, 2)); row_F.append(np.repeat(F_s, 2)); row_st.append(np.ones(2 * nS, bool))
    w = np.concatenate(row_w); f = np.concatenate(row_f); F = np.concatenate(row_F); stay = np.concatenate(row_st)
    assert (np.unique(w) == np.arange(P.nW)).all() and (np.unique(f) == np.arange(P.nJ)).all()
    D = SymDesign(w, f, F, stay)
    valid = py_row >= 0
    py_w = np.zeros(P.n); py_w[valid] = 1.0 / np.sqrt(F[py_row[valid]])
    return D, dict(py_row=py_row, py_w=py_w, valid=valid)


def aggregate(info, n_rows, x_py):
    """Scaled row values from person-year values: sum over the row's person-years / sqrt(F)."""
    v = info['valid']
    return np.bincount(info['py_row'][v], weights=(x_py * info['py_w'])[v], minlength=n_rows)


class Kernel(AutoSE):
    """AutoSE's operators with given diagonals; skips the structured feature basis, which this prototype does not use."""
    def __init__(self, d, Pd, Bd):
        self.d = d; self.solves = 0; self.Pd, self.Bd = Pd, Bd; self.Md = 1.0 - Pd; self.dt = Bd / self.Md


def jla_diag(D, probes=200, seed=8675309, step=25):
    """Leverage and target diagonals from Rademacher probes (as in AutoSE's first pass)."""
    K = Kernel(D, np.zeros(D.n), np.zeros((3, D.n))); rng = np.random.default_rng(seed)
    a = np.zeros(D.n); b = np.zeros(D.n); Bd = np.zeros((3, D.n))
    for k0 in range(0, probes, step):
        kk = min(step, probes - k0)
        r = rng.choice([-1.0, 1.0], size=(D.n, kk)); pr = K.P(r)
        a += (pr ** 2).sum(1); b += ((r - pr) ** 2).sum(1)
        r2 = rng.choice([-1.0, 1.0], size=(D.n, kk))
        ya = D.X(*D.solve(*D.Ah_t(0, r2))); yp = D.X(*D.solve(*D.Ah_t(1, r2))); K.solves += 2 * kk
        Bd[0] += (ya ** 2).sum(1); Bd[1] += (yp ** 2).sum(1); Bd[2] += (ya * yp).sum(1)
    return a / (a + b), Bd / probes, K.solves


class LP:
    def __init__(self, D, probes=200, seed=8675309, exact_diag=None, verbose=False):
        self.D = D; t0 = time.time()
        if exact_diag is None:
            Pd, Bd, self.solves_pass1 = jla_diag(D, probes, seed)
        else:
            Pd, Bd = exact_diag[0].copy(), exact_diag[1].copy(); self.solves_pass1 = 0
        st = D.stay; N = D.F.sum()
        self.rows_w = np.bincount(D.w, minlength=D.nW)                       # rows per worker
        ng = D.nw                                                            # person-years per worker
        self.sw = np.flatnonzero(np.bincount(D.w[st], minlength=D.nW) > 0)   # stayer workers
        assert st.sum() == 0 or np.allclose(D.F[st] * self.rows_w[D.w[st]], ng[D.w[st]]), 'stayer rows must have equal mass'
        Agg = (ng / N) * (1 - ng / N)
        Pd[st] = D.F[st] / ng[D.w[st]]
        Bd[0][st] = D.F[st] * Agg[D.w[st]] / ng[D.w[st]] ** 2; Bd[1][st] = 0.0; Bd[2][st] = 0.0
        self.A = Kernel(D, Pd, Bd)
        self.r = 1.0 / self.A.Md
        R = self.rows_w[self.sw].astype(float)
        self.R = R; self.c0 = (D.F[st][np.r_[True, D.w[st][1:] != D.w[st][:-1]]] if False else None)
        Fs = ng[self.sw] / R
        self.c = Fs * Agg[self.sw] / ng[self.sw] ** 2 / (1 - 1 / R)         # pair weight, worker component
        self.sidx = -np.ones(D.nW, np.int64); self.sidx[self.sw] = np.arange(len(self.sw))
        self.st_rows = np.flatnonzero(st); self.st_s = self.sidx[D.w[self.st_rows]]
        self.seconds_setup = time.time() - t0
        if verbose: print(f'LP setup {self.seconds_setup:.1f}s, pass-1 solves {self.solves_pass1}', flush=True)

    # ---- basic pieces ----------------------------------------------------
    def fit(self, y):
        A, D = self.A, self.D
        z = A.H(y); a, p = A.beta(z); e, g = A.kernel(y, bv=(a, p))
        theta3 = D.theta(a, p) - (A.dt * z * e).sum(1)
        s = z * e * self.r; hc2 = e * e * self.r
        floor = 1e-8 * np.median(hc2)
        va = np.maximum(rust_working(s, A.Pd, D.stay, D.F), floor)
        return dict(theta=T34 @ theta3, theta3=theta3, alpha=a, psi=p, z=z, e=e, g=np.array(g), s=s, hc2=hc2, va=va, floor=floor)

    def stayer_stats(self, y, g):
        """Per stayer: rows R, sample variance q, k-statistics k3, k4 (nan when undefined), mean of g_t over its rows."""
        rows, s = self.st_rows, self.st_s; nS = len(self.sw); R = self.R
        ys = y[rows]; mean = np.bincount(s, ys, nS) / R; dev = ys - mean[s]
        S2 = np.bincount(s, dev ** 2, nS); S3 = np.bincount(s, dev ** 3, nS); S4 = np.bincount(s, dev ** 4, nS)
        q = S2 / (R - 1)
        with np.errstate(divide='ignore', invalid='ignore'):
            k3 = np.where(R >= 3, R ** 2 * (S3 / R) / ((R - 1) * (R - 2)), np.nan)
            k4 = np.where(R >= 4, R ** 2 * ((R + 1) * (S4 / R) - 3 * (R - 1) * (S2 / R) ** 2) / ((R - 1) * (R - 2) * (R - 3)), np.nan)
        mbar = np.array([np.bincount(s, g[t][rows], nS) / R for t in range(3)])
        return dict(q=q, k3=k3, k4=k4, mbar=mbar, mean=mean)

    def Ga(self, V, g):
        return np.array([[(V * g[t] * g[u]).sum() for u in range(3)] for t in range(3)])

    def pseudo(self, var, draws, rng, noise=False, parts=False):
        """Double sum tau (and the calibration noise N of the shipped estimator).  With parts=True also returns
        N restricted to mover rows and tau restricted to stayer rows (for the hybrid)."""
        A = self.A; n = self.D.n; mv = ~self.D.stay
        zeta = (np.sqrt(var) * rng.standard_normal((n, draws)).T).T
        mz, kz = A.kernel(zeta)
        tau = np.zeros((3, 3)); N = np.zeros((3, 3)); Nm = np.zeros((3, 3)); ts = np.zeros((3, 3))
        for t in range(3):
            for u in range(t, 3):
                tk = (var * kz[t].T).T * kz[u]
                tau[t, u] = tau[u, t] = tk.sum(0).mean()
                if noise or parts:
                    nk = (self.r * (mz ** 2).T).T * kz[t] * kz[u]
                    N[t, u] = N[u, t] = nk.sum(0).mean()
                    if parts:
                        Nm[t, u] = Nm[u, t] = nk[mv].sum(0).mean(); ts[t, u] = ts[u, t] = tk[~mv].sum(0).mean()
        if parts: return tau, N, Nm, ts
        return tau, N

    def adjust(self, ss, kappa=3.0, free=False):
        """3x3 adjustment sum_U T_U q_U^2/(1+nu_U); only the worker component has non-zero stayer pair weights."""
        R, c, q = self.R, self.c, ss['q']
        TU = c ** 2 * (4 * (2 - R) + 2 * (kappa - 3) * (R - 1))
        base = TU * q ** 2 / (1 + 2 / (R - 1) + (kappa - 3) / R)
        adj = np.zeros((3, 3))
        if not free:
            adj[0, 0] = base.sum(); return adj
        hi = R >= 4; adj[0, 0] = base[~hi].sum()
        Rh, ch, qh, k3, k4, mb = R[hi], c[hi], q[hi], ss['k3'][hi], ss['k4'][hi], ss['mbar'][:, hi]
        adj[0, 0] += (ch ** 2 * (4 * (2 - Rh) * (Rh - 1) / (Rh + 1) * (qh ** 2 - k4 / Rh) + 2 * (Rh - 1) * k4)).sum()
        cvec = np.array([ch, np.zeros_like(ch), np.zeros_like(ch)])
        est = np.array([mb[t] * k3 - cvec[t] * (Rh - 1) * k4 / Rh for t in range(3)])     # unbiased for m_t mu3
        for t in range(3):
            for u in range(3):
                adj[t, u] += (4 * (Rh - 1) * (cvec[t] * est[u] + cvec[u] * est[t])).sum()
        return adj

    def stayer_var(self, base, ss):
        """Row variances: base everywhere, the stayer's own sample variance on stayer rows."""
        V = np.asarray(base, float).copy(); V[self.st_rows] = ss['q'][self.st_s]; return V

    def estimate(self, y, draws=16, dseed=1, vtrue=None, kappas=(3.0,), which=('shipped', 'model', 'prop', 'propfree', 'transfer'), index=None):
        o = self.fit(y); g = o['g']; rng = np.random.default_rng(dseed + 7919)
        ss = self.stayer_stats(y, g); out = dict(theta=o['theta'], fit=o, ss=ss)
        if 'shipped' in which or 'model' in which:
            tau, N = self.pseudo(o['va'], draws, rng, noise=True)
            out['model'] = psd(4 * self.Ga(o['va'], g) - 2 * tau)
            out['shipped'] = psd(4 * (self.Ga(o['hc2'], g) - N) + 2 * tau)
        if 'final' in which:
            # the estimator of the note (tr), its version with the movers' true variances (or) and its version with
            # the profile evaluated at given effects instead of the estimated ones (trx)
            for name, base in (('tr', lambda: self.working_transfer(o, ss)), ('or', (lambda: vtrue) if vtrue is not None else None),
                               ('trx', (lambda: self.working_transfer(o, ss, index)) if index is not None else None)):
                if base is None: continue
                Vx = self.stayer_var(base(), ss)
                if name != 'or': out['diag_' + name] = dict(self.last_transfer)
                tau, _ = self.pseudo(np.maximum(Vx, 1e-300), draws, rng)
                out[name] = psd(4 * self.Ga(Vx, g) - 2 * tau - self.adjust(ss, 3.0))
            return out
        mv = ~self.D.stay; stm = self.D.stay.astype(float)
        for name, base in (('prop', o['va']), ('propo', vtrue)):
            if base is None or not any(w.startswith('prop') for w in which): continue
            Vx = self.stayer_var(base, ss)
            tau, _, Nm, ts = self.pseudo(np.maximum(Vx, 1e-300), draws, rng, parts=True)
            raw = 4 * self.Ga(Vx, g) - 2 * tau
            out[name + '_noadj'] = psd(raw)
            for k in kappas:
                out[f'{name}_k{k:g}'] = psd(raw - self.adjust(ss, k))
            out[name + 'free'] = psd(raw - self.adjust(ss, 3.0, free=True))
            # hybrid: movers' signal from their own residuals with the shipped calibration, stayers by repeat measurement
            hyb = 4 * (self.Ga(o['hc2'] * mv, g) - Nm) + 4 * self.Ga(Vx * stm, g) + 2 * tau - 4 * ts - self.adjust(ss, 3.0)
            out['hyb' if name == 'prop' else 'hybo'] = psd(hyb)
        if 'transfer' in which:
            vt = self.working_transfer(o, ss); out['vt_profile_r2'] = self.last_profile_r2
            Vx = self.stayer_var(vt, ss)
            tau, _, Nm, ts = self.pseudo(np.maximum(Vx, 1e-300), draws, rng, parts=True)
            out['propt'] = psd(4 * self.Ga(Vx, g) - 2 * tau - self.adjust(ss, 3.0))
            out['hybt'] = psd(4 * (self.Ga(o['hc2'] * mv, g) - Nm) + 4 * self.Ga(Vx * stm, g) + 2 * tau - 4 * ts - self.adjust(ss, 3.0))
        return out

    def working_transfer(self, o, ss, index=None):
        """Transfer model for movers' variances.  A profile in the estimated person and firm effects is learned from
        the stayers' repeat-measurement variances (unbiased for each stayer's variance), by least squares of q_U on a
        quadratic in the standardised effects; movers' variances are the working model's basis multiplied by that
        profile, scaled by least squares of the movers' KSS leave-out products.  Assumption: given the estimated
        effects, movers' and stayers' variances share the profile, up to the working model's leverage terms."""
        D = self.D; a, p = (o['alpha'], o['psi']) if index is None else index
        w_st = self.sw; f_st = np.zeros(len(self.sw), np.int64); f_st[self.st_s] = D.f[self.st_rows]
        def basis(av, pv):
            return np.column_stack([np.ones(len(av)), av, av ** 2, pv, pv ** 2, av * pv])
        wts = np.bincount(D.w, D.F, D.nW); ma = np.average(a, weights=wts); sa = np.sqrt(np.average((a - ma) ** 2, weights=wts))
        fw = np.bincount(D.f, D.F, D.nJ); mp = np.average(p, weights=fw); sp = np.sqrt(np.average((p - mp) ** 2, weights=fw))
        Bs = basis((a[w_st] - ma) / sa, (p[f_st] - mp) / sp); wt = self.R - 1.0          # weight: degrees of freedom
        coef = np.linalg.lstsq(Bs.T @ (Bs * wt[:, None]), Bs.T @ (wt * ss['q']), rcond=None)[0]
        fit_s = Bs @ coef; self.last_profile_r2 = float(1 - np.average((ss['q'] - fit_s) ** 2, weights=wt) / np.average((ss['q'] - np.average(ss['q'], weights=wt)) ** 2, weights=wt))
        mv = np.flatnonzero(~D.stay)
        raw = basis((a[D.w[mv]] - ma) / sa, (p[D.f[mv]] - mp) / sp) @ coef; pfloor = 1e-3 * np.average(ss['q'], weights=wt)
        prof = np.maximum(raw, pfloor)
        prof = prof / prof.mean()
        lr = midrank(self.A.Pd[mv]); mr = midrank(D.F[mv])
        Bm = np.column_stack([np.ones(len(mv)), lr, mr, lr ** 2, mr ** 2]) * prof[:, None]
        c = np.linalg.lstsq(Bm.T @ Bm, Bm.T @ o['s'][mv], rcond=None)[0]
        fitm = Bm @ c
        self.last_transfer = dict(coef=coef.tolist(), scale=c.tolist(), r2=self.last_profile_r2, std=[float(ma), float(sa), float(mp), float(sp)],
                                  profile_floored=float((raw < pfloor).mean()), variance_floored=float((fitm < o['floor']).mean()),
                                  profile_min=float(prof.min()), profile_max=float(prof.max()))
        v = o['va'].copy(); v[mv] = np.maximum(fitm, o['floor']); return v

    def working_m(self, o, g):
        """Working variance for movers fitted on the narrow basis plus the six products g_t g_u (standardised)."""
        D = self.D; mv = np.flatnonzero(~D.stay); s = o['s']
        lr = midrank(self.A.Pd[mv]); mr = midrank(D.F[mv])
        cols = [np.ones(len(mv)), lr, mr, lr ** 2, mr ** 2]
        for t in range(3):
            for u in range(t, 3):
                x = g[t][mv] * g[u][mv]; sd = x.std(); cols.append(x / sd if sd > 0 else x)
        B = np.column_stack(cols)
        coef = np.linalg.lstsq(B.T @ B, B.T @ s[mv], rcond=None)[0]
        v = o['va'].copy(); v[mv] = np.maximum(B @ coef, o['floor']); return v

    def theory(self, mu_rows, v_rows, draws=128, seed=99):
        """Variance of theta-hat for independent row errors with variances v_rows (kurtosis terms on the movers'
        small diagonal ignored): 4 sum v m^2 + 2 sum K^2 v v."""
        rng = np.random.default_rng(seed); _, g = self.A.kernel(mu_rows); g = np.array(g)
        taus = [self.pseudo(v_rows, min(32, draws), rng)[0] for _ in range(max(1, draws // 32))]
        tau = np.mean(taus, 0)
        V3 = 4 * self.Ga(v_rows, g) + 2 * tau
        return dict(V3=V3, V4=np.diag(T34 @ V3 @ T34.T), signal4=np.diag(T34 @ (4 * self.Ga(v_rows, g)) @ T34.T), m=g)


def synth_panel(nJ=60, nW=1500, years=6, p_move=0.10, seed=0, minT=2):
    """Synthetic unbalanced person-year panel, pruned to a leave-one-match-out connected set."""
    rng = np.random.default_rng(seed)
    size = np.exp(rng.standard_normal(nJ)); pj = size / size.sum()
    w, f, yr = [], [], []
    for i in range(nW):
        T = rng.integers(minT, years + 1); t0 = rng.integers(0, years - T + 1); j = rng.choice(nJ, p=pj)
        for t in range(t0, t0 + T):
            if t > t0 and rng.random() < p_move:
                j = rng.choice(nJ, p=pj)
            w.append(i); f.append(j); yr.append(t)
    w, f, yr = np.array(w), np.array(f), np.array(yr)
    while True:
        P = Panel(w, f, yr)
        st_py = P.stay_w[P.w]
        mv = ~P.stay_w[P.match_w]
        Fm = np.bincount(P.match, minlength=len(P.match_w)).astype(float)
        # rows for pruning: mover matches + stayer person-years
        rw = np.r_[P.match_w[mv], P.w[st_py]]; rf = np.r_[P.match_f[mv], P.f[st_py]]
        rF = np.r_[Fm[mv], np.ones(st_py.sum())]; rs = np.r_[np.zeros(mv.sum(), bool), np.ones(st_py.sum(), bool)]
        _, _, _, _, keep, _ = prune(rw, rf, rF, rs)
        keep_match = np.zeros(len(P.match_w), bool); keep_match[np.flatnonzero(mv)[keep[:mv.sum()]]] = True
        keep_py = np.where(st_py, False, keep_match[P.match])
        keep_py[np.flatnonzero(st_py)[keep[mv.sum():]]] = True
        cnt = np.bincount(P.w[keep_py], minlength=P.nW); keep_py &= cnt[P.w] >= 2
        # a mover left with a single match and one person-year is dropped; with several person-years it becomes a stayer
        if keep_py.all(): return P
        w, f, yr = P.w[keep_py], P.f[keep_py], P.year[keep_py]

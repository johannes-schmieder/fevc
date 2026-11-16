"""Simulation on a fixed longer-panel design: the effects fitted on the data are the truth, errors are redrawn.

Usage:
  lp_mc.py prepare <design-dir> <design> [probes]         cache diagonals, truth and scenario inputs (once per design)
  lp_mc.py theory  <design-dir> <out-root> <manifest> <cell-index>
  lp_mc.py task    <design-dir> <out-root> <manifest> <task-index>
The manifest (JSON) lists cells {scenario, design, reps_per_task, tasks, draws} and scenario definitions; task
indices enumerate (cell, task) pairs in manifest order.  Every task writes one result.json in its own directory,
atomically.  Error draws are addressed by (scenario seed, design-independent person-year stream, task, rep), so
results do not depend on scheduling order."""
import sys, os, json, time, numpy as np
from lp6 import Panel, build_design, aggregate, LP, T34

DESIGNS = {'default': None, 'gap0': 0, 'gap1': 1, 'gap2': 2, 'firstlast': 'firstlast'}
EST_DEFAULT = ('shipped', 'model', 'prop_noadj', 'prop_k3', 'propfree', 'propo_noadj', 'propo_k3', 'propofree', 'hyb', 'hybo', 'propt', 'hybt')


def load_panel(ddir):
    z = np.load(ddir + '/panel.npz'); return Panel(z['w'], z['f'], z['year'], z['y'])


def engine(ddir, P, design, probes=200):
    D, info = build_design(P, DESIGNS[design])
    dfile = ddir + f'/diag_{design}_{probes}.npz'
    if os.path.exists(dfile):
        dz = np.load(dfile); E = LP(D, exact_diag=(dz['Pd'], dz['Bd']))
    else:
        E = LP(D, probes=probes); tmp = dfile + '.tmp.npz'; np.savez(tmp, Pd=E.A.Pd, Bd=E.A.Bd); os.replace(tmp, dfile)
    return D, info, E


def prepare(ddir, design, probes=200):
    P = load_panel(ddir); D, info, E = engine(ddir, P, design, probes)
    tfile = ddir + '/truth.npz'
    if design == 'default' and not os.path.exists(tfile):
        y = aggregate(info, D.n, P.y); f = E.fit(y)
        v0_py = f['va'][info['py_row']]                           # working variance of the row, per person-year
        tmp = tfile + '.tmp.npz'; np.savez(tmp, alpha=f['alpha'], psi=f['psi'], v0_py=v0_py); os.replace(tmp, tfile)
    print('prepared', design, 'rows', D.n, flush=True)


def stayer_order(P):
    """Stayer person-years sorted by worker then year; first-of-worker flags."""
    sidx = np.flatnonzero(P.stay_w[P.w]); o = sidx[np.lexsort((P.year[sidx], P.w[sidx]))]
    first = np.r_[True, P.w[o][1:] != P.w[o][:-1]]
    return o, first


def stayer_deciles(P, a):
    """Decile cut points of the person effect among stayers, and each worker's decile (0..9)."""
    cuts = np.quantile(a[P.stay_w], np.arange(1, 10) / 10); return cuts, np.searchsorted(cuts, a)


def decile_profile(P, a):
    """Stayers' sample variance of the observed outcome by decile of the person effect, relative to its overall
    level (weighted by degrees of freedom).  A property of the data, not of a scenario."""
    st_py = P.stay_w[P.w]; T = P.T_w.astype(float)
    mean = np.bincount(P.w, P.y * st_py, P.nW) / np.maximum(T, 1)
    dev2 = np.bincount(P.w, (P.y - mean[P.w]) ** 2 * st_py, P.nW)
    _, dec = stayer_deciles(P, a); sw = P.stay_w & (T >= 2)
    num = np.bincount(dec[sw], dev2[sw], 10); den = np.bincount(dec[sw], T[sw] - 1, 10)
    return (num / den) / (num.sum() / den.sum())


def profile_factor(P, sc, tr):
    """Person-year variance multiplier of a scenario (mean one over the person-years of stayers and of movers
    separately, except 'expo', which is normalised over all person-years)."""
    a, psi = tr['alpha'], tr['psi']; st_py = P.stay_w[P.w]; fac = np.ones(P.n)
    def norm_groups(f):
        f = f.copy()
        for gmask in (st_py, ~st_py):
            if gmask.any(): f[gmask] /= f[gmask].mean()
        return f
    if sc.get('grad', 0) > 0 or sc.get('grad_mover') is not None:
        ac = a - np.average(a[P.w]); s2 = np.average(ac[P.w] ** 2)
        gw = np.where(P.stay_w, sc.get('grad', 0.0), sc.get('grad_mover', sc.get('grad', 0.0)))
        f = (1 + gw * ac ** 2 / s2)[P.w]
        fac *= norm_groups(f) if sc.get('grad_mover') is not None else f / f.mean()
    if sc.get('decile'):
        _, dec = stayer_deciles(P, a); prof = decile_profile(P, a)[dec]
        if sc.get('decile_movers') is not None:                       # movers follow their own profile (ten multipliers, by the same deciles)
            prof = np.where(P.stay_w, prof, np.asarray(sc['decile_movers'], float)[dec])
        fac *= norm_groups(prof[P.w])
    if sc.get('expo') is not None:
        u = a[P.w] + psi[P.f]; u = (u - u.mean()) / u.std(); f = np.exp(sc['expo'] * u); fac *= f / f.mean()
    return fac


def scenario_inputs(P, sc, tr, mbarT_w, sto=None):
    """Person-year variances and fixed sign pattern for a scenario (deterministic given the scenario seed)."""
    rng = np.random.default_rng([sc['seed'], 0])
    v = tr['v0_py'].copy(); a = tr['alpha']
    if sc['kind'] == 'rw':                                         # stayers: marginal variance of random walk plus noise
        o, first = sto; pos = np.arange(len(o)) - np.maximum.accumulate(np.where(first, np.arange(len(o)), 0))
        v[o] = sc['se2'] + sc['su2'] * pos
    v *= profile_factor(P, sc, tr)
    if sc.get('mix', 0) > 0:
        shape = 1.0 / sc['mix'] ** 2; lam = rng.gamma(shape, 1.0 / shape, P.nW); v *= lam[P.w]
    sign_py = None
    if sc['kind'] == 'skew':
        sign_py = -np.sign(mbarT_w)[P.w]; sign_py[sign_py == 0] = 1.0
    return v, sign_py


def draw_errors(P, sc, v_py, sign_py, sto, rng):
    n = P.n; kind = sc['kind']; st_py = P.stay_w[P.w]
    if kind == 'gauss':
        u = rng.standard_normal(n)
    elif kind == 'fat':                                            # symmetric, kurtosis 3 + 6/shape
        k = sc['shape']; u = (rng.gamma(k, 1.0, n) - k) / np.sqrt(k); u *= np.where(rng.random(n) < 0.5, 1.0, -1.0)
    elif kind == 'skew':                                           # stayers signed against the wage level with prob (1+align)/2
        k = sc['shape']; u = (rng.gamma(k, 1.0, n) - k) / np.sqrt(k); al = sc.get('align', 1.0)
        sg = np.where(rng.random(n) < (1 + al) / 2, sign_py, -sign_py)
        u *= np.where(st_py, sg, np.where(rng.random(n) < 0.5, 1.0, -1.0))
    elif kind == 'ar':                                             # stayers AR(1) across their consecutive person-years
        u = rng.standard_normal(n); o, first = sto; rho = sc['rho']; x = u[o].copy(); prev = 0.0
        # positions within stayer: iterate by position (at most the panel length)
        pos = np.arange(len(o)) - np.maximum.accumulate(np.where(first, np.arange(len(o)), 0))
        for p in range(1, int(pos.max()) + 1):
            m = np.flatnonzero(pos == p); x[m] = rho * x[m - 1] + np.sqrt(1 - rho ** 2) * x[m]
        u[o] = x
    elif kind == 'rw':                                             # stayers: random walk from the first year plus transitory noise
        e = np.sqrt(v_py) * rng.standard_normal(n); o, first = sto
        pos = np.arange(len(o)) - np.maximum.accumulate(np.where(first, np.arange(len(o)), 0))
        W = np.sqrt(sc['su2']) * rng.standard_normal(len(o)); W[first] = 0.0
        for p in range(1, int(pos.max()) + 1):
            mm = np.flatnonzero(pos == p); W[mm] += W[mm - 1]
        e[o] = W + np.sqrt(sc['se2']) * rng.standard_normal(len(o))
        return e
    else:
        raise ValueError(kind)
    return np.sqrt(v_py) * u


def context(ddir, manifest, cell):
    c = manifest['cells'][cell]; sc = dict(manifest['scenarios'][c['scenario']])
    P = load_panel(ddir); D, info, E = engine(ddir, P, c['design'], manifest.get('probes', 200))
    tr = np.load(ddir + '/truth.npz'); mu = D.X(tr['alpha'], tr['psi'])
    _, g = E.A.kernel(mu); gT = T34[3] @ np.array(g)
    mbarT_w = np.bincount(D.w, gT * D.F, D.nW) / np.bincount(D.w, D.F, D.nW)   # mass-weighted mean of m_T over the worker's rows
    sto = stayer_order(P) if sc['kind'] in ('ar', 'rw') else None
    v_py, sign_py = scenario_inputs(P, sc, tr, mbarT_w, sto)
    v_rows = aggregate(info, D.n, v_py * info['py_w'])            # variance of the scaled row under independent errors
    return c, sc, P, D, info, E, tr, mu, v_py, sign_py, v_rows


def write_json(path, obj):
    os.makedirs(os.path.dirname(path), exist_ok=True); tmp = path + '.tmp'
    json.dump(obj, open(tmp, 'w')); os.replace(tmp, path)


def sigma_rows(P, info, D, v_py, sc, sto):
    """Returns a function applying the covariance matrix of the row errors: diagonal for independent errors;
    for kind 'ar', stayers' person-year errors are AR(1) with stationary variance v_py."""
    v, row, w = info['valid'], info['py_row'], info['py_w']
    if sc['kind'] == 'rw':                                         # stayers: se2 I + su2 min(s, t), positions from 0
        o, first = sto; pos = np.arange(len(o)) - np.maximum.accumulate(np.where(first, np.arange(len(o)), 0))
        npos = int(pos.max()) + 1; idx = [np.flatnonzero(pos == p) for p in range(npos)]
        last = np.r_[first[1:], True]; se2, su2 = sc['se2'], sc['su2']; st_py = P.stay_w[P.w]
        def cov_st(xs):
            tail = xs.copy()                                       # tail_p = sum_{t >= p} x_t within the stayer
            for p in range(npos - 2, -1, -1):
                m = idx[p][~last[idx[p]]]; tail[m] += tail[m + 1]
            out = np.zeros_like(xs)                                # out_s = sum_{p=1}^{s} tail_p
            for p in range(1, npos): out[idx[p]] = out[idx[p] - 1] + tail[idx[p]]
            return se2 * xs + su2 * out
        def apply_rw(x):
            xp = np.zeros((P.n,) + x.shape[1:]); xp[v] = (x[row[v]].T * w[v]).T
            y = (v_py * xp.T).T                                    # movers: independent with variance v_py
            y[o] = cov_st(xp[o])
            if x.ndim == 1: return np.bincount(row[v], (y * w)[v], D.n)
            return np.column_stack([np.bincount(row[v], (y[:, j] * w)[v], D.n) for j in range(x.shape[1])])
        return apply_rw
    if sc['kind'] != 'ar':
        v_rows = aggregate(info, D.n, v_py * w)
        return lambda x: (v_rows * x.T).T
    o, first = sto; rho = sc['rho']; pos = np.arange(len(o)) - np.maximum.accumulate(np.where(first, np.arange(len(o)), 0))
    npos = int(pos.max()) + 1; idx = [np.flatnonzero(pos == p) for p in range(npos)]
    last = np.r_[first[1:], True]
    def corr(xs):                                                   # xs: stayer person-years in sorted order; returns R(rho) xs
        f = xs.copy(); b = xs.copy()
        for p in range(1, npos): f[idx[p]] += rho * f[idx[p] - 1]
        for p in range(npos - 2, -1, -1):
            m = idx[p][~last[idx[p]]]; b[m] += rho * b[m + 1]
        return f + b - xs
    sd = np.sqrt(v_py)
    def apply(x):
        xp = np.zeros((P.n,) + x.shape[1:]); xp[v] = (x[row[v]].T * w[v]).T          # Agg' x
        z = (sd * xp.T).T; zs = z[o]; z[o] = corr(zs); y = (sd * z.T).T              # Sigma_py
        if x.ndim == 1: return np.bincount(row[v], (y * w)[v], D.n)
        return np.column_stack([np.bincount(row[v], (y[:, j] * w)[v], D.n) for j in range(x.shape[1])])
    return apply


def theory(ddir, oroot, mfile, cell):
    manifest = json.load(open(mfile)); c, sc, P, D, info, E, tr, mu, v_py, sign_py, v_rows = context(ddir, manifest, cell)
    sto = stayer_order(P) if sc['kind'] in ('ar', 'rw') else None; Sig = sigma_rows(P, info, D, v_py, sc, sto)
    _, g = E.A.kernel(mu); g = np.array(g); draws = manifest.get('theory_draws', 128); rng = np.random.default_rng([sc['seed'], 2])
    G = np.array([[(g[t] * Sig(g[u])).sum() for u in range(3)] for t in range(3)])
    tau = np.zeros((3, 3)); bias = np.zeros(3); nb = max(1, draws // 16)
    for b in range(nb):
        if sc['kind'] in ('ar', 'rw'):
            Z = np.column_stack([aggregate(info, D.n, draw_errors(P, sc, v_py, sign_py, sto, rng)) for _ in range(16)])
        else:
            Z = (np.sqrt(v_rows) * rng.standard_normal((D.n, 16)).T).T
        _, kz = E.A.kernel(Z)
        for t in range(3):
            bias[t] += (Z * kz[t]).sum(0).mean() / nb
            SK = Sig(kz[t])
            for u in range(3): tau[t, u] += (SK * kz[u]).sum(0).mean() / nb
    tau = 0.5 * (tau + tau.T); V3 = 4 * G + 2 * tau
    # exact bias from the stayer blocks: c_g (1' Sigma_g 1 - tr Sigma_g), worker component only
    ones = np.zeros(D.n); ones[E.st_rows] = 1.0; s1 = Sig(ones)
    # diagonal of the row covariance on stayer rows: apply it to the indicator of each stayer's r-th row
    ws = D.w[E.st_rows]; firstrow = np.r_[True, ws[1:] != ws[:-1]]
    rpos = np.arange(len(ws)) - np.maximum.accumulate(np.where(firstrow, np.arange(len(ws)), 0))
    diag_st = np.zeros(len(ws))
    for r in range(int(rpos.max()) + 1):
        ind = np.zeros(D.n); sel = E.st_rows[rpos == r]; ind[sel] = 1.0; diag_st[rpos == r] = Sig(ind)[sel]
    tot = np.bincount(E.st_s, s1[E.st_rows], len(E.sw)); trc = np.bincount(E.st_s, diag_st, len(E.sw))
    bias_exact = np.array([(E.c * (tot - trc)).sum(), 0.0, 0.0])
    rw_shift = None
    if sc['kind'] == 'rw':                                         # extra person-effect variance if the estimand is the worker's average component
        R = E.R; n_g = D.nw[E.sw]; F = n_g / R; N = D.F.sum()
        # variance of the mean of the retained person-years is not tracked by block; report the default-design value only
        rw_shift = float((n_g / N * sc['su2'] * (R - 1) * (2 * R - 1) / (6 * R)).sum()) if c['design'] == 'default' else None
    # realised regime of the cell (a scenario name is not evidence): variance ratio to the baseline by decile of the
    # person effect for stayers and movers, their dispersion, and the movers' share of the first term
    st_py = P.stay_w[P.w]; _, dec = stayer_deciles(P, tr['alpha']); ratio = v_py / tr['v0_py']; dpy = dec[P.w]
    g4 = T34 @ g; lead = v_rows * g4 ** 2
    regime = dict(ratio_by_decile_stayers=[float(ratio[st_py & (dpy == d)].mean()) for d in range(10)],
                  ratio_by_decile_movers=[float(ratio[~st_py & (dpy == d)].mean()) for d in range(10)],
                  cv_ratio_stayers=float(ratio[st_py].std() / ratio[st_py].mean()), cv_ratio_movers=float(ratio[~st_py].std() / ratio[~st_py].mean()),
                  mean_var_stayers=float(v_py[st_py].mean()), mean_var_movers=float(v_py[~st_py].mean()),
                  mover_share_first_term=(lead[:, ~D.stay].sum(1) / lead.sum(1)).tolist())
    out = dict(cell=cell, scenario=c['scenario'], design=c['design'], rows=int(D.n), person_years_kept=int(info['valid'].sum()), regime=regime,
               theta_true=(T34 @ D.theta(tr['alpha'], tr['psi'])).tolist(), V_indep=np.diag(T34 @ V3 @ T34.T).tolist(),
               signal=np.diag(T34 @ (4 * G) @ T34.T).tolist(), independent=True, bias_exact=(T34 @ bias_exact).tolist(),
               rw_avg_shift=rw_shift,
               bias_draws=(T34 @ bias).tolist(), kind=sc['kind'])
    write_json(f"{oroot}/{c['scenario']}__{c['design']}/theory.json", out); print(out, flush=True)


def task(ddir, oroot, mfile, tindex):
    manifest = json.load(open(mfile)); pairs = [(ci, t) for ci, c in enumerate(manifest['cells']) for t in range(c['tasks'])]
    cell, t = pairs[tindex]; c, sc, P, D, info, E, tr, mu, v_py, sign_py, v_rows = context(ddir, manifest, cell)
    odir = f"{oroot}/{c['scenario']}__{c['design']}/task-{t:03d}"; rfile = odir + '/result.json'
    if os.path.exists(rfile): raise SystemExit(f'{rfile} exists; refusing to overwrite')
    sto = stayer_order(P) if sc['kind'] in ('ar', 'rw') else None
    EST = tuple(manifest.get('estimators', EST_DEFAULT)); which = tuple(manifest.get('which', ('shipped', 'model', 'prop', 'propfree', 'transfer')))
    index = (tr['alpha'], tr['psi']) if 'trx' in EST else None; dg = {}
    th, V, fails = [], {e: [] for e in EST}, 0; t0 = time.time()
    for r in range(c['reps_per_task']):
        rng = np.random.default_rng([sc['seed'], 1, t, r])
        err = aggregate(info, D.n, draw_errors(P, sc, v_py, sign_py, sto, rng))
        try:
            o = E.estimate(mu + err, draws=c.get('draws', 16), dseed=1000 * t + r, vtrue=v_rows, which=which, index=index)
        except Exception as ex:                                    # counted, never silently dropped
            fails += 1; print('rep failed', r, repr(ex), flush=True); continue
        th.append(o['theta'].tolist())
        for e in EST: V[e].append(np.diag(T34 @ o[e] @ T34.T).tolist())
        for k, x in o.get('diag_cells', {}).items():
            if isinstance(x, float): dg.setdefault('cells:' + k, []).append(x)
        for e in EST:
            for k, x in o.get('diag_' + e, {}).items():
                if k in ('r2', 'profile_floored', 'variance_floored', 'profile_min', 'profile_max'): dg.setdefault(e + ':' + k, []).append(x)
        if r == 0: print(f'task {tindex} cell {cell} rep 0 done in {time.time() - t0:.0f}s', flush=True)
    out = dict(schema='LP-MC-TASK-V1', cell=cell, scenario=c['scenario'], design=c['design'], task=t, reps_requested=c['reps_per_task'],
               reps_done=len(th), failures=fails, theta=th, V=V, seconds=time.time() - t0, rows=int(D.n),
               diag={k: [float(np.mean(x)), float(np.min(x)), float(np.max(x))] for k, x in dg.items()},
               job_id=os.environ.get('JOB_ID', ''), host=os.environ.get('HOSTNAME', ''))
    write_json(rfile, out); print('wrote', rfile, f'{out["seconds"]:.0f}s', flush=True)


if __name__ == '__main__':
    mode = sys.argv[1]
    if mode == 'prepare': prepare(sys.argv[2], sys.argv[3], int(sys.argv[4]) if len(sys.argv) > 4 else 200)
    elif mode == 'theory': theory(sys.argv[2], sys.argv[3], sys.argv[4], int(sys.argv[5]))
    elif mode == 'task': task(sys.argv[2], sys.argv[3], sys.argv[4], int(sys.argv[5]))
    else: raise SystemExit('unknown mode')

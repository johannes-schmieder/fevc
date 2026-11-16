"""Oracle error of the models for movers, per scenario (no outcome draws).

The model's steps are run with the TRUE variances as outcomes and the TRUE effects as (fixed) characteristics.
That gives v*_i on movers' rows, Delta_i = v*_i - sigma_i^2, and
    b(Delta) = 4 sum_R m_t m_u Delta_i - 2 sum_R sum_R C_t,il C_u,il Delta_i Delta_l ,
the term by which the variance estimate is off on average when movers' rows receive v* (Proposition 1). It is
reported relative to V[theta-hat] from the scenario's theory.json. The double sum uses
B(d, d) = 2 B(a, a) + 2 B(b, b) - B(a + b, a + b) with a = max(d, 0), b = max(-d, 0) and common draws.

Usage: lp_bdelta.py <design-dir> <theory-root> <out-root> <manifest> <cell-index> [draws]
Writes <out-root>/<scenario>__<design>/bdelta.json atomically; refuses to overwrite."""
import sys, os, json, time, numpy as np
from lp6 import T34, midrank
from lp_mc6 import context, write_json

MODELS = ('tw', 'twl', 'tlw', 'dq')


def oracle_fits(E, D, tr, v_rows, cap=3.0):
    """v* on movers' rows for each model, from true variances at true effects; mirrors LP.round6_models."""
    mv = np.flatnonzero(~D.stay); s = v_rows[mv]
    at, pt = E.std_effects(dict(alpha=tr['alpha'], psi=tr['psi']), cap)
    basis = lambda av, pv: np.column_stack([np.ones(len(av)), av, av ** 2, pv, pv ** 2, av * pv])
    ls = lambda X, yv: np.linalg.lstsq(X.T @ X, X.T @ yv, rcond=None)[0]
    lr = midrank(E.A.Pd[mv]); mr = midrank(D.F[mv]); Z = np.column_stack([np.ones(len(mv)), lr, mr, lr ** 2, mr ** 2])
    raw = Z @ ls(Z, s); fs = 0.05 * s.mean(); sc = np.maximum(raw, fs)
    Bm = basis(at[D.w[mv]], pt[D.f[mv]])
    w_st = E.sw; f_st = np.zeros(len(w_st), np.int64); f_st[E.st_s] = D.f[E.st_rows]; wt = E.R - 1.0
    sig_g = np.bincount(E.st_s, v_rows[E.st_rows], len(w_st)) / E.R                 # the stayer's variance
    spread = np.bincount(E.st_s, (v_rows[E.st_rows] - sig_g[E.st_s]) ** 2, len(w_st))  # zero when A2 holds
    Bs = basis(at[w_st], pt[f_st]); pis = np.linalg.lstsq(Bs.T @ (Bs * wt[:, None]), Bs.T @ (wt * sig_g), rcond=None)[0]
    qbar = np.average(sig_g, weights=wt); rawp = Bm @ pis; pS = np.maximum(rawp, 1e-3 * qbar) / qbar
    out = {}; info = dict(scale_floored=float((raw < fs).mean()), profile_floored=float((rawp < 1e-3 * qbar).mean()),
                          max_within_stayer_spread=float(np.sqrt(spread.max()) / sig_g.mean()),
                          profile_min=float(pS.min()), profile_max=float(pS.max()), profile_sd_movers=float(pS.std()))
    out['tw'] = sc * pS * (s.sum() / (sc * pS).sum())
    X = np.column_stack([sc, sc * pS]); c = ls(X, s); out['twl'] = X @ c; info['load'] = [float(c[0]), float(c[1])]
    info['load_centre'] = float((sc ** 2 * pS).sum() / (sc ** 2).sum())
    X = np.column_stack([Z, Z * pS[:, None]]); out['tlw'] = X @ ls(X, s)
    X = sc[:, None] * Bm; out['dq'] = X @ ls(X, s)
    for k in MODELS: info[k + '_negative'] = float((out[k] <= 0).mean())
    return mv, s, out, info


def bdelta(E, D, g, mv, delta, draws, seed):
    """3x3 linear and quadratic parts of b(Delta) for the base components (worker, firm, covariance)."""
    d = np.zeros(D.n); d[mv] = delta
    lin = 4 * E.Ga(d, g)
    a = np.maximum(d, 0.0); b = np.maximum(-d, 0.0); nb = max(1, draws // 16); B = {}
    for name, v in (('a', a), ('b', b), ('ab', a + b)):
        B[name] = np.mean([E.pseudo(v, 16, np.random.default_rng([seed, j]))[0] for j in range(nb)], 0)   # same draws for the three
    quad = 2 * B['a'] + 2 * B['b'] - B['ab']
    return lin, -2 * quad


def main(ddir, troot, oroot, mfile, cell, draws=32):
    t0 = time.time(); manifest = json.load(open(mfile)); c, sc, P, D, info, E, tr, mu, v_py, sign_py, v_rows = context(ddir, manifest, cell)
    name = f"{c['scenario']}__{c['design']}"; ofile = f'{oroot}/{name}/bdelta.json'
    if os.path.exists(ofile): raise SystemExit(f'{ofile} exists; refusing to overwrite')
    th = json.load(open(f'{troot}/{name}/theory.json')); V = np.array(th['V_indep'])
    _, g = E.A.kernel(mu); g = np.array(g)
    mv, s, fits, finfo = oracle_fits(E, D, tr, v_rows)
    at, _ = E.std_effects(dict(alpha=tr['alpha'], psi=tr['psi']), 3.0); a_mv = at[D.w[mv]]
    cuts = np.quantile(a_mv, np.arange(1, 10) / 10); dec = np.searchsorted(cuts, a_mv, side='right')
    g4 = T34 @ g; wsum = s * g4[3][mv] ** 2                                        # weight of the row in the first term, variance of the sum
    res = dict(schema='LP-BDELTA-V1', cell=cell, scenario=c['scenario'], design=c['design'], rows=int(D.n), mover_rows=int(len(mv)),
               draws=draws, V=V.tolist(), fit=finfo, models={},
               true_by_decile=[float(s[dec == k].mean()) for k in range(10)],
               first_term_share_by_decile=[float(wsum[dec == k].sum() / wsum.sum()) for k in range(10)])
    for k in MODELS:
        delta = fits[k] - s; lin, quad = bdelta(E, D, g, mv, delta, draws, sc['seed'] + 7)
        lin4 = np.diag(T34 @ lin @ T34.T); quad4 = np.diag(T34 @ quad @ T34.T); b4 = lin4 + quad4
        res['models'][k] = dict(b_over_V=(b4 / V).tolist(), linear_over_V=(lin4 / V).tolist(), quadratic_over_V=(quad4 / V).tolist(),
                                se_ratio=np.sqrt(np.maximum(1 + b4 / V, 0)).tolist(),
                                rel_rmse=float(np.sqrt(np.mean(delta ** 2)) / s.mean()), mean_delta_over_mean=float(delta.mean() / s.mean()),
                                fitted_by_decile=[float(fits[k][dec == j].mean()) for j in range(10)])
    res['seconds'] = time.time() - t0; res['job_id'] = os.environ.get('JOB_ID', ''); res['host'] = os.environ.get('HOSTNAME', '')
    write_json(ofile, res); print(json.dumps(res), flush=True)


if __name__ == '__main__':
    main(sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], int(sys.argv[5]), int(sys.argv[6]) if len(sys.argv) > 6 else 32)

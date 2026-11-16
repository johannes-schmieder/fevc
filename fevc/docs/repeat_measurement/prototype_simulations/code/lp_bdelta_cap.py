"""Diagnostic companion of lp_bdelta.py: the same oracle error b(Delta)/V of the loading model (twl), computed
for several winsorising limits of the standardised effects.  Shows how much of the model's error at fixed
characteristics comes from capping the effects at 3 standard deviations.
Usage: lp_bdelta_cap.py <design-dir> <theory-root> <out-root> <manifest> <cell-index> [draws]
Writes <out-root>/<scenario>__<design>/bdelta_cap.json atomically; refuses to overwrite."""
import sys, os, json, time, numpy as np
from lp6 import T34
from lp_mc6 import context, write_json
from lp_bdelta import oracle_fits, bdelta

CAPS = (3.0, 4.0, 6.0, 1e6)


def main(ddir, troot, oroot, mfile, cell, draws=32):
    t0 = time.time(); manifest = json.load(open(mfile)); c, sc, P, D, info, E, tr, mu, v_py, sign_py, v_rows = context(ddir, manifest, cell)
    name = f"{c['scenario']}__{c['design']}"; ofile = f'{oroot}/{name}/bdelta_cap.json'
    if os.path.exists(ofile): raise SystemExit(f'{ofile} exists; refusing to overwrite')
    V = np.array(json.load(open(f'{troot}/{name}/theory.json'))['V_indep']); _, g = E.A.kernel(mu); g = np.array(g)
    at, _ = E.std_effects(dict(alpha=tr['alpha'], psi=tr['psi']), 1e9); mvr = np.flatnonzero(~D.stay); a_mv = np.abs(at[D.w[mvr]])
    g4 = T34 @ g; lead = v_rows[mvr] * g4[3][mvr] ** 2
    res = dict(schema='LP-BDELTA-CAP-V1', cell=cell, scenario=c['scenario'], design=c['design'], draws=draws, V=V.tolist(), caps={},
               share_rows_beyond={str(k): float((a_mv > k).mean()) for k in (3, 4, 6)},
               first_term_share_beyond={str(k): float(lead[a_mv > k].sum() / lead.sum()) for k in (3, 4, 6)})
    for cap in CAPS:
        mv, s, fits, finfo = oracle_fits(E, D, tr, v_rows, cap=cap); delta = fits['twl'] - s
        lin, quad = bdelta(E, D, g, mv, delta, draws, sc['seed'] + 7)
        lin4 = np.diag(T34 @ lin @ T34.T); quad4 = np.diag(T34 @ quad @ T34.T); b4 = lin4 + quad4
        res['caps'][str(cap)] = dict(se_ratio=np.sqrt(np.maximum(1 + b4 / V, 0)).tolist(), linear_over_V=(lin4 / V).tolist(),
                                     quadratic_over_V=(quad4 / V).tolist(), load=finfo['load'], negative=finfo['twl_negative'],
                                     scale_floored=finfo['scale_floored'], profile_floored=finfo['profile_floored'])
    res['seconds'] = time.time() - t0; res['job_id'] = os.environ.get('JOB_ID', ''); res['host'] = os.environ.get('HOSTNAME', '')
    write_json(ofile, res); print(json.dumps(res), flush=True)


if __name__ == '__main__':
    main(sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], int(sys.argv[5]), int(sys.argv[6]) if len(sys.argv) > 6 else 32)

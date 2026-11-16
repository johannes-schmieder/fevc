"""Feasibility of the direct firm-Laplacian solver on the six-year design (SCC).  Usage: lp_feas.py <design-dir>"""
import sys, time, json, resource, numpy as np
from scipy.sparse.linalg import splu
from lp import Panel, build_design
t0 = time.time(); z = np.load(sys.argv[1] + '/panel.npz'); P = Panel(z['w'], z['f'], z['year'], z['y'])
D, info = build_design(P); rss = lambda: resource.getrusage(resource.RUSAGE_SELF).ru_maxrss / 1e6
out = dict(rows=int(D.n), mover_rows=int((~D.stay).sum()), firms=int(D.nJ), L_nnz=int(D.L.nnz), build_seconds=time.time() - t0, rss_gb_after_build=rss())
print(out, flush=True)
Lg = D.L[:-1, :-1].tocsc()
for name, kw in (('symmetric_mmd', dict(permc_spec='MMD_AT_PLUS_A', diag_pivot_thresh=0.0, options=dict(SymmetricMode=True))), ('colamd', dict())):
    t1 = time.time()
    try:
        lu = splu(Lg, **kw); tf = time.time() - t1
        rhs = np.random.default_rng(0).standard_normal((Lg.shape[0], 8)); t2 = time.time(); x = lu.solve(rhs); ts = (time.time() - t2) / 8
        res = float(np.abs(Lg @ x - rhs).max())
        out[name] = dict(factor_seconds=tf, nnz_L=int(lu.L.nnz), nnz_U=int(lu.U.nnz), solve_seconds_per_rhs=ts, max_residual=res, rss_gb=rss())
    except Exception as e:
        out[name] = dict(error=repr(e), seconds=time.time() - t1, rss_gb=rss())
    print(name, out[name], flush=True)
    if 'error' not in out[name] and out[name]['factor_seconds'] < 600: break
json.dump(out, open(sys.argv[1] + '/feasibility.json', 'w'), indent=1)

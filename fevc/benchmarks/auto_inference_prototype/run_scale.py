"""One large design: cost split and numerical noise of the SE from few draws.

Usage:
    python run_scale.py FIRMS MOVERS STAYERS SEED
"""

import sys
import time

import numpy as np
from akm_design import make_design_fast
from dense_mc import true_effects
from run_sparse import scen_v
from sparse_auto import AutoSE, true_cov

nf, nm, ns, seed = map(int, sys.argv[1:5])
t0 = time.time()
d = make_design_fast(nf, nm, ns, seed)
print(
    "design n=%d rows (%d mover matches, %d stayer obs), workers=%d firms=%d, build %.0fs"
    % (d.n, (~d.stay).sum(), d.stay.sum(), d.nW, d.nJ, time.time() - t0),
    flush=True,
)
t0 = time.time()
d.factor()
print("factorisation %.0fs" % (time.time() - t0), flush=True)
rng = np.random.default_rng(seed + 1)
a, p = true_effects(d, rng)
mu = d.X(a, p)
A = AutoSE(d, probes=200, seed=seed + 2)
print("pass 1 (point estimate): %d solves, %.0fs" % (A.solves_pass1, A.t_pass1), flush=True)
for scen in ("mass", "firm_omit"):
    v = scen_v(d, A.Pd, scen)
    y = mu + np.sqrt(v) * rng.standard_normal(d.n)
    sd_true = np.sqrt(np.diag(true_cov(A, mu, v, draws=32)))
    for R2 in (2, 8):
        ses = []
        sem = []
        for s in range(6):
            r = A.fit(y, draws=R2, seed=50 + s)
            ses.append(np.sqrt(np.diag(r["V_rob"])))
            sem.append(np.sqrt(np.diag(r["V_model"])))
        ses = np.array(ses)
        sem = np.array(sem)
        print(
            "%-9s draws=%d: pass 2 = %d solves, %.1fs | theta %s | true SD %s"
            % (scen, R2, r["solves_pass2"], r["t_pass2"], r["theta"].round(5), sd_true.round(6))
        )
        print(
            "    robust SE/true SD %s  (CV across draw seeds %s)"
            % ((ses.mean(0) / sd_true).round(3), (ses.std(0) / ses.mean(0)).round(4))
        )
        print(
            "    model  SE/true SD %s  (CV across draw seeds %s)"
            % ((sem.mean(0) / sd_true).round(3), (sem.std(0) / sem.mean(0)).round(4))
        )
        print(
            "    r_eff %s  max influence share %s  floored %.4f"
            % (r["reff"].round(0), r["infl"].round(6), r["floored"]),
            flush=True,
        )

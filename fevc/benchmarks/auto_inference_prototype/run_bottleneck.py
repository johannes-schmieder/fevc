"""Bottleneck design: does the draw-based concentration diagnostic flag it?

Usage:
    python run_bottleneck.py BRIDGE_MOVERS REPS
"""

import sys

import numpy as np
from akm_design import make_design_bottleneck
from dense_mc import DenseOracle, evaluate, features, true_effects, variance_scenarios
from sparse_auto import AutoSE

nb = int(sys.argv[1])
R = int(sys.argv[2])
d = make_design_bottleneck(200, 1500, 100, 21, n_bridge=nb)
o = DenseOracle(d)
print("bottleneck design: n=%d workers=%d firms=%d bridge movers=%d" % (d.n, d.nW, d.nJ, nb))
rng = np.random.default_rng(3)
a, p = true_effects(d, rng)
mu = d.X(a, p)
v = variance_scenarios(d, o)["mass"]
lam = [np.linalg.eigvalsh((np.sqrt(v)[:, None] * o.C[t]) * np.sqrt(v)[None, :]) for t in range(3)]
lead = [float((ev**2).max() / (ev**2).sum()) for ev in lam]
reff = [float((ev**2).sum() ** 2 / (ev**4).sum()) for ev in lam]
print(
    "exact leading share l1^2/sum l^2 %s ; exact r_eff %s ; bound 1/sqrt(r_eff) %s"
    % (np.round(lead, 3), np.round(reff, 1), np.round(1 / np.sqrt(reff), 3))
)
A = AutoSE(d, exact_diag=(o.Pd, np.array(o.Bd)))
y = mu + np.sqrt(v) * rng.standard_normal(d.n)
for dr in (8, 16):
    r = A.fit(y, draws=dr, seed=5)
    print(
        "draws=%d estimated r_eff %s -> leading-share bracket [%s , %s]; max influence share %s"
        % (
            dr,
            r["reff"].round(1),
            (1 / r["reff"]).round(3),
            (1 / np.sqrt(r["reff"])).round(3),
            r["infl"].round(4),
        )
    )
Zn = features(o.Pd, o.Bd, d.F, d.stay, "lean")
specs = {
    "true variances": dict(fit="oracle"),
    "model (proxy, lean)": dict(fit="proxy", Z=Zn),
    "robust (lean)": dict(fit="proxy", Z=Zn, lin="robust"),
}
res = evaluate(o, mu, v, R, 9, specs)
om = o.true_cov(mu, v)
print(
    "trace share", np.round(np.array([2 * v @ o.CC[(t, t)] @ v for t in range(3)]) / np.diag(om), 3)
)
for name, oo in res["specs"].items():
    r = oo["trunc"]
    print(
        "  %-20s cover %s rmsSE/SD %s"
        % (name, np.round(r["cover"], 3), np.round(r["rms_ratio"], 3))
    )

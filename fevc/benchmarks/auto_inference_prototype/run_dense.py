"""Exact dense Monte Carlo: compare variance estimators on one design.

Usage:
    python run_dense.py FIRMS MOVERS STAYERS SEED REPS SCENARIOS [SD_PSI] [ERRORS] [KURT]

SCENARIOS is a comma list from homo,mass,leverage,firm_omit,worker_omit.
ERRORS is gauss, t5 or skew.  KURT adds the exact fourth-cumulant noise term
with that excess kurtosis to one extra robust row.
"""

import sys
import time

import numpy as np
from akm_design import make_design
from dense_mc import DenseOracle, evaluate, features, true_effects, variance_scenarios


def specs_for(d, o, kurt=0.0):
    common = features(o.Pd, o.Bd, d.F, d.stay, "common")
    lean = features(o.Pd, o.Bd, d.F, d.stay, "lean")
    homo = features(o.Pd, o.Bd, d.F, d.stay, "homo")
    specs = {
        "true variances": dict(fit="oracle"),
        "current (moment, common)": dict(fit="moment", Z=common),
        "model (proxy, common)": dict(fit="proxy", Z=common),
        "model (proxy, lean)": dict(fit="proxy", Z=lean),
        "naive plug-in": dict(fit="proxy", Z=lean, lin="naive"),
        "robust (lean)": dict(fit="proxy", Z=lean, lin="robust"),
        "robust-dr (lean)": dict(fit="proxy", Z=lean, lin="dr"),
        "robust (common)": dict(fit="proxy", Z=common, lin="robust"),
        "robust (homo)": dict(fit="proxy", Z=homo, lin="robust"),
    }
    if kurt:
        specs["robust (lean) + kurtosis"] = dict(fit="proxy", Z=lean, lin="robust", kurt=kurt)
    return specs


def main(argv):
    nf, nm, ns, seed, reps = map(int, argv[:5])
    scenarios = argv[5].split(",")
    sd_p = float(argv[6]) if len(argv) > 6 else 0.18
    errors = argv[7] if len(argv) > 7 else "gauss"
    kurt = float(argv[8]) if len(argv) > 8 else 0.0
    t0 = time.time()
    d = make_design(nf, nm, ns, seed)
    o = DenseOracle(d)
    print(
        "design n=%d workers=%d firms=%d stayer rows=%d leverage mean %.3f max %.3f mover matches/firm %.1f"
        % (d.n, d.nW, d.nJ, d.stay.sum(), o.Pd.mean(), o.Pd.max(), (~d.stay).sum() / d.nJ),
        flush=True,
    )
    a, p = true_effects(d, np.random.default_rng(seed + 1), sd_p=sd_p)
    mu = d.X(a, p)
    specs = specs_for(d, o, kurt)
    sc = variance_scenarios(d, o)

    def f3(x):
        return np.array2string(np.asarray(x), precision=3, floatmode="fixed", separator=", ")

    for s in scenarios:
        v = sc[s]
        om = o.true_cov(mu, v)
        th0 = np.array([mu @ o.C[t] @ mu for t in range(3)])
        noise = np.array([2 * v @ o.CC[(t, t)] @ v for t in range(3)])
        corr = np.array([np.diag(o.B[t]) @ v for t in range(3)])
        print("== %s / %s errors, %d samples" % (s, errors, reps))
        print(
            "   theta %s  SD/|theta| %s  noise share of variance %s  KSS correction/theta %s"
            % (
                f3(th0),
                f3(np.sqrt(np.diag(om)) / np.abs(th0)),
                f3(noise / np.diag(om)),
                f3(corr / th0),
            )
        )
        res = evaluate(o, mu, v, reps, seed + 7, specs, errors=errors)
        print("   Monte Carlo SD / analytic SD %s" % f3(res["sd_emp"] / res["sd_true"]))
        for name, out in res["specs"].items():
            for key in ("raw", "trunc"):
                r = out[key]
                print(
                    "   %-26s %-5s available %s  SE ratio rms %s median %s  coverage %s"
                    % (
                        name,
                        key,
                        f3(r["avail"]),
                        f3(r["rms_ratio"]),
                        f3(r["med_ratio"]),
                        f3(r["cover"]),
                    ),
                    flush=True,
                )
    print("total %.0fs" % (time.time() - t0))


if __name__ == "__main__":
    main(sys.argv[1:])

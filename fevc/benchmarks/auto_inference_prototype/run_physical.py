"""Controls, deletion units and centering: Monte Carlo on physical rows.

Usage:
    python run_physical.py FIRMS MOVERS STAYERS SEED REPS DELETION CONTROLS SCENARIOS

DELETION is match, split (two declared units inside longer mover matches) or
observation.  CONTROLS is the number of controls (0 for none).  SCENARIOS is a
comma list from base,firm_omit,worker_omit.
"""

import sys
import time

import numpy as np

from akm_design import make_design
from dense_mc import T34, true_effects
from physical_mc import PhysicalOracle, error_cov, evaluate, expand, make_controls


def f3(x):
    return np.array2string(np.asarray(x), precision=3, floatmode="fixed", separator=", ")


def main(argv):
    nf, nm, ns, seed, reps = map(int, argv[:5])
    deletion = argv[5]
    k = int(argv[6])
    scenarios = argv[7].split(",")
    t0 = time.time()
    d = make_design(nf, nm, ns, seed)
    rng = np.random.default_rng(seed + 100)
    ph = expand(d, deletion, rng)
    Z = make_controls(d, ph, k, rng)
    o = PhysicalOracle(d, ph, Z)
    a, p = true_effects(d, np.random.default_rng(seed + 1))
    gamma = np.array([0.03, 0.10] + [0.02] * max(k - 2, 0))[:k]
    mu = a[ph["w"]] + p[ph["f"]] + Z @ gamma
    theta = T34 @ d.theta(a, p)
    print(
        "design: %d physical rows, %d deletion units (%s), %d workers, %d firms, %d controls"
        % (o.nP, o.nU, deletion, o.du.nW, o.du.nJ, k),
        flush=True,
    )
    for sc in scenarios:
        # observation deletion assumes independent observations, so no unit effect
        Gam = error_cov(d, ph, sc, s_m2=0.0 if deletion == "observation" else 0.02)
        r = evaluate(o, mu, Gam, reps, seed + 7)
        sdJ = r["sdJ"]
        print("== %s errors, %d samples" % (sc, reps))
        print("   theta %s   analytic SD of the joint estimator %s" % (f3(theta), f3(sdJ)))
        est = (("joint", "thJ"), ("fixed offset", "thF"), ("corrected", "thC"))
        for name, key in est:
            th = r[key]
            print(
                "   %-13s Monte Carlo SD / joint analytic SD %s   mean bias / SD %s"
                % (name, f3(th.std(1, ddof=1) / sdJ), f3((th.mean(1) - theta) / sdJ))
            )
        print(
            "   RMS difference from the joint estimate, in joint SDs: fixed offset %s corrected %s"
            % (
                f3(np.sqrt(((r["thF"] - r["thJ"]) ** 2).mean(1)) / sdJ),
                f3(np.sqrt(((r["thC"] - r["thJ"]) ** 2).mean(1)) / sdJ),
            )
        )
        print(
            "   corrected-centering increment, RMS in joint SDs %s"
            % np.array2string(
                np.sqrt(((r["thC"] - r["thF"]) ** 2).mean(1)) / sdJ, precision=5, separator=", "
            )
        )
        rows = (
            (("with control term", "var"), ("without control term", "var_nc"))
            if k
            else (("standard error", "var"),)
        )
        for label, key in rows:
            se = np.sqrt(r[key])
            print(
                "   %-22s rms SE / joint analytic SD %s"
                % (label, f3(np.sqrt((se**2).mean(1)) / sdJ))
            )
            for name, ekey in est:
                cover = (np.abs(r[ekey] - theta[:, None]) <= 1.959964 * se).mean(1)
                print("   %-22s coverage, %-13s %s" % ("", name, f3(cover)), flush=True)
    print("total %.0fs" % (time.time() - t0))


if __name__ == "__main__":
    main(sys.argv[1:])

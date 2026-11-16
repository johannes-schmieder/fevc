"""Local smoke test of lp_real.py on a synthetic panel written as panel.npz."""
import sys, os, numpy as np, subprocess
from lp import synth_panel
rng = np.random.default_rng(5); P = synth_panel(nJ=60, nW=2500, seed=11)
alpha = 0.3 * rng.standard_normal(P.nW); psi = 0.15 * rng.standard_normal(P.nJ)
y = alpha[P.w] + psi[P.f] + 0.2 * rng.standard_normal(P.n)
d = sys.argv[1]; os.makedirs(d, exist_ok=True); np.savez(d + '/panel.npz', w=P.w, f=P.f, year=P.year, y=y)

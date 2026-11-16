"""KSS's heuristic for choosing q uses the eigenvalue share lambda_1^2 / sum_l lambda_l^2 (threshold 1/10).  Compute it for
the geometries of the q1-conf1 designs (reads the manifest and the stored lambda_1; writes one JSON next to this script)."""
import sys, json, os, time
import numpy as np
L = '<scratch>'
sys.path.insert(0, L + '/code')
from lp6 import build_design, T34
from lp_q1sim import two_block_panel
man = json.load(open(L + '/runs_q1/manifest_q1-conf1.json')); COMP = ['person', 'firm', 'cov', 'sum']; TP = 96
out = {}; seen = {}
for cell in man['cells']:
    key = (cell['nJ'], cell['nW'], cell['n_bridge'], cell['design_seed'])
    if key not in seen:
        t0 = time.time(); P, blk, bridge = two_block_panel(*key); D, info = build_design(P)
        rng = np.random.default_rng(424242); R = (2.0 * (rng.random((D.n, TP)) < 0.5) - 1.0)
        bR = D.solve(*D.Xt(R)); tr = []
        for c in range(4):
            ra = 0.0; rp = 0.0
            for t in range(3):
                if T34[c, t] != 0: x, yv = D.A(t, *bR); ra = ra + T34[c, t] * x; rp = rp + T34[c, t] * yv
            BR = D.X(*D.solve(ra, rp)); sq = (BR ** 2).sum(0); tr.append((float(sq.mean()), float(sq.std(ddof=1) / np.sqrt(TP))))
        seen[key] = tr; print('geometry', key, f'{time.time() - t0:.1f}s', flush=True)
    des = json.load(open(f"{L}/runs_q1/q1-conf1/{cell['name']}/design.json"))['comp']
    out[cell['name']] = {COMP[c]: dict(lam1=des[COMP[c]]['lam1'], sum_lam_sq=seen[key][c][0], rel_se=seen[key][c][1] / seen[key][c][0],
                                       eigen_share=des[COMP[c]]['lam1'] ** 2 / seen[key][c][0]) for c in range(4)}
json.dump(out, open(os.path.dirname(os.path.abspath(__file__)) + '/eigshare.json', 'w'), indent=1)
sw = json.load(open(os.path.dirname(os.path.abspath(__file__)) + '/switch.json'))
print(f"{'design':22s} {'comp':6s} {'eig share':>9s} {'KSS q':>5s} {'kappa':>6s} {'normal':>6s} {'q1':>6s} {'KSS-rule cover':>14s}")
bad = []
for name, r in out.items():
    for cp in COMP[:3]:
        s = r[cp]['eigen_share']; q = int(s >= 0.1); w = sw[name][cp]; cv = w['q1'] if q else w['normal']
        print(f"{name:22s} {cp:6s} {s:9.4f} {q:5d} {w['kappa_true']:6.2f} {w['normal']:6.3f} {w['q1']:6.3f} {cv:14.3f}")
        bad.append((cv, name, cp, s))
cv = np.array([b[0] for b in bad]); print(f"\nKSS heuristic (share >= 1/10 -> q = 1): min cover {cv.min():.3f} mean {cv.mean():.3f} below 0.93: {(cv < 0.93).sum()} of {len(cv)}; q = 1 chosen in {sum(b[3] >= 0.1 for b in bad)}")

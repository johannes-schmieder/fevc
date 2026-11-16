"""Build a sub-panel of a person-year panel (SCC only; the output stays private; prints aggregates only).
Usage: lp_sub.py <design-dir-in> <design-dir-out> years <first> <last>
       lp_sub.py <design-dir-in> <design-dir-out> half <seed>
Keeps the selected person-years, then prunes: stayers need at least two person-years; movers' matches are pruned to
the leave-one-match-out connected set (akm_design.prune; movers keep at least two matches); stayers are kept if
their firm is in that set.  Writes panel.npz (w, f, year, y) and panel_descriptives.json; refuses to overwrite."""
import sys, os, json, time, numpy as np
from akm_design import prune
din, dout, mode = sys.argv[1], sys.argv[2], sys.argv[3]; t0 = time.time()
if os.path.exists(dout + '/panel.npz'): raise SystemExit(dout + '/panel.npz exists; refusing to overwrite')
os.makedirs(dout, exist_ok=True)
z = np.load(din + '/panel.npz'); w, f, year, y = z['w'].astype(np.int64), z['f'].astype(np.int64), z['year'], z['y']
if mode == 'years':
    sel = (year >= int(sys.argv[4])) & (year <= int(sys.argv[5])); how = dict(mode='years', first=int(sys.argv[4]), last=int(sys.argv[5]))
elif mode == 'half':
    rng = np.random.default_rng(int(sys.argv[4])); pick = rng.random(int(w.max()) + 1) < 0.5; sel = pick[w]; how = dict(mode='half', seed=int(sys.argv[4]))
else:
    raise SystemExit('unknown mode')
w, f, year, y = w[sel], f[sel], year[sel], y[sel]
_, w = np.unique(w, return_inverse=True); _, f = np.unique(f, return_inverse=True); nW, nJ = w.max() + 1, f.max() + 1
key = w * nJ + f; uk, match = np.unique(key, return_inverse=True); mw, mf = uk // nJ, uk % nJ; mlen = np.bincount(match)
nfirm = np.bincount(mw, minlength=nW); T = np.bincount(w, minlength=nW); stay_w = nfirm == 1
ok_match = ~stay_w[mw] | (T[mw] >= 2)                                # drop stayers with a single person-year
rows = np.flatnonzero(ok_match)
_, _, _, _, keep, _ = prune(mw[rows], mf[rows], mlen[rows].astype(float), stay_w[mw[rows]])
keep_match = np.zeros(len(uk), bool); keep_match[rows[keep]] = True
py = keep_match[match]; w2, f2 = w[py], f[py]
_, w2 = np.unique(w2, return_inverse=True); _, f2 = np.unique(f2, return_inverse=True)
tmp = dout + '/panel.tmp.npz'; np.savez(tmp, w=w2.astype(np.int32), f=f2.astype(np.int32), year=year[py], y=y[py]); os.replace(tmp, dout + '/panel.npz')
nW2, nJ2 = w2.max() + 1, f2.max() + 1; key2 = w2.astype(np.int64) * nJ2 + f2; uk2, cnt2 = np.unique(key2, return_counts=True)
mw2 = uk2 // nJ2; nf2 = np.bincount(mw2, minlength=nW2); st2 = nf2 == 1; T2 = np.bincount(w2, minlength=nW2)
d = dict(source=how, person_years_selected=int(sel.sum()), person_years=int(py.sum()), workers=int(nW2), firms=int(nJ2), matches=int(len(uk2)),
         stayers=int(st2.sum()), stayer_rows=int(T2[st2].sum()), movers=int((~st2).sum()), mover_matches=int((~st2[mw2]).sum()),
         stayer_years_hist={int(k): int(v) for k, v in zip(*np.unique(T2[st2], return_counts=True))},
         firms_per_mover_hist={int(k): int(v) for k, v in zip(*np.unique(nf2[~st2], return_counts=True))},
         y_mean=float(y[py].mean()), y_var=float(y[py].var()), seconds=time.time() - t0)
json.dump(d, open(dout + '/panel_descriptives.json', 'w'), indent=1); print(json.dumps(d, indent=1))

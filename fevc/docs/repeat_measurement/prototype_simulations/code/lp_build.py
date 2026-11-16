"""Build the person-year panel arrays from the private pooled Veneto CSV (SCC only; output stays private).
Usage: lp_build.py <csv> <design-dir>.  Prints aggregate descriptives only."""
import sys, json, time, numpy as np, pandas as pd
src, out = sys.argv[1], sys.argv[2]; t0 = time.time()
df = pd.read_csv(src, usecols=['year', 'worker', 'firm', 'y_adjusted'])
w = pd.factorize(df['worker'])[0].astype(np.int32); f = pd.factorize(df['firm'])[0].astype(np.int32)
year = df['year'].to_numpy().astype(np.int16); y = df['y_adjusted'].to_numpy(float)
np.savez(out + '/panel.npz', w=w, f=f, year=year, y=y)
nW, nJ = w.max() + 1, f.max() + 1
key = w.astype(np.int64) * nJ + f; uk, minv, mcnt = np.unique(key, return_inverse=True, return_counts=True)
mw = (uk // nJ).astype(np.int64); nfirm = np.bincount(mw, minlength=nW); stay = nfirm == 1
T = np.bincount(w, minlength=nW)
dup = pd.Series(w.astype(np.int64) * 3000 + year).duplicated().sum()
d = dict(rows=int(len(w)), workers=int(nW), firms=int(nJ), matches=int(len(uk)), stayers=int(stay.sum()),
         stayer_rows=int(T[stay].sum()), mover_rows=int(T[~stay].sum()), mover_matches=int((~stay[mw]).sum()),
         stayer_years_hist={int(k): int(v) for k, v in zip(*np.unique(T[stay], return_counts=True))},
         mover_years_hist={int(k): int(v) for k, v in zip(*np.unique(T[~stay], return_counts=True))},
         mover_match_length_hist={int(k): int(v) for k, v in zip(*np.unique(mcnt[~stay[mw]], return_counts=True))},
         firms_per_mover_hist={int(k): int(v) for k, v in zip(*np.unique(nfirm[~stay], return_counts=True))},
         duplicate_worker_years=int(dup), y_mean=float(y.mean()), y_var=float(y.var()), seconds=time.time() - t0)
json.dump(d, open(out + '/panel_descriptives.json', 'w'), indent=1); print(json.dumps(d, indent=1))

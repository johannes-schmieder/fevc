"""Can the pooled panel be linked to the file with age?  Prints aggregate counts only (no rows, no identifiers)."""
import sys, json, numpy as np, pandas as pd
pooled, lcs = sys.argv[1], sys.argv[2]
a = pd.read_csv(pooled, usecols=['year', 'worker', 'firm', 'observation_key', 'y_adjusted'])
b = pd.read_csv(lcs, usecols=['worker', 'firm', 'year', 'y', 'age'])
out = dict(pooled_rows=len(a), lcs_rows=len(b))
for col in ('worker', 'firm'):
    ua, ub = np.unique(a[col].to_numpy()), np.unique(b[col].to_numpy())
    out[col] = dict(pooled_unique=int(len(ua)), lcs_unique=int(len(ub)), pooled_in_lcs=float(np.isin(ua, ub).mean()),
                    pooled_range_is_dense=bool(ua.min() >= 0 and ua.max() < 2 * len(ua)), lcs_range_is_dense=bool(ub.max() < 2 * len(ub)))
ok = a['observation_key'].to_numpy()
out['observation_key'] = dict(unique=int(len(np.unique(ok))), min_is_small=bool(ok.min() < 10), max_over_rows=float(ok.max() / len(a)))
m = a.merge(b, on=['worker', 'year'], how='inner'); out['match_worker_year'] = int(len(m))
m2 = a.merge(b, on=['firm', 'year'], how='inner'); out['match_firm_year_rows'] = int(len(m2))
print(json.dumps(out, indent=1))

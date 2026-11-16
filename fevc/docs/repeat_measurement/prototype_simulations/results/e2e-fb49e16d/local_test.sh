#!/bin/bash
# Local test of the SCC do-files with the mac plugin: the smoke, the result writer, and its refusal to overwrite.
set -uo pipefail
here=<session-tmp>/e2e
pkg=${PKG:-<session-tmp>/pkgflip/fevc}
stata=/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp
rm -rf "$here/lt"; mkdir -p "$here/lt/smoke" "$here/lt/run" "$here/lt/out"
cd "$here/lt/smoke" && env -u FEVC_DEFAULT_INFERENCE "$stata" -b do "$here/smoke.do" "$pkg" 2
grep -hE "^SMOKE|^E2E SMOKE PASS|^r\(" "$here/lt/smoke/"*.log
<repo>/.venv/bin/python - <<'EOF'
import numpy as np, pandas as pd
d = pd.read_csv('<scratch>/pkgmc/runs/smoke1/designs/s_b10_g0_flat_skew.csv')
d['y'] = d['alpha'] + d['psi'] + d['sd'] * np.random.default_rng(3).standard_normal(len(d)) + 4.8
d[['worker', 'firm', 'year', 'y']].to_csv('<session-tmp>/e2e/lt/panel.csv', index=False)
EOF
cd "$here/lt/run" || exit 1
for deletion in match observation; do
    env -u FEVC_DEFAULT_INFERENCE "$stata" -b do "$here/veneto_e2e.do" "$pkg" "$here/lt/panel.csv" testpanel $deletion 2 "$here/lt/out"
    grep -E "^E2E|^r\(" "$here/lt/run/"*.log | tail -3
done
env -u FEVC_DEFAULT_INFERENCE "$stata" -b do "$here/veneto_e2e.do" "$pkg" "$here/lt/panel.csv" testpanel match 2 "$here/lt/out"
grep -E "refusing|^r\(" "$here/lt/run/"*.log | tail -2
ls -la "$here/lt/out"; head -c 1500 "$here/lt/out/testpanel__match.txt"

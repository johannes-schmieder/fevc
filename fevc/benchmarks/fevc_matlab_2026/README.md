# FEVC--MATLAB 2026 benchmark campaign

This directory implements the new referee-facing benchmark campaign.  It is
separate from `comparative_scaling`, whose code and source-bound evidence remain
unchanged.

The implemented main path is:

1. `build_run.py` creates an immutable source bundle plus 240-cell and 24-bundle
   manifests.
2. `deploy_scc.sh` installs the run under `/projectnb/welfgr/vckss/runs`.
3. `submit_scc.sh ... prepare` builds or imports exact Rust/MATLAB artifacts.
4. `submit_scc.sh ... pilot-small` and `pilot-worst` run the registered resource
   gates.
5. `submit_scc.sh ... production` submits the 24 whole-node bundles.
6. Validators require scheduler, application, numerical, memory, and output
   evidence before aggregation.

MATLAB is pinned to `matlab/2026a`.  Production tasks request 28 Broadwell
E5-2680v4 slots and use the application ladder 1/2/4/8/14/28.  Mata is absent
from the production manifest.  See `PROTOCOL.md` for the complete registered
design, exclusions, memory definitions, additional modules, and recovery rules.

Focused local gate:

```bash
./.venv/bin/python -m pytest -q \
  fevc/benchmarks/fevc_matlab_2026/tests/test_campaign.py
```

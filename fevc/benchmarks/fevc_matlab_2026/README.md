# FEVC--MATLAB 2026 SCC campaign

This directory implements the referee-facing FEVC Rust versus maintained
MATLAB R2026a benchmark.  The registered 240-cell scientific matrix is
unchanged; this harness keeps development testing separate from publication
evidence.

## Commands

Run from the implementation repository:

```bash
# Current working bytes, one four-core SCC job, no heartbeat.
fevc/benchmarks/fevc_matlab_2026/campaign.sh smoke

# Clean commit only. Queues smoke -> dual-boundary pilot -> 24-bundle production.
fevc/benchmarks/fevc_matlab_2026/campaign.sh submit

fevc/benchmarks/fevc_matlab_2026/campaign.sh status RUN_ID
fevc/benchmarks/fevc_matlab_2026/campaign.sh collect RUN_ID [DESTINATION]

# Only for bundle tasks whose original qacct record has failed != 0.
fevc/benchmarks/fevc_matlab_2026/campaign.sh retry RUN_ID BUNDLE_TASK_RANGE
```

`smoke` snapshots tracked and untracked non-ignored candidate bytes and records
the archive hash; it is diagnostic evidence.  `submit` requires a clean commit
and creates one source-bound campaign directory.  The smoke job builds the
campaign's only Rust/MATLAB artifact set and then runs a 7,680-row, four-worker
cell through the real launch and measurement path.  A successful smoke releases
one 28-core pilot job that runs the largest `strong_d2` convergence boundary
and largest `weak_d3` resource boundary sequentially.  Both pass receipts gate
production.  The campaign explicitly receipts the package-default 10,000 PCG
iteration cap; estimator tolerances and the registered 240-cell matrix are
unchanged.  SGE dependencies provide ordering; downstream wrappers fail closed
when an upstream pass receipt is absent.

No stage requires a heartbeat.  `status` is a read-only campaign summary, and
final collection requires scheduler, wrapper, numerical, memory, schema, and
output validation.  `retry` permits one contiguous SGE retry of bundles affected
by scheduler or execution-host failure; `status` prints those bundle IDs.
Application failures stop for diagnosis.

## Development gate

```bash
./.venv/bin/python -m pytest -q \
  fevc/benchmarks/fevc_matlab_2026/tests/test_campaign.py
```

See `PROTOCOL.md` for the frozen scientific design, platform restrictions,
memory definitions, acceptance rules, and additional publication modules.

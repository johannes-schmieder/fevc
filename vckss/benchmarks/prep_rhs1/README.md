# PREP-RHS-1 benchmark harness

`run_local.py` compares two committed revisions in separate Git archives and
fresh Stata processes. The tracked `../numopt2_local.do` driver owns fixture
generation, estimator execution, scientific assertions, and timing capture;
Python owns only archival isolation, process launch, hashes, receipt checks,
and descriptive summaries.

The default local fixture has 60,000 stored rows, 30,000 coefficient cells,
10,000 workers, 1,000 firms, and 200 probes. Run it before SCC work:

```sh
./.venv/bin/python vckss/benchmarks/prep_rhs1/run_local.py \
  --candidate "$(git rev-parse HEAD)" \
  --output-dir /private/tmp/vckss-prep-rhs1-local
```

Timing targets are advisory. Scientific, structural, residual, and repeated-
result checks are fail-closed. A warm median within two percent is neutral;
safe work-count or memory improvements are retained. A slowdown above two
percent requests additional interleaved repetitions before promotion rather
than silently discarding the candidate.

For SCC runs, validate every experiment with the source-bound
`../scc/validate_numopt2_scale.py` gate before summarizing copied receipts:

```sh
./.venv/bin/python vckss/benchmarks/prep_rhs1/analyze_scc.py \
  --baseline-dir /path/to/baseline-run-copy \
  --candidate-dir /path/to/candidate-run-copy \
  --output /path/to/prep-rhs1-scc-summary.json
```

The analyzer requires exact structural equality and bounds scientific output
differences at registered binary64 roundoff. It records scheduler hosts for
every pair. Timing medians are descriptive only when the scheduler placed any
pair on different hosts; do not interpret a `HOST_CONFOUNDED` summary as a
causal speed comparison.

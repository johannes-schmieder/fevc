# PREP-RHS-1 benchmark harness

`run_local.py` compares two committed revisions in separate Git archives and
fresh Stata processes. The tracked `../numopt2_local.do` driver owns fixture
generation, estimator execution, scientific assertions, and timing capture;
Python owns only archival isolation, process launch, hashes, receipt checks,
and descriptive summaries.

The default local fixture has 60,000 stored rows, 30,000 coefficient cells,
10,000 workers, 1,000 firms, and 200 probes. Run it before SCC work:

```sh
./.venv/bin/python varcomp_kss/benchmarks/prep_rhs1/run_local.py \
  --candidate "$(git rev-parse HEAD)" \
  --output-dir /private/tmp/varcomp-kss-prep-rhs1-local
```

Timing targets are advisory. Scientific, structural, residual, and repeated-
result checks are fail-closed. A warm median within two percent is neutral;
safe work-count or memory improvements are retained. A slowdown above two
percent requests additional interleaved repetitions before promotion rather
than silently discarding the candidate.

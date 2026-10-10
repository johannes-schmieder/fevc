# Pooled component assessment

This is a bounded development assessment of the explicit pooled structured
variance model. It does not establish general coverage. The registrations in
`fevc/docs/pooled_component_inference_*v1*.json` fix the designs, estimands,
diagnostic exclusions, budgets and screens. Amendment 1 preserves the stopped
first attempt and registers the structural-zero repair before replacement draws.

Run from the repository root with its Python environment and a locally built
native plugin directory. Freeze into a new output directory, then execute its
copied harness. For example:

```bash
OPENBLAS_NUM_THREADS=1 ./.venv/bin/python \
  fevc/benchmarks/pooled_component_inference_20261009/run.py \
  freeze .local/pooled-smoke --profile smoke --plugin-dir /path/to/plugins

OPENBLAS_NUM_THREADS=1 ./.venv/bin/python \
  .local/pooled-smoke/input/harness/run.py run-all .local/pooled-smoke \
  --stata /path/to/stata-mp --workers 2
```

The profiles are `smoke`, `pilot`, `assessment`, `diagnostic` and `numerical`.
The assessment contains eight design cells, 1,000 outcomes each, paired Mean
and fixed-population-mean calls, and all four targets: 16,000 calls and 64,000
rows. The diagnostic profile contains nine separate 100-outcome stress cells.
The numerical profile uses four fixed outcomes, three numerical seeds and four
budget settings; its output measures numerical variation, not sampling coverage.

The frozen manifest records geometry, support, variance positivity, semantic
random keys, exact task inventory, runtime and native source identities. Each
task has its own input, log, raw output, validated rows and terminal receipt.
Existing task directories are never overwritten. A deliberate-failure smoke
uses `run-task ... --deliberate-failure` in a separate frozen directory.

Aggregation checks every input and output hash, expected row, seed, count and
status. Every attempted replication is counted, including withheld targets.
`SCREENS_PASS` means only that the registered primary descriptive screens did
not fail; stress cells have no primary coverage acceptance gate. Read their
availability and all-attempt coverage directly. A scientific screen failure
exits nonzero and remains a failure; it is distinct from an execution or
inventory failure. Raw local Stata logs may contain installation information
and must be sanitized before inclusion in shared evidence.

The independent oracle uses dense matrices only on these small fixtures.
Production Rust remains matrix-free and exact Mata uses coefficient-space
contractions. Test the harness separately because the repository's default
pytest selection excludes benchmark directories:

```bash
./.venv/bin/python -m pytest -q \
  fevc/benchmarks/pooled_component_inference_20261009/test_pooled_harness.py
```

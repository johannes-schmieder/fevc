# Existing-draw match target audit

This isolated diagnostic reads target-specific state before the unchanged
whole-call covariance check. It never changes the variance fitter, repairs a
matrix, removes a rejection, or adds a Stata/FFI option. Its protocol is
`fevc/docs/match_target_interval_audit_v1.json`.

The exact q0 source bundle also contains the unchanged q1 harness. Before
reusing it, the runner checks production/build/RNG and q1-harness identity
between the two recorded source commits. Only a disposable copy of
`generic_jla.rs` receives `capture.rs`; `export.rs` is inserted into separate
generated executables. Original successful outputs and original failures are
both reconciled against the immutable raw campaign results.

Use repository Python and a fresh output directory:

```bash
./.venv/bin/python -m pytest -q rust/experiments/match_target_interval_audit/test_audit.py
./.venv/bin/python rust/experiments/match_target_interval_audit/run.py freeze RUN
./.venv/bin/python rust/experiments/match_target_interval_audit/run.py build RUN
./.venv/bin/python rust/experiments/match_target_interval_audit/run.py tiny RUN
VECLIB_MAXIMUM_THREADS=1 OPENBLAS_NUM_THREADS=1 ./.venv/bin/python rust/experiments/match_target_interval_audit/audit.py RUN --profile tiny
./.venv/bin/python rust/experiments/match_target_interval_audit/run.py replay RUN
VECLIB_MAXIMUM_THREADS=1 OPENBLAS_NUM_THREADS=1 ./.venv/bin/python rust/experiments/match_target_interval_audit/audit.py RUN
```

All 5,189 original PSD rejections and 16 fixed successful comparisons are
captured. The dense independent slice is prospectively fixed at 333 calls.
It constructs scalar match rows from physical rows and checks influence,
covariance, recenter and ellipse identities. Separate exact-trace/fitted and
exact-trace/true-variance calculations diagnose realized covariance estimates.
They are not population variances or qualification arms. The independent
quadrature/ellipse helpers from the completed observation target audit are
reused read-only and hash-bound in the new manifest.

The old 4,000-draw native critical values are retained for exact replay; new
diagnostic intervals use independent deterministic quadrature. Neither changes
the public critical-draw minimum or any old confirmation result. Computable
intervals in selected weak/null/multi-mode cases are not evidence of calibrated
coverage. Do not apply a success-conditioned coverage gate to this audit.

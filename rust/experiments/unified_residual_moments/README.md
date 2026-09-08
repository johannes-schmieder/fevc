# Unified residual-moment saved-draw comparison

This is the bounded development assessment in
`fevc/docs/unified_residual_moments_v1.json`, not independent confirmation.
It changes no production code, outcome generator, scientific cutoff or saved
receipt. The paired native executables use the preserved V2 policy and the
unified V3 policy, both at the current numerical settings. Stata exposes V3
only; this comparison does not add a legacy-fitter user option.

From the repository root:

```bash
./.venv/bin/python -m pytest -q fevc/tests/python/test_unified_saved_draws.py
./.venv/bin/python rust/experiments/unified_residual_moments/build.py BUILD
./.venv/bin/python rust/experiments/unified_residual_moments/run.py tiny \
  BUILD SAVED_PARENT TINY_OUTPUT --workers 8
./.venv/bin/python rust/experiments/unified_residual_moments/run.py comparison \
  BUILD SAVED_PARENT COMPARISON_OUTPUT --workers 8 --pipeline TINY_OUTPUT
```

Every output directory must be new. `SAVED_PARENT` is the registered
`public-development-1` evidence directory. Both profiles reconstruct only its
saved outcome draws with the byte-verified generators and original semantic
seeds. The complete tiny pipeline uses two saved draws per cell in both arms
(192 calls), split/reversed-task replays, explicit single-draw CSV captures,
and deliberate CLI failures. It makes no coverage claim.

The main comparison contains 22,400 match calls (28 cells, 400 draws, two arms)
and 160 observation regression calls (20 cells, four saved draws, two arms).
The observation slice is a regression check, not a four-draw coverage study.
The source bundle includes native build inputs, including the CMG `.rs.in`
template, generated adapters, executables, gate code and input hashes.
Changing a bound source or input invalidates the run. Each task has a private
output directory and a validated atomic compressed result; failed processes
retain their partial output and a failure receipt rather than silently losing
attempts. Timing is descriptive: eight concurrent single-threaded calls may
share a local CPU, and the within-task arm order alternates.

All four targets remain in the accounting, including shared and target-local
failures. The summary retains original two-sided gates and separately applies
the registered availability, lower-coverage and upper-SD/SE safeguards to
correctly specified, eligible match cases. Conservative coverage or wide
intervals do not become exact-calibration evidence. Genuine fit failure or
undercoverage requires an owner discussion, not automatic retuning, new draws,
target exclusions or a larger campaign. Known estimated-offset and
misspecification limitations remain explicit diagnostic cases.

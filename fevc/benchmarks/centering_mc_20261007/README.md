# Centering Monte Carlo, October 7, 2026

Owner-requested exploratory comparison of `centering(none|mean|corrected)`
with exact and JLA point estimates, plus repeated-sketch MCSE calibration.
This harness does not change the estimator or qualify a release.

## Design

`fesim`'s committed `akm/stylized` generator creates 100 workers, 15 firms,
six annual periods and five years of burn-in, seed 7102026. Sorting is positive
(`theta_sort=1.5`); worker and firm effect SDs are .45 and .30. Wage noise SD
is 1.5. `run.py:DGP` records the complete command, including mobility settings.
The frozen source uses fesim commit
`0242b5c2656f0bcfb241a17ff344774391506345`; unrelated uncommitted fesim work
is not included. fevc source is
`283d2524a011a71b4ed1e2e7e65d3fd605cb1576` with the adopted Linux plugin.

The retained network contains 573 observations, 99 workers, 15 firms, 294
deletion units and 56 eligible-stayer observations. The original panel has
600 rows, of which 574 are employed; one employed observation is pruned.
All targets use the retained observation population and denominator N.
Match deletion and `stayers(both)` retain the package's default hybrid
deletion convention. No controls, frequency weights or target weights are used.

Network, worker effects and firm effects remain fixed. Each of 2,000 sampling
replications draws independent Gaussian errors. The noiseless signal is
`lnwage_true-epsilon_true`, asserted equal to `3+alpha_true+psi_true`.
In fesim, `lnwage_true` includes the realized shock; it is not the conditional
mean. An initial development smoke used that field incorrectly. The independent
oracle caught this before the main run; queued SCC job 7918587 was cancelled.
The initial files are preserved under ignored local output.

Each outcome replication runs all three centerings in exact and JLA(256).
Separately, three fixed outcomes each receive 300 independent JLA sketches at
64 and 256 probes, with all three centerings sharing each sketch seed. Each
fixed outcome has its own exact reference for each centering. Total: 17,409
fits. Outcome and Counter-V1 sketch seeds derive from canonical semantic keys,
not task order, process count or scheduler identifiers. The estimator's target
parity folds remain unchanged.

## Gates and interpretation

- A complete local smoke and a two-core SCC smoke exercise generation, Stata,
  exports, validation and receipts. A deliberate Stata `r(459)` must be rejected
  even when Stata's process returns zero.
- The manifest freezes runtime bytes, DGP, dimensions, seeds, repetitions,
  outputs and thresholds before the main run. Every attempted fit is exported;
  fitting failures and withheld MCSEs are counted separately. The fit-success
  gate is 100%; calibration is descriptive without a post-hoc pass cutoff.
- Independent dense calculations check positive covariance, identification and
  deletion-maker positivity, derive plug-in bias, and calculate exact Gaussian
  expectations/SDs for all three centerings. The dense oracle matches the local
  exact smoke within 1.4e-13. It is an audit tool, never production code.
- Calibration compares RMS reported MCSE with empirical sketch SD and checks
  numerical 95% intervals against the corresponding exact estimate. Separate
  centered coverage isolates dispersion from finite-probe bias. These are not
  sampling confidence intervals.
- Corrected reports exactly Mean's MCSE for shared probes. Its added increment
  has uncertainty that the reported MCSE omits; the analysis measures that
  increment's dispersion and covariance with Mean directly.
- Homoskedastic errors and this single small network do not establish general
  estimated-mean bias correction under heteroskedasticity or representative-scale
  performance.

## Entry points and evidence

Use the repository's `./.venv/bin/python` locally. `freeze.py RUN_DIR` copies
the committed fesim package and the current fevc runtime into an isolated
snapshot and writes `manifest.json`. Local Mac validation uses a separately
hashed Mac plugin; deployment excludes it. SCC uses Python 3.12.4 and Stata/MP
19. `run.sge RUN_DIR smoke|main` is the real entrypoint; `NSLOTS` bounds the
number of independent one-thread Stata processes.

```bash
./.venv/bin/python -m pytest -q fevc/benchmarks/centering_mc_20261007/test_harness.py
./.venv/bin/python fevc/benchmarks/centering_mc_20261007/analyze.py RUN_DIR
```

The collected local campaign is under `.local/centering-mc-20261007/`.
The SCC root is
`/projectnb/welfgr/vckss/runs/20261007-centering-mc-v2`.
The accepted smoke is job 7919099 (2 slots; 17 seconds; maxvmem 1015.543M).
The main job is 7919475 (14 slots, 30-minute limit, 3 GB/core). It passed all
17,409 fits with zero withheld MCSEs, `failed=0` and `exit_status=0`, on
`dm-pub@scc-gq4`, using 619 seconds and maxvmem 6.439G. The submission
ledger, qacct records, manifests, sanitized logs, raw per-task CSVs and validation
receipts are retained in ignored output. No commit, push, tag or release is made.

The final report is
`.local/centering-mc-20261007/scc/analysis/main/report.html`.
Mean reduces observed exact sampling SD by 39.5% for worker variance and 43.8%
for variance of the sum. Corrected is nearly identical. JLA(256)'s paired mean
deviations from exact are at most .000327. Within-outcome pooled MCSE/SD ratios
range from .951 to 1.050 across both budgets, modes and targets; pooled numerical
95% coverage ranges from 93.7% to 96.1%. Corrected's increment changes empirical
numerical SD by at most .65% in these fixed outcomes. This does not establish
that its omitted increment uncertainty is negligible generally.

The finite Monte Carlo covariance means are about .0022 above truth (about
2.6 simulation SEs for Mean/Corrected). The independently checked dense Gaussian
oracle gives exact expectation equal to truth for all centerings. The raw run is
preserved; repetitions and thresholds were not changed after observing this
Monte Carlo deviation.

Source gates: the 13 focused harness tests and deterministic CMG assembly pass.
The repository Python suite passes 878 tests and rejects the untracked new
benchmark directory in its conservative dirty-bundle guard. The integrated
runner stops at the same guard before its Stata stages. This limitation is
recorded rather than weakening that unrelated packaging guard.

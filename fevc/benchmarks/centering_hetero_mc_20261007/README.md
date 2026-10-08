# Heteroskedastic centering Monte Carlo, October 7, 2026

Owner-requested extension of the earlier [homoskedastic experiment](../centering_mc_20261007/README.md).
This is exploratory evidence for exact/JLA point estimation and numerical MCSE,
not release qualification or sampling-confidence-interval coverage.
The original harness and accepted results remain unchanged.

## Registered design

The same `fesim` AKM/stylized seed, 100 workers, 15 firms, six annual periods,
five-year burn-in, positive sorting, and noise scale 1.5 generate a fixed
worker–firm network. All three variance profiles use the same effects,
histories, standardized Gaussian outcome draws, and paired estimator probes:

| Case | `het_worker` | `het_firm` | `het_interaction` |
| --- | ---: | ---: | ---: |
| Homoskedastic | 0 | 0 | 0 |
| Firm only | 0 | 2 | 0 |
| Worker–firm interaction | 0 | 0 | 2 |

For each full-population latent effect, the score is
`2*(midrank-.5)/population_size-1`. Log variance is the specified linear
combination of worker score, firm score, and their product. The profile is
normalized so the average conditional variance across **generated employed
output observations** is `sd_error^2`. Connectivity and estimator pruning do
not renormalize wages. The actual retained variance vector comes from
`sigma2_true`; the noiseless signal comes from `conditional_mean_true`.
Conditional errors are independent. Heteroskedasticity does not introduce
within-match error correlation.

Each case has 2,000 outcome draws, all three centerings, exact and JLA(256).
Three fixed outcomes per case each receive 300 independent sketches at 64 and
256 probes, with matching exact references. The complete inventory is 52,227
fits. Semantic outcome/sketch keys exclude the variance-profile name so cases
remain paired; task allocation and scheduler order never determine seeds.
Targets, deletion, stayer handling, tolerance, and strict Rust routing match
the earlier experiment.

Before freezing the campaign, an independent dense oracle evaluates each
exact estimator as a quadratic form. For fixed mean `m`, diagonal conditional
covariance `Omega`, and symmetric estimator matrix `F`, its expectation is
`m' F m + trace(F Omega)` and variance is
`2 trace(F Omega F Omega) + 4 m' F Omega F m`.
The same calculation applies to the paired Mean-minus-Corrected contrast.

Firm-only conditional variance lies in the fixed-effect design span. Its
residual projection is zero, so heteroskedasticity alone need not produce
estimated-mean bias. The interaction case has a nonzero residual projection.
The analytic bias may be small relative to each estimator's sampling SD;
paired contrasts, rather than separate sample means, measure the adjustment.
No target is excluded based on empirical significance.

## Validation and evidence

The updated `fesim` working-tree runtime is frozen by installed-file hashes,
with its Git base and existing dirty state recorded. This includes the owner's
pre-existing log-probability work; it is not represented as a clean committed
release. The unchanged `fevc` runtime and platform plugins are independently
hashed. Manifest, source, parameters, seeds, output inventory, and acceptance
rules are fixed before the main run.

Local and compute-node smokes exercise generation, truth/variance validation,
all estimator modes, exports, and receipts. An intentional Stata failure must
be rejected even if the Stata process returns zero. Every attempted fit is
counted. Acceptance requires complete accounting, successful application
markers, unique complete output keys, hashes, sample/target identities, and
100% fit success. Numerical MCSE calibration is descriptive, without a
post-hoc acceptance cutoff.

Reported JLA MCSE is numerical uncertainty conditional on each fixed outcome.
Corrected returns exactly Mean's MCSE for shared probes and excludes uncertainty
in its added increment. The analysis separately reports that increment's
dispersion and covariance with Mean, and empirical full Corrected dispersion.

Use the repository `.venv/bin/python` for `freeze.py`, focused tests, and
`analyze.py`. `run.sge` is the SCC entry point; each reserved slot runs at most
one single-threaded Stata process.

## Execution record

Frozen manifest SHA-256:
`9e719ad1bbb11ef8ae390c505f2728ac540775ae366513e032b4f9d95b5b84fe`.
The remote campaign is
`/projectnb/welfgr/vckss/runs/20261007-centering-hetero-mc-v1`;
local development and collected evidence are under
`.local/centering-hetero-mc-20261007/`.

The final local smoke passed 171 fits in six tasks. Its intentional `r(459)`
test rejected all three cases and returned harness exit 1 despite Stata process
exit 0. The earlier `dev-v1` harness incorrectly expected time values 1–6;
fesim annual output uses 2000–2005. That attempt is preserved, and the corrected
`dev-v2` and final frozen smokes pass. No scientific thresholds were changed.

SCC smoke job **7922309** passed accounting, application, complete output,
fixture, source-hash, and independent oracle checks: 171 fits, no failures or
withheld MCSEs, two slots, 61 seconds, maxvmem 1021.363M, `p-int@scc-pi2`.
The full campaign, job **7922488**, passed the same checks and an independent
final evidence audit: **52,227 unique fits in 48 tasks**, zero fit failures and
zero withheld MCSEs. It used 16 slots, ran for 2,353 seconds on
`p8@scc-pf1.scc.bu.edu`, and recorded maxvmem 7.355G; scheduler failure and exit
status were both zero. All 116 frozen input/harness files match their hashes.
The final receipt is `scc/main.accepted.json`, and tables, figures, raw-data
validation, and the HTML report are under `scc/analysis/main/` in the local
evidence directory. This accepts the registered exploratory execution, without
making a release or platform-qualification claim.

The frozen `fesim` runtime matches the bytes tested by all 77 registered Stata
files, including clean installation, all model/statistical checks, and executable
help/manual examples. Ten static checks and 16 Python tests also pass. A
100,000-row comparison matches all original data columns bitwise against the
preserved pre-change dirty runtime. Five warm timing repetitions gave median
1.973 seconds before and 1.995 seconds after; these are small local diagnostics,
not representative-scale performance claims. Its development record and four
preserved full-suite test/inventory/label failures are in
`/Users/johannes/Git/fesim/build/heteroskedasticity/development-validation.json`.

The new benchmark has 55 focused harness/oracle/analysis tests passing and an
independent source review. The `fevc` minimum source gate reports 878 Python
tests passed and one failure: the existing dirty-source native-bundle guard
rejects untracked benchmark source. `run_checks.py` stops at that same guard
before its Stata stages. CMG assembly verification passes. The guard was not
weakened and no production `fevc` estimator or native payload was changed.

## Findings

The retained design has 573 observations, 99 workers, 15 firms, and true
worker–firm covariance 0.0202503. The original homoskedastic fixture's existing
columns reproduce exactly. Firm and interaction conditional variance ratios
are 41.82 and 40.29. The plug-in covariance averages remain negative in every
profile (approximately −0.035), despite the positive truth; noise therefore
produces substantial plug-in bias as intended.

The independent dense oracle verifies that exact None and Corrected are
unbiased in all three profiles, with point parity within 1.42e-13. Mean is
unbiased for homoskedastic and firm-only noise. Its interaction-profile biases
are small but nonzero:

| Target | Analytic Mean bias | Exact MC Mean − Corrected | Paired simulation SE |
| --- | ---: | ---: | ---: |
| Worker variance | 0.00015083 | 0.00016228 | 0.00001055 |
| Firm variance | 0.00001760 | 0.00001767 | 0.00000082 |
| Covariance | 0.00000269 | 0.00000379 | 0.00000132 |
| Variance of sum | 0.00017382 | 0.00018753 | 0.00001202 |

Every exact/JLA paired centering contrast is within 1.15 simulation SEs of its
analytic expectation. Mean reduces exact worker-variance sampling SD by 39.5%,
37.1%, and 36.5% across homoskedastic, firm-only, and interaction profiles;
corresponding sum-variance reductions are 43.8%, 41.7%, and 35.1%. Corrected
changes these SDs by less than 0.2%. Its bias adjustment is detectable through
pairing but tiny relative to sampling uncertainty; this experiment establishes
no material RMSE improvement from the additional correction.

JLA(256) tracks exact closely: its maximum absolute average paired difference
is 0.000345, and every difference is within 1.12 paired simulation SEs.
Centering roughly halves worker-variance JLA approximation RMSE.

The following ranges pool numerical dispersion within the three fixed outcomes
for each profile, target, and probe budget. They compare RMS reported MCSE
with empirical sketch SD, and numerical 95% intervals with each mode's exact
estimate:

| Centering | RMS MCSE / empirical SD | Numerical 95% coverage |
| --- | ---: | ---: |
| None | 0.951–1.066 | 93.8–96.6% |
| Mean | 0.958–1.066 | 92.9–96.4% |
| Corrected | 0.955–1.061 | 92.9–96.4% |

Calibration is broadly reasonable, with mild undercoverage for interaction
firm variance at 64 probes. Individual fixed-outcome cell ratios span the
wider range 0.912–1.152. Increasing probes from 64 to 256 approximately halves
MCSE (ratios 1.990–2.033). Corrected returns Mean's MCSE and omits the extra
increment's uncertainty; that omission changes empirical numerical SD by at
most 0.645% here, which is not a general guarantee.

Finite-outcome Monte Carlo deviations remain visible and were not tuned away.
For example, firm-profile exact Corrected covariance averages 0.023177 versus
truth 0.020250, a 3.36 simulation-SE deviation; its worker-variance mean is
2.34 simulation SEs low. Covariance deviations in the other profiles are
2.20–2.63 SEs high. Shared standardized outcome draws make these deviations
correlated across profiles. The analytic unbiasedness calculation and precise
paired centering contrasts distinguish this sampling fluctuation from the
small structural Mean bias.

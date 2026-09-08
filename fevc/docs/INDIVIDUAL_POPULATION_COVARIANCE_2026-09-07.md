# Population-covariance diagnosis at 200 JLA probes

## Conclusion

The two selected q1 designs do **not** show the large upward variance-estimation
bias suggested by comparing fitted covariance with the covariance of just 400
saved draws. Exact population calculations largely remove that concern.
Observation total is the clearest case: its average reported remainder
variance is 99.6% of the true variance, but its saved sample variance is only
79.7% of the true variance. Match-total coverage remains 98.25% even when the
interval uses the true population covariance. A new variance fitter is not
justified by these overcoverage rows.

This is a diagnostic result, not qualification. The original development
**FAIL** remains unchanged. No production code, 200-probe setting, gate,
target exclusion, paper, or release artifact was changed in this follow-up.

## Scope and independent reconstruction

The [registration](individual_population_covariance_v1.json) fixes the same
observation `dominant_common_controls`, k=16, and match
`one_mode_equal_independent`, k=20, designs examined in the
[previous follow-up](INDIVIDUAL_INFERENCE_FOLLOWUP_2026-09-06.md).
The former has 536 independent observations with jointly fitted controls;
the latter has 1,200 physical rows collapsed to 400 independent match units,
without controls. These are Gaussian simulation designs. Neither establishes
validity for arbitrary errors, other spectra, or jointly estimated match offsets.

All 400 original outcome vectors per design were reproduced using their
unchanged generators and semantic seeds. **No new outcome draws were used.**
All four targets remain in the diagnosis; covariance remains nonprimary.

A read-only capture in a disposable copy of the frozen Rust source exports
the numerical geometry at replications 0 and 399. All non-timing native
outputs and receipts reproduce the saved calls at scaled tolerance 1e-12.
An independent dense calculation reconstructs all 800 point vectors, 3,200
leading scores and remainders, and all associated recentering corrections.
Maximum scaled discrepancy is 6.66e-11 for observation and 1.78e-14 for match,
below the registered 1e-8 gate. Within-match permutations are checked as
membership sets; they do not alter the ordered collapsed inference units.

The calculation uses the **actual fixed numerical sketches**, not only an
ideal exact-diagonal estimator. Let M be the residual maker, B the plug-in
target matrix, d the captured correction ratio, and sym(C)=(C+C')/2. Then

    K = B - sym(diag(d) M)
    point = y' K y.

For captured leading mode v, eigenvalue lambda, and leave-out multiplier m,
the q1 remainder matrix is

    R = K - lambda v v' + sym(diag(lambda v^2 m) M).

Thus the two q1 coordinates are v'y and y'Ry. For the known Gaussian design
y ~ N(mu, Sigma), their exact population covariance is

    Var(v'y)       = v' Sigma v
    Cov(v'y,y'Ry)  = 2 v' Sigma R mu
    Var(y'Ry)      = 4 mu' R Sigma R mu + 2 tr(R Sigma R Sigma).

Here Sigma is diagonal at the independent inference-unit level. Match
outcomes are sum_i f_i y_i / sqrt(sum_i f_i); their population variances come
directly from the frozen DGP. Target weights remain stored-row target mass,
not target mass multiplied by frequency. Controls enter the observation
design but not the worker/firm target matrices. Dense matrices are confined
to these small synthetic oracles; production remains matrix-free.

Deterministic Gauss-Hermite quadrature independently verifies means,
variances, cross covariances, and fourth central moments. The fourth moments
give the exact Monte Carlo variance of the unbiased sample-variance statistic
for 400 independent draws. No simulated reference outcomes are needed.

## What the population comparison says

Each entry below is a percentage of the true population variance: 100 means
exact agreement. L denotes the leading score; R denotes the remainder.
These are **variance**, not standard-error, ratios.

| Design / target | Mean reported L | Saved-sample L | Mean reported R | Saved-sample R |
|---|---:|---:|---:|---:|
| Observation worker | 101.94 | 90.42 | 100.57 | 89.35 |
| Observation firm | 101.95 | 94.77 | 100.51 | 100.08 |
| Observation covariance, nonprimary | 102.01 | 92.46 | 101.41 | 101.18 |
| Observation total | 99.95 | 102.72 | 99.61 | 79.71 |
| Match worker | 100.00 | 105.00 | 100.01 | 105.76 |
| Match firm | 99.34 | 96.50 | 99.77 | 98.59 |
| Match covariance, nonprimary | 99.62 | 102.74 | 99.83 | 98.85 |
| Match total | 99.62 | 102.75 | 99.94 | 102.16 |

Observation worker's low saved leading/remainder variances are about
1.35/1.50 Monte Carlo standard deviations below population. Observation
total's remainder is 2.86 below population. This is an unusually quiet sample
for that coordinate, not evidence that the reported variance is 25% too big.
These cases were selected after seeing their coverage: these standardized
deviations are descriptive, not unselected hypothesis tests.

The reported means also have sampling uncertainty. For observation worker,
the mean leading-variance ratio is 1.0194 with Monte Carlo SE 0.0128; the
remainder ratio is 1.0057 with SE 0.0034. For observation total's remainder,
the ratio is 0.9961 with SE 0.0034. For match total, the leading and remainder
ratios are 0.9962 (SE 0.0045) and 0.9994 (SE 0.0038).

This does **not** establish perfect estimation of the whole covariance pair.
There are detectable discrepancies in the small cross-covariance terms.
Observation total's population correlation is -0.0217; the correlation implied
by the mean reported covariance matrix is -0.0081. For observation firm the
corresponding values are 0.00485 and 0.00122. The paired true-variance,
exact-trace calculation on the same outcomes detects these differences, so
they should not be dismissed as Monte Carlo noise. Their absolute size is
small, and correcting the full covariance pair does not cure total overcoverage.

The paired calculation evaluates 4(Ry)'Sigma(Ry)-2 tr(R Sigma R Sigma)
and 2v'Sigma Ry for every saved outcome. It separates outcome-dependent
influence variation from replacing the true variances/traces with estimated
ones. For observation total its average remainder-variance ratio is 1.0008,
versus 0.9961 reported. For match total it is 1.0034, versus 0.9994 reported.
Thus neither points to substantial systematic variance inflation.

## Does using the true covariance fix coverage?

The following is an intentionally non-runnable oracle diagnostic: replace
each fitted coordinate covariance with its population value, retain the
original centers and target truth, and recompute curvature and the critical
radius by independent numerical integration. There is no bias recentering.
All 400 draws per target enter both columns.

| Design / target | Original coverage | Population-covariance coverage |
|---|---:|---:|
| Observation worker | 97.75% | 97.00% |
| Observation firm | 96.25% | 97.00% |
| Observation covariance, nonprimary | 98.25% | 98.50% |
| Observation total | 97.75% | 97.75% |
| Match worker | 95.75% | 95.75% |
| Match firm | 96.50% | 96.75% |
| Match covariance, nonprimary | 95.50% | 96.50% |
| Match total | 98.25% | 98.25% |

Match total's mean interval width changes from 0.05340 to 0.05334; observation
total's changes from 1.36762 to 1.36849. Covariance fitting is therefore not
the main explanation for either total's overcoverage. The previous
fixed-covariance radius comparison showed that using 1.96 instead of the q1
radius lowered match-total coverage by one percentage point. Together these
results support a role for the conservative q1 construction and finite-sample
variation. They do not establish its limiting coverage or license replacing
the q1 radius with 1.96.

The actual 200-sketch point variance differs from the ideal exact-diagonal
variance by at most 0.17% across these eight targets. Fixed-sketch point bias
is at most 0.0312 population standard deviations. Increasing JLA probes is
not indicated by this comparison. Observation total is itself diffuse
(leading share about 0.046), whereas match total is strongly one-mode
(about 0.985); a cell's q1 label does not make every target one-mode.

As a supplementary check, the mean raw scalar point-variance estimates are
98.9--101.3% of population across the six primary design/target combinations.
This is not uniform across every target: observation covariance's scalar
estimate averages 87.1% (Monte Carlo SE 3.8 percentage points). That target is
nonprimary and strongly one-mode, so this is not a qualified q0 normal-reference
case. It remains a limitation of any broader claim, not an exclusion from the
q1 diagnosis. These scalar means include all 400 draws, including nonpositive
estimates, and do not condition on joint admission. Their formula and values
are in the compact receipt.

All 3,200 q1 coordinate pairs are available. The optional full joint matrix is
available in 389/400 observation calls and 400/400 match calls. Its separate
descriptive mean is explicitly conditional on availability; no q1 analysis
conditions on those joint-matrix admissions.

## Recommendation and boundary

Freeze the present estimator and 200-probe setting. The next owner decision
should be a prospectively registered, higher-replication validation of the
flagged cases, not another variance-fitter rewrite. It should retain the
small cross-covariance discrepancies as diagnostics and assess coverage and
width together. The population oracle is a simulation check, not software
that readers could use with unknown error variances.

The old development failure cannot be relabeled. Under the current contract
it blocks confirmation and paper/release promotion. If persistent q1
conservativeness proves to be expected and acceptable for the intended
method, any change to its acceptance contract requires an explicit,
scientifically justified prospective decision and separate new evidence—not
a retroactive relaxation of the observed failing cutoffs.

## Reproduction and engineering

Source: `rust/experiments/individual_population_covariance/`.
The [compact receipt](individual_population_covariance_v1_result.json) binds
the full ignored evidence under
`.local/diagnostics/individual-population-covariance-20260907-final/`.

Commands, from the repository root:

```sh
./.venv/bin/python -m pytest -q rust/experiments/individual_population_covariance/test_audit.py --basetemp=.local/diagnostics/population-covariance-unit-tests-20260907-final
./.venv/bin/python rust/experiments/individual_population_covariance/run.py .local/diagnostics/individual-inference-upgrade-20260906/public-development-1 .local/diagnostics/individual-population-covariance-20260907-final
./.venv/bin/python rust/experiments/individual_population_covariance/audit.py .local/diagnostics/individual-population-covariance-20260907-final
./.venv/bin/python -m pytest -q
./.venv/bin/python fevc/cmg/tools/assemble.py --all --check
./.venv/bin/python fevc/tools/run_checks.py
```

Use a fresh output directory to reproduce; existing evidence is not overwritten.
The 17 focused diagnostic tests and 733 package Python tests pass. CMG
assembly is unchanged and passes. Integrated local checks, including Stata
quick/full and isolated installation, pass; the exact log is bound in the
compact receipt. Python is 3.13.0, NumPy 2.5.2, Rust 1.85.1; no SciPy
or external compute is used. The previously recorded 114 Rust/build source
paths are byte-identical. No new platform qualification is claimed.

Preflight logs are retained separately. The first build found that the old
bundle omitted `cmg_impl.rs.in`; its unchanged tracked template was supplied
with an explicit hash and source commit. A second build corrected only library
discovery for Cargo's un-hashed plugin filename. The first complete capture
exposed the validator's within-match order and nullable-joint-matrix assumptions;
both were corrected without changing an estimator or numerical result. Four
native preflight calls and four final calls reused the same four saved
cell/replication keys. All original failures and preflight logs remain intact.

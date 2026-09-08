# Observation inference: residual-moment variance candidate

## Decision

Develop an internal observation-only residual-moment variance fitter. It
addresses the diagnosed shrinkage failure in the original difficult cases,
with substantially fewer negative predictions than simply removing the
current penalty. A projected-Gaussian calculation supplies a feasible
matrix-free route to its small moment matrix.

This is **promising development evidence, not qualification**. The original
observation confirmation remains FAIL. Production code, inference options,
the point estimator, q1 algebra, the accepted match route, and release scope
are unchanged. No paper changes or external jobs were undertaken.

The [initial diagnosis](OBSERVATION_VARIANCE_FIT_DIAGNOSIS_2026-09-06.md)
identified the variance-learning step. The
[machine-readable development result](observation_variance_remedy_development_v1_result.json)
records all targets, failures, source identities, paired comparisons and
the local diagnostic archive.

## Candidate and its interpretation

Write the specified error-variance vector as s = Z gamma, where Z contains
the existing three leverage-only or fifteen common variance features. Let
P be the full-model projection, M = I-P, and e = My. Fit gamma through

```text
K = Z' (M elementwise-squared) Z
gamma_hat = inverse(K) Z' (e elementwise-squared)
s_hat = Z gamma_hat.
```

The reason is exact: E[e_i^2 | X] = sum_j M_ij^2 s_j. Consequently,
E[Z'e^2 | X] = K gamma. With a correctly specified variance model and
invertible fixed K, the untruncated coefficient and variance predictions are
unbiased. This moment identity does not require Gaussian outcome errors.
It does not, by itself, establish exact finite-sample confidence intervals.

For a constant variance model, the method reduces to the familiar residual
sum of squares divided by n minus the regression rank. The general version
allows the specified heteroskedasticity while accounting for how estimation
of the fixed effects mixes the residual variances. Simply regressing raw or
HC2-adjusted residual squares on Z would not use this exact heteroskedastic
moment relationship.

Because MX = 0, changing the fixed-effect signal while holding errors fixed
does not alter this variance fit. This removes the signal-times-residual noise
that dominated the old leave-out responses in the difficult design.

The candidate uses all residuals, without claiming independent cross-fitting.
Its positive predictions enter inference covariance only; the raw leave-out
point correction and leading-mode recenter remain unchanged. Negative
predictions are floored at the predeclared 1e-8 times median residual-positive
scale. An entirely floored fit, rank failure, or nonpositive covariance is
counted as failure, never silently discarded. Truncation can introduce bias;
it is measured explicitly rather than included in the unbiasedness claim.

## Paired comparison on original draws

All entries use the same original 2,500 draws per case and the unchanged q1
interval machinery. The residual-OLS comparator regresses e^2 on
D = (M elementwise-squared)Z within the original five outer-fold training
sets, then predicts Z gamma on validation rows. Its folds do not create
independent split fixed-effect fits.

| Firm-variance case | Original penalized fit | Unpenalized leave-out fit | Cross-fitted residual OLS | Full residual moments | True variances |
| --- | ---: | ---: | ---: | ---: | ---: |
| Dominant leverage, q1 | 93.52% | 94.64% | 95.96% | 95.88% | 95.84% |
| Dominant common, q1 | 94.68% | 94.40% | 95.56% | 95.72% | 96.12% |
| Diffuse leverage, q0 | 95.16% | 95.12% | 95.12% | 95.20% | 95.16% |

Every arm produces all 2,500 firm-target intervals in each of these cases.
For the full-moment candidate, empirical SD / mean SE is 1.0252, 1.0300 and
0.9963, respectively. Mean estimated variance / exact population variance is
1.0006, 0.9977 and 1.0010. This closes the earlier 13.1% and 10.5% variance
shortfalls relative to population truth in the two dominant cases.

The paired q1 coverage gains over the original procedure are 2.36 percentage
points (MCSE 0.319 pp; 62 gains, three losses) and 1.04 pp (MCSE 0.276 pp;
37 gains, eleven losses). These are exploratory, post-result comparisons,
not independent confirmation. Small differences between the two residual
methods are not evidence that one has definitively better coverage.

## Why not just remove the penalty?

Removing the penalty restores the elevated bridge variances, but leaves the
old variance responses noisy. In the common case, 2,498 of 2,500 replications
then produce at least one nonpositive variance prediction: 117,248 predictions
out of 1,340,000. Flooring them changes the estimator materially.

Cross-fitted residual OLS reduces those counts to 1,460 replications and
12,346 predictions. Full residual moments reduces them further to 171
replications and 843 predictions, about 0.063% of all predictions. In the
leverage case both residual methods have zero negative predictions; the
unpenalized leave-out fit has 3,043 across 237 replications.

The full-moment candidate estimates bridge variance at 1.4313 versus truth
1.4324 in the leverage case. In the common case it estimates 1.2594 versus
truth 1.2492. Its advantage is the lower-noise response and small moment
system, not an empirical SE multiplier or a changed confidence level.

## Bounded robustness checks

The extension was recorded before its outcomes were inspected. It keeps the
methods fixed and reuses 500 original draws in each additional case.

| Additional firm-target case | Full-moment coverage | True-variance coverage | Full-moment availability |
| --- | ---: | ---: | ---: |
| Dominant common, k=12 | 96.0% | 96.0% | 500/500 |
| Dominant common, t8 errors | 96.8% | 97.0% | 500/500 |
| Dominant common, joint controls | 95.6% | 95.6% | 500/500 |
| Diffuse common | 95.8% | 95.8% | 500/500 |
| Diffuse common, t8 errors | 95.4% | 95.2% | 500/500 |
| Dominant, mild functional misspecification | 95.6% | 95.0% | 500/500 |
| Dominant, mild omitted variance driver | 97.2% | 96.0% | 500/500 |
| Dominant, severe omitted variance driver | 98.8% | 94.6% | 500/500 |
| Dominant common, zero signal | 50.2% of all attempts | 50.4% of all attempts | 254/500 |

With 500 replications, coverage near 95% has Monte Carlo SE around one
percentage point. These checks support further development, not precise
claims of calibration under each perturbation. Joint controls here refer to
the existing observation-level full-model fit; this is not a correction for
estimated fixed offsets in match inference.

Misspecification remains consequential: the severe omitted-driver case
reports about 2.26 times the true firm-target variance, so its high coverage
is not good calibration. The zero-signal case still often fails the current
variance/covariance positivity requirements, just as the oracle does. Its
98.82% coverage among successful candidate intervals must not be presented
without the 50.8% availability and 50.2% all-attempt coverage.

All four targets are retained in the JSON and raw outputs. The deliberately
multi-mode covariance targets remain outside the q1 coverage claim. The
candidate does not remedy every covariance-target or null-signal failure.
The public command's additional full-joint-PSD boundary is not exercised by
this target-local harness.

## Matrix-free feasibility

Forming M elementwise-squared is useful for the independent small oracle,
but is not an acceptable production implementation. A different identity
avoids that matrix. For independent numerical probes g distributed N(0,I),

```text
K = Z' diag(1 - 2 diag(P)) Z
    + 0.5 Cov_g { Z' [(P g) elementwise-squared] }.
```

This follows from M_ij^2 = delta_ij(1-2P_ii)+P_ij^2 and the covariance of
squared Gaussian projections. The first term is accumulated directly; the
second needs one projection action per Gaussian probe and a covariance matrix
of dimension three or fifteen. Probe vectors can be processed in batches.
The normal probes are numerical integration devices, not a Gaussian-outcome
assumption. The Gram computation accepts a projection-action callback and
does not require a stored observation-by-observation matrix.

An independent dense identity check, zero-projection check and probe-batch
invariance check pass. With a single fixed outcome-free probe stream per
design, 512 probes yield whitened Gram operator errors of 0.48%, 0.94% and
0.43% across the three original cases. At 2,048 probes the errors are 0.41%,
0.52% and 0.21%. Both predeclared counts are reported; no count was selected
using coverage.

The 512-probe firm coverages are 95.92%, 95.72% and 95.24%; at 2,048 probes
they are 95.88%, 95.76% and 95.16%. These are close to the dense candidate.
This is an algorithmic feasibility check, not large-data timing or native
qualification. It uses exact leverages, exact projection actions and one
fixed numerical-probe stream per design. Integration must separately address
approximate leverage, solver errors, high-leverage designs, multiple probe
streams, conditioning, memory and Counter-V1 independence/accounting.

## Next implementation checkpoint

1. Specify the residual-moment fitter as a distinct internal observation
   candidate; do not silently redefine the existing named fitted-variance
   methods or change the accepted match route.
2. Implement the small moment system and projected-probe calculation through
   existing solver/RNG machinery, with explicit conditioning, positivity,
   memory, numerical-probe and failure diagnostics. Retain independent dense
   oracles and the constant-variance residual-df identity.
3. Compare observation and match deletion on the same weak graph and outcome
   draws before attributing differences to deletion itself. No such paired
   deletion experiment has yet been performed.
4. Freeze the feasible candidate and register an independent confirmation
   covering the original supported q0/q1 targets and diagnostic failures.
   Preserve all-attempt availability gates, misspecification limits and the
   original failed result. Qualification of a changed public/native route
   then needs its affected-surface source, Stata and plugin gates.

## Provenance and validation

Scientific and production source remains
`1aeed0651fc3d85a4d9ea2a3c6cdf8e147ddc0b5`. The relevant core, RC observation
harness and deterministic q1 reference are unchanged from confirmation source
`73fa758`. The existing raw aggregate SHA-256 is verified before paired
comparison. The Rust helper uses the unchanged source-bound generator and
q1 routines; NumPy separately constructs residual and quadratic kernels and
fits the candidate variances.

The dense comparison contains 192,000 target-arm attempts: 188,166 successes,
3,367 nonpositive variance failures and 467 q1 covariance failures. The probe
comparison contains 60,000 attempts: 59,964 successes and 36 nonpositive
variance failures. The full failed-cell/type inventory is retained. This is
12,000 original simulated datasets evaluated under several arms, not 252,000
independent newly generated datasets.

All semantic seeds and keys reconcile, and successful point estimates match
the archived originals within maximum absolute difference 1.994e-13. Twelve
focused tests pass, covering malformed/missing/duplicate/nonfinite inputs,
truncated binary rejection, explicit fit-failure counting, rank rejection,
noiseless recovery, independent matrix identities, mean-shift invariance,
the homoskedastic residual-df special case, prior oracle reproduction and
numerical-probe checks. The repository Python suite passes all 726 tests in
69.92 seconds; the CMG assembly consistency and whitespace checks also pass.
Rust 1.85.1, Python 3.13.0 and NumPy 2.5.2 were used, with a locked offline
release library build. No native/plugin or public Stata qualification is
claimed for these scratch diagnostics.

Plans, programs, synthetic outcome exports, fitted vectors, complete results
and tests are preserved in the ignored local archive
`.local/diagnostics/observation-variance-remedy-20260906.tar.gz`, with SHA-256
`18ec9eb12c507c582e3e1db1d242dba3381ecc8e9e999283b58615c1bc617b18`.
Its gzip integrity passes. The result JSON binds individual diagnostic source
and output identities. The archive is reproducible exploratory material,
not an immutable confirmation campaign or public package payload.

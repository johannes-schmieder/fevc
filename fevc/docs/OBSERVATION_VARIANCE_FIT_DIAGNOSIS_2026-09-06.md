# Observation-inference shortfall: variance-fitting diagnosis

## Conclusion

The two troublesome observation-level q1 firm-variance cases have a concrete
variance-fitting problem. Replaying the original outcomes with the true error
variances supplied, while retaining the point estimator and q1 interval
calculation, removes their undercoverage and most of their SE-ratio gap. The
fitted variance model substantially understates the error variances on the few
observations carrying most of the sampling uncertainty.

This is a post-result causal diagnostic, not a new confirmation or a repaired
estimator. The original confirmation remains **FAIL**. No production source,
public interface, paper, release decision, or frozen evidence is changed.
The [machine-readable results](observation_variance_fit_diagnosis_v1_result.json)
bind the inputs, diagnostic sources, and checks.

## Controlled replay

Each row below uses the original k=16 design and all 2,500 original Gaussian
outcome draws, with the original semantic seeds and replication indices.
Only the variance information supplied to inference changes. The population
arm replaces the entire leading-score/remainder covariance matrix by its exact
population value; the oracle arm supplies true observation variances but keeps
the existing random-influence covariance estimator.

| Firm-variance case | Original fitted q1 coverage | True-variance q1 coverage | Fixed-population q1 coverage | Original SD / mean SE | True-variance SD / mean SE |
| --- | ---: | ---: | ---: | ---: | ---: |
| Dominant leverage | 93.52% | 95.84% | 95.76% | 1.1012 | 1.0224 |
| Dominant common | 94.68% | 96.12% | 96.08% | 1.0877 | 1.0286 |

The paired coverage improvements are 2.32 percentage points (MCSE 0.322 pp;
62 gains, four losses) and 1.44 pp (MCSE 0.276 pp; 42 gains, six losses).
These are descriptive, post-selection Monte Carlo comparisons, not fresh
qualification tests. The modest oracle overcoverage is compatible with the
at-least-nominal q1 reference; it is not evidence of exact finite-sample
calibration in every design.

For leverage, mean reported variance is 86.91% of the **exact population
variance** under fitting and 100.21% with true variances. For common the
corresponding figures are 89.46% and 99.19%. The earlier 84.65% and 86.71%
figures used empirical variance as the denominator, which also has Monte
Carlo error. These denominators must not be conflated.

The diffuse-leverage q0 control has 95.16% coverage both originally and with
true variances (four gains and four losses); its oracle SD / mean SE is 0.9961.
The registered observation failure was an SE-ratio gate, not a failure of its
coverage tolerance. All primary observation q0 rows already passed.

## Where the variance fit goes wrong

Both dominant designs have 536 observations, 31 regression coefficients, 144
distinct worker-firm matches, and no nuisance controls. Two dense halves of
the mobility graph are connected by only 16 one-observation bridge matches.

| Diagnostic | Dominant leverage | Dominant common |
| --- | ---: | ---: |
| Bridges' share of exact sampling variance | 72.26% | 69.63% |
| Mean true error variance on bridges | 1.4324 | 1.2492 |
| Mean fitted error variance on bridges | 1.1814 | 1.0877 |
| Bridge variance understatement | 17.52% | 12.93% |
| Mean fitted variance over all observations (true mean = 1) | 0.9997 | 1.0027 |
| Outer-fold fits selecting ridge penalty at least 1 | 90.54% | 98.60% |

The variance learner estimates the overall level very well but smooths away
the elevated variances at the crucial observations. Its nested cross-validation
criterion weights prediction errors equally across observations, whereas the
sampling uncertainty is concentrated on a tiny subset. A good average
variance prediction need not be good for inference on this target.

Correct specification is not the issue: supplying true variances as the
regression response recovers the true variance functions, including out-of-fold
predictions, to maximum absolute errors 9.77e-15 and 1.39e-13. Nevertheless,
the fitted procedure chooses its penalty from noisy outcomes. The largest
registered penalty, 100, is selected in 58.26% of leverage-model fits and
80.41% of common-model fits. This shrinks standardized nonconstant coefficients
toward zero. Floors affect only 939 and 3,657 of the respective 1,340,000
predictions; the mechanism is not widespread truncation at zero.

The raw leave-out response is unbiased but noisy. In the leverage case its
average on the bridges is 1.4315, close to the true 1.4324, before regression
reduces the fitted value to 1.1814. The RMS standard deviation of these raw
responses across observations is 4.413, versus a mean variance of one.
Under this Gaussian DGP, about 89% of their total variance comes from the
signal-times-residual term. The diffuse control's corresponding figures are
1.765 and 32.1%. Thus this is also a much noisier variance-learning problem
than the passing diffuse observation case.

These facts strongly implicate shrinkage and its data-dependent selection.
An explicit penalty ablation has not yet been run, so this diagnosis identifies
the variance-fitting step causally but does not separately identify every
contribution of penalty selection, regularization, and response dependence.

## Separating prediction bias from covariance-estimation effects

Let the zero-diagonal leave-out kernel be C, the conditional mean be mu, and
the independent error covariance be Sigma = diag(s), where s contains
variances. Then

```text
theta_hat = y' C y
Var(theta_hat) = 4 (C mu)' Sigma (C mu) + 2 tr(C Sigma C Sigma)
V_hat(t) = 4 sum_i (C y)_i^2 t_i - 2 tr(C diag(t) C diag(t)).
```

For fixed t=s the last expression is unbiased. With fitted, random t,
correct specification of its regression is not sufficient to preserve this
property. In particular, the ordinary leave-out point-estimation assumptions
do not by themselves establish valid feasible component inference. KSS
Section 5.2 explicitly distinguishes unbiased variance proxies from the
additional variance-product and sample-splitting requirements for inference
([Kline, Saggio, and Solvsten, 2020](https://eml.berkeley.edu/~pkline/papers/KSS2020.pdf)).
FEVC's structured fit is a separately restricted approximation, not that
unrestricted construction; excluding responses from the small variance
regression does not create independent split FE predictions.

To gauge the importance of mean prediction bias, replace t by the
across-replication mean fitted vector and evaluate E[V_hat(t)] using exact
population moments. This retains a 13.29% variance shortfall in the leverage
case, almost the observed 13.09%. For common it retains 9.74%, versus the
observed 10.54%. Mean prediction bias therefore accounts for essentially all
of the first gap and most of the second. The remaining differences include
random-fit dependence/product effects and finite-replay error; they are not
exact isolated estimates of a dependence bias.

## Why match deletion appeared better

The accepted match confirmation is not a controlled deletion-rule comparison.
It uses a complete 20-by-20 worker-firm grid with 400 independent collapsed
matches. Its one-mode target is created by multiplying the target weight of
one match by 1,000, not by a weak mobility bridge. Its correctly specified
aggregate variance is a simple function of match mass, constant when masses
are equal. Its mean function and signal scale also differ.

It therefore does not subject the variance learner to the same conjunction
of bottleneck observations, elevated bottleneck variances, and very noisy
variance proxies. Its success is consistent with this diagnosis, but does
not establish that collapsing to matches automatically cures it. A weakly
connected match graph can also concentrate uncertainty on a few matches.

Likewise, millions of observations are not by themselves the relevant remedy:
adding observations inside the two dense halves while retaining only a few
bridges need not provide much new information about the influential variances.

## Bounded next step

1. On these same designs and draws, compare the current variance fit with an
   unpenalized, correctly specified low-dimensional fit. Count negative
   predictions, covariance failures, and interval failures; do not assess
   coverage only among successful calls. This isolates the shrinkage mechanism
   without changing q1 algebra or nuisance handling.
2. If noisy leave-out responses make that unstable, diagnose a residual-based
   structured variance fit with the correct moment transformation. For example,
   E[e_i^2 | X] = sum_j M_ij^2 s_j. Under s=Z gamma the fitted mean must use
   (M elementwise-squared) Z gamma, not naively regress leverage-adjusted
   residual squares on Z under heteroskedasticity. This is a proposed diagnostic,
   not an implemented or qualified fix.
3. Evaluate observation and match deletion on the same weak graph, mean,
   target, and independent observation-error draws before attributing any
   remaining contrast to deletion or collapse. New confirmation comes only
   after choosing and freezing a justified feasible method.

## Reproduction and validation

The local scratch directory is
`/private/tmp/fevc-obs-causal-diagnostic.NAFPug`. Its `plan.json` recorded the
bounded replay scope before results. `probe.rs` preserves the original
observation harness and adds a diagnostic entrypoint. Original core and
harness paths are unchanged between confirmation source `73fa758` and
current source `1aeed0651fc3d85a4d9ea2a3c6cdf8e147ddc0b5`.

The library was checked by an offline, locked release build under Rust 1.85.1.
The scratch probe was compiled against that library. Each of the two fitted
2,500-replication replays took approximately 91 seconds with both running
locally; no SCC, AWS, licensed Stata, or new simulation campaign was used.
The diffuse control was replayed with oracle variances only and compared
with archived fitted outputs.

Validation checks all replication indices, semantic seeds, target inventory,
success counts, and original raw SHA-256. No fit, q0-variance, q1-covariance,
or interval failures occurred in the replay arms. The regenerated fitted
points, SEs, widths, and coverage reproduce the originals; maximum absolute
differences are 1.74e-13, 7.11e-15, and 3.51e-14, with no coverage mismatches.
Independent NumPy dense matrices verify the target, zero-diagonal kernel,
full population variance, and leading/remainder population covariance;
relative error in the full variance is below 3.4e-14. Python 3.13.0 and
NumPy 2.5.2 were used. `finalize.py` reruns the analyses and writes the result.
The JSON records exact commands and source/output hashes. Scratch programs
and output remain diagnostic material, not committed production code or an
immutable confirmation archive. Full package and platform gates are not
rerun for this diagnosis.

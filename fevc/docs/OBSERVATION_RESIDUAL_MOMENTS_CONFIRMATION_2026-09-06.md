# Full native observation confirmation: calibration improves, availability fails

The internal residual-moment candidate passes every predeclared bias,
coverage and standard-error calibration check in the correctly specified
primary cases. The complete confirmation nevertheless **fails**: the
heavy-tailed dominant-mode design returns inference in 98.28% of samples,
below the required 99%. These are three failed target gates arising from
one shared availability problem, not three different calibration failures.

The estimator, public Stata/FFI options and fixed-offset match RC were not
changed during this experiment. All earlier failures remain recorded.

## Experiment and accounting

The prospective
[`observation_residual_moments_confirmation_v1.json`](observation_residual_moments_confirmation_v1.json)
retains all 20 original design/dimension entries, with 2,500 fresh outcomes
per entry. An immutable source bundle, exact executable, semantic task keys,
numerical settings and original scientific gates were frozen before outcomes.
The 200 local tasks completed 50,000 native calls, 200,000 native target
attempts and 200,000 paired exact-reference attempts. The reference uses the
**same fitted variances**; it is not another independent Monte Carlo sample.

There were 46,209 successful native calls and 184,516 available native target
intervals. All 3,791 whole-call failures occurred at the existing joint
covariance PSD check. Another 320 q1 target-local failures occurred only in
the dominant zero-signal diagnostic. Nothing was silently dropped. On a
whole-call failure the paired exact output is marked unavailable because
the native fitted variances were not exported; it is not called an
independently observed exact-reference failure.

The run used numerical seed `326417`, 3,200 JLA probes, 512 residual-moment
Gram probes, 1,000 covariance probes, 128 spectrum probes, at most 512
spectrum iterations with tolerance 0.002, 100,000 q1 critical-value draws
and batch width 16. Solver and material-PSD tolerances were unchanged.
Outcome master `1956048372195603` differs from the pipeline master and all
earlier campaigns. Numerical draws were fixed across outcomes; this is
conditional evidence for one numerical stream, not a seed-tail study.

Exact outcome-free preflight verified the intended diffuse, one-mode and
deliberately multi-mode regimes, positive true variances, deletion support,
and identified exact/numerical variance bases in every case. The labels
"correct" refer to the original exact-design variance basis. Finite JLA
inputs approximate that basis: the recorded relative true-variance projection
error is about 3.6--8.0% in the nonconstant correctly specified cases, and
essentially zero in the homoskedastic case. No case was relabeled after this
check or after outcomes. Actual numerical leverage, target diagonals and
Gram matrices were identical across every successful outcome and shard.

## Calibration and the failed gate

All 39 correctly specified primary design--target rows pass their bias,
coverage and SD/mean-SE checks. Coverage among available intervals ranges
from 93.72% to 96.21%; empirical SD divided by mean reported SE ranges from
0.972 to 1.031. The deliberately multi-mode q1 covariance target remains
outside the primary coverage claim, exactly as registered.

Selected firm-component results at the larger design dimension:

| Case | Reference | Interval availability | Coverage when available | Coverage across all attempts | SD / mean SE |
| --- | --- | ---: | ---: | ---: | ---: |
| Diffuse common | q0 | 100.00% | 95.00% | 95.00% | 0.979 |
| Dominant leverage | q1 | 99.60% | 96.14% | 95.76% | 1.010 |
| Dominant common | q1 | 99.48% | 95.90% | 95.40% | 1.002 |
| Dominant common, two joint controls | q1 | 99.76% | 95.79% | 95.56% | 1.017 |
| Dominant common, t8 errors | q1 | **98.28%** | 95.93% | 94.28% | 1.015 |

"Coverage across all attempts" counts an unavailable interval as not covering;
it does not substitute for the separately registered availability gate.
The t8 case has 43 rejected calls out of 2,500, exceeding the permitted 25.
The complete failure list is:

- `dominant_common_t8/16/worker: success rate`
- `dominant_common_t8/16/firm: success rate`
- `dominant_common_t8/16/total: success rate`

All mild-misspecification gates pass; primary coverage there ranges from
92.55% to 96.08%. Severe omitted-driver cases expose the intended limitation:
total-component coverage falls to 85.32% in the diffuse case and 83.12% in
the dominant case. This is evidence that the variance-model restriction
matters, not evidence of unrestricted heteroskedastic robustness.

Zero-signal cases remain diagnostics, not qualification. Diffuse q0 interval
availability is 79.88%. Concentrated q0 availability is 36.96%. In the
dominant q1 null case, worker and firm availability is 34.52%, covariance
availability is 24.48%, and total availability is 35.44%. Conditioning on
the surviving intervals would conceal a substantial limitation. These
results do not support general tests of zero variance components.

## Exact diagnosis of all 43 heavy-tailed rejections

A separate, source-bound existing-draw audit selected every rejected t8 draw
and the first successful draw as a reconstruction check. It did not change
the confirmation executable, outcomes, probes, gates or interval status.
Independent dense fitted predictions match the successful native check
within `1.47e-12`; its original 1,000-probe primitive covariance matches
within `4.48e-14`.

| Calculation on the 43 rejected draws | Joint covariance admissible |
| --- | ---: |
| Original fitted variances and 1,000 probes, independently reconstructed | 0 / 43 |
| Same fitted variances and native kernel, exact trace | 0 / 43 |
| Same fitted variances, exact point kernel and exact trace | 0 / 43 |
| True error variances, native kernel and exact trace | 43 / 43 |

Thus neither more covariance probes nor an exact point calculation removes
these rejections. They arise from finite-sample fitted-variance effects in
the shared joint covariance estimate. The true-variance comparison still
uses the same **realized-outcome covariance estimator**; it is not the
population sampling covariance. Thirteen rejected draws have some floored
variance predictions, but all 43 fail even with exact traces: flooring alone
does not describe the complete failure set.

Every rejected draw nevertheless has positive worker, firm and total
marginal variance estimates, and an admissible worker--firm principal
covariance block, under both the original probes and exact traces. The
whole-matrix validity rule therefore withholds some potentially usable
marginal information. This is a reason to investigate target-specific
inference, **not** proof that those q1 intervals are valid: q1 requires its
own leading-coefficient/remainder covariance and interval checks.

## Numerical checks, source compatibility and limitations

Among 184,426 pairs with both intervals available, native and exact results
disagree on coverage in 214 cases (0.116%). The largest direct native-kernel
point identity error is `1.91e-11`, and the largest rank-one remainder
identity error is `2.42e-11`. The largest primitive covariance-diagonal error
is 3.65 reported numerical MCSEs; the largest critical-value discrepancy
from quadrature at the same curvature is 0.00981. Repeated use of the fixed
numerical stream is not treated as independent numerical replication.

The full tiny generator--validator--receipt path, split/reversed task checks,
50 confirmation-harness tests, 10 generated numerical-oracle tests, 726
package Python tests and CMG assembly passed before confirmation. Four
additional independent algebra tests cover the existing-draw audit.
Malformed, missing, duplicated, partial, wrong-source/profile/seed outputs
and scientifically failed aggregates are tested. A deliberately invalid
native command exits 101; the completed scientific failure exits 2. The
initial package test run caught a registration-label naming conflict; it was
corrected before confirmation, and both logs are retained. Pytest temporary
directory cleanup warnings are not test failures.

All 67 source entries registered by the preceding integration manifest
`f60eab471443ca2e30b41db98ed95c1d8c518e38f250078b32f92f26c9bc88fd`
remain byte-identical. Only new experiment orchestration, diagnosis and
documentation were added this turn. Previous Rust/native/Stata checks carry
only their unchanged-production-surface claims; they do not qualify this
new experiment executable or a new public option. The source is the
preserved dirty worktree based on `1aeed0651fc3d85a4d9ea2a3c6cdf8e147ddc0b5`,
not a clean-SHA release candidate.

Execution used rustc 1.85.1 (`4eb161250`, LLVM 19.1.7), Python 3.13.0 and
NumPy 2.5.2 on macOS 26.6.2 arm64. The confirmation, including aggregation,
took about 71.5 minutes. Designs contain 307--922
observations and 23--33 coefficients. This is not a large-data or standalone
latency benchmark and does not replace the earlier limited preparation-only
scaling exercise.

The confirmation manifest is
`7254b584b0ab9debd8d5d3351f1e1f1b17677ff4ad770a1ae729f542ad82a926`;
its source bundle is
`63b9081a49cf40ef65ff6bdd306346cab46f0a29de35c68e296b5d380f3586df`.
The [machine result](observation_residual_moments_confirmation_v1_result.json)
records the executable, complete result, diagnostic and archive identities,
all 80 native and 80 paired-reference summaries, and all failed gates.
Full per-call failures remain in the archived raw result. Reproduction lives
in `rust/experiments/residual_moment_confirmation/` and the separate
`rust/experiments/residual_moment_confirmation_audit/`.

## Next bounded step

Keep the residual-moment fitter and numerical budgets fixed. First diagnose
the target-specific q1 covariance matrices and intervals on the rejected
draws, against independent exact calculations and successful comparison
draws. If the primary marginal intervals are admissible, assess an explicit
marginal-only reporting contract that does not certify or repair the
indefinite full covariance. Positive marginal variances alone are insufficient.
Do not add a ridge, force a PSD matrix, waive the existing gate or select a
new seed to turn this campaign into a pass. Any implementation change needs
a prospective contract and separate fresh validation.

The original observation SE-ratio failure and the later exact-input
joint-control overcoverage failure remain unchanged. This new result is
strong internal evidence of improved calibration, not public promotion.
No paper, public match interface, commit, push, tag, Linux/Windows run or
release was changed or undertaken.

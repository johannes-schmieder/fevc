# Existing-output review of the observation SE-ratio failure

## Conclusion and release recommendation

The very small crossing of the registered 1.10 boundary is not distinguishable
from Monte Carlo noise at this run's precision. The broader shortfall in
reported uncertainty is more substantial: empirical SD is about 10% above
mean reported SE in the failed design. These are different conclusions.
Neither changes the original confirmation's **FAIL** status.

There is no new evidence here of an algebraic or numerical implementation
defect. Nor does this review establish the cause of the remaining calibration
gap. It does not justify deleting the leverage-only row or declaring
structured-common inference an established fix: a similar one-mode firm
pattern appears under that model too.

The recommended next action is an explicit owner decision to let the
separately confirmed fixed-offset match q0/q1 interface work proceed, while
retaining the observation-q1 shortfall as an unresolved RC limitation and
withholding a claim of fully passed corrected-observation qualification.
This would amend the prospective RC integration prerequisite, not waive or
rename the failed scientific result. It is a recommendation, not an approved
scope change. The final candidate review must decide how that observation
route is labeled or restricted before any release. No new simulation,
estimator search, joint-controls work, public-interface change, or release
action is performed in this review.

## What the numbers mean

The diagnostic ratio is `R = empirical SD / arithmetic mean reported SE`.
It compares how much the point estimate actually varies across independent
simulated datasets with the average uncertainty reported by the estimator.
A ratio above one means average SE is smaller than empirical dispersion;
the registered upper allowance is 1.10. Each row below uses all 2,500
original replications, without new draws or deletion of influential outcomes.

| Firm-variance target | R | Monte Carlo SE of R | Approximate 95% MC interval | Interval coverage |
| --- | ---: | ---: | --- | ---: |
| Observation q1, dominant leverage, k=16 | 1.1012 | 0.01675 | [1.0689, 1.1345] | 93.52% |
| Observation q1, dominant common, k=16 | 1.0877 | 0.01622 | [1.0564, 1.1200] | 94.68% |
| Observation q1, dominant common, k=12 | 1.0313 | 0.01419 | [1.0039, 1.0595] | 94.84% |
| Observation q0, diffuse leverage, k=16 | 0.9960 | 0.01399 | [0.9689, 1.0238] | 95.16% |

These MC intervals describe uncertainty in simulation performance measures,
not confidence intervals for a user's empirical FEVC estimate. They are
pointwise, approximate, post-result diagnostics, not simultaneous or
selection-adjusted tests. All 39 original primary rows are included in the
[machine-readable review](rc_observation_ratio_review_v1_result.json).

For the failed row, the excess over 1.10 is only 0.001204, or 0.072 Monte
Carlo SEs. Its distance from 1.00 is much larger, and the pointwise MC interval
excludes 1.00. Thus sampling noise can readily explain which side of the
chosen tolerance it lands on, but is not a persuasive explanation of the
whole calibration gap. Mean SE is 0.25349 versus empirical SD 0.27914:
about 9.2% lower when expressed relative to empirical SD.

Its 93.52% coverage passes the registered coverage gate but is not equivalent
to perfect 95% calibration. Its MCSE is 0.492 percentage points, with a
pointwise 95% MC interval of 92.56--94.48%. The q1 confidence interval is not
simply the point estimate plus or minus 1.96 SE, so coverage and SE-ratio
checks need not have identical conclusions. Bias is small (0.002968 versus
empirical SD 0.27914). All primary rows have full target-local availability;
the original harness does not measure the public command's additional full
joint-PSD availability boundary.

## Is this specific to the leverage model?

The leverage-minus-common k=16 ratio difference is 0.01347, with approximate
MCSE 0.02331 and pointwise 95% interval [-0.03222, 0.05916]. Their coverage
difference is -1.16 percentage points, MCSE 0.666 and interval
[-2.466, 0.146] percentage points. These contrasts do not identify a clear
additional leverage-model defect at this precision.

Importantly, these are different variance DGPs with independent semantic
outcome streams, not two variance models fitted to identical outcomes.
The contrasts are descriptive and unpaired. They neither prove equivalence
nor isolate a causal effect of the fitted variance model. The common k=16
row itself has a ratio of 1.0877. Choosing only the passing model after seeing
these outputs would not establish a remedy for the calibration pattern.

## Does averaging standard errors explain it?

Partly, but not mostly. The exact decomposition is
`SD / mean(SE) = [SD / RMS(SE)] * [RMS(SE) / mean(SE)]`.
Because square roots are nonlinear, even a correctly calibrated average
estimated variance need not give `SD / mean(SE) = 1`.

For the failed row, the two factors are 1.08690 and 1.01317. Switching to
root-mean-square SE removes only about a 1.3% multiplicative contribution.
Mean estimated variance is 0.8465 times empirical variance, with approximate
pointwise 95% MC interval [0.7981, 0.8979]. The corresponding common k=16
variance ratio is 0.8671, interval [0.8184, 0.9188]. The data therefore
suggest a genuine finite-design variance-calibration shortfall in both
one-mode firm cases, not just an artifact of averaging SEs. These additional
measures do not replace the registered arithmetic-mean-SE gate. Existing
outputs do not separate the remaining variance-fitting, finite-design
approximation, and other possible contributions.

## Method and independent numerical checks

For paired point errors `x_i` and reported SEs `s_i`, let `v` be the mean
squared centered error and `sbar` the mean SE. The influence values for
`log(R)` are

```text
psi_i = ((x_i - xbar)^2 - v) / (2 v) - (s_i - sbar) / sbar
MCSE(log R) = sample_SD(psi) / sqrt(n)
MCSE(R) = R * MCSE(log R)
```

This retains the within-replication covariance between numerator and
denominator. It does not assume Gaussian point errors or treat the estimated
mean SE as fixed. The 95% ratio intervals use the log scale. An exact
delete-one jackknife independently checks the delta calculation: the failed
row gives MCSE 0.016748 (delta) versus 0.016778 (jackknife), and intervals
[1.068863, 1.134524] versus [1.068806, 1.134584]. The delete-one ratio range
is [1.096212, 1.101636]; it is an influence diagnostic, not grounds to drop
any replication. RMS and variance ratios use the analogous paired influence
formula. Coverage MCSE is the binomial simulation MCSE.

Reporting Monte Carlo uncertainty alongside simulation performance measures
follows the general guidance of
[Morris, White and Crowther (2019)](https://doi.org/10.1002/sim.8086).
The formula above is implemented directly and checked independently; no
external estimator or bootstrap resampling is used.

## Provenance, accounting, and validation

Scientific source remains `73fa75805c8cef6d4d1a6ad843da5894ccedc956`.
The review was developed from source checkpoint `a88cd7f`, after the owner
approved this existing-output-only diagnosis. The original confirmation
report, registration, receipts, archive and JSON result remain unchanged.
The reader verifies the raw SHA-256 against the frozen audit, every attempted
target key and replication index, and the complete 200,000-attempt status
counts: 192,234 successful, 7,175 q0 variance failures, 591 q1 failures.
It independently reproduces the original ratio and coverage in each of the
39 prospectively primary rows; no primary row is selected using this review's
results. Excluded targets and unsuccessful attempts remain accounted for in
the full immutable confirmation record.

Raw SHA-256:
`e2f863372aff29a4e9782272a17f883d40e40457154a28475ee978e62ce33102`.
The new JSON binds the original result and review-script hashes. The raw data
are preserved in the original verified ignored diagnostic archive identified
in [the confirmation report](RC_OBSERVATION_CONFIRMATION_2026-09-05.md).

Reproduction command (Python 3.13.0; no RNG or production estimator calls):

```bash
./.venv/bin/python fevc/tools/review_rc_observation_ratio.py /private/tmp/fevc-rcobs-73fa758/scc-confirmation/revalidated/aggregate.jsonl
```

The focused tests check independent brute-force delete-one results, paired
delta covariance algebra, units/location/order invariance, ratio
decomposition, and rejection of invalid, corrupt, duplicate, missing or
inconsistent inputs. Floating comparisons use relative tolerances 1e-12 to
1e-14; reproduction of frozen ratios uses 1e-12 and coverage matches exactly.
All 16 focused tests pass. Under Python 3.13.0 and pytest 7.4.4, the full
source gates pass:

```bash
./.venv/bin/python -m pytest -q --basetemp /private/tmp/fevc-ratio-review-pytest.Oi7w74/run
./.venv/bin/python fevc/cmg/tools/assemble.py --all --check
git diff --check
```

The full Python suite reports 700 passed in 73.68 seconds; the assembler
and whitespace checks exit zero. An independent second invocation of the
review reproduces every saved JSON value and input/script hash exactly.
A fresh temporary pytest root avoids
unrelated pre-existing temporary-directory cleanup warnings.
Rust, Stata, platform and installation gates are not rerun for this
Python-only diagnostic and documentation change. No native, production,
public-interface or campaign-generation source changes.

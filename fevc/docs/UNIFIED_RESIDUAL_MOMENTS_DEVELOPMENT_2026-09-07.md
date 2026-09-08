# Unified fitter: bounded saved-draw assessment

## Decision

**BOUNDED_READINESS_PASS.** The common residual-moment fitter is ready for
the paper's runnable-example and local-scaling work under the saved plan.
This is development evidence on reused outcomes, not independent confirmation,
a release qualification, or a guarantee of nominal coverage.

All 22,560 native calls and 90,240 target attempts are accounted for: 28 match
cells with 400 saved replications and two fitter arms (22,400 calls), plus
four saved replications in each of 20 observation cells and two arms (160).
No new outcomes were drawn. Numerical settings were identical between arms:
200 JLA, 512 residual-moment Gram, 1,000 covariance and 128 spectral probes;
observation and match-q0 used 512 spectral iterations, match-q1 used 128.
The baseline match fitter does not use Gram probes. Native V2/V3 calls isolate
the fitter change without exposing a legacy choice in the public Stata command.

There are no accounting failures, unexplained point regressions or prospective
readiness failures. The unified fitter has no shared fit failures in any cell.
The legacy baseline retains nine shared positivity failures in the match-q0
severe-misspecification cell and five in the corresponding match-q1 cell.
Those attempts remain in the availability and all-attempt coverage denominators.

## Eligible match results

The ranges below cover the prospectively eligible, correctly specified target
rows, not every diagnostic target. Each row has 400 attempts. SD/SE means the
empirical standard deviation divided by the mean reported standard error;
it is not a q1 interval-width statistic.

| Fitter and reference | Eligible rows | Minimum availability | Coverage range | SD/SE range |
|---|---:|---:|---:|---:|
| Legacy match q0 | 24 | 100% | 93.00–96.50% | 0.943–1.076 |
| Unified match q0 | 24 | 100% | 93.00–96.75% | 0.934–1.067 |
| Legacy match q1 | 22 | 100% | 93.75–98.25% | 0.945–1.043 |
| Unified match q1 | 22 | 100% | 92.50–97.25% | 0.948–1.057 |

The unified match arms also pass the original two-sided development checks,
not just the saved plan's one-sided safeguards. The legacy match-q1 arm fails
the original upper-coverage check for the total component in
`one_mode_equal_independent`. Historical results are not rewritten.
The four-draw observation slice is a reproducibility/regression check only;
its mechanically reported coverage checks have no useful calibration meaning.

## Important failures and limitations

- Null and weak-signal diagnostic cells still produce unavailable individual
  intervals. In match q0, worker availability is 382/400 under the null and
  394/400 under weak signal; firm availability is 383/400 and 395/400.
  Match-q1 covariance availability is 196/400 and 273/400, respectively.
  All target-level failures, including those outside this short list, remain
  enumerated in the full result's `summaries[].failure_counts`.
- A misspecified aggregate-variance model can substantially understate
  uncertainty. Unified total-component coverage under severe misspecification
  is 81.75% for match q0 and 88.25% for match q1.
- Estimated fixed offsets remain an approximation. In the controls diagnostic,
  unified total-component coverage is 66.25% for q0 and 79.00% for q1, with
  100% computational availability. Successful fitting does not account for
  control-estimation uncertainty or establish independence across matches.
- The largest positivity-floor shares across all unified match cells are
  19.5% for q0 and 9.25% for q1. Minimum Gram reciprocal-condition diagnostics
  are approximately 3.72e-5 and 9.09e-5. These are numerical diagnostics, not
  tests that the variance model or spectral approximation is appropriate.
- q0 still needs a diffuse spectrum; q1 needs a suitable one-leading-mode
  approximation. Individual interval availability does not authorize exporting
  an invalid joint covariance matrix, and q1 does not post Gaussian `e(V)`.

These limitations were registered diagnostic cases, not exclusions chosen
after seeing this comparison. No cutoff, model, target set or numerical
budget was retuned during the run.

## Evidence and independent audit

Local evidence root:
`.local/diagnostics/unified-residual-moments-20260907/`.

- `paired-build-1/receipt.json`: immutable executable/source identities.
- `tiny-1/result.json`: complete pipeline, deliberate errors, split/reversed
  replay and captured saved fixtures; 192 calls pass.
- `comparison-1/manifest.json`, `source.tar.gz`, `tasks/`, `result.json`:
  frozen registration, source and executable bundle, all raw outputs and
  complete summaries. Match outputs contain all 400 saved replications;
  observation outputs contain replications 0, 133, 266 and 399.
- `comparison-1/independent-audit.json`: independent standard-library-only
  reconstruction of all 384 target summaries, 96 diagnostic rows and 11,280
  baseline comparisons with the original saved draws; source/input/executable
  hashes, inventory, semantic seeds, interval arithmetic and readiness agree.
- `local-engineering-1/`: source-local Mac binaries, native/Stata/install
  qualification and full engineering logs. This is a dirty-tree checkpoint,
  not a public binary release.

The independent auditor imports neither the campaign nor the estimator.
Its 12 arithmetic/failure-accounting tests and five completed-output corruption
tests pass (`17 passed in 10.33s`). The corruption checks operate on temporary
result copies and reject missing/duplicate summaries, altered coverage,
incorrect counts and a forged decision without modifying accepted evidence.
All frozen current source files and executable hashes still matched after
the run. Receipt hashes are recorded in the companion machine-readable result.

Next: update the paper's common-fitter explanation and reporting rules,
refresh saved-example/native-to-Stata checks and runnable examples, perform
the registered 27 local scaling calls, and build/inspect the PDFs. The paper
has not yet been updated at this checkpoint. No new scientific campaign,
joint-control implementation, commit, tag or release is authorized here.

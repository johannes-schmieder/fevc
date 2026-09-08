# Internal native observation inference: integration checkpoint

The residual-moment candidate now runs inside actual generic-JLA observation
component inference. This is a hidden Rust development path, not a new Stata
option. The existing public variance learners, point defaults and fixed-offset
match release candidate are unchanged.

## What was integrated

`residual_moment_inference::prepare_with_interrupt` selects the existing
three-term leverage or 15-term common rank-polynomial basis. The private
adapter uses actual JLA leverage and target diagonals and the live full-model
solver to construct the 512-probe residual-moment Gram matrix. Every projection
has a complete original-system residual certificate. The fitted positive
variances enter covariance estimation only; the raw leave-out point correction
and q1 leading-square recentering retain their definitions.

The constructor requires a finite, globally unique, outcome-free probe-order
key for each physical observation. Control canonicalization, row traversal and
numerical entity ranks use that order. The caller must establish the key before
outcomes: the implementation verifies finite uniqueness, not the provenance of
the key. Existing constructors retain their previous ordering. No legacy RNG
contract is silently redefined.

The complete memory forecast admits the fitter, basis/rank/permutation work,
projection callback, retained predictions and probe receipts before estimator
RNG. Its separate Gram-domain atoms and words enter the component Counter
receipt; it does not allocate or count the old learner's CV fold streams. This
path rejects non-unit frequencies, match deletion, fixed-offset nuisance,
stayers, simultaneous projection, automatic routing and invalid keys. It
introduces no ridge, basis dropping or failure fallback.

## Full native development experiment

The prospective
[`observation_residual_moments_integration_v1.json`](observation_residual_moments_integration_v1.json)
specifies four designs, 100 outcomes per design and two fixed numerical seeds.
Each call uses 3,200 JLA probes, 512 Gram probes, 1,000 covariance probes,
128 spectrum probes, at most 512 spectrum iterations and, for q1, 100,000
critical-value simulations. The reference is q0 for the diffuse design and q1
for the dominant designs. This is development, not confirmation. The two
numerical seeds share the same outcomes and are not 200 independent draws.

All 800 native calls and 3,200 native target attempts were audited, together
with 3,200 paired exact-oracle attempts. There were 799 complete native
successes and 3,196 available native intervals. The single failed call is
included in each affected target's denominator. Its paired exact output is
marked unavailable too because the failed native call did not export fitted
variances; it is not mislabeled an independently observed oracle failure.

Firm coverage, with every attempted draw in the denominator:

| Design | Rows | Native, seed 570239 | Native, seed 791503 | Paired exact calculation |
| --- | ---: | ---: | ---: | --- |
| Diffuse common, q0 | 922 | 97% | 97% | Same in both seeds |
| Dominant leverage, q1 | 536 | 94% | 97% | Same in both seeds |
| Dominant common, q1 | 536 | 95% | 96% | Same in both seeds |
| Dominant common with two controls, q1 | 536 | 96% | 96% | Same in both seeds |

The exact comparison uses the **same fitted variances**, not the true error
variances. Across all targets, native and exact intervals disagreed on
coverage in only one of 3,196 comparable pairs. For the firm target, the RMS
JLA point approximation error was 0.079--0.131% of the empirical sampling
standard deviation. The independent realized-kernel point identity had maximum
absolute error `1.49e-11`. The largest primitive covariance-diagonal discrepancy
from the exact trace was 3.64 reported numerical MCSEs. The largest simulated
critical-value discrepancy from quadrature at the same curvature was 0.0140.
These are checks for the two fixed numerical streams, not an assessment of
numerical-seed tail probabilities.

All successful outcomes within each design/seed had identical leverage,
target diagonals and Gram matrix. Average call times under four concurrent
local tasks were 0.58--0.76 seconds at these small dimensions; the complete
campaign took 146 seconds. This is not a large-data or standalone latency
benchmark, and does not replace the previous variance-preparation scaling
exercise.

## The remaining failure

`dominant_common`, numerical seed `791503`, replication `75` returns
`JLA_CONSTRAINT_FAILED` at `component_inference_psd`: its estimated joint
covariance is indefinite. The unmodified failure discards the complete
attachment, including otherwise positive marginal worker and firm variances.

A separate post-run dense audit reconstructs the fitted variances from the
fixed native Gram matrix. On a successful check outcome the independent fitted
predictions match native results within `6.49e-12`. On the failed outcome:

| Calculation | Smallest joint covariance eigenvalue |
| --- | ---: |
| Same fitted variances, native kernel, exact trace | -0.000470 |
| Same fitted variances, native kernel, original 1,000 probes | -0.000194 |
| Same fitted variances, exact kernel and trace | -0.000482 |
| True error variances, native kernel, exact trace | +0.000844 |

No predictions were floored. Thus increasing covariance probes or making the
JLA point calculation exact does not cure this draw. Positive estimated
observation variances do not guarantee that the bias-corrected **component
covariance estimate** is positive semidefinite in every sample. This is a
finite-sample variance-learning/availability limitation, not evidence of an
incorrect projection or a numerical-probe shortfall. The strict joint validity
rule remains unchanged; this audit does not replace the failed attempt or
authorize target-specific salvage.

## Validation and evidence

- 506 Rust workspace all-target tests; 13 standalone-backend tests.
- 10 generated independent-oracle/native tests, including dense Gram agreement,
  outcome/key and fitter-batch invariance, point invariance to the learner's
  streams, exact memory admission, cancellation and forced CMG/diagonal checks.
- 24 harness tests, including missing/duplicate/wrong-key/nonfinite outputs,
  deliberately failing scientific results and outcome-dependent geometry.
  The tiny generator-to-validator-to-receipt path passed; a deliberately
  invalid native CLI call exited 101 and produced no success receipt.
- 726 package Python tests, CMG assembly, pinned format and strict Clippy pass.
  Python emitted temporary-directory cleanup warnings, not test failures.
- Integrated local Stata quick/full/install checks pass. Fresh thin/universal
  macOS builds pass arm64 and Rosetta tests, including the existing public
  observation and match inference routes. The qualifier is explicitly
  `LOCAL_CHECKPOINT_DIRTY_TREE`, not a clean-SHA release qualification. Its
  underlying qualifier was run directly with new receipt/artifact paths to
  preserve existing CI evidence. The initial sandbox DNS failure and the
  subsequent network-approved successful attempt are both retained.

Source/task manifest SHA-256:
`f60eab471443ca2e30b41db98ed95c1d8c518e38f250078b32f92f26c9bc88fd`.
The machine-readable
[`observation_residual_moments_integration_v1_result.json`](observation_residual_moments_integration_v1_result.json)
records source-bundle, executable, native artifact and receipt identities.
Raw output, task receipts, failure audit and validation logs are in
`.local/diagnostics/residual-moment-integration-20260906/`.
The local archive
`.local/diagnostics/observation-residual-moments-integration-20260906.tar.gz`
has SHA-256
`0596fa6214fe0e116ba76e4bb687f0e3530267982381bfaa6fcdd421d6c5ac26`;
its eight raw development task files reconcile to all 800 calls.
The reproducible harness is `rust/experiments/residual_moment_integration/`.
No commit, push, tag, release, Linux or Windows run was performed.

## Interpretation and next boundary

The integration supports the earlier explanation: the old influential-variance
shrinkage problem is not reintroduced by actual JLA point/covariance work. The
small native slice closely follows the corresponding exact calculation.
It does **not** establish general coverage or resolve the previous exact-input
joint-control firm overcoverage FAIL of 96.60% against a 96.50% ceiling.
Null/weak-signal availability and deliberately multi-mode exclusions remain.

The next bounded step is a newly registered end-to-end confirmation using this
fixed internal estimator and explicit outcome-free key semantics. Freeze the
full task/DGP inventory, all-attempt success and coverage gates, and numerical
budgets before drawing new outcomes. Retain the joint PSD rule and report its
availability cost; do not enlarge this into a new covariance estimator or
joint-control redesign. Public option/ABI design and clean-source native
qualification follow only after that evidence is assessed. The fixed-offset
match release candidate remains separate.

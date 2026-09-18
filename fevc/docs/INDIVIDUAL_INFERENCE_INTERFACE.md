# Individual structured inference: development interface

This is the owner-approved candidate in
[`inference_completion_v1.json`](inference_completion_v1.json),
not a qualified release. Existing source-bound confirmations and failures are
unchanged. No platform or coverage claim transfers automatically.

## Scientific and reporting boundary

The four established point estimands, supported request tuples and the
200-probe JLA default remain. Both deletion units use the same residual
moments with a 2,048-probe direct residual Gram, 1,000 covariance probes and 128
spectral probes. Observation uses 512 spectral iterations; match uses 512
for q0 and 128 for q1. The bounded existing-draw q0 follow-up resolves the selected near-tied
spectral failures without changing the 0.002 certificate or 200 JLA probes;
it adds solve work, not a new coverage claim. The exact Mata route
and point-only defaults are separate and unchanged.

Observation fitting uses full-model residuals and projection; match fitting
uses weighted fixed-offset aggregates and the FE-only projection. The common
polynomial basis has 15 raw terms for observations and 21 for matches,
including match regression mass. The leverage-only basis has three terms.
The unified policy removes outcome-free redundant columns in original order,
certifying scaled reconstruction at 1e-12 while keeping original column scales
and the existing 1e-10 conditioning gates. Support requires five independent
units per active term. It does not select a variance model or add ridge fitting.

Each target must pass its own numerical checks. For q0 this includes a finite
positive scalar variance, linear influence and certified spectral calculation.
For q1 it includes the existing leading-mode, leading/remainder variance,
determinant, remainder-identity and interval checks. A computed interval does
not prove the variance model or q-specific asymptotic assumptions.

A materially invalid joint matrix is never repaired or exported as an
admissible covariance. Individual intervals may survive its rejection.
Structured q0 posts `e(V)` only when the joint matrix and all four q0 targets
pass; structured q1 never posts a Gaussian `e(V)`. An admissible
`e(V_primitive)` remains a diagnostic under q1. Shared variance-fit, rank,
solver, resource and corrupted-transport errors remain atomic failures.
There is no new reporting-mode option, fallback or positive target-variance
floor. Fixed-offset match inference still ignores estimated-control uncertainty.

## Additive transport

- `vckss_rust_component_inference_interface_version()` returns 4. Stata checks
  this before native preparation or estimator RNG.
- Observation/match `...augment...interrupt_v4` entrypoints reuse the frozen
  V1 interrupt request pointer plus a `uint32_t gram_probes` argument, selecting
  direct residual covariance and individual reporting. V1–V3 entrypoints retain
  their previous fitting/reporting semantics and 512-probe residual defaults.
- Result V5 is 288 bytes, retaining the 208-byte V4 prefix. Its added fields
  identify joint status, fitter, four q0 statuses, Gram probes/order, Gram and
  fit residuals, positivity floor and floor counts. A separate 4-by-2 buffer
  carries scalar target variances and status. All output capacities are checked
  before any matrix write.
- Invalid joint buffers contain IEEE NaN at the native boundary, translated
  to Stata missing values and never posted as joint matrices. Both units' CV
  buffers are explicitly inapplicable, not fictitious successful diagnostics.
- Stata's 38-column component receipt reconciles method, status and counts;
  the unit receipt remains V1. `q0_status`, `q1_status`,
  `inference_joint_status`, `inference_joint_posted` and
  `inference_computed_targets` have separate meanings.

Gram work is included in planned RHS, Counter, complete-residual and solver
counts. Its small-matrix/streamed workspace and both synchronous V5 caller
buffer copies are charged before RNG. No production observation-square matrix
is introduced.

## Gram precision

For `M=I-P`, use `s_r=Z'[(M g_r)^2]` and
`Khat = sum_r (s_r-sbar)(s_r-sbar)' / [2(R-1)]`. The same small-system
fitter solves `Khat gamma = Z'e^2`; basis construction, positivity and rank
checks remain. Approximate leverage still enters the variance basis and HC2
floor scale, but not an additional subtractive Gram term. The sample covariance
is not clipped or regularized; deficient information remains a typed failure.

`inferencegramprobes(#)` is accepted only for the supported explicit structured
Rust observation or fixed-offset match tuple. Omitted means 2,048; explicit
counts must be integers in [512,2147483647], subject to overflow and memory
admission. Values below 2,048 are a lower-precision speed tradeoff. Counts are
reconciled through the plan, solver records, Counter totals and V5 receipt.
`e(inference_gram_probes)` reports the count and `e(inference_gram_method)`
reports `direct_residual_covariance`. Other numerical domains retain their
addresses. There is no automatic budget escalation or legacy public option.

## Numerical ordering

The observation attachment orders by worker, firm, frequency, target mass,
raw controls and any supplied `probeorder()` key, never by outcome. Exact
design duplicates receive distinct exchangeable probe addresses. Without an
external stable key, exchanging different outcomes among those duplicates
need not preserve a finite-probe result bitwise. Outcome-free geometry and
distributional equivalence are distinct requirements. A stable outcome-free
key can make that permutation comparison pathwise reproducible; no new key
is required for ordinary use. The separate original explicit-key research API
keeps its historical contract.

Match fitting orders existing match addresses by (design entity, subdraw),
with one Gaussian numerical draw per declared match. It does not change the
point estimator's match ordering. V5 ordering codes are 2 for canonical
observation rows and 3 for canonical matches (1 retains the research-key case).

Changing from point-only ordering can change the finite-probe point
realization, not its formula. Use the registered numerical-equivalence policy
rather than claiming unchanged floating-point results. The duplicate smoke
and prospectively separated repeated-seed check are recorded in
[`individual_ordering_development_v1.json`](individual_ordering_development_v1.json).

## Remaining gates

The complete 48-cell, 19,200-call development run is FAIL; see
[`the checkpoint`](https://github.com/johannes-schmieder/fevc/blob/ffca8b5cfc0ff8c495923c00d93ca292528e57d9/fevc/docs/INDIVIDUAL_INFERENCE_DEVELOPMENT_2026-09-06.md).
The new owner-approved milestone uses bounded saved-draw comparisons, local
native/Stata validation and refreshed complete-command timings before updating
the paper as approximate model-based inference. It does not require the older
120,000-call confirmation milestone and does not relabel that FAIL. The earlier unified comparison and local software checks passed on their own
source. The current completion plan fixes a 70-call saved-outcome engineering
replay, 13-call Veneto example and 27-call local scaling inventory; these are
not independent coverage confirmation. Complete
Mac/Linux/Windows distribution and public release remain separate decisions.

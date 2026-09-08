# Observation residual moments: internal implementation checkpoint

## Outcome

The residual-moment variance candidate now has a separate Rust-core
implementation. Its projected-probe calculation reproduces the independent
dense calculation, and replaying the original difficult draws retains the
calibration improvement. This is an **internal engineering checkpoint**, not
independent confirmation or a new supported Stata option.

The original observation confirmation remains FAIL. Existing variance-model
options, point estimation, the raw q1 leading recenter, confidence-interval
formulas, the accepted match route, and the paper are unchanged.

The prospective implementation contract is
[`observation_residual_moments_internal_v1.json`](observation_residual_moments_internal_v1.json).
The numerical motivation and earlier exploratory comparisons remain in the
[residual-moment development report](OBSERVATION_VARIANCE_REMEDY_DEVELOPMENT_2026-09-06.md).
The new [implementation result](observation_residual_moments_internal_v1_result.json)
records source hashes, every target cell and failure, validation and archive
identity. Neither report replaces the original confirmation receipt.

## What is implemented

`rust/crates/vckss-core/src/residual_moments.rs` accepts an outcome-free
variance basis Z, exact full-model leverages h, canonical observation RNG
addresses, and a caller-owned full-model projection action. Intended callers
supply the existing three-term leverage or fifteen-term common basis. The
engine accepts a constant-only basis for the homoskedastic special case; it
does not select terms, drop collinear columns, or redefine the old fitter.

Preparation uses a fixed numerical stream before any outcome fitting:

```text
t_r = Z' [(P g_r) elementwise-squared]
Khat = Z' diag(1 - 2h) Z + sample_covariance(t_r) / 2.
```

Gaussian probes use a new Counter-V1 domain, independent of leverage,
target, covariance, fold and critical-value domains. Each probe needs one
full-model projection. Row reductions are compensated, and the small
covariance uses centered Welford accumulation in logical probe order.
Changing batch width leaves the same numerical draws and accumulation order.
No observation-by-observation matrix is stored by the candidate.

For each outcome, the fitter solves
`Khat gamma_hat = Z' e^2`, where e is the residual from the identical full
model. It returns raw and positive variance predictions, coefficients,
nonpositive/floored counts, the floor and a small-system residual check.
Positive predictions are for inference covariance only. There is no ridge,
coefficient clipping, cross-validation or post-failure fallback. The floor
remains `1e-8 * median(e_i^2 / (1-h_i))`; a nonpositive scale or an entirely
floored fit fails explicitly.

Input, structural basis-rank, address and memory checks precede RNG.
Preparation rejects singular, ill-conditioned or indefinite estimated moment
matrices and requires an acceptable complete original-system residual for
every projection solve. Missing or partial callback outputs are rejected.
The API polls cancellation through validation, allocation, probe work, fitting
and sorting. Its checked memory envelope covers candidate-owned work plus a
caller-declared projection workspace allowance. The caller must separately
admit retained input and solver memory; this is not a whole-command receipt.

## Independent checks and original-draw replay

Twelve focused Rust tests cover independent dense identities, noiseless
exact-K recovery, the exact-K residual-degrees-of-freedom special case,
Counter-V1 domain separation, probe prefixes and batch invariance, positivity
accounting, invalid/nonfinite input, near-singularity, exact memory admission,
overflow, cancellation and failed/missing solver certificates. A small
worker--firm fixture compares the actual full-firm zero-sum diagonal solver
against an independently grounded **test-only** dense oracle, both with and
without joint controls. The projection calculations and fitted predictions
agree within `1e-8`; shifting the fitted mean leaves the variance fit unchanged.
This does not qualify CMG attachment or estimated match offsets.

The new Rust fitter was also applied to the same 2,500 original draws in each
of three development cases. The probe count (512), numerical seed (20260906),
batch width (16), cases and parity tolerance (`1e-8`) were fixed in the local
replay plan before its output was inspected. The previous q0/q1 evaluator was
held unchanged. All 7,500 variance fits succeeded.

| Firm-variance case | Original fitted method | Dense residual moments | Rust projected residual moments | SD / mean SE, Rust |
| --- | ---: | ---: | ---: | ---: |
| Dominant leverage, q1 | 93.52% | 95.88% | 95.92% | 1.0214 |
| Dominant common, q1 | 94.68% | 95.72% | 95.76% | 1.0258 |
| Diffuse leverage, q0 | 95.16% | 95.20% | 95.24% | 0.9958 |

Every firm-target interval is available, so these coverages include all
attempts. The tiny differences from the dense candidate are not evidence of
superior statistical performance; the numerical Gram is approximated.

NumPy independently projected the identical Counter-V1 draws, formed the
sample covariance, solved the small system and applied the floor. Maximum
absolute differences were `7.96e-13` for the moment matrix and `1.47e-11`
for variance predictions. Successful point estimates were unchanged exactly
in the paired evaluator. Maximum complete projection residual was
`4.54e-15`; maximum small inverse residual was `3.13e-12`.

Relative to the exact moment matrix, whitened operator errors were 0.61%,
1.15% and 0.26%. The common moment matrix had scaled reciprocal condition
number `1.70e-5`, versus about 0.14 for the leverage-only cases. The common
fit floored 803 predictions across 1,340,000 row-by-replication predictions;
neither leverage case needed a floor.

All four targets were retained: **29,982 of 30,000 target attempts succeeded**.
The remaining eighteen were covariance-target nonpositive-variance failures
(seven dominant leverage and eleven dominant common). Those targets are
outside the intended one-mode coverage claim and remain explicit in the
result. The public command's additional full-joint-PSD boundary was not
exercised by this target-local replay. Four local harness tests also check
truncated/trailing inputs, invalid headers/dimensions, nonfinite fixtures and
explicit preservation of an outcome-fit failure.

## Limits and next step

The exact-K identity gives unbiased untruncated variance predictions only
when the variance model is correctly specified and identified. Inverting
a finite-probe Khat and flooring predictions need not preserve that property.
A positive, well-conditioned *estimated* Gram is not an independent
certificate that a difficult population moment system is well identified.

This implementation still assumes exact leverages and an outcome-free fixed
basis. The replay uses exact factorized projections; the small solver test
uses tightly certified iterative projections. Neither establishes performance
or accuracy with noisy JLA leverages/target ranks, high-dimensional AKM data,
or multiple numerical probe streams. The default 512-probe count is a
development setting, not a universal accuracy guarantee.

The next bounded slice should compare exact and actual JLA diagnostic inputs
on the same designs, across fixed numerical seeds and a small size sequence.
Keep the variance method and acceptance criteria fixed before observing
results. Record Gram conditioning, prediction/interval availability,
numerical error and total projection cost; do not repair a failed fit by
silently increasing probes or changing the model. A same-design observation
versus match deletion comparison is also still owed. Only after those checks
should the feasible candidate be frozen for separately registered independent
confirmation and then explicit public/native integration.

Severe variance-model misspecification, null-signal interval withholding,
multi-mode limitations and estimated fixed-offset uncertainty remain open.
No public promotion, release, paper revision or external computation is
implied by this checkpoint.

## Source checks and evidence boundary

The tested implementation is an uncommitted working-tree addition to
`1aeed0651fc3d85a4d9ea2a3c6cdf8e147ddc0b5`, bound by file hashes in the
new result rather than represented as a clean exact-SHA candidate. Existing
structured fitting, component/q1 kernels and original replay source are
unchanged. Before confirmation or native promotion, freeze a clean source.

Pinned Rust 1.85.1 formatting, strict workspace Clippy, workspace tests,
standalone backend tests, the twelve focused tests, all 726 repository Python
tests and the CMG assembly check pass. The integrated local checks also pass,
including Stata quick/full, clean installation and their local smokes.
The Stata runs protect the existing package; they do not execute or qualify
this unattached candidate. The result records exact commands and log hashes.

No source-bound plugin-build, new native binary qualification, Linux/Windows
run, scale benchmark or independent coverage confirmation was performed.
Nothing was committed, pushed, tagged or released. Full synthetic replay
inputs, numerical draws, fit/target outputs, programs and useful logs are
preserved in an ignored local diagnostic archive, not a release payload.

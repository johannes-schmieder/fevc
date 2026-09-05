# Paired controls experiment — 2026-09-05

## Finding

The paired experiment isolates omitted control-estimation uncertainty as the
main cause of the original controls fixture's undercoverage. For the total
component, nominal 95% q1 coverage is **95.1% with known controls and fitted
match variances**, but **75.9% with estimated controls and fitted variances**.
Giving the latter calculation the true original match variances only raises
coverage to **76.1%**. Better fitting those original scalar variances cannot
recover the covariance induced by estimating controls on the same data.

The prospectively registered known-offset calibration checks all pass. This
is a bounded diagnostic success, **not** successful calibration of estimated-
offset inference and not public match-inference qualification. Production
estimation, inference, defaults, and routing are unchanged. No second-stage
nuisance correction was added.

## Registered design and accounting

Registration: [`fixed_offset_paired_v1.json`](fixed_offset_paired_v1.json).
Exact clean execution source: `8e1ed33e47e95c92e4f579510464e4dc85fb5b7f`.
Machine-readable manifest, all 16 cell summaries, all 20 paired contrasts,
exact moment references, hashes, and validation receipts:
[`fixed_offset_paired_v1_result.json`](fixed_offset_paired_v1_result.json).

The original design has 400 matches, 1,199 stored physical rows, frequency
mass 8,387, two within-match-varying controls, and serial Gaussian errors
with rho 0.65 inside each match. Across-match original errors are independent.
The correct original aggregate variance is linear in normalized match-mass
midrank. Component target weights remain distinct from regression frequency
weights, including the original 1,000-fold target-mass multiplier at match
(0,0). No larger-sample design or post-result exclusion was introduced.

For each of 1,000 replications, all four arms use the same physical-row errors
and production seed. Known offsets subtract the true control contribution;
estimated offsets use the unchanged joint FE/control fit. Known variance
supplies the true original aggregate vector `tau`; fitted variance uses the
unchanged `structured_common` model. **Known variance does not mean supplying
the transformed full covariance after estimating controls.** That deliberate
omission identifies the limitation of the fixed-offset approximation.

All 40 local tasks completed in 320.01 seconds with four workers. There are
16,000 attempted and 16,000 computed arm-target intervals: no shared failures,
no target-local withholdings, no missing rows, and no complete-case selection.
Thus available-interval and all-attempt coverage coincide here. Fitted arms
used the same five outcome-free folds across all replications. Same-offset
known/fitted-variance point estimates are bitwise identical, including their
agreement with the unattached production point calculation.

## Coverage results

Each cell below contains 1,000 intervals; all are nominal 95% q1 intervals.

| Control offset | Original match variance | Worker | Firm | Covariance* | Total |
| --- | --- | ---: | ---: | ---: | ---: |
| Known | Known | 96.0% | 96.1% | 94.8% | 95.4% |
| Known | Fitted | 94.7% | 94.8% | 93.9% | 95.1% |
| Estimated | Known | 85.8% | 89.4% | 90.0% | 76.1% |
| Estimated | Fitted | 84.7% | 88.8% | 88.8% | 75.9% |

*The covariance target is prospectively a multi-mode diagnostic, excluded
from q1 calibration claims even when its realized coverage looks close to
nominal. The primary known-offset worker, firm, and total cells all pass the
unchanged availability, coverage, bias, and SE-ratio checks.

For the total component, estimating the offset reduces coverage by **19.3
percentage points** at known original variance (paired Monte Carlo SE 1.421
points), and **19.2 points** at fitted variance (MCSE 1.412 points). Fitting
variance instead of knowing it changes coverage by only -0.3 points with
known offsets (MCSE 0.361), or -0.2 points with estimated offsets (MCSE 0.469).
The paired interaction is +0.1 points (MCSE 0.575). These are uncertainty
measures for simulation comparisons, not SEs for the component estimand.

For worker and firm, estimating the offset at fitted variance reduces
coverage by 10.0 and 6.0 points (paired MCSE 1.123 and 0.941 points). Variance
fitting has smaller, nonzero effects: with known offsets, worker and firm
coverage each fall by 1.3 points. The conclusion is not that variance fitting
is costless, but that it is not the main explanation of this large controls
shortfall.

## What happens to uncertainty

| Total-component quantity | Known offset, known variance | Known offset, fitted variance | Estimated offset, known variance | Estimated offset, fitted variance |
| --- | ---: | ---: | ---: | ---: |
| Empirical SD | 0.004618 | 0.004618 | 0.007891 | 0.007891 |
| Mean reported SE | 0.004635 | 0.004597 | 0.004636 | 0.004592 |
| Empirical SD / mean SE | 0.996 | 1.004 | 1.702 | 1.719 |
| Mean interval width | 0.019001 | 0.018846 | 0.019007 | 0.018821 |

Estimating controls increases actual sampling variation substantially while
the reported SE and interval width scarcely change. The estimated/fitted
total bias is -0.000060, with MCSE 0.000250; bias is not the main visible
problem in this simulation. The earlier exact calculation gives a small
nonzero bias, 0.000073, and an estimated-offset population SD of 0.007804.
The finite simulation does not establish exact unbiasedness.

The independently calculated exact-kernel SDs in these same draws are
0.00461847 with known offsets and 0.00789170 with estimated offsets, nearly
the production values 0.00461771 and 0.00789098. Randomized correction
differences have total-component RMS about 0.000047; they are too small to
explain the uncertainty gap. The tiny direct production check uses its
exported randomized correction diagonal, so it tests fitting and collapse
without incorrectly demanding equality to a nonrandom exact diagonal.

The q1 remainder makes the mechanism especially clear. Its exact population
variance is 0.00005318 after estimating controls. The mean reported remainder
variance is only 0.00001282 with known original variances and 0.00001246 with
fitted variances. The former closely matches the exact expected diagonal-
variance calculation, 0.00001282: the code is estimating the omitted-
nuisance approximation, not the actual unconditional uncertainty.

The leading variance changes less: actual 0.29499 versus mean reported
0.27022/0.27158. With leading modes oriented consistently, actual leading-
remainder covariance is -0.00017130, versus reported means +0.00001183 and
+0.00000954. Thus a universal scalar SE inflation would not recover the
full q1 covariance structure. None was applied.

Exact worker/firm/total remainder concentrations remain
0.0614/0.0621/0.0428. The noisy 128-probe total remainder ratio has median
0.0480 but mean 0.4034, with 375/1,000 draws above 0.99. Its adverse mean
must not be mistaken for adverse exact geometry. All four arms share these
randomized spectral diagnostics; they cannot explain the paired offset gap.
The previously identified deflated-trace diagnostic improvement remains a
separate engineering follow-up, not an adjustment to these frozen results.

## Validation and reproducibility

Before diagnostic draws, the real two-replication generator-to-validator-to-
receipt pipeline passed twice: one worker in forward order and two workers
in reverse order. Aggregate rows were byte-identical. Exporting production
state did not change any target result. Independent physical-row joint
weighted regressions agreed with all 32 tiny point calculations to at most
3.86e-13; dense physical-row covariance/moment calculations agreed with the
low-rank oracle to 3.56e-18. The true and estimated aggregate covariance
matrices both passed the outcome-free positivity check.

The completed diagnostic was reaggregated independently from all saved
inputs, raw outputs, and receipts. Every generated input was reproduced from
its semantic key; all file hashes, source identities, binary identities,
folds, task keys, and arm-target inventories passed. Both aggregate rows and
aggregate receipt matched byte for byte. The complete synthetic raw evidence
and gate logs are preserved in the ignored local archive recorded and hashed
in the JSON result. No restricted or licensed comparator data were added.

Source gates: 632 Python tests, including 34 focused paired-harness tests;
two Rust example tests; workspace Rust formatting; focused Clippy with
warnings denied; CMG assembly check; and the complete integrated local
qualification, including Stata quick/full, existing plugin-interface tests,
clean installation, benchmark, and sample-preparation smokes. Python 3.13.0,
NumPy 2.5.2, Rust 1.85.1, Stata app 19.0.115, macOS arm64. Matrix-library
thread counts were fixed at one, independently of four outer task workers.

An initial development test caught a wrong nested moment-field lookup in
the new checker; it was fixed before any real tiny or diagnostic outcomes.
The checker also corrected its assumed fold count to the unchanged production
five. The failed initial logs are preserved. Deliberate malformed input
exited 2 with a typed parser message and no target rows; adversarial unit
tests cover missing, duplicate, partial, nonfinite, modified, wrong-source,
wrong-seed, scientifically failing, and process-failing results.

No production Rust/Mata/ado, ABI, dependency, native build configuration, or
frozen earlier campaign changed. The existing licensed-Stata plugin tests
passed; a fresh cross-platform native-binary qualification was not run and
is not claimed. There was no SCC campaign, push, tag, release, or distribution.

Execution commands (from the repository root; prefix Python commands with
`OPENBLAS_NUM_THREADS=1 VECLIB_MAXIMUM_THREADS=1 OMP_NUM_THREADS=1`):

```bash
./.venv/bin/python -m pytest -q fevc/tests/python/test_fixed_offset_paired.py
./.venv/bin/python fevc/tools/run_checks.py
PATH=/Users/johannes/.rustup/toolchains/1.85.1-aarch64-apple-darwin/bin:$PATH cargo test --locked --manifest-path rust/Cargo.toml -p vckss-core --example fixed_offset_paired
PATH=/Users/johannes/.rustup/toolchains/1.85.1-aarch64-apple-darwin/bin:$PATH cargo build --release --locked --manifest-path rust/Cargo.toml -p vckss-core --example fixed_offset_paired
PATH=/Users/johannes/.rustup/toolchains/1.85.1-aarch64-apple-darwin/bin:$PATH cargo clippy --locked --manifest-path rust/Cargo.toml -p vckss-core --example fixed_offset_paired -- -D warnings
PATH=/Users/johannes/.rustup/toolchains/1.85.1-aarch64-apple-darwin/bin:$PATH cargo fmt --manifest-path rust/Cargo.toml --all -- --check
./.venv/bin/python fevc/tools/run_fixed_offset_paired.py create-manifest /private/tmp/fevc-offset-paired-8e1ed33-tiny-manifest.json --binary rust/target/release/examples/fixed_offset_paired --profile tiny
./.venv/bin/python fevc/tools/run_fixed_offset_paired.py run-local /private/tmp/fevc-offset-paired-8e1ed33-tiny-manifest.json /private/tmp/fevc-offset-paired-8e1ed33-tiny --binary rust/target/release/examples/fixed_offset_paired --workers 1
./.venv/bin/python fevc/tools/run_fixed_offset_paired.py run-local /private/tmp/fevc-offset-paired-8e1ed33-tiny-manifest.json /private/tmp/fevc-offset-paired-8e1ed33-tiny-reversed --binary rust/target/release/examples/fixed_offset_paired --workers 2 --reverse
./.venv/bin/python fevc/tools/run_fixed_offset_paired.py verify-tiny /private/tmp/fevc-offset-paired-8e1ed33-tiny-manifest.json /private/tmp/fevc-offset-paired-8e1ed33-tiny/tasks /private/tmp/fevc-offset-paired-8e1ed33-tiny-independent --binary rust/target/release/examples/fixed_offset_paired
./.venv/bin/python fevc/tools/run_fixed_offset_paired.py create-manifest /private/tmp/fevc-offset-paired-8e1ed33-diagnostic-manifest.json --binary rust/target/release/examples/fixed_offset_paired --profile diagnostic
./.venv/bin/python fevc/tools/run_fixed_offset_paired.py run-local /private/tmp/fevc-offset-paired-8e1ed33-diagnostic-manifest.json /private/tmp/fevc-offset-paired-8e1ed33-diagnostic --binary rust/target/release/examples/fixed_offset_paired --workers 4
./.venv/bin/python fevc/tools/run_fixed_offset_paired.py aggregate /private/tmp/fevc-offset-paired-8e1ed33-diagnostic-manifest.json /private/tmp/fevc-offset-paired-8e1ed33-diagnostic/tasks /private/tmp/fevc-offset-paired-8e1ed33-revalidated
```

## Next steps toward integration

1. Register full match q0 independent confirmation on the repaired source,
   preserving the historical 14-design coverage/misspecification matrix and
   recording fresh confirmation seeds and complete prerequisites. The old
   frozen development documents' estimated-offset conditioning language must
   be superseded in the new registration, not silently rewritten in place.
2. Register fresh observation q1 confirmation using the previously registered
   eligible designs and acceptance gates, with corrected curvature and
   target-local availability. Old q1 receipts do not qualify the repair.
3. After those evidence gates pass, incorporate explicit match q0/q1 options,
   target-specific results/diagnostics, documentation, and affected native/
   Stata interface tests. Keep model choice and q explicit. Preserve mover-
   population, frequency-weight, and target-weight semantics; do not silently
   drop stayers or switch reference distributions.
4. Retain the requested no-second-stage-correction boundary. Describe estimated-
   controls inference as a fixed-offset approximation omitting nuisance-
   estimation uncertainty, not proven conditional inference given an estimated
   offset or generally calibrated 95% inference. A practical uncertainty
   diagnostic may remain useful, but this fixture's roughly 76% coverage must
   remain visible. Revisiting nuisance correction would be a separate decision.

This experiment resolves the paired diagnosis. It does not finish public
q0/q1 match integration or broaden release authority.

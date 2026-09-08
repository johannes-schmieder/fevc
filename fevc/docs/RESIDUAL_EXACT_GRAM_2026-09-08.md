# Exact-Gram diagnosis on three flagged designs

## Finding and decision

Replacing only the approximate Gram matrix with its exact counterpart resolves
two of the three registered failures from the 2,048-probe comparison. The
observation-with-controls firm SE discrepancy remains. This separates a
material numerical approximation problem from a remaining downstream/model
calibration limitation; it does not identify the latter's precise cause.

The owner-approved [registration](residual_exact_gram_v1.json) is diagnosis
only. The [machine-readable result](residual_exact_gram_v1_result.json)
records **DIAGNOSTIC_COMPLETE_NOT_PROMOTED**: accounting passes, one calibration
screen fails. Production, installed plugins and the paper are unchanged. These
three designs were selected because of earlier failures, so even a complete
pass would not qualify the estimator.

## What changes, and what does not

Each cell reuses replications 0–199 of the same saved outcomes, with numerical
seed 8675309 and 200 point JLA probes. The exact substitution retains the
realized variance basis, leverage inputs, outcome residuals, small-system
inverse and rank gates, positivity floor, covariance and spectral probes,
critical-value simulation, q1 recentering and interval construction. All
point estimates and point MCSEs are bit-identical to the 2,048-probe arm.

The isolated native copy captures the independent-unit design through
coefficient predictions, in the fitter's actual row order. This includes the
two joint controls for the controls design and square-root match regression
masses for match deletion. Python independently constructs the orthogonal
projector P by SVD and evaluates K = Z'((I-P) elementwise squared)Z. A second
audit drops a redundant worker dummy, solves full-rank normal equations and
reconstructs K without the SVD route. Their relative Gram differences are
below 1.6e-16. Independent trace and elementwise contractions agree as well.

The exact matrix file includes dimensions and every basis value. The isolated
native loader checks these on every outcome before substitution, then applies
the unchanged inverse and fitted-moment residual gates. The original 2,048
Gaussian probes are still computed and counted, but their Gram is unused after
substitution. Thus Counter receipts and all other numerical draws are unchanged;
this experiment makes no runtime claim about a usable exact implementation.
Dense matrices exist only in the bounded research oracle, not production.

## Matrix checks

| Design | Independent units | Design rank | Active variance terms | Native/SVD projection relative difference |
| --- | ---: | ---: | ---: | ---: |
| Observation, dominant common | 536 | 31 | 15 | 1.50e-12 |
| Observation, diffuse common with controls | 922 | 33 | 15 | 1.37e-12 |
| Match q1, equal independent | 400 | 39 | 15 | 1.45e-15 |

Symmetry, idempotence, orthogonality and design reproduction pass at 1e-9.
All exact Grams are identified under the unchanged rank threshold. Relative
to exact K, the 2,048-probe matrices have generalized eigenvalue ranges
0.858–1.165, 0.869–1.139 and 0.865–1.176, respectively. These quantify
direction-dependent approximation error, not statistical confidence bounds.

## Saved-outcome results

All 600 calls and 2,400 selected-reference target intervals are accounted for
and computable; there are no shared failures or point regressions. Coverage
uses every attempted outcome. All originally eligible targets are screened,
not only the three previously flagged firm targets.

| Firm target | Direct 512 coverage | Direct 2,048 coverage | Exact coverage | Direct 2,048 SD / RMS SE | Exact SD / RMS SE |
| --- | ---: | ---: | ---: | ---: | ---: |
| Match q1, equal independent | 89.5% | 93.5% | 97.5% | 1.026 | 0.931 |
| Observation q0, diffuse with controls | 88.0% | 90.5% | 91.5% | 1.163 | 1.151 |
| Observation q1, dominant common | 91.0% | 91.5% | 93.0% | 1.120 | 1.054 |

The match and dominant-observation cells now pass every eligible registered
V1 screen. Match firm coverage also passes the paired comparison with the
archived current fitter (97.0%). In the controls cell, firm coverage passes the
registered absolute and paired screens, but SD/RMS-SE remains 1.151 versus
the unchanged 1.10 ceiling. It is almost the same as the archived current
fitter's 1.150. Increasing Gram precision alone therefore does not resolve
this discrepancy on these saved outcomes.

Ratios above one mean that the empirical standard deviation exceeds the root
mean square reported SE. The q1 interval is not a Gaussian interval determined
by that auxiliary SE alone. Do not interpret the match 97.5% as a guaranteed
improvement toward nominal 95%, or the controls 91.5% as generally satisfactory
coverage merely because a small-sample diagnostic screen passes.

All twelve target summaries, four comparison arms and floor counts are in the
machine-readable result. The dominant-observation covariance target remains
nonprimary: all 200 q1 intervals return, but only 196 auxiliary high-rank SEs
are available. That denominator is explicit and does not remove four outcomes
from interval coverage. Maximum exact-arm floor shares are 1.50% for match,
0.108% for controls and 0.933% for dominant observation.

## Validation and attempt accounting

- Five independent Python oracle tests pass: analytical intercept Gram,
  weighted/redundant design, rank/nonfinite rejection, malformed headers and
  row permutation. Six inherited reporting tests pass, including RMS versus
  mean SE and the separate q1 interval/auxiliary-SE denominators.
- The focused Rust loader test passes, rejecting wrong dimensions or basis,
  nonfinite/asymmetric matrices and truncated/extra payloads.
- Three capture calls reproduce every archived 2,048 result field other than
  measured elapsed time. An initial extra capture was rejected only because
  the harness compared elapsed time too; that attempt and original script
  remain under `captures-attempt1` and `capture-attempt1.py`. No numerical
  result changed, and captures are not coverage replications.
- Three exact-Gram tiny calls and three deliberately malformed CLI calls
  complete before the main manifest. Tiny corruption checks reject missing,
  duplicate, wrong-seed and wrong-budget output. Main tasks write separate
  atomic compressed results, with exact source, input, matrix and binary hashes.
- The separate main auditor verifies all 600 raw calls, task keys, exact-Gram
  load markers and identities, points, Counter counts, target availability,
  coverage, SE denominators and all registered failures. It agrees with the
  main reporter. No failed cell or target is omitted.
- Minimum source gates pass: 780 Python tests and generated CMG assembly check.
  No source-local Stata, cross-platform or release qualification was run:
  public code and artifacts did not change.

Pinned Rust 1.85.1 builds use `--release --locked --offline`; Python is the
repository `.venv` (NumPy 2.5.2). The native replay uses four concurrent
single-threaded tasks. Commands and logs are preserved below, including
`build.log`, `loader-tests.log`, `oracle-tests.log`, `reporting-tests.log`,
`capture-final.log`, `tiny.log`, `main.log`, `independent-audit.log`,
`source-tests.log` and `assemble-check.log`. The initial Python test run passed
but emitted cleanup warnings for unrelated pre-existing temporary files;
subsequent tests used isolated `--basetemp` directories.

## Evidence identities

Local root: `.local/diagnostics/residual-exact-gram-20260908`. The frozen main
manifest enumerates all source, harness, exact-matrix, original input, archived
comparison and executable identities. SHA-256:

| Artifact | SHA-256 |
| --- | --- |
| `source_manifest.json` | `5a8a7920b8e6f2d9c68befbef86c10630409a7f68b0f195d4c106bc89e4f3e3c` |
| `candidate-bin/receipt.json` | `a5a99fea7c8e1ae0d2d5577b5b48d6f3a7be6ae41686d23fcdb6ed4967749fdd` |
| `oracle_audit.json` | `51ccf6415c930300a0aff64890d8443bca2a1d7024f2f4c1a422b282b5868b2a` |
| `tiny/result.json` | `0403f9e126b35459d9f939c379a3c932990b86a934a501bf3f9dabdb7acdb558` |
| `main/manifest.json` | `3793740226d117ab437835defdf833778a509e0d87032376e913a6bf723c5eb2` |
| `main/result.json` | `01e798d8ffb1f85e7ac99e369ef310149147fe09cade5e1b81b82d7d059751c2` |
| `independent_audit.json` | `2b39193f5f3c4e0aa3515cb60acc79e7a69f2b5f28f0c58aff4c04b9c60bb932` |

## Implication for the next decision

There is no evidence here that a different fitter is needed for match and
observation deletion. The common fitter with an exact Gram removes the two
additional failures seen with the direct-probe approximation. This supports
retaining the common scientific architecture while treating accurate scalable
Gram computation as an engineering/numerical task with these exact matrices
as benchmarks. It does not establish a feasible large-data exact method.

The controls discrepancy is a separate limitation. This experiment leaves
variance-model approximation, finite-sample fitted-variance effects, remaining
inference approximations and Monte Carlo uncertainty unresolved; it does not
prove that controls themselves cause the problem. Do not use more Gram probes
as a proposed cure for that residual discrepancy. Any further targeted
diagnosis or bounded approximate-inference paper decision requires an explicit
next scope; neither integration nor a new experiment starts automatically.

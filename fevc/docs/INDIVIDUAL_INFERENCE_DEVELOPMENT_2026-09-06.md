# Runnable individual inference: development fails promotion gates

The approved software integration is implemented locally, with **200 JLA
probes unchanged**. The predeclared 48-cell development experiment is complete
and **FAIL**. No independent confirmation, paper promotion, version bump,
public release or all-platform distribution follows this result.

The machine-readable [result](individual_inference_development_v1_result.json)
binds the run to its immutable source bundle, executables, manifest, raw
output and independent audit. The source commit is `1aeed065`, with the tested
uncommitted implementation captured in that bundle. It is not evidence about
the old committed code alone.

## Implemented interface

Observation structured inference uses residual moments with 512 projected
Gram probes; match retains its cross-fitted fitter. Native V5 and Stata
separate individual target availability from joint covariance admissibility.
Invalid joint matrices are withheld, not clipped. q1 posts no Gaussian `e(V)`.
Fit-specific diagnostics, resource/Counter/solve accounting, stale-interface
rejection and target-status validation are integrated. The exact Mata family,
point-only defaults, explicit q selection and supported routing are unchanged.
Fixed-offset match inference still omits control-estimation uncertainty.

Observation ordering is outcome-free. Identical-design rows need not be
bitwise permutation-invariant without a stable optional key. The separate
32-seed/64-call [ordering check](individual_ordering_development_v1_result.json)
passes its prospectively registered distributional and point-equivalence
checks. That is not a coverage result.

## Default-setting development

There are 400 replications in each of 20 observation, 14 match-q0 and 14
match-q1 cells: **19,200 native calls and 76,800 target attempts**. Every call
uses the public native prepare/augment/capability/solve/V5-export/release
lifecycle. Settings are 200 JLA probes, 1,000 covariance probes, 128 spectral
probes, 512 observation or 128 match spectral iterations, and 100,000 critical
draws per computable q1 target. Numerical and fold seeds are fixed; fresh
outcomes use registered semantic keys. At most eight local workers ran.

Before repeated outcomes, the 96-call tiny pipeline passed record validation,
unchanged high-precision match oracle checks, exact observation-regime checks,
and split/reversed-task reproducibility. Six identical-data Stata/native
comparisons passed a scaled `1e-7` gate, including both deletion units, both
references and controls. Sixteen harness tests cover malformed, missing,
duplicate and scientifically failing inputs. A separate auditor, with six
fault-injection tests, independently reconciles the raw counts and summary
statistics. Accounting passes; scientific qualification does not.

All attempts are classified: 19,186 returned calls, 72,236 computed target
intervals, 3,200 q0 spectral-certificate failures, 771 local q0 nonpositive-
variance failures, 537 local q1 nonpositive-variance failures, and 56 target
failures belonging to 14 shared variance-fit failures. The shared failures
occur only in severe omitted-driver match stress cells: nine q0 and five q1
calls have an all-floored variance-regression fold.

There are 1,663 calls with an inadmissible joint matrix. The new interface
retains **5,388 individually computable intervals** from those calls. This
demonstrates reporting behavior, not coverage validity in weak/null or
multi-mode cases, which retain their original exclusions.

### Failed gates

| Family and case | Target | Availability | Coverage of nominal 95% interval | Empirical SD / mean reported SE |
| --- | --- | --- | --- | --- |
| Observation q1, dominant mode with controls | Worker | 400/400 | 97.75%: fail | 0.9075: pass |
| Same | Total | 400/400 | 97.75%: fail | 0.8952: fail |
| Match q1, equal independent matches with one mode | Total | 400/400 | 98.25%: fail | 0.9946: pass |
| Match q0, equal independent matches | Worker and firm | 0/400 each | Unavailable | Unavailable |

The q0 failure also occurs for the forced-CMG diagnostic, leverage-only
sensitivity and match-q1 campaign's q0 comparator: four distinct cells,
accounting for all 3,200 status-6 failures. The JSON enumerates every failed
row and criterion; no target is removed because neighboring targets pass.

The three coverage failures are **overcoverage**, not undercoverage. The
inherited gate uses `max(0.015, 3*coverage_MCSE)` around 0.95. Its measured
upper bounds are approximately 97.2245% for the two observation rows and
96.9669% for the match row. The observation total SE ratio misses the unchanged
0.90 lower bound. Four hundred replications are development, not the planned
2,500-replication confirmation. Sampling variation may explain marginal
failures, but is not permission to relabel this result as a pass.

### Interpretation of the q0 failure

This is a spectral convergence limitation at the public defaults, not failure
of the regression solves or a negative scalar variance. In the equal-match
tiny fixture, worker spectral residuals are about 0.00220 and 0.00217; the firm
second-mode residual is about 0.00257, versus a 0.002 limit. Scalar variances
are positive, joint covariance is admissible, and the maximum regression
residual is about `5.2e-15`, below its `1e-9` gate. The current contract still
requires a certified q0 spectral calculation. Nearly tied eigenmodes are the
immediate numerical obstacle.

Do not infer that 200 JLA probes must be increased. For the failed observation
worker/total rows, reported point-estimator numerical MCSE is about 0.85%/0.45%
of empirical sampling SD; for the failed match-q1 total row it is about 1.42%.
These descriptive checks do not measure all random approximation error in
fitted variances or spectral calculations.

## Engineering evidence and limitations

The full Python suite passes 732 tests; the separate harness passes 16.
Workspace Rust tests, individual-covariance tests, plugin tests, Clippy and CMG
generated-source checks pass. Native Apple Silicon/Rosetta/universal and
isolated-install checks pass, as do the integrated Stata quick/full suites and
local package audits. Exact identities are in the
[engineering receipt](individual_inference_engineering_v1_result.json).
A dirty-tree Mac pass is not
exact-commit release qualification. No Linux or Windows requalification or
AWS instance was started. The complete payload remains unqualified.

Harness failures are preserved locally. The first seed validator used the
standard FNV multiplier instead of the fixture's frozen literal; it was
corrected against a known frozen seed vector. The Stata parity driver initially
used unsupported `threads()` and observation frequency-weight syntax; the
corrected driver uses `set processors 1` and supported tuples. These problems
were caught before repeated development outcomes. The Rosetta test now
compares intervals/statuses rather than near-tied mode coordinates, retaining
its original `1e-8` interval gate. Its initial matrix-slice syntax failure and
successful focused retests are retained.

A final transport review added a narrow guard: partial q0 output cannot use
legacy pre-V5 result interfaces, even when joint covariance is admissible.
This does not alter the campaign's V5 path, core estimator, fixtures or gates.
Its later source/binary identities are separate from the failed campaign.

## Next owner decision

Stop before confirmation and paper promotion. The next useful bounded task
is to settle the q0 spectral requirement/convergence problem at **200 JLA
probes**, then diagnose conservative q1 rows on saved draws. Do not replace
the match fitter, change critical-value formulas, weaken gates or add draws
merely to obtain a pass. A changed method or reporting contract needs a
documented justification and a new prospective validation decision. Historical
match confirmations and the 3,200-probe internal observation result retain
their original source, settings and limitations.

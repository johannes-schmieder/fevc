# Q1 curvature and fixed-offset inference erratum

The repair is registered in `inference_repair_v1.json`. Earlier exact-source
receipts and reports remain historical evidence and are not edited. Their q1
coverage findings do not qualify the corrected implementation: both production
and the old comparison harness used the same incorrect curvature formula.
Observation q1 and internal match q1 require fresh confirmation. No public
match route or release is authorized by this repair.

## Curvature

Write the q1 map as `lambda*b^2 + r`, and let its estimated joint covariance
have entries `V_b`, `C`, `V_r`. The maximal curvature after whitening is

```text
kappa = 2*abs(lambda)*V_b / sqrt(V_r - C^2/V_b).
```

The previous Rust, Mata, and V4 comparison formulas used `sqrt(V_b)` in the
numerator. A standardized parabola's second derivative supplies an independent
geometric regression. Rescaling outcomes multiplies `V_b` by the square of
the scale and the conditional remainder variance by its fourth power, leaving
the corrected curvature unchanged. The previous formula failed this check.
The reference is [KSS (2020), p. 1878](https://eml.berkeley.edu/~pkline/papers/KSS2020.pdf).

Covariance admissibility now uses `1 - C^2/(V_b*V_r)` after requiring positive
marginal variances. This is dimensionless; comparing the raw determinant with
the square of the larger marginal variance mixed different units. Invalid
covariance estimates remain unavailable. No ridge, clipping to positivity,
automatic q switch, or alternative confidence interval is introduced.

## Recenter and failure scope

Both implementations recenter the leading square with the raw leave-out
product `sum(v_i^2*y_i*ehat_i,-i)`, not the positive fitted variance used for
studentization. They separately check the direct rank-one-subtracted remainder.

An unavailable target no longer erases other computed q1 targets. Point
estimates and the full shared component covariance remain unchanged. Status
and raw covariance diagnostics distinguish nonpositive marginal variance,
singular covariance, interval failure, and an uncertified target mode. Shared
model, solver, structured-variance, full-covariance, resource, and identity
failures still fail the entire attachment. Successful execution does not
establish the one-mode/diffuse-remainder condition.

Native result ABI V4 preserves the first 16 q1 diagnostic columns and appends
status, standardized determinant, linear-influence variance, and trace
correction. V2/V3 reject partial results before exporting any arrays. Stata
keeps the 17-column `e(q1_inference)` layout, adds `e(q1_status)` and
`e(q1_computed_targets)`, and posts missing AM endpoints for unavailable
targets. The q0 columns in the q1 table are explicitly comparator results,
not substituted q1 intervals.

The Mata inference runtime is API 2. An already-loaded API-1 runtime is rejected
with `STALE_INFERENCE_RUNTIME`; restart Stata or use `discard` after updating.
Native V4 also reports actual solver columns and critical draws, including
zero critical draws for targets withheld before interval construction.

## Fixed-offset approximation

The required description is **fixed-offset approximate match inference,
ignoring nuisance-control estimation uncertainty**. The match collapse is an
exact algebraic reduction for a given offset. That fact does not establish a
valid conditional sampling distribution after conditioning on a control
coefficient estimated from the same outcomes. Such conditioning can change
the dependence and mean-zero properties of the residualized errors.

The working model allows unrestricted covariance inside each original match,
independence across original matches, and a structured aggregate-match
variance model. Estimated-offset uncertainty is omitted by owner choice;
neither a correction nor a second stage is included. Known-offset and
no-control experiments assess the approximation without this additional
omission. Repeated estimated-offset experiments are required limitation
diagnostics, not evidence of conditional validity. Historical fields named
`nuisance_uncertainty_conditioned_away` mean only that the implementation
omitted this uncertainty; the name is not a statistical guarantee.

## Development replay

The first local repair replay retained all 93 selected replications (the 31
historical failures plus their adjacent replications) and all 372 targets.
Independent dense reconstruction agreed with the production leading variance,
cross covariance, linear-influence variance, score, and direct remainder to
`4.33e-14` in absolute value. This is a development diagnostic, not a new
confirmation run.

All 279 worker, firm, and total target intervals were computed. Thirty
covariance-target failures reproduced: 28 remained invalid with exact traces
and known aggregate variances, one became valid with exact traces and fitted
variances, and one required the known variance vector as well. Thus most
failures are negative realized-influence-minus-trace estimates in a target
whose remainder is not diffuse, not Monte Carlo trace noise. The local replay
of historical replication 26 succeeded; the exact source/platform difference
must remain explicit rather than assigning it an unobserved failure cause.
No passing replay target was rescued solely by the new dimensionless gate.

The replay's synthetic raw inputs, outputs, and comparison values are generated
by `fevc/tools/diagnose_inference_repair.py` and the dedicated Rust example.
The old campaign is still a failure under its original atomic success rule.
Target-specific reporting changes what future campaigns count; it does not
retroactively turn that campaign into a pass.

## Current validation and remaining gates

The corrected formula passes independent parabola geometry, outcome scales
0.01 through 100 in both Rust and Mata, leading-coordinate rescaling, and a
dense angular interval oracle. Production Counter-V1 critical values at
100,000 draws match an independent Simpson integration check at five curvature
values and all four target streams. Native tests exercise real partial results
through V4 and rejection without array writes through V3. Focused Mata and
Rust-backed Stata tests pass, as do Rust workspace/backend tests and Clippy.

The new 14-cell tiny pipeline completes design preflight, frozen manifest,
generator, row validator, per-task receipts, and aggregate: 56 target rows and
no omitted attempts. This is a pipeline test, not a coverage result. Python
tests cover malformed/duplicate/missing outputs, source and binary mismatches,
partial targets, fixed-fold fingerprints, and confirmation inventory.

Exact-source native qualification, the required compute-node smoke, fresh
match development/confirmation, full match q0 confirmation, and independent
observation q1 confirmation remain required. The new match harness includes
only one diffuse q0 comparator and must not be presented as full q0 coverage
qualification. Match inference remains internal and q1 metadata explicitly
identifies the pending fresh confirmation.

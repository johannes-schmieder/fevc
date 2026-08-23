# Rust exact stayer-hybrid implementation plan

Date: 2026-08-23
Branch: `codex/rust-backend-completion`

## Objective

Close Rust parity for the separately labelled `stayers(both)` exact hybrid
without changing the existing estimand or silently broadening its robustness
claim. The implementation must reproduce the established Mata route, not
invent a new treatment of stayers.

Rust and Stata are tested only through the private `Licensed Stata CI` runner;
no local execution is claimed.

## Frozen statistical and sample contracts

1. Begin from the same robust mover sample and mover graph fixed point used by
   the existing exact route.
2. Never reclassify movers removed by graph pruning as stayers.
3. Attach only the one-firm stayers declared eligible by the existing public
   selector and only when their firm survives in the retained mover graph.
4. Preserve the existing physical-frequency semantics and explicit target
   weights exactly.
5. Normalize all reported target moments by the pooled retained mover-plus-
   stayer target mass.
6. Keep mover-match deletion corrections and stayer-observation deletion
   corrections as separate receipt blocks before adding them.
7. Preserve both joint-nuisance and fixed-offset modes.
8. Continue to label the stayer observation correction accurately: it is not a
   claim of robustness to arbitrary dependence across observations or matches
   beyond the declared deletion units.
9. Apply the same no-hidden-ridge, no-silent-tolerance-change, and complete
   original-system residual-certification rules as the mover-only exact route.

## Implementation sequence

### Phase S1: source-bound oracle and transport

- Extract the existing Mata stayer selector, target accounting, normalization,
  and correction identities into deterministic small fixtures.
- Add one fixture for each combination of frequency/explicit target weights and
  joint/fixed-offset nuisance handling.
- Include graph-dropped movers, ineligible stayers, stayers on removed firms,
  duplicate stored rows, and zero/near-zero residual cases.
- Freeze plugin, correction, corrected target, pooled-mass, and accounting
  identities before writing production Rust code.

### Phase S2: columnar Rust input and validation

Add a stayer-augmentation input surface containing only dense columnar arrays
needed after mover preparation:

- retained-firm index;
- stayer worker index;
- outcome, frequency, target weight, and optional controls/offset;
- observation deletion-unit index;
- canonical row order and eligibility flags supplied by the public selector.

Validation must reject non-finite data, invalid indices, nonpositive physical
frequency, negative target weight, inconsistent lengths, integer overflow, and
rows attached to firms outside the prepared mover graph. No observation-by-
parameter matrix may be formed.

### Phase S3: exact point targets

Reuse the prepared mover exact solution and nuisance coefficients. Accumulate
stayer worker means and target moments with deterministic fixed-order
reductions. Maintain separate mover, stayer, and pooled numerators so every
identity is independently checkable before final division by pooled target
mass.

### Phase S4: stayer observation correction

Implement the established observation-deletion correction using matrix-free
inverse actions from the prepared mover session and stayer-local sufficient
statistics. Batch right-hand sides under the existing memory planner; do not
materialize dense leverage matrices. Export separate correction numerators for
worker variance, firm variance, covariance, and total variance, plus maximum
complete residuals and accepted/deleted observation counts.

### Phase S5: ABI and Stata boundary

- Add a versioned capability/solve/result surface rather than extending a
  frozen prefix in place.
- Reconcile every dimensional, target-mass, deletion-count, batch, residual,
  memory, and accounting field on the Stata side.
- Preserve primary errors across result export and release cleanup.
- Keep `stayers(both)` explicitly opt-in; omitted `stayers()` remains the
  current mover-only behavior.

### Phase S6: qualification and performance

Require, in order:

1. Rust unit and dense-oracle tests;
2. exact-SHA `quick` receipt;
3. licensed Stata mover-only non-regression and Mata/Rust stayer differential
   fixtures;
4. comprehensive `plugin-build` with native arm64 and Rosetta x86_64 loading;
5. `full` and `benchmark-small` profiles;
6. large synthetic memory/lifetime and 1--32 thread invariance tests.

## Memory and speed rules

- Reuse the prepared mover graph, factorization/preconditioner, controls basis,
  and thread executor.
- Store dense IDs in the narrowest checked representation supported by the
  problem size.
- Process stayer rows in deterministic tiles; memory must be linear in rows,
  retained parameters, and admitted batch width.
- Pack converged right-hand sides and stop traversing inactive columns while
  preserving logical output order.
- Record setup, mover solve, stayer aggregation, correction, verification, and
  export times separately.

## Immediate coding checkpoint

Create the validated Rust stayer input/receipt types and dense finite oracle
first. Do not expose a public route until those tests and the exact-SHA quick
receipt are green.

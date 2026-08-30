# Package agent instructions

## Scope and sources of truth

This directory is the sole public `vckss` Stata package. It implements KSS
leave-out bias-corrected estimates for linear worker--firm variance
decompositions. CMG is an internal component under `cmg/`; the optional Rust
plugin is a backend, not a replacement package.

Read the repository-root `AGENTS.md`, this file, `PLAN.md`, and
`docs/README.md` before substantive work. `PLAN.md` contains only the
current objective and checkpoint. Durable decisions and exact contracts live
in `docs/DECISIONS.md`, `docs/ESTIMATOR_CONTRACT.md`,
`docs/NUMERICAL_ARCHITECTURE.md`, and `docs/FAILURES_AND_RETURNS.md`.

Candidate comparison and qualification follow
`docs/development_acceptance_v1.json`. Current feature and platform status
comes from `docs/RUST_MATA_PARITY.md`, not historical receipts.

## Statistical contract

Development requires equivalent statistical results, not identical
floating-point paths. Bitwise/ULP identity, iteration counts, reduction order,
and legacy fixed-roundoff gates remain diagnostics unless the registered policy
says otherwise.

Preserve:

- the estimator, retained sample, target population, deletion fixed point,
  weighting, nuisance, and accounting meanings;
- worker variance, firm variance, worker--firm covariance, and variance of
  their sum as the four corrected targets;
- point estimation as the default, with econometric covariance posted only for
  an explicit capability-gated inference request;
- match deletion as the default, `deletionid()` independent of coefficient
  cells, and mover-only match headlines;
- positive integer frequency weights as literal physical copies and explicit
  target weights as stored-row mass;
- coefficient cells, deletion units, and exact target-scale strata as distinct
  indices; and
- typed withholding for unsupported, unidentified, singular, nonestimable,
  unconverged, or resource-inadmissible requests.

Never describe probe dispersion or numerical MCSE as an econometric standard
error. The finite-projection formula and its legacy-comparator distinction live
in `docs/JLA_FINITE_PROJECTION.md`; do not restate or fork them here.

## Numerical contract

- Never construct a production observation-by-observation matrix.
- Solve on the full-firm zero-sum quotient. Ground a displayed coordinate only
  after convergence and still certify the original equation.
- Accept a solve only through the complete original-system residual contract;
  graph, Schur, recursive, or reduced residuals are insufficient.
- Use the phase tolerances and residual gates registered in the development
  policy and numerical architecture. Do not change public `tolerance()`
  behavior as part of a comparison.
- Keep cancellation-sensitive reductions stable or compensated.
- Preserve rank, inverse, reciprocal, maker, control-basis, deletion,
  accounting, direct-memory, and finite-output gates.
- Do not introduce hidden regularization, sample changes, tolerance relaxation,
  or post-failure estimator changes.

Different backends may use different reductions, stopping points, and numerical
representations when they satisfy the same registered statistical and hard
correctness gates.

## Backend, routing, RNG, and resources

- Resolve backend, algorithm, engine, solver route, batch, memory, and every
  permitted fallback from the effective request before estimator RNG.
- Missing native runtime or a structurally unsupported tuple may fall back to
  Mata only before native preparation and RNG. Later failures fail closed.
- Keep `backend(rust)` strict and `backend(mata)` explicitly Mata.
- Exact uses no estimator RNG. JLA follows the registered runtime-scoped RNG
  contract with separate leverage and target domains.
- Restore caller RNG algorithm, stream, complete state, data, `e(sample)`,
  and sort state on every exit.
- Explicit CMG fails closed. Automatic CMG-to-diagonal fallback is allowed only
  before RNG and must be receipted.
- Reconcile requested and selected capability, route, solver, batch, memory,
  fallback, residual, and result-family fields in returned receipts.
- Treat `memory_gib()` as a per-command direct-allocation safety envelope,
  not a repository-wide development ceiling. Forecast, admit, and reconcile
  material allocations and measured RSS honestly.
- Treat wall forecasts and headroom as advisory unless a real allocation limit
  applies. Performance claims require measured complete-command evidence.

## Development and evidence

For behavioral or numerical work, add a focused failing regression, keep its
oracle independent, and run the smallest relevant gate while iterating. Select
integrated, clean-install, native, platform, or scale gates from the affected
surface described in `TESTING.md`.

A green quick receipt is not native qualification. Rust-boundary changes need
the source-local plugin profile and exact-SHA evidence. Documentation-only work
normally reuses unaffected scientific and performance evidence.

Do not edit source-bound reports, receipts, reviews, manifests, or archived
benchmark outputs. Do not infer a new benchmark or release claim from them.
Follow `../CODE_LICENSE.md`, `docs/SOURCE_PROVENANCE.md`, and the CMG
provenance records; public release remains a separate owner decision.

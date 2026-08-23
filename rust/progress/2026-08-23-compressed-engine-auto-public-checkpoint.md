# Compressed `engine(auto)` public-route checkpoint

Date: 2026-08-23

## Implemented source

The public Rust router now admits the previously withheld no-control,
match-deletion `engine(auto)` tuple into the planned V4/V7 JLA runner.  This
admission is deliberately narrow:

- `backend(rust)` and `rng(counter_v1)` remain explicit;
- `algorithm(jla)` and `engine(auto)` remain explicit;
- match deletion and movers-only semantics are preserved;
- the request has no materialized controls;
- engine selection is frozen before estimator RNG; and
- the native planner may select the compressed engine only under its existing
  scientific eligibility rule.

The qualified generic-only `engine(auto)` cases, forced generic routes, legacy
explicit-batch compressed route, exact route, omitted-backend Mata policy, and
all failure behavior are otherwise unchanged.

## Result-family boundary

When the frozen plan selects compressed, the Stata boundary now:

1. requires compressed RHS receipt schema V1 and execution-plan V1/V7 truth;
2. reconciles capability, preparation, graph, memory, route, batching,
   Counter, residual, accounting, and lifecycle receipts without interpreting
   them as generic V2 diagnostics;
3. posts common point-estimate and numerical fields plus a separately labelled
   compressed receipt; and
4. deliberately omits generic-only maker, control-rank, Schur, and generic
   memory diagnostics.

The public regression verifies the selected engine and route, result and RHS
families, complete original-model residual certificates, four-target
accounting identities, memory and capability receipts, absence of
family-inapplicable generic postings, `e(sample)`, caller data and sort state,
caller RNG state, and native release to idle.

## Evidence boundary

The implementation source commit is
`f8c7a908b0a3ed93e1586b42b4c64dfea23e16fe`.  This progress commit is a
recovery checkpoint on the same source tree and must receive its own exact-SHA
`quick` receipt.  Until that receipt passes, this route is implemented but not
quick-qualified.

Because this changes the public Rust/native result boundary, a comprehensive
`plugin-build` profile is also mandatory before the route may be described as
qualified.  Public distribution and license/provenance gates remain closed.

## Next steps

1. Require the exact-SHA quick receipt and separately verify the Rust quick
   gate.
2. Repair only source-bound failures and rerun quick.
3. Extend public compressed coverage across the registered automatic and
   forced preconditioner boundaries, including setup-only fallback behavior.
4. Run the full macOS arm64/Rosetta/universal `plugin-build` qualifier.
5. Update the principal progress and implementation-status documents with the
   final qualified SHA and run evidence.

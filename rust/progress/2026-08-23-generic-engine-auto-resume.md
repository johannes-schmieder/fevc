# Generic-only engine(auto) public route: production checkpoint

Date: 2026-08-23
Branch: `codex/rust-backend-completion`

## Generated source

The trusted transformer produced commit:

`880b147d1b9352e15e549085ae73c017759e4750`

The production change exposes `engine(auto)` only for planned JLA tuples whose
scientific eligibility guarantees that the native engine planner must select
the generic engine:

- at least one materialized control; or
- observation deletion.

No-control match-deletion tuples remain withheld because they may select the
compressed engine, whose result family is not yet reconciled by the public
planned generic wrapper.

The change is intentionally limited to explicit `backend(rust)` and
`algorithm(jla)`. Omitted `backend()` and `backend(auto)` continue to use Mata.
The frozen V2 explicit generic-diagonal route remains unchanged.

## Receipt contract being tested

For the admitted auto-engine tuple, the public boundary must report:

- capability schema 3/profile 4;
- requested engine code 0 and deferred engine resolution;
- selected engine code 2 (`generic`);
- result requested engine code 0 and selected engine code 2;
- the existing V4/V7 route, batch, wall, memory, Counter-V1, residual, and
  cleanup contracts; and
- numerical identity with the explicit generic forced-diagonal reference.

The public tests also require a no-control match-deletion `engine(auto)` tuple
to fail closed before native preparation.

## Exact continuation

This documentation commit is a normal push so the generated production source
receives an exact `quick` receipt plus Rust formatting, strict Clippy, and
workspace-test evidence. After quick closure, run the comprehensive
`plugin-build` profile before widening the engine-auto surface further.

Rust and Stata are not executed in the ChatGPT sandbox. All qualification must
come from the exact-SHA private `Licensed Stata CI` receipts.

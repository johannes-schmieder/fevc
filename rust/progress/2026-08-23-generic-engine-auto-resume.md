# Generic-only engine(auto) public route: production checkpoint

Date: 2026-08-23
Branch: `codex/rust-backend-completion`

## Generated source

The trusted transformer produced the initial route commit:

`880b147d1b9352e15e549085ae73c017759e4750`

The production change exposes `engine(auto)` only for planned JLA tuples whose
scientific eligibility guarantees that the native engine planner must select
the generic engine:

- at least one parsed/materialized control; or
- observation deletion.

No-control match-deletion tuples remain withheld because they may select the
compressed engine, whose result family is not yet reconciled by the public
planned generic wrapper.

The change is intentionally limited to explicit `backend(rust)` and
`algorithm(jla)`. Omitted `backend()` and `backend(auto)` continue to use Mata.
The frozen V2 explicit generic-diagonal route remains unchanged.

## Quick qualification of the initial source

Source SHA `880b147d1b9352e15e549085ae73c017759e4750` passed the exact
`quick` profile in run `32642134572`, including licensed Stata, Rust formatting,
strict Clippy, workspace tests, receipt finalization, and publication.

## Comprehensive failure and exact repair

The first comprehensive checkpoint, source SHA
`88c35da729e7c049a18b324fe8b8ae9bacada13b`, failed in run
`32642295418`. Rust quick checks passed. Licensed Stata reached the public
`engine(auto)` fixture but the command failed before native preparation with
`RUST_OPTION_UNSUPPORTED`.

The early public support predicate incorrectly inspected the later materialized
`controlvars` local. At that phase only the parsed `controls()` specification
is authoritative. The trusted repair therefore changed the predicate to inspect
`controls` and added `ci/tests/test_stata_engine_auto_phase.py`, which requires
the early predicate to use parsed controls and forbids `controlvars` there.

The repair staging SHA `9f12da7c0396aac54ec0ee59786f9388043d8063`
passed the exact `quick` profile in run `32642620345`, including all Rust gates
and the new regression test. The trusted apply job then generated production
commit:

`889bdebf277f71d2dc800c39bb69732cc9f159ba`

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

This documentation commit is a normal push so repaired production source
`889bdebf277f71d2dc800c39bb69732cc9f159ba` receives source-bound `quick`
evidence. After quick closure, rerun the comprehensive `plugin-build` profile
before widening the engine-auto surface further.

Rust and Stata are not executed in the ChatGPT sandbox. All qualification must
come from exact-SHA private `Licensed Stata CI` receipts.

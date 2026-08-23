# Generic-only engine(auto) public route qualified

Date: 2026-08-23
Branch: `codex/rust-backend-completion`

## Qualified scientific subset

The public planned Rust route now accepts explicit `engine(auto)` only when the
native engine planner is scientifically guaranteed to select the generic
engine:

- the parsed command contains at least one control; or
- deletion mode is observation.

No-control match-deletion tuples remain withheld because they may select the
compressed engine. That case requires a public reconciler covering the
compressed result family rather than forcing compressed receipts through the
generic wrapper.

Omitted `backend()` and `backend(auto)` continue to use Mata. The frozen V2
explicit generic-diagonal and compressed routes remain unchanged.

## Initial quick evidence

Initial production source `880b147d1b9352e15e549085ae73c017759e4750`
passed the exact `quick` profile in run `32642134572`.

## Comprehensive failure and repair

The first comprehensive source SHA
`88c35da729e7c049a18b324fe8b8ae9bacada13b` failed in run
`32642295418` before native preparation. The early public support predicate
looked at the later materialized `controlvars` local, so a match-deletion tuple
with a parsed control was incorrectly classified as unsupported.

The repair changed the early predicate to inspect the parsed `controls()`
specification and added a static phase-order regression guard. Staging SHA
`9f12da7c0396aac54ec0ee59786f9388043d8063` passed the full quick lane in
run `32642620345`. The trusted apply job generated production commit
`889bdebf277f71d2dc800c39bb69732cc9f159ba`, which passed the exact quick
profile in run `32642718436`.

## Comprehensive qualification

Exact source SHA:

`6bca4a7f46e5ff55e7b96c0e87bf6e29dc465cea`

Licensed Stata CI evidence:

- profile: `plugin-build`;
- status: `success`;
- Stata RC: `0`;
- process RC: `0`;
- run ID: `32642864491`;
- Rust formatting, strict Clippy, and workspace tests: PASS;
- C shim interrupt/error transport and ABI compatibility: PASS;
- native arm64 plugin: `PASS_NATIVE`;
- Rosetta x86_64 plugin: `PASS_ROSETTA`;
- signed universal plugin and deployment/export audits: PASS; and
- classification: `CLEAN_LOCAL_MACOS_CANDIDATE_QUALIFICATION`.

The licensed public fixture verifies requested engine code 0, deferred engine
resolution, selected engine code 2, numerical equality with explicit generic
forced diagonal, V4/V7 route/batch/wall/memory receipts, complete residuals,
Counter-V1 accounting, state restoration, and idle lifecycle cleanup.

Rust and Stata were not executed in the ChatGPT sandbox. This is source-local
macOS ARM64/Rosetta development evidence, not native Windows/Linux/Intel Stata
or cross-hardware performance evidence.

## Next recovery point

1. Restore ordinary pushes to profile `quick`.
2. Implement the compressed-eligible no-control match `engine(auto)` route with
   a reconciler based on the frozen compressed public result contract.
3. Require exact quick and comprehensive plugin qualification before exposing
   the full engine-auto surface.
4. Then address `algorithm(auto)`, Rust stayer-hybrid parity, and large-N
   performance/memory optimization.

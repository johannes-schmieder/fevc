# Public planned Rust route: exact resume point

Date: 2026-08-23
Branch: `codex/rust-backend-completion`

## Qualified prerequisite

The private V4/V7 boundary is comprehensively qualified at source SHA
`0d47e7e1a464c068c0cd61d8a1fcb6d0738e42eb` under the private self-hosted
`Licensed Stata CI` workflow with profile `plugin-build`, status `success`,
Stata RC 0, and process RC 0. The run covered Rust 1.81 fmt/Clippy/tests, C and
ABI fixtures, native arm64 and Rosetta x86_64 candidates, signed universal
loading, isolated installation, lifecycle, planned V4/V7 reconciliation,
differential checks, and numerical residual checks. Rust and Stata were not
run in the ChatGPT sandbox.

## Public route scope

The first planned public route is deliberately narrow:

- explicit `backend(rust)`;
- `algorithm(jla)`;
- `engine(generic)`;
- `preconditioner(auto)`;
- explicitly supplied `batch(auto|#)`;
- `rng(counter_v1)`;
- optional advisory `wallseconds()`;
- match or observation deletion;
- joint or fixed-offset nuisance handling;
- movers only and no `probeorder()`.

Omitted `backend()` and `backend(auto)` continue to use Mata. The proven legacy
explicit generic/diagonal V2 route remains a separate unchanged program.

## Applied production structure

The stale chained-anchor transformer was replaced at staging SHA
`902ea7f3b5575287f945a588daa72f8ed0f8967d` by a fail-closed structural
transformer tied to the current program boundaries. The trusted apply workflow
completed successfully and removed its staging files.

The generated production source now contains:

- the unchanged `_vckss_rust_generic` V2 lifecycle;
- a separate `_vckss_rust_generic_planned` lifecycle;
- V3 request-capability reconciliation;
- V4 planned solve dispatch;
- additive V7 route, batch, wall, and memory reconciliation;
- dynamic selected-route and selected-batch validation;
- planned-route `e()` metadata; and
- a narrow support predicate and dispatch in `_vckss_impl`.

Static inspection confirms that the permanent omitted-backend and
`backend(auto)` Mata defaults remain in the generated source. The generated
source is not called green until an exact-SHA receipt is present under
`.ci/stata/results/` with profile `quick`, status `success`, and Stata RC 0.

## Immediate continuation

1. Verify the exact-SHA quick receipt for this checkpoint.
2. Fix any Stata syntax, receipt, Rust fmt/Clippy, or Rust-test failure and push
   a new focused checkpoint.
3. Add a public synthetic fixture to
   `varcomp_kss/tests/stata/test_rust_public_generic.do` using
   `preconditioner(auto) batch(auto) wallseconds(60)`.
4. Require schema 3/profile 4, requested route auto, selected diagonal on the
   small fixture, automatic phase batches, V7 plan fields, memory lifetime
   identities, complete residuals, and point-target identities.
5. Rerun the comprehensive `plugin-build` profile and fix until exact-SHA
   green before exposing forced CMG or wider automatic routing.
6. Then continue Rust `stayers(both)` parity and large-N performance work.

Every subsequent source checkpoint must be committed and pushed to this
private branch, tested by exact SHA, and recorded here or in a successor
progress note.

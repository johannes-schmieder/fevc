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

Normal iterative pushes were restored to profile `quick`; source SHA
`d6c7772e9d704ee0c2e8dca0656e534c6c39ad94` is exact-SHA quick-green with
Stata RC 0 and all normal Rust gates passing.

## Public route being added

The first planned public route is deliberately narrow:

- explicit `backend(rust)`;
- `algorithm(jla)`;
- `engine(generic)`;
- `preconditioner(auto)`;
- `batch(auto|#)`;
- `rng(counter_v1)`;
- optional advisory `wallseconds()`;
- match or observation deletion;
- joint or fixed-offset nuisance handling;
- movers only and no `probeorder()`.

Omitted `backend()` and `backend(auto)` must continue to use Mata. The proven
legacy explicit generic/diagonal V2 route must remain untouched.

## Transformer state

The production edit is staged through the trusted fail-closed
`.ci/codex/apply.py` mechanism. The transformer clones the current legacy
`_vckss_rust_generic` program into a separate
`_vckss_rust_generic_planned`, then adds only the new support predicate and
dispatch in `_vckss_impl`.

Earlier transformer attempts failed before writing production source because
of stale textual anchors. Those failures did not modify `varcomp_kss.ado`.
The fresh structural transformer and successive anchor repairs were each
qualified by the exact-SHA quick lane. The most recent staging checkpoint is
`1c05cdd029ef3daf041511f079aff97f314f9bc8`, profile `quick`, status
`success`, Stata RC 0, run ID `32622559540`.

## Immediate continuation

1. Inspect the trusted apply job associated with run `32622559540` and the
   current branch head.
2. If the apply job failed, repair only the reported fail-closed anchor and
   rerun the exact quick lane.
3. If it succeeded, identify the generated production source SHA, require its
   exact quick receipt, and inspect the generated planned program and public
   dispatch.
4. Add a public synthetic fixture to
   `varcomp_kss/tests/stata/test_rust_public_generic.do` using
   `preconditioner(auto) batch(auto) wallseconds(60)`.
5. Require the public result to report schema 3/profile 4, requested route auto,
   selected diagonal on the small fixture, automatic phase batches, V7 plan
   fields, memory lifetime identities, complete residuals, and point-target
   identities.
6. Rerun the comprehensive `plugin-build` profile and fix until exact-SHA
   green before exposing forced CMG or wider automatic routing.

Every subsequent source checkpoint must be committed and pushed to this
private branch, tested by exact SHA, and recorded here or in a successor
progress note.

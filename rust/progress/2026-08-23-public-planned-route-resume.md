# Public planned Rust route: qualified checkpoint

Date: 2026-08-23
Branch: `codex/rust-backend-completion`

## Exact qualified source

The first public planned V4/V7 route is comprehensively qualified at source
SHA `c7a45237449e2060c4afe4797f42537d5f5964c9` under the private self-hosted
`Licensed Stata CI` workflow:

- profile: `plugin-build`;
- status: `success`;
- Stata RC: `0`;
- process RC: `0`;
- run ID: `32629327527`;
- Rust 1.81 formatting, strict Clippy, and workspace tests: PASS;
- C shim interrupt and error-transport tests: PASS;
- ABI-header compatibility: PASS;
- native arm64 candidate: `PASS_NATIVE`;
- Rosetta x86_64 candidate: `PASS_ROSETTA`; and
- classification: `CLEAN_LOCAL_MACOS_CANDIDATE_QUALIFICATION`.

Rust and Stata were not executed in the ChatGPT sandbox. The evidence comes
from the private Mac Studio runner and is bound to the exact source SHA above.

## Qualified public route

The public command now supports the complete explicit tuple:

- `backend(rust)`;
- `algorithm(jla)`;
- `engine(generic)`;
- `preconditioner(auto)`;
- explicitly supplied `batch(auto|#)`;
- `rng(counter_v1)`;
- optional advisory `wallseconds()`;
- match or observation deletion;
- joint or fixed-offset nuisance handling;
- movers only and no `probeorder()`.

The route performs V3 request-capability reconciliation, V4 planned solve
dispatch, and additive V7 route, fallback, independent-batch, wall, Counter,
residual, and memory reconciliation. The public fixture verifies selected
route and batches, V7 memory lifetimes, complete residuals, target identities,
RNG restoration, sort/data restoration, and idle-state cleanup.

Omitted `backend()` and `backend(auto)` continue to use Mata. The proven legacy
explicit generic/diagonal V2 route remains a separate unchanged program.

## Stata-boundary hardening

Licensed execution exposed two Stata name-limit defects that static source
review had missed:

1. dynamically generated V3 capability locals exceeded Stata's 32-character
   macro-name limit; and
2. the first public solve-peak `e()` name exceeded Stata's 32-character stored
   result-name limit.

Both were repaired. The public solve peak is now posted as
`e(rust_plan_solve_peak_bytes)`. A repository unit test now audits the planned
program's literal names, public `e()` names, and dynamically generated
`cap_<field>` aliases against the 32-character limit.

## Immediate continuation

1. Keep normal pushes on the exact-SHA `quick` Stata/Rust lane.
2. Expose and qualify forced generic CMG while retaining fail-closed behavior
   and setup-only fallback exclusively for `preconditioner(auto)`.
3. Broaden the public planned router to `engine(auto)` and then
   `algorithm(auto)` only after exact public receipt tests are in place.
4. Implement and independently verify Rust exact parity for the separately
   labelled `stayers(both)` mixed-deletion point hybrid.
5. Continue large-N memory and performance work, including reusable parallel
   execution, active-RHS packing, batched CMG applications, and representative
   same-machine benchmarks.
6. Run the manual cross-platform Rust/release matrices and complete packaging,
   documentation, and human license/provenance gates before release.

Every subsequent source checkpoint must be committed and pushed to this
private branch, tested by exact SHA, and recorded here or in a successor
progress note.

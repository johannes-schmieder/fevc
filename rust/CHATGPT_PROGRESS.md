# ChatGPT Rust backend progress log

Last updated: 2026-08-23
Active branch: `codex/rust-backend-completion`
Trusted remote test workflow: `Licensed Stata CI`

## Execution and evidence boundary

The ChatGPT execution sandbox does not run Rust or Stata locally. All Rust,
C, plugin, Mata, and Stata evidence is produced by the private self-hosted Mac
runner through GitHub Actions. The public `playground` repository is no longer
used for current development or qualification.

For every pushed source checkpoint:

1. wait for `.ci/stata/results/<full-source-sha>.json`;
2. require its `tested_sha`, `profile`, and `status` to match the intended run;
3. inspect the corresponding workflow jobs because a normal push also runs
   Rust formatting, strict Clippy, and workspace tests; and
4. treat the publisher's later `[skip ci]` commit as bookkeeping rather than
   the tested source SHA.

Manual plugin profiles use the comprehensive macOS arm64/Rosetta qualifier and
are required after meaningful plugin or public native-boundary changes.

## Current implementation state

Implemented and tested source now includes:

- exact worker--firm--control estimation;
- generic Counter-V1 JLA;
- diagonal and CMG PCG routes;
- structural algorithm, engine, preconditioner, and independent-batch plans;
- direct memory admission, advisory wall planning, and detailed receipts;
- capability V3, solve V4, execution-plan V1, and detailed receipt V7;
- Stata V4 dispatch and V7 reconciliation through the private boundary;
- public planned generic JLA with automatic routing and forced CMG;
- native arm64, Rosetta x86_64, and signed universal macOS plugin qualification;
- deterministic executor tests through 32 requested threads; and
- exact-SHA CI receipts for both successful and pre-Stata failure paths.

Omitted `backend()` and `backend(auto)` continue to select Mata. Public release
and license/provenance gates remain closed.

## Latest exact-SHA checkpoints

### CI guard and receipt finalization

Source SHA `e7fbcc80e6f5ec0600b150d91207254d0e9b4c24`:

- profile `quick`;
- status `success`;
- Stata RC `0`;
- process RC `0`;
- run ID `32640392921`;
- receipt regression tests: success;
- Rust formatting, strict Clippy, and workspace tests: success;
- finalizer and artifact upload: success; and
- exact-SHA publisher: success.

The syntax guard now distinguishes standalone `//` comments from required
`///` continuation markers. The always-run finalizer preserves a valid profile
receipt or synthesizes a schema-valid exact-SHA failure receipt when an earlier
gate prevents Stata from starting.

### Public planned forced-CMG route

Source SHA `b8ca41c635c6fb4c17f575a1b00aa75df76e4463`:

- profile `plugin-build`;
- status `success`;
- Stata RC `0`;
- process RC `0`;
- run ID `32640549303`;
- Rust fmt, strict Clippy, and workspace tests: PASS;
- C shim interrupt/error transport and ABI compatibility: PASS;
- native arm64: `PASS_NATIVE`;
- Rosetta x86_64: `PASS_ROSETTA`; and
- classification: `CLEAN_LOCAL_MACOS_CANDIDATE_QUALIFICATION`.

The qualified public planned generic surface is explicit `backend(rust)` with
`algorithm(jla)`, `engine(generic)`, `preconditioner(auto|cmg)`,
`batch(auto|#)`, `rng(counter_v1)`, optional advisory `wallseconds()`, match or
observation deletion, joint or fixed-offset nuisance handling, movers only,
and no `probeorder()`.

### Planned forced-diagonal staging

The trusted transformer and its commit message are staged at source SHA
`5f4d751615f8ccea1d78ff26f1f09b0ae96d225e`. It widens the V4/V7 public
predicate only for `preconditioner(diagonal)` requests that need automatic
batching or wall planning. The frozen explicit numeric-batch/no-wall diagonal
route remains on V2. This checkpoint must pass the quick lane before the
trusted apply job may create the production source commit.

## Remaining production work

1. Complete and qualify the staged planned forced-diagonal route without
   changing the frozen V2 path.
2. Widen public planned routing toward `engine(auto)` and `algorithm(auto)`
   only after exact/compressed/generic result and receipt reconciliation is
   complete.
3. Implement Rust exact parity for the separately labelled `stayers(both)`
   mixed-deletion point hybrid.
4. Continue large-N memory, multi-RHS PCG, CMG application, and deterministic
   parallel optimization, recording same-machine evidence separately from
   cross-hardware claims.
5. Complete native Windows/Linux and native Intel Stata qualification,
   packaging, documentation, release-security, mathematical review, and human
   license/provenance gates.

## Exact resume order

1. Require the staging checkpoint quick receipt and inspect its Rust gates.
2. Inspect the generated production commit and require its exact quick receipt.
3. Run `plugin-build` after that public-boundary change.
4. Continue to engine/algorithm automatic routing or, if that requires a larger
   universal wrapper refactor, checkpoint the design before implementation.
5. Then implement and independently verify Rust stayer-hybrid parity.

## Evidence rules

- Never claim Rust or Stata was executed locally.
- Never infer Rust success from a Stata receipt alone.
- Bind every result to the exact source SHA and profile.
- Preserve estimator, deletion, weighting, nuisance, RNG, solver, residual,
  target, memory, failure, and cleanup contracts.
- Keep public distribution disabled until the documented human
  license/provenance review is complete.

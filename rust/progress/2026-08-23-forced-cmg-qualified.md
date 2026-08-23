# Public forced-CMG route and CI finalization qualified

Date: 2026-08-23
Branch: `codex/rust-backend-completion`

## Exact quick checkpoint

The false-positive Stata syntax guard and exact-SHA receipt finalization were
qualified at source SHA:

`e7fbcc80e6f5ec0600b150d91207254d0e9b4c24`

Licensed Stata CI evidence:

- profile: `quick`;
- status: `success`;
- Stata RC: `0`;
- process RC: `0`;
- run ID: `32640392921`;
- receipt machinery tests: success;
- quick Rust formatting, strict Clippy, and workspace tests: success;
- exact-SHA receipt finalizer: success; and
- receipt publisher: success.

The static guard now distinguishes standalone `//` comments from required
`///` continuation markers. An `if: always()` finalizer preserves a valid
Stata-generated receipt or synthesizes a schema-valid exact-SHA failure receipt
when an earlier gate prevents Stata from running. This prevents completed red
runs from losing their per-SHA diagnosis and leaving only a stale `latest.json`.

## Comprehensive forced-CMG checkpoint

The repaired public planned generic forced-CMG route was qualified at source
SHA:

`b8ca41c635c6fb4c17f575a1b00aa75df76e4463`

Licensed Stata CI evidence:

- profile: `plugin-build`;
- status: `success`;
- Stata RC: `0`;
- process RC: `0`;
- run ID: `32640549303`;
- Rust 1.81 formatting, strict Clippy, and workspace tests: PASS;
- C shim interrupt and error-transport tests: PASS;
- ABI-header compatibility: PASS;
- native arm64 candidate: `PASS_NATIVE`;
- Rosetta x86_64 candidate: `PASS_ROSETTA`;
- signed universal plugin and deployment/export audits: PASS; and
- classification: `CLEAN_LOCAL_MACOS_CANDIDATE_QUALIFICATION`.

This qualifies the source-local macOS ARM64/Rosetta implementation and licensed
Stata lifecycle on the Mac Studio. It is not native Windows, Linux, or Intel
Stata qualification and is not a cross-hardware performance claim.

## Current public planned generic surface

The public command now has qualified planned generic JLA paths for explicit
`backend(rust)` with:

- `algorithm(jla)`;
- `engine(generic)`;
- `preconditioner(auto)` or `preconditioner(cmg)`;
- `batch(auto|#)`;
- `rng(counter_v1)`;
- optional advisory `wallseconds()`;
- match or observation deletion;
- joint or fixed-offset nuisance handling; and
- movers only, with no `probeorder()`.

Omitted `backend()` and `backend(auto)` continue to select Mata. The frozen
legacy compressed and explicit generic-diagonal routes remain unchanged.

## Next recovery point

1. Restore ordinary pushes to profile `quick`.
2. Expose and test the already implemented planned forced-diagonal route for
   automatic batching and wall receipts without changing the frozen V2 route.
3. Widen the planned public router toward `engine(auto)` and
   `algorithm(auto)` only with complete exact/compressed/generic receipt
   reconciliation.
4. Implement Rust exact parity for the separately labelled `stayers(both)`
   hybrid.
5. Continue large-N memory, multi-RHS, CMG, and deterministic parallel
   optimization, with same-machine benchmarks recorded separately.

Rust and Stata were not executed in the ChatGPT sandbox. All evidence above is
bound to the exact GitHub source SHAs through the private self-hosted runner.

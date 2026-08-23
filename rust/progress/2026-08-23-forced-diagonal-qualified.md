# Public planned forced-diagonal route qualified

Date: 2026-08-23
Branch: `codex/rust-backend-completion`

## Production source

The trusted transformer generated production commit:

`80409fdbe8a3d143dbfeed237a46b37177493108`

The change exposes the V4/V7 planned generic route for explicit
`preconditioner(diagonal)` requests only when automatic phase batching or wall
planning is requested. The frozen explicit numeric-batch/no-wall diagonal
route remains on capability V2/result V6.

## Exact quick evidence

Source SHA `80409fdbe8a3d143dbfeed237a46b37177493108`:

- workflow: `Licensed Stata CI`;
- profile: `quick`;
- status: `success`;
- Stata RC: `0`;
- process RC: `0`;
- run ID: `32641139637`;
- receipt tests: success;
- Rust formatting: success;
- strict Clippy: success; and
- Rust workspace tests: success.

## Comprehensive plugin evidence

Source SHA `1e854866ec8a1413ceb30c613e6a62aea6475553`:

- workflow: `Licensed Stata CI`;
- profile: `plugin-build`;
- status: `success`;
- Stata RC: `0`;
- process RC: `0`;
- run ID: `32641392217`;
- Rust 1.81 formatting, strict Clippy, and workspace tests: PASS;
- C shim interrupt/error transport and ABI-header fixtures: PASS;
- native arm64 plugin: `PASS_NATIVE`;
- Rosetta x86_64 plugin: `PASS_ROSETTA`;
- signed universal plugin and deployment/export audits: PASS; and
- classification: `CLEAN_LOCAL_MACOS_CANDIDATE_QUALIFICATION`.

The public tests verify that forced diagonal with `batch(auto)` and
`wallseconds()` uses capability V3/solve V4/result V7, returns requested and
selected route code 2, disables fallback, preserves automatic independent
phase widths, posts wall and memory receipts, matches the diagonal numerical
reference, restores RNG/sort/data state, and leaves the native lifecycle idle.
The existing explicit diagonal `batch(#)` without wall planning continues to
assert capability schema 2/profile 3.

Rust and Stata were not executed in the ChatGPT sandbox. This is source-local
Mac ARM64/Rosetta development evidence, not native Windows/Linux/Intel Stata or
cross-hardware performance evidence.

## Next recovery point

1. Restore normal pushes to profile `quick`.
2. Add a narrow `engine(auto)` public route for tuples structurally guaranteed
   to resolve to the generic engine and reconcile its requested/selected engine
   receipts.
3. Extend `engine(auto)` to compressed-eligible tuples only after a universal
   planned result reconciler covers both compressed and generic receipts.
4. Then address `algorithm(auto)`, Rust stayer-hybrid parity, and large-N
   performance/memory work.

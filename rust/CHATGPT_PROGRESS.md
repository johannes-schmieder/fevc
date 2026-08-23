# ChatGPT Rust backend progress log

Last updated: 2026-08-23
Active branch: `codex/rust-backend-completion`
Trusted remote test workflow: `Licensed Stata CI`

## Execution and evidence boundary

The ChatGPT execution sandbox does not run Rust or Stata locally. All Rust,
C, plugin, Mata, and Stata evidence is produced by the private self-hosted Mac
runner through GitHub Actions. No playground repository is used for current
development or qualification.

For every pushed source checkpoint:

1. wait for `.ci/stata/results/<full-source-sha>.json`;
2. require its `tested_sha`, `profile`, and `status` to match the intended run;
3. inspect the corresponding workflow run because the receipt describes the
   Stata profile while the same job separately executes Rust fmt, strict
   Clippy, and workspace tests on normal pushes; and
4. treat the receipt publisher's later `[skip ci]` commit as bookkeeping, not
   as the tested source SHA.

Manual plugin profiles use the comprehensive macOS arm64/Rosetta qualifier and
are required after meaningful plugin-boundary changes.

## Current implementation state

The repository already contains:

- exact worker--firm--control estimation;
- generic Counter-V1 JLA;
- diagonal and CMG PCG routes;
- structural algorithm, engine, preconditioner, and batch planning;
- direct memory admission and execution receipts;
- capability V3, solve V4, execution-plan V1, and detailed receipt V7 on the
  Rust/C side; and
- extensive Rust, C, Mata, Stata, ABI, and differential tests.

The main unfinished production work is:

1. complete solve V4 dispatch in `varcomp_kss/varcomp_kss_rust.ado`;
2. reconcile and export the full V7 execution-plan receipt;
3. add private success, corruption, nonconvergence, UserBreak, restoration,
   release, and idle-state tests around the V4/V7 boundary;
4. expose the planned automatic Rust routing surface while preserving Mata for
   omitted `backend()` and `backend(auto)`;
5. implement Rust exact parity for the separately labelled stayer hybrid; and
6. finish performance, packaging, source-bound plugin qualification, and
   documentation gates without weakening the human license/provenance gate.

## Completed self-hosted checkpoints

### CI infrastructure merge

Source SHA `020e63b95d5e6f27f61a672db11e92b4495b3ae6` brought the trusted
`Licensed Stata CI` workflow and receipt machinery from `main` into the feature
branch.

- Stata profile: `quick`
- Stata status: `success`
- Stata RC: `0`
- Rust result: rustfmt found committed drift in two new integration tests

This established that Stata receipts and the separate Rust job conclusion must
both be checked for every push.

### Formatting closure

SHAs `34bf2e69a2dfe8bb3840ea6b8b7cb3489355926e` and
`7300ab00117bba38ae0e7c7eac5ce2dd9b847c9c` normalized the two new tests.
The latter exposed the intended implementation gap: every executor partition
was still spawned even though the new invariant required the first partition
to run on the caller thread.

### Caller-thread deterministic executor

Source SHA `649ae908b5c48c49d02375cefd1f5ad219713dfa` now:

- executes the first partition on the calling thread;
- spawns only the remaining partitions;
- joins every spawned partition;
- maps caller and worker panics to the existing typed panic error; and
- preserves partition-order result and error selection.

Exact-SHA evidence:

- profile `quick`, status `success`, Stata RC `0`;
- Rust fmt success;
- strict Clippy success; and
- Rust workspace tests success.

### Error and panic ordering through 32 threads

Source SHA `9fb0551d96834717fb929e32e4718b62f32177aa` adds explicit tests that:

- partitioning and reconstruction remain stable for 1--32 requested threads;
- only the first partition runs on the caller thread;
- an earlier returned error wins over a later partition panic; and
- caller-partition panics are contained and mapped to the typed panic status.

Exact-SHA evidence:

- profile `quick`, status `success`, Stata RC `0`;
- Rust fmt success;
- strict Clippy success; and
- Rust workspace tests success.

## Exact resume point

1. Trace and complete the private Ado solve-V4 invocation and V7 export path.
2. Add static/source-bound tests before widening the public routing matrix.
3. Push each coherent checkpoint and require exact-SHA quick/Rust success.
4. Run a manual comprehensive plugin profile after the V4/V7 boundary is
   complete.
5. Continue with automatic routing, stayer parity, and large-N performance work.

## Evidence rules

- Never claim Rust or Stata was executed locally.
- Never infer Rust success from a successful Stata receipt alone.
- Bind every result to the exact source SHA and profile.
- Preserve estimator, deletion, weighting, nuisance, RNG, solver, residual,
  target, memory, and failure contracts.
- Keep public release disabled until the documented human license/provenance
  review is complete.

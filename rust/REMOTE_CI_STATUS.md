# Remote Rust CI status

Last updated: 2026-08-22

Development branch: `codex/rust-backend-completion`
Baseline source commit: `ce76810348b96d377c1b52907a2fd9fdb76f4909`
Public compiler/test controller: `johannes-schmieder/playground`

## Current checkpoint

The first public six-cell matrix run (`32586393479`) did not test a complete
copy of the baseline Rust tree. The cargo commands ran, but the public snapshot
was missing several large modules, including `engine.rs`, `exact_estimator.rs`,
`generic_jla.rs`, `model_solver.rs`, and the plugin `ffi_engine.rs`; some other
large files were older blobs. Therefore the red result is a snapshot-controller
failure and is not qualification evidence for or against the private source.

A hash-fenced source assembler exists in `playground`, and `engine.rs` has been
staged in line chunks. The exact next action is to finish an assembly manifest
for every private/public blob mismatch, trigger the assembler, verify the
resulting public Rust tree against the private tree, and rerun the required
Ubuntu/macOS/Windows by Rust-1.81/stable matrix.

## Evidence policy

- Rust has not been compiled locally in the ChatGPT execution environment.
- GitHub Actions is the only Rust compiler and test runner used here.
- Stata has not been executed in this environment.
- No Rust checkpoint is called green until `.ci/latest.json` is read back from
  `playground`, names the exact private source SHA, and reports every required
  matrix command successful.

## Resume order after source-mirror repair

1. Establish a green exact-source Rust/C baseline through `playground`.
2. Finish private Ado solve-V4 dispatch and detailed V7 reconciliation.
3. Add static/C/Rust boundary tests for the V4/V7 lifecycle; keep licensed
   Stata execution explicitly pending.
4. Expose the planned routing and receipt surface only after those gates pass.
5. Implement Rust exact parity for the separately labelled stayer hybrid.
6. Profile and optimize large-N memory traffic, batched PCG, CMG application,
   and deterministic parallel kernels with source-bound benchmarks.

Update this file at each diagnostically useful or green remote checkpoint so a
future thread can resume without relying on chat history.

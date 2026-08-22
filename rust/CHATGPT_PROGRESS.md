# ChatGPT Rust backend progress log

Last updated: 2026-08-22
Active branch: `codex/rust-backend-completion`
Starting source commit: `ce76810348b96d377c1b52907a2fd9fdb76f4909`

## Purpose

This file is the live recovery point for continuing the `varcomp_kss` Rust
backend in the ChatGPT execution environment. Rust is not compiled locally in
this environment. GitHub Actions in the public
`johannes-schmieder/playground` repository is the intended compiler and test
runner. Licensed Stata is not available here, so no Stata execution is claimed.

## Current source status

The source state at the starting commit already contains:

- exact worker--firm--control estimation;
- generic Counter-V1 JLA;
- diagonal and CMG PCG routes;
- structural algorithm, engine, preconditioner, and batch planning;
- direct memory admission and execution receipts;
- capability V3, solve V4, execution-plan V1, and detailed receipt V7 on the
  Rust/C side; and
- extensive Rust, C, Python, Mata, and Stata tests from prior development.

The immediate implementation gaps recorded in `IMPLEMENTATION_STATUS.md` are:

1. finish solve V4 dispatch in `varcomp_kss/varcomp_kss_rust.ado`;
2. reconcile and export all V7 execution-plan fields;
3. add private lifecycle/corruption/nonconvergence/UserBreak tests;
4. expose the completed automatic Rust routing matrix while preserving Mata
   for omitted `backend()` and `backend(auto)`;
5. implement Rust exact parity for the separately labelled stayer hybrid; and
6. complete source-bound native qualification and documentation/release gates.

## Public CI controller status

A six-cell matrix was created in `playground` for Ubuntu, macOS, and Windows
under Rust 1.81.0 and stable. It runs formatting checks, strict Clippy, debug
and release tests, and release builds. The first recorded matrix failed before
providing trustworthy source qualification because the public source mirror
was not byte-identical to the private repository.

The mirror audit subsequently established that the transport problem was much
broader than the original three-byte `engine.rs` mismatch. Several large
production modules and both integration-test directories were absent or
truncated, so none of the initial compiler diagnostics are yet source-bound.
Direct private checkout from public Actions remains unverified and the first
explicit receipt recorded failure.

A deterministic source assembler now reconstructs staged line-range chunks and
accepts a file only when both its byte count and Git blob SHA match the private
source. It handles repeated inclusive-range boundary lines and can recover a
small bounded number of blank lines lost at chunk boundaries by testing the
possible placements against the registered private Git hash.

### Verified transfer milestones

Workflow run `32601339467` restored
`crates/vckss-core/src/exact_estimator.rs` byte-for-byte:

- Git blob: `e101b16b9b6d10f2b6e80a73da7d4756df07007f`;
- byte count: 99101;
- selected rule: equal boundary-line deduplication; and
- public checkpoint: `3586cb262b14d17585dda8979e7b3ba9a6bcd6f5`.

A later bounded hash search restored `crates/vckss-core/src/engine.rs`:

- Git blob: `63e4db1028b462096f9be9fa0de6ec26ca5af206`;
- byte count: 132633;
- selected rule: restore three lost boundary blank lines;
- search: 5,450 candidates, exact hash match at boundary indices 28, 30, 31;
  and
- public checkpoint: `efa1d8880a817bf0748811a817a0a34afe9d5f52`.

These are exact source-transfer results, not local compilation or Stata
execution claims. The whole public workspace is not yet exact.

## Remaining mirror restoration

Before treating Cargo diagnostics as source-bound, restore and verify at least:

- `vckss-core/src/generic_jla.rs`;
- `vckss-core/src/model_solver.rs`;
- `vckss-core/src/control_basis.rs`;
- `vckss-core/src/cmg/hierarchy.rs`;
- `vckss-core/src/solver.rs`;
- `vckss-plugin/src/ffi_engine.rs` and any mismatched plugin modules; and
- all files under `vckss-core/tests` and `vckss-plugin/tests`.

Every restored file must match the private Git blob SHA and size before it is
used for qualification.

## Required workflow from here

1. Finish the byte-exact public mirror using the verified assembler.
2. Run the full six-cell matrix and read `.ci/latest.json` back from GitHub.
3. Fix any real formatting, Clippy, compilation, or test failures in small
   commits on this branch, pushing after each meaningful checkpoint.
4. Only after the baseline is green, continue the V4/V7 Ado boundary and
   subsequent feature-parity work.
5. Update this file after every significant checkpoint so a later thread can
   resume without reconstructing chat history.

## Evidence rules

- Never claim Rust was compiled locally.
- Never claim Stata was executed in this environment.
- Bind every CI result to the exact `varcomp_kss` source SHA and Rust tree.
- Distinguish source-transfer failures from compiler/test failures.
- Keep public-release and license/provenance gates closed until the documented
  human reviews are complete.

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

The mismatch was localized to
`rust/crates/vckss-core/src/engine.rs`: the private Git blob is
`63e4db1028b462096f9be9fa0de6ec26ca5af206` with 132633 bytes, while the
reassembled public candidate was three bytes shorter. This was a source
transport/assembly failure, not established evidence of a Rust compiler or
test failure.

Direct checkout of private `varcomp_kss` from public `playground` was then
probed. The first receipt recorded checkout failure. A second unambiguous probe
workflow was committed and triggered, but no V2 receipt was present when this
progress file was created. Do not claim that direct private checkout works
unless `.ci/private-checkout-probe-v2.json` exists and binds the exact source
commit and Rust tree.

## Required workflow from here

1. Establish a trustworthy way to present the exact private Rust source to
   public GitHub Actions. Prefer direct read-only checkout if a suitable
   repository secret is already configured. Otherwise use connector-mediated
   byte-exact transfer with blob SHA and byte-count verification before running
   Cargo.
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

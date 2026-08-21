# Rust backend implementation status

Status snapshot: 2026-08-21 in the uncommitted `main` working tree based on
`fa5fe94`. Local source gates and the current native arm64 Stata route pass.
The earlier `/private/tmp/vckss-public-rust-route-final3.txt` receipt predates
the lifecycle, receipt, ABI, C-transport, packaging, and qualifier repairs
described below and therefore is not evidence for the current source. A fresh
source-bound macOS arm64/Rosetta qualifier is pending independent review. No
current clean-commit, cross-platform, or release qualification is claimed.

## Current implemented source-local route

The public command now has a deliberately narrow Rust route. It is selected
only by explicit
`backend(rust) rng(counter_v1) algorithm(jla) preconditioner(diagonal)` with a
numeric `batch(#)`. Match deletion, joint nuisance, movers,
`engine(auto|compressed)`, `if`/`in`, frequency weights, target weights,
deletion IDs, seed/probes/tolerance/maxiter, and memory admission are wired.
Omitted, Mata, and auto backend requests permanently remain on Mata and the
historical Stata RNG contract. Backend/RNG mismatches and unsupported Rust
structures fail before native preparation; there is no fallback after a Rust
generation is created.

Only three public support flags are enabled: JLA, match deletion, and diagonal
PCG (mask 38). Exact, observation deletion, controls, CMG, scale, and automatic
Rust routing remain false.

The public lifecycle performs common Stata validation and dense-ID mapping,
then native prepare, authoritative retained-mask scatter, receipt
reconciliation, solve, result/RHS export, release, idle-state verification,
and only then `ereturn post`. It posts the scientific matrices plus lossless
native RHS, graph, preparation, memory, topology, solver, RNG, and routing
receipts. The caller result-matrix allocation is charged before solve
admission. Native errors are structurally retrievable as code/status/detail;
stale errors clear at operation entry and UserBreak never reuses an old error.
Primary failures take precedence over release/clear failures.

V1 and V2 ABI layouts are frozen. Additive V3 preparation/result receipts
carry target mass, Counter-V1, full-fit zero-RHS, topology halves, RHS row
count, and caller result-copy accounting. The additive RHS V1 export has fixed
full-fit, leverage-probe, target-worker, target-firm ordering.

## Executed gates

- Rust 1.81.0: root workspace 115/115, formatting, and strict Clippy pass.
- Rust stable 1.97.1: root workspace 115/115, formatting, and strict Clippy
  pass.
- Standalone Stata-boundary crate: 8/8 tests, strict Clippy, and release builds under both
  toolchains with authenticated public SPI 3.0 inputs.
- Python/packaging/benchmark harnesses: 375/375 pass; deterministic CMG
  assembly passes; the integrated local gate, including quick/full Stata,
  CMG, separations, and canonical portable `net install`, passes.
- The authenticated C interrupt and injected error-transport harnesses pass,
  and the frozen-header ABI compatibility fixture compiles.
- Licensed Stata 18 on native macOS arm64 passes the plugin lifecycle, bounded
  Mata diagnostic, shared Counter-V1 atoms, strict public route, routing matrix,
  Stata-side fault/corrupt-receipt cleanup, canonical unavailable install, and
  isolated local artifact install.
- The repaired qualifier is designed to test the exact signed thin arm64 and
  x86_64 candidates that it stages, then test the universal candidate
  separately, and to bind all results to the source hash. Its fresh
  arm64/Rosetta execution is pending; Rosetta will be compatibility evidence,
  not native Intel qualification.

## Packaging boundary

The tracked package manifest ships the portable `varcomp_kss_rust.ado` helper
and internal public-call dispatcher but no native binaries. A canonical clean
install therefore returns typed `RUST_BACKEND_UNAVAILABLE` for an explicit
Rust request. The qualifier generates a temporary local manifest that adds its
verified macOS artifacts, proves isolated installation, and only then stages
ignored local candidates. This keeps macOS build products out of the tracked
cross-platform/SCC package closure.

## Remaining exclusions

This checkpoint does not establish public release, Windows or Linux Stata,
native Intel hardware, target-scale performance/RSS, exact/observation/control/
CMG parity, broad differential coverage, Miri/sanitizer/fuzz evidence, SBOM or
release packet, or final human mathematical/license/provenance approval. Point
estimates and numerical diagnostics only are exposed; no `e(V)` or
econometric standard errors are provided.

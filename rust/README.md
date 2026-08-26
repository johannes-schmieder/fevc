# Rust backend

This directory contains the optional native backend for `vckss`. It is a
package-owned implementation, not a separate public command. Omitted
`backend()` and `backend(auto)` prefer Rust after a complete preflight
capability check, with Mata fallback allowed only before preparation and RNG.

Rust is explicit opt-in and must cross the compositional Stata/plugin boundary:
request capability, prepare, solve, result, and release. The native result and
its execution-plan, numerical, memory, counter, and cleanup receipts must all
reconcile before Stata posts estimates.

Backend development compares corrected statistical results rather than exact
floating-point paths. The active promotion thresholds are registered in
[`../vckss/docs/development_acceptance_v1.json`](../vckss/docs/development_acceptance_v1.json);
bitwise identity and legacy fixed roundoff gates are diagnostic when hard
correctness and corrected-result equivalence pass.

## Source map

- `crates/vckss-core/`: estimator, graph, exact/JLA, solver, CMG adapter,
  planning, memory, wall, counter, and receipt logic.
- `crates/vckss-plugin/`: generation-safe native lifecycle and versioned ABI.
- `stata_backend/`: Stata C-plugin shim, build, packaging, and macOS qualifier.
- `RNG_CONTRACT.md`: Counter-V1 contract.
- `SOURCE_PROVENANCE.md`: native-source provenance.
- `TEST_PLAN.md`: active native validation gates.
- `progress/`: dated, source-bound checkpoints. These are evidence, not current
  instructions.

The active cross-language milestone is maintained in
[`../vckss/PLAN.md`](../vckss/PLAN.md). Do not create a second
current plan in this directory.

## Implemented result families

The native core implements three result families:

1. **Exact** — deterministic dense exact calculation for eligible retained
   dimensions. Estimator RNG, probe batches, and iterative preconditioning are
   not applicable.
2. **Compressed JLA** — the registered no-control match specialization with
   separate coefficient-cell, deletion-unit, target-stratum, and Counter-V1
   semantics.
3. **Generic JLA** — the general worker--firm/control estimator with planned
   diagonal or CMG PCG, independent batch widths, memory admission, and
   complete per-RHS receipts.

Algorithm, engine, route, fallback, batch, memory, and wall planning are frozen
before estimator RNG. A later rank, setup, resource, convergence, or numerical
failure cannot choose a different scientific estimator. Explicit CMG fails
closed; automatic setup fallback to diagonal is allowed only before RNG and is
recorded.

## Public boundary

The package exposes explicit Rust exact and planned compressed/generic JLA.
The V3/V4/V7 public `algorithm(auto)` path is qualified when its frozen plan
selects exact, including the exact-family poster and zero-RNG reconciliation.

Effective default admission, `algorithm(auto)` selecting JLA, and semantic
`probeorder()` tie breaking are qualified on macOS arm64 and Rosetta. Exact
`stayers(both)` is also qualified there through the versioned native
augmentation lifecycle, separate stayer correction, exact-family poster,
differential oracles, zero-RNG contract, and release/idle-registry checks. The
platform and bounded safety gates are qualified: macOS arm64/Rosetta, SCC
Linux x86-64, Miri, C-shim ASan/UBSan, malformed-ABI fuzzing, RustSec audits,
license inventory, and CycloneDX SBOMs have source-bound evidence under
`qualification/evidence/`. Representative performance, the benchmark report,
and the frozen alpha packet remain open.

M5 instrumentation uses the additive
`VckssEnginePerformanceReceiptV1`, separate from the frozen V7 numerical and
pre-RNG plan receipt. It reports native ingress, canonicalization, graph,
compression, plan, stayer-augmentation, solve, and summed wall-clock phases.
These diagnostics never enter admission, routing, numerical work, RNG
accounting, or result reconciliation decisions.

The active performance experiment is the private `CMG_FULL_SPIKE_V1` direct
hybrid-Laplacian batch route under `crates/vckss-core/src/full_cmg_spike.rs`.
It links exact standalone CMG source `dbefbc5`, requires explicit private
environment consent, uses one `ParallelPcgSolver` and one bounded Rayon pool,
places transformed firm RHS values on firm vertices with zeros on auxiliary
worker vertices, extracts and recenters firm solutions, recovers workers, and
applies the independent complete original-system residual gate. It is not a
public capability or ABI. Its first registered local hard-case timing improves
the prior backend by 12.35%. Its approximately `2.22e-12` covariance difference
failed the legacy pathwise gate but passes the active development-equivalence
policy and is not a scientific blocker. Accepted SCC job `7306628` measured
223.232 seconds for the candidate versus 329.260 seconds for the matched
baseline and 171.733 seconds for maintained MATLAB on the same node. C is
29.99% slower than MATLAB; its 153.432-second direct solve is the dominant
phase. That original route was rejected for performance, not numerical parity.

The owner-authorized renewed direct-route wave preserves that evidence and
adds ordered parallel Counter-V1 generation, direct leverage RHSs, independent
target preparation, parallel moment accumulation, and bounded parallel
recovery and complete-residual certification. Clean source `598a08d` completes
the same macOS headline in 81.145 seconds, or 0.473 times the registered MATLAB
command, with bit-identical corrected targets and a `6.97e-6` maximum complete
residual. This first 2.12x single run is a development checkpoint only.
Alternating warm medians and the checksum-bound fixed CZ18 SCC case remain
mandatory before vendoring, hardening, or exposing the route. Further
optimization of the simplified embedded hierarchy remains stopped.

On SCC, synthetic job `7311964` reaches 156.455 seconds versus MATLAB's
260.928 seconds, while accepted direct-versus-fused job `7312041` reaches
151.351 seconds for direct and 239.610 seconds for fused f64. Direct is kept;
fused f64 is disabled. The direct result is 1.669x faster than MATLAB but
25.079 seconds short of the 2x threshold, with bit-identical direct/fused
targets and a `6.971259e-6` maximum complete residual. The official full-CMG
repeated solve is the dominant remaining cost. Fixed-CZ18 P20 completes its
61 numerical solves at a `7.993e-6` maximum residual, but its post-engine
Stata/native result boundary is still being diagnosed; this is not a P20 or
P200 qualification result.

Result families must continue to use their matching posters and all receipts
remain mandatory.

## Development gates

From the repository root, use the pinned toolchain and locked manifests:

```bash
cargo fmt --manifest-path rust/Cargo.toml --all -- --check
cargo clippy --manifest-path rust/Cargo.toml --workspace --all-targets \
  --locked -- -D warnings
cargo test --manifest-path rust/Cargo.toml --workspace --all-targets --locked
cargo test --manifest-path rust/stata_backend/Cargo.toml \
  --all-targets --locked
```

The ordinary push lane also runs licensed Stata quick tests. Native route
qualification requires the source-local plugin profile and its exact-SHA
receipt; a green quick receipt alone is insufficient. See
[`TEST_PLAN.md`](TEST_PLAN.md),
[`stata_backend/README.md`](stata_backend/README.md), and
[`../STATA_CI_RUNNER.md`](../STATA_CI_RUNNER.md).

## Release boundary

The tracked package ships portable source and Ado boundary helpers, not native
binaries. macOS plugin artifacts are local qualification products. The alpha
packet will add qualified macOS arm64/Rosetta and SCC Linux x86-64 artifacts;
Windows, native Intel hardware, public distribution, and the final public
mathematical/license/provenance sign-off remain outside the alpha claim.

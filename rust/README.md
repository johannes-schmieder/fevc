# Rust backend

This directory contains the optional native backend for `vckss`. It is a
package-owned implementation, not a separate public command. Omitted
`backend()` and `backend(auto)` prefer Rust after a complete preflight
capability check, with Mata fallback allowed only before preparation and RNG.

Rust is explicit opt-in and must cross the compositional Stata/plugin boundary:
request capability, prepare, solve, result, and release. The native result and
its execution-plan, numerical, memory, counter, and cleanup receipts must all
reconcile before Stata posts estimates.

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
remaining alpha gaps are Linux/SCC qualification, fuzz/dependency/SBOM safety
evidence, and representative performance. A source-bound bounded Miri and
C-shim ASan/UBSan receipt is registered under `qualification/evidence/`.
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

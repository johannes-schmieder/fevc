# Rust backend

This directory contains the optional native backend for `vckss`. It is a
package-owned implementation, not a separate public command. The established
Mata backend remains the permanent default: omitted `backend()`,
`backend(mata)`, and `backend(auto)` all select Mata.

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

The package currently exposes explicit Rust exact and explicit/planned Rust
JLA subsets. The native V3/V4/V7 planner can select exact for
`algorithm(auto)`, and the package ships the exact-V7 reconciler and poster.
The public command does **not** yet admit or dispatch `algorithm(auto)` to that
exact family. Its option predicate still admits only explicit exact or JLA
subsets, and its planned family switch handles compressed/generic results. The
exact-selected public branch is the active unfinished milestone.

The current dispatcher already posts compressed and generic result families.
An exact-selected plan still reaches the unknown-family guard until the
matching exact context and poster are integrated. Do not work around this by
routing exact output through a JLA poster or weakening receipt reconciliation.

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
binaries. macOS plugin artifacts are local qualification products. Native
Intel, Linux, and Windows Stata qualification, representative scale evidence,
release-security evidence, and final human mathematical and license/provenance
review remain separate gates. Public distribution is disabled.

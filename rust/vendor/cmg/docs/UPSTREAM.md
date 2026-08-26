# Upstream provenance and implementation coverage

## Pinned source

This project is an independent Rust port of the official CMG implementation:

- Ioannis Koutis and Gary Miller, `ikoutis/cmg-solver`
- pinned commit `19752fc102f8cae8e34f66457bfaccb1aaa60375`
- upstream source path `matlab/cmg`
- upstream license: GNU GPL version 3

The pinned commit includes the 2026 correction to the C forest-component workspace initialization.

## Source-of-truth order

1. Pinned official MATLAB implementation and C MEX kernels.
2. Published CMG algorithm and its mathematical invariants.
3. Independent dense algebraic oracles and differential tests used for verification.

## Production-path coverage

| Upstream behavior | Rust location | Status |
|---|---|---|
| SDDM/Laplacian validation and augmentation | `src/sddm.rs` | implemented |
| heaviest incident-edge forest | `src/forest.rs` | implemented |
| forest splitting / conductance logic | `src/forest.rs` | implemented |
| low-effective-degree correction | `src/forest.rs` | implemented |
| forest components / aggregate labels | `src/forest.rs`, `src/coarsen.rs` | implemented |
| Galerkin coarse contraction | `src/coarsen.rs` | implemented |
| hierarchy stopping logic | `src/hierarchy.rs` | implemented |
| recursive repeat-count logic | `src/hierarchy.rs` | implemented |
| grounded terminal LDL solve | `src/ldl.rs` | implemented |
| stationary recursive CMG cycle | `src/preconditioner.rs` | implemented |
| SDDM preconditioner wrapper | `src/sddm.rs`, `src/preconditioner.rs` | implemented |
| PCG outer solver | `src/pcg.rs` | implemented natively |
| sparse/vector utility kernels | `src/graph.rs`, `src/workspace.rs` | implemented |
| optional multicore execution | `src/execution.rs`, `src/parallel_solver.rs` | Rust extension |

The benchmark-only `benchmarks/c-kernel/` crate compiles isolated pinned C kernels and checks numerical agreement with Rust.

## Behavioral constants retained from upstream

- direct terminal below 700 vertices;
- damped Jacobi inverse diagonal `1 / (2 * diag(A))`;
- low-effective-degree threshold `1/8`;
- hierarchy cumulative-nonzero guard `5 * nnz(A_initial)`;
- stagnation when the coarse graph has at least `n - 1` vertices;
- repeat count `max(floor(nnz(A_fine) / nnz(A_coarse) - 1), 1)`;
- one top-level stationary cycle per preconditioner application;
- component grounding for direct Laplacian solves;
- one extra augmentation vertex for strictly dominant SDDM systems.

The Rust API allows relevant constants to be overridden for testing while keeping the upstream values as defaults.

## Deliberate Rust extensions

The Rust package supports several behaviors beyond the original MATLAB-facing interface while preserving the stationary CMG algorithm:

- all graph sizes rather than refusing small inputs;
- disconnected Laplacians with explicit component compatibility checks;
- original-system residual and backward-error certification;
- reusable immutable hierarchies and caller-owned workspaces;
- repeated and batched right-hand sides;
- deterministic compact storage and memory accounting;
- optional package-owned parallel execution and automatic routing.

## Attribution

CMG was developed by Ioannis Koutis and Gary Miller. This repository is an independent Rust port and is not an official upstream release.

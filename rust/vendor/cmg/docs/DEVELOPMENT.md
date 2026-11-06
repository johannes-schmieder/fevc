# Development guide

## Repository policy

The current tree is the maintained product; Git history and Actions artifacts are the experiment archive. Do not leave completed one-shot optimization scripts, self-removing workflows, recovery launchers, or per-experiment JSON records on `main`.

Keep durable files only when they serve production code, correctness/regression tests, runnable examples, reusable benchmark/profiling harnesses, durable CI/benchmark workflows, maintained documentation, or a small number of current machine-readable status records.

## Module map

| Module | Responsibility |
|---|---|
| `graph.rs` | canonical weighted Laplacian storage, matvec, graph metadata |
| `components.rs` | connected components and quotient-space operations |
| `forest.rs` | heavy-edge forest, splitting, correction, component labels |
| `coarsen.rs` | aggregate mappings and Galerkin contraction |
| `hierarchy.rs` | recursive hierarchy construction and stopping rules |
| `ldl.rs` | grounded terminal factorization and solve |
| `build.rs` | `BuildOptions` and the construction `BuildContext` (team, cancellation, budget) |
| `cancel.rs` | cancellation meters, bounded chunks and parallel-lane polling |
| `preconditioner.rs` | the one construction pipeline, `ApplyContext` and the row-operator plan |
| `preconditioner/cycle.rs` | the one stationary cycle recursion over scalar or interleaved lanes |
| `preconditioner/batch.rs` | interleaved multi-RHS workspace and entry point |
| `pcg.rs` | the one certified PCG recurrence, `SolveContext` and its entry points |
| `prepared.rs` | fixed-topology validation and changing-weight numeric assembly |
| `sddm.rs` / `sddm_solver.rs` | SDDM validation, augmentation, solve wrapper |
| `workspace.rs` | reusable numerical scratch storage |
| `execution.rs` | borrowed `Team` handle and optional standalone owned executor |
| `execution_stage.rs` | fixed-chunk stages dispatched on a borrowed team |
| `csr.rs` | deterministic row-oriented operators for parallel kernels |
| `parallel_solver.rs` | automatic memory-aware parallel routing |
| `memory.rs` | checked pre-build estimates and retained-memory reports |
| `pcg_profile.rs` | profiling recorder for the production PCG recurrence |

The public API is re-exported from `src/lib.rs`; implementation details should remain private unless a stable user-facing need exists.

### Executor lifetime

`ParallelExecutor` owns its worker join handles. Retirement drops the Rayon pool
first (signalling termination), then joins every successfully started worker,
including during partial construction and caller unwinding. Joining includes
thread-local destructors; an exit callback or a sampled thread count would not
establish that boundary. Join errors are consumed to avoid a second panic.

The pool and its `install` method must remain private. All dispatched work is
internal and scoped, borrows the executor, and completes before its caller can
drop it. Public methods accept numerical data, not worker callbacks. Exposing
asynchronous tasks or callbacks that can own the final executor would invalidate
this invariant and could deadlock a worker joining itself or a dependent peer.
The lifetime tests exercise actual pool workers and thread-local completion on
normal retirement, partial startup, changing pool sizes and unwinding.

Rayon-core 1.13.0's registry construction uses a termination guard on failure;
`ThreadPool::drop` only signals termination. Neither that implementation nor
joining establishes a byte bound on Rayon/crossbeam queues, allocator caches,
thread stacks or arbitrary third-party TLS. Retirement is a lifetime boundary,
not a whole-process memory guarantee.

## Correctness invariants

Performance work must preserve deterministic canonical graph construction, exact Laplacian symmetry, component-wise compatibility checks, pinned stationary CMG hierarchy/repeat semantics, deterministic grounding/normalization, final residual/backward-error verification against the original system, and hierarchy metadata unless the algorithm is deliberately changed and separately reviewed.

Never accept a benchmark improvement by relaxing tolerances, changing the RHS or matrix, skipping certification, or silently changing the stationary algorithm.

Prepared-topology frames carry two distinct identities. Every numeric assembly
gets a fresh numeric lineage, while clones of one
`PreparedLaplacianTopology` share an exact topology/component lineage. Legacy
preconditioner matching remains numeric. The explicit retained-preconditioner
PCG entry points require the shared prepared lineage and must use the current
frame for every matrix-vector product, residual replacement, operator bound,
and terminal certificate.

## Build and test

```bash
cargo fmt --all -- --check
cargo fmt --manifest-path benchmarks/Cargo.toml --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo clippy --manifest-path benchmarks/Cargo.toml --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items --all-features
cargo test --all-targets
cargo test --all-targets --release
cargo test --all-targets --all-features
cargo test --all-targets --all-features --release
cargo build --release --all-features
cargo build --release --manifest-path benchmarks/Cargo.toml --all-targets
```

`rust.yml` runs the cross-platform qualification matrix on Ubuntu, macOS, and
Windows. GitHub checks are the authoritative current status; workflow outputs
are uploaded as artifacts and never committed back to `main`.

## Performance work

Use the durable binaries in `benchmarks/` rather than committing a long-lived executable for every hypothesis. Temporary one-shot gates should be deleted once a decision is recorded; completed experiment artifacts should not remain on `main`.

Durable benchmark workflows:

- `performance.yml` — frozen-baseline serial performance plus certified routing matrix;
- `parallel-performance.yml` — hosted-runner parallel scaling;
- `c-kernel.yml` — pinned C/Rust differential comparison;
- `manual-32-thread-qualification.yml` — controlled large-machine scaling.

Automated benchmark workflows publish step summaries and immutable Actions
artifacts. Promoting a result into `.ci/performance/` is an explicit reviewed
documentation change, not an automated writeback commit.

See `docs/PERFORMANCE.md` for interpretation.

## Documentation maintenance

Keep the root directory small. User-facing orientation belongs in `README.md`; maintained technical notes belong in `docs/`. `CHANGELOG.md` is the one maintained user-facing chronological release record; avoid separate status or release logs because commit history and GitHub Releases already record that chronology.

Follow `RELEASING.md` for version changes, tags, GitHub Releases, and future SSC publication. Run `python3 scripts/check_release_metadata.py` whenever release metadata changes.

## Downstream source snapshots

Consumers pin one reviewed full commit SHA reachable from `main`. Offline
packages may vendor unmodified source from that same commit. Refresh with
`python3 scripts/vendor.py refresh --source <CMG-checkout> --rev <SHA>
--destination <vendor/cmg> --record <outside-payload/CMG-SOURCE.json>`.
Run the same command with `check` to compare against upstream, or omit
`--source` and `--rev` for a packaging-time byte check against the recorded
inventory. Consumer adapters and provenance belong outside the CMG payload.
Do not edit vendored numerical code or Cargo metadata.

When a user-visible implementation constant or routing threshold changes, update its code comment/tests and the maintained documentation in the same checkpoint.

## Scope boundary

The crate targets stationary CMG and certified PCG. C ABI/Stata integration, flexible Krylov methods, GPU support, and NUMA-specific tuning are explicit later layers rather than cleanup or micro-optimization work.

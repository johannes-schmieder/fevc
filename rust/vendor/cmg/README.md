# CMG in Rust

A deterministic Rust implementation of stationary Combinatorial Multigrid (CMG) for weighted graph Laplacians and symmetric diagonally dominant M-matrices (SDDM), with certified preconditioned conjugate-gradient solves.

The implementation follows the official `ikoutis/cmg-solver` source pinned at commit `19752fc102f8cae8e34f66457bfaccb1aaa60375`. See [`docs/UPSTREAM.md`](docs/UPSTREAM.md) for provenance and routine coverage.

## Status

The stationary CMG path is implemented and tested on Linux, macOS, and Windows. Tests cover exact small systems, disconnected graphs, weighted adversarial cases, deterministic hierarchy construction, SDDM augmentation, terminal factorization, repeated right-hand sides, and original-system residual certification.

The default crate has no parallel runtime dependency. Optional multicore support uses a package-owned Rayon pool behind the `parallel` feature. Functional thread-pool coverage extends through 32 threads; controlled 8/16/32-core performance qualification still requires suitable hardware.

## Quick start

For the weighted path

```text
0 --1-- 1 --1-- 2
```

the Laplacian is

```text
L = [ 1 -1  0 ]
    [-1  2 -1 ]
    [ 0 -1  1 ]
```

and `b = [1, 0, -1]` has zero-mean solution `x = [1, 0, -1]`.

```rust
use cmg::{CmgOptions, CmgPreconditioner, Laplacian, PcgOptions, solve_pcg};

fn main() -> Result<(), cmg::CmgError> {
    let graph = Laplacian::from_edges(3, [(0, 1, 1.0), (1, 2, 1.0)])?;
    let rhs = [1.0, 0.0, -1.0];

    let cmg = CmgPreconditioner::build(&graph, CmgOptions::default())?;
    let result = solve_pcg(&graph, &cmg, &rhs, PcgOptions::default())?;

    println!("x = {:?}", result.solution());
    println!("iterations = {}", result.iterations());
    println!("backward error = {:.3e}", result.backward_error());
    Ok(())
}
```

Run it with:

```bash
cargo run --example laplacian_pcg
```

A graph Laplacian is singular. Each connected component of the submitted right-hand side must sum to zero within the configured compatibility tolerance. Solutions use a deterministic component-wise zero-mean normalization.

## Parallel and repeated-RHS solves

Enable multicore support with:

```toml
cmg = { git = "https://github.com/johannes-schmieder/CMG", features = ["parallel"] }
```

For application code, `ParallelPcgSolver` is the preferred high-level API. It owns the reusable hierarchy, selectively routed parallel plan, package-owned thread pool, and reusable workspaces. It chooses among serial PCG, planned within-solve PCG, and memory-bounded concurrency across independent right-hand sides.

The default single-RHS routing threshold is **350,000 canonical retained edges**. This is a measured performance heuristic, not a mathematical CMG constant, and can be overridden through `ParallelPcgPolicy`.

```rust,ignore
use cmg::{CmgOptions, ParallelOptions, ParallelPcgSolver, PcgOptions};

let solver = ParallelPcgSolver::build(
    &graph,
    CmgOptions::default(),
    ParallelOptions {
        threads: 16,
        workspace_memory_budget_bytes: Some(8 * 1024 * 1024 * 1024),
        ..ParallelOptions::default()
    },
)?;

let mut workspace = solver.workspace();
let batch = solver.solve_batch_with_workspace(
    &right_hand_sides,
    PcgOptions::default(),
    &mut workspace,
)?;
println!("execution = {:?}", batch.report().execution());
```

Reuse the same solver and workspaces whenever the graph and weights are unchanged. For many RHSs, across-RHS parallelism is generally the lowest-overhead strategy when memory permits.

## Performance

A controlled SCC study on Intel Gold 6242 nodes compares this Rust package with the official MATLAB/C MEX CMG package on five connected graph families from 100,000 to 1,000,000 vertices. At 32 application CPUs, the geometric-mean Rust/MATLAB ratios across the 15 main cases are 0.265 for preconditioner setup, 0.783 for reused-preconditioner PCG, 0.666 for setup plus solve, and 0.150 for process peak RSS. All main cases converged, but complete single-RHS workflow scaling from 1 to 32 CPUs was modest for both packages. In the 16-RHS supplement, Rust's across-RHS executor instead achieved 7.76x geometric-mean scaling and 0.122x MATLAB's per-RHS time at 32 CPUs.

A frozen cumulative checkpoint versus the early Rust implementation separately reports roughly 20% faster graph construction, 28% faster hierarchy construction, 4.4x faster stationary CMG application, 2.7x faster solve-per-RHS, and substantial memory reductions. The hosted four-CPU routing record reaches about 2.2x planned-versus-serial speedup on its largest dense worker-firm case.

These are project benchmarks, not universal hardware guarantees. See [`docs/PERFORMANCE.md`](docs/PERFORMANCE.md) for exact interpretation, accuracy diagnostics, maintained benchmark records, and limitations.

## Repository layout

```text
src/                 numerical library and optional parallel implementation
examples/            small runnable API examples
tests/               correctness, determinism, adversarial, and parity tests
benchmarks/           durable benchmark/profiling harnesses
benchmarks/c-kernel/  isolated comparison with pinned upstream C kernels
docs/                 maintained design, performance, and provenance notes
.github/workflows/    durable CI and benchmark workflows
.ci/                  latest machine-readable CI/performance records only
```

Completed one-shot optimization experiments are intentionally kept in Git history rather than the current source tree.

## Build and test

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
cargo test --all-targets --release
cargo test --all-targets --all-features
cargo test --all-targets --all-features --release
cargo build --release --all-features
cargo build --release --manifest-path benchmarks/Cargo.toml --all-targets
```

## Documentation

- [`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md) — module map, testing, repository policy, and performance-change discipline.
- [`docs/PERFORMANCE.md`](docs/PERFORMANCE.md) — current evidence, routing guidance, bottlenecks, and large-machine qualification.
- [`docs/UPSTREAM.md`](docs/UPSTREAM.md) — pinned upstream source, behavioral constants, implementation mapping, and attribution.

## Scope

The crate implements stationary CMG, certified PCG, repeated-RHS operation, optional deterministic multicore kernels, and SDDM wrapping. K-cycles, flexible CG, GPU kernels, NUMA-specific placement, a C ABI, and Stata integration are future layers.

## License

GNU GPL version 3 only. See [`LICENSE`](LICENSE) and [`docs/UPSTREAM.md`](docs/UPSTREAM.md).

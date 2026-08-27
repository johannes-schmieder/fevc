# CMG: Combinatorial Multigrid in Rust

[![Rust CI](https://github.com/johannes-schmieder/CMG/actions/workflows/rust.yml/badge.svg)](https://github.com/johannes-schmieder/CMG/actions/workflows/rust.yml)
[![License: GPL v3](https://img.shields.io/badge/License-GPL_v3-blue.svg)](LICENSE)

CMG is a deterministic Rust solver for large weighted graph Laplacians and
symmetric diagonally dominant M-matrices (SDDM). It combines a reusable
Combinatorial Multigrid preconditioner with certified preconditioned conjugate
gradient (PCG) solves, plus optional multicore and repeated-right-hand-side
execution.

> **Origin and attribution.** Combinatorial Multigrid was introduced by Ioannis
> Koutis, Gary L. Miller, and David Tolliver in
> [*Combinatorial preconditioners and multilevel solvers for problems in
> computer vision and image processing*](https://doi.org/10.1016/j.cviu.2011.05.013),
> *Computer Vision and Image Understanding* 115(12), 2011. This repository is
> an independent Rust port of Koutis and Miller's
> [official MATLAB/C implementation](https://github.com/ikoutis/cmg-solver),
> developed against
> [upstream commit `19752fc`](https://github.com/ikoutis/cmg-solver/tree/19752fc102f8cae8e34f66457bfaccb1aaa60375).
> It is not an official upstream release.

## What CMG does

For an undirected graph with nonnegative edge weights, the weighted graph
Laplacian has entries

$$
L_{ii}=\sum_j w_{ij}, \qquad L_{ij}=-w_{ij}\quad(i\ne j).
$$

CMG solves sparse systems of the form $Lx=b$. It first aggregates vertices to
construct a hierarchy of progressively smaller graphs. One stationary CMG
cycle then acts as an inexpensive approximate inverse of $L$, and PCG uses that
preconditioner to reach the requested accuracy. The result includes the
solution, iteration count, convergence diagnostics, and a backward-error check
against the original system.

Graph Laplacians are singular: adding a constant to a solution within a
connected component does not change $Lx$. The right-hand side must therefore
sum to zero within each component, up to the configured compatibility
tolerance. CMG returns a deterministic solution normalized to zero mean within
each component. Disconnected graphs are supported explicitly.

The crate also solves SDDM systems: symmetric diagonally dominant matrices with
nonpositive off-diagonal entries. It is not a general solver for nonsymmetric,
indefinite, or arbitrary positive-off-diagonal matrices.

## Why economists may care

Large fixed-effects estimators often hide a graph problem. In the
[Abowd–Kramarz–Margolis (AKM) model](https://doi.org/10.1111/1468-0262.00020),

$$
y_{it}=\alpha_i+\psi_{J(i,t)}+x_{it}'\beta+\varepsilon_{it},
$$

workers and firms form the two vertex sets of a weighted bipartite graph, and
employment relationships form its edges. After assembling the model,
partialling out controls, choosing signs, and imposing the required
normalization, the sparse normal equations for worker and firm effects can be
written as graph-Laplacian or closely related SDDM solves. Connectivity of the
mobility graph governs which effects can be compared and how difficult the
linear system is to solve.

Repeated linear solves also arise in randomized projections, leverage
calculations, and leave-out variance-component corrections such as
[Kline, Saggio, and Sølvsten (2020)](https://doi.org/10.3982/ECTA16410).
Their public MATLAB
[LeaveOutTwoWay](https://github.com/rsaggio87/LeaveOutTwoWay) implementation
includes the original CMG solver. This Rust crate is designed so a hierarchy,
parallel plan, and bounded workspace pool can be reused across many such
right-hand sides.

**CMG is the numerical solver underneath these workflows.** It does not ingest
worker–firm panels, assemble or estimate an AKM model, find leave-out connected
sets, or calculate bias-corrected variance components by itself.

## Highlights

- Faithful stationary CMG behavior developed against a pinned upstream source.
- Deterministic graph construction, hierarchy construction, reductions, and
  component-wise solution normalization.
- Final residual and backward-error certification against the submitted
  Laplacian or SDDM system.
- Reusable immutable preconditioners and caller-owned workspaces for repeated
  solves.
- Optional package-owned parallel execution, automatic routing, and
  memory-bounded concurrency across independent right-hand sides.
- Linux, macOS, and Windows coverage, no unsafe Rust, and no parallel-runtime
  dependency in the default feature set.

## Installation

CMG is currently a development crate at version `0.1.0`. It has not yet been
published on crates.io and does not have a final GitHub release. Install the
current development version directly from `main`:

```toml
[dependencies]
cmg = { git = "https://github.com/johannes-schmieder/CMG", branch = "main" }
```

Enable the optional parallel implementation with:

```toml
[dependencies]
cmg = { git = "https://github.com/johannes-schmieder/CMG", branch = "main", features = ["parallel"] }
```

The crate requires Rust 1.85 or newer. See [`RELEASING.md`](RELEASING.md) for
the release policy and [`CHANGELOG.md`](CHANGELOG.md) for user-facing changes.

## Serial quick start

The following example solves a three-vertex weighted path:

```text
0 --1-- 1 --1-- 2
```

Its Laplacian and a compatible right-hand side are

```text
L = [ 1 -1  0 ]       b = [ 1 ]
    [-1  2 -1 ]           [ 0 ]
    [ 0 -1  1 ]           [-1 ]
```

with zero-mean solution `x = [1, 0, -1]`.

```rust
use cmg::{CmgOptions, CmgPreconditioner, Laplacian, PcgOptions, solve_pcg};

fn main() -> Result<(), cmg::CmgError> {
    let graph = Laplacian::from_edges(3, [(0, 1, 1.0), (1, 2, 1.0)])?;
    let rhs = [1.0, 0.0, -1.0];

    // Build once, then reuse this preconditioner for other right-hand sides.
    let cmg = CmgPreconditioner::build(&graph, CmgOptions::default())?;
    let result = solve_pcg(&graph, &cmg, &rhs, PcgOptions::default())?;

    println!("x = {:?}", result.solution());
    println!("iterations = {}", result.iterations());
    println!("backward error = {:.3e}", result.backward_error());
    Ok(())
}
```

Run the repository example with:

```bash
cargo run --example laplacian_pcg
```

## Reusable parallel and batch solves

For application code, `ParallelPcgSolver` is the preferred high-level parallel
API. It retains the graph, CMG hierarchy, selective parallel plan, package-owned
Rayon pool, and reusable workspaces. Depending on the workload, it chooses
serial PCG, parallelizes one sufficiently large solve, or solves independent
right-hand sides concurrently within the configured memory budget.

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
println!(
    "retained bytes = {}",
    solver.memory_report(&workspace).total_retained_bytes()
);
```

Reuse the solver and workspace while the graph and edge weights remain
unchanged. For many right-hand sides, coarse-grained across-RHS parallelism
usually has less synchronization overhead than parallelizing a single solve.
The default single-RHS routing threshold is 350,000 canonical retained edges;
it is a measured heuristic, not an algorithmic constant, and can be changed
through `ParallelPcgPolicy`.

Call `CmgMemoryEstimate::conservative` before construction when a host process
has a hard allocation budget. The estimate uses checked arithmetic and includes
raw submitted edges, the complete bounded hierarchy, an optional parallel plan,
and the requested reusable workspace pool. `memory_report` provides exact
principal retained bytes after construction without changing solver behavior.

## Validated benchmarks

A controlled study on Intel Xeon Gold 6242 nodes compared this crate with the
official MATLAB solver and its C MEX kernels on five deterministic connected
graph families, three sizes from 100,000 to 1,000,000 vertices, and matched
application CPU counts.

Across the 15 cases at 32 application CPUs, the geometric-mean results were:

| Measurement | Rust relative to MATLAB | Plain-language interpretation |
|---|---:|---|
| CMG setup | `0.265x` | Rust setup was about 3.8 times faster |
| Reused-preconditioner PCG | `0.783x` | Rust PCG was about 1.3 times faster |
| Setup plus one solve | `0.666x` | Rust took about two-thirds of MATLAB's time |
| Process peak RSS | `0.150x` | Rust used about one-sixth of MATLAB's memory |

All primary cases converged under each package's native stopping rule, with
independently recomputed accuracy diagnostics retained. Complete single-RHS
scaling was modest and sometimes nonmonotone: allocating 32 CPUs should not be
interpreted as a proportional speedup. In the separate 16-RHS supplement,
Rust's across-RHS executor achieved 7.76-fold geometric-mean scaling from 1 to
32 CPUs and `0.122x` MATLAB's normalized per-RHS time at 32 CPUs.

These are measurements from one controlled environment, not universal hardware
guarantees. See the [performance guide](docs/PERFORMANCE.md), the
[technical benchmark report](output/pdf/benchmarks.pdf), and the
[machine-readable summary](.ci/performance/scc-latest.json) for methodology,
accuracy conventions, detailed results, and limitations.

## Project status and scope

The Rust solver is implemented and tested, but the project is still preparing
its first final tagged release. It is not available on crates.io or SSC. A Stata
interface, C ABI, K-cycles, flexible CG, GPU kernels, and NUMA-specific tuning
are not currently included.

Tests cover exact small systems, disconnected graphs, difficult weighted
cases, deterministic hierarchy construction, SDDM augmentation, terminal
factorization, repeated right-hand sides, and original-system residual checks.
The optional thread pool is exercised through 32 threads; practical performance
depends strongly on graph structure, problem size, reuse, and memory topology.

## Development and documentation

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
cargo test --all-targets --all-features --release
cargo build --release --all-features
cargo build --release --manifest-path benchmarks/Cargo.toml --all-targets
```

- [`docs/UPSTREAM.md`](docs/UPSTREAM.md) — algorithm provenance, pinned source,
  behavioral constants, and implementation coverage.
- [`docs/PERFORMANCE.md`](docs/PERFORMANCE.md) — benchmark evidence, routing
  guidance, bottlenecks, and limitations.
- [`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md) — code map, repository policy,
  and performance-change discipline.
- [`CHANGELOG.md`](CHANGELOG.md) — user-facing release history.
- [`RELEASING.md`](RELEASING.md) — version, tag, GitHub Release, and future SSC
  publication process.
- [`benchmarks/README.md`](benchmarks/README.md) — reproducible benchmark and
  profiling harnesses.

## Citation

When referring to the CMG method, cite the original paper:

```bibtex
@article{KoutisMillerTolliver2011,
  author  = {Ioannis Koutis and Gary L. Miller and David Tolliver},
  title   = {Combinatorial Preconditioners and Multilevel Solvers for Problems
             in Computer Vision and Image Processing},
  journal = {Computer Vision and Image Understanding},
  volume  = {115},
  number  = {12},
  pages   = {1638--1646},
  year    = {2011},
  doi     = {10.1016/j.cviu.2011.05.013}
}
```

For implementation provenance, also identify this repository as an independent
Rust port and record the version or source commit used.

## License

GNU GPL version 3 only. See [`LICENSE`](LICENSE) and
[`docs/UPSTREAM.md`](docs/UPSTREAM.md).

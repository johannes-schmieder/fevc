# CMG: Combinatorial Multigrid in Rust

[![Rust CI](https://github.com/johannes-schmieder/CMG/actions/workflows/rust.yml/badge.svg)](https://github.com/johannes-schmieder/CMG/actions/workflows/rust.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

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

CMG 0.2.0 is distributed from this repository, not crates.io. Pin an exact
reviewed commit on `main` for reproducible builds; the `v0.2.0` release identifies
the consolidated API. For example, resolve the tag to its full commit SHA and
use it as `rev`:

```toml
[dependencies]
cmg = { git = "https://github.com/johannes-schmieder/CMG", rev = "<full-reviewed-commit-sha>" }
```

Enable the optional parallel implementation with:

```toml
[dependencies]
cmg = { git = "https://github.com/johannes-schmieder/CMG", rev = "<full-reviewed-commit-sha>", features = ["parallel"] }
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
API. It retains the graph, CMG hierarchy, selective parallel plan and the shape
of the team it was built for; it owns no pool. Every parallel operation borrows
a `Team`: a host's own Rayon pool via `Team::new`, or a standalone
`ParallelExecutor` created once and lent with `executor.team()`. Depending on
the workload, the solver chooses serial PCG, parallelizes one sufficiently
large solve, or solves independent right-hand sides concurrently within the
configured memory budget.

```rust,ignore
use cmg::{CmgOptions, ParallelExecutor, ParallelOptions, ParallelPcgSolver, PcgOptions};

let executor = ParallelExecutor::new(ParallelOptions {
    threads: 16,
    workspace_memory_budget_bytes: Some(8 * 1024 * 1024 * 1024),
    ..ParallelOptions::default()
})?;
let solver = ParallelPcgSolver::build(&graph, CmgOptions::default(), executor.team())?;

let mut workspace = solver.workspace();
let batch = solver.solve_batch_with_workspace(
    &right_hand_sides,
    PcgOptions::default(),
    &mut workspace,
    executor.team(),
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

### Fixed-cycle multi-RHS applications

Outer Krylov solvers can apply the stationary preconditioner to a small tile of
component-compatible right-hand sides without starting an inner PCG solve:

```rust
use cmg::{ApplyContext, CmgBatchWorkspace, CmgOptions, CmgPreconditioner, Laplacian};

# fn main() -> Result<(), cmg::CmgError> {
let graph = Laplacian::from_edges(3, [(0, 1, 1.0), (1, 2, 1.0)])?;
let cmg = CmgPreconditioner::build(&graph, CmgOptions::default())?;
let required = CmgBatchWorkspace::allocation_bytes(&cmg, 4)?;
let mut workspace = CmgBatchWorkspace::try_new(&cmg, 4, required)?;
// Two columns, vertex-major: [b0(vertex0), b1(vertex0), b0(vertex1), ...].
let rhs = [1.0, 2.0, 0.0, -1.0, -1.0, -1.0];
let mut output = [0.0; 6];
cmg.apply_compatible_batch_into(&rhs, &mut output, 2, &mut workspace, ApplyContext::default())?;
# Ok(())
# }
```

Each application uses its actual column count as the row stride, so the same
workspace accepts smaller tiles and tails. Sparse matrix, transfer and terminal
factor passes share their traversal across the columns. An `ApplyContext` may
supply a `ParallelCmgPlan` with its borrowed `Team` and a cancellation query;
there is no pool or per-column CMG-owned allocation. The checked
workspace budget includes all retained numerical buffers and descriptor banks,
but excludes the borrowed hierarchy, caller input/output and parallel runtime.
`CmgStorageReport` includes the new workspace in capacity-based ownership reports.

Callers must supply component-compatible columns: this API does not validate or
project the fine-level null space. It applies the fixed cycle and its configured
recursive repeats, without tolerance-based stopping. Capacity one preserves the
existing scalar workspace and cycle exactly. Larger-capacity tiles use serial
compensated coarse-component sums independently in each lane, including when a
parallel plan is supplied; this can differ by rounding from scalar parallel
chunk reductions. Existing scalar and PCG APIs retain their routing. Any
team-carrying solve or application, including a one-worker team and every
`ParallelPcgSolver` mode, uses the same fixed-chunk reductions and centering
above their thresholds, so its bits do not depend on the worker count. Tile width is a caller resource choice, not an automatic performance
claim.

## Validated benchmarks

A current-code qualification on Intel Xeon Gold 6242 nodes compared this crate
with the official MATLAB solver and its C MEX kernels on identical
one-million-vertex inputs from five deterministic connected graph families.
Both implementations ran on the same host for each family. At 16 application
CPUs, the geometric-mean results were:

| Measurement | Rust relative to MATLAB | Plain-language interpretation |
|---|---:|---|
| CMG hierarchy setup | `0.182x` | Rust setup was about 5.5 times faster |
| Stationary CMG application | `0.379x` | Rust application was about 2.6 times faster |
| Reused-preconditioner PCG | `0.388x` | Rust PCG was about 2.6 times faster |
| Setup, parallel plan, and one solve | `0.391x` | Rust was about 2.6 times faster end to end |
| Process peak RSS | `0.227x` | Rust used about 23% of MATLAB's process memory |

All 40 implementation/thread configurations passed scheduler, identity,
timing, and numerical validation. Rust was faster end to end for every family
at 16 CPUs, from `0.656x` MATLAB time on the weighted path to `0.248x` on the
dense worker–firm graph. Sixteen CPUs gave the best geometric mean; grid and
weak-community were individually fastest at 8 CPUs, while the other three
families were fastest at 16. Every family regressed at 32 CPUs: the
geometric-mean Rust total rose from 1.334 seconds at 16 CPUs to 1.803 seconds,
where Rust still took `0.505x` MATLAB time. Single-RHS scaling therefore remains
nonmonotone at high core counts.

The packages retain their native stopping rules. Rust's maximum certified
backward error was `9.24e-9`; MATLAB's maximum native relative residual was
`9.73e-9`. Rust used fewer PCG iterations in these cases, so the timings compare
validated package workflows rather than identical iteration counts or residual
definitions.

The broader first study covers all 15 graph/size cases from 100,000 to
1,000,000 vertices and a separate 16-RHS supplement. In that supplement,
Rust's across-RHS executor achieved 7.76-fold geometric-mean scaling from 1 to
32 CPUs and `0.122x` MATLAB's normalized per-RHS time at 32 CPUs.

These are measurements from one controlled environment, not universal hardware
guarantees. See the [performance guide](docs/PERFORMANCE.md), the
[technical benchmark report](https://github.com/johannes-schmieder/CMG/blob/main/output/pdf/benchmarks.pdf),
and the
[current machine-readable qualification](https://github.com/johannes-schmieder/CMG/blob/main/.ci/performance/scc-rust-matlab-current.json)
for the latest numbers. The report and the
[first-study machine record](https://github.com/johannes-schmieder/CMG/blob/main/.ci/performance/scc-first-study-2026-08.json)
preserve the broader graph-size and repeated-RHS evidence.

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
- [Component-aware experiments](docs/component-aware-cmg.md) — opt-in kernel
  options, integration and qualification limits.
- [`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md) — code map, repository policy,
  and performance-change discipline.
- [CMG teaching supplement](docs/teaching/README.md) — a visual, step-by-step
  introduction to graph Laplacians, CG, PCG, the CMG hierarchy, and a public
  Veneto worker–firm example.
- [`CHANGELOG.md`](CHANGELOG.md) — user-facing release history.
- [`RELEASING.md`](RELEASING.md) — version, tag, GitHub Release, and future SSC
  publication process.
- [Benchmark and profiling tools](https://github.com/johannes-schmieder/CMG/blob/main/benchmarks/README.md)
  — reproducible benchmark and profiling harnesses.

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

MIT. See [`LICENSE`](LICENSE) and
[`docs/UPSTREAM.md`](docs/UPSTREAM.md).

Controlled serial candidate construction, cooperative cancellation, and scalar workspace preparation are documented in [controlled construction](docs/controlled-construction.md).

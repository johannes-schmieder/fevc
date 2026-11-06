# Performance and parallel execution

This is the maintained performance reference for the crate. Detailed one-shot
optimization experiments are preserved in Git history and GitHub Actions
artifacts; the current tree keeps durable harnesses, accepted study records,
and a few clearly labeled historical checkpoints.

## Retained implementation

The production implementation includes compact canonical edge storage, shared Laplacian hierarchy ownership, compact aggregation/CSR indices where qualified, compressed terminal factors, reduced CMG/PCG workspaces, cached endpoint keys, optimized forest splitting and contraction, row-owned parallel construction of dense planned operators, fixed-order parallel centering and norm reductions, a package-owned Rayon executor, selectively routed within-solve parallelism, memory-bounded concurrency across right-hand sides, and final original-system residual certification.

### Fixed topology with changing weights

`PreparedLaplacianTopology` validates endpoints, duplicate groups, components,
and the input-to-canonical edge map once. Each numeric frame still validates
all weights and deterministically rebuilds canonical weights, the diagonal,
matrix-nonzero metadata, and the operator-norm bound. Duplicate weights retain
the legacy weight sort and compensated summation order. Numeric graph storage
is new for every frame; topology and numeric-assembly scratch are reusable.

Caller-buffer PCG entry points return `PcgDiagnostics` without constructing an
owned solution result. `PcgBatchRef` and `PcgBatchMut` accept checked contiguous
or two-stride layouts, and `PcgBatchWorkspace` owns any gather/guess scratch.
Optional guesses are centered component by component and initialize the
residual with the current operator. Passing no guess follows the legacy
zero-start path.

The explicit retained-preconditioner functions accept only current and stale
graphs sharing the same prepared topology/component identity. The stale
hierarchy and optional plan act only as the preconditioner; the current graph
defines all operator products and certificates. CMG reports iterations,
restarts, projection norm, residual, backward error, and failures, but embeds
no rebuild rule. Consumer policy can therefore use observed weight change,
preconditioner age, batch width, fresh setup cost, iterations/restarts,
certificate quality, and prior failures.

The durable `fixed-topology-sequence` benchmark covers the 80-by-64 balanced
bipartite and 5,000-by-4,000 chain fixtures, changing-weight sequences, caller
buffers, retained hierarchies, warm starts, one/eight-thread strategies,
profiling, allocations, and retained memory. Fixture generation, tracing, and
workspace/plan preparation are outside timed regions. Use separate invocations
for timing and allocation interpretation; no result file is committed by the
harness.

## Current Rust versus official MATLAB/C qualification

Run `20260828T021628Z-6fe9be77084a-b2v1-rust-matlab-current` is the matched current-production comparison. It uses source `6fe9be77084a60cca330760361dd4c7addc77ccf`, Rust 1.98.0, MATLAB 2026a with its default C MEX compiler, and the official CMG source pinned at `19752fc102f8cae8e34f66457bfaccb1aaa60375`. The five deterministic connected graph families each have one million vertices and span roughly one million to eight million canonical edges. Each family ran both implementations and all four application CPU counts on the same Intel Xeon Gold 6242 host. Timings are medians of seven measurements after two warmups.

Geometric means across the five families are:

| CPUs | Setup Rust/MATLAB | Apply Rust/MATLAB | PCG Rust/MATLAB | Total Rust/MATLAB | RSS Rust/MATLAB | Rust total speedup vs. 1 CPU |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 0.162x | 0.867x | 0.949x | 0.597x | 0.138x | 1.00x |
| 8 | 0.179x | 0.408x | 0.432x | 0.404x | 0.233x | 1.78x |
| 16 | 0.182x | 0.379x | 0.388x | 0.391x | 0.227x | 1.87x |
| 32 | 0.208x | 0.489x | 0.513x | 0.505x | 0.223x | 1.39x |

At 16 CPUs, Rust's geometric-mean setup, PCG, and total times are 0.229, 0.785, and 1.334 seconds, versus MATLAB's 1.261, 2.025, and 3.412 seconds. The family-level setup-plus-solve comparison is:

| Family | Canonical edges | Rust seconds | MATLAB seconds | Rust/MATLAB |
|---|---:|---:|---:|---:|
| weighted path | 999,999 | 2.015 | 3.073 | 0.656x |
| 2D grid | 1,998,000 | 0.899 | 2.223 | 0.404x |
| worker–firm degree 3 | 1,499,996 | 0.646 | 2.328 | 0.278x |
| worker–firm degree 16 | 7,999,978 | 3.565 | 14.366 | 0.248x |
| weak community | 1,992,015 | 1.013 | 2.023 | 0.500x |

Sixteen CPUs gave the best Rust geometric mean. Dense worker–firm, sparse worker–firm, and path were individually fastest at 16 CPUs; grid and weak community were fastest at 8. Moving from 16 to 32 CPUs increased the Rust geometric-mean PCG time by 42% and total time by 35%; MATLAB total time increased by about 5%. Every Rust family regressed at 32 CPUs. The extra parallel scheduling, synchronization, reduction, and memory-traffic overhead outweighed the additional cores. Sixteen CPUs are therefore the measured aggregate default for one large single right-hand side on this hardware, with 8 CPUs worth testing on sparse spatial or community graphs; 32 remains supported, not preferred.

All 40 configurations passed SGE accounting, source/binary/input identity, application, timing, and numerical validation. Rust's maximum backward error was `9.24e-9`, and MATLAB's maximum native relative residual was `9.73e-9`. Rust's independently recomputed relative residual reached `6.76e-8` because the packages intentionally retain different native stopping rules. Rust also used fewer iterations for every family at 16 CPUs: 53 versus 62 on path, 26 versus 29 on grid and weak community, 20 versus 23 on sparse worker–firm, and 9 versus 11 on dense worker–firm. Runtime ratios therefore compare certified native package workflows, not fixed iteration counts or identical residual definitions.

MATLAB reported official hierarchy flag 3 on the four dense worker--firm configurations. The outputs remained converged and passed the numerical validator; the warning is retained in the archive and compact record. SGE array `7341600.1-5` used 32 slots and 3 GiB per slot. Its tasks completed with `failed=0`, `exit_status=0`, 9.05--13.47 GiB `maxvmem`, and 1,101--2,822 seconds wall time including all configurations, warmups, diagnostics, and process launches.

The compact accepted record is `.ci/performance/scc-rust-matlab-current.json`. Raw repetitions, inputs, logs, and accounting remain in the immutable SCC run archive.

### Broader first study

The first controlled study covers five graph families at 100,000, 300,000, and 1,000,000 vertices with 1, 2, 4, 8, 16, and 32 application CPUs. Its Rust snapshot predates the retained parallel optimization, so it remains the broad size-scaling record rather than the best estimate of current Rust latency. At 32 CPUs, the geometric-mean Rust/MATLAB ratios across its 15 graph/size cases were 0.265x for setup, 0.790x for stationary CMG application, 0.783x for PCG, 0.666x for setup plus one solve, and 0.150x for process peak RSS.

The 16-RHS supplement provides an important contrast. Across sparse and dense worker-firm graphs at 300,000 and 1,000,000 vertices, Rust's memory-bounded across-RHS executor has a 7.76x geometric-mean 1-to-32-CPU speedup, while sequential MATLAB PCG has 1.03x. At 32 CPUs, Rust's geometric-mean normalized per-RHS time is 0.122x MATLAB's. The four individual Rust speedups range from 6.63x to 9.89x. Coarse-grained independent-RHS concurrency is therefore substantially more effective on this machine than the selectively parallelized single-RHS workflow.

All 180 main and 48 batch implementation/thread points pass the scheduler, identity, timing, memory, and numerical validator. The maximum Rust backward error is `9.95e-9`; MATLAB's maximum native reported relative residual is below `1e-8`. Independently recomputed diagnostics are retained because the two packages preserve different native stopping rules. Twelve of 15 main graph/size cases have matching intermediate hierarchy sizes but a one-vertex difference in the final coarse level; the three dense cases match exactly. The official MATLAB hierarchy reports its nonzero status flag for the dense cases, which remain included and explicitly marked.

The original main archive is immutable. Its Rust and standalone-C JSON rows recorded `source_commit=unknown` because the wrapper and compiled benchmark driver used different environment-variable names. A derived copy repairs only that field in 96 files from the exact source manifest; its receipt records the raw-tree digest and all before/after hashes. No timing, numerical, input, or environment value is changed. The strengthened validator accepts the derived run, and the report discloses this bookkeeping correction.

The complete first-study report, figures, and compact record live in
`benchmarks/report/` and
`.ci/performance/scc-first-study-2026-08.json`. The exact first-generation
harness is preserved at Git tag `benchmarks-v1-2026-08-24`; the active SCC
workflow is `benchmarks/scc/`. Raw repetitions, logs, resource receipts, and
SGE accounting remain in the isolated run archive.

## Direct comparison with pinned C kernels

`benchmarks/c-kernel/` checks pinned upstream C sparse kernels against Rust. In the SCC study, Rust/C SpMV ratios range from 0.666x to 0.915x, restriction and prolongation are near parity, and the bounded recursive-cycle ratios range from 0.737x to 0.803x. This is direct hot-kernel evidence, not an end-to-end C-solver comparison, because upstream hierarchy construction is primarily MATLAB code.

## Parallel routing

`ParallelPcgSolver` chooses among serial PCG, selectively planned within-solve PCG, and independent solves distributed across right-hand sides.

The default single-RHS planned threshold is **350,000 canonical retained edges**, provided more than one worker thread and either a routed operator or sufficiently large deterministic parallel vector work is available. The vector-only route is limited to connected graphs large enough for fixed-chunk centering and reductions. This is a measured performance heuristic, not a mathematical CMG constant, and can be overridden through `ParallelPcgPolicy`.

Within a plan, a level receives a row operator when it is non-terminal, has at least `1.25 n` edges and at least `min_parallel_len` edges. A row-operator matvec runs on the team when its directed entries times lanes (at least its row count) reach `min_parallel_len`, so every planned level's matvec is team-parallel on a multi-worker team, dense coarse levels with few rows included; each row keeps its serial canonical-order sum, so dispatch never changes bits. Diagonal scaling, Jacobi updates and prolongation gate on rows times lanes.

The retained hosted routing checkpoint is
`.ci/performance/routing-checkpoint-959f635.json`; its largest dense worker-firm
case reports roughly 2.2x planned-versus-serial full-PCG speedup with unchanged
iteration count and solution certificate.
`.ci/performance/parallel-checkpoint-88bf024.json` records repeated-RHS thread
scaling. These are historical checkpoints, not current-head claims.

### SCC optimization qualification

The post-diagnostic optimization bundle (`761a0f0`, run `20260827T064306Z-761a0f022f20-b2v1-routing-opt2`) was qualified against its immediate baseline (`5f45ded`, run `20260826T222423Z-5f45ded81164-b2v1-routing-opt1`) on the same Gold-6242 host. The comparison uses all five deterministic one-million-vertex families, forced planned execution at 8, 16, and 32 threads, two warmups, and five measured repetitions. The optimized/baseline geometric ratios were **0.701x for reused-preconditioner PCG** and **0.747x for setup plus one solve**. Serial controls were 0.980x and 0.975x, respectively, which isolates most of the improvement to the intended parallel path. Dense worker-firm plan construction was 0.688x baseline.

Iterations, hierarchy structure, routed-operator counts, plan bytes, and workspace bytes were unchanged; the maximum candidate backward error was `9.24e-9`. Sixteen threads remained the fastest planned setting for all five families. Moving from 16 to 32 threads increased solve time by 3% for dense worker-firm and by 16% to 76% for the other families, showing that extra high-core overhead outweighs the additional parallel work. On the operator-free path family, the planned vector kernels reduced setup-plus-solve time by 11% to 21% versus serial at 8 to 32 threads, motivating the connected vector-only automatic route described above.

## Memory budgeting

Let `W = solver.workspace_bytes()` and let `B` be the configured reusable workspace budget. Batch concurrency is bounded approximately by:

```text
min(thread_count, rhs_count, floor(B / W))
```

The budget excludes the shared graph, hierarchy, optional parallel plan, input RHSs, result vectors, allocator overhead, and host-process memory. Large machines should not assume that one full PCG workspace per CPU is affordable.

## Current bottleneck

For single-right-hand-side latency, the current qualification identifies 16 CPUs as the useful ceiling on Gold 6242. The 16-to-32 regression appears across every family, including the dense case whose planned operators have the most work, so future high-core optimization should target task granularity, global reductions, synchronization, memory placement, and bandwidth rather than assuming more operator parallelism is sufficient. Any change should demonstrate a lower complete setup-plus-solve time, not only a faster inner kernel.

Hierarchy setup is still dominated by coarse contraction, and contraction is dominated by endpoint sorting. Several bucket/radix alternatives were benchmarked and rejected because their end-to-end or peak-memory tradeoffs were worse than the retained cached-key comparison sort. Further setup changes should therefore require a clear end-to-end gain on large worker-firm graphs with bounded peak memory.

## Durable machine-readable records

`.ci/performance/index.json` identifies the source revision, scope, and status of
every retained record. The two accepted SCC records are
`scc-rust-matlab-current.json` and `scc-first-study-2026-08.json`; all other
files are explicitly named checkpoints tied to older source revisions.

Automated performance workflows publish current-head outputs as Actions
artifacts and step summaries. They do not mutate `main`. Promoting a new
accepted record is a deliberate, reviewed documentation change. Older
experiments remain available through Git history and immutable SCC archives.

## Benchmark discipline

1. Keep the production baseline untouched.
2. Build baseline and candidate with the same compiler, features, and CPU settings.
3. Use deterministic identical graphs and RHSs.
4. Alternate baseline/candidate measurements and use medians.
5. Verify hierarchy metadata, iteration counts, backward errors, and scaled solution differences before interpreting time.
6. Measure requested allocation and process RSS when memory can change.
7. Retain a change only when its end-to-end benefit justifies its complexity and memory cost.

Durable benchmark binaries live in `benchmarks/`; see `benchmarks/README.md`.

## 8/16/32-core qualification

The library can construct package-owned pools at 1, 2, 4, 8, 16, and 32 threads. That establishes functional support, not scaling performance.

`.github/workflows/manual-32-thread-qualification.yml` is a manually dispatched, read-only workflow for a configured larger runner or controlled self-hosted machine. It records machine topology, compiler versions, numerical agreement, setup/solve timing, iteration counts, hierarchy/workspace bytes, peak RSS, and 1/2/4/8/16/32-thread measurements where hardware permits.

The current SCC qualification supplies controlled 1/8/16/32-core evidence for the five one-million-vertex graph families described above. It supports 16 CPUs as the best aggregate single-RHS setting on Gold 6242, with 8 CPUs best for grid and weak-community inputs; 32 CPUs regressed for every family. Ordinary hosted-runner results and extrapolations beyond the measured SCC environment should still be described as directional only.

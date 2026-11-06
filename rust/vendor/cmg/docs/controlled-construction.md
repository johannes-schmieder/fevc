# Cancellable construction, application and PCG

## Host execution

With `parallel`, `Team::new(&pool, worker_ceiling, ParallelOptions)` borrows an
already admitted Rayon pool without creating workers or retaining an owner;
`ParallelExecutor::team()` lends a standalone owned pool the same way. A team
is `Copy`, never stored in a hierarchy or workspace, and every parallel kernel
dispatches on its pool, from the coordinator or from a foreign pool alike.
`ParallelPcgSolver` and `ParallelCmgPlan` record the team's shape (worker
ceiling and thresholds) and reject a differently shaped team; CMG never
creates a pool during construction, application or a solve.

Every parallel stage runs on at most `worker_ceiling` lanes of the borrowed
pool, even when the pool is larger. Row and item kernels claim fixed chunks
through one dispatcher that forks exactly `min(ceiling, chunks)` lanes;
reductions use one fixed balanced tree over fixed leaves whose forks stop once
the lane budget is spent, so the tree, and every bit, is independent of the
ceiling. No other CMG code forks or installs work on the pool.

A team of any ceiling, one included, uses that tree for PCG dot products, norm
sums and single-component centering whenever a length reaches
`max(min_parallel_len, 8 * reduction_chunk_size)`. `Team::single_lane()` is a
one-lane view of the same pool with the same thresholds; `ParallelPcgSolver`
runs serial solves and each concurrent right-hand-side lane on it, so every
execution mode and worker ceiling produces the same bits. Row operators (CSR
gather) and edge-list products (scatter) are bit-identical, so which levels
carry row operators remains a worker-dependent resource choice. Team-free
calls keep the serial compensated sums.

Owned-executor destruction joins every worker and its TLS destructors. If a
misbehaving detached callback retains the final executor owner on its own
worker, defensive destruction signals termination and detaches the handles to
avoid self/peer-join deadlock; that unsupported path has no immediate join
guarantee and is not an additional asynchronous execution API.

`ControlledStage::admit` validates fixed chunk dimensions. It is
allocation-free, performs no scan and charges nothing. The resulting
`AdmittedStage` is consumed once by `run(team, ..)`. Chunk boundaries depend
only on the slice length and the chunk length, never on the number of workers.

The caller must admit and prepare all output and kernel scratch storage before
dispatch, including its coexistence with all live parent objects. `run` splits
an exact-length mutable slice into disjoint logical chunks and recursively caps
terminal lanes at the worker ceiling. It allocates no task, result or scratch
banks. Kernels must do bounded work per chunk, use only admitted disjoint
storage, and cannot allocate, launch nested work or run unbounded adaptive
loops. Caller closures and the Rayon runtime are not allocator-sandboxed.
Runtime and stack allowances remain the host's responsibility.

Every admitted chunk runs unless cancellation or panic intervenes; numerical
errors do not race to stop sibling tasks. All tasks join before return/unwind,
and the first logical chunk error wins. Workers query the cheap, sticky
cancellation signal before every chunk, and the coordinator queries it again
after the join. Cancellation dominates numerical errors after joining and is
reported as `CmgError::Cancelled`. Partial output remains private and must be
discarded after failure. Serial fallback is chosen before numerical execution.

The package declares an empty root Cargo workspace so a standalone CMG
development checkout nested inside an embedding workspace remains independent.
This does not alter dependency features or the runtime.

## One construction pipeline

`Laplacian::build`/`from_edges_with`, `CmgHierarchy::build_with`,
`CmgPreconditioner::build_with` and `GroundedLdl::factor_with` each take a
`BuildContext { team, cancel, budget_bytes }`. The pipeline is the same for
every context: graph canonicalization, the hierarchy loop (forest grouping,
aggregation, stopping rules, contraction, optional isolate pruning),
finalization (finest components, coarse centering plans, terminal factor and
repeat adjustment). A team only selects the existing deterministic parallel
kernels above their fixed thresholds (initial sorting, contraction and
heavy-forest selection); a
budget only admits `construction_allocation_bound` before the first allocation
and makes every bank fallible; a cancellation query only replaces opaque
traversals with fallible bounded ones. Edge sorting then uses one counted
in-place introsort (heapsort fallback) and other key sorts a counted heapsort.
Edge ordering remains endpoint pair then total-ordered weight; terminal
permutation keys include the original vertex as the final tie-break. Thus
hierarchies, factors, repeats and cycle bits never depend on the context. The
terminal factor is chosen by the hierarchy options (`direct_threshold` and the
stopping rules) and memory admission only; no work allowance redirects it.

With a cancellation query, `Laplacian::build` also bounds input validation,
canonical edge sorting, compensated duplicate aggregation, and graph metadata
construction. Its input is a borrowed slice, avoiding arbitrary user iterator
work inside a bounded construction operation; callers must include their own
input preparation in their resource protocol. Scalar application storage is
prepared with `try_workspace_controlled` after admitting
`workspace_allocation_bytes_controlled`, and interleaved storage with
`CmgBatchWorkspace::try_new_controlled`. They share their implementations with
`try_workspace`, `workspace_allocation_bytes` and `CmgBatchWorkspace::try_new`,
producing the same layout, zeroed contents, byte count and error
classification; only the cancellation polls are added. Every level,
centering-plan and component-bank loop and every value initialized contributes
to the cadence below, every allocation is preceded by a poll, and the queries
poll once more before returning.

## Application and PCG

`ApplyContext { plan, team, cancel }` and `SolveContext { plan, team,
concurrent_right_hand_sides, retained_preconditioner, cancel }` drive the one
cycle recursion and the one PCG recurrence. With a cancellation query every
edge, row, transfer entry, centering value and factor entry visited counts
toward the same fixed cadence, carried across row, level and iteration
boundaries; each PCG iteration polls once more. Long traversals (a CSR row,
a terminal-factor row, a reduction leaf) are counted in segments of at most
1024 units while they run, never once before an unpolled loop. Each parallel
lane keeps its own count and polls the sticky query every 65,536 units, also
inside a row or leaf, and stops early; all lanes join, the coordinator polls,
and only then is `CmgError::Cancelled` returned. A stopped lane stays stopped:
the fixed reduction tree visits none of its remaining leaves and every other
lane stops at its next leaf, so the entries traversed after the observation are
bounded by one poll interval plus one segment per lane. A batch row wider than
one counted segment (more than 1024 lanes) is processed and counted in lane
tiles of at most 1024 lanes; each lane keeps its operation order. Concurrent solves poll after
the join before choosing errors, publish through the bounded meter and poll
once more before success. Outputs are then unspecified, workspaces stay
reusable, and no context changes a result bit. Plan construction
(`ParallelCmgPlan::build_with`, `ParallelPcgSolver::try_plan_with`) and PCG
workspace banks (`PcgBatchWorkspace::new_bank_with`) take the same context.
`solve_pcg_with` copies its owned solution through the counted meter before
its final poll. Prepared topology preparation polls before each map reserve;
numeric assembly counts the diagonal metadata scan, polls before allocating the
shared handles and once more after publication, and
`PreparedLaplacianWorkspace::new_with` polls before and after allocating its
scratch.

Team construction stages are bounded too: contraction maps edges in fixed
logical chunks and fills and compacts its slot bank in counted segments, and
team canonicalization sorts in place with a counted introsort whose lanes poll
inside every partition and fork only within the worker ceiling. Heavy-forest
selection on a level with at least 65,536 directed entries and more than 1.5
edges per vertex gathers incoming edges into a counted transient index (row
starts plus one `u32` or `usize` per edge), used only when it is no larger
than the contraction slot bank that the parallel bound already adds to the
serial forest scratch; lanes own fixed 4,096-vertex blocks and count every
scanned entry, so a hub row polls inside itself. The selection is a maximum
under a total order (weight, then lowest neighbor), so parents and selected
weights equal the serial edge scan's. The
initial-graph constructor from a borrowed slice maps edges in fixed logical
chunks and then sorts them in place with the same team introsort, which needs
no spare sort bank. Its admitted bound is unchanged and still counts two edge
banks: the second covers the transient compaction copy that duplicate
reduction (`from_sorted_edges`) allocates while the merged bank is live.

Every canonical edge sort (initial graph, contraction, owned compact edges) is
the one introsort in `graph/edge_sort.rs`: three-way partitions, insertion
below 17 records and a heapsort fallback at depth `2 log2(n)`. Partitions of
more than 40 records take Tukey's ninther as pivot (Bentley and McIlroy): with
`s = len / 8`, `last = len - 1` and `m = last / 2`, the median of the medians
of positions (0, s, 2s), (m - s, m, m + s) and (last - 2s, last - s, last).
Smaller partitions take the median of the first, middle and last records.
Pivot selection is conservatively charged after it runs: 192 units for at
most twelve comparisons in a ninther, or 48 for at most three in a median
of three (16 units per possible comparison). A partition then advances its meter by
1,024 units (16 per step) after every 64 steps; the meter queries
cancellation once 65,536 units have accumulated, so inside a partition a
query falls about every 4,096 steps. Work between two queries is at most
65,535 + 1,024 counted units plus one uncounted bounded segment: 64
partition steps, one pivot selection (at most twelve comparisons and nine record copies),
one insertion record or one heap step. The ninther avoids the depth fallback
on the tested sorted, reversed, organ-pipe and block-structured fixtures;
an adaptive
adversary still reaches the fallback. Serially it counts on the caller's
meter. Sampling and the resulting partition tree can change work counts
and query positions without changing the cadence or boundedness. On a team, partitions of at least 16,384 records fork within the
ceiling, every logical node of the partition tree (partition, insertion leaf
or heapsort fallback) additionally queries on entry with its own lane
counter, and the coordinator queries once more after the join. The tree
depends only on the input, so a successful run's total query count and each
node's logical checkpoints are identical at every ceiling, and work between
two queries of a lane stays bounded deep in the tree. The chronological order
of queries across lanes, and therefore where a cancellation stops, may differ
between runs and ceilings. The total comparator (endpoint key, then
`total_cmp` weight) makes the result equal to any other sort's.

## Cancellation contract

Each entry point accepts an optional `Cancellation` in its context: a
caller-owned, cheap, allocation-free and sticky query. Kernels count conservative primitive units
(arithmetic, record visits, comparisons and moves) in bounded operations of 1
through 1024 units. Inner tree/root walks, forest splitting, edge merging,
dense/sparse factor updates, sorting comparisons and swaps, and storage
initialization all contain these operations. The serial meter polls the query
whenever the running count crosses a fixed cadence of 65,536 units, regardless
of row, group, pivot or level boundaries, and before every allocation. Work
between two polls is therefore at most the cadence plus one bounded operation.
Units are a polling cadence, not instructions, elapsed time, an admission
budget or a claim of equal cost across phases. Nothing is charged, refused or
reported on the basis of work.

A true query returns `CmgError::Cancelled { phase }`, naming the phase of the
next unstarted operation, through ordinary `Result` propagation. The private
partial candidate is dropped; no partially constructed graph or preconditioner
is returned and the query is not consulted again. Memory admission uses
`MemoryBudgetExceeded` and is checked before the first allocation; allocator
failure uses `AllocationFailed`. Internal invalid state remains an error.

The query runs on the constructing thread (and, in parallel stages, on workers
between chunks). It may block, for example to park a construction, and then
return either value. No construction, application or solve creates a pool. Allocator, deallocator, query, page-fault,
join and operating-system latency are excluded from the bounded work between
polls. In particular this is not a hard real-time cancellation guarantee or a
bound on cleanup latency. Small Arc allocations retain stable Rust's existing
infallible allocation semantics. Controlled boxed component-label conversion
requires exact capacity so that conversion cannot hide an allocation; excess
capacity is a reported allocation error rather than an unchecked shrink.

Graph and preconditioner memory bounds retain the existing requested-layout
model and its allocator/runtime exclusions. The graph input tuple bank is
borrowed and excluded from the graph constructor's allocation bound. The caller
must account for its coexistence with candidate storage. Scalar workspace bytes
are reported by `workspace_allocation_bytes` (or its controlled form).

## Fixed-cycle symmetry and positivity

The mathematical contract is on each component's compatible quotient/range,
where a positive-weight Laplacian A is positive definite. It does not assert
positive definiteness on the full vertex space containing component constants.
Write S = (1/2) diag(A)^(-1) on nonisolated vertices. The graph energy inequality
A <= 2 diag(A) gives 0 < A^(1/2) S A^(1/2) <= I on that range. Isolates have zero
compatible right-hand sides and carry no positive-dimensional range.

Restriction is the transpose of prolongation P and the coarse graph is the
Galerkin product P^T A P. Pruning deletes only isolated coarse vertices; their
coarse range is empty, so it does not change that identity on the retained
range. Coarse component centering is the orthogonal projection onto its range;
exact restricted compatible residuals already lie in that range. Direct
terminals use a positive-pivot grounded LDL solve with component normalization,
which represents the inverse on the quotient. Other terminals use S, which has
the same positive contraction property. Positive-pivot checks remain errors.

For a coarse inverse C that is symmetric positive and bounded by the coarse
inverse on the range, one symmetric pre/post smoothing cycle has operator

    B = 2S - SAS + (I - SA) P C P^T (I - AS).

In energy coordinates, its error operator is

    I - A^(1/2) B A^(1/2)
      = (I - T) (I - A^(1/2) P C P^T A^(1/2)) (I - T),
    T = A^(1/2) S A^(1/2).

It is positive semidefinite and has eigenvalues strictly below one on the
compatible range. Thus B is symmetric positive and A^(1/2) B A^(1/2) <= I.
Repeating this fixed stationary correction k >= 1 times replaces the energy
error operator E by E^k, retaining those properties. Induction over the stored
hierarchy proves the fixed cycle's quotient-space symmetry and positivity.
Repeat counts are fixed during construction, and the controlled builder leaves
them unchanged. These statements concern exact arithmetic; finite-precision
application and centering retain the existing numerical certification rules.
RHS-dependent hierarchy changes, adaptive repeat counts, a non-transpose
transfer, or arbitrary indefinite/non-Laplacian inputs are outside this argument.

`apply_compatible_into_with` applies that fixed cycle with a prepared workspace
and an `ApplyContext`. Its caller must supply a compatible RHS. Construction
adds no adaptive selection during application; a cancellation query in the
context only adds the bounded polls described above.

## Validation

The controlled unit and integration tests compare complete hierarchy/factor
structures, floating-point factor and cycle bits, and repeat counts of the
cancellable and ordinary contexts of the one builder; `tests/frozen_bits.rs`
compares every consolidated construction, cycle and solve path at one and four
workers with digests frozen from the pre-consolidation implementations. Fixtures include empty, isolated, connected,
disconnected, sparse and dense graphs, nontrivial scales, and all component
option combinations. Unit tests poll after every bounded operation and cancel
at each poll of small graph, forest, contraction, pruning, component, factor
and complete-candidate examples; integration tests cancel at every poll of the
public builders, verify deterministic poll schedules and exercise a blocking
query that parks and then continues or cancels. Workspace tests cancel at every
poll of component, centering and complete scalar workspaces and of the layout
query, observe polls inside each bank's initialization, and then rebuild a
workspace that equals the ordinary one and applies identical cycle bits. An independent allocator
tracer verifies that every actual allocation follows a fresh poll, that the
observed requested-layout peak is within the declared bound, and that
cancellation at each poll releases all candidate storage. These tests support
implementation equivalence and cancellation behavior; the positivity argument
above is independent of sampled numerical positivity.

History: before 2026-10-19 the controlled builders also carried a construction
work ledger with per-boundary callbacks, a work budget, a factor-affordability
preflight, cumulative prefix grants and a scalar-cycle work bound. Johannes
Schmieder removed that optional accounting on 2026-10-19; cancellation and
memory admission are the remaining controls. The 2026-10-13 Mac validation of
the former interface (Rust 1.97.1, 200 tests across 32 groups) and commit
`977b1e3`, which separated zero-sized Arc and vector identity namespaces in the
storage report, remain part of the record.

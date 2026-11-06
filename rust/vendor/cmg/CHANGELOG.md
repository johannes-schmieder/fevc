# Changelog

All notable user-facing changes to CMG are documented here. This project uses
semantic-style version numbers; release dates use ISO `YYYY-MM-DD` format.

## Unreleased

## 0.2.0 - 2026-11-06

### Consolidation

- Adopt one maintained implementation with host-owned teams, unified build,
  application and solve contexts, checked cancellation and the 32-level default.
  Consumers can explicitly retain a 128-level cap without a source fork.
- Remove the unused `ComponentPcgExperiment`, `ComponentPcgWorkspace` and
  `ComponentPcgDiagnostics` portfolio APIs and their dedicated benchmark.
  Component pruning and terminal-factor options remain available and opt-in.
- Replace the superseded controlled work-budget APIs with the unified contexts;
  controlled scalar and interleaved workspace allocation remain supported.
- Provide reproducible, byte-verified vendor snapshots for downstream packaging.
- Reduce scalar PCG vector-update and terminal-correction loop overhead while
  preserving arithmetic order, storage, and bounded cancellation counts.


### Changed

- License CMG under MIT following permission from its developers via Ioannis
  Koutis, confirmed by Johannes Schmieder on 2026-11-03. Preserve upstream
  attribution and the existing numerical implementation.

### Added

- Allocation-free fixed-cycle interleaved multi-RHS applications through
  `CmgBatchWorkspace` and `apply_compatible_batch_into`, with checked workspace
  admission, capacity ownership accounting, and optional existing parallel plans.
  Capacity one retains the scalar path; wider tiles share matrix, transfer and
  factor traversals and use serial compensated coarse sums, whose rounding may
  differ from scalar parallel chunk reductions. Existing APIs are unchanged.
- Deterministic stationary CMG preconditioning and certified PCG solves for
  weighted graph Laplacians and SDDM systems.
- Optional package-owned parallel execution and reusable repeated-right-hand-side
  solvers.
- Checked conservative pre-build memory estimates and exact retained-memory
  reports for prepared parallel solvers and workspace pools.
- Cross-platform correctness tests, reproducible benchmark harnesses, and pinned
  upstream provenance.
- Prepared fixed-topology Laplacians for deterministic changing-weight numeric
  assembly, solution-free caller-buffer PCG diagnostics, checked strided batch
  views, optional warm starts, and explicit certified retained-preconditioner
  solves.
- Conservative and exact repeated-solve memory accounting that separates
  prepared topology, current numeric state, stale hierarchy/plan, assembly
  scratch, workspace pools, and caller buffers.
- Opt-in `experimental-components` builders for compact coarse graphs and
  ordered sparse terminal factors, with an option to preserve the original
  hierarchy stopping rules; component benchmark fixtures and allocation checks.

### Changed

- `CmgOptions::default().max_levels` is 32 instead of 128. The value is both
  the hard stop of hierarchy construction (a level that would contract further
  at that depth becomes the iterative `MaximumLevels` terminal) and the level
  count every pre-build admission bound assumes, so default construction,
  scalar-workspace and solver memory bounds no longer provision 127 coarse
  graph copies that ordinary 3-6-level hierarchies never build; they admit
  budgets the former assumption refused. Every hierarchy of at most 32
  levels is bit-identical; one that would need more levels now stops at level
  32 inside the admitted bytes, its first 31 levels unchanged. Explicit `max_levels` values
  keep their own stop and admission. No terminal reason is added.

- The row-operator (CSR) matvec gates team dispatch on its work, directed
  entries times lanes (at least the row count), instead of its row count,
  against the team's `min_parallel_len`. A planned level always has at least
  that many edges, so every planned level's matvec now runs on a multi-worker
  team, including dense coarse levels with fewer rows than the threshold
  (previously serial); tiny operators and one-lane teams stay serial. Each row
  keeps its serial canonical-order sum, so the change is arithmetic-preserving:
  every output bit, plan, eligibility rule, hierarchy and frozen digest is
  unchanged. Only the dispatch, and with it where a cancellation query lands,
  differs.

- Canonical edge sorting under a cancellation query uses one counted in-place
  introsort (`graph/edge_sort.rs`) on every path. Serial initial graphs and
  contraction no longer use the counted heapsort, and the hosted initial
  graph no longer runs insertion-run and merge passes through a spare bank.
  Partitions advance the meter by 1,024 units every 64 steps, so a query
  falls about every 4,096 steps (65,536 units). On a team every logical
  partition node also queries on entry with its own lane counter, so a
  successful run's query total and per-node checkpoints are identical at
  every worker ceiling; the chronological order of queries and where a
  cancellation stops are not. Order, duplicate reduction, hierarchies,
  factors, repeats and every bit are unchanged. Only the spare sort bank is
  gone; the unchanged public construction bound still counts two edge banks,
  the second covering the transient compaction copy after duplicate
  reduction.

- The canonical edge introsort takes Tukey's ninther (nine evenly spaced
  samples) as pivot for partitions of more than 40 records, keeping the
  median of first, middle and last records below. Pivot selection is now
  conservatively charged at 48 or 192 units for at most three or twelve
  comparisons, within one 1,024-unit segment. Sampling and the resulting
  partition tree can change work counts and query positions; the cadence,
  boundedness and worker-ceiling independence remain. The tested structured
  orderings avoid the depth fallback; it remains for adversarial input.
  Order, duplicate reduction and every successful scientific bit are unchanged.

- Team-carrying solves and applications are bit-identical at every worker
  ceiling, one included, and in every `ParallelPcgSolver` execution mode. PCG
  dot products, norm sums and single-component centering (finest PCG level and
  scalar-cycle coarse levels) use the team's fixed-chunk tree whenever a length
  reaches `max(min_parallel_len, 8 * reduction_chunk_size)`, also on a
  one-worker team, and serial and concurrent right-hand-side solves run on
  `Team::single_lane()`, a one-lane view of the caller's team, instead of the
  team-free recurrence. This changes rounding (not iteration logic or
  certification) of one-worker, serial-mode and across-RHS solves at or above
  that length; multi-worker planned results are unchanged. Team-free calls keep
  the serial compensated sums. Row operators remain a worker-dependent
  resource choice: they reproduce the edge-list products bit for bit. No
  allocation, pool or admission changes.
- Prepared topology and numeric-frame publication polls before each shared-owner
  allocation, and topology preparation polls after constructing its private
  result. Cancellation drops partial owners before returning; allocation order,
  layout admission and successful arithmetic are unchanged.
- Planned within-solve PCG uses fixed-order parallel component centering for
  large connected systems, preserving deterministic results across eligible
  multithreaded worker-pool sizes (one worker too, since the change above);
  prepared automatic routing can select these qualified vector kernels even
  when no hierarchy operator qualifies for row-parallel storage.
- Dense parallel-plan construction fills deterministic row-owned CSR blocks in
  parallel, and prepared solvers compute workspace requirements without a
  throwaway allocation.
- Reused CMG/PCG workspaces skip redundant vector clears and repeated internal
  validation after the public solve boundary has validated the complete
  workspace once.
- Controlled serial and hosted construction is cancellation-only. The
  experimental controlled builders (since consolidated into `BuildContext`'s
  `cancel` query, below) take a sticky `Cancellation` query instead of a work
  budget and control callback.
  They poll it at a fixed cadence of counted primitive operations and before
  every allocation, and report `CmgError::Cancelled { phase }`.
  `ControlledStage::admit` validates chunk geometry only. Hierarchies, factors
  and cycle bits of successful builds are unchanged.
- `CmgPreconditioner::workspace_allocation_bytes_controlled` and
  `try_workspace_controlled` are now cancellation-only. They share one
  implementation with `workspace_allocation_bytes` and `try_workspace`, return
  the same bytes, layout, contents and errors, poll the `Cancellation` before
  every allocation, at the construction cadence while initializing storage and
  before returning, and report `CmgError::Cancelled { phase }`.

- One construction pipeline. `CmgPreconditioner::build_with(graph,
  BuildOptions, BuildContext)`, `CmgHierarchy::build_with`,
  `Laplacian::build`/`from_edges_with` and `GroundedLdl::factor_with` replace
  the serial, budgeted, executor, controlled and component-experiment builder
  cross-product. `BuildOptions` carries the hierarchy and component options;
  `BuildContext` carries an optional borrowed `Team`, an optional sticky
  `Cancellation` and an optional CMG-owned requested-layout budget. Hierarchies,
  terminal factors, repeat counts and cycle bits do not depend on the context.
  `CmgPreconditioner::construction_allocation_bound(n, m, options, parallel)`
  merges the serial, executor and component bounds with identical values.
- One cycle algorithm. Scalar, planned and interleaved applications share one
  level recursion over the two existing sparse operator kernels (canonical edge
  list or row-owned CSR). `ApplyContext { plan, team, cancel }` replaces the
  separate plan/executor entry points; `apply_projected_into` and
  `apply_compatible_into_with` are the validating and prevalidated scalar
  entries, and `apply_compatible_batch_into` takes the same context.
- One PCG recurrence. `SolveContext { plan, team, concurrent_right_hand_sides,
  retained_preconditioner, cancel }` selects serial, planned, across-RHS and
  retained-preconditioner solves through `solve_pcg_with`, `solve_pcg_into` and
  `solve_pcg_batch_into`; `solve_pcg_batch_concurrent` keeps the owned-vector
  across-RHS convenience and `profile_pcg`/`profile_pcg_batch_into` record the
  same recurrence. Reduction orders are unchanged for every context.
- Host-owned execution. `Team::new(&pool, worker_ceiling, ParallelOptions)`
  borrows a caller's Rayon pool; `ParallelExecutor::team()` lends an owned
  pool for standalone use. `ParallelPcgSolver`, `ParallelCmgPlan`,
  `ControlledStage`/`AdmittedStage::run` and every parallel kernel take a
  `Team` and never create a pool; `ParallelPcgSolver` records the team's shape
  and rejects a differently shaped team.
- Construction, application and PCG are cancellable with a `Cancellation` in
  their context: counted operations are polled at a fixed cadence, parallel
  lanes poll between bounded sub-ranges, all lanes join before
  `CmgError::Cancelled` is returned, and partial outputs are discarded.
  `CmgBatchWorkspace::try_new_controlled` adds cancellable batch-workspace
  preparation with the layout and contents of `try_new`.
- Profiling builds (`build_with_profiled`, `ParallelCmgPlan::build_profiled`,
  `profile_apply_compatible_into`, `profile_pcg*`) record the production
  kernels instead of maintaining separate copies.

- Bounded cancellation inside every traversal (M3b review repair). Long CSR
  rows, terminal-factor rows (scalar and interleaved, forward and backward)
  and fixed-chunk reduction leaves (PCG dot, norm scale, scaled squares,
  centering sums) are counted in segments of at most 1024 units while they
  run, serially through the meter and in parallel lanes through a lane-local
  poll of the sticky query every 65,536 units; accumulators cross segments
  unchanged, so every bit is preserved. Lanes no longer count rows before an
  unpolled traversal.
- Every parallel CMG stage runs on at most the team's `worker_ceiling` lanes:
  row kernels claim fixed blocks through one capped dispatcher, reductions use
  one fixed balanced tree whose forks are limited by the ceiling, concurrent
  right-hand sides are claimed by at most that many lanes, and no kernel forks
  or installs work on the pool outside these dispatchers and `ControlledStage`.
- Team construction is bounded: contraction maps edges in fixed logical
  chunks and fills and compacts its slot bank in counted segments, and team
  canonicalization (contraction and `Laplacian::from_edges_with`) sorts with an
  in-place introsort (three-way partitions, median-of-three pivots, heapsort
  fallback) that counts every partition, insertion and heap step, polls in its
  lanes and forks only within the worker ceiling, instead of Rayon's opaque
  parallel sort. It allocates nothing, so allocations, admission and bounds
  are those of the earlier in-place sort; graphs and hierarchies are
  bit-identical.
- Cancellable plan and PCG workspace preparation: `ParallelCmgPlan::build_with`
  and `ParallelPcgSolver::try_plan_with` take a `BuildContext` (team, budget,
  cancellation) and meter row-operator construction including its parallel
  fill; `PcgBatchWorkspace::bank_allocation_bytes_with` and `new_bank_with`
  share one metered layout and preparation with the existing entry points;
  `CmgBatchWorkspace::allocation_bytes_controlled` is the metered layout that
  `try_new_controlled` now uses.
- `PreparedLaplacianTopology::assemble_with` assembles a numeric frame under
  a `BuildContext` (budget, cancellation): weight validation, duplicate
  gathering, the counted heapsort and compensated sum of each duplicate group
  and the diagonal are polled; frames are bit-identical to
  `assemble_with_workspace`, which shares the implementation.
  `PreparedLaplacianTopology::prepare_with` prepares a topology the same way
  (validation, counted heapsort of the unique indexed keys, grouping and
  components polled); `prepare_with_budget` and `prepare` share it.
- Concurrent PCG polls the sticky query after its lanes join and before
  choosing errors, publishes solutions through the bounded meter and polls
  once more before success, so a cancellation is never masked by an earlier
  numerical error or lost during publication.
- Profiled hierarchy and preconditioner builds reserve their phase-record and
  profile banks fallibly under the build's meter and include them in budget
  admission.
- Reductions stop after cancellation (second M3b review repair). The fixed
  balanced reduction tree takes fallible leaves: a lane that observes
  cancellation keeps a stopped state, skips the rest of its leaves and stops
  every other lane at its next leaf; started lanes join, and without
  cancellation the leftmost failing leaf's error is reported. PCG dot, norm
  scale and scaled-square reductions and component-centering sums propagate
  `Cancelled` instead of treating a cancelled leaf as zero. Leaf arithmetic,
  compensation, the tree and every bit are unchanged.
- Batch lanes wider than one counted segment are processed and counted in
  lane tiles of at most 1024 lanes in every lane kernel (edge-list and CSR
  matrix products, elementwise and parallel rows, restriction, prolongation,
  pruned transfers, centering and the terminal batch solves), so the work
  between counts is bounded at any supported width. Each lane keeps its
  operation order; widths up to 1024 run exactly as before.
- Prepared numeric publication is metered: the diagonal metadata scan is
  counted, the shared handles are allocated after a poll and the frame polls
  once more after publication; topology preparation polls before each of its
  four map reserves. `PreparedLaplacianWorkspace::new_with` constructs the
  assembly scratch under a `BuildContext` (budget, cancellation);
  `new` and `new_with_budget` share it.
- `solve_pcg_with` copies its owned solution through a fallible, counted copy
  followed by the final checkpoint, so a cancellation during or after the copy
  returns no result.
- Lane kernels regain their specialized scalar loops (M3b performance repair).
  Lane-tiled kernels receive whole counted segments of rows with every lane
  (one lane tile per row only for batches wider than one segment), and the
  segment bodies are inlined so a constant width reaches the loop under a
  polling meter; the edge-list product reslices its input to the output's
  length so one bounds check covers both; row operations capture the width by
  value so a polled serial loop does not reload it after each segment. Counts,
  polls, tiles, operation order and every bit are unchanged.

- Heavy-forest selection runs on the build's team (M3c). With a team, a level
  with at least 65,536 directed entries and more than 1.5 edges per vertex
  gathers each vertex's incoming edges into a counted transient index that
  fits within the contraction slot bank the parallel bound already admits,
  and lanes select each row's maximum (weight, then lowest neighbor) in fixed
  vertex blocks, counting every scanned entry inside hub rows. Parents,
  weights, hierarchies and every bit equal the serial scan's.

### Removed

- The construction-work accounting interface: `BuildControl`, `BuildBoundary`,
  `BuildOperation`, `BuildDecision`, `BuildStopReason`, `CmgError::BuildStopped`,
  the `BuildPrefix` grant ledger and its prefix types,
  `build_component_experiment_serial_controlled_with_factor_preflight`, the
  work-metered forms of `workspace_allocation_bytes_controlled` and
  `try_workspace_controlled`, `scalar_cycle_work_bound_controlled` and
  `AdmittedStage::charged_work`.
- The superseded entry points of the consolidated pipelines:
  `Laplacian::from_edges_with_executor`, `from_edges_controlled` and
  `from_edges_hosted_controlled`; `CmgHierarchy::build_serial_with_budget`,
  `build_with_executor`, `build_with_executor_and_cmg_budget`,
  `serial_construction_allocation_bound` and
  `executor_construction_cmg_allocation_bound`; `CmgPreconditioner::
  build_serial_with_budget`, `build_with_executor`, `build_with_executor_profiled`,
  `build_with_executor_and_cmg_budget`, `build_component_experiment`,
  `build_component_experiment_serial_with_budget`,
  `build_component_experiment_serial_controlled`,
  `build_component_experiment_with_executor`,
  `build_component_experiment_with_executor_and_cmg_budget` and their
  `*_allocation_bound` companions; `GroundedLdl::factor_with_budget` and
  `factor_by_component_with_budget`; `Aggregation::contract_with_executor`;
  `apply_into_with_validation`, `apply_compatible_into_with_validation` and the
  `ParallelCmgPlan` apply methods; the 17 `solve_pcg_*_with_*` variants and
  `profile_pcg_with_plan`; `HostExecutor`, `InstalledTeam` and
  `ParallelExecutor::with_installed`; `maximum_weight_forest_with_executor` and
  `build_forest_grouping_with_executor`; `ParallelPcgSolver::executor` and
  `plan`.

There has not yet been a final tagged release. The `0.1.0` value in
`Cargo.toml` is the version currently being prepared, not evidence of a
published release.

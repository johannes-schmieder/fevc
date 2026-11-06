# CMG memory ownership and admission

The new capacity reports and narrowly scoped admission APIs supplement, rather
than redefine, `CmgMemoryEstimate`, `CmgMemoryReport`, and the repeated-PCG
principal-array interfaces. **A combined executor-aware construction/runtime
admission is still unavailable.** Serial and executor-aware CMG-owned
hierarchy/preconditioner construction and terminal factors have scoped finite bounds;
they do not change the scope of the old estimate. `complete_build_peak_bytes()` and
`ParallelExecutor::runtime_allocation_bound()` return `None`; the corresponding
strict admission methods reject before allocation. Do not use the principal
estimate or this allocation inventory as a whole-action memory certificate.

## Retained ownership

`CmgStorageReport::measure(&[&owner, ...])` walks private ownership fields and
counts the union of live allocations. It covers ordinary and component
hierarchies, terminal factors, prepared topology/current graphs, plans,
workspaces and executor handle banks. `Vec<T>` roots include spare outer
capacity as well as nested owners; passing two live banks counts their overlap.
Supported `Vec<f64>` and nested numerical banks can also be supplied explicitly
for caller-owned outputs. Owned `PcgResult`, `Vec<PcgResult>` and
`ParallelPcgBatchResult` roots include solution capacities and outer descriptor
banks; they are separate owners, not implicitly part of the workspace. Borrowed input/output slices are never implicitly
charged. Root values themselves are inline, not separate heap allocations.

The report has two disjoint categories:

* `capacity_bytes`: requested backing capacity for vectors, boxes and strings,
  including descriptor banks, compact buffers retaining larger capacity, lazy
  aggregation label/size caches, and error strings in batch workspace outcomes.
* `arc_allocation_bytes`: the Arc allocation containing its inline payload and
  two atomic reference counts, padded using `Layout`. This is the reviewed Rust
  `ArcInner<T>` model, not a promised stable standard-library ABI. It is tested
  against requested allocations on the validation platform.

Repeated references are de-duplicated by live allocation address. This includes
the finest graph shared with its hierarchy, topology/components shared by
numeric generations, and plan lineage references. Cloned topology maps are
separate allocations even though their component/lineage Arcs are shared.
No observation-sized table, hash set, or other reporting allocation is made.
The current traversal is quadratic in the number of allocation descriptors;
it never scans scalar vector payloads. Report at preparation/retirement
boundaries, not inside numerical kernels, and quiesce other users that could
initialize shared `OnceLock` caches during a report.

Do not add the old principal-array report or fereg's retained-container
supplement to this report: those bytes overlap. Include every still-live old
numeric frame, hierarchy, plan and bank in one measurement. Add independently
owned caller parents, outputs, allocator overhead and runtime resources once.
If measuring an incremental new frame, subtract the union of its already
charged parents, not each object's standalone report.

## Scoped pre-allocation admission

Budgets below refer to **requested allocation layouts**, excluding allocator
metadata, size-class rounding, retained allocator pages and OS resources. They
are not allocator hard caps. Existing parents are borrowed and excluded; the
caller must reserve them before supplying the remaining incremental budget.

| Operation | Before-allocation query and entry point | Covered storage |
|---|---|---|
| Prepared topology | `preparation_allocation_bound`, `prepare_with_budget` | indexed endpoint input, four retained maps, union/find/component scratch, new Arc blocks |
| Assembly scratch | `PreparedLaplacianWorkspace::allocation_bytes`, `new_with_budget` | maximum duplicate-weight buffer |
| Numeric generation | `numeric_allocation_bytes`, `assemble_with_workspace_budgeted` | canonical edges, diagonal, two Arc<Vec> blocks, fresh numeric lineage |
| One batch workspace | `PcgBatchWorkspace::allocation_bytes`, `new_with_budget` | PCG, CMG and component scratch, gather/guess arrays, nested descriptor banks |
| Fixed RHS bank | `bank_allocation_bytes`, `new_bank_with_budget` | every workspace plus the outer capacity bank |
| Serial CSR | `CsrLaplacian::construction_allocation_bound`, `from_laplacian_with_budget` | new row/column/weight arrays and simultaneous cursor bank |
| Parallel plan | `ParallelCmgPlan::construction_allocation_bound`, `build_with_budget` | descriptor banks, earlier retained operators, current serial/parallel CSR temporaries; **not Rayon runtime** |
| Ordinary terminal factor | `GroundedLdl::construction_allocation_bound`, `factor_with` with a budget | components, dense numeric banks, retained factor and conversion overlap |
| Component terminal factor | `component_construction_allocation_bound`, `factor_with` with a budget and component blocks | grouping, component-local sparse fill, accumulated factors, compaction/conversion |
| Hierarchy | `CmgHierarchy::construction_allocation_bound`, `build_with` with a budget | retained levels, forest, contraction, report, pruning-stage allowance; with `parallel` set, the fixed parallel mapping bank |
| Preconditioner (ordinary or component options) | `CmgPreconditioner::construction_allocation_bound`, `build_with` with a budget | hierarchy plus centering metadata, repeat bank and direct factor under the supplied `BuildOptions`; with `parallel` set, the fixed parallel mapping bank; **not Rayon runtime** |

All size arithmetic is checked, including `Layout::array`'s addressable-size
limit. Budgets are checked before the first covered allocation. Fixed banks use
fallible exact reservation and construct elements one at a time. A failed
element releases all previous elements. An old bank remains the caller's owner:
reserve old plus new bytes before replacement, then drop the old bank before
admitting another action. A fixed bank avoids the automatic solver's lazy
growth; concurrency is still selected using its existing workspace policy.

Prepared topology accepts a slice at the admitted boundary so an arbitrary
iterator cannot violate the declared input count. Its bound sums the live
indexed input `(u64, usize)[m]`, canonical keys `u64[m]`, offsets
`usize[m+1]`, two input maps `usize[m]`, and four component arrays
`usize[n]` (parents, root labels, labels, sizes). Adding the two retained Arcs
is conservative even when scratch retires earlier. No sort heap scratch is
needed by its unstable sort. Numeric assembly reuses duplicate scratch and
adds only its new graph allocations. Old frames and the topology remain
separately live. Stable Rust still lacks fallible `Arc::new`: these small Arc
allocations can abort on allocator exhaustion. The admitted byte model is not
a promise that every historical constructor has become fallible.

Workspace queries follow the same dimensions as their constructors: six PCG
vectors, seven-vector compatibility scratch, the CMG projected RHS and
compatibility scratch, per-level residual/coarse/factor vectors, coarse
centering scratch, dimension metadata and the outer banks. Two additional
vectors stage batch RHSs/guesses. Independent allocation tests check exact and
one-byte-short budgets, overlap, zero reporting allocations, spare capacities,
overflow and every injected allocation failure in fixed-bank construction.

### CSR and plan construction proof

CSR construction now fallibly reserves its known-size banks before filling;
prefix sums reuse the counts as scatter cursors. Serial peak is retained row
offsets, columns and weights plus `usize[n]` cursors. The compact/native offset
choice depends on `2*m`, independently of the compact/native column choice
depending on `n`. Both choices use checked layouts even for dimensions too large
to allocate in tests. Existing graph/lineage allocations are borrowed, not new.

Parallel construction preserves its density/thread/length routing and row
partition. Let `R` be row-offset bytes, `U=sizeof(usize)`, `N=(n+1)*U`, `E=m*U`,
`C` the new retained CSR layouts and `B` the exact row-fill descriptor bank.
The bound takes the maximum of sequential stages:

* counting/first prefix sum: `R + 3*n*U`;
* auxiliary prefix sums: `R + 2*N + 2*n*U`;
* incoming-edge scatter: `R + 2*N + n*U + E`;
* row fill: `C + 2*N + E + B`.

The row-fill descriptor count is `ceil(n / work_chunk_len(n))`; its capacity is
reserved once rather than grown. All CMG-owned allocations occur on the caller
before parallel fill. Rayon task/queue allocations remain explicitly uncovered;
the presence of a finite CMG-owned layout bound does not certify that runtime.

Plan construction reserves both level descriptor banks first, then builds levels
in their original order. At each eligible level, add its construction bound to
both descriptor banks and all earlier retained operators; take the maximum
across levels. Shared lineage clones allocate nothing. Partial failure drops all
earlier operators and both banks. The profiling-only result adds its own profile
bank and is outside this admitted entry point. Old plans remain separately live.

`ParallelPcgSolver::plan_construction_allocation_bound` and
`try_plan_with_budget` expose the same boundary inside the solver's existing
lazy-plan owner. After initialization the required incremental bytes are zero.
This avoids a second plan/pool. It does not change automatic routing or impose
admission on other, unbudgeted solver methods. Callers must initialize the plan
at their admitted preparation boundary before relying on allocation-free reuse.

Independent allocator fixtures check the serial CSR and eligible serial-CSR
plan peaks against these formulas, one-byte-short rejection before allocation,
all fallible allocation failures, repeated plan initialization and old/new plan
overlap. Dense parallel CSR numerical identity is checked separately. Runtime
activity is not silently subtracted from a sampled parallel process peak.

### Terminal-factor construction proof

Both admitted factor APIs check the budget before even component discovery.
They use fallible exact reservation for every buffer; no Arc or worker is
created. The stable sort is replaced only in these opt-in APIs by an unstable
sort with the same unique key (degree, original vertex; additionally component
for the block variant). Unique original vertex IDs prove identical ordering.
All LDL arithmetic, pivot checks and packed/sparse selection rules are unchanged.

Let `n` be vertices, `m` edges, `U=sizeof(usize)`, `q=n*(n-1)/2`, and
`D=sizeof(Vec<f64>)`. Bounds use checked layouts and products, not floating
fill estimates. Ordinary construction takes the maximum of component discovery
(`4*n*U`) and the simultaneous factor stage:

* six native metadata banks (component labels/sizes, anchors, permutation,
  pattern, factor-index), plus `n` anchor flags;
* two dense row banks: `16*n*n + 2*n*D`, plus `8*n` pivots;
* sparse conversion: `2*(n+1)*U + n*U + 24*q`, which also bounds the alternative
  packed conversion. Counts retire before cursors, so one `n*U` allowance
  suffices for either. Dense rows remain live during conversion.

The component constructor has common parents `P` consisting of four native
metadata banks; two component-offset banks; two native vertex banks and one
edge-index bank for grouping; and the accumulated factor's pivots, row offsets,
column indices and values. Actual fill reservations use
`sum_c triangular(size_c-1)` and each current block's triangular dimension,
never a dense matrix across components. The public bound replaces those
dimensions by `n`, making it conservative even before discovering components.
To `P`, add the maximum of:

* input reorder: `(n+1)*U + m*U + n*U`;
* ordered sparse updates: `2*(n+1)*U + m*U + 2*n*U + 16*n + (16+U)*q`;
* packed conversion or one fallible exact-compaction copy: `8*q`;
* sparse transpose: `(n+1)*U + n*U + 12*q`.

Finally take the maximum with discovery (`4*n*U`) and ordering preparation
(`5*n*U+n` flags). Entry banks, row links, and accumulated factors are reserved
before their pushes; fill cannot exceed strict triangular slots. Compaction
copies fallibly before releasing the old bank, and the bound includes both.
This opt-in provisioning can reserve much more than actual sparse fill;
`CmgStorageReport` reports the resulting real capacities. It is an admission
bound, not a claim that sparse setup requires these pessimistic bytes.

`tests/factor_admission.rs` independently observes live requested layouts,
checks exact/one-byte-short admission, compares full factors to the ordinary
constructors, checks old/new owner overlap, and injects every allocation failure
on these Arc-free paths. The test includes isolates, paths, cycles, dense blocks
and a genuine weak bridge with many pairs. This test's numerical identity check
does not replace the separate coefficient/residual oracle.

### Explicitly serial hierarchy/preconditioner proof

These are opt-in **serial setup** APIs. Existing executor-aware builders keep
their parallel contraction routing; there is no misleading executor argument
that is silently ignored. The admitted path shares their hierarchy control
flow, pruning/stopping choices, forest arithmetic, contraction duplicate sums,
terminal-factor selection and repeat-count finalization. It changes provisioning,
not the coarsening or numerical policy. Prepared numeric graphs reuse their
existing component Arc. No duplicate topology, worker pool or setup is built.

Let `L=options.max_levels` (default 32, also the hard construction stop), `G=m*sizeof(Edge)+8*n+A`, where `A` is the reviewed
sum of `Arc<Vec<Edge>>`, `Arc<Vec<f64>>` and lineage `Arc<()>` allocations.
Every current/coarse graph has at most `n` vertices and `m` edges. Define the
retained hierarchy ceiling `H` as:

* `L*sizeof(HierarchyLevel)` plus two report banks `2*L*U`;
* `(L-1)*G` newly owned graphs (the finest graph is a borrowed Arc clone);
* `8*n*L` inverse diagonals and `n*L*max(U,8)` transfer storage.

The last term covers compact/native full labels or pruned `(u32,u32)` entries;
lazy public aggregation-label/size caches are not initialized by construction.
Each level, including an early terminal with a discarded aggregation, is
covered. The method does not use the cumulative-fill guard as a hard cap, so
crossing levels and all supported stopping reasons are included.

Hierarchy peak is `H + max(F, C, M, P)`:

* `F=(4*U+24+sizeof(bool))*n`: heavy and split parents, native-width indegrees,
  walk, selected weights, two i64 ancestor banks, visited flags. Labeling and
  low-degree correction are smaller sequential stages. The walk has at most
  `n` distinct vertices: the maximum-weight/lowest-neighbor forest has no
  directed cycles longer than two, and the traversal stops before repeating a
  self/two-cycle endpoint. No general user-supplied functional graph is admitted
  through this private fixed-capacity kernel.
* `C=(U+4)*n`: simultaneous native labels and compact conversion.
* `M=G+m*sizeof(Edge)`: mapped edges coexist with exact canonical compaction,
  diagonal and Arc finalization. Endpoint and weight sorts use no heap scratch.
* `P=2*G+2*n*U+16*n`: unpruned/compact children, isolate map, still-live full
  aggregation and both old/new pruned-transfer entry banks. This deliberately
  covers a non-suffix isolate removal even when suffix removal shares edges.

The retained ceiling includes all previously completed levels in each stage;
the common ceiling is added once to the maximum, not substituted for scratch.
Maximum-level provisioning can be pessimistic, but occurs only on explicitly
admitted paths. It rejects overflow before allocation. Each Vec has a proved
fixed capacity; cleanup on any fallible error drops all earlier partial owners.

For complete serial preconditioner setup, define `B` as centering labels/sizes
(`2*n*L*U`), one finest-component Arc, coarse-plan descriptors (`L-1`), and `L`
repeat counts. Finalization peak is `H+B+max(4*n*U,T)`, where `T` is the selected
terminal-factor bound at `min(n,max(1,direct_threshold-1))` and at most `m` edges.
Only a Direct terminal factors; the other stopping reasons retain diagonal
iteration. Take the maximum of this finalization stage and hierarchy peak.
Component discovery/compact conversion fit the `4*n*U` scratch allowance;
earlier centering plans coexist and are already in `B`. Factor return/conversion
overlap is included in `T`. Old preconditioners or failed attempts independently
retained by callers are NOT deducted: reserve their union before this incremental
budget, and keep them charged until actually dropped.

`tests/hierarchy_admission.rs` exercises every stopping reason, thresholds and
fill boundaries, ordinary and both component stopping policies, prepared
changing-weight frames, shared-parent/old-new overlap, zero-allocation reports,
overflow and independently observed peak layouts. Failure injection explicitly
skips sizes used by infallible Arc allocations (and coincidentally equal Vec
sizes); it therefore claims selected-vector failure cleanup, not exhaustive OOM
recovery. All vectors are fallibly provisioned, but stable `Arc::new` can still
abort on true exhaustion. There is no allocator cap, and no promise of complete
process memory control or of a stable Arc ABI. Peak/failure fixtures must be
executed on the exact candidate; compilation alone is not validation.

The serial APIs do not admit ordinary iterator ingestion before a `Laplacian` exists,
parallel `filter_map().collect()` contraction, runtime task queues, profiling
banks, later lazy accessors, solves or workspaces. For a caller choosing serial
setup, use the matching bound and serial constructor, then separately admit its
plan/workspace banks. Choosing serial setup is an explicit downstream policy
decision, not a recommendation to replace fereg's parallel setup automatically.
Executor-aware complete construction/runtime admission remains a blocker to
strictly budgeted parallel adoption.

### Executor-aware CMG-owned construction proof

The new executor-aware methods are explicitly named `cmg_budget`: they do not
claim runtime coverage. A caller with a strict combined/runtime policy must
still reject this route. The existing executor, graph and all caller-retained
parents are borrowed and must already be charged. There is no new pool or
duplicate hierarchy. Ordinary unbudgeted constructors are unchanged.

The admitted contraction preserves the existing mapping and sorting thresholds.
Before dispatch it fallibly provisions `Option<Edge>[m]` and `Edge[m]` banks.
Each worker writes a disjoint input-indexed slot, with no CMG-owned worker
allocation. Serial gathering appends at most `m` surviving edges in original
input order into the already reserved bank; the slot bank is then dropped.
The unchanged endpoint/weight comparator and compensated duplicate summation
preserve canonical numerical order. Rayon's unstable sort is in-place
(`rayon-1.12.0/src/slice/sort.rs::par_quicksort`); its recursive join/runtime
allocations remain outside this scope. Graph compaction and finalization use
the fallible admitted serial allocation machinery.

Consequently `serial_bound + m*sizeof(Option<Edge>)` is a conservative bound
for hierarchy or complete preconditioner CMG-owned construction. The serial
contraction allowance already covers the `Edge[m]` bank and old/new graph
compaction overlap. Adding the slot bank even to the largest *other* stage
overcounts sequential storage safely. All additions/layouts reject overflow;
one-byte-short budgets reject before forest, factor or mapping allocation.
Factor, pruning, stopping, centering and maximum-level proofs are unchanged.
Plans and solve banks remain separately admitted operations.

`tests/parallel_construction_admission.rs` checks one/four workers, both sides
of the map/sort crossover, exact/short budgets, overflow, ordinary/component
policies, prepared weight changes, ownership overlap and failure of each fixed
mapping bank before contraction dispatch (forest dispatch may precede it).
Failure cleanup tracks caller-origin allocation pointers in a fixed,
allocation-free test ledger; frees and reallocations on any thread update
the originating cohort. Exactly zero outstanding allocations and no ledger
overflow are required after failure. Cross-thread transfer/reallocation and
deliberately retained-allocation controls check that accounting. Thread-local
net allocation/free bytes cannot establish cleanup after ownership transfers.
An independent process-wide allocator records
live peaks including runtime activity. Those observations are **not compared
to a purported whole-runtime bound** or relabelled as CMG-only peaks. Retained
CMG layouts are checked separately. Serial peak/failure fixtures still cover
the shared admitted kernels. The Mac review on 2026-10-20 retained a failed release attempt: the old
thread-local assertion read 3,229,314 bytes against 3,229,378 before the failure
(a negative 64-byte delta, not evidence of a leak). Allocation and deallocation
were attributed to their executing threads, so cross-thread ownership invalidated
that comparison; the individual 64-byte owner was not identified. After the
origin-tracker repair, the release/all-features suite passes 254 executions in
38 groups, the focused debug cleanup test passes, and all-target/all-feature
Clippy with warnings denied passes on Rust 1.85.1. Production source is unchanged.
The reallocation control permits an in-place resize; a moved pointer is covered
by source review, not guaranteed by that control. Compact log hashes and the
retained failed attempt are recorded in fereg's M3c validation evidence.

## Fill guard proof and remaining construction scopes

The builder adds the current level's nonzeros before comparing the cumulative
count to `factor * initial_nnz`. It can therefore retain a crossing level.
Direct and full-contraction terminals can also exit before that comparison.
Every contraction preserves nonnegative graph weights and cannot increase
nonzeros, so one additional finest-sized level bounds the crossing/unchecked
terminal. The estimate rounds the floating threshold outward and also applies
the independent `max_levels * initial_nnz` bound. A path fixture with fill
factor one demonstrates actual `StagnatedFill` and a retained sum larger than
the old factor-only bound. This establishes the defective sub-bound; it does
not claim that every old aggregate estimate was exceeded on that fixture.

The builder's nonzeros exclude isolated diagonals. Consequently nonzeros do
not bound the vertex vectors of levels retaining isolated representatives.
The estimate uses `vertices * max_levels` for that separate dimension; pruning
only reduces it. This is deliberately pessimistic, especially for connected
graphs, and can reject more work than the previous estimate. Principal
hierarchy allowances cover inverse diagonals, component metadata and both
aggregation representations/caches. Finest workspace allowances include both
CMG and PCG compatibility scratch; they must cover all-isolated inputs even
with `max_levels=1`. Zero-thread requests resolve available parallelism rather
than being estimated as one worker. For reproducible admission, pass the
already chosen explicit thread count.

Prepared keys and group offsets retain their input-sized reserved capacities,
even when many duplicate inputs canonicalize to one edge. The repeated estimate
now uses input counts for those two maps; a duplicate-rich regression would
exceed the previous canonical-count sub-bound.

The proof applies to positive finite fill factors and every termination reason
(direct, full contraction, vertex stagnation, fill stagnation, level limit),
including the preserved-stopping and active-count experiments. It changes no
stopping decision, repeat count or numerical reduction.

The following inventory identifies **unresolved peak scopes** rather than
certifying a whole-build bound:

| Stage | Simultaneous allocations that a full bound must cover |
|---|---|
| Ordinary edge ingestion | iterator-owned input, growing raw edge bank, diagonal, cached endpoint-sort keys, old/new buffers during growth/shrink, Arc finalization |
| Legacy hierarchy builders outside admitted serial entry points | historical growth and compact-conversion paths are not governed by the new budget |
| Legacy parallel contraction | dynamically collected result/chunk banks; the new `cmg_budget` path covers fixed CMG banks, but both paths exclude runtime tasks/queues |
| Legacy LDL entry points | data-dependent growth/stable-sort allocation outside the admitted factor methods; admitted factor scope is covered above |
| Plans outside scoped entry points | profiled-result banks, concurrent independent callers and Rayon runtime; new ordinary-plan CMG layouts are covered above, not full solver construction |
| Automatic repeated solves | old workspace bank plus newly allocated descriptors/workspaces, lazy plan setup, owned-result banks and per-chunk Result buffers |
| Failure/retry | already retained parents, partially built current owners and any separately retained failed attempt; errors do not make those parents disappear |

Sequential stages use a maximum only **after** adding their common live
parents. Replacements and final factor conversion require sums. The existing
principal estimate does not prove these full overlaps; the downstream
construction supplement remains incomplete too. The serial and terminal-factor
APIs now replace their covered growth paths with bounded fallible provisioning.
The admitted parallel collection now has fixed CMG-owned banks; a bounded runtime
design and ordinary ingestion boundary remain necessary. The proof does not transfer to legacy
or parallel methods just because they share arithmetic. No weak-graph solver
alternative was removed to simplify the inventory.

## Runtime scope and lifetime

### Optional estimated-runtime policy

Callers may explicitly accept engineering headroom instead of a certified
runtime bound. `ParallelRuntimeAllowance::default()` charges 32 MiB per pool
plus 2 MiB per worker for runtime heap, and 8 MiB per worker for stacks. These
are initial, configurable rules of thumb, not calibrated empirical bounds.
`allowance.estimate(options)` separates known join-handle layouts, heuristic
runtime heap and requested stacks, with checked arithmetic. At 16 workers the
heap-plus-stack allowance is 192 MiB, plus the small known handle bank.

`ParallelExecutor::new_with_estimated_memory_budget(options, allowance, budget)`
checks that total before pool/handle allocation and returns `(executor, estimate)`.
It fixes an auto-detected worker count once and explicitly requests
`allowance.worker_stack_bytes` for every worker. This overrides the platform or
`RUST_MIN_STACK` default for this constructor only: set a larger explicit value
if the application requires larger stacks. Native rounding, guard pages, TLS,
allocator caches and actual residency still require whole-process headroom.
It installs no hard allocator cap and makes no promise to prevent OOM.

Keep the returned estimate with the owning solver's memory-policy metadata.
Before creating a new pool, charge its entire `total_bytes()`. Once its handles
are included in a `CmgStorageReport`, add only the separate runtime-heap and
stack allowances; otherwise the handle bank is counted twice. Do not also add
a host's charge for these same worker stacks. Charge two live pools separately;
reuse a pool where possible, and keep allowance charged through joined shutdown.
Process-global deferred garbage may survive retirement, so retain process
headroom and check representative long-running workloads, not just one pool.

The strict constructor and `runtime_allocation_bound()` are unchanged and still
reject/return `None`. `runtime_heap_unbounded()` stays true even for an estimated
pool: reporting does not disguise uncertainty. Applications choosing this
explicit estimated policy can proceed without a bounded-executor rewrite,
subject to numerical/integration testing and observed process memory. Ordinary
construction/default routing do not automatically opt into the policy.

The lockfile pins Rayon 1.12.0, rayon-core 1.13.0, crossbeam-deque 0.8.7,
crossbeam-epoch 0.9.20 and crossbeam-utils 0.8.22. Rayon creates worker and
broadcast deques, registry/thread descriptors, sleep state and an injector.
Crossbeam workers begin with 64 slots, dynamically resize, and defer old-buffer
reclamation through the epoch collector. The injector adds linked blocks as
jobs arrive. Public thread-count/stack options do not provision or cap those
heaps. Thread spawn adds native runtime resources and TLS outside these layouts.
Thread joining does not force all process-global deferred reclamation or
allocator caches to return memory to the OS.

Concretely, rayon-core 1.13.0 `src/registry.rs` constructs private normal and
broadcast Crossbeam Workers for each thread and a private Injector. In
crossbeam-deque 0.8.7 `src/deque.rs`, `MIN_CAP=64`; `Worker::push` doubles a full
buffer, and `resize` allocates a new buffer while deferring the old buffer through
`guard.defer_unchecked`. crossbeam-epoch 0.9.20 `src/default.rs` uses a
process-global collector and thread-local participants. An unrelated epoch
participant can delay reclamation after this pool has joined. Public Rayon
`ThreadPoolBuilder` exposes neither fixed queue provisioning, allocation hooks,
per-pool epoch isolation nor a no-growth mode. A finite `threads*constant` claim
would therefore be unsupported. Implementing a genuinely bounded pool requires
owning/forking both scheduling and reclamation or replacing execution machinery,
then separately proving the required scheduling/reduction behavior. This change
does not introduce that scheduler replacement.

The executor report counts only its CMG-owned join-handle capacity and sets
`runtime_heap_unbounded=true`. Strict runtime admission rejects before spawning
any worker. The ordinary constructor remains available under an explicitly
unbounded-runtime policy. Joined shutdown supplies an actual worker lifetime
boundary; it does not resolve the runtime peak. Introducing a bounded executor
would require a separately validated scheduling/provisioning design. No dynamic
allocator hard limit is installed, because ordinary infallible allocations
could otherwise abort the host.

This work is not sufficient to recommend memory-certified fereg adoption.
Retained container reconstruction can be replaced by the new union report, and
workspace/prepared-stage supplements can use the scoped APIs. Full setup,
runtime, allocator/OS and all fereg-owned construction/inference/recovery/
publication scopes still require their own coverage. Historical component
performance gates and downstream development gates remain separate.

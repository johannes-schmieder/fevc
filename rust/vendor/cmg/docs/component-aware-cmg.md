# Component-aware CMG

The `experimental-components` feature retains opt-in coarse-isolate pruning and
component terminal factors used by fereg. Ordinary builders do not enable these
options automatically. CMG 0.2.0 removes the unused tiny-component portfolio API
and its dedicated tooling; it does not promote abandoned experimental defaults.

The default maximum hierarchy depth is 32. Consumers retaining the earlier
policy set `CmgOptions { max_levels: 128, ..Default::default() }` explicitly.
Construction and memory admission both use the selected value.

## Current approach

Start with two independent changes that retain one global PCG recurrence:

1. Remove **exactly isolated coarse vertices after contraction**. Preserve every
   parent row and its pre/post smoothing. A collapsed component's coarse range
   has dimension zero; removing that coarse degree of freedom does not mean the
   original component has been solved exactly.
2. Factor direct terminals with ordered sparse updates, one component at a time,
   including connected terminals. Keep the same anchor and relative degree
   ordering within each block, and store the final factors in shared buffers.
   One-vertex grounded blocks use their scalar pivot without a dense allocation.
   No heavy solver or executor is constructed per component.

All experimental options are under `experimental-components` and are off by default:

```rust
use cmg::{BuildContext, BuildOptions, CmgOptions, CmgPreconditioner, ComponentBuildOptions};
let preconditioner = CmgPreconditioner::build_with(
    graph,
    BuildOptions {
        hierarchy: CmgOptions::default(),
        components: ComponentBuildOptions {
            prune_coarse_isolates: true,
            factor_terminal_components: true,
            preserve_unpruned_stopping: true,
        },
    },
    BuildContext::default(),
)?;
// Use the certified solve_pcg_with / solve_pcg_batch_into interfaces.
```

Keep baseline, pruning-only, block-LDL-only, and combined builds available. These
separate reduced vector work, changed hierarchy/terminal behavior, and dense
factorization work.

## Why this addresses the observed pathology

The original direct threshold uses the complete level vertex count. A graph
with 1,001 connected components can never drop below the default threshold of
700 while retaining one representative per component. A material component
therefore continues coarsening, with tiny representatives scanned repeatedly.
Recursive repeats multiply these scans even when setup and PCG iteration counts
look ordinary.

For the 4,096-vertex path plus 1,000 independent pairs, the baseline dimensions
are `6096, 2024, 1256, 1064, 1016, 1004, 1001`. Active-count pruning gives
`6096, 1024, 256`. The baseline's nonterminal cycle-weighted vertex count is
502,592, of which 484,000 represent isolated vertices. The candidate count is
10,192 with no isolated vertices at nonterminal coarse levels. This is a work
model, not a count of all arithmetic instructions or all finest-level scans.

Block factorization addresses an independent threshold cliff. With 349 pairs
the baseline immediately factors a 349-dimensional grounded matrix; with 350
pairs it coarsens first and factors an empty grounded matrix. Per-component
factorization reduces dense arithmetic from `O((sum d_c)^3)` to
`O(sum d_c^3)`, with dense temporary storage bounded by the largest block.
It does not remove finest-level PCG operations.

## Transfer and numerical contract

`Aggregation` continues to describe a complete partition with valid coarse
indices for every fine vertex. A pruned level instead holds `PrunedTransfer`, a
flat ordered list of `(fine_vertex, active_coarse_vertex)` entries. It retains
both the original aggregate count and the active child dimension. The full
aggregation is released; there are no sentinel indices presented as ordinary
coarse vertices and no inactive child vectors.

`HierarchyLevel::aggregation()` is `None` on a pruned level; inspection code must
check `pruned_transfer()`. `graph()` and the existing vertex-count report always
describe the actual stored graph. The transfer exposes the pre-pruning coarse
count separately. The experimental benchmark handles both kinds; existing SCC
exporters have not been extended to accept this experimental representation.

If `E` selects nonisolated coarse vertices, the transfer is `R_a = E R`, its
prolongation is `R_a^T`, and the child graph is exactly `R_a L R_a^T`. Only whole
isolated coarse vertices are removed, using exact zero degree, never a weight
threshold. Within each surviving aggregate the summation order is unchanged.
The compact graph preserves vertex order and positive edge values.

The existing symmetric pre/post damped-Jacobi cycle, coarse component centering,
and fixed recursive repeat counts remain in use. Active-count pruning changes
which terminal is reached and can change repeat counts; equality to the old
preconditioner is not promised. No inner tolerance-stopped solve, RHS-dependent
route, timing probe, or flexible preconditioner is introduced. Positivity and
symmetry are on the component-compatible quotient space; intermediate grounded
corrections need not have zero mean. Final PCG centering and original-system
certification remain authoritative.

The block factor's exposed permutation is grouped by deterministic component
order. Its local degree order and highest-index anchors match the baseline.
The component builder now also uses ordered sparse factorization for connected
terminals; the ordinary builder retains its reference dense factorization.
New sparse factor buffers
discard spare capacity before retention. Pruned-transfer retained storage is
included in hierarchy reports; ordinary workspace sizing follows actual compact
child dimensions. The workspace budget still has its documented meaning: it
does not admit total process RSS or every setup allocation.

## Validation and experiment

`tests/component_experiment.rs` covers:

- connected/disabled hierarchy and application identity;
- expected retirement on the mixed regression fixture;
- transpose and Galerkin identities for partial transfers and dimension errors;
- range linearity, symmetry, positivity, and dirty-workspace reuse;
- empty/all-isolated graphs, empty coarse levels, and incompatible isolated RHSs;
- grounded reference agreement with interleaved components;
- independently certified known solutions and heterogeneous component weights;
- no correction leaking into an untouched component;
- prepared numerical generations, current-operator certification, and rejection
  of unrelated topology lineage;
- deterministic planned execution with 1/2/4 threads and a single-workspace
  concurrency budget.

`component-bench` supplies connected paths/grids, material-plus-pairs,
material-plus-isolates, two material components, all-pairs, all-triangles, and
both sides of the direct threshold. It rotates the selected arms and records raw
paired samples after two warm-up rounds. Total includes setup, PCG workspace
allocation, solves, normal certification, and result allocation. Extra external
verification is performed for every sample. The current harness adds a fifth
arm preserving the original stopping decisions. Structural work and retained
bytes are reported separately; requested-allocation counters use a separate
build. Process RSS and allocation-failure recovery remain unqualified.

See `benchmarks/README.md` for the exact command. Small local results are
development evidence, not production thresholds or cross-platform speed claims.


## Host integration and memory

Pass a borrowed `Team` through `BuildContext`; construction creates no worker
pool. `CmgPreconditioner::construction_allocation_bound` admits CMG-owned
requested layouts. Host runtime, stacks, caller buffers and other live owners
remain separate charges. Principal-array estimates are not whole-process caps.
See [memory ownership](memory-ownership.md).

`ParallelPcgSolver::from_preconditioner` retains a hierarchy and a lazy plan for
that team's shape. Initialize a budgeted plan with `try_plan_with` or
`try_plan_with_budget`, then prepare the admitted workspace bank. Carry the same
team, optional plan and cancellation query through `SolveContext` and
`ApplyContext`. Do not build a second pool or plan beside the prepared solver.

For changing weights retain `PreparedLaplacianTopology` and its lineage, and use
the checked numeric-assembly interfaces. Explicit retained-preconditioner solves
certify against the current graph; the consumer still owns refresh and routing.

## Historical qualification

The original [development evidence](https://github.com/johannes-schmieder/CMG/blob/5c5b5b2b138226ea928514fa9db27439ba1cffdb/docs/component-aware-cmg.md)
retains its exact source identities, results and limitations. Large disconnected
fixture gains do not establish end-to-end consumer gains. Inconclusive connected
controls remain inconclusive; no unconditional default promotion is implied.

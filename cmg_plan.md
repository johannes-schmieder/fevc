# Pure-Mata CMG-Inspired Preconditioner Plan

## Header

- Plan ID: `CMG-MATA-V1`
- Milestone series: `CMG0`--`CMG11`
- Branch/worktree: `main`, current worktree
- Owner authorization: implement the reviewed CMG plan, 2026-08-14
- Status: active
- Runtime: Stata/Mata 18 and 19 only
- Protected paths: `archive/`, `paper/`, `paper/releases/`, `theory/`,
  `proof-audit/`, `state/`, `application/`, and every imported upstream CMG source

## Goal and reviewed decisions

Build a clean-room, pure-Mata, CMG-inspired symmetric preconditioner for the
weighted two-way fixed-effect Schur systems used by `ppml_talo` and `kss_bc`.
The estimator contracts, retained samples, deletion rules, targets, probes,
random streams, tolerances, failure semantics, and full residual gates do not
change.

Five independent read-only reviews covered the mathematics, Mata performance,
`ppml_talo`, `kss_bc`, and adversarial benchmarking. Targeted second reviews
resolved the main disagreements. Their binding conclusions are:

- implement an exact degree-three hybrid graph;
- call the solver a clean-room CMG-inspired V-cycle, not a port or exact
  reproduction of the GPL implementation or the full KMT multi-call recursion;
- retain diagonal-PCG as the easy-graph route and rollback;
- benchmark true KSS batching separately from CMG;
- preserve KSS's zero-sum quotient and use an adjoint grounded pullback for
  PPML;
- finish automatic routing before estimator RNG initialization; and
- generate package-specific Mata namespaces from one canonical source.

The mathematical sources are Koutis, Miller, and Tolliver, *Combinatorial
Preconditioners and Multilevel Solvers for Problems in Computer Vision and
Image Processing*, and Koutis and Miller, *Graph Partitioning into Isolated,
High Conductance Clusters*. Implementation authors must not inspect, copy, or
translate the imported GPL CMG implementation under `application/`.

The active KSS KB5/KB6 work must finish or be explicitly superseded before an
ownership transfer permits edits to overlapping KSS files. CMG is a new solver
milestone series and does not alter prior qualification evidence.

## Mathematical architecture

### Exact degree-three hybrid

Collapse observations to unique positive worker--firm cell weights

\[
a_{wf}=\sum_{i:w(i)=w,f(i)=f}W_i,\qquad d_w=\sum_f a_{wf}.
\]

The firm Schur complement is

\[
S=\sum_w\left\{\operatorname{diag}(a_w)-\frac{a_wa_w'}{d_w}\right\}.
\]

For worker degree `k_w`, construct the fine preconditioner graph as follows:

- `k_w=1`: store no edge; the Schur contribution is zero;
- `k_w=2`: store one firm edge with weight `(a_wf/d_w)*a_wg`;
- `k_w=3`: store all three firm-clique edges;
- `k_w>=4`: retain one auxiliary worker vertex and its star edges with weights
  `a_wf`.

Eliminating retained auxiliary vertices recovers `S` exactly. If `C` is the
number of unique worker--firm cells, the fine graph satisfies

\[
E_0\le C,\qquad V_0\le F+C/4.
\]

The full explicit clique, full original bipartite graph, and a multilevel
implicit hypergraph are oracle or deferred designs, not v1 production paths.
The packages' current exact Schur actions remain the outer PCG operators until
a separate collapsed-cell equivalence and performance gate passes.

### Deterministic CMG-style hierarchy

At each ordinary weighted graph level:

1. Compute degree `d_v`, maximum incident weight `m_v`, weighted degree
   `d_v/m_v`, and its level mean `awd`.
2. Give every edge the strict key `(descending weight, ascending canonical
   endpoints)`. Each vertex nominates its highest-key incident edge. The union
   is a forest.
3. With `kappa=8`, remove a high-weighted-degree vertex's nominated edge when
   `d_v/m_v > 8*awd` and its forest incident volume is below `d_v/awd`.
4. Partition each tree deterministically. Start from the smallest canonical
   unassigned key, grow over the highest-key forest boundary until reaching
   four vertices or exhausting the boundary, and merge residual clusters
   smaller than four when the union has at most eight vertices.
5. For every proposed cluster `C`, enumerate its nontrivial cuts and compute

   \[
   \psi(C)=\min_{\emptyset\ne A\subsetneq C}
   \frac{w(A,C\setminus A)}
        {\min\{\operatorname{vol}_G(A),
               \operatorname{vol}_G(C\setminus A)\}}.
   \]

   If `psi(C) < 1/(8*awd)`, split at the lexicographically first minimizing
   cut, take the forest-connected components on both sides, and recurse.
   Singletons pass by convention.
6. Number aggregates by their minimum canonical fine-vertex key.
7. Use binary indicator prolongation `P`, restriction `P'`, and exact Galerkin
   contraction `K_next=P'*K*P`. Delete self-loops, lexicographically sort
   endpoint pairs, and collapse duplicates deterministically.

Abort hierarchy construction before unsafe allocation if:

- `V_next > 0.8*V`;
- cumulative edges exceed `3*E0`;
- cumulative vertices exceed `4*V0`;
- levels exceed 32;
- component counts change;
- a positive edge underflows to zero or a sum becomes nonfinite;
- a terminal component remains above 128 vertices; or
- predicted construction scratch exceeds 1.5 GiB or 25% of the registered
  process/job memory envelope.

No edge sparsification, ridge, eigenvalue clipping, generalized inverse,
silent component selection, or tolerance relaxation is permitted.

### Symmetric quotient-SPD V-cycle

Let `C_l` contain component indicators and `D_l=diag(K_l)`. Define

\[
R_{0,l}=D_l^{-1}-
D_l^{-1}C_l(C_l'D_l^{-1}C_l)^{-1}C_l'D_l^{-1},
\qquad R_l=\frac23R_{0,l}.
\]

Use one fixed pre-sweep, one recursive coarse application, and one identical
post-sweep from a zero initial state. Algebraically,

\[
B_l=2R_l-R_lK_lR_l+
(I-R_lK_l)P_lB_{l+1}P_l'(I-K_lR_l).
\]

This is linear, symmetric, and positive definite on the component quotient.
At the coarsest level, select one stable ground per component, diagonally
equilibrate each grounded principal minor, factor it once by Cholesky, and use

\[
B_c=\Pi E(E'K_cE)^{-1}E'\Pi.
\]

Compatibility projections occur on both sides. Cumulative dense-factor
storage is capped at 64 MiB. A singleton zero-edge component accepts only a
zero compatible RHS. Any failed pivot, nonfinite factor, or failed factor
residual makes CMG unavailable.

The v1 cycle uses exactly one recursive child application. The full published
KMT recursion sometimes uses multiple child corrections; implementing those
would require a new linearity, symmetry, and positivity proof.

### Package coordinate pullbacks

Let `Q` inject firm coordinates into the hybrid graph with zeros on auxiliary
nodes.

For KSS use

\[
B_{KSS}=\Pi_FQ'B_KQ\Pi_F.
\]

The solver remains on the full firm quotient and applies the displayed firm
ground only after convergence.

For PPML, let `E_g` insert a zero at the current ground and

\[
J_g=E_g-e_g\mathbf 1'.
\]

Use

\[
B_{PPML}=J_g'Q'B_KQJ_g.
\]

Plain coordinate extraction is nonsymmetric and forbidden. A three-firm path
counterexample must be registered as a unit test.

## Automatic routing

Routing is per frozen weighted system. PPML's full- and positive-information
systems route independently and may share only immutable topology/index maps.

Immediately select diagonal when `F<256`, fewer than eight inverse-action RHSs
are planned, canonical ID-free vertex keys are unavailable, normalized weights
are unsafe, or the hierarchy memory forecast fails.

The planned KSS count is

\[
3R+k+1+\mathbf1\{\text{fixed offset and }k>0\},
\]

independent of `batch()`. PPML passes its actual planned count separately for
each weighted system.

Otherwise construct four compatible pilot RHSs in canonical firm order:

1. a centered linear ramp;
2. centered alternating signs;
3. Park--Miller signs with seed 1;
4. Park--Miller signs with seed 48,271.

Use the exact overflow-safe Park--Miller recurrence

\[
s_{t+1}=16807s_t\bmod 2147483647.
\]

Center within each component, correct the final coordinate so the stored sum
is exactly zero, and scale by maximum absolute value. This must not read or
advance Stata's RNG.

Run diagonal-PCG from zero with caps 128 for 8--31 planned RHSs, 64 for 32--127,
and 32 for 128 or more. If every pilot reaches the requested tolerance, select
diagonal without building CMG.

Otherwise build and validate CMG, then continue diagonal and run CMG pilots to
`min(maxiter(),250)`. Select CMG only when every CMG pilot converges, it reduces
pilot iterations by at least fourfold or diagonal remains capped, and the
calibrated setup-plus-solve work score is at most 80% of diagonal work.
Routing never uses wall-clock timing, estimator probes, `seed()`, or `batch()`.

Setup or pilot failure falls back before RNG initialization. A deterministic
pre-RNG CMG failure rebuilds dependent solver state under diagonal. After RNG
initialization the route is locked; a CMG breakdown or failed full residual
withholds rather than switching paths.

## Implementation interfaces and performance rules

The canonical source lives under `shared/cmg/` and is assembled into
package-local monolithic Mata artifacts. Package instances use
`ppmltalo_cmg__*` and `kssbc_cmg__*`; a future unified package may use
`vch_cmg__*`.

Canonical internal structures are `cmg_cells`, `cmg_level`, `cmg_hierarchy`,
`cmg_workspace`, and `cmg_apply_result`. Canonical entry points are:

```text
@CMG_NS@__cells_prepare(...)
@CMG_NS@__preflight(...)
@CMG_NS@__hierarchy_build(...)
@CMG_NS@__workspace_init(...)
@CMG_NS@__apply(...)
@CMG_NS@__diagnostics(...)
```

Every symbol is namespaced and no Mata global is permitted. The core does not
select samples, generate probes, apply targets, reconstruct worker effects,
manage controls, choose deletion units, alter tolerances, or certify final
estimator convergence.

Mata implementation rules:

- store canonical undirected edges with `u<v`; never use an arithmetic pair
  key in a double;
- normalize weights by one recorded power-of-two scale and undo the scale in
  the returned preconditioner;
- compute clique weights as `(a_wf/d_w)*a_wg`;
- keep interpreted loops out of every PCG/V-cycle edge pass;
- preallocate and reuse level workspaces;
- cap transient edge-action chunks at 64 MiB;
- use independent scalar PCG recurrences sharing matrix operations, not block
  CG;
- keep inactive columns exactly zero;
- use per-column max-absolute scaling, robust norms, high-accuracy dot
  products, residual replacement every 100 iterations with recurrence restart,
  and explicit nonfinite/nonpositive-curvature checks; and
- require every accepted RHS to pass the package's existing complete
  worker-plus-firm residual gate.

Benchmark directed-arc and two-reduction edge kernels, `panelsum()` variants,
stored-row and collapsed-cell actions, workspace reuse, broadcast scaling, and
coarse solves at 32/64/128/256. Freeze the fastest bounded-memory kernel per
supported Stata version. Stop if neither avoids interpreted hot loops or
bounded scratch.

## Standalone assembly and runtime API

Use one LF-normalized UTF-8 template whose only transformation is replacement
of `@CMG_NS@`. Generated headers record the canonical template SHA-256,
generator API, namespace, and generated-section SHA-256.

The assembler supports `--target`, `--all`, and `--check`. Check mode regenerates
under a temporary directory, reverse-substitutes the namespace, and requires
byte identity. Release builders check but do not regenerate.

Both ado loaders require exact equality of the package API, build ID, CMG API,
template hash, and generated hash. A loaded mismatch returns
`STALE_MATA_RUNTIME` and requires `discard` or restart.

After qualification, expose `preconditioner(auto|diagonal|cmg)`. Forced CMG
never silently falls back. Auto becomes the default only in a package that
passes every routing gate; otherwise diagonal remains the default.

## Milestones

| Milestone | Deliverable | Exit or rollback |
|---|---|---|
| CMG0 | Persist this plan, governance/provenance, and B0 evidence | Handover/full checks pass; overlapping ownership is resolved |
| CMG1 | Mathematical contract and independent dense-oracle specification | Two fresh design reviews leave no unresolved counterexample |
| CMG2 | Stata 18/19 kernel prototypes and calibrated report | Bounded scratch and viable vectorized kernels, otherwise stop |
| CMG3 | Exact cell/hybrid builder and allocation guards | Zero algebra, nullspace, or component failures |
| CMG4 | True KSS lockstep diagonal-PCG baseline B1 — local candidate | Scalar/separate/batched equivalence and throughput gate passes locally |
| CMG5 | Deterministic hierarchy and Galerkin levels | Every level and complexity guard passes |
| CMG6 | Symmetric scalar/batched V-cycle harness | Materialized/randomized linearity, symmetry, and SPD gates pass |
| CMG7 | Forced test-only PPML adapter | Dense/matrix-free estimator equivalence and unchanged failures |
| CMG8 | Forced test-only KSS adapter — local solver candidate | Quotient, grounding, per-RHS, complete-residual, and speed gates pass locally; end-to-end estimator gate remains open |
| CMG9 | Auto router and adversarial local campaign | No false success, hidden allocation, or RNG change |
| CMG10 | Stata 19 SCC qualification | Correctness, scale, runtime, RSS, and portability pass |
| CMG11 | Hash-checked artifacts and promotion decision | Promote per package only under registered gates |

After CMG1, two fresh non-cross-contaminated reviews separately challenge
graph equivalence/grounding and hierarchy/SPD validity. After CMG8, two
different reviews inspect accepted Mata paths for false convergence, batching
contamination, memory blowups, and executable counterexamples. Model review is
evidence, not named-human independent review.

## Testing and benchmarking

The independent oracle exhausts connected incidence graphs through three
workers/four firms with unit and single-edge rational perturbations and runs at
least 10,000 fixed-seed rational cases through five workers, six firms, and ten
cells. It compares dense two-way systems, dense Schur complements, explicit
cliques, hybrid elimination, grounded Cholesky, and zero-sum pseudoinverse
solutions.

Required core tolerances are:

- Schur/hybrid action relative error `<=2e-12`;
- normalized symmetry and row-sum error `<=2e-13`;
- grounded/quotient fitted-value agreement `<=2e-10` when the grounded
  condition number is at most `1e8`;
- materialized V-cycle linearity `<=5e-13` and symmetry `<=1e-12` for `F<=12`;
- randomized bilinear symmetry `<=1e-11` and positive curvature
  `r'Br >= 1e-14*||r||*||Br||`; and
- final estimator agreement within the stricter package tolerance, never
  exceeding `2e-9*(1+abs(reference))` on well-conditioned overlap fixtures.

Adversaries include one/two firms, disconnected components, paths, rings,
trees, stars, expanders, lollipops, barbells, cluster chains, hubs, duplicate
cells, degree-three/four threshold cases, one worker visiting every firm,
weight ratios through `1e16`, underflow/overflow escalation, zero/dependent/
cancellation-heavy RHSs, scales `1e-200`, `1`, `1e200`, batches
`1,2,3,4,7,8,9,16,31,32`, ID relabeling, row permutation, frequency regrouping,
all grounds, and dirty workspace reuse.

Fault injection corrupts one RHS, recursive residual, V-cycle symmetry,
curvature, workspace, coarse output, stagnation, and iteration limits. No solve
may succeed without a fresh original-system residual. Every allocation records
dimensions, bytes, purpose, and high-water mark before allocation.

Benchmark four paths:

- B0: current committed diagonal implementation;
- B1: post-refactor diagonal path with true KSS batching;
- C: forced CMG on B1;
- A: automatic routing.

CMG claims use C/B1. Workloads cover kernel, solver-only, and unchanged
end-to-end estimators; 1/8/32/200/800 RHSs; easy 11--25 iteration graphs;
moderate 50--300 iteration graphs; weak capped graphs; 10k/100k/1m/5m/10m
observations; batches 1/2/4/8/16/32; Stata/MP 18 locally; and Stata/MP 19 on
SCC. Use five repeats at 10k/100k, three at 1m, and two at 5m/10m in fresh
processes.

Record canonicalization, graph setup, each level, workspace, PCG, final
residual, total time, per-RHS iterations/statuses/residuals, V-cycles, edge
passes, level sizes, setup amortization, structural bytes, peak RSS,
processor/license counts, seeds, tolerances, source hashes, and failures.

## Promotion and abandonment gates

| Gate | Requirement |
|---|---|
| Correctness | Zero oracle failures, false successes, cross-component coupling, or estimator regressions |
| Easy auto | Always diagonal; geometric-mean overhead `<=5%`, no case `>10%` |
| Moderate | With 200+ RHSs, setup-plus-solve speedup `>=1.5x` in at least 75% of cases per platform; no case slower by `>10%` |
| Weak rescue | Registered weak RHSs converge in `<=250` iterations and at least `2x` faster than capped diagonal |
| Batching | Batch-8 B1 throughput `>=1.5x` batch 1 at 1m+ in both adapters |
| Setup | Break-even at `<=32` RHSs for every CMG-routed case |
| V-cycle | At most six measured fine-Schur-action equivalents at 1m+ |
| Memory | Hierarchy guards pass; RSS `<=1.5x` B1 and below 56 GiB |
| Auto quality | `T_auto/min(T_B1,T_C)<=1.15` in 95% of cases and `<=1.30` everywhere |
| Portability | Same route/status on Stata 18/19; final values within registered tolerance |
| Scale | 10m easy/moderate cases complete without changing sample, probes, tolerance, or estimator |

Abandon automatic CMG if the quotient-SPD argument remains unresolved, any
invalid calculation becomes accepted, memory cannot be bounded before
allocation, the weak rescue fails, forced CMG has no high-value workload with
at least a 20% B1 gain, easy overhead or portability fails, or 10m
qualification requires changed estimator behavior.

If correctness passes but gains are narrow, keep CMG test-only or experimental
and retain diagonal as default. If B1 batching accounts for the KSS gain, ship
batching and close CMG as unnecessary.

## Assumptions and release boundary

- `reghdfe`, `ppmlhdfe`, three-plus fixed effects, heterogeneous slopes,
  compiled plugins, and estimator changes are out of scope.
- Python is a development-time oracle/assembler only, never a runtime
  dependency.
- Use `main` and the current worktree; no Git branch/worktree mutation is
  authorized.
- Preserve existing untracked and concurrent-thread changes.
- Prototype effort is estimated at 15--20 engineer-days and production
  qualification at 60--90 engineer-days.
- Expected speedups are hypotheses: easy graphs should remain diagonal;
  moderate repeated-RHS systems may gain 1.5--3x in the solver; weak systems
  may gain more; true KSS batching alone may gain 1.5--4x.
- The repository has no selected public software license. Internal development
  and install testing may proceed, but no package may be publicly distributed
  until licensing and third-party provenance are resolved.

## KSS-NUMOPT execution amendment — 2026-08-15

API 15 evaluates diagonal B1 and forced test-only C through one lockstep PCG
kernel. This removes the adapter's duplicated recurrence without changing any
estimator or numerical acceptance formula. A public-ado end-to-end test now
holds sample, probes, seed, tolerance, RNG end state, quotient normalization,
grounding, worker reconstruction, and complete residual checks fixed.

At 10,000 workers, 1,000 firms, 200 probes, seed `8675309`, and tolerance
`1e-10`, local command-level C/B1 speedups are 1.35x on moderate and 3.60x on
weak. The full estimator-matrix relative differences are `3.24e-12` and
`2.01e-11`. Easy C retains the typed `HIERARCHY_STALLED` rejection. These are
candidate measurements pending source-bound Stata 19/RSS evidence; they do
not satisfy the 5m/10m scale gate or authorize automatic routing.

For the rest of KSS-NUMOPT, every estimator submission—B1, C, or MATLAB
reference—must have a measured projected wall time at or below 5,400 seconds
and must be independently stopped at 5,400 seconds. Stata jobs use four slots;
a 64 GB reservation and 56 GiB CMG envelope are permitted. Do not submit a
large case. The owner-authorized real-data ladder is read-only and SCC-only:
start with a deterministic 5,000-worker CZ24 wage slice, keep derived rows and
match identifiers on SCC, and consider a natural full CZ route only if that
route's small calibration projects below 90 minutes. Only aggregate results,
timings, residuals, hashes, and scheduler accounting may enter Git.

# Pure-Mata CMG-Inspired Preconditioner Plan

## Header

- Plan ID: `CMG-MATA-V1`
- Milestone series: `CMG0`--`CMG11`
- Branch/worktree: `main`, current worktree
- Owner authorization: implement the reviewed CMG plan, 2026-08-14; promote
  and qualify it for KSS-PROD-1, 2026-08-15
- Status: API 5 installed KSS backend; KSS-PROD-1 failed at larger stress;
  KSS-SCALE-1 owner-stopped; KSS-STREAMLINE-1 uses the unchanged backend with
  structural routing; PPML integration pending
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

## KSS-STREAMLINE-1 routing addendum

Owner authorization on 2026-08-17 supersedes the KSS-specific pilot and scale
gates below without changing the CMG kernel. `shared/cmg/**` remains unchanged.
The historical KSS-PROD-1 and KSS-SCALE-1 sections preserve what was tried and
measured; their pilot caps, projected-work ratios, predecessor ladder,
56-GiB/12-hour envelope, improvement thresholds, and mandatory SCC rungs are
no longer active requirements.

The installed KSS command now selects a preconditioner structurally before
estimator RNG. Explicit diagonal uses B1. Explicit CMG constructs the hierarchy
and fails closed. Automatic mode uses diagonal for structurally small inputs or
when CMG is unavailable, and otherwise uses CMG after successful construction.
An automatic CMG setup failure may fall back to diagonal before RNG. The
command does not run routing pilot right-hand sides and does not apply the
historical `NO_REALISTIC_SOLVER_ROUTE` projected-work gate. Actual estimator
solves still enforce the user iteration limit and the complete original
worker-plus-firm residual for every accepted RHS.

Direct allocation fit remains a hard safety check. Memory headroom, wall-time,
work, and performance forecasts are advisory. Optional SCC experiments take
their target and resource specification independently and are not CMG or KSS
completion gates. This addendum makes no new CMG mathematical, performance,
scale, production, or release claim.

## KSS-PROD-1 hierarchy and routing addendum

This addendum supersedes the original v1 limits below where they conflict.
The dense terminal remains hard-capped at 6,144 hybrid vertices and may be
selected only after its factor and scratch forecasts fit the caller's bounded
memory envelope. Ordinary hierarchy construction may use up to 96 levels.
When the first deterministic forest clustering does not reduce a component,
API 5 applies component-aware normalized-heavy-edge pairing with canonical
endpoint tie-breaking and accepts it only when component counts, positive
weights, Galerkin contraction, and the registered cumulative edge/vertex
budgets remain valid. Failure preserves attempted-level diagnostics and never
changes the operator or adds regularization.

KSS routing now performs structural preflight and four deterministic
Park--Miller pilot RHSs before estimator RNG. Easy systems select diagonal B1
directly. Other systems compare bounded B1 and CMG pilot convergence with
deterministic structural-work proxies calibrated from retained dimensions,
hierarchy complexity, pilot iterations, and planned RHS count; live wall time
is diagnostic and cannot change the route. Forced CMG fails closed. Automatic
CMG failure may select B1 only before RNG and only when its bounded pilots and
projected work establish a realistic route. The estimator reuses one hierarchy
and terminal factors across all RHS batches and recomputes the complete
original-system residual for each accepted RHS.

The active KSS KB5/KB6 work must finish or be explicitly superseded before an
ownership transfer permits edits to overlapping KSS files. CMG is a new solver
milestone series and does not alter prior qualification evidence.

## KSS-SCALE-1 ownership addendum

Handoff commit `4dfc416d2a7f4fd2a1172586b044e0a709e3e936` closes
KSS-PROD-1 and releases the KSS/CMG paths for KSS-SCALE-1. The new milestone
may add richer pilot-failure diagnostics, share a canonical coefficient-cell
table with the outer FE operator, and optimize matrix-RHS applications. It
must preserve the exact CMG graph, fixed symmetric quotient-SPD V-cycle,
bounded terminal, deterministic construction, no-regularization rule, and
package-level complete original-system residual gate. Solver experiments are
accepted only through measured complete-command wall improvements; lower
iteration counts alone are insufficient.

### API 19 single-process scale integration — local candidate

API 19 shares one canonical coefficient-cell table between the outer
no-control match engine and the CMG adapter. Actual deletion units remain a
separate index, including parallel deletion IDs at one cell, and exact target-
scale strata remain a third index. The experimental path is eligible only for
JLA, match deletion, no controls, no deletion unit crossing a coefficient
cell, exact target strata, physical mass below `2^53`, at most 16,383 probes,
a registered runtime RNG contract, and passing pre-RNG resource admission.
All other supported designs retain the API 18 general engine. Forced
compressed calls return their typed eligibility failure; automatic generic
fallback runs only after its row-resident resource forecast passes.
The 311,730 value from CZ18 is a measured deletion-unit count and remains only
a provisional coefficient-cell count until the independent SCC diagnostic;
CMG sizing must not infer one index from the other.

The compressed FE transpose, Schur action, diagonal preconditioner, worker
reconstruction, fit/RSS, leverage/target RHS construction, target contraction,
and complete residual certificate use cell/unit/stratum arrays. The CMG graph
continues to use the exact cell weights and the unchanged degree-three hybrid,
Galerkin hierarchy, symmetric quotient-SPD cycle, and bounded terminal.
Target-scale groups use exact equality, never tolerance-based coalescing, and
cancellation-sensitive grouped terms use compensated accumulation.

Every API 19 fit, leverage, and target RHS is accepted only after evaluating
the original normal equations

\[
r_w=b_w-\left[d_w\alpha_w+\sum_{c:w_c=w}F_c\gamma_{f_c}\right],
\qquad
r_f=b_f-\left[e_f\gamma_f+\sum_{c:f_c=f}F_c\alpha_{w_c}\right].
\]

Here \(F_c\) is the cell's physical mass,
\(d_w=\sum_{c:w_c=w}F_c\), \(e_f=\sum_{c:f_c=f}F_c\), and \(b_w,b_f\)
are the original RHS blocks.

The numerical solve remains on the full-firm zero-sum quotient. The displayed
last-firm ground is applied afterward, and that firm's original equation is
still checked. The combined worker/firm Euclidean norm is relative to the
original RHS, or absolute for a zero RHS, and must not exceed
`max(1e-11,10*tolerance())`. A hybrid-graph, Schur, or recursive residual by
itself is insufficient.

The scale engine's versioned `mt64s` contract generates leverage and target
atoms in separate fixed-order domains. A logical atom is fixed by runtime RNG
version, master seed, domain, probe index, and canonical semantic atom
identity/order; batch width, solver route, tiling, convergence history,
processor count, and scheduling do not change it. Local Stata 18 and
source-bound SCC Stata 19 K1 evidence select a stateful per-domain cursor over
repeated per-probe resets, share identical golden vectors, and restore the
caller's full RNG state on every exit. Contract
`KSS-MT64S-DOMAIN-CURSOR-V2-STATA18-19` registers those two runtimes; every
other runtime fails closed.

The compressed command uses native disk-backed Stata preservation, clears raw
rows before the CMG/numerical peak, frees large Mata state, then restores
caller data and exact `e(sample)`. Admission accounts separately for raw Stata
data, persistent cell/unit/stratum state, hierarchy/factors, phase matrix RHS
scratch, sort/compression temporaries, solve-ahead coefficients,
outputs/certificates, preservation overhead, and maximum overlap. Both
compressed and generic routes add 25--30 percent memory and 50 percent
wall-time headroom and must fit 56 GiB and 12 hours before probes.

Source-bound API 5 raw-input CZ18 P40/P200 jobs 7204143 and 7206467
reconciled their phase forecasts. Fixed-retained P200 job 7206667 passed all
scientific, residual, identity, and restoration gates but exposed a distinct
allocator-overlap omission: numerical RSS reached 3,775,438,848 bytes against
a 3,181,596,634.6-byte registered peak. Resource API 6 and solver receipt API
23 now assume no allocator reuse between transition high-water and
numerical-only scratch/accepted solver allocation. Their exact component
upper bound for job 7206667 is 5,328,065,418.6 bytes before 30-percent
headroom. This is resource calibration only; it does not change CMG algebra,
routing, probes, estimates, or completed SCC evidence. Scale progression
remains blocked until a clean API 6 rerun reconciles.

API 6 CZ24 P200 job 7207871 completed successfully at the scheduler and
estimator layers but remains failed validation evidence. Its deployed
source-bound validator still reconstructed the pre-API-6 live numerical sum
and rejected the new receipt. The validator now mirrors the API 6 no-reuse
maximum in a new source revision; job 7207871 will not be revalidated, and the
scale ladder remains blocked until a clean bundle reruns CZ24.

The repaired API 6 bundle subsequently passed CZ24 P200 job 7209064, CZ25
P200 job 7209095, fixed CZ18 P40 job 7209195, and fixed CZ18 P200 job
7209358. The last run certified 601 original-system right-hand sides with
maximum relative residual `9.99040353264e-11`, measured 3,869,261,824 bytes
peak process RSS, and reconciled the 5,328,065,589.75-byte no-reuse forecast.
This validates the current CMG allocation receipt at 1x; it is not 2x scale
evidence. The first well-connected 2x P40 job, 7209896, failed before CMG
construction because unweighted connector rows did not receive the fixture's
internal literal-one frequency variable. The focused local repair and
regression do not reinterpret that failed job; all source-bound predecessors
must be rerun from the new commit before scale progression resumes.

Source `cdc74f4` reran and passed CZ24/CZ25 P200 plus fixed-CZ18 P40/P200 in
jobs 7209971, 7209987, 7210003, and 7210098. The required fixed P200
predecessor certified 601 original-system right-hand sides and measured
3,767,971,840 bytes peak process RSS. Repaired well-connected 2x P40 job
7210297 then built a valid nine-level hierarchy with 57,154 hybrid vertices,
339,183 hybrid edges, and terminal size 226, but returned typed pre-RNG
`NO_REALISTIC_SOLVER_ROUTE`. P20 profile job 7210431 reproduced the failure,
but its 61 planned right-hand sides retain the same 64-iteration pilot cap.
P10 profile job 7210689 supplied the controlled 128-iteration comparison and
again returned `NO_REALISTIC_SOLVER_ROUTE`, so a cap of 64 or less is not the
sole cause. The old bundle did not serialize the per-pilot rows, and no
numerical gate is relaxed from this evidence. The SCC driver now writes the
existing route and per-pilot matrices before typed exit so the next source-bound
diagnostic identifies status, residual, iteration, action, or projected-work
failure exactly.

KSS-SCALE SCC experiments are single-job and single-process. The provisional
reservation is `-pe omp 14` with `mem_per_core=4G`; those slots reserve shared
capacity and do not make the estimator distributed. The wrapper separately
sets and verifies `c(processors)==4`, stages the input and Stata temporary
files under `$TMPDIR`, and records requested/actual slots, Stata processors,
process RSS, and `qacct maxvmem`. There are no estimator shards or reducers.

Well-connected and ring replication fixtures are separate. The former is the
ordinary dimensional-scaling input; the latter is an adverse-connectivity and
deletion-safety stress whose deteriorating spectral gap cannot justify normal
4x extrapolation. Record condition proxies, hierarchy changes, iterations,
actions, and complete stage timing for each.

New complex CMG/Krylov mechanisms require a repeatable complete-command wall
gain of at least 10 percent after dense block work. A simple low-risk kernel
change may survive with at least 5 percent repeatable gain or a measured
scale-enabling reduction in peak memory or required passes. Use cold wall,
warm command wall, CPU, repetitions, and spread; do not accept iteration gains
alone. Stop after mandatory well-connected 4x P200 qualification if no
remaining candidate projects 5 percent end-to-end improvement or admits an
otherwise blocked larger scale. Local API 19 tests implement the algebra,
RNG, lifecycle, resource, fixtures, and command path. SCC CZ24/CZ25 and CZ18
jobs currently provide calibration and fail-closed resource evidence, not a
qualified 2x/4x rung; MATLAB scale timing remains pending. Unmeasured scale
and MATLAB forecasts remain hypotheses.
Successful KSS-SCALE-1 completion would establish an experimental,
scale-qualified single-job engine, not production polish or public-release
status.

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
and 32 for 128 or more. Select diagonal without building CMG only when every
pilot passes its complete-residual gate in at most four iterations and the
projected repeated-RHS work is within the fixed B1 envelope.

Otherwise build and validate CMG and run CMG pilots to
`min(maxiter(),250)`. Select CMG only when every CMG pilot passes its complete
residual gate and either B1 is not a realistic bounded route or the
deterministic setup-plus-solve work score is at most 80% of diagonal work.
Routing never uses wall-clock timing, estimator probes, `seed()`, or `batch()`.

Setup or pilot failure falls back before RNG initialization only when bounded
B1 pilots have already passed their iteration, complete-residual, and
projected-work gates. A CMG preflight failure without that evidence fails
closed. After RNG initialization the route is locked; a CMG breakdown or
failed full residual withholds rather than switching paths.

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
supplemented by source-bound Stata 19 SCC command-level gains of 1.725x and
4.620x, estimator differences of `3.23e-12` and `2.01e-11`, complete
residuals below `1e-10`, and peak RSS below 122 MiB. Easy C again fails closed.
The evidence does not satisfy the 5m/10m scale gate or authorize automatic
routing.

For the rest of KSS-NUMOPT, every estimator submission—B1, C, or MATLAB
reference—must have a measured projected wall time at or below 5,400 seconds
and must be independently stopped at 5,400 seconds. Stata jobs use four slots;
a 64 GB reservation and 56 GiB CMG envelope are permitted. Do not submit a
large case. The owner-authorized real-data ladder is read-only and SCC-only:
start with a deterministic 5,000-worker CZ24 wage slice, keep derived rows and
match identifiers on SCC, and consider a natural full CZ route only if that
route's small calibration projects below 90 minutes. Only aggregate results,
timings, residuals, hashes, and scheduler accounting may enter Git.

The read-only ladder subsequently distinguished a genuine 500-worker dense
core from the 5,000-worker cap, which admitted all 4,653 eligible movers. On
the 500-worker core, CMG constructs a hierarchy but both CMG and B1 reject the
same nonestimable match deletion. On the all-mover sample, B1 reaches that
same estimator rejection and CMG rejects hierarchy construction. No estimate
or residual success is posted, so no natural-full estimator route and no
automatic router are authorized.

The MATLAB-first diagnostic then uses the successful maintained retained set
only as a benchmark selector. API 17 passes the small exact oracle before B1
or C. API 4 raises the memory-rich repeated-RHS terminal cap to 6,144 hybrid
vertices, subject to the existing RHS, declared-memory, predicted-factor, and
pre-allocation checks. Source-bound Stata 19 pairs pass on 11,549, 60,160, and
256,472 rows. C/B1 command speedups are 2.283x, 2.585x, and 4.122x; the final
pair has estimator `mreldif=1.2879e-10`, maximum complete residuals
`9.9978e-11`/`4.0524e-13`, and peak RSS 443,140/490,172 KiB. The final
estimators finish in 416.750/101.096 seconds after 900-second measured
projections, well below the universal 5,400-second limit.

This completes the bounded CMG10 forced-KSS real-data gate. It does not
authorize automatic routing. Keep B1 on easy/well-conditioned graphs; easy C
still fails closed, CMG remains outside the installed package, and
automatic-dispatch/no-regression promotion remains a CMG11 obligation.

## KSS-PROD-1 closeout amendment — 2026-08-16

API 18 supersedes the historical forced-only and B1-only package statements
above. The normal internal package installs CMG and exposes deterministic
exact, B1, bounded-terminal CMG, and multilevel-CMG routing before estimator
RNG. Source-bound candidate
`5e2687c6a12c221ad899f1b31227b2f81693d383` passes CZ24/CZ25 retained-sample,
route, equality, residual, installation, and Stata 18/19 gates. Full CZ18 job
`7197620` passes with 8,201,888 retained rows, nine hierarchy levels, a
131-vertex terminal, 601 accepted RHSs, maximum complete residual
`9.9601e-11`, and 24.561 GiB peak RSS.

KSS-PROD-1 nevertheless closes **not production-qualified**. Three
independently scheduled two-times-CZ18 P20 jobs (`7197621`--`7197623`) all
withheld before RNG at the same typed automatic-route boundary. Each built a
57,154-vertex/339,183-edge, ten-level hierarchy with a 195-vertex terminal;
neither B1 nor CMG passed all bounded convergence, complete-residual, and
deterministic-work gates. Their timing spread is unexplained and is not
attributed to concurrent submission or host class. The registered validator
therefore refused the production phase, no upper-envelope projection exists,
and no P200 stress job was submitted.

For KSS, this closeout supersedes the exploratory 5,400-second KSS-NUMOPT
matrix and CMG8--CMG11 promotion language only to the extent described here:
the backend is installed and CZ18-capable, but it is not production-qualified.
It also supersedes the prospective default-policy sentence above for this
internal API 18 candidate: omitted `preconditioner()` currently means `auto`,
even though the candidate failed production qualification. That default is an
implemented candidate API, not a production endorsement; the command fails
closed when no realistic route passes.
PPML remains pending. A future milestone must expose retained per-pilot
failure diagnostics and repair the larger-graph route without weakening the
complete residual, tolerance, RNG, formula, memory, or no-regularization
contracts. This thread does not begin that work.

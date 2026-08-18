# CMG-MATA-1 execution plan

## Header

- Plan ID: `CMG-MATA-1`
- Milestone: source-informed, GPL-3.0-only, Mata-only CMG hierarchy update
- Branch/worktree: `main`, current worktree
- Owner: Johannes Schmieder
- Authorization date: 2026-08-18
- Baseline commit: `652102d9e712087ee8d279e46f5347e99c40a436`
- Predecessor: completed `KSS-NUMOPT-2`; shared CMG API 5 is the frozen
  numerical and performance reference
- Runtime constraint: Mata only; no C/C++/Fortran executable, MEX file, Stata
  plugin, subprocess, or binary interchange layer
- License: covered code is `GPL-3.0-only`
- Allowed changes: `shared/cmg/**`, `kss_bc/**`, narrowly necessary generated
  CMG artifacts under `ppml_talo/**`, `cmg_plan.md`, `LICENSE_STATUS.md`,
  `CODE_LICENSE.md`, and `LICENSES/**`
- Protected: `archive/**`, `application/**`, `paper/**`, `paper/releases/**`,
  `theory/**`, `proof-audit/**`, root `state/**`, and imported upstream bytes

## Goal

Replace the scale-limiting parts of the current hierarchy construction with a
Mata implementation of the robust graph-profile, forest-splitting,
weak-group-repair, component-packing, and hierarchy-control ideas in the
official GPL CMG implementation. Remove the degree-four setup cliff and make
balanced degrees five through seven routine, while preserving the installed
estimator, exact hybrid graph, symmetric quotient-SPD V-cycle, PCG, probe
count, RNG, tolerances, and full worker-plus-firm residual gate.

The milestone produces local and SCC evidence against both frozen API 5 and
matched maintained MATLAB runs. It does not authorize a compiled helper,
change a statistical estimand, establish a theorem, or release the package.

## Non-negotiable contracts

The following remain unchanged unless a later owner-authorized milestone says
otherwise:

- all hierarchy construction and application executed by Stata stays in Mata;
- positive unique worker--firm cell weights and power-of-two scaling;
- exact degree-three hybrid representation: degree one contributes no edge,
  degrees two and three use exact clique edges, and degree four and above use
  exact auxiliary stars;
- exact positive Galerkin contraction, with no sparsification, clipping,
  ridge, pseudoinverse repair, or component selection;
- deterministic canonical tie breaks based on semantic vertex keys;
- component containment and unchanged component count at every level;
- the fixed one-child, one-pre/one-post symmetric quotient-SPD V-cycle;
- package PCG, per-RHS status, and fresh full original-system residual checks;
- route choice before estimator RNG, with no post-RNG solver switch;
- requested probes, batches, sample, deletion unit, target weights, and caller
  state; and
- a typed failure instead of an unverified success.

The official recursive repeat schedule may be measured only as a separate
Mata experiment after the fixed cycle passes. It is not promoted in this
milestone without independent symmetry, positive-curvature, residual, and
complete-command timing gates.

## GPL and provenance policy

1. Use SPDX identifier `GPL-3.0-only`. The inspected upstream notices specify
   GNU GPL Version 3 and do not expressly grant an “or later” option.
2. Record exact upstream commit, path, and SHA-256 for every inspected source
   file. Imported application files remain read-only.
3. Mark the API-6 Mata implementation as source-informed and GPL-derived; do
   not call it clean-room. Preserve the historical fact that API 1--5 was
   written before source inspection.
4. Preserve upstream copyright and GPL notices in the canonical source and
   generated artifacts. Mark the Mata port and subsequent modifications.
5. Do not copy or distribute headerless maintained MATLAB KSS wrappers,
   compiled MEX files, data, or job artifacts.
6. GPL-3.0-only covers the CMG code and repository-authored package code when
   distributed as a CMG-containing program. It does not license the paper,
   proofs, imported application material, or data.
7. Public distribution remains gated on a human review of file notices,
   complete corresponding source, third-party boundaries, and data exclusions.

## Technical design

### 1. Frozen reference and API boundary

Retain the current API-5 hierarchy builder as an internal reference during
development. Name the paths explicitly, for example
`hierarchy_build_api5_reference()` and `hierarchy_build()`, so tests can run
both in fresh Stata processes. The public shared CMG API advances only after
the new path passes the local gate. Generated KSS, PPML, and test namespaces
must come from one canonical template and a source-hashed manifest.

The API-5 path is removed from automatic production routing only after SCC
acceptance. It may remain as an internal benchmark/reference path until
closeout. No user option is needed unless retaining it has diagnostic value.

### 2. Source-informed Mata primitives

Port the following algorithmic ideas into idiomatic vectorized Mata:

1. compute degree, incident volume, maximum edge conductance, and each
   vertex's canonically tie-broken heaviest neighbor in adjacency order;
2. form the selected directed forest without relying on input row order;
3. split long forest branches using bounded-diameter pointer walks;
4. cut weak branches/groups using the official conductance logic;
5. detach or repair vertices whose selected-tree internal weight is below
   one eighth of total incident weight;
6. label the remaining forest components and canonically pack dense aggregate
   labels; and
7. contract with exact `P'KP`, drop self loops, and deterministically sum
   duplicate coarse edges.

The implementation must use bulk `order()`, `panelsetup()`, `panelsum()`,
indexed assignment, and bounded pointer-jumping passes. An interpreted loop
over all vertices or edges is allowed only where profiling shows bounded cost
and no vectorized equivalent; every such loop is documented and benchmarked.
Avoid per-vertex searches, repeated full-edge sorts, quadratic insertion,
dense adjacency matrices, and storage-order tie breaks.

### 3. Robust hierarchy controller

At each level:

- forecast persistent and peak construction bytes before material allocation;
- prepare and certify components once, reusing the certified adjacency panels;
- apply official-style grouping and the one-eighth repair;
- require the fixed component-surplus reduction gate;
- use one deterministic component-contained fallback only when the primary
  grouping misses that gate;
- build and certify the exact coarse graph;
- check cumulative edge/vertex complexity, component count, finite weights,
  canonical keys, and level caps; and
- stop at the existing registered dense terminal and prepare its Cholesky
  factors with the current residual check.

The fallback must itself be near-linear apart from documented sorting. It may
use normalized heavy-edge matching followed by canonical packing within each
certified component. It may not merge components or change the operator.

### 4. Diagnostics and resource accounting

Record, per attempted level:

- vertices, edges, components, proposed coarse vertices, reduction, and
  grouping method;
- time for profile/adjacency, forest split, weak repair, component labeling,
  contraction, certification, and terminal factorization;
- predicted persistent/temporary bytes and observed Stata memory evidence
  where available; and
- typed status and message for a failed attempt.

KSS command diagnostics retain estimator setup/solve/correction/full-command
times and add hierarchy substage times without changing returned estimates.

### 5. Deferred representation experiments

After the primary Mata port passes, benchmark but do not automatically promote:

- exact degree-four clique versus the current auxiliary star;
- an exact adaptive clique/star choice based on predicted vertices, collapsed
  coarse edges, and measured Mata setup coefficients;
- alternative bounded pointer-jumping block sizes; and
- the official recursive repeat schedule implemented entirely in Mata.

Degrees five through seven remain exact stars by default because clique work
grows as `k(k-1)/2`. Any representation change must pass exact Schur-action,
memory, PCG, residual, and complete-command gates.

## Work sequence

### M0. Governance and frozen baseline

Produce this plan, GPL files, upstream manifest, and a baseline receipt binding
API 5, Optimization III evidence, Stata/MATLAB versions, source commit, and
existing degree-three/four timings. Run the handover and shared-CMG gates.

Exit: baseline is reproducible and every protected path is unchanged.

### M1. Algorithm contract and adversarial registry

Write a Mata-level contract for graph profile, forest selection, branch
splitting, weak repair, canonical component labels, exact contraction, and
failure statuses. Add small hand-computable fixtures and randomized reference
oracles before replacing the builder.

Fixtures cover paths, rings, stars, cliques, barbells, lollipops, cluster
chains, hubs, disconnected components, singleton components, duplicate cells,
ties, one-heavy-edge designs, and log-spread weights through `1e12`.

Exit: each primitive has an independent slow oracle and a minimized failure
fixture; semantic relabeling and row permutation leave canonical results
unchanged.

### M2. Vectorized profile and forest primitives

Implement the heaviest-neighbor profile, bounded forest split, one-eighth weak
repair, and component labeling in the canonical Mata template. Benchmark each
primitive separately across degrees two through seven and geometrically
increasing graphs.

Exit:

- primitive outputs match the slow oracle on exhaustive tiny and randomized
  cases;
- no pointer cycle, missing vertex, cross-component aggregate, or nondense
  label survives certification;
- repeated runs and input permutations are deterministic; and
- fitted primitive wall-time slopes against `V+E` are at most 1.20 locally,
  allowing documented sort terms.

### M3. Exact contraction and hierarchy integration

Integrate the new grouping with the existing exact contraction, resource
guards, attempted-level ledger, and dense terminal. Optimize duplicate edge
collapse only if profiling identifies it as material. Keep the API-5 builder
callable for A/B tests.

Exit:

- every coarse graph equals the independent contraction on small cases;
- large randomized action tests satisfy
  `x' K_coarse x = (P x)' K_fine (P x)` at the registered tolerance;
- row sums, positive weights, components, and canonical keys pass;
- balanced degree two through seven hierarchies terminate successfully; and
- malformed/adversarial cases either succeed correctly or return the expected
  typed failure.

### M4. V-cycle, PCG, and generator gate

Prepare levels with the existing projection, smoother, prolongation, and
terminal-factor code. Regenerate all namespaces and advance the API/generator
identity only after tests pass.

Exit:

- V-cycle linearity `<=5e-13`, symmetry `<=1e-12`, and positive-curvature
  tests pass;
- zero, cancellation-heavy, dependent, scalar, and batched RHSs pass;
- dirty-workspace and repeated-apply tests pass;
- PCG converges with the unchanged full residual gate; and
- KSS, PPML, and test namespaces have no drift beyond generated hashes and
  intended source-informed/GPL notices.

### M5. KSS integration and scientific equivalence

Route KSS CMG construction to the new Mata builder before RNG. Extend
diagnostics and resource forecasts. Keep estimands, probes, semantic RNG atoms,
and lifecycle behavior frozen.

Exit:

- exact and JLA small estimators agree with frozen API 5 at existing strict
  tolerances;
- API-5 and API-6 use identical probes on matched runs;
- all accepted RHSs pass full worker-plus-firm residuals;
- caller data, sample, RNG state/streams, processors, and sort state restore;
  and
- forced CMG cannot silently downgrade after RNG begins.

### M6. Local quick correctness and calibration gate

Run this before any SCC deployment.

Correctness matrix:

- worker degrees `2,3,4,5,6,7`;
- balanced, tied, one-heavy-edge, log-spread `1e4/1e8/1e12`, hub, ring,
  barbell, lollipop, cluster-chain, and disconnected fixtures;
- tiny exhaustive graphs plus approximately 1k, 10k, 100k, and a one-million-
  edge setup stress;
- RHS batches `1,2,4,8,16,32`; and
- new Mata, API-5 reference, and diagonal routes where eligible.

Quick benchmark ladder:

- fixed worker/firm ratio 40;
- degrees `2--7`, rows per cell `1` and `8`;
- workers `1,000`, `10,000`, and the largest case that keeps each local run
  near one minute;
- five fresh-process repetitions for subsecond cells and three otherwise;
- setup-only hierarchy timings and complete KSS `P8` timings; and
- input hashes, status, levels, iterations, residuals, memory forecast, and
  substage timings for every cell.

SCC admission requires zero correctness failures, no adjacent-degree setup
cliff greater than `3 *` the corresponding `(V+E)` ratio, no memory forecast
violation, and a clear degree-four improvement over API 5. Gross regression at
degrees two or three blocks deployment.

### M7. Source-bound SCC smoke

Create an immutable run under
`/projectnb/welfgr/ppml-variance-cmg/runs/<UTC>-<source>/`, with isolated
`input`, `logs`, `work`, `output`, and `receipts`. Deploy incrementally without
deleting or modifying earlier runs. All sustained compute runs through SGE.

Smoke cells:

- `f1024_d3_r1`, `f1024_d4_r1`, and `f1024_d7_r1`;
- new Mata and selected API-5 reference runs in Stata;
- maintained MATLAB KSS with official CMG on the identical generated input;
- `P20` for every headline comparator;
- four effective Stata processors and four effective MATLAB workers/threads,
  with numeric libraries constrained so neither side oversubscribes; and
- identical worker/firm/cell design, deletion definition, tolerance, and
  reported probe metadata.

Accept only if `qacct` reports `failed=0` and `exit_status=0`, application
success markers and schemas validate, hashes match, estimates are finite,
residuals pass, and wall/CPU/maxvmem evidence is complete. Only then submit the
production arrays.

### M8. Exhaustive parallel SCC campaign

Freeze task manifests before submission. One SGE array task owns one
application, implementation, shape, and repetition, and writes an isolated
output directory. Stata and MATLAB arrays may run concurrently after smoke.

Primary matched grid:

- applications: Stata new Mata CMG and maintained MATLAB official CMG;
- reference: API-5 Stata at small/medium scales and selected larger
  degree-four cells when its wall request is reasonable;
- degrees `2,3,4,5,6,7`;
- scales `f1024`, `f256`, `f64`, `f32`, and `f16` relative to the 40-million-
  worker target;
- rows per cell `r1` and `r8`;
- `P20` for every Stata/MATLAB headline match;
- repetitions: three at `f1024/f256`, two at `f64`, one at `f32/f16`; and
- an additional admitted Stata `P200` block at `f64` to measure repeated-RHS
  amortization, kept separate from the matched MATLAB comparison.

Robustness grid at `f256`, `P20`, two repetitions:

- degrees `4--7`;
- balanced, tied, one-heavy-edge, and log-uniform weight ratios
  `1e4/1e8/1e12`;
- hub, ring, barbell, lollipop, cluster-chain, disconnected, and singleton-
  component setup/solver fixtures; and
- new Mata, with API-5 or diagonal controls where informative.

Real-data holdout:

- frozen CZ18 retained design at `P20` in new-Mata Stata and maintained
  MATLAB, with identical retained-input hash and the established four-
  effective-core policy;
- API-5 Stata only if its forecasted wall request is admissible; and
- Stata `P200` only after the resource forecast passes.

No task exceeding a 12-hour request is submitted without splitting or a
documented exception. A dependent merge job independently validates all task
receipts; scheduler completion alone is insufficient.

### M9. Analysis, tuning, and controlled reruns

Report setup, hierarchy substages, terminal factor, solve, correction, and
complete-command wall separately. Include level shapes, reductions, PCG
iterations, V-cycles, per-RHS/max residuals, probes, batches, effective cores,
runtime versions, host/CPU, requested resources, qacct CPU/wall/maxvmem, and
all source/input/task hashes.

Compute new/API-5 and Stata/MATLAB ratios only on exactly matched cells.
Estimate log-log slopes against cells, rows, and `V+E`. Retain censored/time-
limited jobs as lower bounds. Report degree-three-to-four and every adjacent-
degree discontinuity. Keep queue wait separate from application wall.

Tune only a diagnosed implementation stage. Preserve the failed run, use a new
source hash/run ID, rerun the local regression first, and resubmit the complete
affected comparison block. Do not mix source generations or select the fastest
repetition.

### M10. Closeout

Run shared CMG, KSS quick/full, isolated install, PPML generated-code, handover,
and full repository gates. Record all run/job IDs, qacct validation, evidence
hashes, runtime versions, changed/untouched files, failures, limitations, and
the promote/experimental/rollback decision.

## Quantitative promotion gates

| Gate | Requirement |
|---|---|
| Runtime architecture | zero compiled CMG runtime artifacts or subprocess calls; Stata path is Mata only |
| Graph correctness | zero component, positive-edge, row-sum, action, or Galerkin failures |
| Preconditioner algebra | registered linearity, symmetry, and positive-curvature tolerances pass |
| Estimator correctness | zero false successes; unchanged complete residual gate passes every accepted RHS |
| Determinism | identical hierarchy diagnostics under repetition, row permutation, and semantic relabeling |
| Degree robustness | every admitted balanced degree `2--7` case constructs and solves |
| Density-four cliff | new `d4/d3` setup ratio is at most `3 * ((V+E)_d4/(V+E)_d3)` |
| Setup scaling | fitted new-Mata setup slope against `V+E` is `<=1.20` on uncensored SCC ladders |
| API-5 improvement | at least `5x` faster hierarchy setup on the largest matched degree-four cell API 5 completes |
| Low-degree regression | complete-command median is no more than 10% slower at matched degree-two/three sub-minute cells |
| Memory | forecast and measured maxvmem stay within registered direct and scheduler envelopes |
| MATLAB comparison | same input, P20, and effective-core policy; report and diagnose any setup or command gap above `2x` |
| Portability | local Stata 18 and SCC Stata agree on statuses/routes and estimator tolerances |
| Real data | CZ18 passes unchanged scientific, residual, lifecycle, and resource gates |

Correctness and safety gates are hard. If the new Mata hierarchy is correct but
misses performance gates, retain it as experimental and keep API 5 or diagonal
automatic behavior. Never weaken tolerances, residuals, probes, or memory
checks to obtain a timing claim.

## Main risks and mitigations

1. Mata interpreter loops reproduce the C algorithm but not its speed. Use
   bulk adjacency panels and bounded pointer-jumping; profile substages before
   changing algorithms.
2. Storage order affects ties. Use semantic keys in all orderings and run
   permutation/relabeling tests.
3. Weak-edge repair can stall or fragment a graph. Certify component
   containment and use one registered component-aware fallback.
4. Exact duplicate collapse can dominate. Reuse sorted canonical edge panels
   and avoid repeated full-edge sorts.
5. Better setup can worsen PCG. Evaluate iterations, residuals, and full
   command, not setup alone.
6. Extreme weights can destabilize ratios. Keep inherited power-of-two scaling
   and test ratios through `1e12` plus representable scale extremes.
7. MATLAB comparison can be mismatched. Freeze input/probe/core manifests and
   label formula or RNG differences explicitly.
8. SCC hardware and queueing can confound timings. Record host/CPU and qacct;
   keep queue wait out of application wall.
9. GPL provenance can be incomplete. Maintain file-level hashes/notices and
   require human review before distribution.

## Done when

`CMG-MATA-1` closes only when the Mata-only implementation and GPL/provenance
records are complete; all local gates pass; every expected SCC smoke and
production task has validated scheduler, application, and output evidence or
an explicitly preserved failure; matched MATLAB comparisons use the same
inputs, P20, and effective-core policy; analysis reproduces from frozen
manifests; package/repository validation passes; and the completion report
makes an explicit promotion decision without claiming public release or
independent mathematical certification.

## Final disposition

Completed 2026-08-18 as an **internal experimental KSS candidate**. Production
and public release remain disabled.

The retained API-6 implementation is Mata only and GPL-3.0-only. Local gates
pass 27 shared Python tests and all registered Mata checks, 235 KSS Python
tests, and the full installed KSS Stata suite. The SCC campaign validates:

- 120 matched Stata/MATLAB P20 task pairs across degrees 2--7, rows-per-cell
  1/8, and 16--1,024 firms;
- all 18 corrected hierarchy tasks, including degree 2--7 setup ladders and
  API-5 setup controls;
- all 13 adversarial families at 10,240 vertices;
- 12 f1,024 CMG P200 repeated-RHS cells with 601 certified right-hand sides;
  and
- the fixed 8,201,888-row CZ18 retained input in both pure-Mata Stata and the
  official-CMG MATLAB comparator, using P20, seed 8675309, a 56-GiB envelope,
  and four effective application workers.

The hierarchy setup gates pass: degree-four is 12.81 times faster than API 5
at the largest matched API-5 SCC cell, and degree-specific endpoint slopes are
1.045--1.125 on the registered hierarchy ladder. Complete-command scaling is
not universally linear. Across the full synthetic 16-to-1,024-firm ladder,
Stata endpoint slopes are .50--1.02 and MATLAB slopes are .33--.55. On CZ18,
the registered Stata command is 305 seconds and the maintained MATLAB command
is 43.802 seconds; formula, RNG, and target-weight differences keep this a
descriptive throughput comparison.

The predeclared low-degree performance gate remains unclosed because the SCC
campaign did not run direct frozen-API-5 complete-command medians for the
matched degree-two/three sub-minute cells. API 6 uses the retained API-5
screened forest for those dense ordinary quotients and local setup evidence
shows no material cliff, but that does not replace the planned command-level
A/B test. This missing performance comparison, the absent named human
mathematical review, and the pending human GPL/provenance review block
production promotion. The complete evidence and run registry are in
`shared/cmg/benchmarks/reports/CMG_MATA_1_2026-08-18.md`.

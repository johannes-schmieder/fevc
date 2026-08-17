# `kss_bc` living implementation plan

- Plan ID: `KSS-BC-DEV-2026-08`
- Milestone series: KB0--KB6, KSS-NUMOPT-1, KSS-PROD-1, and KSS-SCALE-1
- Branch/worktree: `main`, current worktree; no branch or worktree creation
- Owner: Johannes
- Start date: 2026-08-14
- Status: active; KSS-PROD-1 complete and failed; KSS-SCALE-1 API 19 local
  candidate and RNG K1 complete, SCC scale qualification pending
- Historical base commit: `2b3bb6b15b6a6d4d638c8ec6e7d3eea8641a7abb`
- KSS-PROD-1 base commit: `0e2fdd2b1d810f507f6768b4b77f2461ecabfd10`
- Allowed changes for KSS-SCALE-1: `kss_bc/**`, `shared/cmg/**`,
  `cmg_plan.md`, narrowly related KSS/CMG benchmark and SCC tooling, focused
  tests, and experimental documentation; the hash-frozen repository runner
  remains unchanged
- Protected: `ppml_talo/**`, `application/**`, `software/**`, `paper/**`,
  `theory/**`, `proof-audit/**`, `state/**`, `archive/**`, `paper/releases/**`

## KSS-SCALE-1 checkpoint — ACTIVE

- Handoff commit: `4dfc416d2a7f4fd2a1172586b044e0a709e3e936`.
- Runtime baseline: `5e2687c6a12c221ad899f1b31227b2f81693d383`.
- Owner authorization: implement the reviewed single-job scale plan, 2026-08-16.
- Runtime boundary: one Stata/Mata 18--19 process per estimate; no estimator
  sharding, cross-node numerical work, inter-job coefficient files, or
  numerical reducers.
- SCC boundary: SGE slots reserve CPU, memory, I/O, and shared-node capacity
  separately from Stata processor use. The provisional large-job request is
  14 `omp` slots at 4 GiB per slot while Stata sets and verifies four actual
  processors.

KSS-SCALE-1 targets the common Separations regime: improved JLA, match
deletion, no controls, repeated worker--firm observations, 200 probes, and
multilevel CMG. It introduces an experimental compressed path with separate
coefficient cells and deletion units. Ineligible inputs use the unchanged
general engine only when that route independently passes pre-probe memory and
wall-time admission.

The checkpoint sequence and current state are:

1. **K0 — COMPLETE:** bind handoff `4dfc416`, scope, single-job boundary, and
   ownership. The documentation checkpoint is `9d6a5ea`.
2. **K1 — COMPLETE:** retain complete
   route-pilot diagnostics, independently count cells and deletion units, and
   compare one registered `mt64s` stream per probe with one fixed-order stream
   per domain. Local Stata 18 golden vectors, invariant partitions, caller-
   state restoration, scalar/vector call shape, large-count chunking, and
   paired timings select the simpler stateful per-domain cursor. Stata 19 was
   required to match those vectors or receive a distinct registration. The first
   source-bound attempt, job 7200951 at commit `48b478b`, reached its
   registered 3,480-second application limit with `failed=0`,
   `exit_status=124`, 581.066 MiB `maxvmem`, and no completed K1 receipt. Its
   qacct SHA-256 is
   `931dcbfa22376d81c7e7e21cc559920a27ae1da3866e2838f1b23ff283f1f8ca`.
   That censored lower bound justifies one new 5,400-second scheduler limit:
   5,220 seconds is exactly 1.5 times the observed application limit, with a
   120-second wrapper reserve and 60 seconds of remaining scheduler margin.
   Job 7201036 then completed the unchanged Stata workload in 4,599 seconds:
   its Stata 19 vectors matched Stata 18, every RNG/state gate passed, and the
   three P40/50,000-atom timing repetitions measured 918.702--920.267 seconds
   for per-probe streams versus 3.499--3.516 seconds for the fixed-domain
   cursor. It remains failed evidence because Mata wrote literal `\\t` text
   into the nominal TSV receipts; the wrapper's real-tab check returned 1 and
   qacct therefore recorded `exit_status=1`. The preserved qacct SHA-256 is
   `cf5a16e6d568bf931047be40d107049f5f3d1ca205d0f234c7ed4e48081e89e3`.
   A serialization-only source fix replaced those literals with byte 0x09.
   Final job 7201105 at source `c4e7ab3`, bundle
   `d722b2ab2e2429b234887393a7b4e8ce7b5a0743312ce033f2947b1d03d239ac`,
   passed the application and wrapper layers in 4,610 seconds with 4,798.039
   CPU seconds and 581.438 MiB `qacct maxvmem`; qacct SHA-256 is
   `b40a03442e5ff3f2bb8408d24eaa61088d2568b7a4753198ba84ab878ec019e2`.
   Its three P40/50,000-atom timing pairs measured 918.336--927.412 seconds
   for per-probe streams and 3.468--3.523 seconds for the fixed-domain cursor.
   Every golden, partition, processor, call-shape, large-count, domain, and
   complete-state-restoration gate passed. The source-bound external validator
   initially rejected the SCC launcher display label `c(flavor)="IC"` despite
   `c(MP)==1` and four verified processors. Validator-only commit `2e2e1e1`
   corrected that known SCC flavor assumption and preserved the rejected
   metadata. Its qacct-bound validation receipt SHA-256 is
   `cb6b882b5eb3636c500eb72a3f25ae5f3a7a0ca8d48c1dc6b3d1b288420501f4`.
   Stata 18 and 19 now share registered contract
   `KSS-MT64S-DOMAIN-CURSOR-V2-STATA18-19`; unregistered runtimes fail closed.
3. **K2 — LOCAL IMPLEMENTATION COMPLETE:** build canonical coefficient-cell,
   deletion-unit, and exact target-stratum aggregates. The command uses native
   disk-backed Stata `preserve`/`clear`/`restore`, releases row data during
   numerical work, frees large Mata state before restoration, and verifies the
   restored caller data and `e(sample)` signature.
4. **K3 — LOCAL IMPLEMENTATION COMPLETE:** compressed FE transpose, Schur
   actions, diagonal preconditioning, reconstruction, RSS, and complete
   original worker-plus-firm residual certificates run without retained rows.
5. **K4 — LOCAL IMPLEMENTATION COMPLETE:** the exact no-control match formula
   and fused cell target contractions avoid per-match generic inversions,
   row-sized deleted-adjusted values, and expanded prediction matrices.
6. **K5 — LOCAL IMPLEMENTATION COMPLETE:** overlap-aware compressed/generic
   memory and wall admission precedes probes. The SCC harness runs one Stata
   process, reserves 14 `omp` slots at 4 GiB each, verifies four Stata
   processors separately, stages through `$TMPDIR`, and emits validation and
   accounting receipts. Preliminary final-source CZ24/CZ25 P200 jobs passed
   their scientific gates and exposed a fail-closed process-residency forecast
   omission; no scale rung is qualified from those attempts.
7. **K6 — LOCAL CORRECTNESS FIXTURE COMPLETE; PERFORMANCE PENDING:** P40/P200,
   multi-batch, relabeling, fallback, lifecycle, identity, and residual tests
   are registered. These small timings are not scale performance evidence.
8. **K7 — PENDING MEASURED OPTIMIZATION GATES:** retain further CMG or Krylov
   work only when complete-command wall and memory gates pass.
9. **K8 — SCC CALIBRATION ACTIVE:** resource-API-4 reruns 7203808 and 7203861
   reconciled CZ24/CZ25 on `scc-gr4` and `scc-ei3`. CZ18 P40 job 7203882
   passed scientific, residual, identity, lifecycle, and hard-limit gates and
   independently measured 311,730 coefficient cells and 311,730 deletion
   units. Its 5,739,220,992-byte process peak nevertheless exceeded the
   5,592,348,054-byte registered transition envelope. Resource API 5 keeps
   the separate 96-MiB fixed runtime charge and adds 32 bytes per retained row
   to sorting/compression high-water temporaries, 1.79 times the measured
   17.91-byte-per-row omission. API 5 raw-input reruns 7204143 (P40) and
   7206467 (P200) reconciled at 925 and 1,461 seconds cold wall. The required
   fixed-retained P200 predecessor 7206667 passed all scientific and lifecycle
   gates, including 601 complete RHS certificates with maximum residual
   `9.99040353264e-11`, but its 3,775,438,848-byte numerical RSS peak exceeded
   the registered 3,181,596,634.6-byte peak. Resource API 6 and solver receipt
   API 23 replace the reuse assumption with a structural allocator-overlap
   upper bound: `max(live nonsolver, transition + phase scratch + solve-ahead)
   + routed solver`. The exact 7206667 component regression is
   5,328,065,418.6 bytes before 30-percent headroom. Clean API 6 calibration
   and P200 evidence must reconcile before separate well-connected and ring
   2x fixtures and the mandatory well-connected 4x P200 run.
10. **K9 — CONDITIONAL SCC PENDING:** run 8x/16x only when upper forecast
    bounds including 25--30 percent memory and 50 percent wall headroom remain
    within 56 GiB and 12 hours, then close only with experimental scale-
    qualified status.

The source-bound CZ18 baseline has 8,201,888 retained rows. Job 7203882
independently measured 311,730 deletion units and 311,730 coefficient cells;
their equality is an observed property of this retained sample and is not
assumed by the compressed representation. Every 1x/4x/8x/
16x time, memory, or MATLAB value remains a hypothesis until a measured run
records its source measurements, fitted scaling rule, uncertainty/range, and
solver-iteration and I/O assumptions.

The measured API 18 bottleneck baseline remains fixed evidence. Diagnostic
automatic-CMG commands took 37 seconds on CZ24 and 103 seconds on CZ25; their
nested leverage/target/correction timers were 9.564/12.420/21.984 and
23.091/30.642/53.733 seconds. Those concurrent diagnostic cells are
correctness evidence, not isolated performance fits. Full CZ18 took 4,356
warm command seconds (4,652 seconds including preparation), with nested
setup/leverage/target/correction timers 349.061/1,277.846/2,050.918/3,328.764
seconds and Schur/PCG timers 2,208.939/2,822.535 seconds. It reached 24.561
GiB process RSS and 24.931 GiB `qacct maxvmem`. These nested timers do not add
to command wall. They identify row-level leverage/target passes, repeated-RHS
solver traffic, and setup as the current end-to-end targets.

The initial admission model below is deliberately a hypothesis. It scales the
KSS-PROD-1 CZ18 dimensions linearly, temporarily sets coefficient cells and
target strata equal to the provisional 311,730 count, charges 1 GiB of raw
Stata data per CZ18 copy, uses leverage/target batches 32/16 at 1x--4x and
16/8 at 8x--16x, and assumes solver work grows linearly without adverse
connectivity. Its wall upper bound is
`2*(509*row_scale + .25*(4652-509)*structure_scale)` seconds, based on the
4,652-second cold CZ18 baseline and doubled for unmeasured I/O, connectivity,
and iteration uncertainty. Admission then adds 30 percent memory and 50
percent wall headroom.

| scale | rows | deletion-unit proxy | workers | firms | forecast peak GiB | peak +30% GiB | wall upper h | wall +50% h | initial status |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|:---|
| 1x | 8,201,888 | 311,730 | 117,529 | 10,603 | 2.321 | 3.017 | 0.858 | 1.288 | model-admitted |
| 4x | 32,807,552 | 1,246,920 | 470,116 | 42,412 | 9.233 | 12.003 | 3.433 | 5.149 | model-admitted |
| 8x | 65,615,104 | 2,493,840 | 940,232 | 84,824 | 18.465 | 24.005 | 6.866 | 10.298 | conditional only |
| 16x | 131,230,208 | 4,987,680 | 1,880,464 | 169,648 | 36.931 | 48.010 | 13.731 | 20.597 | wall-rejected |

These are allocation-formula outputs, not RSS or runtime measurements. They
do not admit 8x before 1x/2x/4x reconciliation, and they do not establish the
actual coefficient-cell, target-stratum, hybrid-graph, hierarchy, or connector
dimensions of either replication fixture.

The payoff order is fixed:

1. canonical row-to-cell/unit/stratum compression reduces data passes and
   memory traffic first, persistent and scratch memory second, and arithmetic
   where repeated rows formerly entered FE actions; this is the primary
   single-process and scale-enabling change;
2. the scalar match correction and fused worker/firm/cross target contractions
   remove per-unit dense algebra, row-sized deleted predictions, and separate
   target traversals;
3. direct exact binomial sums reduce RNG calls and aggregation traffic without
   changing their distribution where the compression conditions hold;
4. matrix-RHS compressed FE/CMG applications reduce repeated graph traversal,
   allocation, and solver actions only when command wall confirms the gain;
5. disk-backed lifecycle and pre-RNG admission reduce peak overlap and doomed
   cold-path work, while sample-selection/loading optimization begins only if
   measured cold-wall shares make it worthwhile;
6. solve-ahead, a compact CMG workspace, recycled PCG, block PCG, deflation,
   and exact elimination remain optional after the simpler engine is measured.

There is no distributed-wall optimization category: KSS-SCALE acceleration
must occur within one Stata process and one job.

The fast path preserves independent leave-out sample construction, literal
frequency and target-weight meaning, coefficient-one finite projection,
quotient normalization and displayed grounding, complete per-RHS original
normal-equation residuals, target identities, typed failures, and the absence
of hidden regularization. Exact target-scale groups never use tolerance-based
merging. Grouped cancellation-sensitive sums use stable pairwise or
compensated accumulation.

Its exact eligibility contract is JLA, match deletion, no controls, no
deletion unit crossing a coefficient cell, exact target-scale strata, total
physical mass below `2^53`, no more than 16,383 probes, a registered runtime
RNG contract, and an admitted resource forecast. `engine(auto)` uses the
general engine for an ineligible design only after that raw-resident route
passes its own admission test. Forced compressed input gets the typed
eligibility failure. An inadmissible general fallback returns
`GENERIC_RESOURCE_ADMISSION_FAILED` before probes.

For a deletion unit `g` and coefficient cell `c`, the specialized correction
is

\[
D_g=E_g\left(m_g^{-1}+B_gm_g^{-2}-V_gm_g^{-3}\right),\qquad
K_c=\sum_{g\to c}Y_gD_g,
\]

and a target correction draw is \(\sum_c K_cz_c^2\). These formulas inherit
the current meanings, conditioning gates, reciprocal-residual gates, and typed
failure behavior for \(B_g,V_g,m_g\). Multiple deletion IDs at one cell remain
separate until their \(Y_gD_g\) terms enter \(K_c\).

Every fit, leverage, and target RHS uses the original normal-equation
certificate

\[
r_w=b_w-\left[d_w\alpha_w+\sum_{c:w_c=w}F_c\gamma_{f_c}\right],
\qquad
r_f=b_f-\left[e_f\gamma_f+\sum_{c:f_c=f}F_c\alpha_{w_c}\right].
\]

Here `F_c` is the exact physical-observation mass of coefficient cell `c`,
`d_w=sum_(c:w_c=w)F_c`, `e_f=sum_(c:f_c=f)F_c`, and `b_w,b_f` are the
original RHS blocks for that solve.

The solve uses the full-firm zero-sum quotient and then displays the last firm
at zero; the grounded firm's original equation remains in the certificate.
The combined Euclidean residual is divided by the original RHS norm, or kept
absolute for a zero RHS, and must not exceed
`max(1e-11,10*tolerance())`. A graph residual alone never passes.

The API 19 candidate uses standard disk-backed Stata preservation. It marks
the sample, constructs compressed state, forces native `preserve` to disk,
clears the raw dataset, runs the numerical callback, frees large Mata
allocations, and calls `restore`. The local lifecycle fixture verifies data,
filename, dirty state, order, labels/formats/characteristics, and exact
`e(sample)`. A Stata `tempfile` with `save`/`clear`/`use` remains the native
fallback design if SCC measurements show that `preserve` retains material
resident memory. Compare import/selection, compression transition, numerical,
and restoration peaks explicitly. If neither native route can restore the
caller dataset within the envelope, stop and request owner authorization for
an experimental scale-only semantic change; do not weaken normal command
semantics silently. A destructive scale-only mode is not authorized.

The RNG invariant is `KSS-RNG-K1-INVARIANT-V1`. A logical atom depends only on
the versioned runtime contract, master seed, leverage/target domain, probe
index, and canonical semantic atom key/order. Solver route, batching, memory
tiling, convergence history, processor count, and phase scheduling cannot
change atoms. Floating-point estimator reductions need only meet registered
tolerances. Large direct binomial atoms implement the exact distribution
`2*Binomial(F,1/2)-F` through scalar fixed-order calls, with exact chunking at
the documented `1e11` Stata call limit. The tests distinguish call limits,
scalar/vector call shape, and total exact integer representation; a total
below `2^53` is not by itself a sufficient RNG-call contract. Leverage and
target never share a stream. Leverage compresses only the sign sum consumed by
one deletion unit. Target signs compress only within a coefficient cell and an
exact common per-copy target scale; unequal scales remain separate strata.
Frequency one is a direct special case. Observation deletion cannot use this
binomial match compression. Every exit restores the caller RNG algorithm,
selected stream, and complete state.

Resource admission separately exposes resident raw Stata data; persistent
cell, deletion-unit and target-stratum arrays; CMG hierarchy and factors;
phase-specific matrix-RHS scratch; sorting/compression temporaries;
solve-ahead coefficients; output and residual-certificate storage;
preservation overhead; and the maximum overlap. It records selection,
transition, numerical, and restoration peaks. Generic fallback retains the
raw data at numerical peak and must pass its own forecast. Each scale
reconciles phase forecasts with process RSS and `qacct maxvmem`; an
underforecast blocks progression.

Normal scaling and adverse connectivity use different connected,
deletion-safe fixtures. The well-connected construction measures dimension
growth without a progressively weaker cut. The ring measures weak
connectivity and deletion safety and is never the sole source for ordinary 4x
runtime extrapolation. Both record condition proxies, CMG levels/terminal,
iterations, actions, and stage times.

Optimization acceptance is end-to-end. Complex solve-ahead, recycled-PCG, or
block-PCG changes need at least 10 percent repeatable complete-command wall
improvement after dense block algebra. A simple low-risk change may remain
with at least 5 percent repeatable improvement or a demonstrated scale-
enabling reduction in peak memory or required passes. Report cold wall, warm
command wall, CPU, repetitions, and spread; iteration reduction alone does not
qualify. Cold wall includes loading, sample selection, compression,
restoration, and output validation; optimize selection if its measured share
becomes Amdahl-limiting. Stop after mandatory 4x P200 qualification
when no remaining candidate projects at least 5 percent end-to-end improvement
or enables an otherwise inadmissible larger scale.

The maintained MATLAB LeaveOutTwoWay program remains a descriptive timing and
sample-selection comparator. Its executable startup, import, selection, pool
startup, MEX setup, core estimator, serialization, teardown, cold wall, warm
call, CPU, and peak RSS stay separate. Corrected estimates are not equality
targets because MATLAB retains its legacy formula and probe schedule. Every
scale and MATLAB forecast remains a hypothesis until a source-bound run states
the source measurements, fitted rule, uncertainty/range, solver-iteration and
I/O assumptions, and actual outcome. Use the same fixed retained sample for a
core-computation comparison and an independent retained-match comparison for
sample selection. Run connected 2x and 4x MATLAB cases only when feasible; if
one fails, preserve the full input, exact failure, dimensions, elapsed time,
and memory without rewriting the maintained estimator or weakening the case.

Implementation ownership remains disjoint by package:

- compressed algebra and RNG:
  `kss_bc/kss_bc_scale.mata`, `kss_bc/kss_bc_rng.mata`, and their focused
  Python/Mata oracles;
- command and lifecycle bridge: `kss_bc/kss_bc.ado`,
  `kss_bc/kss_bc_scale_engine.mata`, `kss_bc/kss_bc_scale_runtime.mata`,
  `kss_bc/kss_bc_lifecycle.ado`, `kss_bc/kss_bc_resource.mata`, package
  metadata, and command/lifecycle/resource tests;
- solver adapter and diagnostics: `kss_bc/kss_bc_solver.mata` and narrowly
  related route tests; `shared/cmg/**` only for an accepted CMG kernel change;
- scale fixtures and one-job harness: `kss_bc/benchmarks/kss_scale_fixtures.*`,
  `kss_bc/benchmarks/scc/*kss_scale*`, the scale validator, and their tests;
- documentation: `kss_bc/README.md`, `kss_bc/kss_bc.sthlp`,
  `kss_bc/TESTING.md`, `kss_bc/CHANGELOG.md`, `kss_bc/AGENTS.md`, this plan,
  `cmg_plan.md`, and the eventual KSS-SCALE aggregate report.

No KSS-SCALE change may touch proof, manuscript, application, release,
archive, theorem-status, frozen-state, `ppml_talo/**`, or maintained MATLAB
source. The MATLAB implementation never becomes a runtime dependency.
Checkpoint commits are rollback boundaries: K0 documentation (`9d6a5ea`),
the local algebra/RNG/lifecycle/resource engine, the local integrated package,
the source-bound SCC harness, then measured qualification evidence. Do not
mix an unaccepted optional Krylov/CMG experiment into a passing compressed-
engine checkpoint; revert that experiment at its own boundary if its wall or
memory gate fails.

Completion requires a single restored-semantics Stata process to pass the
well-connected 4x fixture with 200 probes, every scientific identity and
complete residual, peak use within 56 GiB, and wall below 12 hours. The
result is experimental scale qualification, not public-release or production
status.

## Goal

Build an internal Stata/Mata 18--19 package that implements KSS point-estimate
bias correction for the worker variance, firm variance, worker--firm
covariance, and total variance in a linear two-way fixed-effect model. The
package must support observation and actual-match deletion, exact and improved
JLA algorithms, joint and fixed-offset controls, literal integer-frequency
semantics, target weights, explicit mover/stayer reporting, and production-
shaped matrix-free computation. It remains separate from `ppml_talo` in this
series.

## KSS-PROD-1 replacement checkpoint — COMPLETE, NOT QUALIFIED

The production milestone promotes CMG into the installed package, replaces
MATLAB-retained sample input with a deterministic deletion-unit multigraph
fixed point, routes exact/B1/bounded-terminal CMG/multilevel CMG before random
probes, and optimizes the complete leverage and target paths through matrix
batches. The public command returns route, sample, graph, hierarchy, RHS,
timing, memory, probe, seed, tolerance, and typed fallback diagnostics.

Local candidate integration is complete when the Python oracles, quick/full
Stata suites, clean install, CMG core and namespace gates, driver smokes,
batch invariance, forced-route equality, and deterministic 2x stress fixture
pass. The production qualification contract required source-bound SCC evidence
for CZ24/CZ25 sample and route comparisons, CZ18 preflight/calibration and the
full 200-probe estimator, the connected 2x-CZ18 stress run, Stata 18/19, the
SCC's available four-processor modules, local MP8 behavior, qacct/RSS, and the
repository full gate. SCC run `20260816T035454Z-c3cb6a3` established that
both installed Stata modules are capped at four processors; the Stata 18
module rejects an eight-slot job and Stata 19 reports four actual processors.

Source-bound candidate `5e2687c6a12c221ad899f1b31227b2f81693d383`
completed the CZ24/CZ25 gates and full CZ18 job `7197620`: 8,201,888 retained
rows, a 28,577-vertex/169,591-edge nine-level hybrid, 601 accepted RHSs,
maximum complete residual `9.9601e-11`, 4,356 command seconds, and 24.561 GiB
peak RSS. The candidate did not pass the required larger stress gate. Parallel
jobs `7197621`--`7197623` all built the same 57,154-vertex/339,183-edge
ten-level hybrid and then withheld before RNG because neither B1 nor CMG
passed all bounded routing gates. The production validator rejects their
nonzero qacct exits. It therefore created no stress projection, and no P200
stress job was authorized. The complete report is
`benchmarks/reports/KSS_PROD_1_2026-08-16.md`.

This closes the current source-bound qualification as failed. It does not
disable or remove the installed candidate, and it makes no production claim.
The historical KSS-NUMOPT statements below describe API 17 at the time; its
B1-only and uninstalled-CMG operational statements are superseded by API 18.
A future owner-authorized milestone must diagnose and repair the large-stress
pilot failure before repeating qualification. No such optimization wave is
part of this checkpoint.

## Fixed inputs and decisions

- Governing KSS requirements: this plan, the owner decisions in
  `docs/DECISIONS.md`, and the point-estimator contract in
  `docs/ESTIMATOR_CONTRACT.md`.
- `varcomp_naming.md` is the deferred brief for future package unification. It
  does not alter the current KSS milestones or estimator contract.
- Primary behavioral oracle: `rsaggio87/LeaveOutTwoWay` at commit
  `8b957ffeb10b8465a3584fceb0265cccc48379e1`.
- Mathematical sources: KSS (2020), its computational appendix, the authors'
  `doc/improved_JLA.tex`, and the independent derivation in
  `docs/JLA_FINITE_PROJECTION.md`.
- The R package `LeaveOutKSS` 0.1.0 is a secondary translation by Vahid
  Moghani, not an authoritative KSS source.
- The corrected mixed fourth-moment coefficient is one. MATLAB coefficient
  two remains test-only legacy evidence.
- Match-mode headline targets use the mover fit and mover target population.
  A MATLAB-style all-worker hybrid is optional and separately labeled.
- KB6 qualification uses public and synthetic data only. KSS-NUMOPT-1 has a
  separate owner-authorized, SCC-only, read-only Separations wage benchmark;
  restricted rows remain outside Git and outside local storage.

## Milestones

### KB0 — governance, provenance, and independent oracle — COMPLETE

Deliverables: local rules, this plan, public contract, source ledger, exact
symbolic/dense oracle, coefficient-discrepancy test, and clean-install
skeleton.

Exit: Python oracle tests pass; Stata loads the package skeleton; all source
and license boundaries are recorded.

### KB1 — sample, target, graph, and exact estimator — COMPLETE

Deliverables: deterministic ID encoding; sample marking; connected and
leave-out-estimable mover set; deletion blocks; target centering; dense exact
observation/match correction; frequency and target-weight semantics; typed
failures.

Exit: Mata exact results match independent dense and brute-force refit oracles
on registered fixtures, including general within-match controls.

### KB2 — matrix-free fit and inverse actions — COMPLETE

Deliverables: exact worker elimination, full firm-mobility quotient PCG with
post-solve display grounding,
low-dimensional joint-control Schur complement, scalar/batched PCG, quotient
normalization, and recomputed full-system residual checks.

Exit: matrix-free fits and inverse actions match dense systems; no production
path allocates an observation-space matrix.

### KB3 — improved JLA and scalable block deletion — COMPLETE

Deliverables: indexed Rademacher stream, constrained P/M estimates, corrected
fourth moments, block residual-leverage sketches, common target contractions,
batch invariance, conditional numerical MCSE, and literal-frequency probe
aggregation.

Exit: JLA converges to exact targets; seed and batch gates pass; coefficient-
one bias simulation passes; frequency collapse matches literal expansion.

### KB4 — public command, package, and local qualification — COMPLETE

Deliverables: complete ado interface and `e()` contract, help, examples,
failure catalog, internal install bundle, Stata 18 quick/full/install suites,
benchmark drivers, and a package-local qualification runner.

Exit: all local gates pass and the worktree contains no generated logs or
untracked data.

### KB5 — mathematical review — IN PROGRESS

Deliverables: two independent GPT Pro reviews of the finite-projection and
block-control derivations, verbatim records, validation, and local
adjudication.

Exit: no unresolved critical objection; status may advance to `ai_reviewed`
but not `independently_checked`.

### KB6 — SCC public/synthetic qualification — IN PROGRESS

Deliverables: source-bound deployment, Stata 19 portability job, paired
MATLAB/Stata oracle job, synthetic scale ladder, qacct/log/output validation,
collected compact evidence, and a qualification report.

Exit: every accepted job passes scheduler, application, and output gates.
Final status is limited to public/synthetic point-estimation qualification;
restricted-data production and package unification remain open.

### KSS-NUMOPT-1 — lockstep PCG and forced CMG evaluation — SCC QUALIFIED TEST-ONLY

Deliverables: frozen B0 evidence; true lockstep diagonal B1; separated setup,
Schur, preconditioner-application, PCG, leverage, target, and total timings;
per-RHS iteration/residual diagnostics; a memory-rich shared-CMG profile; a
forced test-only KSS adapter; and easy/moderate/weak B1/C benchmarks.

Outcome: B1 passes estimator and solver equivalence and materially outperforms
B0. API 17 routes B1 and forced CMG through one lockstep PCG kernel; the
public syntax and installed route remain B1-only. The forced test-only
end-to-end CMG estimator preserves the public sample construction, probe
stream, seed, formulas, tolerance, quotient normalization, grounding, worker
reconstruction, and complete residual gate. Source-bound Stata 19 SCC tests
at 10,000 workers, 1,000 firms, and 200 probes record C/B1 end-to-end gains of
1.725x on moderate and 4.620x on weak, estimator-matrix relative differences
of `3.23e-12` and `2.01e-11`, and complete residuals below `1e-10`. Forced C
fails closed on easy. Automatic routing is not implemented because easy and
real-data gates do not pass.

The source-bound SCC ladder is limited to easy, moderate, and weak cases at
the same bounded dimensions. Every B1, C, and MATLAB estimator job must carry
a measured projection at or below 5,400 seconds and is independently stopped
at 5,400 seconds. No large synthetic or real-data step may be submitted. The
read-only real-data ladder starts with a deterministic 5,000-worker CZ24 wage
slice. A natural full-CZ input may run only route by route after the small
calibration projects that route to at most 90 minutes. MATLAB LeaveOutTwoWay
is a checksum-bound timing and descriptive-result reference; its legacy
finite-projection coefficient and language-specific stream are not an API 17
equality oracle.

SCC status: portability job `7185628`, smoke job `7185639`, and medium B1 job
`7185654` pass. Medium completes in 1,580 seconds with 432,284 KB peak RSS and
a 2.45x end-to-end speedup over frozen scalar B0; its scaling projects roughly
17.5 hours for large, so no new large B1 was submitted. The source-bound
forced-C run `20260815T142657Z-3ac4abe` passes scheduler, application, output,
estimator-equality, residual, timing, and RSS gates on moderate and weak, while
easy fails closed. The read-only Separations ladder tests a 500-worker dense
core and the larger all-eligible-mover sample. Both B1 executions reject the
same nonestimable match deletion; CMG never converts that rejection to a
success. The checksum-bound MATLAB reference passes separately on its own
smaller maintained leave-one-out set; it is descriptive rather than an API 17
equality oracle. At the owner's direction, obsolete scalar-B0 job `7185180`
was cancelled and collected as `USER_CANCELLED_OBSOLETE_B0`; exit 137 is not
accepted numerical evidence.

The active bounded real-data plan is MATLAB-first and diagnostic. A successful
maintained MATLAB retained-match file may define a benchmark sample only. An
SCC-only adapter joins its checksum-bound match keys back to every physical
row, repeats KSS graph pruning, and removes audited match bridges to a fixed
point. Exact must pass first on the derived sample. Only then may B1 and forced
C run on the identical DTA, probes, seed, tolerance, target, and formulas.
Scale-up requires estimator equality, complete residuals, stage timings, peak
RSS, successful SCC accounting, and a measured projection no larger than 90
minutes for each route. This diagnostic path must not alter the public sample
selector or make production KSS depend on MATLAB. Automatic routing remains
disabled.

Dense adapter job `7188798` passed with 11,549 rows, 216 workers, 69 firms,
538 matches, and no additional graph removals. Exact jobs `7188809` and
`7188878` then failed closed while forming a raw projection eigenproblem; B1
and C were not submitted from either failure. API 17 replaced that operation
with the smaller positive-definite observation-space or reduced Woodbury
residual maker and retained the complete action-residual check.

Source-bound API 4 run `20260815T221100Z-f0dd3ec` completes the bounded
MATLAB-retained ladder. The small exact oracle and B1/C comparison pass on
11,549 rows; exact/B1 plug-in `mreldif` is `8.07e-12`, B1/C estimator
`mreldif` is `1.16e-11`, and C is 2.283x faster. The post-oracle all-mover
step passes on 256,472 rows, 4,063 workers, 1,285 firms, and 10,343 matches
with no additional removal. B1 takes 416.750 seconds and forced C 101.096
seconds, a 4.122x command speedup. Their estimator `mreldif` is `1.29e-10`;
maximum freshly recomputed complete residuals are `9.998e-11` and
`4.052e-13`. GNU time peak RSS is 443,140 and 490,172 KiB. Every estimator
projection was 900 seconds and each process retained the 5,400-second hard
timeout. CMG uses one 25,776,200-byte factor for the 1,796-vertex,
5,948-edge hybrid and solves every RHS in one PCG step.

The real-data equality, residual, speed, memory, Stata 19, and SCC gates now
pass for this forced route. Automatic routing remains disabled because easy
graphs retain their typed CMG rejection, the core is test-only and uninstalled,
and package promotion/no-regression review is incomplete. Public KSS remains
B1-only; future routing should target repeated-RHS moderate or weak systems,
not easy or well-conditioned graphs.

The next-larger CZ25 scale step passes under source-bound commit `7f07d27`.
MATLAB, B1, and forced CMG ran concurrently on an audited 390,128-row bridge
core with 5,825 workers, 1,780 firms, and 15,097 matches. Three-way retained
match overlap is exact. B1 takes 642.986 seconds and CMG 166.550 seconds, a
3.861x CMG speedup; estimator `mreldif` is `2.17e-11`, and maximum complete
solver residuals are `9.95e-11` and `4.04e-13`. MATLAB takes 6.924 command
seconds plus 5.457 seconds MEX setup, but 156 seconds SCC wall after pool
startup. Its corrected total is about 1.18% below B1 because its legacy
finite projection and probe stream remain descriptive, not an API 17 equality
oracle. Every job passes SCC accounting and the 90-minute rule. Full aggregate
evidence is in `benchmarks/scc/CZ25_COMPARISON_2026-08-15.md`. Automatic
routing remains disabled.

## Completion gates

- `kss_bc/tests/stata/run_all.do quick`, `full`, and clean install pass on
  Stata 18.
- `./.venv/bin/python -m pytest kss_bc/tests/python -q` passes.
- `./.venv/bin/python kss_bc/tools/run_checks.py` and the repository
  `tools/run_checks.py --scope full` pass.
- Exact, JLA, controls, deletion, frequency, target, mover/stayer, accounting,
  failure, invariance, and reproducibility tests pass.
- Required Pro reviews are stored and adjudicated.
- SCC Stata 19 evidence passes the three-layer acceptance rule.
- No protected path changed; public release remains blocked by licensing.

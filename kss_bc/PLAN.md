# `kss_bc` living implementation plan

- Plan ID: `KSS-BC-DEV-2026-08`
- Milestone series: KB0--KB6, KSS-NUMOPT-1, and KSS-PROD-1
- Branch/worktree: `main`, current worktree; no branch or worktree creation
- Owner: Johannes
- Start date: 2026-08-14
- Status: active
- Historical base commit: `2b3bb6b15b6a6d4d638c8ec6e7d3eea8641a7abb`
- KSS-PROD-1 base commit: `0e2fdd2b1d810f507f6768b4b77f2461ecabfd10`
- Allowed changes for KSS-PROD-1: `kss_bc/**`, `shared/cmg/**`,
  `cmg_plan.md`, and narrowly related root workflow text; the hash-frozen
  repository runner remains unchanged
- Protected: `ppml_talo/**`, `application/**`, `software/**`, `paper/**`,
  `theory/**`, `proof-audit/**`, `state/**`, `archive/**`, `paper/releases/**`

## Goal

Build an internal Stata/Mata 18--19 package that implements KSS point-estimate
bias correction for the worker variance, firm variance, worker--firm
covariance, and total variance in a linear two-way fixed-effect model. The
package must support observation and actual-match deletion, exact and improved
JLA algorithms, joint and fixed-offset controls, literal integer-frequency
semantics, target weights, explicit mover/stayer reporting, and production-
shaped matrix-free computation. It remains separate from `ppml_talo` in this
series.

## KSS-PROD-1 replacement checkpoint — ACTIVE

The production milestone promotes CMG into the installed package, replaces
MATLAB-retained sample input with a deterministic deletion-unit multigraph
fixed point, routes exact/B1/bounded-terminal CMG/multilevel CMG before random
probes, and optimizes the complete leverage and target paths through matrix
batches. The public command returns route, sample, graph, hierarchy, RHS,
timing, memory, probe, seed, tolerance, and typed fallback diagnostics.

Local candidate integration is complete when the Python oracles, quick/full
Stata suites, clean install, CMG core and namespace gates, driver smokes,
batch invariance, forced-route equality, and deterministic 2x stress fixture
pass. Production qualification remains open until source-bound SCC evidence
passes CZ24/CZ25 sample and route comparisons, CZ18 preflight/calibration and
the full 200-probe estimator, the connected 2x-CZ18 stress run, Stata 18/19,
the SCC's available four-processor modules, local MP8 behavior, qacct/RSS, and
the repository full gate. SCC run `20260816T035454Z-c3cb6a3` established that
both installed Stata modules are capped at four processors; the Stata 18
module rejects an eight-slot job and Stata 19 reports four actual processors.

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

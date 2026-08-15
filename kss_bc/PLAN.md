# `kss_bc` living implementation plan

- Plan ID: `KSS-BC-DEV-2026-08`
- Milestone series: KB0--KB6 plus KSS-NUMOPT-1
- Branch/worktree: `main`, current worktree; no branch or worktree creation
- Owner: Johannes
- Start date: 2026-08-14
- Status: active
- Base commit: `2b3bb6b15b6a6d4d638c8ec6e7d3eea8641a7abb`
- Allowed changes for KSS-NUMOPT-1: `kss_bc/**`, `shared/cmg/**`, and
  `cmg_plan.md`; the hash-frozen repository runner remains unchanged
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

### KSS-NUMOPT-1 — lockstep PCG and forced CMG evaluation — SCC CANDIDATE

Deliverables: frozen B0 evidence; true lockstep diagonal B1; separated setup,
Schur, preconditioner-application, PCG, leverage, target, and total timings;
per-RHS iteration/residual diagnostics; a memory-rich shared-CMG profile; a
forced test-only KSS adapter; and easy/moderate/weak B1/C benchmarks.

Local outcome: B1 passes estimator and solver equivalence and materially
outperforms B0. API 15 now routes B1 and forced CMG through one lockstep PCG
kernel; the public syntax and installed route remain B1-only. The forced
test-only end-to-end CMG estimator preserves the public sample construction,
probe stream, seed, formulas, tolerance, quotient normalization, grounding,
worker reconstruction, and complete residual gate. At 10,000 workers, 1,000
firms, and 200 probes, forced C fails closed on the easy graph, gives a 1.35x
end-to-end gain on the moderate graph, and a 3.60x gain on the weak graph.
B1/C estimator-matrix relative differences are `3.24e-12` and `2.01e-11`.
Automatic routing is not implemented because the easy rejection, pending
Stata 19/RSS evidence, and pending real-data evidence leave the promotion
gates open.

The source-bound SCC ladder is limited to easy, moderate, and weak cases at
the same bounded dimensions. Every B1, C, and MATLAB estimator job must carry
a measured projection at or below 5,400 seconds and is independently stopped
at 5,400 seconds. No large synthetic or real-data step may be submitted. The
read-only real-data ladder starts with a deterministic 5,000-worker CZ24 wage
slice. A natural full-CZ input may run only route by route after the small
calibration projects that route to at most 90 minutes. MATLAB LeaveOutTwoWay
is a checksum-bound timing and descriptive-result reference; its legacy
finite-projection coefficient and language-specific stream are not an API 15
equality oracle.

SCC exit: collect the unchanged B0 job `7185180`; then run a source-bound
short B1 portability/smoke step and inspect scheduler/application/output
evidence before authorizing any larger step. No 12-hour-or-longer submission
is justified automatically. Source commit
`b2ef752684a7f5267aa09e6979700571b5e1b9c0` is staged as SCC run
`20260815T121809Z-b2ef752`. Portability job `7185628` and smoke job `7185639`
pass all three evidence layers in 14 seconds each, with 79,956 KB and 85,568
KB peak RSS. Medium job `7185654` passes in 1,580 seconds with 432,284 KB peak
RSS and a 2.45x end-to-end speedup over frozen scalar B0. Its size/iteration
scaling projects roughly 17.5 hours for large, so large B1 remains unsubmitted.

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

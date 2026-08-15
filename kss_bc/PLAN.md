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

Outcome: B1 passes estimator and solver equivalence and materially outperforms
B0. API 15 routes B1 and forced CMG through one lockstep PCG kernel; the
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
finite-projection coefficient and language-specific stream are not an API 15
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
smaller maintained leave-one-out set; it is descriptive rather than an API 15
equality oracle. Therefore no full-input estimator was submitted and
automatic routing remains disabled. Continue to monitor and eventually
collect the unchanged B0 job `7185180` without altering it.

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

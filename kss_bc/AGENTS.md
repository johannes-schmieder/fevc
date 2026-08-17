# AGENTS.md

## Scope

This folder is the independent Stata/Mata implementation of Kline--Saggio--
Sølvsten leave-out bias-corrected point estimates for linear worker--firm
variance decompositions. It is developed beside `ppml_talo/`; package
unification is a later owner-authorized task.

KSS-PROD-1 and the owner-authorized KSS-SCALE-1 follow-up may also change
`shared/cmg/**` and `cmg_plan.md` because the clean-room core is part of
their numerical qualification. The root
`tools/run_checks.py` is hash-frozen by the desktop handover. Treat
`ppml_talo/`, `application/`, `software/`, `paper/`, `theory/`, `proof-audit/`,
`state/`, `archive/`, and `paper/releases/` as read-only inputs.

Use `main` and the current worktree. Do not create or switch branches or
worktrees. Do not push without a separate owner request.

## Statistical contract

- Point estimates only. Never post `e(V)` or label numerical probe dispersion
  as an econometric standard error.
- The four targets are worker variance, firm variance, worker--firm
  covariance, and the variance of their sum.
- Match deletion is the default. `deletionid()` defines the deletion unit
  independently of the worker and firm coefficient coordinates.
- Under match deletion the headline population is movers. A MATLAB-style
  all-worker stayer fallback may be returned only as a separately labeled
  hybrid result.
- `nuisance(joint)` lets controls move under deletion. The
  `nuisance(fixedoffset)` result is conditional on the full-sample nuisance
  index and must be labeled that way.
- Frequency weights are positive integer counts of literal physical copies.
  Observation deletion removes one copy; match deletion removes the complete
  declared block.
- Explicit target weights are total stored-row target mass and are not
  multiplied by frequency weights.
- Do not silently change the retained component, target population, deletion
  unit, probe count, tolerance, or estimator.

## Improved-JLA decision

The production finite-projection correction uses coefficient one on the mixed
fourth moment:

`B = R^-1 { M m(P^2) - P m(M^2) + (M-P) m(P,M) }`.

This follows from the Hessian of `Mhat/(Mhat+Phat)`, the authors' final
raw-moment display, exact symbolic reduction, and independent simulation. The
current MATLAB code's coefficient two is retained only as a documented legacy
oracle fixture. Do not expose it as a production option.

## KSS-SCALE-1 experimental contract

- API 19's compressed engine is specialized to JLA, match deletion, no
  controls, deletion units contained in one worker--firm coefficient cell,
  exact target-scale strata, fewer than `2^53` physical copies, no more than
  16,383 probes, a registered runtime RNG contract, and an admitted memory and
  wall forecast. Preserve the general engine for all other supported designs.
- Keep coefficient cells, deletion units, and exact target-scale strata as
  different indices. Multiple deletion IDs may share one cell. Never merge
  target scales by tolerance; use stable pairwise or compensated accumulation
  when regrouping can cancel.
- Treat 311,730 as the measured CZ18 deletion-unit count and only a provisional
  coefficient-cell count until an independent retained-sample diagnostic runs.
  Never infer one count from the other. Treat scale and MATLAB forecasts as
  hypotheses until source-bound measurements replace them.
- The no-control match calculation is exactly
  `D_g=E_g(m_g^-1+B_g*m_g^-2-V_g*m_g^-3)`,
  `K_c=sum_(g->c)Y_g*D_g`, and target draw `sum_c K_c*z_c^2`. Retain all
  existing meanings, coefficient-one moments, conditioning gates,
  reciprocal-residual gates, and typed failures for `B_g`, `V_g`, and `m_g`.
- Every fit, leverage, and target RHS must pass the complete original worker
  and firm normal-equation residual. Solve on the full-firm zero-sum quotient,
  display the last-firm ground only afterward, check its original equation,
  scale by the original RHS Euclidean norm (absolute for a zero RHS), and
  enforce `max(1e-11,10*tolerance())`. A graph residual never substitutes.
- Probe atoms depend only on RNG-contract version, master seed, domain, probe
  index, and canonical semantic atom identity/order. Batch, tile, solver route,
  convergence history, processor count, and scheduling may change reductions
  but not atoms. Leverage and target domains remain separate. Restore the
  caller's RNG algorithm, stream, and complete state on every exit. Register
  golden vectors per Stata runtime or fail closed; never add a floating-point
  hash RNG.
- Use only Stata's native disk-backed `preserve`/`restore` or `tempfile`
  lifecycle. Release row data before numerical peak, free large Mata state
  before restoration, and restore caller data and exact `e(sample)` semantics.
  A destructive scale-only mode needs separate owner authorization.
- Forecast both compressed and generic routes before probes. Include resident
  Stata/runtime overhead, raw data, persistent compressed arrays, CMG state,
  phase scratch,
  compression temporaries, solve-ahead storage, outputs/certificates,
  preservation overhead, and maximum overlap. Add 25--30 percent memory and
  50 percent wall headroom and remain within 56 GiB and 12 hours.

## Milestone workflow

1. Read `PLAN.md` before substantive work.
2. Work on one `KB`, `KSS-PROD`, or `KSS-SCALE` checkpoint at a time and keep
   its evidence with the code.
3. Add an independent failing test before or with every numerical repair.
4. Keep dense/brute-force oracle code independent of the Mata production
   implementation.
5. Record seeds, tolerances, Stata version, platform, source commit, and input
   hashes for numerical evidence.
6. Never form an observation-by-observation matrix in the production path.
7. Reject unidentified, singular, nonestimable, or unconverged calculations
   with a typed status; never repair them by hidden regularization.
8. Recompute full-system residuals before accepting an iterative solve.
9. Preserve target accounting identities probe by probe where randomized
   contractions are used.
10. Run local quick/full/install gates, the SCC evidence validator, and the
    repository full check before closing the series.

## Review and status language

- `formula_verified`: independent finite algebra/oracles agree at registered
  tolerances.
- `ai_reviewed`: a stored GPT Pro review was locally adjudicated.
- `public_synthetic_qualified`: local Stata 18 and SCC Stata 19 public or
  synthetic gates passed.
- `independently_checked`: only a named human reviewer with a stored report.

The finite-projection and general block-control derivations require two fresh,
independent GPT Pro reviews after candidate completion. Model review cannot
assign human-independent status.

## Runtime, performance, and SCC

- Numerical runtime is one Stata/Mata 18 or 19 process per estimate. API 19
  JLA must fail closed on a runtime without a registered RNG golden vector.
- Python, MATLAB, and R are development oracles, never runtime dependencies.
- The installed public command owns deterministic leave-out sample selection,
  fixed-point deletion-unit bridge removal, and exact/diagonal/CMG automatic
  routing. Routing and any admissible fallback finish before estimator RNG.
- CMG is a supported installed backend. Forced CMG never falls back. Every
  accepted RHS retains the complete original-system residual certificate.
- Automatic probe batching is deterministic from retained dimensions, probe
  count, processors, and the declared memory envelope; it cannot change the
  logical probe atoms. The compressed engine may choose different leverage
  and target matrix widths.
- Encode identifiers densely, eliminate worker coordinates exactly, solve the
  full firm-mobility Laplacian on its zero-sum quotient, ground the displayed
  coordinate only after convergence, and stream probe batches.
- Keep large SCC attempts under `/projectnb/welfgr/kss-bc/runs/<run-id>`.
  Submit with `qsub -P welfgr`; never run sustained compute on login nodes.
- Accept SCC evidence only when `qacct`, application logs, and validated
  outputs all pass. Preserve failed attempts under distinct run IDs.
- The owner authorized SCC Separations production qualification for
  KSS-PROD-1 on 2026-08-15. The Separations checkout and existing wage
  artifacts are read-only. Restricted rows and identifiers remain under
  `/projectnb/welfgr/`; only privacy-safe aggregate results, timings, residuals,
  source hashes, and scheduler accounting may enter this repository. Derived
  row-level inputs and retained-match files stay in the SCC run directory and
  must not be copied locally or committed.
- Build one immutable content-addressed lean source bundle per commit; never
  transfer the repository root. KSS-SCALE estimation is not sharded: each
  experiment is one SGE job, one Stata process, and no coefficient files or
  numerical reducer across jobs or nodes.
- Separate scheduler reservation from application processors. The provisional
  large-job request is `-pe omp 14` with `mem_per_core=4G`; those slots reserve
  CPU, memory, I/O, and shared-node capacity. The one Stata process must set
  and verify `c(processors)==4`. Record requested/actual slots, requested/
  actual Stata processors, process RSS, and `qacct maxvmem` separately. Refine
  the 14x4-GiB reservation only from accounting evidence.
- Stage large inputs and native Stata preservation to node-local `$TMPDIR`.
  Set hard timeouts from measured calibrations and record the source data,
  fitted scaling rule, uncertainty, solver-iteration and I/O assumptions. A
  timeout is a safety boundary, not scientific success. Do not submit an
  unexplained long job. Declare at most 56 GiB to KSS and report all phase
  peaks plus actual RSS.
- Use a well-connected, deletion-safe fixture for ordinary scale extrapolation
  and a ring only as a separate weak-connectivity stress. Record graph
  condition proxies, hierarchy changes, iterations, and actions for both.
- Accept complex mechanisms only with at least 10 percent repeatable complete-
  command improvement. Accept simple low-risk changes with at least 5 percent
  repeatable gain or a measured scale-enabling memory/pass reduction. Include
  cold wall, warm wall, CPU, repetitions, and spread. Iterations alone do not
  count. Stop after mandatory 4x P200 qualification when no candidate projects
  a 5 percent end-to-end gain or enables a new scale within hard limits.

## Licensing

The repository has no selected public software license. The maintained MATLAB
repository also lacks a resolved license. Implement from mathematical formulas
and observed behavior; do not copy or redistribute MATLAB source. Internal
install tests are allowed, but public release or publication is not.

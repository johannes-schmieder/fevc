# AGENTS.md

## Scope

This folder is the independent Stata/Mata implementation of Kline--Saggio--
Sølvsten leave-out bias-corrected point estimates for linear worker--firm
variance decompositions. The companion paper lives in the sibling
`varcomp_kss_paper` repository.

`KSS-NUMOPT-2` (Optimization III) is complete. The active solver checkpoint is
`CMG-MATA-1`, authorized on 2026-08-18 after the owner stopped the other KSS
thread. It may change `kss_bc/**`, CMG integration notes in `cmg_plan.md`, and
the shared CMG source/tests/build artifacts specified in
`shared/cmg/plans/CMG_MATA_1.md`. Preserve behavior for every non-KSS CMG
target and every estimator/probe/residual contract. The generated PPML CMG
target is retained as a compatibility artifact, but this repository has no
live dependency on a PPML checkout.

Use `main` and the current worktree. Do not create or switch branches or
worktrees. Do not push without a separate owner request.

## Active development objective

Remove the CMG hierarchy setup cliff at worker degree four and make degrees
five through seven robust using a source-informed GPL Mata hierarchy, while
preserving every estimator and numerical gate. Run local correctness and
quick benchmarks first, then the source-bound parallel SCC Stata/MATLAB matrix
specified by `CMG-MATA-1`. Completed Optimization III is the frozen
performance/evidence baseline.

The owner supplies the target dataset, probe count, and available resources
for each future development thread. Do not infer a next scale from an earlier
receipt and do not require a lower-rung run to unlock another experiment.

## Gate taxonomy

The following are hard gates:

- supported input and deletion semantics;
- identification, estimability, and absence of hidden regularization;
- convergence under the user-supplied tolerance and iteration limit;
- the complete original worker-plus-firm normal-equation residual for every
  accepted right-hand side;
- exact target accounting identities and finite outputs;
- the direct predicted or observed allocation fitting the declared memory;
- restoration of caller data, `e(sample)`, RNG algorithm, streams, complete
  RNG state, and sort-jumbler state; and
- compatibility of a runtime module actually needed by the selected path.

The following are advisory evidence:

- wall-time forecasts and elapsed-time targets;
- memory headroom above the direct peak;
- projected work, iteration comparisons, and route performance models;
- cold/warm timing repetitions and percentage-improvement thresholds;
- SCC accounting and source-bound scale experiments; and
- extrapolations from synthetic fixtures or prior datasets.

An advisory miss should be reported and used to improve the model. It must not
withhold a statistically valid command. Scheduler timeouts and node-local disk
capacity remain concrete execution limits when an SCC run is actually made.

## Statistical contract

- Point estimates only. Never post `e(V)` or call probe dispersion an
  econometric standard error.
- The targets are worker variance, firm variance, worker--firm covariance,
  and the variance of their sum.
- Match deletion is the default. `deletionid()` defines the deletion unit
  independently of worker and firm coefficient coordinates.
- Under match deletion the headline population is movers. Return any stayer
  hybrid only under a separate label.
- `nuisance(joint)` lets controls move under deletion.
  `nuisance(fixedoffset)` is conditional on the full-sample nuisance index.
- Frequency weights are positive integer physical-copy counts. Observation
  deletion removes one copy; match deletion removes the declared block.
- Explicit target weights are stored-row target mass and are not multiplied by
  frequency weights.
- Never silently change the retained component, population, deletion unit,
  probe count, tolerance, algorithm, or estimator.

The improved-JLA finite-projection correction uses coefficient one on the
mixed fourth moment:

`B = R^-1 { M m(P^2) - P m(M^2) + (M-P) m(P,M) }`.

The coefficient-two MATLAB expression remains a legacy oracle only.

## Numerical contract

- Keep coefficient cells, deletion units, and exact target-scale strata as
  separate indices. Multiple deletion IDs may share a coefficient cell.
- Never merge target scales by tolerance. Use stable or compensated
  accumulation where grouped terms can cancel.
- Do not form an observation-by-observation matrix in production.
- Solve on the full-firm zero-sum quotient. Ground the displayed last-firm
  coordinate only after convergence and still check its original equation.
- Scale the complete residual by the original RHS Euclidean norm, or use the
  absolute residual for a zero RHS. Enforce `max(1e-11,10*tolerance())`.
- A graph, Schur, or recursive residual never substitutes for the complete
  original-system residual.
- Reject unidentified, singular, nonestimable, or unconverged calculations
  with a typed status.

## Routing and resources

Automatic routing is structural and completes before estimator RNG:

1. Explicit `preconditioner(diagonal)` uses diagonal B1.
2. Explicit `preconditioner(cmg)` constructs CMG and fails closed if setup or
   execution fails.
3. `preconditioner(auto)` uses diagonal for structurally small systems or when
   CMG setup is unavailable, and uses CMG when the eligible hierarchy builds.

Do not run trial right-hand sides, apply projected-work cutoffs, or return
`NO_REALISTIC_SOLVER_ROUTE` in the installed automatic path. A failed CMG
setup may fall back to diagonal only before RNG. Once estimation begins, the
selected backend must meet the normal iteration and complete-residual gates.

`memory_gib()` is the caller's positive direct allocation envelope; it has no
repository-wide maximum. The registered 30-percent memory headroom and
50-percent wall allowance are planning diagnostics. `wallseconds()` is an
optional planning/scheduler value and is not a command-level scientific gate.
Automatic batch percentages are heuristics; the complete direct-peak model is
the allocation gate.

## RNG contract

The one-time K1 benchmark selected one stateful `mt64s` stream per domain. Do
not rerun the slow per-probe comparison during ordinary development. JLA uses
separate runtime-scoped Stata 18 and Stata 19 contracts and fails closed on an
unregistered runtime. Exact estimation does not require production RNG
registration.

Production uses streams 1 and 2 and has no 16,383-probe registry cap. Probe
atoms depend on runtime contract, seed, domain, probe index, and canonical
identity derived from observed dense worker, firm, deletion-unit, and target
structure. They are invariant to row permutation, batching, route, processor
count, and scheduling. Arbitrary relabeling of observed IDs may change a draw;
it must never change validity, estimability, or deterministic results.
`probeorder()` is an optional tie-breaker, not a uniqueness requirement.

## Fixtures, SCC, and evidence

`replicated_blocks` is the honest name for the former `well_connected`
fixture; retain `well_connected` only as an input alias. Its reported spectral
quantities describe the connector copy meta-graph, not the full graph. Always
report connector volume relative to the replicated data. `ring` is an adverse
connectivity diagnostic.

No SCC run is required to close `KSS-STREAMLINE-1`. If an SCC run is useful:

- submit one scalar SGE job and one Stata process;
- specify slots, memory per core, Stata processors, wall time, fixture, copies,
  and probes in that run's specification;
- keep restricted rows under `/projectnb/welfgr/`;
- stage large inputs under `$TMPDIR` and enforce actual scheduler, disk, and
  memory limits; and
- preserve a typed estimator failure as a diagnostic result instead of
  treating it as missing evidence.

Source commit and input hashes remain ordinary provenance. A run does not need
a predecessor receipt, and its receipt never authorizes a later run. Informal
reuse of prior timings, RNG evidence, and resource calibration is expected;
rerun only when the changed code could invalidate the reused fact.

The active SCC diagnostic bundle is `KSS-STREAMLINE-SOURCE-BUNDLE-V1` and its
run kind is `KSS-STREAMLINE-1`. Do not add the historical K1 timing job or
MATLAB scale harness back to that ordinary diagnostic bundle.

## Development and completion

1. Read `PLAN.md` before substantive work.
2. Add a focused failing regression for a numerical or behavioral repair.
3. Keep dense/brute-force oracles independent of production Mata.
4. Run the smallest relevant test while iterating.
5. Before closing a change, run local Python/static tests, Stata quick/full
   tests when Stata is available, install checks for package changes, and the
   repository checks required by the root `AGENTS.md`.
6. Record limitations and any reused evidence. A new SCC run, performance
   threshold, GPT review, or human review is required only when a new claim
   actually depends on it.

Status language remains precise: executable tests and model review are not a
human independent check. This process milestone does not authorize a public
release, a license claim, or production qualification.

## Licensing

The owner selected GPL-3.0-only on 2026-08-18 for CMG and any distributed KSS
package containing it. Official CMG source may be ported into Mata only under
the provenance, notice, corresponding-source, and read-only-import rules in
`shared/cmg/AGENTS.md`, `CODE_LICENSE.md`, and the active plan. Do not copy the
headerless maintained KSS MATLAB wrapper. Public distribution still requires
the plan's final human license/provenance review.

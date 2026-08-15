# AGENTS.md

## Scope

This folder is the independent Stata/Mata implementation of Kline--Saggio--
Sølvsten leave-out bias-corrected point estimates for linear worker--firm
variance decompositions. It is developed beside `ppml_talo/`; package
unification is a later owner-authorized task.

Only files below `kss_bc/` may change during this milestone series. The root
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

## Milestone workflow

1. Read `PLAN.md` before substantive work.
2. Work on one `KB` milestone at a time and keep its evidence with the code.
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

- Production runtime is Stata/Mata 18 or 19 only.
- Python, MATLAB, and R are development oracles, never runtime dependencies.
- Encode identifiers densely, eliminate worker coordinates exactly, solve the
  full firm-mobility Laplacian on its zero-sum quotient, ground the displayed
  coordinate only after convergence, and stream probe batches.
- Keep large SCC attempts under `/projectnb/welfgr/kss-bc/runs/<run-id>`.
  Submit with `qsub -P welfgr`; never run sustained compute on login nodes.
- Accept SCC evidence only when `qacct`, application logs, and validated
  outputs all pass. Preserve failed attempts under distinct run IDs.
- No restricted Separations data is authorized for this milestone series.

## Licensing

The repository has no selected public software license. The maintained MATLAB
repository also lacks a resolved license. Implement from mathematical formulas
and observed behavior; do not copy or redistribute MATLAB source. Internal
install tests are allowed, but public release or publication is not.

# `kss_bc`: KSS point estimates in Stata/Mata

`kss_bc` is an internal-development Stata/Mata implementation of the
Kline--Saggio--Sølvsten leave-out bias correction for linear two-way
fixed-effect variance decompositions. It is developed beside the existing
`ppml_talo` command. Package unification is deferred.

The command targets worker variance, firm variance, worker--firm covariance,
and the variance of their sum. It supports observation or actual-match
deletion, exact and improved-JLA calculations, joint or fixed-offset controls,
positive integer frequency weights, and separate target weights. Match-mode
headlines use a mover-only fit and target. `stayers(both)` is explicitly
withheld; no observation-level fallback is presented as match robust.
Every randomized calculation with controls, including fixed offset, requires
a deterministic probe-independent full-fit and deletion-rank certificate;
ambiguous designs are withheld for exact verification instead of being passed
by solve error or a favorable leverage draw.
Explicit zero or collinear numeric controls are never treated as omitted
factor levels: they remain in the requested full design and trigger a typed
rank failure. The JLA FE solver works on the permutation-equivariant firm
quotient and applies the displayed last-firm normalization only after solving.
Matrix right-hand sides use independent scalar PCG recurrences in true
lockstep: one matrix Schur traversal serves all active columns, while stopping,
curvature, iteration, and complete residual gates remain per RHS.
An ID-free canonical control-span basis makes controlled exact and iterative
inverse actions independent of an invertible user control reparameterization
on accepted paths. Every controlled backend uses one outcome/target-based
semantic row order. The canonicalizer admits at most 32 controls and carries a
dimensioned numerical uncertainty envelope through its full-fit and deletion
conditioning gates; uncertified cases are withheld as
`AMBIGUOUS_CONTROL_BASIS`. Every JLA route has a typed `physical_limit()`
allocation boundary. Literal frequency totals above the exact binary64 integer
range are withheld before graph ranking. Both backends check the final
plug-in-minus-correction row separately before posting it.
Discrete outcomes may supply an explicit complete and unique `probeorder()`
physical-observation key to refine otherwise ambiguous outcome/target ties.
The command never infers such a key; existing calls retain the original
stream and fail closed when their semantic order remains ambiguous.

This package provides point estimates and numerical diagnostics only. It does
not post `e(V)` or provide econometric confidence intervals. It has no public
release license and must not be published or redistributed.

See [PLAN.md](PLAN.md), [the decision record](docs/DECISIONS.md), and
[the source ledger](docs/SOURCE_PROVENANCE.md). The mathematical and numerical
contracts are recorded in [ESTIMATOR_CONTRACT.md](docs/ESTIMATOR_CONTRACT.md),
[BLOCK_CONTROL_DERIVATION.md](docs/BLOCK_CONTROL_DERIVATION.md), and
[NUMERICAL_ARCHITECTURE.md](docs/NUMERICAL_ARCHITECTURE.md).

An internal install from a checkout is:

```stata
net install kss_bc, from("/absolute/path/to/ppml-variance/kss_bc") replace
```

A production-style call is:

```stata
kss_bc log_wage age2 age3 i.year [fw=freq],                 ///
    worker(person_id) firm(analysis_establishment_id)       ///
    deletion(match) deletionid(actual_match_id)             ///
    algorithm(jla) nuisance(joint) targetweight(target_mass) ///
    probes(200) batch(8) seed(8675309)
```

The package runs from Stata/Mata 18 or 19. Python and MATLAB are validation
oracles only and are not runtime dependencies.

The shared CMG core has a forced test-only KSS adapter. It is not installed or
selectable through the command. Source-bound Stata 19 moderate/weak synthetic
and bounded MATLAB-retained real-data gates pass, but easy CMG fails closed
and installed-route/no-regression promotion remains open. Diagonal lockstep
PCG remains the only public route.

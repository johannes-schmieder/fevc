# `ppmltalo`: PPML TALO point estimates in Stata/Mata

This folder develops a pure Stata/Mata implementation of targeted analytic
leave-out correction for quadratic fixed-effect functionals after PPML. The
primary use case is an AKM-style worker–firm variance decomposition with
millions of observations.

The package is under active development and is not yet a released estimator.
It fits PPML through `ppmlhdfe`, reconstructs an identified coefficient
quotient, and evaluates worker, firm, covariance, and total TALO point
estimates. `worker()` and `firm()` define both the target coefficients and
variance functionals. The deletion unit is chosen separately: one physical
observation, a supplied mutually exclusive cluster, or a worker–firm match.
An exact integer frequency weight represents duplicate physical observations;
one-copy observation deletion and all-copy cluster deletion retain distinct
semantics and agree with literal expansion in the test suite.

The large observation path uses a batched matrix-free two-FE solver plus an
exact low-dimensional Schur augmentation for jointly estimated controls and
nuisance fixed effects. The bounded dense path forms equilibrated full and
positive coefficient-space information matrices and applies exact local
Woodbury calculations to generic deletion blocks, including blocks containing
multiple worker or firm coordinates. `engine(auto)` routes between these
architectures without changing the estimator. Deterministic coefficient-basis
trace is used when the quotient is small enough; otherwise randomized trace
reports numerical MCSE only. Neither path forms an n-by-n matrix.

Every public matrix-free observation calculation passes a deterministic exact
or conservative leverage gate before randomized leverage is used in the
deleted-index approximation. The joint Schur preparation has independent
cross-form and reconstructed-residual checks plus nonnegative gate padding.
Dense information, local systems, and the nuisance Schur block return
equilibrated reciprocal-condition diagnostics. These remain operational
double-precision checks, not exact-real forward-error certificates.

For Separations, the advancing configuration uses 100 or 200 binned worker and
firm effects while deleting actual original-person by original-`estabid`
matches carried through duplicate collapse. This is different from the frozen
worker-bin by firm-bin comparison. The earlier 200-bin bin-cell pilot remains
withheld at maximum block eigenvalue `0.8354554568`, above the unchanged `.8`
safety gate. Both actual-match CZ20 pilots passed, but the subsequent 20-cell
all-CZ matrix did not: every 100-bin cell was available, while five 200-bin
cells were withheld under the unchanged score or positive-face gates. Those
failures and bounded frozen-input diagnostics are retained; no partial matrix
is promoted. All clustered results assume the declared deletion units are
independent. Actual-match deletion allows within-match dependence but does not
address dependence for a person across employers. Every result remains
descriptive with theory applicability unverified, and the command does not
post `e(V)`.

See [PLAN.md](PLAN.md) for the live milestone ledger and
[TESTING.md](TESTING.md) for suite, seed, tolerance, portability, and scale
qualification instructions. See
[theory/ppmltalo_numerics.tex](theory/ppmltalo_numerics.tex) or the compiled
[theory/ppmltalo_numerics.pdf](theory/ppmltalo_numerics.pdf) for the
mathematical/numerical companion.

## Development command

```stata
ppmltalo earnings_count, worker(worker_id) firm(establishment_id) ///
    leverageprobes(200) probes(200) batch(8) seed(8675309)

ppmltalo E_U age2_* age3_* [fw=N_freq],                           ///
    worker(worker_bin) firm(firm_bin) absorb(year) deletion(match) ///
    deletionid(actual_person_estab_match) nuisance(joint)           ///
    meanmax(1.01) block_limit(.8) trace(exact) batch(8)

ppmltalo transitions x1 [fw=duplicate_count],                      ///
    worker(worker_id) firm(firm_id) deletion(cluster)               ///
    deletionid(cluster_id) engine(auto) trace(randomized)
```

The command returns the plug-in, correction, and TALO point estimates for all
four accounting targets.
`probes()` controls numerical randomization, not an econometric resampling
procedure; its reported uncertainty is numerical Monte Carlo error only.
In bounded match mode, `traceexact` replaces that randomized contraction with
a batched coefficient-basis identity. It uses no random probes and reports
`e(trace_directions)=e(quotient_parameters)` and zero trace MCSE.
That zero excludes random trace-probe error only; floating-point error and
econometric uncertainty remain outside the reported MCSE.

Every development result is labeled
`TALO_THEORY_APPLICABILITY_UNVERIFIED`. This label remains until the formula,
software, statistical, scale, and applicable theory gates in `PLAN.md` close.
Typed match-engine withholding retains aggregate block-stage and local-metric
diagnostics but never exports the worker or firm identifiers of the block.

## Runtime requirements

- Stata/MP 18 or 19;
- `ppmlhdfe`, `reghdfe`, and `ftools`;
- no Python, R, Julia, or compiled extension at runtime.

Numeric controls and a bounded number of nuisance fixed effects through
`absorb()` are supported under each deletion mode. The default
`nuisance(joint)` includes them in full and positive information and lets them
move under deletion. `nuisance(fixedoffset)` is an explicitly labeled
conditional approximation. Frequency weights must be positive integers
counting exact duplicate physical rows; other likelihood-weight semantics are
rejected.

Before evaluating the correction, the command applies a conservative
row-deletion certificate for the pure two-way model. The full worker--firm
multigraph and the positive-outcome multigraph must remain connected after any
single row is removed, and positive support must span every retained level.
The condition guarantees rank and a finite interior deleted fit, but it can
withhold samples that an exact component/inequality analysis would accept.
One-level firm samples are explicitly unsupported in the development version.

## Development

Run the local quick suite from the repository root:

```bash
/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp -b do ppml_talo/tests/run_all.do quick
```

The predecessor full statistical suite passes locally and on SCC Stata/MP 19. The
representative `mobile` profile passes through ten million rows with 100+100
probes and measured RSS on both tested platforms. The 10m local Stata 18 run
completed in 631 seconds and peaked at 19.66 decimal GB; the SCC Stata 19 run
completed in 4,114 Stata seconds and peaked at 14.11 decimal GB under the
site's four-core license. The deliberately weak-ring profile is safely
withheld at its solver gate. Representative-data conditioning, production
integration, and release qualification remain open. ST11 has source-bound
Stata 19/Linux portability records, CZ20 actual-match pilots, and a completed
but failed all-CZ qualification matrix. See the
[local qualification report](benchmarks/reports/LOCAL_STATA18_2026-08-12.md)
and [SCC qualification report](benchmarks/reports/SCC_STATA19_2026-08-12.md).

A local AKM-shaped benchmark can be run with:

```bash
/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp \
  -b do ppml_talo/benchmarks/benchmark_akm.do 100000 100 100 8 \
  /private/tmp/ppmltalo_benchmark_100k.csv stable mobile
```

Use `benchmarks/local_qualify.sh` rather than the direct form for registered
local evidence; it binds the result to a clean source commit and records peak
RSS.

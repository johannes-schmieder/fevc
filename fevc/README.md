# Using fevc

`fevc` estimates leave-out bias-corrected variance components in linear
worker–firm fixed-effects models. This guide describes version `0.6.1`.
Start with [installation](../INSTALLATION.md) and `help fevc`.

Requires Stata 15 or newer (through Stata 19).

## Basic use

```stata
fevc log_wage, worker(worker_id) firm(firm_id)
fevc log_wage experience i.year, worker(worker_id) firm(firm_id)
```

The four corrected targets are worker-effect variance, firm-effect variance,
worker–firm covariance, and the variance of the sum of the two effects.
The main display reports twice the covariance as the sorting contribution.
Controls enter the regression; the reported targets concern the worker and
firm effects.

## Deletion and population

Match deletion is the default. It leaves out a whole worker–firm match when
correcting mover contributions, which allows errors to be correlated within
a match. By default, eligible stayers (workers with a single match: one
`deletionid()` value, or one firm by default) are also included. Because a
stayer has only one match, stayer contributions are corrected one
observation at a time, so they are not robust to correlated errors within
the stayer's match.

Use `deletion(observation)` to leave out one observation at a time, or
`stayers(movers)` for a mover-only population in either deletion mode.
`stayers(both)` is the default in both modes. Use `deletionid(match_id)` when
the blocks of observations whose errors may be correlated differ from the
worker–firm pairs, for example separate employment spells at one firm.

```stata
fevc log_wage i.year, worker(worker_id) firm(firm_id) ///
    deletion(match) stayers(movers)
```

With `deletion(match)`, a mover is a worker with more than one deletion
unit: more than one `deletionid()` value, or more than one firm by default.
Employers pooled into one firm group in the model can therefore remain
separate deletion units when `deletionid()` distinguishes them. Errors must be independent across those units, and
every retained deletion must leave the model identified. Workers with a
single unit are stayers and keep the one-observation-at-a-time correction
under `stayers(both)`.

The retained sample must satisfy the command's identification conditions,
so it can be smaller than your data. Inspect it with `estat sample`; see the
[estimator guide](docs/ESTIMATOR_CONTRACT.md) for deletion, sample selection,
and weighting details.

## Calculation and memory

By default, `fevc` uses the randomized Johnson–Lindenstrauss approximation
(JLA) with 200 probes. Set `seed()` for reproducible results with the same
backend. Exact calculation is available for small problems:

```stata
fevc log_wage, worker(worker_id) firm(firm_id) algorithm(exact)
fevc log_wage, worker(worker_id) firm(firm_id) probes(500) seed(12345)
```

The command uses the native Rust plugin when it supports the request and
otherwise the portable Mata implementation; `backend(mata)` selects Mata
explicitly.

Because JLA depends on random draws, the default `mcse(all)` reports a
numerical Monte Carlo standard error (MCSE) for each of the four estimates.
It approximates how much the estimates would change if the random draws
were repeated on the same data. It is not a sampling standard error: it
excludes the bias that remains with a finite number of probes and all
sampling uncertainty, and it does not cover projections or confidence
intervals. Increase `probes()` if the MCSE is large relative to the
differences you want to interpret. Exact calculation reports an MCSE of zero.
Use `mcse(off)` to skip the calculation; the MCSE results are then missing.

```stata
fevc log_wage, worker(worker_id) firm(firm_id)
matrix list e(mcse)
estat diagnostics

fevc log_wage, worker(worker_id) firm(firm_id) mcse(off)
```

The MCSEs are displayed in one line below the decomposition, with the
largest of them as a share of the components' standard errors when these
are reported (above 10% the display warns and suggests more probes, for
example `probes(2000)`), and stored in `e(mcse)` and in the `mcse` row of
`e(results)`. `estat diagnostics` reports their method, status and
availability. `e(mcse_cov_raw)` and `e(mcse_cov)` contain the raw and usable
4 by 4 numerical covariances in the same target order. When exporting MCSEs,
also save `e(mcse_mode)`, `e(mcse_method)`, `e(mcse_status)`,
`e(mcse_available)`, `e(mcse_cov_raw)`, `e(centering)` and
`e(mcse_centering)`, so that a withheld MCSE is not mistaken for zero. A
withheld MCSE leaves the point estimates valid. See the
[returned-results reference](docs/FAILURES_AND_RETURNS.md).

An optional `memory_gib()` budget guides automatic planning and warns by
default if the forecast exceeds it; add `memorycheck(error)` to stop
instead. The budget is a forecast of the command's own allocations, not an
operating-system memory cap. See the [memory guide](docs/MEMORY.md).

By default the command prints its results only. `verbose` adds live
progress while it runs (with the Rust backend: sample preparation, the
selected calculation, memory forecasts and one line per phase) and details
on the calculation and memory planning; `nolog` keeps only the final
results, and `nodisplay` suppresses successful output; `quietly` also
suppresses progress. None of these options changes the estimates.

## Outcome centering

The default is `centering(mean)` for both exact and JLA calculation.

| Option | Point estimate | Numerical MCSE |
| --- | --- | --- |
| `centering(none)` | Uncentered leave-out correction of the Kline, Saggio, and Sølvsten article. | Ordinary calculation. |
| `centering(mean)` (default) | Subtract the retained-sample outcome mean in the bias correction only. | Treats the observed mean as fixed. |
| `centering(corrected)` | Mean centering plus an adjustment for estimating the mean. | Treats the mean and the adjustment as fixed. |

Centering can make the correction considerably more precise, particularly
when the outcome mean is far from zero, although it is not better in every
case. Because the sample mean depends on the same errors, it can introduce
a bias when error variances differ across observations;
`centering(corrected)` adds an adjustment for that bias in point estimates,
which is exactly unbiased with exact calculation and jointly estimated
controls. The fit, plug-in components and retained sample are the same
under all three settings. The KSS Matlab package centers the outcome, so
`centering(none)` does not reproduce that package.

The mean is taken over the retained observations, counting frequency
weights, as in Stata's `summarize u if e(sample) [fw=f]`; target and
projection weights do not change it. With `nuisance(fixedoffset)`, the
fitted controls are subtracted first. Mean centering adds no extra solves
or probes. With exact calculation, `centering(corrected)` solves one extra
linear system; with JLA, it splits the existing probes into two halves, so
`probes()` must be even and at least 4.

```stata
* Small exact problem with the adjustment for estimating the mean
fevc log_wage, worker(worker_id) firm(firm_id) ///
    algorithm(exact) centering(corrected) mcse(off) backend(mata)

* JLA with the default mean centering
fevc log_wage, worker(worker_id) firm(firm_id) ///
    algorithm(jla) centering(mean) seed(12345) backend(mata)
```

With JLA, `centering(corrected)` reports the same MCSE as `centering(mean)`
for the same call, so it does not include the numerical uncertainty in the
adjustment. Projections accept `mean` (the default) and `none`, but not
`corrected`; for projections, centering changes the standard errors but not
the coefficients. The standard errors of the components accept all three
settings: their simulated noise is centred as the outcome is, and under
`corrected` they are those of the mean-centred estimate with the
adjustment treated as fixed. `e(inference_centering)` and
`e(inference_mean_omitted)` record this.
See [the centering guide](docs/CENTERING.md) for formulas, restrictions and
timing evidence.

## Results and inference

```stata
estat decomposition, full
estat sample
estat computation
estat diagnostics
estat inference
```

These views show the decomposition, retained population, computation choices,
diagnostics and the standard errors of the components in detail. Estimates
and supporting results are also stored in `e()`; see `help fevc` for their
names.

By default the command reports repeat-measurement standard errors for the
four components: a confidence interval for each, normal where the estimate
is close to normal and otherwise the `q = 1` interval of Kline, Saggio and
Sølvsten (2020, Section 7.1), chosen component by component from the share
of the estimated sampling variance carried by the leading direction
(threshold ten per cent), and their covariance as `e(V)` when every
component is normal. Deletion units that repeat a worker–firm cell receive
their own sample variance and the others a fitted model, so the intervals
are as good as that model for those units and not robust to arbitrary
heteroskedasticity; with controls, their estimation enters to first order.
`inference(highrank)` gives normal intervals only, `inference(q1)` the
`q = 1` interval wherever it exists, and `inference(none)` point estimates
only. The display shows one table with the same columns in every mode:
estimate, standard error, interval, which interval it is, and the leading
direction's share of the estimated sampling variance (also under
`inference(highrank)`, which finds the directions for that purpose); it
shows no p-values, which `test` and `lincom` give from `e(V)`.
`estat inference` shows both intervals of every component, why it received
the one reported, the leading direction and the method. See
`help fevc##se` and the [inference guide](docs/INFERENCE.md),
which gives the simulations behind the rule and their limits; they are not
a coverage guarantee. The note
[Feasible Standard Errors for Leave-Out Variance Components](../docs/feasible-standard-errors.pdf)
derives the method, reports the coverage of the reported intervals in
simulations of the command (96% on average, 92.3% to 99.6% over 320 cases)
and compares the standard errors with those of KSS: within about 7% of the
KSS Matlab package on five of six samples, at a fraction of the computing
time. Its last section states what
is proved, what rests on simulation and where the standard errors were
measurably off. The proofs, supporting material and earlier editions are in
[docs/repeat_measurement/](docs/repeat_measurement/README.md).

```stata
* Normal intervals for every component and e(V), for test and lincom
fevc log_wage i.year, worker(worker_id) firm(firm_id) inference(highrank)
matrix list e(component_inference)

* The q = 1 interval for every component that has one; no e(V)
fevc log_wage i.year, worker(worker_id) firm(firm_id) inference(q1)
matrix list e(inference_intervals)
```

Projection standard errors (`project()`) are requested separately, have
their own assumptions and accept mean (the default) and no centering; with
`project()` the default reports point estimates for the components. Version
0.6.0 removed the experimental variance models of 0.5.2
(`inferencemodel(target_lowess|structured_common|structured_leverage)`) and
the options that tuned them; a request for one stops with a message.

## Runnable examples

The installed help starts with a clickable example and contains seven more
short, runnable examples; Examples 6 and 7 show `inference(highrank)` and
`inference(q1)` and how to read their results, and Example 8 the detailed
`estat` displays. Clicked examples restore your data. To generate example data and explore them yourself:

```stata
fevc, simulate_data(ex1) clear
fevc log_wage productivity i.period, worker(worker_id) firm(firm_id)
estat decomposition, full
```

`ex1` is a larger positive-sorting panel; `ex2` is its small exact-calculation
counterpart. `ex3` illustrates frequency and target weights, `ex4` a projection
on firm size (its small firms are linked by few movers, a bottleneck that
calls for the `q = 1` interval), and `ex5` the standard-error options on a
fixed small dataset.
The helper prints observations, workers, firms, and the true variance components
for the realized data and example weighting. It retains `worker_fe`, `firm_fe`,
and `error`; inspect the construction with `viewsource fevc__simulate_data.ado`.
`clear` is required to replace existing data. Random examples accept `seed()`;
all generation calls preserve the caller's random-number state and restore
the old data on failure.

## Running on a cluster

`nativethreads(#)` sets how many threads the Rust backend uses, independently
of your Stata license. It defaults to `c(processors)` and must fit within the
machine and, on clusters using the SGE scheduler, within the job's `NSLOTS`.
For example, in a 28-slot job with a four-core Stata/MP license, use
`set processors 4` and add `nativethreads(28)` to `fevc`. This is a cap; not
every step uses all threads. `e(native_threads_*)` records the request, the
cap actually applied, the available capacity and the measured concurrency.

## Further reading

The companion paper, *fevc: Leave-out bias-corrected variance decompositions
in Stata* (Schmieder, 2026), and its technical supplement explain the method
and sample construction. The standard errors of the components are derived
in [Feasible Standard Errors for Leave-Out Variance Components](../docs/feasible-standard-errors.pdf),
which also compares them with those of KSS.

The package code is MIT; see [licensing](../CODE_LICENSE.md).

# Using fevc

`fevc` estimates leave-out bias-corrected variance components in linear
worker–firm fixed-effects models. This guide describes version `0.5.0`.
Start with [installation](../INSTALLATION.md) and `help fevc`.

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

The MCSEs are displayed below the decomposition and stored in `e(mcse)` and
in the `mcse` row of `e(results)`. `estat diagnostics` reports whether they
are available. `e(mcse_cov_raw)` and `e(mcse_cov)` contain the raw and usable
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

With the Rust backend, the command reports sample preparation, the
selected calculation, memory forecasts and live progress while it runs.
`nolog` keeps only the final results, `verbose` adds details on the
calculation and memory planning, and `nodisplay` suppresses successful
output; `quietly` also suppresses progress. None of these options changes
the estimates.

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
adjustment. Projections and component `inference(highrank|q1)` accept
`mean` (the default) and `none`, but not `corrected`. For projections,
centering changes the standard errors but not the coefficients. For
component inference, mean centering treats the observed mean as a known
constant and ignores the uncertainty from estimating it; this is an
approximation, not inference conditional on the observed mean.
`e(inference_centering)` and `e(inference_mean_omitted)` record this choice.
See [the centering guide](docs/CENTERING.md) for formulas, restrictions and
timing evidence.

## Results and inference

```stata
estat decomposition, full
estat sample
estat computation
estat diagnostics
```

These views show the decomposition, retained population, computation choices,
and diagnostics. Estimates and supporting results are also stored in `e()`;
see `help fevc` for their names.

Projection standard errors and component confidence intervals require
explicit options and have different assumptions. Both accept mean (the
default) and no centering. Component inference relies on a model for the
error variances, structured models impose further
assumptions, fixed-offset match inference omits the uncertainty from
estimating the controls, and observation-q1 calibration retains a
documented limitation. Confidence intervals that pool movers and stayers
require explicit structured models and assume independent stayer
observations; see the
[pooled inference guide](docs/POOLED_COMPONENT_INFERENCE.md). Read the
[inference guide](docs/INFERENCE.md) before requesting intervals or standard
errors.

In the package's simulation assessment, the exact Mata method produced
intervals too rarely in three of its four primary designs (22 failed
availability checks), and the native calculation, which produced intervals
in all eight primary designs, was materially sensitive to numerical
settings. These results do not establish
general coverage; see the
[assessment](docs/MEAN_COMPONENT_INFERENCE_ASSESSMENT_20261008.md).

## Runnable examples

The installed help starts with a clickable example and contains five short,
runnable examples. Clicked examples restore your data. To generate example
data and explore them yourself:

```stata
fevc, simulate_data(ex1) clear
fevc log_wage productivity i.period, worker(worker_id) firm(firm_id)
estat decomposition, full
```

`ex1` is a larger positive-sorting panel; `ex2` is its small exact-calculation
counterpart. `ex3` illustrates frequency and target weights, `ex4` a projection
on firm size, and `ex5` component-inference syntax on a fixed small dataset.
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
and sample construction.

The package code is GPL-3.0-only; see [licensing](../CODE_LICENSE.md).

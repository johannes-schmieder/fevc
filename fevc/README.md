# Using fevc

`fevc` estimates leave-out bias-corrected variance components in linear
worker–firm fixed-effects models. This source is version `0.5.0-rc.1`.
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

Match deletion is the default. It leaves out a worker–firm match when
correcting mover contributions. By default, eligible stayers are included
with their separate observation-deletion correction. Those stayer
contributions are not robust to arbitrary within-match dependence.

Use `deletion(observation)` for physical-observation deletion, or
`stayers(movers)` for a mover-only population in either deletion mode.
`stayers(both)` is the default in both modes. Use `deletionid(match_id)` when
the desired match units differ from the implicit worker–firm pairs.

```stata
fevc log_wage i.year, worker(worker_id) firm(firm_id) ///
    deletion(match) stayers(movers)
```

The retained sample must satisfy the command's identification conditions.
Inspect it with `estat sample`; see the [estimator guide](docs/ESTIMATOR_CONTRACT.md)
for deletion, sample selection, and weighting details.

## Calculation and memory

The default is JLA approximation with 200 probes. Set `seed()` for a
reproducible call within the selected runtime. Exact calculation is available
for small problems:

```stata
fevc log_wage, worker(worker_id) firm(firm_id) algorithm(exact)
fevc log_wage, worker(worker_id) firm(firm_id) probes(500) seed(12345)
```

Automatic backend selection prefers an available, compatible native backend.
Use `backend(mata)` to select the portable implementation explicitly.
The default `mcse(all)` reports approximate numerical MCSE for the four main
point estimates, including local leverage- and target-probe uncertainty under
the chosen [centering convention](docs/CENTERING.md). Mean holds the observed
mean fixed; Corrected also holds its added increment fixed. Use `mcse(off)`
to skip the diagnostic's extra work and display. This diagnostic excludes
finite-probe bias and sampling uncertainty; it is not an econometric standard
error. It can accompany supported projection or sampling-inference requests,
but those additional results receive no MCSE.
Exact MCSE is zero when enabled; `mcse(off)` returns missing diagnostics,
including in exact mode.

```stata
fevc log_wage, worker(worker_id) firm(firm_id)
matrix list e(mcse)
estat diagnostics

fevc log_wage, worker(worker_id) firm(firm_id) mcse(off)
```

The results are displayed below the decomposition and stored in
`e(mcse)`; `estat diagnostics` reports method, status and availability.
`e(mcse_cov_raw)` and `e(mcse_cov)` contain the raw and usable 4 by 4
numerical covariances in the same target order. Save `e(mcse_mode)`,
`e(mcse_method)`, `e(mcse_status)` and `e(mcse_available)` with exported MCSEs
and raw covariance, together with `e(centering)` and `e(mcse_centering)`.
The `mcse` row of `e(results)` matches `e(mcse)`. A withheld
numerical diagnostic retains valid point estimates. See the
[returned-results contract](docs/FAILURES_AND_RETURNS.md).

An optional `memory_gib()` budget guides automatic planning and warns by
default. Add `memorycheck(error)` to enforce forecast admission. The budget
is a forecast of command allocations, not an operating-system memory cap.
See the [memory guide](docs/MEMORY.md).

Rust runs display sample preparation, selected algorithms and batches, memory
forecasts, and coarse progress. `Total elapsed` spans the entire command;
leverage and target probe counts appear together while point estimation runs.
Add `nolog` to retain only final results, or
`verbose` for routing and memory-policy details. `nodisplay` suppresses successful
runtime and final output; `quietly` also suppresses progress. These options do
not change estimation. Live progress requires a reporting-capable native build;
Mata retains its current output.

## Outcome centering

The default is `centering(mean)` for both exact and JLA. Use
`centering(none)` to reproduce the former default, or choose
`centering(corrected)` for the estimated-mean adjustment.

| Option | Point estimate | MCSE convention |
| --- | --- | --- |
| `centering(none)` | Ordinary leave-out correction. | Ordinary numerical calculation. |
| `centering(mean)` (default) | Subtract the retained working-outcome mean only in the bias correction. | Treat that observed mean as fixed. |
| `centering(corrected)` | Mean plus an adjustment for estimating the mean. | Treat both mean and additional increment as fixed. |

The mean is `sum(f_i*u_i)/sum(f_i)` over the retained working outcome, as in
Stata's `summarize u if e(sample) [fw=f]`. Neither `targetweight()` nor
`projectweight()` redefines it. With
`nuisance(fixedoffset)`, subtract the fitted controls before taking the mean.
The fit, plug-in components and retained sample are unchanged. Mean adds no
solves or probes. Corrected exact adds one shared correction system;
Corrected JLA uses the existing full leverage pool and its two halves and
requires even `probes()` of at least four.

```stata
* Small exact problem
fevc log_wage, worker(worker_id) firm(firm_id) ///
    algorithm(exact) centering(corrected) mcse(off) backend(mata)

* JLA with inexpensive Mean centering and fixed-mean numerical MCSE
fevc log_wage, worker(worker_id) firm(firm_id) ///
    algorithm(jla) centering(mean) seed(12345) backend(mata)
```

Corrected JLA returns exactly Mean's MCSE/covariance for the same call,
excluding numerical uncertainty in the extra increment. Projection supports
Mean (the default) and None, using the same retained frequency-weighted
working-outcome mean. Neither target nor projection weights redefine it.
Component `inference(highrank|q1)` also accepts Mean or None on its existing
supported tuples. Mean inference treats that observed mean as a fixed constant
and omits its estimation uncertainty; this is not conditional inference given
the observed mean. Corrected remains unsupported with either inference or
projection. `e(inference_centering)` and `e(inference_mean_omitted)` record the
component convention separately from MCSE.

Point centering works in Mata and a matching native build. Native point
centering requires centering API 1; Mean projection additionally requires
projection-centering API 1 and Mean component inference requires
component-centering API 1. Existing combined exact-Mata requests support Mean;
native combined component/projection requests remain unsupported. New Mac
candidates at `b9f80ce9` and Linux at `63757839` pass full qualification and
installed Mean-component checks. They remain unadopted while Windows blocks
the complete update. The retained payloads lack component-centering API 1;
the Windows manual-test payload also lacks Mean-projection support. See the
[current candidate record](../native/mean-component-20261008/checkpoint.json).
Use current Mata source or a matching qualified native build.

The [completed assessment](docs/MEAN_COMPONENT_INFERENCE_ASSESSMENT_20261008.md)
retains 22 exact-Mata availability-screen failures across three cells. Eight
native primary cells have 100% interval availability and pass broad descriptive
screens, but fixed-outcome tests show material numerical sensitivity. These
results do not establish general coverage.
Automatic routing may use Mata when the capability is absent, while strict
Rust/Counter-V1 requires the matching plugin. See [the centering guide](docs/CENTERING.md)
for restrictions, stored assumptions, formulas and measured local costs.

## Results and inference

```stata
estat decomposition, full
estat sample
estat computation
estat diagnostics
```

These views show the decomposition, retained population, computation choices,
and applicable diagnostics. Estimates and supporting results are also stored
in `e()`; see `help fevc` for their names.

Component inference and fixed-effect projection inference require explicit
options and have different assumptions and supported combinations. Component
inference and projection accept Mean (the default) and None on their existing
supported tuples. Mean component inference uses a fixed-observed-mean
approximation; structured component inference also imposes variance-model
assumptions. Fixed-offset match
inference omits nuisance-control estimation uncertainty, and observation-q1
calibration retains a documented limitation. Read the
[inference guide](docs/INFERENCE.md) before requesting intervals or standard errors.

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
all generation calls preserve the caller's RNG state and restore old data on failure.

Use `estat sample` after estimation: leave-out identification can require
excluding observations, and the decomposition describes the retained sample.
The companion paper, *fevc: Leave-out bias-corrected variance decompositions in
Stata* (Schmieder, September 2026), explains the method and sample construction.

The package code is GPL-3.0-only; see [licensing](../CODE_LICENSE.md).

For `deletion(match)`, movers have more than one distinct original deletion
unit: supplied `deletionid()` values, or worker--firm pairs by default. Actual
employers pooled into one model firm can therefore remain separate deletion
blocks. Errors must be independent across those blocks, and every retained
deletion must preserve identification. True one-block stayers retain the
separate observation correction under `stayers(both)`. Observation-mode
populations are unchanged. Mata and current Rust plugins support this partition,
including the existing projection and fixed-offset match-inference routes.
Older plugins require an update for strict native requests; automatic requests
can fall back to Mata before preparation.

Native concurrency can be set independently of the Stata license with
`nativethreads(#)`. Its default remains `c(processors)`; explicit values must
fit within the machine and, on SGE, `NSLOTS`. For a 28-slot SCC job with a
four-core Stata license, use `set processors 4` and add `nativethreads(28)` to
`fevc`. This is a native concurrency cap, not a promise that each phase uses
all workers. The requested cap, selected cap, runtime capacity and available
concurrency measurements are returned separately in `e(native_threads_*)`.

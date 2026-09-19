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
Numerical Monte Carlo error describes approximation precision; it is not an
econometric standard error.

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
options and have different assumptions and supported combinations. Structured
component inference imposes variance-model assumptions. Fixed-offset match
inference omits nuisance-control estimation uncertainty, and observation-q1
calibration retains a documented limitation. Read the
[inference guide](docs/INFERENCE.md) before requesting intervals or standard errors.

## Runnable examples

The installed help contains examples that generate their own synthetic data:

```stata
fevc_run exact_controls using fevc.sthlp
fevc_run jla_controls using fevc.sthlp
fevc_run weights_targets using fevc.sthlp
```

Each restores the caller's data. The first two illustrate positive sorting
and compare estimated components with the known simulated effects.

The package code is GPL-3.0-only; see [licensing](../CODE_LICENSE.md).

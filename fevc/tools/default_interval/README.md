# Monte Carlo harness for the default interval

This harness measures the interval that the plain `fevc` command reports for each variance component
(the normal interval, or the q = 1 interval when the leading direction's share of the variance exceeds the
rule threshold). It passes the Stata command, with a given package copy and its Rust plugin, through
synthetic two-block panels whose true components are known, and records for every replication the
estimates, both intervals, the pieces of the rule and the interval reported. It produced the evidence in
`fevc/docs/default_interval/`.

Licensed Stata runs locally only. Nothing here is meant for a public CI runner.

## Files

- `designs.py`: the panel generator (NumPy only). Two blocks of firms, workers who move within their
  block, `n_bridge` workers who move once across (the bottleneck; `null` for one block), six unbalanced
  years, pruned to the leave-one-match-out connected set; effects and person-year error variances of the
  kinds `flat`, `profile`, `movers` and `bridge`; optional control (`"control": true`, or an object of options, see "Spec and manifest"). It reproduces the
  campaign designs bit for bit (the sha256 recorded in the manifests).
- `pkgmc.py`: `make`, `run`, `check`.
- `mc_task.do`: one Stata task (one design, one deletion mode, a block of replications).
- `aggregate.py`: coverage, lengths and candidate rules over a complete run.
- `confirm.py`: the verdict of a confirmation run against the tolerances in its manifest.

Tests: `fevc/tests/python/test_default_interval_mc.py` (no Stata needed).

## Commands

```bash
py=./.venv/bin/python
H=fevc/tools/default_interval
$py $H/pkgmc.py make  <run dir> <spec.json>          # designs + manifest; refuses an existing run dir
$py $H/pkgmc.py run   <run dir> <parallel> [max] --stata /path/to/stata-mp   # or set FEVC_STATA
$py $H/pkgmc.py check <run dir>                      # inventory; exit 1 when it finds problems
$py $H/aggregate.py   <run dir> [--allow-incomplete] # summary.json, cells.csv, rules.csv
$py $H/confirm.py     <run dir>                      # confirmation.json; exit 0 PASS, 1 FAIL
```

`make` needs no Stata. `run` takes the Stata executable from `--stata`, else from `FEVC_STATA`, and stops
with an error if neither is given. It runs only tasks without output, so an interrupted run is resumed by
running it again.

## Spec and manifest

The spec is JSON with `run`, `run_seed` (1 to 20000), `pkgroot` (the package directory with the ado files
and `fevc_rust_macos_arm64.plugin`), `source_commit`, `cells`, `deletions` (`match`, `observation`),
`reps_per_task`, `tasks_per_cell`, `threads` (native threads; 0 lets the options name the route),
`options` (`"none"` for the plain default command) and `expect_model` (the standard-error model every fit
must report). Optional: `purpose`, `expect_threshold` (the rule threshold every fit must report) and, for
a confirmation, `tolerances` with exactly `min_success_rate`, `min_coverage`, `min_mean_coverage`,
`max_length_ratio_none` and `min_posted_none` (see `confirm.py`). A cell has `name`, `group` (`none`
marks designs without a bottleneck), `nJ`, `nW`, `n_bridge`, `design_seed`, `gap`, `variance`, `errors`
(`normal` or `skew`) and optionally `control` and `realised`. `control` is `true` (a control `x` loading 0.4 on
the worker effect and 0.8 on the firm effect, plus noise of standard deviation 0.25, with coefficient 0.3) or
an object with any of the optional numeric keys `alpha`, `psi`, `noise`, `coef` (those four quantities) and
`hetero`, and the boolean `second`; other keys or types are rejected. `second` adds a control `x2` (a year
trend plus noise, coefficient 0.2) that enters the model beside `x`. A nonzero `hetero` multiplies the
person-year error standard deviation by `exp(hetero*e - hetero^2)`, with `e` the standard normal draw behind
the noise of `x`, so the error variance moves with the control and its mean factor is one. The column `xb` is
the contribution of all controls to the outcome. The object `{}` writes the same design as `true`. For an
object, the manifest design record also carries `control_realised`: the correlation of `x` with the sum of
the effects, the share of the variance of `x` within worker-firm matches, the standard deviations of `xb` and
of the effects, the number of controls and, with `hetero`, the 10th and 90th percentiles of the variance
factor. The regime a design realises (its true
curvature and share) is not computed here; a spec cell may carry it as `realised`, which `make` copies
into the manifest unchanged.

`make` writes `manifest.json` (schema `PKGMC-MANIFEST-V1`: the spec, the sha256 of the plugin, the four
ado/Mata files that carry the method, the task do-file, the driver, aggregator, verdict script and design
code, and for every design its sha256, person-years, workers, firms, bridge workers, collapsed and mover
rows) and `manifest.sha256`. `run` refuses to start when any hashed source or design differs from the
manifest.

A task is (cell, deletion mode, task index) and writes `out/<cell>__<deletion>__<task>.csv`, one row per
replication: keys (cell, deletion, task, rep, seed, stream), return code, status, model, the reason when
the default returned point estimates only, retained sample, seconds, the rule's summary (q = 1
components, steps, unresolved directions, whether e(V) was posted, threshold) and, for each component
1-4, the truth, estimate, standard error, normal and q = 1 bounds, reported bounds and regime, and the
pieces of the rule. Stata's log for each task is under `logs/`.

## Rules the harness enforces

- Random numbers: Stata's stream generator (`set rng mt64s`). Replication r of a cell draws from stream
  r + 1 under the cell seed `run_seed * 10^5 + cell index`, the same under both deletion modes, so the
  draws depend neither on the split into tasks nor on the order in which tasks run.
- Inventory (`check`): every expected task output is present and readable, no unexpected or partial
  (`.tmp`) file, replications exactly `first .. first + reps - 1`, keys and seeds agree with the manifest,
  truths present, every fit used the expected model and threshold and the whole design, no duplicate
  replication, manifest unchanged. `aggregate.py` refuses a run with inventory problems unless told
  otherwise and then marks it incomplete; `confirm.py` fails it.
- Every attempted replication is in every denominator. A failed fit, a fit that returned point estimates
  only, or a fit without an interval for a component counts as not covered.
- Fallbacks: a fit with status `KSS_POINT_ESTIMATES_ONLY`, no model name and no interval is the default's
  documented fallback when its standard-error pass stops. It is an attempted replication without an
  interval, listed with its reason, and counts against the success rate. Any other model, or a row that
  claims to be a fallback but carries an interval, is an inventory problem.
- The package's rule is recomputed from the recorded pieces with the threshold each fit reports and must
  agree with the recorded choice in every replication.
- The verdict evaluates every criterion on every case and lists every failing case, not the first one. It
  is written once: `confirm.py` refuses to overwrite `confirmation.json`. Task outputs and the manifest are
  likewise never overwritten (`make` refuses an existing run directory, the task refuses an existing
  output and writes under a temporary name before renaming).

## Stata details

- Stata's batch mode splits a quoted do-file argument at blanks, so the fevc options travel as one word
  with `+` for blanks; the task file refuses more arguments than it expects. On macOS Stata names the
  batch log after the last argument; `run` reports the newest log of a failed task.
- `matrix T = e(nothing)` does not fail in Stata, so the task file tests for `e(inference_intervals)` and
  `e(inference_leading)` with `confirm matrix`.
- Each task sets `set processors 1`: many tasks run side by side, and idle Stata/MP threads would compete
  with them. Native threads are set by `threads` in the spec.
- The environment variable `FEVC_DEFAULT_INFERENCE` is removed for every task, so that `options: "none"`
  runs the command's own default.
- The plugin name `fevc_rust_macos_arm64.plugin` is the one the runs used; another platform needs that name
  changed in `pkgmc.py`.

# Memory forecasts and optional budgets

`fevc` forecasts memory for every supported calculation. It assumes no budget
when `memory_gib()` is omitted: there is no implicit 4-GiB limit, no automatic
RAM detection, and no substitute large limit. Without a budget, memory
forecasts do not change the batch width, concurrency or solver route.
Normal probe-count, processor, structural and implementation limits still
apply, including the usual performance choices under `batch(auto)`.

## Choose a policy

| Options | Behavior |
| --- | --- |
| No `memory_gib()` | Forecast and continue without memory-based planning. |
| `memory_gib(2)` | Use a 2-GiB budget for automatic batch planning; warn and continue if the admission forecast exceeds it. |
| `memory_gib(2) memorycheck(error)` | Use that budget for planning and reject an over-budget admission forecast. |
| `memory_gib(2) memorycheck(off)` | Use that budget for automatic planning; suppress memory warnings and forecast rejection. |
| `memorycheck(warn)`, `memorycheck(error)` or `memorycheck(off)` alone | No budget is created. Forecast and continue. |

One GiB is 1,073,741,824 bytes. An explicit budget must be finite and positive;
its conversion, rounded down to whole bytes, must be between 1 and 2^53 bytes.
The default check policy is `warn`. To disable memory-based planning as well
as rejection, omit `memory_gib()`; adding `memorycheck(off)` to an explicit
budget still permits that budget to guide automatic batches.

An explicit positive integer `batch()` is preserved. With a supplied budget,
automatic planning may choose smaller batches. If an advisory budget cannot
fit even width one, the planner uses its minimum supported width and proceeds;
no fit is claimed. A strict budget can reject after this planning step.
Memory policy never changes the estimator, retained sample, target population,
random-number contract or numerical tolerances. Invalid requests, overflow,
actual allocation failures, numerical failures and external operating-system
or scheduler limits remain errors under every policy.

## What is forecast, and when

The specialized `CMG_FULL_V2` route uses thread-aware automatic probe widths:
`min(P, max(32, 8*T))` for leverage and `min(P, max(32, 4*T))` for targets,
where `P` is the probe count and `T` the permitted native thread count. Each
target probe creates two RHSs. This fixed policy has no tuning option;
other routes retain their existing width rules. Explicit batches still do
not select full CMG. With an explicit budget, the planner may reduce the
automatic widths. Without a budget, it uses the desired widths without a
memory-based reduction. The selected scalar workspace capacity, queue result
slots and overlapping batch/refinement buffers are admitted before probe RNG.
Wider batches increase real memory use; forecasts are not physical RSS.

The native forecast describes direct command allocation payload, including
Rust working storage and accounted overlapping C input/export buffers. It
excludes the original Stata dataset, allocator-retained pages, libraries and
thread stacks. It is not total process resident set size (RSS), free RAM, or
a guarantee against an allocation failure. A budget is a policy for these
forecast allocations, not an operating-system memory reservation or cap.

Before native preparation, a strict check can reject unavoidable input-copy
storage. Preparation then measures the Rust heap peak while processing the
actual input, and reconciles retained capacities. CMG hierarchy construction
supplies the actual retained graph, levels, terminal factors and workspaces.
The solver forecast is refined from that setup and the selected batches
before estimator random draws. This avoids pricing every problem at a maximum
terminal size or adding allocations whose lifetimes do not overlap.

Preparation and hierarchy construction themselves require memory. A strict
check can therefore stop after deterministic preparation has already run;
it is not a promise to stay below the budget during preparation. An early
preparation warning may appear before solving. The final forecast and any
final over-budget warning are returned with a successful command; they are
not a separate dry-run facility. Caller-state restoration applies on errors
as well as successful completion.

The expected peak and the admission forecast are separate. The latter also
allows for conditional refinement work that might not be needed on a normal
solve. `warn` and `error` compare the admission forecast with the explicit
budget. A legacy percentage headroom diagnostic is not this conditional
reserve and does not independently reject a command.

Mata retains formula-based forecasts; its JLA scope includes modeled Stata
working data. Public Mata forecasts no longer add the historical fixed
runtime-RSS allowances. Read `e(memory_forecast_scope)` before comparing
backends. A zero conditional reserve means the model reports no separate
reserve, not that the calculation has zero uncertainty.

## Outcome-centering scratch

Mean adds the retained weighted mean and centered outcome factor, without
additional fit/probe solves or probe pools. Corrected exact retains unit
maps and the shared coefficient-system scratch, including bounded
exceptional-unit work. Corrected JLA additionally retains the full/two-half
deletion moments and correction maps from the existing leverage probes.
These allocations are forecast and charged before stochastic execution
under the same budget policy and strict-admission rules.

`mcse(off)` removes the numerical attachment's work and scratch. It does
not remove the requested Corrected point systems. No production
observation-square matrix or literal-copy-by-probe expansion is introduced
by centering. See [the centering contract](CENTERING.md).

## All-point-probe attachment

`mcse(off)` removes the additional derivative, fold and replay reservation.
Default all-MCSE can therefore increase the admission forecast; strict budgets
may reject an all request that admits the same point calculation with MCSE off.
It does not weaken allocation or residual gates.

The default `mcse(all)` adds pre-RNG admission reserve for derivative state,
compensated target folds, leverage scores, replay scratch and result export.
Replay width is at most four in Rust and eight in Mata, and never exceeds the
admitted point leverage width; no rows-by-probes matrix is retained.
Observation derivatives and fold storage are allocated only for
observation/stayer-observation portions.
The generic Rust row reserve is conservatively 640 bytes per stored row;
compressed reserve includes four atom/RHS columns.
Mata uses conservative increments `192*physical_copies + 640*stored_rows +
512*R + 4096` for generic execution and `576*deletion_units +
640*coefficient_cells + 512*R + 4096` for compressed execution. These increments
enter the existing phase-scratch model and preserve budget presence/policy.
The larger Mata reserve covers eight-direction buffers and six fixed target-fold
weights per physical observation copy. It trades bounded memory for less replay
work; it does not store copy-by-probe responses. Short-panel gather plans have
at most twice as many entries as source rows. Derivative tiles contain at most
1,024 copies and shrink on small inputs; diagnostic boundaries retain scalar
status handling. Rust allocation and admission identities are unchanged.

Rust charges the bounded attachment while solver state is live and reconciles
owned receipts and overlapping C/Stata destinations at export, including two
24-byte-per-replay-RHS transport copies. The prepared-data identity excludes
the numerical reserve; the command admission peak includes it. There is no
allocation proportional to rows times probe count. `e(mc_allocation_bound_bytes)`
reports the attachment bound, while command forecasts describe the applicable
peak lifetime. Allocation high-water and complete-process RSS are measured and
reported separately in the [source-bound status](ALL_PROBE_MCSE_STATUS.md).
A warning or disabled forecast policy never permits corrupt result receipts.

## Returned results

These returns are available after successful completion. Byte amounts use
bytes, except for `e(memory_gib)` itself.

| Return | Meaning |
| --- | --- |
| `e(memory_budget_supplied)` | 1 if `memory_gib()` was supplied, otherwise 0. |
| `e(memory_gib)` | Explicit budget in GiB; missing when omitted. |
| `e(memory_check)` | Requested policy, with `warn` as the default; a policy alone creates no budget. |
| `e(memory_forecast_bytes)` | Expected peak within the reported forecast scope. |
| `e(memory_admission_forecast_bytes)` | Forecast compared with an explicit budget, including conditional reserve. |
| `e(memory_conditional_reserve_bytes)` | Admission forecast minus expected peak. |
| `e(memory_forecast_scope)` | Text describing the backend's accounting scope. |
| `e(memory_forecast_model)` | Forecast model identifier, currently 1. |
| `e(batch_memory_budget_bytes)` | Route-specific planning allowance; missing when no budget is supplied. It need not equal the whole-command budget. |

For example, after a successful call:

```stata
display e(memory_budget_supplied)
display "`e(memory_check)'"
display "`e(memory_forecast_scope)'"
display e(memory_forecast_bytes) / 1024^3
display e(memory_admission_forecast_bytes) / 1024^3
```

The stored forecasts remain available with `nodisplay`. Disabling result
tables does not disable a requested memory warning. Existing detailed native
receipts continue to reconcile the prepared generation, solver and exported
results; advisory policy does not permit malformed or inconsistent receipts.

## Accuracy and reproducibility

The [September 10 implementation record](https://github.com/johannes-schmieder/fevc/blob/ffca8b5cfc0ff8c495923c00d93ca292528e57d9/fevc/docs/MEMORY_FORECAST_2026-09-10.md) and
[its machine-readable result](https://github.com/johannes-schmieder/fevc/blob/ffca8b5cfc0ff8c495923c00d93ca292528e57d9/fevc/docs/memory_forecast_v1_result.json) report four
full-CMG core fixtures with forecast excess of 0.17–2.64% over independently
measured direct allocation peaks. These are bounded development checks, not a
5% guarantee for other inputs or backends. Generic/exact and projection routes
have behavioral or allocation-coverage checks; component inference and stayer
augmentation still contain conservative bounds. Total process RSS has not
been calibrated to the registered accuracy target.

Record budget presence, policy, forecast scope, source/binary identity,
selected batches, cores and measured RSS separately when reproducing a run.
Historical commands without `memorycheck()` used the older strict policy.
For a current-source rerun that intends an explicit hard forecast gate, add
`memorycheck(error)` and record the changed source and policy. Do not rewrite
old benchmark receipts or interpret a new forecast as a new RSS measurement.

The public Stata command uses the versioned presence/policy interface. Legacy
native and private diagnostic entrypoints retain their previous numeric-limit
semantics for compatibility; their defaults are not public `fevc` defaults.

# Exact-V7 pre-rename baseline failure — 2026-08-24

## Run identity

- Checkout tip: `f9fb00dc6254116a51853b1c2be7162b0a48368e`
- Source commit under receipt: `7fcf1b20105a546e993e0b3186c842f05f1f79a8`
- Repository identity at execution: `johannes-schmieder/varcomp_kss`
- Command: `./ci/run_ci_profile.sh plugin-build`
- Platform: macOS arm64, local licensed Stata/MP 18
- Stata executable:
  `/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp`
- Stata bundle version: `18.0.130`
- Completed: `2026-08-24T19:46:02.594992Z`
- Profile result: `failure_kind=plugin_error`,
  `failure_detail=Stata arm64 clean-install omitted PASS marker`

`./ci/run_stata_ci.sh plugin-build` was first attempted exactly as the
earlier test plan displayed. That wrapper rejected the profile and directed
native qualification through `ci/run_ci_profile.sh`; it did not execute the
qualifier.

## Focused Stata invocation

```stata
quietly _vckss_rust_reconcile_exact_v7 0 0 1 1 `xworkers' `xfirms' 0 ///
    1e-10 1e-10 1e-12 500 `xmem' `xcopy' `xprep' `xresident'        ///
    `xsighi' `xsiglo' 50000000 0 0 1 2 1
```

The fixture had 12 worker levels, 4 firm levels, and planned complexity 15.
The exact diagnostic printed:

```text
EXACT_V7_RECONCILE_FAIL: planned exact V7 receipt reconciliation failed
```

`r(detail)` was exactly
`planned exact V7 receipt reconciliation failed`.

## Complete return list

```text
scalars:
  r(actual_accounting_residual) = 4.84421530667e-16
  r(residual_tolerance)         = 1.00000000000e-11
  r(full_fit_complete_residual) = 3.52069891321e-16
  r(plan_memory_bytes)          = 129196
  r(solve_peak_bytes)           = 129196
  r(pre_rng_lo)                 = 0
  r(pre_rng_hi)                 = 0
  r(counter_complete)           = 1
  r(plan_route_selected)        = 4
  r(plan_route_requested)       = 4
  r(plan_frozen)                = 1
  r(plan_resolved)              = 1
  r(plan_applicability)         = 1
  r(plan_exact_limit)           = 500
  r(plan_complexity)            = 15
  r(plan_compressed_eligibility)= 0
  r(plan_engine_reason)         = 1
  r(plan_engine_selected)       = 3
  r(plan_engine_requested)      = 0
  r(plan_algorithm_reason)      = 3
  r(plan_algorithm_selected)    = 1
  r(plan_algorithm_requested)   = 0
  r(engine_selected)            = 3
  r(engine_requested)           = 0
  r(algorithm_selected)         = 1
  r(algorithm_requested)        = 0
  r(ok)                         = 0

macros:
  r(execution_plan_schema) = "VCKSS-EXECUTION-PLAN-V1"
  r(result_family)         = "exact"
  r(detail)                = "planned exact V7 receipt reconciliation failed"

matrices:
  r(result) : 4 x 4
```

The subsequent `assert r(ok) == 1` failed with `r(9)`. No request, plan,
residual, accounting, memory, exact-limit, selection-reason, capability, or
pre-RNG counter check was removed or weakened.

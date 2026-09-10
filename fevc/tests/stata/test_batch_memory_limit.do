version 18.0
clear all
set more off
set varabbrev off

local oldpwd `"`c(pwd)'"'
capture confirm file "fevc/fevc.ado"
if _rc {
    capture confirm file "../../fevc.ado"
    if _rc exit 601
    quietly cd "../.."
    local pkgroot `"`c(pwd)'"'
}
else local pkgroot `"`c(pwd)'/fevc"'
adopath ++ `"`pkgroot'"'

// The explicit width slightly exceeds the old 35-percent scratch heuristic,
// but the full direct peak fits the declared allocation. It must run.
input double(y worker firm frequency)
 0 1 1 400000
 1 1 2 400000
 6 1 3 400000
11 2 1 400000
13 2 2 400000
14 2 3 400000
end

local rng_before `"`c(rngstate)'"'
quietly fevc y [fw=frequency], worker(worker) firm(firm)        ///
    deletion(observation) algorithm(jla) probes(2) batch(8)       ///
    preconditioner(diagonal) memory_gib(.38) seed(8675309)        ///
    tolerance(1e-10) backend(mata) rng(stata) nodisplay
assert "`e(status)'" == "KSS_POINT_ESTIMATES_ONLY"
assert "`e(batch_requested)'" == "8"
assert e(batch) == 8
assert e(N_physical) == 2400000
assert e(batch_physical_column_bytes) == 19200000
assert e(batch_scratch_forecast_bytes) > e(batch_memory_budget_bytes)
assert e(resource_peak_bytes) <= e(resource_hard_mem_bytes)
assert e(resource_admitted) == 1
assert `"`c(rngstate)'"' == `"`rng_before'"'

local rejecting_gib = .99*e(resource_peak_bytes)/1024^3

// Lowering only the actual allocation envelope fails at the complete direct
// peak before RNG; it is not mislabeled as a percentage-based batch failure.
capture noisily fevc y [fw=frequency], worker(worker) firm(firm) ///
    deletion(observation) algorithm(jla) probes(2) batch(8)       ///
    preconditioner(diagonal) memory_gib(`rejecting_gib') memorycheck(error) seed(8675309)        ///
    tolerance(1e-10) backend(mata) rng(stata) nodisplay
assert _rc == 498
assert inlist("`e(withholding_status)'",                       ///
    "GENERIC_RESOURCE_ADMISSION_FAILED", "SOLVER_MEMORY_LIMIT")
assert "`e(withholding_status)'" != "BATCH_MEMORY_LIMIT"
assert `"`c(rngstate)'"' == `"`rng_before'"'

di as result "PASS test_batch_memory_limit.do"
exit 0

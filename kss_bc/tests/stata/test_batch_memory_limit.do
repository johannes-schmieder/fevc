version 18.0
clear all
set more off
set varabbrev off

local oldpwd `"`c(pwd)'"'
capture confirm file "kss_bc/kss_bc.ado"
if _rc {
    capture confirm file "../../kss_bc.ado"
    if _rc exit 601
    quietly cd "../.."
    local pkgroot `"`c(pwd)'"'
}
else local pkgroot `"`c(pwd)'/kss_bc"'
adopath ++ `"`pkgroot'"'

// This deletion-safe two-worker, three-firm fixture has only six stored rows
// but six million literal physical copies. Observation JLA therefore needs a
// 48,000,000-byte physical sign vector for every simultaneous probe column.
input double(y worker firm observation_key frequency)
 0 1 1 1 1000000
 1 1 2 2 1000000
 6 1 3 3 1000000
11 2 1 4 1000000
13 2 2 5 1000000
14 2 3 6 1000000
end

local rng_before `"`c(rngstate)'"'
capture noisily kss_bc y [fw=frequency], worker(worker) firm(firm) ///
    deletion(observation) algorithm(jla) probeorder(observation_key) ///
    probes(40) batch(auto) preconditioner(diagonal) memory_gib(1) ///
    seed(8675309) tolerance(1e-10) nodisplay
assert _rc == 498
assert "`e(withholding_status)'" == "BATCH_MEMORY_LIMIT"
assert "`e(batch_requested)'" == "auto"
assert e(batch) == 8
assert e(N_retained) == 6
assert e(N_physical) == 6000000
assert e(batch_physical_column_bytes) == 48000000
assert e(batch_column_forecast_bytes) > e(batch_physical_column_bytes)
assert e(batch_scratch_forecast_bytes) == ///
    e(batch)*e(batch_column_forecast_bytes)
assert e(batch_scratch_forecast_bytes) > e(batch_memory_budget_bytes)
assert e(memory_forecast_bytes) == e(batch_scratch_forecast_bytes)
assert strtrim("`e(batch_routing_reason)'") != ""
assert `"`c(rngstate)'"' == `"`rng_before'"'

capture noisily kss_bc y [fw=frequency], worker(worker) firm(firm) ///
    deletion(observation) algorithm(jla) probeorder(observation_key) ///
    probes(40) batch(8) preconditioner(cmg) memory_gib(1) ///
    seed(8675309) tolerance(1e-10) nodisplay
assert _rc == 498
assert "`e(withholding_status)'" == "BATCH_MEMORY_LIMIT"
assert "`e(batch_requested)'" == "8"
assert e(batch) == 8
assert e(batch_scratch_forecast_bytes) > e(batch_memory_budget_bytes)
assert "`e(algorithm)'" == "jla"
assert "`e(deletion)'" == "observation"
assert `"`c(rngstate)'"' == `"`rng_before'"'

di as result "PASS test_batch_memory_limit.do"
exit 0

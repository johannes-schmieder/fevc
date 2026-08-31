version 18.0
clear
set more off
set varabbrev off

input double(y worker firm observation_key)
 0 1 1 1
 1 1 2 2
 6 1 3 3
11 2 1 4
13 2 2 5
14 2 3 6
end

set seed 20260815
fevc y, worker(worker) firm(firm) deletion(match) ///
    algorithm(jla) probeorder(observation_key) probes(40) ///
    engine(generic) ///
    batch(auto) preconditioner(diagonal) memory_gib(4) ///
    seed(8675309) tolerance(1e-10) nodisplay
assert "`e(preconditioner_requested)'" == "diagonal"
assert "`e(preconditioner_selected)'" == "DIAGONAL"
assert "`e(fallback_status)'" == "NOT_NEEDED"
assert "`e(batch_requested)'" == "auto"
if "`e(backend_selected)'" == "rust" {
    assert e(batch) == max(e(rust_plan_leverage_batch), ///
        e(rust_plan_target_batch))
    assert e(batch_scratch_forecast_bytes) == max(     ///
        e(rust_plan_lev_selected_bytes),               ///
        e(rust_plan_tgt_selected_bytes))
}
else assert e(batch) == 8
assert strtrim("`e(batch_routing_reason)'") != ""
assert e(batch_column_forecast_bytes) > 0
assert e(batch_scratch_forecast_bytes) == ///
    e(batch)*e(batch_column_forecast_bytes)
assert e(batch_scratch_forecast_bytes) <= e(batch_memory_budget_bytes)
assert e(memory_forecast_bytes) >= e(batch_scratch_forecast_bytes)
assert e(memory_gib) == 4
confirm matrix e(route_diagnostics)

fevc y, worker(worker) firm(firm) deletion(match) ///
    algorithm(jla) probeorder(observation_key) probes(40) ///
    engine(generic) ///
    batch(17) preconditioner(auto) memory_gib(1) ///
    seed(8675309) tolerance(1e-10) nodisplay
assert "`e(preconditioner_requested)'" == "auto"
assert "`e(preconditioner_selected)'" == "DIAGONAL"
assert strtrim("`e(routing_reason)'") != ""
assert strtrim("`e(fallback_status)'") != ""
assert "`e(batch_requested)'" == "17"
assert e(batch) == 17
assert e(memory_gib) == 1

local rng_before_failure `"`c(rngstate)'"'
capture noisily fevc y, worker(worker) firm(firm) deletion(match) ///
    algorithm(jla) probeorder(observation_key) probes(40) ///
    engine(generic) ///
    batch(8) preconditioner(cmg) memory_gib(4) ///
    seed(8675309) tolerance(1e-10) nodisplay
local forced_rc = _rc
assert inlist(`forced_rc',0,498)
assert "`e(preconditioner_requested)'" == "cmg"
assert "`e(preconditioner_selected)'" == "CMG"
assert "`e(fallback_status)'" == "NOT_NEEDED"
assert strtrim("`e(routing_reason)'") != ""
if `forced_rc' == 498 {
    assert "`e(status)'" == "WITHHELD"
    assert `"`c(rngstate)'"' == `"`rng_before_failure'"'
}
else assert "`e(status)'" == "KSS_POINT_ESTIMATES_ONLY"

capture noisily fevc y, worker(worker) firm(firm) ///
    memory_gib(0) nodisplay
assert _rc == 198
assert "`e(withholding_status)'" == "INVALID_MEMORY_ENVELOPE"
fevc y, worker(worker) firm(firm) algorithm(exact) ///
    memory_gib(57) nodisplay
assert e(memory_gib) == 57
foreach bad_wall in 0 nonsense {
    capture noisily fevc y, worker(worker) firm(firm) ///
        wallseconds(`bad_wall') nodisplay
    assert _rc == 198
    assert "`e(withholding_status)'" == "INVALID_WALL_ENVELOPE"
}
fevc y, worker(worker) firm(firm) algorithm(exact) ///
    wallseconds(1000000) nodisplay
capture noisily fevc y, worker(worker) firm(firm) ///
    preconditioner(unknown) nodisplay
assert _rc == 198
assert "`e(withholding_status)'" == "INVALID_PRECONDITIONER"
capture noisily fevc y, worker(worker) firm(firm) batch(none) nodisplay
assert _rc == 198
assert "`e(withholding_status)'" == "INVALID_TUNING"

di as result "PASS test_routing_api.do"

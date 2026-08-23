version 18.0
clear all
set more off

args package_dir
if `"`package_dir'"' == "" {
    di as error "package source directory argument required"
    exit 198
}
adopath ++ `"`package_dir'"'

varcomp_kss_rust clear
clear
set obs 96
generate long cell = floor((_n - 1) / 2)
generate double worker = floor(cell / 4) + 1
generate double firm = mod(cell, 4) + 1
generate double replicate = mod(_n - 1, 2)
generate double deletion_id = cell + 1
generate double outcome = 0.7 * worker - 0.45 * firm + 0.3 * replicate + ///
    mod(7 * (_n - 1), 5) / 11
generate double frequency = mod(_n - 1, 3) + 1
generate double target_weight = 0.5 + mod(5 * (_n - 1), 7) / 3

quietly varcomp_kss_rust requestcapability, algorithm(jla) deletion(match) ///
    nuisance(joint) route(diagonal) rngcontract(counter_v1) controls(0)    ///
    frequencyused(1) engine(auto) batchmode(explicit)                     ///
    leveragebatchmode(explicit) targetbatchmode(explicit) stayers(movers) ///
    targetweightmode(explicit) deletionsource(matchid) fallback(0)        ///
    wallsecondssupplied(0) wallseconds(0) physicallimit(50000000)
assert r(supported) == 1
assert r(request_schema) == 3
assert r(profile_code) == 4
assert r(engine_code) == 0
assert r(engine_resolution_deferred) == 1
assert r(solver_route_code) == 2
assert r(route_resolution_deferred) == 0
local capability_schema = r(request_schema)
local capability_profile = r(profile_code)
local signature_hi = r(request_signature_hi)
local signature_lo = r(request_signature_lo)

quietly varcomp_kss_rust prepare worker firm deletion_id outcome frequency ///
    target_weight, cleanup memorygib(1) deletion(match)
local handle = r(handle)
assert `handle' > 0
assert r(controls_count) == 0
assert r(deletion_mode_code) == 1

local probes = 7
quietly varcomp_kss_rust solve `handle', algorithm(jla) deletion(match) ///
    nuisance(joint) route(diagonal) seed(81227) probes(`probes')         ///
    leveragebatch(2) targetbatch(2) tolerance(1e-12) engine(auto)       ///
    batchmode(explicit) leveragebatchmode(explicit)                      ///
    targetbatchmode(explicit) stayers(movers)                            ///
    targetweightmode(explicit) deletionsource(matchid)                   ///
    physicallimit(50000000) capabilityschema(`capability_schema')       ///
    capabilityprofile(`capability_profile') frequencyused(1)            ///
    signaturehi(`signature_hi') signaturelo(`signature_lo') fallback(0) ///
    wallsecondssupplied(0) wallseconds(0)

quietly varcomp_kss_rust result `handle'
assert r(capability_schema) == 3
assert r(capability_profile) == 4
assert r(request_signature_hi) == `signature_hi'
assert r(request_signature_lo) == `signature_lo'
assert r(rhs_receipt_schema) == 1
assert r(requested_algorithm_code) == 2
assert r(selected_algorithm_code) == 2
assert r(requested_engine_code) == 0
assert r(selected_engine_code) == 1
assert r(requested_route) == 2
assert r(selected_route) == 2
assert r(solver_fallback) == 0
assert r(solver_fallback_error) == 0
assert r(full_fit_complete_residual) <= r(full_residual_tolerance)
assert r(max_complete_residual) <= r(full_residual_tolerance)
assert r(actual_accounting_residual) <= 1e-10
assert r(rhs_receipt_rows) == 1 + 3 * `probes'
assert r(caller_result_copy_bytes) == r(rhs_receipt_rows) * 112
assert r(rhs_v2_caller_copy_bytes) == 0

assert `"`r(receipt_schema)'"' == "VCKSS-EXECUTION-PLAN-V1"
assert r(plan_struct) == 1000
assert r(plan_schema) == 1
assert r(plan_resolved) == 1
assert r(plan_frozen) == 1
assert r(plan_applicability) == 2
assert r(plan_alg_req) == 2
assert r(plan_alg_sel) == 2
assert r(plan_eng_req) == 0
assert r(plan_eng_sel) == 1
assert r(plan_route_req) == 2
assert r(plan_route_sel) == 2
assert r(plan_route_fallback) == 0
assert r(plan_route_error) == 0
assert r(plan_rhs) == r(rhs_receipt_rows)
assert r(batch_lev_sel) == 2
assert r(batch_tgt_sel) == 2
assert r(batch_command) == r(mem_command)
assert r(mem_command) == r(solve_peak_forecast_bytes)
assert r(mem_command) <= r(command_peak_forecast_bytes)
assert r(ctr_complete) == 1
assert r(plan_res_rng_hi) == 0
assert r(plan_res_rng_lo) == 0

tempname estimates rhs
matrix `estimates' = r(result)
matrix `rhs' = r(rhs_receipts)
assert rowsof(`estimates') == 4
assert colsof(`estimates') == 4
assert rowsof(`rhs') == 1 + 3 * `probes'
assert colsof(`rhs') == 8
forvalues column = 1/4 {
    assert abs(`estimates'[1,`column'] - `estimates'[2,`column'] - ///
        `estimates'[3,`column']) <= 1e-10
}
assert `rhs'[1,1] == 1
assert `rhs'[1,2] == -1
assert `rhs'[1,3] == 0
assert `rhs'[1,4] == 2
local row = 2
forvalues probe = 0/`=`probes'-1' {
    assert `rhs'[`row',1] == 2
    assert `rhs'[`row',2] == `probe'
    assert `rhs'[`row',3] == 0
    assert `rhs'[`row',4] == 2
    local ++row
}
forvalues probe = 0/`=`probes'-1' {
    foreach side in 1 2 {
        assert `rhs'[`row',1] == 3
        assert `rhs'[`row',2] == `probe'
        assert `rhs'[`row',3] == `side'
        assert `rhs'[`row',4] == 2
        local ++row
    }
}
assert `row' == rowsof(`rhs') + 1
forvalues row = 1/`=rowsof(`rhs')' {
    assert `rhs'[`row',5] >= 0
    assert `rhs'[`row',6] >= 0
    assert `rhs'[`row',7] >= 0
    assert inlist(`rhs'[`row',8], 0, 1)
}

quietly varcomp_kss_rust release `handle'
quietly varcomp_kss_rust release `handle'
quietly varcomp_kss_rust snapshot
assert r(state) == 0
assert r(handle) == 0

di as result "VARCOMP_KSS RUST PLANNED COMPRESSED V4 PASS"
exit 0

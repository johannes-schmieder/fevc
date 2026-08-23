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
    nuisance(joint) route(auto) rngcontract(counter_v1) controls(0)        ///
    frequencyused(1) engine(generic) batchmode(independent)               ///
    leveragebatchmode(auto) targetbatchmode(explicit) stayers(movers)     ///
    targetweightmode(explicit) deletionsource(matchid) fallback(1)        ///
    wallsecondssupplied(1) wallseconds(60) physicallimit(50000000)
assert r(supported) == 1
assert r(request_schema) == 3
assert r(profile_code) == 4
assert r(engine_code) == 2
assert r(batch_mode_code) == 2
assert r(leverage_batch_mode_code) == 0
assert r(target_batch_mode_code) == 1
assert r(automatic_fallback_allowed) == 1
assert r(wallseconds_supplied) == 1
assert r(wallseconds) == 60
assert r(wall_advisory_only) == 1
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
capture quietly varcomp_kss_rust solve `handle', algorithm(jla) deletion(match) ///
    nuisance(joint) route(auto) seed(81227) probes(`probes')              ///
    leveragebatch(3) targetbatch(2) tolerance(1e-12) engine(generic)      ///
    batchmode(independent) leveragebatchmode(auto)                        ///
    targetbatchmode(explicit) stayers(movers)                             ///
    targetweightmode(explicit) deletionsource(matchid)                    ///
    physicallimit(50000000) capabilityschema(`capability_schema')        ///
    capabilityprofile(`capability_profile') frequencyused(1)             ///
    signaturehi(`signature_hi') signaturelo(`signature_lo') fallback(1)  ///
    wallsecondssupplied(1) wallseconds(60)
local invalid_auto_rc = _rc
assert `invalid_auto_rc' == 198

capture quietly varcomp_kss_rust solve `handle', algorithm(jla) deletion(match) ///
    nuisance(joint) route(auto) seed(81227) probes(`probes')              ///
    leveragebatch(0) targetbatch(0) tolerance(1e-12) engine(generic)      ///
    batchmode(independent) leveragebatchmode(auto)                        ///
    targetbatchmode(explicit) stayers(movers)                             ///
    targetweightmode(explicit) deletionsource(matchid)                    ///
    physicallimit(50000000) capabilityschema(`capability_schema')        ///
    capabilityprofile(`capability_profile') frequencyused(1)             ///
    signaturehi(`signature_hi') signaturelo(`signature_lo') fallback(1)  ///
    wallsecondssupplied(1) wallseconds(60)
local invalid_explicit_rc = _rc
assert `invalid_explicit_rc' == 198

quietly varcomp_kss_rust solve `handle', algorithm(jla) deletion(match)   ///
    nuisance(joint) route(auto) seed(81227) probes(`probes')              ///
    leveragebatch(0) targetbatch(2) tolerance(1e-12) engine(generic)     ///
    batchmode(independent) leveragebatchmode(auto)                        ///
    targetbatchmode(explicit) stayers(movers)                             ///
    targetweightmode(explicit) deletionsource(matchid)                    ///
    physicallimit(50000000) capabilityschema(`capability_schema')        ///
    capabilityprofile(`capability_profile') frequencyused(1)             ///
    signaturehi(`signature_hi') signaturelo(`signature_lo') fallback(1)  ///
    wallsecondssupplied(1) wallseconds(60)

quietly varcomp_kss_rust result `handle'
assert r(capability_schema) == 3
assert r(capability_profile) == 4
assert r(request_signature_hi) == `signature_hi'
assert r(request_signature_lo) == `signature_lo'
assert r(rhs_receipt_schema) == 2
assert r(requested_algorithm_code) == 2
assert r(selected_algorithm_code) == 2
assert r(requested_engine_code) == 2
assert r(selected_engine_code) == 2
assert r(requested_route) == 0
assert r(selected_route) == 2
assert r(solver_fallback) == 0
assert r(full_fit_complete_residual) <= r(full_residual_tolerance)
assert r(max_complete_residual) <= r(full_residual_tolerance)
assert r(actual_accounting_residual) <= 1e-10

assert `"`r(receipt_schema)'"' == "VCKSS-EXECUTION-PLAN-V1"
assert r(plan_struct) == 1000
assert r(plan_schema) == 1
assert r(plan_alg_schema) == 1
assert r(plan_eng_schema) == 2
assert r(plan_route_schema) == 1
assert r(batch_schema) == 1
assert r(wall_schema) == 1
assert r(ctr_schema) == 1
assert r(mem_schema) == 1
assert r(plan_resolved) == 1
assert r(plan_frozen) == 1
assert r(plan_alg_req) == r(requested_algorithm_code)
assert r(plan_alg_sel) == r(selected_algorithm_code)
assert r(plan_eng_req) == r(requested_engine_code)
assert r(plan_eng_sel) == r(selected_engine_code)
assert r(plan_route_req) == r(requested_route)
assert r(plan_route_sel) == r(selected_route)
assert r(plan_route_fallback) == r(solver_fallback)
assert r(plan_route_error) == r(solver_fallback_error)
assert r(plan_full_dim) == r(solver_dimension)
assert r(plan_rhs) == r(rhs_receipt_rows)
assert r(plan_threads_req) >= 1
assert r(plan_threads_used) >= 1
assert r(plan_threads_used) <= r(plan_threads_req)
assert r(plan_parallel) == (r(plan_threads_used) > 1)

assert r(batch_determ) == 1
assert r(batch_invariant) == 1
assert r(batch_admitted) == 1
assert r(batch_lev_app) == 1
assert r(batch_tgt_app) == 1
assert r(batch_lev_mode) == 0
assert r(batch_tgt_mode) == 1
assert r(batch_lev_sel) == r(leverage_batch_width)
assert r(batch_tgt_sel) == r(target_batch_width)
assert r(batch_lev_sel) >= 1
assert r(batch_lev_sel) <= `probes'
assert r(batch_tgt_sel) == 2
assert r(batch_command) == r(command_peak_forecast_bytes)
assert r(batch_nonbatched) == r(mem_nonbatched)

assert r(wall_routing) == 1
assert r(wall_req_app) == 1
assert r(wall_requested) == 60
assert r(wall_total) == r(wall_prepare) + r(wall_setup) + r(wall_fit) + ///
    r(wall_leverage) + r(wall_target) + r(wall_export)
assert r(ctr_work_app) == 1
assert r(ctr_complete) == 1
assert r(plan_res_rng_hi) == 0
assert r(plan_res_rng_lo) == 0
assert r(plan_res_ctr_hi) == 0
assert r(plan_res_ctr_lo) == 0
assert r(ctr_pre_atom_hi) == 0
assert r(ctr_pre_atom_lo) == 0
assert r(ctr_pre_word_hi) == 0
assert r(ctr_pre_word_lo) == 0
assert r(ctr_pre_trial_hi) == 0
assert r(ctr_pre_trial_lo) == 0

assert r(mem_app) == 1
assert r(mem_hard) == r(memory_limit_bytes)
assert r(mem_prepared) == r(prepared_resident_bytes)
assert r(mem_command) == r(command_peak_forecast_bytes)
assert r(mem_result) == r(result_forecast_bytes)
assert r(plan_sig_hi) == `signature_hi'
assert r(plan_sig_lo) == `signature_lo'

tempname estimates rhs
matrix `estimates' = r(result)
matrix `rhs' = r(rhs_receipts)
assert rowsof(`estimates') == 4
assert colsof(`estimates') == 4
assert rowsof(`rhs') == r(rhs_receipt_rows)
assert colsof(`rhs') == 15
forvalues column = 1/4 {
    assert abs(`estimates'[1,`column'] - `estimates'[2,`column'] -       ///
        `estimates'[3,`column']) <= 1e-10
}

quietly varcomp_kss_rust release `handle'
quietly varcomp_kss_rust release `handle'
quietly varcomp_kss_rust snapshot
assert r(state) == 0

di as result "VARCOMP_KSS RUST PLANNED V4 PASS"
exit 0

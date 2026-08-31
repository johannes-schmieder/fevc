version 18.0
clear all
set more off

args package_dir
if `"`package_dir'"' == "" {
    di as error "package source directory argument required"
    exit 198
}
adopath ++ `"`package_dir'"'

local rust_plugin
if strpos("`c(machine_type)'", "Mac") == 1 {
    local rust_plugin _fevc_rust_macos
}
else if "`c(os)'" == "Windows" {
    local rust_plugin _fevc_rust_windows
}
else if "`c(os)'" == "Unix" {
    local rust_plugin _fevc_rust_linux
}
else {
    di as error "unsupported Rust test platform: `c(os)' / `c(machine_type)'"
    exit 198
}

fevc_rust clear
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

quietly fevc_rust requestcapability, algorithm(jla) deletion(match) ///
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

quietly fevc_rust prepare worker firm deletion_id outcome frequency ///
    target_weight, cleanup memorygib(1) deletion(match)
local handle = r(handle)
assert `handle' > 0
assert r(controls_count) == 0
assert r(deletion_mode_code) == 1

local probes = 7
capture quietly fevc_rust solve `handle', algorithm(jla) deletion(match) ///
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

capture quietly fevc_rust solve `handle', algorithm(jla) deletion(match) ///
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

quietly fevc_rust solve `handle', algorithm(jla) deletion(match)   ///
    nuisance(joint) route(auto) seed(81227) probes(`probes')              ///
    leveragebatch(0) targetbatch(2) tolerance(1e-12) engine(generic)     ///
    batchmode(independent) leveragebatchmode(auto)                        ///
    targetbatchmode(explicit) stayers(movers)                             ///
    targetweightmode(explicit) deletionsource(matchid)                    ///
    physicallimit(50000000) capabilityschema(`capability_schema')        ///
    capabilityprofile(`capability_profile') frequencyused(1)             ///
    signaturehi(`signature_hi') signaturelo(`signature_lo') fallback(1)  ///
    wallsecondssupplied(1) wallseconds(60)

quietly fevc_rust result `handle'
assert r(performance_schema) == 1
assert mod(r(performance_flags),4) == 3
assert r(performance_ingest_ns) >= 0
assert r(performance_canonicalize_ns) >= 0
assert r(performance_graph_ns) >= 0
assert r(performance_compress_ns) >= 0
assert r(performance_plan_ns) >= 0
assert r(performance_stayer_ns) >= 0
assert r(performance_solve_ns) >= 0
assert r(performance_total_ns) >= r(performance_solve_ns)
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
assert r(plan_route_schema) == 2
assert r(batch_schema) == 1
assert r(wall_schema) == 1
assert r(ctr_schema) == 1
assert r(mem_schema) == 1
assert r(plan_resolved) == 1
assert r(plan_frozen) == 1
assert r(plan_applicability) == 3
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
assert r(batch_arithmetic) == 1
assert r(batch_admitted) == 1
assert r(batch_app) == 3
assert r(batch_lev_app) == 3
assert r(batch_tgt_app) == 3
assert r(batch_lev_mode) == 0
assert r(batch_tgt_mode) == 1
assert r(batch_lev_reason) == 1
assert r(batch_tgt_reason) == 2
assert r(batch_lev_sel) == r(leverage_batch_width)
assert r(batch_tgt_sel) == r(target_batch_width)
assert r(batch_lev_sel) >= 1
assert r(batch_lev_sel) <= `probes'
assert r(batch_tgt_sel) == 2
assert r(batch_command) == r(mem_command)
assert r(batch_nonbatched) == r(mem_nonbatched)

assert r(wall_model) == 3
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

assert r(mem_app) == 3
assert r(mem_hard) == r(memory_limit_bytes)
assert r(mem_prepared) == r(prepared_resident_bytes)
assert r(mem_command) == r(solve_peak_forecast_bytes)
assert r(command_peak_forecast_bytes) == max(                      ///
    r(preparation_peak_forecast_bytes), r(solve_peak_forecast_bytes))
assert r(mem_command) <= r(command_peak_forecast_bytes)
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

quietly fevc_rust release `handle'
quietly fevc_rust release `handle'
quietly fevc_rust snapshot
assert r(state) == 0

// Inspect a forced-CMG V4/V7 result before the public rclass wrapper applies
// its legacy-prefix reconciliation.  This remains a private regression test:
// the frozen V6 result and V2 RHS prefixes deliberately remain diagonal,
// while V7 carries the actual forced-CMG route.
quietly fevc_rust requestcapability, algorithm(jla) deletion(match) ///
    nuisance(joint) route(cmg) rngcontract(counter_v1) controls(0)        ///
    frequencyused(1) engine(generic) batchmode(explicit)                  ///
    leveragebatchmode(explicit) targetbatchmode(explicit) stayers(movers) ///
    targetweightmode(explicit) deletionsource(matchid) fallback(0)        ///
    wallsecondssupplied(0) wallseconds(0) physicallimit(50000000)
assert r(supported) == 1 & r(request_schema) == 3 & r(profile_code) == 4
assert r(solver_route_code) == 3 & r(automatic_fallback_allowed) == 0
local cmg_schema = r(request_schema)
local cmg_profile = r(profile_code)
local cmg_signature_hi = r(request_signature_hi)
local cmg_signature_lo = r(request_signature_lo)

quietly fevc_rust prepare worker firm deletion_id outcome frequency ///
    target_weight, cleanup memorygib(1) deletion(match)
local cmg_handle = r(handle)
quietly fevc_rust solve `cmg_handle', algorithm(jla) deletion(match) ///
    nuisance(joint) route(cmg) seed(81227) probes(7) leveragebatch(2)      ///
    targetbatch(2) tolerance(1e-12) engine(generic) batchmode(explicit)   ///
    leveragebatchmode(explicit) targetbatchmode(explicit) stayers(movers) ///
    targetweightmode(explicit) deletionsource(matchid)                    ///
    physicallimit(50000000) capabilityschema(`cmg_schema')                ///
    capabilityprofile(`cmg_profile') frequencyused(1)                    ///
    signaturehi(`cmg_signature_hi') signaturelo(`cmg_signature_lo')      ///
    fallback(0) wallsecondssupplied(0) wallseconds(0)

quietly _fevc_rust_plugin_call `rust_plugin', result `cmg_handle'
assert scalar(__vckss_rust_cap_schema_echo) == 3
assert scalar(__vckss_rust_cap_profile_echo) == 4
assert scalar(__vckss_rust_route_requested) == 2
assert scalar(__vckss_rust_route_selected) == 2
assert scalar(__vckss_plan_route_req) == 3
assert scalar(__vckss_plan_route_sel) == 3
assert scalar(__vckss_plan_route_fallback) == 0
assert scalar(__vckss_plan_route_error) == 0
assert scalar(__vckss_rust_full_route) == 2
assert scalar(__vckss_rust_solve_peak) == scalar(__vckss_mem_command)
assert scalar(__vckss_rust_command_peak) == max(                     ///
    scalar(__vckss_rust_prepare_peak),scalar(__vckss_mem_command))

di as result "VCKSS_CMG_RAW solver_setup="                         ///
    scalar(__vckss_rust_solver_setup) " g_canon="                   ///
    scalar(__vckss_rust_g_canon_peak) " g_fit="                    ///
    scalar(__vckss_rust_g_fit_peak) " g_geometry="                 ///
    scalar(__vckss_rust_g_geometry_peak) " g_lev="                 ///
    scalar(__vckss_rust_g_lev_peak) " g_tgt="                      ///
    scalar(__vckss_rust_g_tgt_peak) " g_maker="                    ///
    scalar(__vckss_rust_g_maker_peak) " g_result="                 ///
    scalar(__vckss_rust_g_result_bytes) " g_peak="                 ///
    scalar(__vckss_rust_g_peak)
di as result "VCKSS_CMG_PLAN mem_setup=" scalar(__vckss_mem_setup) ///
    " mem_fit=" scalar(__vckss_mem_fit)                             ///
    " mem_correction=" scalar(__vckss_mem_correction)               ///
    " mem_lev=" scalar(__vckss_mem_leverage)                        ///
    " mem_tgt=" scalar(__vckss_mem_target)                          ///
    " mem_result=" scalar(__vckss_mem_result)                       ///
    " mem_nonbatched=" scalar(__vckss_mem_nonbatched)               ///
    " mem_command=" scalar(__vckss_mem_command)                     ///
    " shared_cmg=" scalar(__vckss_mem_shared_cmg)                   ///
    " cmg_workspace=" scalar(__vckss_mem_cmg_workspace)             ///
    " cmg_cells=" scalar(__vckss_mem_cmg_cells)                     ///
    " cmg_groups=" scalar(__vckss_mem_cmg_groups)                   ///
    " cmg_graph=" scalar(__vckss_mem_cmg_graph)

local cmg_rhs_rows = scalar(__vckss_rust_rhs_rows)
tempname cmg_rhs_receipts
matrix `cmg_rhs_receipts' = J(`cmg_rhs_rows',15,.)
quietly _fevc_rust_plugin_call `rust_plugin', rhsresult           ///
    `cmg_handle' `cmg_rhs_receipts'
forvalues row = 1/`cmg_rhs_rows' {
    assert `cmg_rhs_receipts'[`row',4] == 2
}
quietly _fevc_rust_plan_receipt
assert r(plan_route_req) == 3 & r(plan_route_sel) == 3
assert r(plan_route_fallback) == 0 & r(plan_route_error) == 0
assert r(mem_command) == scalar(__vckss_rust_solve_peak)
assert scalar(__vckss_rust_route_requested) == 3
assert scalar(__vckss_rust_route_selected) == 3
quietly fevc_rust release `cmg_handle'
quietly fevc_rust snapshot
assert r(state) == 0 & r(handle) == 0

// A V3 algorithm(auto) request must reconcile an exact selection without
// borrowing JLA batch or iterative-route semantics. The retained identified
// dimension is 12+4-1=15, below exactlimit(500).
quietly fevc_rust requestcapability, algorithm(auto) deletion(match) ///
    nuisance(joint) route(auto) rngcontract(counter_v1) controls(0)        ///
    frequencyused(1) engine(auto) batchmode(auto)                          ///
    leveragebatchmode(auto) targetbatchmode(auto) stayers(movers)          ///
    targetweightmode(explicit) deletionsource(matchid) fallback(1)         ///
    wallsecondssupplied(0) wallseconds(0) physicallimit(50000000)
assert r(supported) == 1 & r(request_schema) == 3 & r(profile_code) == 4
assert r(algorithm_code) == 0 & r(engine_code) == 0 & r(solver_route_code) == 0
assert r(algorithm_resolution_deferred) == 1
assert r(engine_resolution_deferred) == 1
assert r(route_resolution_deferred) == 1
assert r(leverage_batch_deferred) == 1
assert r(target_batch_resolution_deferred) == 1
local exact_auto_schema = r(request_schema)
local exact_auto_profile = r(profile_code)
local exact_auto_signature_hi = r(request_signature_hi)
local exact_auto_signature_lo = r(request_signature_lo)

quietly fevc_rust prepare worker firm deletion_id outcome frequency ///
    target_weight, cleanup memorygib(1) deletion(match)
local exact_auto_handle = r(handle)
quietly fevc_rust solve `exact_auto_handle', algorithm(auto)         ///
    deletion(match) nuisance(joint) route(auto) seed(81227) probes(7)      ///
    leveragebatch(0) targetbatch(0) tolerance(1e-12) maxiter(10000)       ///
    exactlimit(500) blocksizelimit(5000) engine(auto) batchmode(auto)      ///
    leveragebatchmode(auto) targetbatchmode(auto) stayers(movers)          ///
    targetweightmode(explicit) deletionsource(matchid)                     ///
    physicallimit(50000000) capabilityschema(`exact_auto_schema')          ///
    capabilityprofile(`exact_auto_profile') frequencyused(1)              ///
    signaturehi(`exact_auto_signature_hi')                                ///
    signaturelo(`exact_auto_signature_lo') fallback(1)                    ///
    wallsecondssupplied(0) wallseconds(0)

quietly fevc_rust result `exact_auto_handle'
assert r(performance_schema) == 1
assert mod(r(performance_flags),4) == 3
assert r(performance_ingest_ns) >= 0
assert r(performance_canonicalize_ns) >= 0
assert r(performance_graph_ns) >= 0
assert r(performance_compress_ns) >= 0
assert r(performance_plan_ns) >= 0
assert r(performance_stayer_ns) >= 0
assert r(performance_solve_ns) >= 0
assert r(performance_total_ns) >= r(performance_solve_ns)
assert `"`r(receipt_schema)'"' == "VCKSS-EXECUTION-PLAN-V1"
assert `"`r(requested_algorithm)'"' == "auto"
assert `"`r(selected_algorithm)'"' == "exact"
assert `"`r(requested_engine)'"' == "unspecified"
assert `"`r(selected_engine)'"' == "not_applicable"
assert `"`r(rng_contract)'"' == "none"
assert r(requested_algorithm_code) == 0
assert r(selected_algorithm_code) == 1
assert r(requested_engine_code) == 0
assert r(selected_engine_code) == 3
assert r(requested_route) == 1 & r(selected_route) == 1
assert r(solver_fallback) == 0 & r(solver_fallback_error) == 0
assert r(rhs_receipt_schema) == 0 & r(rhs_receipt_rows) == 0
assert r(caller_result_copy_bytes) == 0 & r(rhs_v2_caller_copy_bytes) == 0
assert r(seed) == 0 & r(probes) == 0
assert r(parameters) == 15 & r(solver_dimension) == 15
assert r(information_rcond) > 0 & r(information_rcond) <= 1
assert r(actual_accounting_residual) <= 1e-10
assert r(capability_schema) == 3 & r(capability_profile) == 4
assert r(request_signature_hi) == `exact_auto_signature_hi'
assert r(request_signature_lo) == `exact_auto_signature_lo'

assert r(plan_struct) == 1000 & r(plan_schema) == 1
assert r(plan_alg_schema) == 1 & r(plan_eng_schema) == 2
assert r(plan_alg_req) == 0 & r(plan_alg_sel) == 1
assert r(plan_alg_reason) == 3
assert r(plan_eng_req) == 0 & r(plan_eng_sel) == 3
assert r(plan_eng_reason) == 1 & r(plan_comp_elig) == 0
assert r(plan_complexity) == 15 & r(plan_exact_limit) == 500
assert r(plan_resolved) == 1 & r(plan_frozen) == 1
assert r(plan_applicability) == 1
assert r(plan_app_hi) == 0 & r(plan_app_lo) == 59
assert r(plan_contract_hi) == 0 & r(plan_contract_lo) == 31
assert r(plan_eng_fallback) == 0
assert r(plan_route_schema) == 1
assert r(plan_route_req) == 4 & r(plan_route_sel) == 4
assert r(plan_route_fallback) == 0 & r(plan_route_error) == 0
assert r(plan_route_contract) == 0
assert r(plan_rhs) == 0 & r(plan_full_dim) == 0 & r(plan_fe_dim) == 0
assert r(plan_threads_req) >= 1
assert r(plan_threads_used) >= 1 & r(plan_threads_used) <= r(plan_threads_req)
assert r(plan_parallel) == (r(plan_threads_used) > 1)

assert r(batch_schema) == 1 & r(batch_app) == 1
assert r(batch_determ) == 0 & r(batch_invariant) == 0
assert r(batch_arithmetic) == 0 & r(batch_admitted) == 0
assert r(batch_nonbatched) == 0 & r(batch_command) == 0
foreach phase in lev tgt {
    assert r(batch_`phase'_app) == 1
    assert r(batch_`phase'_mode) == 3
    assert r(batch_`phase'_reason) == 0
    assert r(batch_`phase'_req) == 0 & r(batch_`phase'_sel) == 0
    assert r(batch_`phase'_probe) == 0 & r(batch_`phase'_threads) == 0
    assert r(batch_`phase'_threadcap) == 0
    assert r(batch_`phase'_routecap) == 0 & r(batch_`phase'_effcap) == 0
    assert r(batch_`phase'_hard) == 0
    assert r(batch_`phase'_onebytes) == 0 & r(batch_`phase'_selbytes) == 0
}

assert r(wall_schema) == 1 & r(wall_model) == 1 & r(wall_routing) == 1
assert r(wall_req_app) == 0 & r(wall_requested) == 0
assert r(wall_total) == r(wall_prepare) + r(wall_setup) + r(wall_fit) + ///
    r(wall_leverage) + r(wall_target) + r(wall_export)
assert r(ctr_schema) == 1 & r(ctr_rng) == 0 & r(ctr_complete) == 1
assert r(plan_res_rng_hi) == 0 & r(plan_res_rng_lo) == 0
assert r(plan_res_ctr_hi) == 0 & r(plan_res_ctr_lo) == 0
assert r(ctr_pre_atom_hi) == 0 & r(ctr_pre_atom_lo) == 0
assert r(ctr_pre_word_hi) == 0 & r(ctr_pre_word_lo) == 0
assert r(ctr_pre_trial_hi) == 0 & r(ctr_pre_trial_lo) == 0
assert r(mem_schema) == 1 & r(mem_app) == 1
assert r(mem_hard) == r(memory_limit_bytes)
assert r(mem_prepared) == r(prepared_resident_bytes)
assert r(mem_setup) == 0 & r(mem_leverage) == 0 & r(mem_target) == 0
assert r(mem_fit) > 0 & r(mem_correction) > 0
assert r(mem_nonbatched) == r(mem_command)
assert r(mem_command) == r(solve_peak_forecast_bytes)
assert r(command_peak_forecast_bytes) == max(                         ///
    r(preparation_peak_forecast_bytes),r(solve_peak_forecast_bytes))
assert r(plan_sig_hi) == `exact_auto_signature_hi'
assert r(plan_sig_lo) == `exact_auto_signature_lo'

tempname exact_auto_estimates
matrix `exact_auto_estimates' = r(result)
assert rowsof(`exact_auto_estimates') == 4 & colsof(`exact_auto_estimates') == 4
forvalues column = 1/4 {
    assert `exact_auto_estimates'[4,`column'] == 0
    assert abs(`exact_auto_estimates'[1,`column'] -                     ///
        `exact_auto_estimates'[2,`column'] -                            ///
        `exact_auto_estimates'[3,`column']) <= 1e-10
}
quietly fevc_rust release `exact_auto_handle'
quietly fevc_rust snapshot
assert r(state) == 0 & r(handle) == 0

di as result "FEVC RUST PLANNED V4 PASS"
exit 0

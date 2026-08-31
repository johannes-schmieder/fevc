version 18.0
clear all
set more off

args package_dir
if `"`package_dir'"' == "" {
    local package_dir = subinstr("`c(pwd)'","/tests/stata","",.)
}
adopath ++ `"`package_dir'"'
quietly run `"`package_dir'/fevc.ado"'

set obs 96
generate long row0 = _n-1
generate long cell = floor(row0/2)
generate long worker = floor(cell/4)+1
generate long firm = mod(cell,4)+1
generate long deletion_id = cell+1
generate byte replicate = mod(row0,2)
generate double outcome = .45*(worker-1)-.35*(firm-1)+.08*replicate+ ///
    .025*mod(cell,3)
generate long frequency = 1+mod(5*row0+2,3)
generate double target_weight = .75+(row0+1)/192

quietly count
local nscope = r(N)
local ncomplete = r(N)
local nstayers = 0
local nstayerrows = 0

quietly fevc_rust clear
quietly fevc_rust probe
local core_flags = r(core_ready_flags)
local support_flags = r(support_flags)

local caller_rng `"`c(rng)'"'
local caller_stream = c(rngstream)
local caller_state `"`c(rngstate)'"'
local caller_sortedby : sortedby
quietly _datasignature
local caller_signature `"`r(datasignature)'"'

// The direct runner owns its marked-sample variable.  Create that disposable
// marker only after freezing the caller-data signature; ereturn post consumes
// it as the active e(sample) without making it part of caller data.
tempvar internal_touse
generate byte `internal_touse' = 1

capture noisily _fevc_rust_generic_planned outcome worker firm deletion_id ///
    frequency target_weight `internal_touse' `nscope' `ncomplete' `nstayers' ///
    `nstayerrows' 7 2 81227 1e-12 10000 1 jla auto 1 1 1 1 1 1 1 0     ///
    `core_flags' `support_flags' "nodisplay" match joint 500 1e-10   ///
    1e-10 5000 50000000 "" 1 1                                      ///
    "fevc outcome [fw=frequency], backend(rust) engine(auto)" ///
    diagonal auto 0 0
assert _rc == 0

assert `"`e(cmd)'"' == "fevc"
assert `"`e(status)'"' == "KSS_SCALE_EXPERIMENTAL_POINT_ESTIMATES"
assert `"`e(backend_selected)'"' == "rust"
assert `"`e(rng_selected)'"' == "counter_v1"
assert `"`e(algorithm)'"' == "jla"
assert `"`e(engine_requested)'"' == "auto"
assert `"`e(engine_selected)'"' == "compressed"
assert `"`e(result_family)'"' == "compressed"
assert `"`e(route_api)'"' == "VCKSS-NATIVE-COMPRESSED-PLANNED-V4-V7"
assert `"`e(execution_plan_schema)'"' == "VCKSS-EXECUTION-PLAN-V1"
assert `"`e(rust_phase_profile_schema)'"' == "VCKSS-NATIVE-PHASE-PERF-V1"
assert `"`e(rust_phase_profile_units)'"' == "seconds"
assert rowsof(e(rust_phase_profile)) == 1
assert colsof(e(rust_phase_profile)) == 8
forvalues phase = 1/8 {
    assert e(rust_phase_profile)[1,`phase'] >= 0
}
assert e(rust_phase_profile)[1,8] >= e(rust_phase_profile)[1,7]
assert `"`e(preconditioner_requested)'"' == "diagonal"
assert `"`e(preconditioner_selected)'"' == "DIAGONAL"
assert `"`e(deletion)'"' == "match"
assert `"`e(nuisance)'"' == "joint"
assert `"`e(deletion_rank_certificate)'"' == ///
    "compressed match graph, quotient, and complete-model residual gates"

assert e(N_requested) == `nscope'
assert e(N_complete) == `ncomplete'
assert e(N_retained) == e(N_stored)
assert e(N_physical) == 192
assert e(worker_levels) == 12
assert e(firm_levels) == 4
assert e(parameters) == e(worker_levels)+e(firm_levels)-1
assert e(full_parameters) == e(parameters)
assert e(correction_parameters) == e(parameters)
assert e(controls_count) == 0
assert e(coefficient_cells) == 48
assert e(deletion_units) == 48
assert e(target_strata) == 96
assert e(target_strata) == e(N_stored)
assert e(target_weight_sum) == 96.25
assert e(probes) == 7
assert e(seed) == 81227
assert e(rust_requested_algorithm_code) == 2
assert e(rust_selected_algorithm_code) == 2
assert e(rust_requested_engine_code) == 0
assert e(rust_selected_engine_code) == 1
assert e(rust_rhs_receipt_schema) == 1
assert e(rust_result_cap_schema) == 3
assert e(rust_result_cap_profile) == 4
assert e(rust_cap_schema) == 3
assert e(rust_cap_profile_code) == 4
assert e(rust_cap_engine_deferred) == 1
assert e(rust_plan_schema) == 1
assert e(rust_plan_route_schema) == 2
assert e(rust_plan_resolved) == 1
assert e(rust_plan_frozen) == 1
assert e(rust_plan_applicability) == 2
assert e(rust_plan_engine_requested) == 0
assert e(rust_plan_engine_selected) == 1
assert e(rust_counter_plan_complete) == 1
assert e(rust_pre_rng_hi) == 0
assert e(rust_pre_rng_lo) == 0
assert e(rust_plan_rhs) == 1+3*e(probes)
assert e(rust_plan_full_dimension) == e(firm_levels)-1
assert e(rust_solver_dimension) == e(firm_levels)-1
assert e(rust_full_fit_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_max_complete_residual) <= e(residual_acceptance_tolerance)
assert e(target_identity_residual) == e(rust_actual_accounting_residual)
assert e(rust_rhs_v2_copy_bytes) == 0
assert e(rust_result_copy_bytes) == 112*(1+3*e(probes))

tempname results rhs public_rhs memory prep graph cap compressed
matrix `results' = e(results)
matrix `rhs' = e(rust_rhs_receipts)
matrix `public_rhs' = e(solver_rhs_diagnostics)
matrix `memory' = e(rust_memory_receipt)
matrix `prep' = e(rust_preparation_receipt)
matrix `graph' = e(rust_graph_receipt)
matrix `cap' = e(rust_request_capability_receipt)
matrix `compressed' = e(rust_compressed_receipt)
assert rowsof(`results') == 4 & colsof(`results') == 4
assert rowsof(`rhs') == 1+3*e(probes) & colsof(`rhs') == 8
assert rowsof(`public_rhs') == rowsof(`rhs') & colsof(`public_rhs') == 6
assert colsof(`memory') == 12
assert `memory'[1,3] == e(rust_result_copy_bytes)
assert `memory'[1,4] == 0
assert `memory'[1,11] == e(rust_plan_solve_peak_bytes)
assert `memory'[1,12] == max(`memory'[1,5],`memory'[1,11])
assert `memory'[1,12] <= `memory'[1,1]
assert colsof(`prep') == 13
assert colsof(`graph') == 18
assert colsof(`cap') == 23
assert colsof(`compressed') == 11
assert `compressed'[1,2] == 1
assert `compressed'[1,10] == 2
assert `compressed'[1,11] == 1

forvalues row = 1/3 {
    assert abs(`results'[`row',4]-`results'[`row',1]-`results'[`row',2]- ///
        2*`results'[`row',3]) <= 1e-10
}
forvalues column = 1/4 {
    assert abs(`results'[1,`column']-`results'[2,`column']- ///
        `results'[3,`column']) <= 1e-10
}
quietly count if e(sample)
assert r(N) == e(N_retained)

capture confirm matrix e(rust_generic_receipt)
assert _rc != 0
capture confirm matrix e(rust_control_rank_receipt)
assert _rc != 0
capture assert e(rust_generic_diagnostic_flags) < .
assert _rc != 0
capture assert e(rust_control_schur_rcond) < .
assert _rc != 0

quietly fevc_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_stream'
assert `"`c(rngstate)'"' == `"`caller_state'"'
local sortedby_after : sortedby
assert `"`sortedby_after'"' == `"`caller_sortedby'"'
// Keep the validated internal result and e(sample) active.  Because its
// disposable marker was created after caller_signature, this comparison
// covers every caller data variable and detects any raw-data mutation.
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'

// The native V3 planner also accepts an explicit algorithm(auto) request.
// Force the retained identified dimension above exact_limit() so this case
// exercises requested-auto/selected-JLA without opening the public router yet.
tempvar auto_algorithm_touse
generate byte `auto_algorithm_touse' = 1
capture noisily _fevc_rust_generic_planned outcome worker firm deletion_id ///
    frequency target_weight `auto_algorithm_touse' `nscope' `ncomplete'     ///
    `nstayers' `nstayerrows' 7 2 81227 1e-12 10000 1 auto auto             ///
    1 1 1 1 1 1 1 0 `core_flags' `support_flags' "nodisplay"             ///
    match joint 2 1e-10 1e-10 5000 50000000 "" 1 1                       ///
    "fevc outcome [fw=frequency], backend(rust) algorithm(auto) engine(auto)" ///
    auto auto 0 0
assert _rc == 0
assert `"`e(algorithm)'"' == "jla"
assert `"`e(engine_requested)'"' == "auto"
assert `"`e(engine_selected)'"' == "compressed"
assert `"`e(result_family)'"' == "compressed"
assert `"`e(preconditioner_requested)'"' == "auto"
assert `"`e(preconditioner_selected)'"' == "EXACT"
assert e(rust_requested_algorithm_code) == 0
assert e(rust_selected_algorithm_code) == 2
assert e(rust_plan_algorithm_requested) == 0
assert e(rust_plan_algorithm_selected) == 2
assert e(rust_plan_engine_requested) == 0
assert e(rust_plan_engine_selected) == 1
assert e(rust_plan_resolved) == 1
assert e(rust_plan_frozen) == 1
assert e(rust_plan_applicability) == 2
assert e(rust_requested_route) == 0
assert e(rust_selected_route) == 1
assert e(rust_plan_route_requested) == 0
assert e(rust_plan_route_selected) == 1
assert e(rust_counter_plan_complete) == 1
assert e(rust_pre_rng_hi) == 0 & e(rust_pre_rng_lo) == 0
assert e(rust_cap_schema) == 3
assert e(rust_cap_profile_code) == 4
assert e(rust_cap_engine_deferred) == 1
tempname auto_algorithm_cap
matrix `auto_algorithm_cap' = e(rust_request_capability_receipt)
assert `auto_algorithm_cap'[1,7] == 0
assert e(rust_full_fit_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_max_complete_residual) <= e(residual_acceptance_tolerance)
quietly fevc_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_stream'
assert `"`c(rngstate)'"' == `"`caller_state'"'
local auto_algorithm_sortedby : sortedby
assert `"`auto_algorithm_sortedby'"' == `"`caller_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'

// The public engine-auto boundary must preserve the compressed result family.
// On this small F-1=3 quotient the registered pre-RNG automatic solver rule
// selects the exact/direct route without changing the JLA estimator family.
quietly fevc outcome [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(auto) preconditioner(auto)   ///
    batch(auto) probes(7) seed(81227) tolerance(1e-12) memory_gib(1) ///
    targetweight(target_weight) nodisplay
assert `"`e(engine_requested)'"' == "auto"
assert `"`e(engine_selected)'"' == "compressed"
assert `"`e(result_family)'"' == "compressed"
assert `"`e(preconditioner_requested)'"' == "auto"
assert `"`e(preconditioner_selected)'"' == "EXACT"
assert `"`e(fallback_status)'"' == "NOT_NEEDED"
assert e(rust_requested_route) == 0
assert e(rust_selected_route) == 1
assert e(rust_solver_fallback) == 0
assert e(rust_solver_fallback_error) == 0
assert e(rust_plan_route_requested) == 0
assert e(rust_plan_route_selected) == 1
assert e(rust_full_fit_route) == 1
assert e(rust_rhs_receipt_schema) == 1
assert e(rust_rhs_v2_copy_bytes) == 0
assert e(rust_counter_plan_complete) == 1
assert e(rust_pre_rng_hi) == 0 & e(rust_pre_rng_lo) == 0
assert e(rust_full_fit_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_max_complete_residual) <= e(residual_acceptance_tolerance)
assert e(target_identity_residual) == e(rust_actual_accounting_residual)
assert e(targetweight_option_supplied) == 1

tempname auto_results auto_rhs
matrix `auto_results' = e(results)
matrix `auto_rhs' = e(rust_rhs_receipts)
assert colsof(`auto_rhs') == 8
forvalues row = 1/`=rowsof(`auto_rhs')' {
    assert `auto_rhs'[`row',4] == 1
}
quietly count if e(sample)
assert r(N) == e(N_retained)
quietly fevc_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_stream'
assert `"`c(rngstate)'"' == `"`caller_state'"'
local auto_sortedby_after : sortedby
assert `"`auto_sortedby_after'"' == `"`caller_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'

// Automatic diagonal selection requires the firm quotient to exceed the
// registered direct-solver threshold of 500 while remaining below the CMG
// threshold.  K(2,502) is connected after deleting any one match, so this
// fixture tests routing without weakening the match-deletion graph contract.
preserve
clear
set obs 2008
generate long diagonal_row0 = _n-1
generate long diagonal_edge = floor(diagonal_row0/2)
generate long worker = mod(diagonal_edge,2)+1
generate long firm = floor(diagonal_edge/2)+1
generate long deletion_id = diagonal_edge+1
generate byte replicate = mod(diagonal_row0,2)
generate double outcome = .2*(worker-1)-.1*(firm-1)+.04*replicate+ ///
    .005*mod(firm,11)
generate byte frequency = 1
generate double target_weight = 1

local diagonal_caller_rng `"`c(rng)'"'
local diagonal_caller_stream = c(rngstream)
local diagonal_caller_state `"`c(rngstate)'"'
local diagonal_caller_sortedby : sortedby
quietly _datasignature
local diagonal_caller_signature `"`r(datasignature)'"'

quietly fevc outcome [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(auto) preconditioner(auto)   ///
    batch(auto) probes(7) seed(81227) tolerance(1e-12) memory_gib(1) ///
    targetweight(target_weight) nodisplay
assert `"`e(engine_requested)'"' == "auto"
assert `"`e(engine_selected)'"' == "compressed"
assert `"`e(result_family)'"' == "compressed"
assert `"`e(preconditioner_requested)'"' == "auto"
assert `"`e(preconditioner_selected)'"' == "DIAGONAL"
assert `"`e(fallback_status)'"' == "NOT_NEEDED"
assert e(N_stored) == 2008
assert e(N_physical) == 2008
assert e(worker_levels) == 2
assert e(firm_levels) == 502
assert e(coefficient_cells) == 1004
assert e(deletion_units) == 1004
assert e(target_strata) == 1004
assert e(target_weight_sum) == 2008
assert e(rust_requested_route) == 0
assert e(rust_selected_route) == 2
assert e(rust_solver_fallback) == 0
assert e(rust_solver_fallback_error) == 0
assert e(rust_plan_route_requested) == 0
assert e(rust_plan_route_selected) == 2
assert e(rust_full_fit_route) == 2
assert e(rust_plan_full_dimension) == 501
assert e(rust_solver_dimension) == 501
assert e(rust_rhs_receipt_schema) == 1
assert e(rust_rhs_v2_copy_bytes) == 0
assert e(rust_counter_plan_complete) == 1
assert e(rust_pre_rng_hi) == 0 & e(rust_pre_rng_lo) == 0
assert e(rust_full_fit_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_max_complete_residual) <= e(residual_acceptance_tolerance)
assert e(target_identity_residual) == e(rust_actual_accounting_residual)
assert e(targetweight_option_supplied) == 1

tempname diagonal_results diagonal_rhs
matrix `diagonal_results' = e(results)
matrix `diagonal_rhs' = e(rust_rhs_receipts)
assert rowsof(`diagonal_results') == 4 & colsof(`diagonal_results') == 4
assert colsof(`diagonal_rhs') == 8
forvalues row = 1/`=rowsof(`diagonal_rhs')' {
    assert `diagonal_rhs'[`row',4] == 2
}
forvalues row = 1/3 {
    assert abs(`diagonal_results'[`row',4]-                       ///
        `diagonal_results'[`row',1]-`diagonal_results'[`row',2]- ///
        2*`diagonal_results'[`row',3]) <= 1e-10
}
forvalues column = 1/4 {
    assert abs(`diagonal_results'[1,`column']-                    ///
        `diagonal_results'[2,`column']-                           ///
        `diagonal_results'[3,`column']) <= 1e-10
}
quietly count if e(sample)
assert r(N) == e(N_retained)
quietly fevc_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`diagonal_caller_rng'"'
assert c(rngstream) == `diagonal_caller_stream'
assert `"`c(rngstate)'"' == `"`diagonal_caller_state'"'
local diagonal_sortedby_after : sortedby
assert `"`diagonal_sortedby_after'"' == `"`diagonal_caller_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`diagonal_caller_signature'"'
restore

assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_stream'
assert `"`c(rngstate)'"' == `"`caller_state'"'
local diagonal_restore_sortedby : sortedby
assert `"`diagonal_restore_sortedby'"' == `"`caller_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'

// Forced CMG must remain fail-closed, select CMG before Counter addressing,
// and post the same compressed scientific family without generic-only fields.
quietly fevc outcome [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(auto) preconditioner(cmg)    ///
    batch(auto) probes(7) seed(81227) tolerance(1e-12) memory_gib(1) ///
    targetweight(target_weight) nodisplay
assert `"`e(engine_requested)'"' == "auto"
assert `"`e(engine_selected)'"' == "compressed"
assert `"`e(result_family)'"' == "compressed"
assert `"`e(preconditioner_requested)'"' == "cmg"
assert `"`e(preconditioner_selected)'"' == "CMG"
assert `"`e(fallback_status)'"' == "NOT_NEEDED"
assert e(rust_requested_route) == 3
assert e(rust_selected_route) == 3
assert e(rust_solver_fallback) == 0
assert e(rust_solver_fallback_error) == 0
assert e(rust_plan_route_requested) == 3
assert e(rust_plan_route_selected) == 3
assert e(rust_full_fit_route) == 3
assert e(rust_rhs_receipt_schema) == 1
assert e(rust_rhs_v2_copy_bytes) == 0
assert e(rust_counter_plan_complete) == 1
assert e(rust_pre_rng_hi) == 0 & e(rust_pre_rng_lo) == 0
assert e(rust_full_fit_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_max_complete_residual) <= e(residual_acceptance_tolerance)
assert e(target_identity_residual) == e(rust_actual_accounting_residual)

capture confirm matrix e(rust_generic_receipt)
assert _rc != 0
capture confirm matrix e(rust_control_rank_receipt)
assert _rc != 0
capture assert e(rust_generic_diagnostic_flags) < .
assert _rc != 0

tempname cmg_results cmg_rhs
matrix `cmg_results' = e(results)
matrix `cmg_rhs' = e(rust_rhs_receipts)
assert colsof(`cmg_rhs') == 8
forvalues row = 1/`=rowsof(`cmg_rhs')' {
    assert `cmg_rhs'[`row',4] == 3
}
forvalues row = 1/4 {
    forvalues column = 1/4 {
        assert abs(`cmg_results'[`row',`column']-                  ///
            `auto_results'[`row',`column']) <=                    ///
            1e-8*max(1,abs(`auto_results'[`row',`column']))
    }
}
quietly count if e(sample)
assert r(N) == e(N_retained)
quietly fevc_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_stream'
assert `"`c(rngstate)'"' == `"`caller_state'"'
local cmg_sortedby_after : sortedby
assert `"`cmg_sortedby_after'"' == `"`caller_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'

di as result "FEVC RUST COMPRESSED PUBLIC ROUTES PASS"

// Planned algorithm(auto) selecting exact: direct V4/V7 Stata bridge certificate.
preserve
clear
set obs 96
generate long xrow = _n-1
generate long xcell = floor(xrow/2)
generate long worker = floor(xcell/4)+1
generate long firm = mod(xcell,4)+1
generate long deletion_id = xcell+1
generate byte replicate = mod(xrow,2)
generate double outcome = .45*(worker-1)-.35*(firm-1)+.08*replicate+ ///
    .025*mod(xcell,3)
generate long frequency = 1+mod(5*xrow+2,3)
generate double target_weight = .75+(xrow+1)/192

local exact_rng `"`c(rng)'"'
local exact_stream = c(rngstream)
local exact_state `"`c(rngstate)'"'
local exact_sortedby : sortedby
quietly _datasignature
local exact_signature `"`r(datasignature)'"'

quietly fevc_rust clear
quietly fevc_rust probe
local xcore = r(core_ready_flags)
local xsupport = r(support_flags)
quietly fevc_rust requestcapability, algorithm(auto) deletion(match) ///
    nuisance(joint) route(auto) rngcontract(counter_v1) controls(0)        ///
    frequencyused(1) engine(auto) batchmode(auto) leveragebatchmode(auto)  ///
    targetbatchmode(auto) stayers(movers) targetweightmode(explicit)       ///
    deletionsource(matchid) probeordersupplied(0) wallsecondssupplied(0)   ///
    fallback(1) wallseconds(0) physicallimit(50000000)
assert r(supported) == 1
assert r(request_schema) == 3
assert r(profile_code) == 4
assert r(algorithm_code) == 0
assert r(engine_code) == 0
assert r(algorithm_resolution_deferred) == 1
assert r(engine_resolution_deferred) == 1
assert r(route_resolution_deferred) == 1
assert r(leverage_batch_deferred) == 1
assert r(target_batch_resolution_deferred) == 1
tempname xcapctx
matrix `xcapctx' = (r(struct_size),r(abi_version),r(request_schema), ///
    r(supported),r(reason_code),r(profile_code),r(algorithm_code), ///
    r(deletion_mode_code),r(nuisance_mode_code),r(solver_route_code), ///
    r(rng_contract_code),r(controls_count),r(frequency_use_code),  ///
    r(engine_code),r(batch_mode_code),r(stayers_mode_code),        ///
    r(target_weight_mode_code),r(deletion_source_code),            ///
    r(probeorder_supplied),r(wallseconds_supplied),r(physical_limit), ///
    r(request_signature_hi),r(request_signature_lo),               ///
    r(leverage_batch_mode_code),r(target_batch_mode_code),         ///
    r(automatic_fallback_allowed),r(algorithm_resolution_deferred), ///
    r(engine_resolution_deferred),r(route_resolution_deferred),    ///
    r(leverage_batch_deferred),r(target_batch_resolution_deferred), ///
    r(wall_advisory_only),r(wallseconds))
local xsighi = r(request_signature_hi)
local xsiglo = r(request_signature_lo)

tempvar xkeep
quietly fevc_rust prepare worker firm deletion_id outcome frequency ///
    target_weight, cleanup generate(`xkeep') memorygib(1) deletion(match)
local xhandle = r(handle)
local xworkers = r(workers)
local xfirms = r(firms)
local xmem = r(memory_limit_bytes)
local xcopy = r(caller_copy_bytes)
local xprep = r(preparation_peak_forecast_bytes)
local xresident = r(prepared_resident_bytes)
tempname xprepctx xgraphctx
matrix `xprepctx' = (r(input_rows),r(retained_rows),r(workers),r(firms), ///
    r(cells),r(deletion_units),r(target_strata),r(target_weight_sum), ///
    r(controls_count),r(memory_limit_bytes),r(caller_copy_bytes),    ///
    r(preparation_peak_forecast_bytes),r(prepared_resident_bytes))
matrix `xgraphctx' = (r(graph_input_rows),r(graph_retained_rows),     ///
    r(graph_input_physical_mass),r(graph_retained_physical_mass),    ///
    r(graph_initial_components),r(graph_maximum_components),         ///
    r(graph_initial_component_rows),r(graph_mover_input_rows),       ///
    r(graph_initial_deletion_edges),r(graph_retained_deletion_edges), ///
    r(graph_degree_workers_removed),r(graph_artic_workers_removed),  ///
    r(graph_bridge_units_removed),r(graph_bridge_rows_removed),      ///
    r(graph_degree_iterations),r(graph_articulation_iterations),     ///
    r(graph_bridge_iterations),r(graph_fixed_point_iterations))
assert `xworkers'+`xfirms'-1 == 15

quietly fevc_rust solve `xhandle', algorithm(auto) deletion(match) ///
    nuisance(joint) route(auto) seed(81227) probes(7) leveragebatch(0)   ///
    targetbatch(0) tolerance(1e-12) maxiter(10000) exactlimit(500)      ///
    blocksizelimit(5000) ranktolerance(1e-10) blocktolerance(1e-10)    ///
    engine(auto) batchmode(auto) leveragebatchmode(auto)                ///
    targetbatchmode(auto) stayers(movers) targetweightmode(explicit)    ///
    deletionsource(matchid) probeordersupplied(0) wallsecondssupplied(0) ///
    physicallimit(50000000) capabilityschema(3) capabilityprofile(4)    ///
    frequencyused(1) signaturehi(`xsighi') signaturelo(`xsiglo')       ///
    fallback(1) wallseconds(0)
quietly fevc_rust result `xhandle'
quietly _fevc_rust_reconcile_exact_v7 0 0 1 1 `xworkers' `xfirms' 0 ///
    1e-10 1e-10 1e-12 500 `xmem' `xcopy' `xprep' `xresident'        ///
    `xsighi' `xsiglo' 50000000 0 0 1 2 1 1 15
local exact_reconcile_ok = r(ok)
local exact_reconcile_detail `"`r(detail)'"'
if `exact_reconcile_ok' != 1 {
    di as error "EXACT_V7_RECONCILE_FAIL: `exact_reconcile_detail'"
    return list
}
assert `exact_reconcile_ok' == 1
assert `"`r(result_family)'"' == "exact"
assert `"`r(execution_plan_schema)'"' == "VCKSS-EXECUTION-PLAN-V1"
assert r(algorithm_requested) == 0
assert r(algorithm_selected) == 1
assert r(engine_requested) == 0
assert r(engine_selected) == 3
assert r(plan_algorithm_requested) == 0
assert r(plan_algorithm_selected) == 1
assert r(plan_algorithm_reason) == 3
assert r(plan_engine_requested) == 0
assert r(plan_engine_selected) == 3
assert r(plan_engine_reason) == 1
assert r(plan_compressed_eligibility) == 0
assert r(plan_complexity) == 15
assert r(plan_exact_limit) == 500
assert r(plan_applicability) == 1
assert r(plan_resolved) == 1
assert r(plan_frozen) == 1
assert r(plan_route_requested) == 4
assert r(plan_route_selected) == 4
assert r(counter_complete) == 1
assert r(pre_rng_hi) == 0 & r(pre_rng_lo) == 0
assert r(plan_memory_bytes) == r(solve_peak_bytes)
assert r(full_fit_complete_residual) <= r(residual_tolerance)
assert r(actual_accounting_residual) >= 0
tempname xresult
matrix `xresult' = r(result)
assert rowsof(`xresult') == 4 & colsof(`xresult') == 4
forvalues row=1/3 {
    assert abs(`xresult'[`row',4]-`xresult'[`row',1]-                ///
        `xresult'[`row',2]-2*`xresult'[`row',3]) <= 1e-10
}
forvalues col=1/4 {
    assert `xresult'[4,`col'] == 0
    assert abs(`xresult'[1,`col']-`xresult'[2,`col']-                ///
        `xresult'[3,`col']) <= 1e-10
}
quietly _fevc_rust_post_exact_v7 `xhandle' outcome frequency ///
    target_weight `xkeep' 96 96 0 0 7 8 81227 1e-12 10000 1 auto auto ///
    1 1 counter_v1 1 1 1 1 1 0 `xcore' `xsupport' "nodisplay" match ///
    joint 1e-10 1e-10 500 50000000 auto auto 1                     ///
    "fevc outcome [fw=frequency], backend(rust) algorithm(auto) engine(auto)" ///
    0 0 `xprepctx' `xgraphctx' `xcapctx' movers 15
assert `"`e(algorithm_requested)'"' == "auto"
assert `"`e(algorithm)'"' == "exact"
assert `"`e(engine_requested)'"' == "auto"
assert `"`e(engine_selected)'"' == "NOT_APPLICABLE"
assert `"`e(result_family)'"' == "exact"
assert `"`e(execution_plan_schema)'"' == "VCKSS-EXECUTION-PLAN-V1"
assert e(rust_requested_algorithm_code) == 0
assert e(rust_selected_algorithm_code) == 1
assert e(rust_requested_engine_code) == 0
assert e(rust_selected_engine_code) == 3
assert e(rust_plan_applicability) == 1
assert e(rust_plan_resolved) == 1 & e(rust_plan_frozen) == 1
assert e(rust_plan_algorithm_requested) == 0
assert e(rust_plan_algorithm_selected) == 1
assert e(rust_plan_algorithm_reason) == 3
assert e(rust_plan_engine_requested) == 0
assert e(rust_plan_engine_selected) == 3
assert e(rust_plan_engine_reason) == 1
assert e(rust_plan_compressed_eligibility) == 0
assert e(rust_plan_complexity) == 15
assert e(rust_plan_exact_limit) == 500
assert e(rust_plan_route_requested) == 4
assert e(rust_plan_route_selected) == 4
assert e(rust_counter_plan_complete) == 1
assert e(rust_pre_rng_hi) == 0 & e(rust_pre_rng_lo) == 0
assert e(probes) == 0 & e(numerical_mcse_available) == 0
assert mreldif(e(results),`xresult') == 0
quietly count if e(sample)
assert r(N) == e(N_retained)
quietly fevc_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`exact_rng'"'
assert c(rngstream) == `exact_stream'
assert `"`c(rngstate)'"' == `"`exact_state'"'
local exact_sortedby_after : sortedby
assert `"`exact_sortedby_after'"' == `"`exact_sortedby'"'
capture drop `xkeep'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`exact_signature'"'

// The public boundary admits only explicit Rust/counter consent with the
// frozen algorithm(auto), engine(auto), preconditioner(auto), batch(auto)
// tuple.  The native V4/V7 plan selects the exact result family before RNG.
quietly fevc outcome [fw=frequency], worker(worker) firm(firm)     ///
    deletion(match) deletionid(deletion_id)                         ///
    targetweight(target_weight) backend(rust) rng(counter_v1)       ///
    algorithm(auto) engine(auto) tolerance(1e-12) maxiter(10000)    ///
    memory_gib(1) exact_limit(500) physical_limit(2) nodisplay
assert `"`e(cmd)'"' == "fevc"
assert `"`e(version)'"' == "0.5.0-alpha.1"
assert `"`e(backend_requested)'"' == "rust"
assert `"`e(backend_selected)'"' == "rust"
assert `"`e(rng_requested)'"' == "counter_v1"
assert `"`e(rng_selected)'"' == "NOT_APPLICABLE"
assert `"`e(algorithm_requested)'"' == "auto"
assert `"`e(algorithm)'"' == "exact"
assert `"`e(engine_requested)'"' == "auto"
assert `"`e(engine_selected)'"' == "NOT_APPLICABLE"
assert `"`e(preconditioner_requested)'"' == "auto"
assert `"`e(preconditioner_selected)'"' == "NOT_APPLICABLE"
assert `"`e(batch_requested)'"' == "auto"
assert `"`e(physical_limit_status)'"' == "NOT_APPLICABLE_TO_EXACT"
assert `"`e(result_family)'"' == "exact"
assert `"`e(execution_plan_schema)'"' == "VCKSS-EXECUTION-PLAN-V1"
assert `"`e(route_api)'"' == "VCKSS-NATIVE-EXACT-PLANNED-V4-V7"
assert `"`e(rust_phase_profile_schema)'"' == "VCKSS-NATIVE-PHASE-PERF-V1"
assert `"`e(rust_phase_profile_units)'"' == "seconds"
assert rowsof(e(rust_phase_profile)) == 1
assert colsof(e(rust_phase_profile)) == 8
forvalues phase = 1/8 {
    assert e(rust_phase_profile)[1,`phase'] >= 0
}
assert e(rust_phase_profile)[1,8] >= e(rust_phase_profile)[1,7]
assert e(backend_option_supplied) == 1
assert e(rng_option_supplied) == 1
assert e(algorithm_option_supplied) == 1
assert e(engine_option_supplied) == 1
assert e(preconditioner_option_supplied) == 0
assert e(batch_option_supplied) == 0
assert e(physical_limit) == 2 & e(physical_limit_applied) == 0
assert e(rust_requested_algorithm_code) == 0
assert e(rust_selected_algorithm_code) == 1
assert e(rust_requested_engine_code) == 0
assert e(rust_selected_engine_code) == 3
assert e(rust_plan_applicability) == 1
assert e(rust_plan_resolved) == 1 & e(rust_plan_frozen) == 1
assert e(rust_plan_algorithm_requested) == 0
assert e(rust_plan_algorithm_selected) == 1
assert e(rust_plan_algorithm_reason) == 3
assert e(rust_plan_engine_requested) == 0
assert e(rust_plan_engine_selected) == 3
assert e(rust_plan_engine_reason) == 1
assert e(rust_plan_compressed_eligibility) == 0
assert e(rust_plan_complexity) == 15
assert e(rust_plan_exact_limit) == 500
assert e(rust_plan_route_requested) == 4
assert e(rust_plan_route_selected) == 4
assert e(rust_plan_rhs) == 0
assert e(rust_counter_plan_complete) == 1
assert e(rust_pre_rng_hi) == 0 & e(rust_pre_rng_lo) == 0
assert e(rust_cap_schema) == 3 & e(rust_cap_profile_code) == 4
assert e(rust_cap_algorithm_deferred) == 1
assert e(rust_cap_engine_deferred) == 1
assert e(rust_cap_route_deferred) == 1
assert e(rust_cap_leverage_batch_deferred) == 1
assert e(rust_cap_target_batch_deferred) == 1
assert colsof(e(rust_request_capability_receipt)) == 23
assert colsof(e(rust_preparation_receipt)) == 8
assert colsof(e(rust_graph_receipt)) == 18
assert e(probes) == 0 & e(seed) == 0 & e(batch) == 0
assert e(numerical_mcse_available) == 0
assert e(rust_full_fit_complete_residual) <=                    ///
    e(residual_acceptance_tolerance)
assert e(rust_max_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_actual_accounting_residual) <= 1e-10
assert mreldif(e(results),`xresult') == 0
forvalues row=1/3 {
    assert abs(e(results)[`row',4]-e(results)[`row',1]-          ///
        e(results)[`row',2]-2*e(results)[`row',3]) <= 1e-10
}
forvalues col=1/4 {
    assert e(results)[4,`col'] == 0
    assert abs(e(results)[1,`col']-e(results)[2,`col']-          ///
        e(results)[3,`col']) <= 1e-10
}
quietly count if e(sample)
assert r(N) == e(N_retained)
capture confirm matrix e(V)
assert _rc != 0
quietly fevc_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`exact_rng'"'
assert c(rngstream) == `exact_stream'
assert `"`c(rngstate)'"' == `"`exact_state'"'
local public_exact_sortedby : sortedby
assert `"`public_exact_sortedby'"' == `"`exact_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`exact_signature'"'
restore

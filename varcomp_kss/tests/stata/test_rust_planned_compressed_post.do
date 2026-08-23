version 18.0
clear all
set more off

args package_dir
if `"`package_dir'"' == "" {
    local package_dir = subinstr("`c(pwd)'","/tests/stata","",.)
}
adopath ++ `"`package_dir'"'
quietly run `"`package_dir'/varcomp_kss.ado"'

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

quietly varcomp_kss_rust clear
quietly varcomp_kss_rust probe
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

capture noisily _vckss_rust_generic_planned outcome worker firm deletion_id ///
    frequency target_weight `internal_touse' `nscope' `ncomplete' `nstayers' ///
    `nstayerrows' 7 2 81227 1e-12 10000 1 auto 1 1 1 1 1 1 1 0     ///
    `core_flags' `support_flags' "nodisplay" match joint 500 1e-10   ///
    1e-10 5000 50000000 "" 1 1                                      ///
    "varcomp_kss outcome [fw=frequency], backend(rust) engine(auto)" ///
    diagonal auto 0 0
assert _rc == 0

assert `"`e(cmd)'"' == "varcomp_kss"
assert `"`e(status)'"' == "KSS_POINT_ESTIMATES_ONLY"
assert `"`e(backend_selected)'"' == "rust"
assert `"`e(rng_selected)'"' == "counter_v1"
assert `"`e(algorithm)'"' == "jla"
assert `"`e(engine_requested)'"' == "auto"
assert `"`e(engine_selected)'"' == "compressed"
assert `"`e(result_family)'"' == "compressed"
assert `"`e(route_api)'"' == "VCKSS-NATIVE-COMPRESSED-PLANNED-V4-V7"
assert `"`e(execution_plan_schema)'"' == "VCKSS-EXECUTION-PLAN-V1"
assert `"`e(preconditioner_requested)'"' == "diagonal"
assert `"`e(preconditioner_selected)'"' == "diagonal"
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

quietly varcomp_kss_rust snapshot
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

// The public engine-auto boundary must preserve the compressed result family
// while the native pre-RNG plan resolves an automatic route to diagonal.
// Keep this trace tightly scoped to the unresolved public-boundary failure so
// the comprehensive qualifier exports the first raw Stata/native 498 rather
// than only the outer withheld-result display.
set tracedepth 6
set trace on
capture noisily varcomp_kss outcome [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(auto) preconditioner(auto)   ///
    batch(auto) probes(7) seed(81227) tolerance(1e-12) memory_gib(1) ///
    targetweight(target_weight) nodisplay
local public_auto_rc = _rc
set trace off
if `public_auto_rc' {
    noisily di as error "PUBLIC_COMPRESSED_AUTO_RC=`public_auto_rc'"
    capture noisily ereturn list
    capture noisily varcomp_kss_rust lasterror
    capture noisily return list
    capture noisily varcomp_kss_rust snapshot
    capture noisily return list
}
assert `public_auto_rc' == 0
assert `"`e(engine_requested)'"' == "auto"
assert `"`e(engine_selected)'"' == "compressed"
assert `"`e(result_family)'"' == "compressed"
assert `"`e(preconditioner_requested)'"' == "auto"
assert `"`e(preconditioner_selected)'"' == "diagonal"
assert `"`e(fallback_status)'"' == "ELIGIBLE_NOT_USED"
assert e(rust_requested_route) == 0
assert e(rust_selected_route) == 2
assert e(rust_solver_fallback) == 0
assert e(rust_solver_fallback_error) == 0
assert e(rust_plan_route_requested) == 0
assert e(rust_plan_route_selected) == 2
assert e(rust_full_fit_route) == 2
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
    assert `auto_rhs'[`row',4] == 2
}
quietly count if e(sample)
assert r(N) == e(N_retained)
quietly varcomp_kss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_stream'
assert `"`c(rngstate)'"' == `"`caller_state'"'
local auto_sortedby_after : sortedby
assert `"`auto_sortedby_after'"' == `"`caller_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'

// Forced CMG must remain fail-closed, select CMG before Counter addressing,
// and post the same compressed scientific family without generic-only fields.
quietly varcomp_kss outcome [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(auto) preconditioner(cmg)    ///
    batch(auto) probes(7) seed(81227) tolerance(1e-12) memory_gib(1) ///
    targetweight(target_weight) nodisplay
assert `"`e(engine_requested)'"' == "auto"
assert `"`e(engine_selected)'"' == "compressed"
assert `"`e(result_family)'"' == "compressed"
assert `"`e(preconditioner_requested)'"' == "cmg"
assert `"`e(preconditioner_selected)'"' == "cmg"
assert `"`e(fallback_status)'"' == "NOT_ELIGIBLE"
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
quietly varcomp_kss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_stream'
assert `"`c(rngstate)'"' == `"`caller_state'"'
local cmg_sortedby_after : sortedby
assert `"`cmg_sortedby_after'"' == `"`caller_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'

di as result "VARCOMP_KSS RUST COMPRESSED PUBLIC ROUTES PASS"

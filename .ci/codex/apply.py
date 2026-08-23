from pathlib import Path

path = Path("varcomp_kss/tests/stata/test_rust_public_generic.do")
text = path.read_text()
anchor = "\n// Registered deletion/nuisance/control/weight combinations are all public.\n"
if text.count(anchor) != 1:
    raise SystemExit("compressed public route insertion anchor changed")

block = r'''

// Automatic preconditioning must preserve the compressed engine choice and
// select its frozen diagonal route before Counter-V1 begins.  Advisory wall
// planning may report work but may not change results or caller state.
local compressed_preauto_rng `"`c(rng)'"'
local compressed_preauto_stream = c(rngstream)
local compressed_preauto_state `"`c(rngstate)'"'
local compressed_preauto_sortedby : sortedby
quietly _datasignature
local compressed_preauto_signature `"`r(datasignature)'"'
quietly varcomp_kss outcome [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(auto) preconditioner(auto) ///
    batch(auto) probes(4) seed(81227) tolerance(1e-12) memory_gib(1) ///
    wallseconds(60) nodisplay
tempname compressed_preauto_results compressed_preauto_memory
matrix `compressed_preauto_results' = e(results)
matrix `compressed_preauto_memory' = e(rust_memory_receipt)
assert mreldif(`compressed_preauto_results',`compressed_auto_results') <= 1e-12
assert `"`e(engine_requested)'"' == "auto"
assert `"`e(engine_selected)'"' == "compressed"
assert `"`e(result_family)'"' == "compressed"
assert `"`e(preconditioner_requested)'"' == "auto"
assert `"`e(preconditioner_selected)'"' == "diagonal"
assert `"`e(fallback_status)'"' == "ELIGIBLE_NOT_USED"
assert e(rust_requested_engine_code) == 0
assert e(rust_selected_engine_code) == 1
assert e(rust_requested_route) == 0
assert e(rust_selected_route) == 2
assert e(route_code) == 2
assert e(rust_solver_fallback) == 0
assert e(rust_solver_fallback_error) == 0
assert e(rust_rhs_receipt_schema) == 1
assert e(rust_plan_engine_selected) == 1
assert e(rust_plan_route_requested) == 0
assert e(rust_plan_route_selected) == 2
assert e(rust_plan_route_fallback) == 0
assert e(rust_plan_route_error) == 0
assert e(rust_wallseconds_supplied) == 1
assert e(rust_wallseconds_requested) == 60
assert e(rust_wallseconds_forecast) >= 0
assert e(rust_wallseconds_advisory) >= 0
assert e(rust_wallseconds_margin) >= 0
assert `compressed_preauto_memory'[1,11] == e(rust_plan_solve_peak_bytes)
assert `compressed_preauto_memory'[1,12] == max(                 ///
    `compressed_preauto_memory'[1,5],`compressed_preauto_memory'[1,11])
assert `compressed_preauto_memory'[1,12] <= `compressed_preauto_memory'[1,1]
assert e(rust_full_fit_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_max_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_actual_accounting_residual) == e(target_identity_residual)
quietly varcomp_kss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`compressed_preauto_rng'"'
assert c(rngstream) == `compressed_preauto_stream'
assert `"`c(rngstate)'"' == `"`compressed_preauto_state'"'
local compressed_preauto_sortedby_after : sortedby
assert `"`compressed_preauto_sortedby_after'"' == `"`compressed_preauto_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`compressed_preauto_signature'"'

// A forced CMG compressed request must remain on CMG: setup failure may not
// fall back, batching is frozen before RNG, and the accepted scientific result
// must agree with the diagonal route up to registered numerical tolerance.
local compressed_cmg_rng `"`c(rng)'"'
local compressed_cmg_stream = c(rngstream)
local compressed_cmg_state `"`c(rngstate)'"'
local compressed_cmg_sortedby : sortedby
quietly _datasignature
local compressed_cmg_signature `"`r(datasignature)'"'
quietly varcomp_kss outcome [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(auto) preconditioner(cmg) ///
    batch(2) probes(4) seed(81227) tolerance(1e-12) memory_gib(1) nodisplay
tempname compressed_cmg_results compressed_cmg_memory
matrix `compressed_cmg_results' = e(results)
matrix `compressed_cmg_memory' = e(rust_memory_receipt)
assert mreldif(`compressed_cmg_results',`compressed_auto_results') <= 1e-9
assert `"`e(engine_requested)'"' == "auto"
assert `"`e(engine_selected)'"' == "compressed"
assert `"`e(result_family)'"' == "compressed"
assert `"`e(preconditioner_requested)'"' == "cmg"
assert `"`e(preconditioner_selected)'"' == "cmg"
assert `"`e(fallback_status)'"' == "NOT_ELIGIBLE"
assert e(rust_requested_engine_code) == 0
assert e(rust_selected_engine_code) == 1
assert e(rust_requested_route) == 3
assert e(rust_selected_route) == 3
assert e(route_code) == 3
assert e(rust_solver_fallback) == 0
assert e(rust_solver_fallback_error) == 0
assert e(rust_batch_mode_code) == 1
assert e(rust_leverage_batch_mode_code) == 1
assert e(rust_target_batch_mode_code) == 1
assert e(leverage_batch) == 2
assert e(target_batch) == 2
assert e(batch) == 2
assert e(rust_plan_engine_selected) == 1
assert e(rust_plan_route_requested) == 3
assert e(rust_plan_route_selected) == 3
assert e(rust_plan_route_fallback) == 0
assert e(rust_plan_route_error) == 0
assert `compressed_cmg_memory'[1,11] == e(rust_plan_solve_peak_bytes)
assert `compressed_cmg_memory'[1,12] == max(                     ///
    `compressed_cmg_memory'[1,5],`compressed_cmg_memory'[1,11])
assert `compressed_cmg_memory'[1,12] <= `compressed_cmg_memory'[1,1]
assert e(rust_full_fit_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_max_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_actual_accounting_residual) == e(target_identity_residual)
quietly varcomp_kss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`compressed_cmg_rng'"'
assert c(rngstream) == `compressed_cmg_stream'
assert `"`c(rngstate)'"' == `"`compressed_cmg_state'"'
local compressed_cmg_sortedby_after : sortedby
assert `"`compressed_cmg_sortedby_after'"' == `"`compressed_cmg_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`compressed_cmg_signature'"'

// With no controls, fixed-offset and joint nuisance dimensions coincide, but
// the native capability, plan, and public result must still retain the
// requested fixed-offset and explicit stored-row target-weight conventions.
local compressed_fixed_rng `"`c(rng)'"'
local compressed_fixed_stream = c(rngstream)
local compressed_fixed_state `"`c(rngstate)'"'
local compressed_fixed_sortedby : sortedby
quietly _datasignature
local compressed_fixed_signature `"`r(datasignature)'"'
quietly varcomp_kss outcome [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(fixedoffset) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(auto) preconditioner(diagonal) ///
    batch(auto) probes(5) seed(81227) tolerance(1e-12) memory_gib(1) ///
    targetweight(target_weight) nodisplay
tempname compressed_fixed_results compressed_fixed_capability
matrix `compressed_fixed_results' = e(results)
matrix `compressed_fixed_capability' = e(rust_request_capability_receipt)
assert `"`e(engine_selected)'"' == "compressed"
assert `"`e(result_family)'"' == "compressed"
assert `"`e(nuisance)'"' == "fixedoffset"
assert `"`e(preconditioner_selected)'"' == "diagonal"
assert e(controls_count) == 0
assert e(parameters) == e(worker_levels)+e(firm_levels)-1
assert e(full_parameters) == e(parameters)
assert e(correction_parameters) == e(parameters)
assert e(N_physical) == 192
assert e(target_weight_sum) == 144
assert e(rust_target_weight_mode_code) == 1
assert e(targetweight_option_supplied) == 1
assert e(rust_requested_engine_code) == 0
assert e(rust_selected_engine_code) == 1
assert e(rust_requested_route) == 2
assert e(rust_selected_route) == 2
assert `compressed_fixed_capability'[1,9] == 2
assert `compressed_fixed_capability'[1,17] == 1
assert e(rust_full_fit_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_max_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_actual_accounting_residual) == e(target_identity_residual)
forvalues row = 1/3 {
    assert abs(`compressed_fixed_results'[`row',4]-               ///
        `compressed_fixed_results'[`row',1]-                     ///
        `compressed_fixed_results'[`row',2]-                     ///
        2*`compressed_fixed_results'[`row',3]) <= 1e-10
}
forvalues column = 1/4 {
    assert abs(`compressed_fixed_results'[1,`column']-           ///
        `compressed_fixed_results'[2,`column']-                 ///
        `compressed_fixed_results'[3,`column']) <= 1e-10
}
quietly count if e(sample)
assert r(N) == e(N_retained)
quietly varcomp_kss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`compressed_fixed_rng'"'
assert c(rngstream) == `compressed_fixed_stream'
assert `"`c(rngstate)'"' == `"`compressed_fixed_state'"'
local compressed_fixed_sortedby_after : sortedby
assert `"`compressed_fixed_sortedby_after'"' == `"`compressed_fixed_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`compressed_fixed_signature'"'
'''

path.write_text(text.replace(anchor, block + anchor))

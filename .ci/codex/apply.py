from pathlib import Path

ado_path = Path("varcomp_kss/varcomp_kss.ado")
test_path = Path("varcomp_kss/tests/stata/test_rust_public_generic.do")

ado = ado_path.read_text()
old_gate = '''        local rust_auto_engine_generic =                        ///
            "`engine_requested'"=="auto" &                         ///
            ("`deletion'"=="observation" |                        ///
                strtrim(`"`controls'"')!="")
        local rust_planned_generic_supported =                 ///
            `algorithm_supplied' & "`algorithm'" == "jla" &       ///
            `engine_supplied' &                                   ///
            ("`engine_requested'"=="generic" |                   ///
                `rust_auto_engine_generic') &                      ///
'''
new_gate = '''        local rust_auto_engine_generic =                        ///
            "`engine_requested'"=="auto" &                         ///
            ("`deletion'"=="observation" |                        ///
                strtrim(`"`controls'"')!="")
        local rust_auto_engine_compressed =                     ///
            "`engine_requested'"=="auto" &                         ///
            "`deletion'"=="match" &                              ///
            strtrim(`"`controls'"')==""
        local rust_planned_generic_supported =                 ///
            `algorithm_supplied' & "`algorithm'" == "jla" &       ///
            `engine_supplied' &                                   ///
            ("`engine_requested'"=="generic" |                   ///
                `rust_auto_engine_generic' |                       ///
                `rust_auto_engine_compressed') &                   ///
'''
if ado.count(old_gate) != 1:
    raise SystemExit("public engine-auto gate anchor changed")
ado_path.write_text(ado.replace(old_gate, new_gate))

test = test_path.read_text()
old_test = '''

// No-control match tuples may select compressed and remain withheld until the
// public planned reconciler supports both compressed and generic result families.
capture quietly varcomp_kss outcome [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(auto) preconditioner(diagonal) ///
    batch(auto) probes(4) seed(81227) memory_gib(1) nodisplay
assert _rc == 498
assert `"`e(withholding_status)'"' == "RUST_OPTION_UNSUPPORTED"
assert `"`e(backend_selected)'"' == "" & `"`e(rng_selected)'"' == ""
'''
new_test = '''

// No-control match engine(auto) is resolved before Counter-V1 begins and may
// select the compressed result family.  The public poster must retain that
// family's own V1/V7 receipts rather than manufacture generic diagnostics.
local compressed_auto_rng `"`c(rng)'"'
local compressed_auto_stream = c(rngstream)
local compressed_auto_state `"`c(rngstate)'"'
local compressed_auto_sortedby : sortedby
quietly _datasignature
local compressed_auto_signature `"`r(datasignature)'"'
quietly varcomp_kss outcome [fw=frequency], worker(worker) firm(firm) ///
    deletion(match) deletionid(deletion_id) nuisance(joint) algorithm(jla) ///
    backend(rust) rng(counter_v1) engine(auto) preconditioner(diagonal) ///
    batch(auto) probes(4) seed(81227) tolerance(1e-12) memory_gib(1) nodisplay
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
assert e(N_requested) == 96
assert e(N_complete) == 96
assert e(N_retained) == 96
assert e(N_physical) == 192
assert e(worker_levels) == 12
assert e(firm_levels) == 4
assert e(parameters) == e(worker_levels)+e(firm_levels)-1
assert e(full_parameters) == e(parameters)
assert e(correction_parameters) == e(parameters)
assert e(controls_count) == 0
assert e(coefficient_cells) == 48
assert e(deletion_units) == 48
assert e(target_strata) == 48
assert e(target_weight_sum) == e(N_physical)
assert e(probes) == 4
assert e(seed) == 81227
assert e(rust_requested_algorithm_code) == 2
assert e(rust_selected_algorithm_code) == 2
assert e(rust_requested_engine_code) == 0
assert e(rust_selected_engine_code) == 1
assert e(rust_requested_route) == 2
assert e(rust_selected_route) == 2
assert e(rust_solver_fallback) == 0
assert e(rust_solver_fallback_error) == 0
assert e(rust_solver_dimension) == e(firm_levels)-1
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
assert e(rust_plan_algorithm_requested) == 2
assert e(rust_plan_algorithm_selected) == 2
assert e(rust_plan_engine_requested) == 0
assert e(rust_plan_engine_selected) == 1
assert e(rust_plan_route_requested) == 2
assert e(rust_plan_route_selected) == 2
assert e(rust_plan_route_fallback) == 0
assert e(rust_plan_route_error) == 0
assert e(rust_counter_plan_complete) == 1
assert e(rust_pre_rng_hi) == 0
assert e(rust_pre_rng_lo) == 0
assert e(rust_plan_rhs) == 1+3*e(probes)
assert e(rust_plan_full_dimension) == e(firm_levels)-1
assert e(rust_leverage_probes_accepted) == e(probes)
assert e(rust_target_probes_accepted) == e(probes)
assert e(rust_leverage_rhs_count) == e(probes)
assert e(rust_target_rhs_count) == 2*e(probes)
assert e(rust_batch_mode_code) == 0
assert e(rust_leverage_batch_mode_code) == 0
assert e(rust_target_batch_mode_code) == 0
assert inrange(e(leverage_batch),1,e(probes))
assert inrange(e(target_batch),1,e(probes))
assert e(rust_stayers_mode_code) == 1
assert e(rust_target_weight_mode_code) == 0
assert e(rust_deletion_source_code) == 2
assert e(rust_probeorder_supplied) == 0
assert e(rust_wallseconds_supplied) == 0
assert e(rust_wall_request_applicable) == 0
assert e(rust_wallseconds_requested) == 0
assert e(rust_frequency_use_code) == 1
assert e(rust_solve_physical_limit) == e(physical_limit)
assert e(rust_full_fit_complete_residual) <= e(residual_acceptance_tolerance)
assert e(rust_max_complete_residual) <= e(residual_acceptance_tolerance)
assert e(target_identity_residual) == e(rust_actual_accounting_residual)
assert e(rust_rhs_v2_copy_bytes) == 0
assert e(rust_result_copy_bytes) == 112*(1+3*e(probes))
assert e(algorithm_option_supplied) == 1
assert e(engine_option_supplied) == 1
assert e(preconditioner_option_supplied) == 1
assert e(batch_option_supplied) == 1
assert e(targetweight_option_supplied) == 0

tempname compressed_auto_results compressed_auto_rhs compressed_auto_public_rhs
    tempname compressed_auto_memory compressed_auto_prep compressed_auto_graph
    tempname compressed_auto_cap compressed_auto_receipt
matrix `compressed_auto_results' = e(results)
matrix `compressed_auto_rhs' = e(rust_rhs_receipts)
matrix `compressed_auto_public_rhs' = e(solver_rhs_diagnostics)
matrix `compressed_auto_memory' = e(rust_memory_receipt)
matrix `compressed_auto_prep' = e(rust_preparation_receipt)
matrix `compressed_auto_graph' = e(rust_graph_receipt)
matrix `compressed_auto_cap' = e(rust_request_capability_receipt)
matrix `compressed_auto_receipt' = e(rust_compressed_receipt)
assert rowsof(`compressed_auto_results') == 4 & colsof(`compressed_auto_results') == 4
assert rowsof(`compressed_auto_rhs') == 1+3*e(probes)
assert colsof(`compressed_auto_rhs') == 8
assert rowsof(`compressed_auto_public_rhs') == rowsof(`compressed_auto_rhs')
assert colsof(`compressed_auto_public_rhs') == 6
assert colsof(`compressed_auto_memory') == 12
assert `compressed_auto_memory'[1,3] == e(rust_result_copy_bytes)
assert `compressed_auto_memory'[1,4] == 0
assert `compressed_auto_memory'[1,11] == e(rust_plan_solve_peak_bytes)
assert `compressed_auto_memory'[1,12] == max(                     ///
    `compressed_auto_memory'[1,5],`compressed_auto_memory'[1,11])
assert `compressed_auto_memory'[1,12] <= `compressed_auto_memory'[1,1]
assert colsof(`compressed_auto_prep') == 13
assert colsof(`compressed_auto_graph') == 18
assert colsof(`compressed_auto_cap') == 23
assert colsof(`compressed_auto_receipt') == 11
assert `compressed_auto_receipt'[1,2] == 1
assert `compressed_auto_receipt'[1,10] == 2
assert `compressed_auto_receipt'[1,11] == 1
forvalues row = 1/3 {
    assert abs(`compressed_auto_results'[`row',4]-                  ///
        `compressed_auto_results'[`row',1]-                        ///
        `compressed_auto_results'[`row',2]-                        ///
        2*`compressed_auto_results'[`row',3]) <= 1e-10
}
forvalues column = 1/4 {
    assert abs(`compressed_auto_results'[1,`column']-              ///
        `compressed_auto_results'[2,`column']-                    ///
        `compressed_auto_results'[3,`column']) <= 1e-10
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
assert `"`c(rng)'"' == `"`compressed_auto_rng'"'
assert c(rngstream) == `compressed_auto_stream'
assert `"`c(rngstate)'"' == `"`compressed_auto_state'"'
local compressed_auto_sortedby_after : sortedby
assert `"`compressed_auto_sortedby_after'"' == `"`compressed_auto_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`compressed_auto_signature'"'
'''
if test.count(old_test) != 1:
    raise SystemExit("withheld compressed public test anchor changed")
test_path.write_text(test.replace(old_test, new_test))

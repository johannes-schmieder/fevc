from pathlib import Path

helper_path = Path("varcomp_kss/_vckss_rust_reconcile_comp_v7.ado")
test_path = Path("varcomp_kss/tests/stata/test_rust_planned_compressed.do")

helper = helper_path.read_text()
capture_old = """        plan_res_rng_hi:r_pre_rng_hi plan_res_rng_lo:r_pre_rng_lo        ///
        wall_req_app:r_wall_req_app wall_requested:r_wall_requested {
"""
capture_new = """        plan_res_rng_hi:r_pre_rng_hi plan_res_rng_lo:r_pre_rng_lo        ///
        wall_req_app:r_wall_req_app wall_requested:r_wall_requested      ///
        wall_forecast:r_wall_forecast wall_advisory:r_wall_advisory      ///
        wall_margin:r_wall_margin {
"""
if helper.count(capture_old) != 1:
    raise SystemExit("compressed reconciler wall capture anchor changed")
helper = helper.replace(capture_old, capture_new)

wall_check_anchor = """    if `ok' & `batch_mode'==0 {
        if `r_lev_batch'<1 | `r_lev_batch'>`probes_expected' |          ///
            `r_tgt_batch'<1 | `r_tgt_batch'>`probes_expected' {
            local ok = 0
            local detail "automatic compressed phase batch width was out of range"
        }
    }

"""
wall_check_new = wall_check_anchor + """    if `ok' & (missing(`r_wall_forecast') | missing(`r_wall_advisory') | ///
        missing(`r_wall_margin')) {
        local ok = 0
        local detail "compressed wall-work receipt was incomplete"
    }

"""
if helper.count(wall_check_anchor) != 1:
    raise SystemExit("compressed reconciler return prelude anchor changed")
helper = helper.replace(wall_check_anchor, wall_check_new)

return_start = helper.index("    return scalar ok = `ok'\n")
return_end = helper.index("end\n", return_start)
new_return = """    return scalar ok = `ok'
    return local detail `"`detail'"'
    return local result_family "compressed"
    return local execution_plan_schema `"`receipt_schema'"'
    return scalar expected_rhs_rows = `expected_rhs_rows'
    return scalar rhs_max_iterations = `rhs_max_iterations'
    return scalar rhs_max_reduced = `rhs_max_reduced'
    return scalar rhs_max_complete = `rhs_max_complete'
    return scalar accounting_truth = `accounting_truth'
    return scalar seed = `r_seed'
    return scalar probes = `r_probes'
    return scalar leverage_probes_accepted = `r_lev_accepted'
    return scalar target_probes_accepted = `r_tgt_accepted'
    return scalar requested_algorithm_code = `r_alg_req'
    return scalar selected_algorithm_code = `r_alg_sel'
    return scalar requested_engine_code = `r_eng_req'
    return scalar selected_engine_code = `r_eng_sel'
    return scalar requested_route = `r_route_req'
    return scalar selected_route = `r_route_sel'
    return scalar solver_fallback = `r_fallback'
    return scalar solver_fallback_error = `r_fallback_error'
    return scalar solver_dimension = `r_dimension'
    return scalar leverage_batch_width = `r_lev_batch'
    return scalar target_batch_width = `r_tgt_batch'
    return scalar rng_contract_code = `r_rng'
    return scalar rhs_receipt_rows = `r_rhs_rows'
    return scalar caller_result_copy_bytes = `r_result_copy'
    return scalar memory_limit_bytes = `r_mem_limit'
    return scalar caller_copy_bytes = `r_input_copy'
    return scalar preparation_peak_bytes = `r_prep_peak'
    return scalar prepared_resident_bytes = `r_resident'
    return scalar solver_setup_bytes = `r_solver_setup'
    return scalar leverage_phase_bytes = `r_lev_phase'
    return scalar target_phase_bytes = `r_tgt_phase'
    return scalar result_forecast_bytes = `r_result_bytes'
    return scalar solve_peak_bytes = `r_solve_peak'
    return scalar command_peak_bytes = `r_command_peak'
    return scalar full_fit_route = `r_full_route'
    return scalar full_fit_iterations = `r_full_iter'
    return scalar full_fit_reduced_residual = `r_full_reduced'
    return scalar full_fit_complete_residual = `r_full_complete'
    return scalar full_fit_zero_rhs = `r_full_zero'
    return scalar leverage_rhs_count = `r_lev_rhs'
    return scalar target_rhs_count = `r_tgt_rhs'
    return scalar max_reduced_residual = `r_max_reduced'
    return scalar max_complete_residual = `r_max_complete'
    return scalar max_leverage = `r_max_leverage'
    return scalar max_reciprocal_residual = `r_max_reciprocal'
    return scalar accounting_residual = `r_accounting'
    return scalar actual_accounting_residual = `r_actual_accounting'
    return scalar weighted_rss = `r_weighted_rss'
    return scalar rank_tolerance = `r_rank_tolerance'
    return scalar block_tolerance = `r_block_tolerance'
    return scalar full_residual_tolerance = `r_full_tolerance'
    return scalar deletion_mode_code = `r_deletion'
    return scalar nuisance_mode_code = `r_nuisance'
    return scalar parameters = `r_parameters'
    return scalar full_parameters = `r_full_parameters'
    return scalar correction_parameters = `r_correction_parameters'
    return scalar topology_checksum_hi = `r_topology_hi'
    return scalar topology_checksum_lo = `r_topology_lo'
    return scalar rhs_receipt_schema = `r_rhs_schema'
    return scalar capability_schema = `r_cap_schema'
    return scalar capability_profile = `r_cap_profile'
    return scalar request_signature_hi = `r_signature_hi'
    return scalar request_signature_lo = `r_signature_lo'
    return scalar batch_mode_code = `r_batch_mode'
    return scalar leverage_batch_mode_code = `r_lev_batch_mode'
    return scalar target_batch_mode_code = `r_tgt_batch_mode'
    return scalar stayers_mode_code = `r_stayers_mode'
    return scalar target_weight_mode_code = `r_target_mode'
    return scalar deletion_source_code = `r_deletion_source'
    return scalar probeorder_supplied = `r_probeorder'
    return scalar wallseconds_supplied = `r_wall_supplied'
    return scalar frequency_use_code = `r_frequency'
    return scalar physical_limit = `r_physical_limit'
    return scalar rhs_v2_copy_bytes = `r_rhs_v2_copy'
    return scalar plan_struct = `r_plan_struct'
    return scalar plan_schema = `r_plan_schema'
    return scalar plan_route_schema = `r_plan_route_schema'
    return scalar plan_resolved = `r_plan_resolved'
    return scalar plan_frozen = `r_plan_frozen'
    return scalar plan_applicability = `r_plan_applicability'
    return scalar plan_algorithm_requested = `r_plan_alg_req'
    return scalar plan_algorithm_selected = `r_plan_alg_sel'
    return scalar plan_engine_requested = `r_plan_eng_req'
    return scalar plan_engine_selected = `r_plan_eng_sel'
    return scalar plan_route_requested = `r_plan_route_req'
    return scalar plan_route_selected = `r_plan_route_sel'
    return scalar plan_route_fallback = `r_plan_route_fallback'
    return scalar plan_route_error = `r_plan_route_error'
    return scalar plan_rhs = `r_plan_rhs'
    return scalar plan_full_dimension = `r_plan_full_dim'
    return scalar plan_leverage_batch = `r_batch_lev_sel'
    return scalar plan_target_batch = `r_batch_tgt_sel'
    return scalar plan_batch_command = `r_batch_command'
    return scalar plan_memory_command = `r_mem_command'
    return scalar counter_complete = `r_ctr_complete'
    return scalar pre_rng_hi = `r_pre_rng_hi'
    return scalar pre_rng_lo = `r_pre_rng_lo'
    return scalar wall_request_applicable = `r_wall_req_app'
    return scalar wallseconds_requested = `r_wall_requested'
    return scalar wallseconds_forecast = `r_wall_forecast'
    return scalar wallseconds_advisory = `r_wall_advisory'
    return scalar wallseconds_margin = `r_wall_margin'
    return matrix result = `raw_results'
    return matrix rhs_receipts = `rhs_native'
"""
helper = helper[:return_start] + new_return + helper[return_end:]
helper_path.write_text(helper)

test = test_path.read_text()
test_anchor = """assert r(selected_engine) == 1
assert r(selected_route) == 2
tempname helper_estimates helper_rhs
"""
test_new = """assert r(selected_engine) == 1
assert r(selected_route) == 2
assert `"`r(result_family)'"' == "compressed"
assert `"`r(execution_plan_schema)'"' == "VCKSS-EXECUTION-PLAN-V1"
assert r(seed) == 81227
assert r(probes) == `probes'
assert r(leverage_probes_accepted) == `probes'
assert r(target_probes_accepted) == `probes'
assert r(requested_algorithm_code) == 2
assert r(selected_algorithm_code) == 2
assert r(requested_engine_code) == 0
assert r(selected_engine_code) == 1
assert r(requested_route) == 2
assert r(solver_fallback) == 0
assert r(solver_fallback_error) == 0
assert r(solver_dimension) == `prepared_firms' - 1
assert r(leverage_batch_width) == 2
assert r(target_batch_width) == 2
assert r(rng_contract_code) == 1
assert r(rhs_receipt_rows) == 1 + 3 * `probes'
assert r(caller_result_copy_bytes) == 112 * r(rhs_receipt_rows)
assert r(memory_limit_bytes) == `prepared_memory_limit'
assert r(caller_copy_bytes) == `prepared_input_copy'
assert r(preparation_peak_bytes) == `prepared_peak'
assert r(prepared_resident_bytes) == `prepared_resident'
assert r(full_fit_complete_residual) <= r(full_residual_tolerance)
assert r(max_complete_residual) <= r(full_residual_tolerance)
assert r(leverage_rhs_count) == `probes'
assert r(target_rhs_count) == 2 * `probes'
assert r(parameters) == `prepared_workers' + `prepared_firms' - 1
assert r(full_parameters) == r(parameters)
assert r(correction_parameters) == r(parameters)
assert r(rhs_receipt_schema) == 1
assert r(capability_schema) == `capability_schema'
assert r(capability_profile) == `capability_profile'
assert r(request_signature_hi) == `signature_hi'
assert r(request_signature_lo) == `signature_lo'
assert r(batch_mode_code) == 1
assert r(leverage_batch_mode_code) == 1
assert r(target_batch_mode_code) == 1
assert r(stayers_mode_code) == 1
assert r(target_weight_mode_code) == 1
assert r(deletion_source_code) == 2
assert r(probeorder_supplied) == 0
assert r(wallseconds_supplied) == 0
assert r(frequency_use_code) == 1
assert r(physical_limit) == 50000000
assert r(rhs_v2_copy_bytes) == 0
assert r(plan_struct) == 1000
assert r(plan_schema) == 1
assert r(plan_route_schema) == 2
assert r(plan_resolved) == 1
assert r(plan_frozen) == 1
assert r(plan_applicability) == 2
assert r(plan_algorithm_requested) == 2
assert r(plan_algorithm_selected) == 2
assert r(plan_engine_requested) == 0
assert r(plan_engine_selected) == 1
assert r(plan_route_requested) == 2
assert r(plan_route_selected) == 2
assert r(plan_route_fallback) == 0
assert r(plan_route_error) == 0
assert r(plan_rhs) == 1 + 3 * `probes'
assert r(plan_full_dimension) == `prepared_firms' - 1
assert r(plan_leverage_batch) == 2
assert r(plan_target_batch) == 2
assert r(plan_batch_command) == r(plan_memory_command)
assert r(plan_memory_command) == r(solve_peak_bytes)
assert r(counter_complete) == 1
assert r(pre_rng_hi) == 0
assert r(pre_rng_lo) == 0
assert r(wall_request_applicable) == 0
assert r(wallseconds_requested) == 0
assert !missing(r(wallseconds_forecast))
assert !missing(r(wallseconds_advisory))
assert !missing(r(wallseconds_margin))
tempname helper_estimates helper_rhs
"""
if test.count(test_anchor) != 1:
    raise SystemExit("compressed helper assertion anchor changed")
test = test.replace(test_anchor, test_new)
test_path.write_text(test)

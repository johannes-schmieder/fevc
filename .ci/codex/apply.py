from pathlib import Path


def splice(text: str, start_anchor: str, end_anchor: str, replacement: str, label: str) -> str:
    start_count = text.count(start_anchor)
    end_count = text.count(end_anchor)
    if start_count != 1 or end_count != 1:
        raise SystemExit(
            f"{label}: expected unique anchors, got start={start_count}, end={end_count}"
        )
    start = text.index(start_anchor)
    end = text.index(end_anchor, start)
    return text[:start] + replacement + text[end:]


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one anchor, found {count}")
    return text.replace(old, new)


plan_path = Path("varcomp_kss/_vckss_rust_plan_receipt.ado")
plan = plan_path.read_text()

invariant_start = """    if !`receipt_mismatch' {
        local invariant_names plan_struct plan_schema plan_alg_schema       ///
"""
reconcile_start = """    if !`receipt_mismatch' {
        local plan_names plan_alg_req plan_alg_sel plan_eng_req plan_eng_sel ///
"""
new_invariants = r'''    if !`receipt_mismatch' {
        local invariant_names plan_struct plan_schema plan_alg_schema       ///
            plan_eng_schema plan_route_schema batch_schema wall_schema     ///
            ctr_schema mem_schema plan_resolved plan_frozen wall_routing   ///
            ctr_complete
        local invariant_values 1000 1 1 2 2 1 1 1 1 1 1 1 1 1
        local invariant_count : word count `invariant_names'
        forvalues index = 1/`invariant_count' {
            local name : word `index' of `invariant_names'
            local expected : word `index' of `invariant_values'
            local actual = scalar(__vckss_`name')
            if `actual' != `expected' {
                local receipt_mismatch = 1
                if "`mismatch_detail'" == "" {
                    local mismatch_detail "invariant __vckss_`name'=`actual', expected `expected'"
                }
            }
        }
    }

    if !`receipt_mismatch' {
        local applicability = scalar(__vckss_plan_applicability)
        if !inlist(`applicability',1,2,3) {
            local receipt_mismatch = 1
            local mismatch_detail "unknown execution-plan applicability `applicability'"
        }
        else {
            local expected_batched = (`applicability' != 1)
            if scalar(__vckss_batch_determ) != `expected_batched' |      ///
                scalar(__vckss_batch_invariant) != `expected_batched' | ///
                scalar(__vckss_batch_arithmetic) != `expected_batched' | ///
                scalar(__vckss_batch_admitted) != `expected_batched' | ///
                scalar(__vckss_batch_app) != `applicability' |          ///
                scalar(__vckss_batch_lev_app) != `applicability' |      ///
                scalar(__vckss_batch_tgt_app) != `applicability' |      ///
                scalar(__vckss_wall_model) != `applicability' |         ///
                scalar(__vckss_mem_app) != `applicability' {
                local receipt_mismatch = 1
                local mismatch_detail "execution-plan applicability did not reconcile across batch, wall, and memory receipts"
            }
            local expected_app_flags = cond(`applicability'==1,59,63)
            local expected_contract_flags = cond(`applicability'==1,31,15)
            if !`receipt_mismatch' & (                               ///
                scalar(__vckss_plan_app_hi) != 0 |                   ///
                scalar(__vckss_plan_app_lo) != `expected_app_flags' | ///
                scalar(__vckss_plan_contract_hi) != 0 |              ///
                scalar(__vckss_plan_contract_lo) !=                  ///
                    `expected_contract_flags' |                       ///
                scalar(__vckss_plan_eng_fallback) != 0) {
                local receipt_mismatch = 1
                local mismatch_detail "execution-plan applicability or fail-closed contract flags were inconsistent"
            }
            if !`receipt_mismatch' & `applicability' == 1 {
                if scalar(__vckss_plan_full_dim) != 0 |              ///
                    scalar(__vckss_plan_fe_dim) != 0 |                ///
                    scalar(__vckss_plan_rhs) != 0 |                   ///
                    scalar(__vckss_plan_auto_firms) != 0 |            ///
                    scalar(__vckss_plan_auto_rhs) != 0 |              ///
                    scalar(__vckss_batch_nonbatched) != 0 |           ///
                    scalar(__vckss_batch_command) != 0 |              ///
                    scalar(__vckss_ctr_rng) != 0 |                    ///
                    scalar(__vckss_mem_nonbatched) !=                 ///
                        scalar(__vckss_mem_command) |                 ///
                    scalar(__vckss_mem_leverage) != 0 |              ///
                    scalar(__vckss_mem_target) != 0 {
                    local receipt_mismatch = 1
                    local mismatch_detail "exact execution plan carried a JLA-only dimension, batch, RNG, or phase value"
                }
                foreach phase in lev tgt {
                    if scalar(__vckss_batch_`phase'_mode) != 3 |      ///
                        scalar(__vckss_batch_`phase'_reason) != 0 |   ///
                        scalar(__vckss_batch_`phase'_req) != 0 |      ///
                        scalar(__vckss_batch_`phase'_sel) != 0 |      ///
                        scalar(__vckss_batch_`phase'_probe) != 0 |    ///
                        scalar(__vckss_batch_`phase'_threads) != 0 |  ///
                        scalar(__vckss_batch_`phase'_threadcap) != 0 | ///
                        scalar(__vckss_batch_`phase'_routecap) != 0 | ///
                        scalar(__vckss_batch_`phase'_effcap) != 0 |   ///
                        scalar(__vckss_batch_`phase'_hard) != 0 |     ///
                        scalar(__vckss_batch_`phase'_onebytes) != 0 | ///
                        scalar(__vckss_batch_`phase'_selbytes) != 0 {
                        local receipt_mismatch = 1
                        if "`mismatch_detail'" == "" {
                            local mismatch_detail "exact execution plan carried an applicable `phase' batch receipt"
                        }
                    }
                }
            }
        }
    }

'''
plan = splice(plan, invariant_start, reconcile_start, new_invariants, "plan invariant block")

legacy_peak_start = """    if !`receipt_mismatch' {
        local legacy_command_peak = max(                               ///
"""
new_reconciliation = r'''    if !`receipt_mismatch' {
        local plan_names plan_alg_req plan_alg_sel plan_eng_req plan_eng_sel ///
            plan_rhs ctr_rng mem_hard mem_prepared mem_command             ///
            plan_sig_hi plan_sig_lo
        local result_names rust_algorithm_req rust_algorithm_sel            ///
            rust_engine_requested rust_engine_selected rust_rhs_rows        ///
            rust_rng_contract rust_memory_limit rust_prepared_resident      ///
            rust_solve_peak rust_solve_signature_hi rust_solve_signature_lo
        local reconciliation_count : word count `plan_names'
        forvalues index = 1/`reconciliation_count' {
            local plan_name : word `index' of `plan_names'
            local result_name : word `index' of `result_names'
            local plan_value = scalar(__vckss_`plan_name')
            local result_value = scalar(__vckss_`result_name')
            if `plan_value' != `result_value' {
                local receipt_mismatch = 1
                if "`mismatch_detail'" == "" {
                    local mismatch_detail "reconciliation `plan_name'=`plan_value' versus `result_name'=`result_value'"
                }
            }
        }
    }

    if !`receipt_mismatch' & scalar(__vckss_plan_applicability) != 1 {
        local plan_names plan_full_dim batch_command batch_nonbatched
        local result_names rust_solver_dimension mem_command mem_nonbatched
        local reconciliation_count : word count `plan_names'
        forvalues index = 1/`reconciliation_count' {
            local plan_name : word `index' of `plan_names'
            local result_name : word `index' of `result_names'
            local plan_value = scalar(__vckss_`plan_name')
            local result_value = scalar(__vckss_`result_name')
            if `plan_value' != `result_value' {
                local receipt_mismatch = 1
                if "`mismatch_detail'" == "" {
                    local mismatch_detail "JLA reconciliation `plan_name'=`plan_value' versus `result_name'=`result_value'"
                }
            }
        }
    }

'''
plan = splice(plan, reconcile_start, legacy_peak_start, new_reconciliation, "plan/result reconciliation block")

route_start = """    if !`receipt_mismatch' {
        // The frozen V6 generic prefix predates automatic/CMG generic routing
"""
pre_rng_start = """    if !`receipt_mismatch' {
        foreach stem in plan_res_rng plan_res_ctr plan_pre_atom plan_pre_word ///
"""
new_route = r'''    if !`receipt_mismatch' {
        // The frozen V6 generic prefix predates automatic/CMG generic routing
        // and must remain diagonal/no-fallback. V7 is authoritative for the
        // requested and selected route of a planned generic solve.
        local applicability = scalar(__vckss_plan_applicability)
        if `applicability' == 3 {
            if scalar(__vckss_rust_route_requested) != 2 |               ///
                scalar(__vckss_rust_route_selected) != 2 |               ///
                scalar(__vckss_rust_fallback) != 0 |                     ///
                scalar(__vckss_rust_fallback_error) != 0 {
                local receipt_mismatch = 1
                local mismatch_detail "legacy generic V6 route prefix was not diagonal/no-fallback"
            }
        }
        else if `applicability' == 2 {
            local route_plan_names plan_route_req plan_route_sel          ///
                plan_route_fallback plan_route_error
            local route_result_names rust_route_requested rust_route_selected ///
                rust_fallback rust_fallback_error
            local route_count : word count `route_plan_names'
            forvalues index = 1/`route_count' {
                local plan_name : word `index' of `route_plan_names'
                local result_name : word `index' of `route_result_names'
                local plan_value = scalar(__vckss_`plan_name')
                local result_value = scalar(__vckss_`result_name')
                if `plan_value' != `result_value' {
                    local receipt_mismatch = 1
                    if "`mismatch_detail'" == "" {
                        local mismatch_detail "reconciliation `plan_name'=`plan_value' versus `result_name'=`result_value'"
                    }
                }
            }
        }
        else if scalar(__vckss_plan_route_req) != 4 |                ///
            scalar(__vckss_plan_route_sel) != 4 |                    ///
            scalar(__vckss_plan_route_fallback) != 0 |               ///
            scalar(__vckss_plan_route_error) != 0 |                  ///
            scalar(__vckss_plan_route_contract) != 0 |               ///
            scalar(__vckss_rust_route_requested) != 1 |              ///
            scalar(__vckss_rust_route_selected) != 1 |               ///
            scalar(__vckss_rust_fallback) != 0 |                     ///
            scalar(__vckss_rust_fallback_error) != 0 {
            local receipt_mismatch = 1
            local mismatch_detail "exact plan/result route applicability was inconsistent"
        }
    }

'''
plan = splice(plan, route_start, pre_rng_start, new_route, "plan route reconciliation block")

promotion_old = r'''    // Promote the additive V7 route truth to the existing public result names.
    // The raw V6 prefix stays frozen inside the plugin receipt itself.
    scalar __vckss_rust_route_requested = scalar(__vckss_plan_route_req)
    scalar __vckss_rust_route_selected = scalar(__vckss_plan_route_sel)
    scalar __vckss_rust_fallback = scalar(__vckss_plan_route_fallback)
    scalar __vckss_rust_fallback_error = scalar(__vckss_plan_route_error)
'''
promotion_new = r'''    // Promote additive V7 route truth for JLA. Exact keeps the frozen
    // dense-estimator route code while V7 separately reports not-applicable
    // iterative routing.
    if scalar(__vckss_plan_applicability) != 1 {
        scalar __vckss_rust_route_requested = scalar(__vckss_plan_route_req)
        scalar __vckss_rust_route_selected = scalar(__vckss_plan_route_sel)
        scalar __vckss_rust_fallback = scalar(__vckss_plan_route_fallback)
        scalar __vckss_rust_fallback_error = scalar(__vckss_plan_route_error)
    }
'''
plan = replace_once(plan, promotion_old, promotion_new, "route promotion")
plan_path.write_text(plan)


test_path = Path("varcomp_kss/tests/stata/test_rust_planned_v4.do")
test = test_path.read_text()
insert_anchor = 'di as result "VARCOMP_KSS RUST PLANNED V4 PASS"\n'
if test.count(insert_anchor) != 1:
    raise SystemExit("planned V4 final marker changed")
exact_test = r'''// A V3 algorithm(auto) request must reconcile an exact selection without
// borrowing JLA batch or iterative-route semantics. The retained identified
// dimension is 12+4-1=15, below exactlimit(500).
quietly varcomp_kss_rust requestcapability, algorithm(auto) deletion(match) ///
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

quietly varcomp_kss_rust prepare worker firm deletion_id outcome frequency ///
    target_weight, cleanup memorygib(1) deletion(match)
local exact_auto_handle = r(handle)
quietly varcomp_kss_rust solve `exact_auto_handle', algorithm(auto)         ///
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

quietly varcomp_kss_rust result `exact_auto_handle'
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
assert r(plan_route_schema) == 2
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
quietly varcomp_kss_rust release `exact_auto_handle'
quietly varcomp_kss_rust snapshot
assert r(state) == 0 & r(handle) == 0

'''
test_path.write_text(test.replace(insert_anchor, exact_test + insert_anchor))

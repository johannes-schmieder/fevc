*! version 0.3.0-dev 23aug2026
program define _vckss_rust_plan_receipt, rclass
    version 18.0

    local integer_names plan_struct plan_schema plan_alg_schema plan_alg_req ///
        plan_alg_sel plan_alg_reason plan_eng_schema plan_eng_req           ///
        plan_eng_sel plan_eng_reason plan_comp_elig plan_resolved           ///
        plan_eng_fallback plan_complexity plan_exact_limit                  ///
        plan_route_schema plan_route_req plan_route_sel plan_route_fallback ///
        plan_full_setup plan_fe_setup plan_fe_reuse plan_frozen             ///
        plan_route_contract plan_threads_req plan_threads_used plan_parallel ///
        plan_applicability plan_rhs plan_auto_firms plan_auto_rhs           ///
        plan_full_dim plan_fe_dim batch_schema batch_determ batch_invariant ///
        batch_arithmetic batch_admitted batch_app batch_nonbatched          ///
        batch_command wall_schema wall_model wall_status wall_routing       ///
        wall_req_app wall_fcst_app wall_adv_app wall_margin_app             ///
        wall_prepare wall_setup wall_fit wall_leverage wall_target          ///
        wall_export wall_total ctr_schema ctr_rng ctr_work_app ctr_complete ///
        mem_schema mem_app mem_phase mem_hard mem_prepared mem_setup        ///
        mem_fit mem_correction mem_leverage mem_target mem_result           ///
        mem_nonbatched mem_command mem_shared_cmg mem_control mem_transient ///
        mem_cmg_workspace mem_cmg_cells mem_cmg_groups mem_cmg_graph       ///
        mem_nq mem_q2
    foreach phase in lev tgt {
        local integer_names `integer_names' batch_`phase'_mode             ///
            batch_`phase'_reason batch_`phase'_app batch_`phase'_req       ///
            batch_`phase'_sel batch_`phase'_probe batch_`phase'_threads   ///
            batch_`phase'_threadcap batch_`phase'_routecap                ///
            batch_`phase'_effcap batch_`phase'_hard                       ///
            batch_`phase'_onebytes batch_`phase'_selbytes
    }
    local signed_names plan_route_error
    local floating_names wall_requested wall_forecast wall_advisory wall_margin
    local half_names plan_app_hi plan_app_lo plan_contract_hi plan_contract_lo ///
        plan_sig_hi plan_sig_lo plan_res_rng_hi plan_res_rng_lo               ///
        plan_res_ctr_hi plan_res_ctr_lo plan_pre_atom_hi plan_pre_atom_lo      ///
        plan_pre_word_hi plan_pre_word_lo plan_pre_trial_hi plan_pre_trial_lo  ///
        ctr_pre_atom_hi ctr_pre_atom_lo ctr_pre_word_hi ctr_pre_word_lo        ///
        ctr_pre_trial_hi ctr_pre_trial_lo
    foreach phase in lev tgt all {
        foreach field in pla ala puw auw ppt apt pgw agw {
            local half_names `half_names' ctr_`phase'_`field'_hi             ///
                ctr_`phase'_`field'_lo
        }
    }
    local common_names rust_algorithm_req rust_algorithm_sel               ///
        rust_engine_requested rust_engine_selected rust_route_requested    ///
        rust_route_selected rust_fallback rust_fallback_error              ///
        rust_solver_dimension rust_lev_batch rust_tgt_batch                ///
        rust_rng_contract rust_rhs_rows rust_memory_limit                  ///
        rust_prepared_resident rust_prepare_peak rust_solve_peak           ///
        rust_command_peak rust_solve_signature_hi rust_solve_signature_lo

    local all_names `integer_names' `signed_names' `floating_names' `half_names'
    local receipt_mismatch = 0
    local mismatch_detail
    foreach name of local all_names {
        capture confirm scalar __vckss_`name'
        if _rc {
            local receipt_mismatch = 1
            if "`mismatch_detail'" == "" {
                local mismatch_detail "missing scalar __vckss_`name'"
            }
        }
    }
    foreach name of local common_names {
        capture confirm scalar __vckss_`name'
        if _rc {
            local receipt_mismatch = 1
            if "`mismatch_detail'" == "" {
                local mismatch_detail "missing common scalar __vckss_`name'"
            }
        }
    }

    if !`receipt_mismatch' {
        foreach name of local integer_names {
            local value = scalar(__vckss_`name')
            if missing(`value') | `value' < 0 | `value' != floor(`value') {
                local receipt_mismatch = 1
                if "`mismatch_detail'" == "" {
                    local mismatch_detail "invalid nonnegative integer __vckss_`name'"
                }
            }
        }
        foreach name of local signed_names {
            local value = scalar(__vckss_`name')
            if missing(`value') | `value' != floor(`value') |             ///
                `value' < -2147483648 | `value' > 2147483647 {
                local receipt_mismatch = 1
                if "`mismatch_detail'" == "" {
                    local mismatch_detail "invalid signed integer __vckss_`name'"
                }
            }
        }
        foreach name of local floating_names {
            local value = scalar(__vckss_`name')
            if missing(`value') | `value' < 0 {
                local receipt_mismatch = 1
                if "`mismatch_detail'" == "" {
                    local mismatch_detail "invalid nonnegative floating __vckss_`name'"
                }
            }
        }
        foreach name of local half_names {
            local value = scalar(__vckss_`name')
            if missing(`value') | `value' < 0 | `value' > 4294967295 |    ///
                `value' != floor(`value') {
                local receipt_mismatch = 1
                if "`mismatch_detail'" == "" {
                    local mismatch_detail "invalid u32 half __vckss_`name'"
                }
            }
        }
    }

    if !`receipt_mismatch' {
        local invariant_names plan_struct plan_schema plan_alg_schema       ///
            plan_eng_schema batch_schema wall_schema ctr_schema mem_schema ///
            plan_resolved plan_frozen wall_routing ctr_complete
        local invariant_values 1000 1 1 2 1 1 1 1 1 1 1 1 1
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
            local expected_route_schema = cond(`applicability'==1,1,2)
            local expected_batched = (`applicability' != 1)
            if scalar(__vckss_plan_route_schema) !=                      ///
                    `expected_route_schema' |                            ///
                scalar(__vckss_batch_determ) != `expected_batched' |    ///
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

    if !`receipt_mismatch' {
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

    if !`receipt_mismatch' {
        local legacy_command_peak = max(                               ///
            scalar(__vckss_rust_prepare_peak),                         ///
            scalar(__vckss_rust_solve_peak))
        if scalar(__vckss_rust_command_peak) != `legacy_command_peak' {
            local receipt_mismatch = 1
            local mismatch_detail "legacy command peak did not equal max(preparation, solve)"
        }
    }

    if !`receipt_mismatch' {
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

    if !`receipt_mismatch' {
        foreach stem in plan_res_rng plan_res_ctr plan_pre_atom plan_pre_word ///
            plan_pre_trial ctr_pre_atom ctr_pre_word ctr_pre_trial {
            if scalar(__vckss_`stem'_hi) != 0 | scalar(__vckss_`stem'_lo) != 0 {
                local receipt_mismatch = 1
                if "`mismatch_detail'" == "" {
                    local mismatch_detail "pre-RNG counter `stem' was nonzero"
                }
            }
        }
        foreach phase in lev tgt all {
            foreach pair in "pla ala" "puw auw" "ppt apt" "pgw agw" {
                gettoken planned actual : pair
                if scalar(__vckss_ctr_`phase'_`planned'_hi) !=             ///
                        scalar(__vckss_ctr_`phase'_`actual'_hi) |         ///
                    scalar(__vckss_ctr_`phase'_`planned'_lo) !=           ///
                        scalar(__vckss_ctr_`phase'_`actual'_lo) {
                    local receipt_mismatch = 1
                    if "`mismatch_detail'" == "" {
                        local mismatch_detail "counter `phase' `planned' did not equal `actual'"
                    }
                }
            }
        }
        if scalar(__vckss_batch_lev_app) != 0 &                          ///
            scalar(__vckss_batch_lev_sel) != scalar(__vckss_rust_lev_batch) {
            local receipt_mismatch = 1
            if "`mismatch_detail'" == "" {
                local mismatch_detail "selected leverage batch disagreed with result"
            }
        }
        if scalar(__vckss_batch_tgt_app) != 0 &                          ///
            scalar(__vckss_batch_tgt_sel) != scalar(__vckss_rust_tgt_batch) {
            local receipt_mismatch = 1
            if "`mismatch_detail'" == "" {
                local mismatch_detail "selected target batch disagreed with result"
            }
        }
        local wall_sum = scalar(__vckss_wall_prepare) +                  ///
            scalar(__vckss_wall_setup) + scalar(__vckss_wall_fit) +      ///
            scalar(__vckss_wall_leverage) + scalar(__vckss_wall_target) + ///
            scalar(__vckss_wall_export)
        if scalar(__vckss_wall_total) != `wall_sum' {
            local receipt_mismatch = 1
            if "`mismatch_detail'" == "" {
                local mismatch_detail "wall total disagreed with phase sum"
            }
        }
        if scalar(__vckss_wall_req_app) == 0 & scalar(__vckss_wall_requested) != 0 {
            local receipt_mismatch = 1
            if "`mismatch_detail'" == "" {
                local mismatch_detail "inapplicable requested wall value was nonzero"
            }
        }
        if scalar(__vckss_wall_fcst_app) == 0 & scalar(__vckss_wall_forecast) != 0 {
            local receipt_mismatch = 1
            if "`mismatch_detail'" == "" {
                local mismatch_detail "inapplicable forecast wall value was nonzero"
            }
        }
        if scalar(__vckss_wall_adv_app) == 0 & scalar(__vckss_wall_advisory) != 0 {
            local receipt_mismatch = 1
            if "`mismatch_detail'" == "" {
                local mismatch_detail "inapplicable advisory wall value was nonzero"
            }
        }
        if scalar(__vckss_wall_margin_app) == 0 & scalar(__vckss_wall_margin) != 0 {
            local receipt_mismatch = 1
            if "`mismatch_detail'" == "" {
                local mismatch_detail "inapplicable wall margin was nonzero"
            }
        }
    }

    if `receipt_mismatch' {
        if "`mismatch_detail'" == "" {
            local mismatch_detail "unspecified execution-plan mismatch"
        }
        foreach name of local all_names {
            capture scalar drop __vckss_`name'
        }
        di as err "Rust V7 execution-plan receipt mismatch: `mismatch_detail'"
        exit 498
    }

    // Promote additive V7 route truth for JLA. Exact keeps the frozen
    // dense-estimator route code while V7 separately reports not-applicable
    // iterative routing.
    if scalar(__vckss_plan_applicability) != 1 {
        scalar __vckss_rust_route_requested = scalar(__vckss_plan_route_req)
        scalar __vckss_rust_route_selected = scalar(__vckss_plan_route_sel)
        scalar __vckss_rust_fallback = scalar(__vckss_plan_route_fallback)
        scalar __vckss_rust_fallback_error = scalar(__vckss_plan_route_error)
    }

    foreach name of local all_names {
        return scalar `name' = scalar(__vckss_`name')
    }
    foreach name of local all_names {
        capture scalar drop __vckss_`name'
    }
    return local receipt_schema "VCKSS-EXECUTION-PLAN-V1"
end

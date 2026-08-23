*! version 0.3.0-dev 23aug2026
program define _vckss_rust_reconcile_comp_v7, rclass
    version 18.0
    args probes_expected seed_expected maxiter tolerance workers firms      ///
        rank_tolerance block_tolerance nuisance_code route_requested        ///
        fallback_allowed batch_mode leverage_batch_requested                ///
        target_batch_requested target_mode deletion_source frequency_use    ///
        memory_limit input_copy preparation_peak prepared_resident          ///
        signature_hi signature_lo physical_limit wall_supplied wall_requested

    // This helper must be called immediately after the native V7 result
    // export. Copy every result used below before issuing any r-class command.
    tempname raw_results rhs_native
    matrix `raw_results' = r(result)
    matrix `rhs_native' = r(rhs_receipts)
    local receipt_schema `"`r(receipt_schema)'"'
    foreach pair in seed:r_seed probes:r_probes                         ///
        leverage_probes_accepted:r_lev_accepted                        ///
        target_probes_accepted:r_tgt_accepted                          ///
        requested_algorithm_code:r_alg_req selected_algorithm_code:r_alg_sel ///
        requested_engine_code:r_eng_req selected_engine_code:r_eng_sel  ///
        requested_route:r_route_req selected_route:r_route_sel          ///
        solver_fallback:r_fallback solver_fallback_error:r_fallback_error ///
        solver_dimension:r_dimension leverage_batch_width:r_lev_batch   ///
        target_batch_width:r_tgt_batch rng_contract_code:r_rng          ///
        rhs_receipt_rows:r_rhs_rows memory_limit_bytes:r_mem_limit       ///
        caller_copy_bytes:r_input_copy                                  ///
        caller_result_copy_bytes:r_result_copy                           ///
        preparation_peak_forecast_bytes:r_prep_peak                      ///
        prepared_resident_bytes:r_resident                               ///
        solver_setup_forecast_bytes:r_solver_setup                       ///
        leverage_phase_forecast_bytes:r_lev_phase                        ///
        target_phase_forecast_bytes:r_tgt_phase                          ///
        result_forecast_bytes:r_result_bytes                             ///
        solve_peak_forecast_bytes:r_solve_peak                           ///
        command_peak_forecast_bytes:r_command_peak                       ///
        full_fit_route:r_full_route full_fit_iterations:r_full_iter      ///
        full_fit_reduced_residual:r_full_reduced                         ///
        full_fit_complete_residual:r_full_complete                       ///
        full_fit_zero_rhs:r_full_zero leverage_rhs_count:r_lev_rhs       ///
        target_rhs_count:r_tgt_rhs max_reduced_residual:r_max_reduced    ///
        max_complete_residual:r_max_complete                             ///
        max_leverage:r_max_leverage                                      ///
        max_reciprocal_residual:r_max_reciprocal                         ///
        accounting_residual:r_accounting                                 ///
        actual_accounting_residual:r_actual_accounting                   ///
        weighted_rss:r_weighted_rss rank_tolerance:r_rank_tolerance      ///
        block_tolerance:r_block_tolerance                                ///
        full_residual_tolerance:r_full_tolerance                         ///
        deletion_mode_code:r_deletion nuisance_mode_code:r_nuisance     ///
        parameters:r_parameters full_parameters:r_full_parameters       ///
        correction_parameters:r_correction_parameters                   ///
        topology_checksum_hi:r_topology_hi topology_checksum_lo:r_topology_lo ///
        rhs_receipt_schema:r_rhs_schema capability_schema:r_cap_schema  ///
        capability_profile:r_cap_profile request_signature_hi:r_signature_hi ///
        request_signature_lo:r_signature_lo batch_mode_code:r_batch_mode ///
        leverage_batch_mode_code:r_lev_batch_mode                        ///
        target_batch_mode_code:r_tgt_batch_mode                          ///
        stayers_mode_code:r_stayers_mode target_weight_mode_code:r_target_mode ///
        deletion_source_code:r_deletion_source                           ///
        probeorder_supplied:r_probeorder wallseconds_supplied:r_wall_supplied ///
        frequency_use_code:r_frequency physical_limit:r_physical_limit  ///
        rhs_v2_caller_copy_bytes:r_rhs_v2_copy plan_struct:r_plan_struct ///
        plan_schema:r_plan_schema plan_route_schema:r_plan_route_schema ///
        plan_resolved:r_plan_resolved plan_frozen:r_plan_frozen          ///
        plan_applicability:r_plan_applicability                          ///
        plan_alg_req:r_plan_alg_req plan_alg_sel:r_plan_alg_sel          ///
        plan_eng_req:r_plan_eng_req plan_eng_sel:r_plan_eng_sel          ///
        plan_route_req:r_plan_route_req plan_route_sel:r_plan_route_sel  ///
        plan_route_fallback:r_plan_route_fallback                        ///
        plan_route_error:r_plan_route_error plan_rhs:r_plan_rhs          ///
        plan_full_dim:r_plan_full_dim batch_lev_sel:r_batch_lev_sel      ///
        batch_tgt_sel:r_batch_tgt_sel batch_command:r_batch_command      ///
        mem_command:r_mem_command ctr_complete:r_ctr_complete            ///
        plan_res_rng_hi:r_pre_rng_hi plan_res_rng_lo:r_pre_rng_lo        ///
        wall_req_app:r_wall_req_app wall_requested:r_wall_requested {
        gettoken source target : pair, parse(":")
        gettoken colon target : target, parse(":")
        local `target' = r(`source')
    }

    local ok = 1
    local detail
    local expected_rhs_rows = 1 + 3*`probes_expected'
    local expected_parameters = `workers' + `firms' - 1
    local expected_full_tolerance = max(1e-11,10*`tolerance')
    local roundoff_gate = 4096*c(epsdouble)
    local reciprocal_gate = max(1e-10,100*`rank_tolerance')

    foreach value in probes_expected seed_expected maxiter tolerance workers firms ///
        rank_tolerance block_tolerance nuisance_code route_requested           ///
        fallback_allowed batch_mode leverage_batch_requested                   ///
        target_batch_requested target_mode deletion_source frequency_use       ///
        memory_limit input_copy preparation_peak prepared_resident             ///
        signature_hi signature_lo physical_limit wall_supplied wall_requested {
        if missing(``value'') {
            local ok = 0
            if `"`detail'"' == "" local detail                         ///
                "missing expected compressed-V7 argument `value'"
        }
    }
    if `ok' & (`probes_expected'<2 | `probes_expected'!=floor(`probes_expected') | ///
        `seed_expected'<0 | `seed_expected'!=floor(`seed_expected') |       ///
        `maxiter'<=0 | `maxiter'!=floor(`maxiter') | `tolerance'<=0 |       ///
        `workers'<=0 | `workers'!=floor(`workers') | `firms'<=1 |          ///
        `firms'!=floor(`firms') | `rank_tolerance'<=0 |                    ///
        `block_tolerance'<=0 | `memory_limit'<=0 |                         ///
        `memory_limit'!=floor(`memory_limit') | `input_copy'<0 |           ///
        `input_copy'!=floor(`input_copy') | `preparation_peak'<0 |         ///
        `preparation_peak'!=floor(`preparation_peak') |                    ///
        `prepared_resident'<=0 | `prepared_resident'!=floor(`prepared_resident') | ///
        `signature_hi'<0 | `signature_hi'>4294967295 |                    ///
        `signature_hi'!=floor(`signature_hi') | `signature_lo'<0 |        ///
        `signature_lo'>4294967295 | `signature_lo'!=floor(`signature_lo') | ///
        `physical_limit'<=0 | `physical_limit'>9007199254740992 |          ///
        `physical_limit'!=floor(`physical_limit')) {
        local ok = 0
        local detail "invalid expected compressed-V7 numerical argument"
    }
    if `ok' & (!inlist(`nuisance_code',1,2) |                       ///
        !inlist(`route_requested',0,2,3) |                          ///
        !inlist(`fallback_allowed',0,1) |                           ///
        (`fallback_allowed'==1 & `route_requested'!=0) |            ///
        !inlist(`batch_mode',0,1) |                                 ///
        !inlist(`target_mode',0,1) | !inlist(`deletion_source',1,2) | ///
        !inlist(`frequency_use',0,1) | !inlist(`wall_supplied',0,1)) {
        local ok = 0
        local detail "invalid expected compressed-V7 semantic tuple"
    }
    if `ok' & ((`batch_mode'==0 &                                  ///
            (`leverage_batch_requested'!=0 | `target_batch_requested'!=0)) | ///
        (`batch_mode'==1 &                                         ///
            (`leverage_batch_requested'<1 |                        ///
             `leverage_batch_requested'>`probes_expected' |        ///
             `target_batch_requested'<1 |                          ///
             `target_batch_requested'>`probes_expected' |          ///
             `leverage_batch_requested'!=floor(`leverage_batch_requested') | ///
             `target_batch_requested'!=floor(`target_batch_requested')))) {
        local ok = 0
        local detail "invalid expected compressed-V7 batch tuple"
    }
    if `ok' & ((`wall_supplied'==0 & `wall_requested'!=0) |         ///
        (`wall_supplied'==1 & `wall_requested'<=0)) {
        local ok = 0
        local detail "invalid expected compressed-V7 wall tuple"
    }

    if `ok' & (rowsof(`raw_results')!=4 | colsof(`raw_results')!=4 | ///
        rowsof(`rhs_native')!=`expected_rhs_rows' | colsof(`rhs_native')!=8) {
        local ok = 0
        local detail "compressed result or RHS-V1 matrix dimensions disagreed with the request"
    }

    local accounting_truth = 0
    if `ok' {
        forvalues row = 1/4 {
            forvalues col = 1/4 {
                if missing(`raw_results'[`row',`col']) local ok = 0
            }
        }
        forvalues row = 1/3 {
            local component_scale = max(1,abs(`raw_results'[`row',1]), ///
                abs(`raw_results'[`row',2]),abs(`raw_results'[`row',3]), ///
                abs(`raw_results'[`row',4]))
            local identity = abs(`raw_results'[`row',4] -              ///
                `raw_results'[`row',1] - `raw_results'[`row',2] -     ///
                2*`raw_results'[`row',3])/`component_scale'
            local accounting_truth = max(`accounting_truth',`identity')
            if `identity' > `roundoff_gate' local ok = 0
        }
        forvalues col = 1/4 {
            local correction_scale = max(1,abs(`raw_results'[1,`col']), ///
                abs(`raw_results'[2,`col']),abs(`raw_results'[3,`col']))
            if abs(`raw_results'[1,`col']-`raw_results'[2,`col']-     ///
                `raw_results'[3,`col']) > `roundoff_gate'*`correction_scale' ///
                local ok = 0
            if `raw_results'[4,`col'] < 0 local ok = 0
        }
        if !`ok' & `"`detail'"' == "" local detail                  ///
            "compressed public result accounting identities failed"
    }

    local rhs_max_iterations = 0
    local rhs_max_reduced = 0
    local rhs_max_complete = 0
    if `ok' {
        forvalues row = 1/`expected_rhs_rows' {
            forvalues col = 1/8 {
                if missing(`rhs_native'[`row',`col']) local ok = 0
            }
            local phase = `rhs_native'[`row',1]
            local probe = `rhs_native'[`row',2]
            local side = `rhs_native'[`row',3]
            local route = `rhs_native'[`row',4]
            local iterations = `rhs_native'[`row',5]
            local reduced = `rhs_native'[`row',6]
            local complete = `rhs_native'[`row',7]
            local zero_rhs = `rhs_native'[`row',8]
            local expected_phase = cond(`row'==1,1,                    ///
                cond(`row'<=1+`probes_expected',2,3))
            local expected_probe = cond(`row'==1,-1,                  ///
                cond(`expected_phase'==2,`row'-2,                     ///
                    floor((`row'-2-`probes_expected')/2)))
            local expected_side = cond(`expected_phase'==3,           ///
                mod(`row'-2-`probes_expected',2)+1,0)
            if `phase'!=floor(`phase') | `probe'!=floor(`probe') |    ///
                `side'!=floor(`side') | `route'!=floor(`route') |    ///
                `iterations'!=floor(`iterations') |                  ///
                `zero_rhs'!=floor(`zero_rhs') |                      ///
                `phase'!=`expected_phase' | `probe'!=`expected_probe' | ///
                `side'!=`expected_side' | `route'!=`r_route_sel' |   ///
                `iterations'<0 | `iterations'>`maxiter' |            ///
                `reduced'<0 | `reduced'>`tolerance' |                ///
                `complete'<0 | `complete'>`expected_full_tolerance' | ///
                !inlist(`zero_rhs',0,1) local ok = 0
            if `zero_rhs' & (`iterations'!=0 | `reduced'!=0 | `complete'!=0) ///
                local ok = 0
            local rhs_max_iterations = max(`rhs_max_iterations',`iterations')
            local rhs_max_reduced = max(`rhs_max_reduced',`reduced')
            local rhs_max_complete = max(`rhs_max_complete',`complete')
        }
        if !`ok' & `"`detail'"' == "" local detail                  ///
            "compressed RHS-V1 ordering, routing, or residual certificate failed"
    }

    if `ok' & `"`receipt_schema'"' != "VCKSS-EXECUTION-PLAN-V1" {
        local ok = 0
        local detail "compressed result did not carry the V7 execution-plan schema"
    }

    if `ok' {
        local ok = `r_seed'==`seed_expected' & `r_probes'==`probes_expected' & ///
            `r_lev_accepted'==`probes_expected' &                         ///
            `r_tgt_accepted'==`probes_expected' &                         ///
            `r_alg_req'==2 & `r_alg_sel'==2 & `r_eng_req'==0 & `r_eng_sel'==1 & ///
            `r_route_req'==`route_requested' & inlist(`r_route_sel',2,3) & ///
            (`route_requested'==0 | `r_route_sel'==`route_requested') &   ///
            inlist(`r_fallback',0,1) &                                   ///
            (`fallback_allowed' | `r_fallback'==0) &                     ///
            (`r_fallback' | `r_fallback_error'==0) &                     ///
            `r_fallback'==`r_plan_route_fallback' &                      ///
            `r_fallback_error'==`r_plan_route_error' &                   ///
            `r_dimension'==`firms'-1 & `r_rng'==1 &                     ///
            `r_rhs_rows'==`expected_rhs_rows' & `r_rhs_schema'==1 &      ///
            `r_result_copy'==112*`expected_rhs_rows' & `r_rhs_v2_copy'==0 & ///
            `r_full_route'==`r_route_sel' &                              ///
            `r_full_iter'>=0 & `r_full_iter'<=`maxiter' &                ///
            inlist(`r_full_zero',0,1) &                                  ///
            `r_full_reduced'>=0 & `r_full_reduced'<=`tolerance' &        ///
            `r_full_complete'>=0 &                                      ///
            `r_full_complete'<=`expected_full_tolerance' &               ///
            `r_lev_rhs'==`probes_expected' &                             ///
            `r_tgt_rhs'==2*`probes_expected' &                           ///
            abs(`r_max_reduced'-`rhs_max_reduced')<=                     ///
                `roundoff_gate'*max(1,abs(`r_max_reduced')) &            ///
            abs(`r_max_complete'-`rhs_max_complete')<=                   ///
                `roundoff_gate'*max(1,abs(`r_max_complete')) &           ///
            `r_max_reduced'>=0 & `r_max_reduced'<=`tolerance' &          ///
            `r_max_complete'>=0 &                                       ///
            `r_max_complete'<=`expected_full_tolerance' &                ///
            `r_max_leverage'>=0 & `r_max_leverage'<1 &                   ///
            `r_max_reciprocal'>=0 & `r_max_reciprocal'<=`reciprocal_gate' & ///
            `r_rank_tolerance'==`rank_tolerance' &                       ///
            `r_block_tolerance'==`block_tolerance' &                     ///
            `r_full_tolerance'==`expected_full_tolerance' &              ///
            `r_deletion'==1 & `r_nuisance'==`nuisance_code' &           ///
            `r_parameters'==`expected_parameters' &                      ///
            `r_full_parameters'==`expected_parameters' &                 ///
            `r_correction_parameters'==`expected_parameters' &           ///
            `r_topology_hi'>=0 & `r_topology_hi'<=4294967295 &          ///
            `r_topology_hi'==floor(`r_topology_hi') &                    ///
            `r_topology_lo'>=0 & `r_topology_lo'<=4294967295 &          ///
            `r_topology_lo'==floor(`r_topology_lo') &                    ///
            `r_cap_schema'==3 & `r_cap_profile'==4 &                     ///
            `r_signature_hi'==`signature_hi' & `r_signature_lo'==`signature_lo' & ///
            `r_batch_mode'==`batch_mode' &                               ///
            `r_lev_batch_mode'==`batch_mode' & `r_tgt_batch_mode'==`batch_mode' & ///
            `r_stayers_mode'==1 & `r_target_mode'==`target_mode' &       ///
            `r_deletion_source'==`deletion_source' & `r_probeorder'==0 & ///
            `r_wall_supplied'==`wall_supplied' &                         ///
            `r_frequency'==`frequency_use' &                             ///
            `r_physical_limit'==`physical_limit' &                       ///
            `r_accounting'>=0 & `r_accounting'<=`roundoff_gate' &        ///
            `r_actual_accounting'>=0 &                                  ///
            abs(`r_actual_accounting'-`accounting_truth')<=              ///
                `roundoff_gate'*max(1,abs(`accounting_truth')) &         ///
            abs(`r_accounting'-`r_actual_accounting')<=                  ///
                `roundoff_gate'*max(1,abs(`r_actual_accounting')) &      ///
            `r_weighted_rss'>=0
        if !`ok' local detail "compressed V4/V6 result prefix did not reconcile with the request"
    }

    if `ok' {
        local ok = `r_full_iter'==`rhs_native'[1,5] &                    ///
            `r_full_zero'==`rhs_native'[1,8] &                          ///
            abs(`r_full_reduced'-`rhs_native'[1,6])<=                   ///
                `roundoff_gate'*max(1,abs(`r_full_reduced')) &          ///
            abs(`r_full_complete'-`rhs_native'[1,7])<=                  ///
                `roundoff_gate'*max(1,abs(`r_full_complete'))
        if !`ok' local detail "compressed full-fit receipt disagreed with RHS-V1 row one"
    }

    if `ok' {
        local ok = `r_mem_limit'==`memory_limit' &                       ///
            `r_input_copy'==`input_copy' & `r_prep_peak'==`preparation_peak' & ///
            `r_resident'==`prepared_resident' & `r_solver_setup'>0 &    ///
            `r_lev_phase'>0 & `r_tgt_phase'>0 &                         ///
            `r_result_bytes'>=`r_result_copy' &                         ///
            `r_solve_peak'>=`r_resident'+`r_solver_setup'+              ///
                `r_result_bytes'+max(`r_lev_phase',`r_tgt_phase') &     ///
            `r_command_peak'==max(`r_prep_peak',`r_solve_peak') &       ///
            `r_command_peak'<=`r_mem_limit'
        if !`ok' local detail "compressed memory V6 receipt did not reconcile"
    }

    if `ok' {
        local ok = `r_plan_struct'==1000 & `r_plan_schema'==1 &          ///
            `r_plan_route_schema'==2 & `r_plan_resolved'==1 &           ///
            `r_plan_frozen'==1 & `r_plan_applicability'==2 &            ///
            `r_plan_alg_req'==2 & `r_plan_alg_sel'==2 &                 ///
            `r_plan_eng_req'==0 & `r_plan_eng_sel'==1 &                 ///
            `r_plan_route_req'==`route_requested' &                     ///
            `r_plan_route_sel'==`r_route_sel' &                         ///
            `r_plan_rhs'==`expected_rhs_rows' &                         ///
            `r_plan_full_dim'==`firms'-1 &                              ///
            `r_batch_lev_sel'==`r_lev_batch' &                          ///
            `r_batch_tgt_sel'==`r_tgt_batch' &                          ///
            `r_batch_command'==`r_mem_command' &                        ///
            `r_mem_command'==`r_solve_peak' & `r_ctr_complete'==1 &     ///
            `r_pre_rng_hi'==0 & `r_pre_rng_lo'==0 &                     ///
            `r_wall_req_app'==`wall_supplied' &                         ///
            `r_wall_requested'==`wall_requested'
        if !`ok' local detail "compressed V7 execution plan did not reconcile"
    }

    if `ok' & `batch_mode'==1 {
        if `r_lev_batch'!=`leverage_batch_requested' |                  ///
            `r_tgt_batch'!=`target_batch_requested' {
            local ok = 0
            local detail "explicit compressed phase batch widths changed after plan freeze"
        }
    }
    if `ok' & `batch_mode'==0 {
        if `r_lev_batch'<1 | `r_lev_batch'>`probes_expected' |          ///
            `r_tgt_batch'<1 | `r_tgt_batch'>`probes_expected' {
            local ok = 0
            local detail "automatic compressed phase batch width was out of range"
        }
    }

    return scalar ok = `ok'
    return local detail `"`detail'"'
    return scalar expected_rhs_rows = `expected_rhs_rows'
    return scalar rhs_max_iterations = `rhs_max_iterations'
    return scalar rhs_max_reduced = `rhs_max_reduced'
    return scalar rhs_max_complete = `rhs_max_complete'
    return scalar accounting_truth = `accounting_truth'
    return scalar selected_route = `r_route_sel'
    return scalar selected_engine = `r_eng_sel'
    return scalar leverage_batch = `r_lev_batch'
    return scalar target_batch = `r_tgt_batch'
    return scalar max_reciprocal_residual = `r_max_reciprocal'
    return matrix result = `raw_results'
    return matrix rhs_receipts = `rhs_native'
end

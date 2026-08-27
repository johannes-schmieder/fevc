*! version 0.4.0-alpha.1 24aug2026
program define _vckss_rust_post_exact_v7, eclass sortpreserve
    version 18.0
    args handle depvar frequency target touse nscope ncomplete nstayers ///
        nstayerrows probesrequested batchnumeric seedrequested         ///
        tolerancerequested maxiterrequested memorygib algorithmrequested ///
        enginerequested backendsupplied rngsupplied rngrequested       ///
        deletionidsupplied enginesupplied algorithmsupplied            ///
        preconditionsupplied batchsupplied stayerssupplied             ///
        rustcoreflags rustsupportflags nodisplay deletionmode nuisance ///
        ranktol blocktol exactlimit physicallimit preconditionerrequested ///
        batchrequested targetweightsupplied cmdline wallsecondssupplied ///
        wallseconds prepctx graphctx capctx stayersmode

    foreach input in `depvar' `frequency' `target' `touse' {
        confirm numeric variable `input'
    }
    tempname prep graph cap
    matrix `prep' = `prepctx'
    matrix `graph' = `graphctx'
    matrix `cap' = `capctx'
    local context_ok = rowsof(`prep')==1 & colsof(`prep')==13 &       ///
        rowsof(`graph')==1 & colsof(`graph')==18 &                    ///
        rowsof(`cap')==1 & colsof(`cap')==33
    if !`context_ok' {
        capture quietly _vckss_rust_public_call release `handle'
        capture quietly vckss_rust clear
        quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"       ///
            "The planned exact poster received an invalid context shape."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "exact_context"
        exit 498
    }

    local p_input       = `prep'[1,1]
    local p_retained    = `prep'[1,2]
    local p_workers     = `prep'[1,3]
    local p_firms       = `prep'[1,4]
    local p_cells       = `prep'[1,5]
    local p_units       = `prep'[1,6]
    local p_strata      = `prep'[1,7]
    local p_target      = `prep'[1,8]
    local p_controls    = `prep'[1,9]
    local p_mem_limit   = `prep'[1,10]
    local p_input_copy  = `prep'[1,11]
    local p_prep_peak   = `prep'[1,12]
    local p_resident    = `prep'[1,13]

    local g_input_rows      = `graph'[1,1]
    local g_keep_rows       = `graph'[1,2]
    local g_input_mass      = `graph'[1,3]
    local g_keep_mass       = `graph'[1,4]
    local g_init_comp       = `graph'[1,5]
    local g_max_comp        = `graph'[1,6]
    local g_init_rows       = `graph'[1,7]
    local g_mover_rows      = `graph'[1,8]
    local g_init_edges      = `graph'[1,9]
    local g_keep_edges      = `graph'[1,10]
    local g_degree_removed  = `graph'[1,11]
    local g_art_removed     = `graph'[1,12]
    local g_bridge_units    = `graph'[1,13]
    local g_bridge_rows     = `graph'[1,14]
    local g_degree_iters    = `graph'[1,15]
    local g_art_iters       = `graph'[1,16]
    local g_bridge_iters    = `graph'[1,17]
    local g_fixed_iters     = `graph'[1,18]

    local capstruct       = `cap'[1,1]
    local capabi          = `cap'[1,2]
    local capschema       = `cap'[1,3]
    local capsupported    = `cap'[1,4]
    local capreason       = `cap'[1,5]
    local capprofile      = `cap'[1,6]
    local capalgorithm    = `cap'[1,7]
    local capdeletion     = `cap'[1,8]
    local capnuisance     = `cap'[1,9]
    local caproute        = `cap'[1,10]
    local caprng          = `cap'[1,11]
    local capcontrols     = `cap'[1,12]
    local capfrequency    = `cap'[1,13]
    local capengine       = `cap'[1,14]
    local capbatch        = `cap'[1,15]
    local capstayers      = `cap'[1,16]
    local captarget       = `cap'[1,17]
    local capdelsource    = `cap'[1,18]
    local capprobeorder   = `cap'[1,19]
    local capwallsup      = `cap'[1,20]
    local capphysical     = `cap'[1,21]
    local capsignaturehi  = `cap'[1,22]
    local capsignaturelo  = `cap'[1,23]
    local caplevmode      = `cap'[1,24]
    local captgtmode      = `cap'[1,25]
    local capfallback     = `cap'[1,26]
    local capalgdefer     = `cap'[1,27]
    local capengdefer     = `cap'[1,28]
    local caproutedefer   = `cap'[1,29]
    local caplevdefer     = `cap'[1,30]
    local captgtdefer     = `cap'[1,31]
    local capwalladvisory = `cap'[1,32]
    local capwallseconds  = `cap'[1,33]

    local algorithmrequested = lower(strtrim("`algorithmrequested'"))
    local enginerequested = lower(strtrim("`enginerequested'"))
    local preconditionerrequested = lower(strtrim("`preconditionerrequested'"))
    local batchrequested = lower(strtrim("`batchrequested'"))
    local deletionmode = lower(strtrim("`deletionmode'"))
    local nuisance = lower(strtrim("`nuisance'"))
    local stayersmode = lower(strtrim("`stayersmode'"))
    if "`stayersmode'"=="" local stayersmode movers
    local algreq = cond("`algorithmrequested'"=="auto",0,           ///
        cond("`algorithmrequested'"=="exact",1,.))
    local engreq = cond("`enginerequested'"=="auto",0,               ///
        cond("`enginerequested'"=="generic",2,.))
    local delcode = cond("`deletionmode'"=="match",1,               ///
        cond("`deletionmode'"=="observation",2,.))
    local nuiscode = cond("`nuisance'"=="joint",1,                  ///
        cond("`nuisance'"=="fixedoffset",2,.))
    local wallsup = real("`wallsecondssupplied'")
    local wallvalue = real("`wallseconds'")
    local expected_batch_mode = cond("`batchrequested'"=="auto",0,1)
    local expected_stayers_mode = cond("`stayersmode'"=="both",2,   ///
        cond("`stayersmode'"=="movers",1,.))
    local expected_algorithm_deferred = (`algreq'==0)
    local expected_engine_deferred = (`algreq'==0)
    local expected_route_deferred = (`algreq'==0)
    local expected_capability_rng = (`algreq'==0)
    local expected_input_copy = `p_input'*(6+`p_controls')*8
    local expected_prep_peak = `expected_input_copy'+`p_input'*768+ ///
        `p_input'*`p_controls'*32+4096
    local tuple_ok = !missing(`algreq') & !missing(`engreq') &       ///
        !missing(`delcode') & !missing(`nuiscode') &                 ///
        !missing(`expected_stayers_mode') &                          ///
        `exactlimit'>=2 & `exactlimit'<=2000 &                       ///
        `exactlimit'==floor(`exactlimit') &                          ///
        "`preconditionerrequested'"=="auto" &                       ///
        inlist("`batchrequested'","auto","`batchnumeric'") &      ///
        inlist(`wallsup',0,1) &                                     ///
        ((`wallsup'==0 & `wallvalue'==0) |                          ///
            (`wallsup'==1 & `wallvalue'>0))
    if !`tuple_ok' {
        capture quietly _vckss_rust_public_call release `handle'
        capture quietly vckss_rust clear
        quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"       ///
            "The planned exact poster received an invalid expected tuple."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "exact_tuple"
        exit 498
    }

    capture noisily _vckss_rust_public_call result `handle'
    if _rc {
        local failure_rc = _rc
        capture noisily _vckss_rust_abort, rc(`failure_rc')          ///
            handle(`handle') phase(exact_result_export)
        exit _rc
    }
    capture quietly _vckss_rust_reconcile_exact_v7 `algreq' `engreq' ///
        `delcode' `nuiscode' `p_workers' `p_firms' `p_controls'      ///
        `ranktol' `blocktol' `tolerancerequested' `exactlimit'       ///
        `p_mem_limit'                                                 ///
        `p_input_copy' `p_prep_peak' `p_resident' `capsignaturehi'   ///
        `capsignaturelo' `physicallimit' `wallsup' `wallvalue'       ///
        `captarget' `capdelsource' `capfrequency' `expected_stayers_mode'
    local reconcile_rc = _rc
    if `reconcile_rc' {
        capture quietly _vckss_rust_public_call release `handle'
        capture quietly vckss_rust clear
        quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"       ///
            "The planned exact V7 reconciler was unavailable."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "exact_reconcile"
        exit 498
    }
    local reconcile_ok = r(ok)
    local reconcile_detail `"`r(detail)'"'
    tempname validated_results
    matrix `validated_results' = r(result)
    if `reconcile_ok'!=1 | `"`r(result_family)'"'!="exact" |        ///
        `"`r(execution_plan_schema)'"'!="VCKSS-EXECUTION-PLAN-V1" {
        capture quietly _vckss_rust_public_call release `handle'
        capture quietly vckss_rust clear
        quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"       ///
            "Planned exact V7 reconciliation failed: `reconcile_detail'."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "exact_reconcile"
        exit 498
    }

    capture noisily _vckss_rust_public_call result `handle'
    if _rc {
        local failure_rc = _rc
        capture noisily _vckss_rust_abort, rc(`failure_rc')          ///
            handle(`handle') phase(exact_result_reexport)
        exit _rc
    }
    tempname raw_results
    matrix `raw_results' = r(result)
    if mreldif(`raw_results',`validated_results') != 0 {
        capture quietly _vckss_rust_public_call release `handle'
        capture quietly vckss_rust clear
        quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"       ///
            "The immutable exact generation changed between reconciliation and posting."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "exact_reexport"
        exit 498
    }

    foreach pair in seed:r_seed probes:r_probes                       ///
        leverage_probes_accepted:r_lev_acc target_probes_accepted:r_tgt_acc ///
        requested_route:r_req_route selected_route:r_sel_route       ///
        solver_fallback:r_fallback solver_fallback_error:r_fallback_err ///
        solver_dimension:r_dimension leverage_batch_width:r_lev_batch ///
        target_batch_width:r_tgt_batch rank_tolerance:r_rank_tol      ///
        block_tolerance:r_block_tol full_residual_tolerance:r_full_tol ///
        full_fit_route:r_full_route full_fit_iterations:r_full_iter  ///
        full_fit_reduced_residual:r_full_red                         ///
        full_fit_complete_residual:r_full_complete                   ///
        full_fit_zero_rhs:r_full_zero leverage_rhs_count:r_lev_rhs  ///
        target_rhs_count:r_tgt_rhs max_reduced_residual:r_max_red   ///
        max_complete_residual:r_max_complete max_leverage:r_max_lev ///
        max_reciprocal_residual:r_max_recip accounting_residual:r_accounting ///
        topology_checksum_hi:r_top_hi topology_checksum_lo:r_top_lo ///
        rng_contract_code:r_rng rhs_receipt_rows:r_rhs_rows          ///
        caller_result_copy_bytes:r_rhs_copy                         ///
        requested_algorithm_code:r_algorithm_req                    ///
        selected_algorithm_code:r_algorithm_sel                     ///
        requested_engine_code:r_engine_req selected_engine_code:r_engine_sel ///
        deletion_mode_code:r_deletion nuisance_mode_code:r_nuisance ///
        parameters:r_parameters full_parameters:r_full_parameters   ///
        correction_parameters:r_correction_parameters              ///
        information_rcond:r_info_rcond inverse_relative_residual:r_inverse_relres ///
        exact_peak_forecast_bytes:r_exact_peak exact_diagnostic_flags:r_exact_flags ///
        working_fit_complete_residual:r_working_fit                 ///
        inverse_sqrt_relative_residual:r_inverse_sqrt               ///
        maker_relative_residual:r_maker                             ///
        control_basis_relative_residual:r_control_relres            ///
        control_basis_forward_error:r_control_forward               ///
        deletion_rank_gap:r_rank_gap firm_zero_sum_residual:r_firm_zero ///
        fit_peak_forecast_bytes:r_fit_peak                          ///
        correction_peak_forecast_bytes:r_correction_peak            ///
        actual_accounting_residual:r_actual_accounting weighted_rss:r_rss ///
        memory_limit_bytes:r_mem_limit caller_copy_bytes:r_input_copy ///
        preparation_peak_forecast_bytes:r_prep_peak                 ///
        prepared_resident_bytes:r_resident                          ///
        solver_setup_forecast_bytes:r_solver_setup                  ///
        leverage_phase_forecast_bytes:r_lev_phase                   ///
        target_phase_forecast_bytes:r_tgt_phase                     ///
        result_forecast_bytes:r_result_bytes                        ///
        solve_peak_forecast_bytes:r_solve_peak                      ///
        command_peak_forecast_bytes:r_command_peak                  ///
        capability_schema:r_cap_schema capability_profile:r_cap_profile ///
        batch_mode_code:r_batch_mode stayers_mode_code:r_stayers_mode ///
        target_weight_mode_code:r_target_mode deletion_source_code:r_deletion_source ///
        probeorder_supplied:r_probeorder wallseconds_supplied:r_wallseconds ///
        frequency_use_code:r_frequency physical_limit:r_physical_limit ///
        request_signature_hi:r_signature_hi request_signature_lo:r_signature_lo ///
        generic_controls_count:r_result_controls                    ///
        batch_lev_mode:r_lev_batch_mode batch_tgt_mode:r_tgt_batch_mode ///
        plan_struct:r_plan_struct plan_schema:r_plan_schema          ///
        plan_route_schema:r_plan_route_schema plan_resolved:r_plan_resolved ///
        plan_frozen:r_plan_frozen plan_applicability:r_plan_applicability ///
        plan_alg_req:r_plan_alg_req plan_alg_sel:r_plan_alg_sel      ///
        plan_alg_reason:r_plan_alg_reason                            ///
        plan_eng_req:r_plan_eng_req plan_eng_sel:r_plan_eng_sel      ///
        plan_eng_reason:r_plan_eng_reason                            ///
        plan_comp_elig:r_plan_comp_elig                              ///
        plan_complexity:r_plan_complexity                            ///
        plan_exact_limit:r_plan_exact_limit                          ///
        plan_route_req:r_plan_route_req plan_route_sel:r_plan_route_sel ///
        plan_route_fallback:r_plan_route_fallback                    ///
        plan_route_error:r_plan_route_error plan_rhs:r_plan_rhs      ///
        plan_full_dim:r_plan_full_dim batch_lev_sel:r_plan_lev_batch ///
        batch_tgt_sel:r_plan_tgt_batch mem_command:r_plan_mem_command ///
        ctr_complete:r_ctr_complete plan_res_rng_hi:r_pre_rng_hi     ///
        plan_res_rng_lo:r_pre_rng_lo wall_req_app:r_wall_req_app     ///
        wall_requested:r_wall_requested wall_forecast:r_wall_forecast ///
        wall_advisory:r_wall_advisory wall_margin:r_wall_margin {
        gettoken returned localname : pair, parse(":")
        local localname = substr("`localname'",2,.)
        local `localname' = r(`returned')
    }

    quietly count if `touse'
    local retained_count = r(N)
    quietly summarize `frequency' if `touse', meanonly
    local retained_physical = r(sum)
    quietly summarize `target' if `touse', meanonly
    local retained_target = r(sum)
    local context_ok = `p_input'>0 & `p_retained'>0 & `p_workers'>0 & ///
        `p_firms'>1 & `p_cells'>0 & `p_units'>0 & `p_strata'>0 &    ///
        `p_controls'>=0 & `p_input'==`ncomplete' &                  ///
        `p_retained'==`retained_count' &                            ///
        `p_input_copy'==`expected_input_copy' &                      ///
        `p_prep_peak'==`expected_prep_peak' & `p_resident'>0 &      ///
        `p_target'>0 & `retained_target'>0 &                        ///
        abs(`p_target'-`retained_target')<=1e-10*max(1,abs(`p_target')) & ///
        `g_input_rows'==`ncomplete' & `g_keep_rows'==`retained_count' & ///
        `g_keep_mass'==`retained_physical' & `g_input_mass'>=`g_keep_mass' & ///
        `g_init_comp'>0 & `g_max_comp'>=`g_init_comp' &             ///
        `g_init_rows'>=`g_mover_rows' & `g_mover_rows'>=`g_keep_rows' & ///
        `g_init_edges'>=`g_keep_edges' &                            ///
        `g_degree_iters'<=`g_degree_removed' &                      ///
        `g_art_iters'<=`g_art_removed' & `g_bridge_iters'<=`g_bridge_units' & ///
        `g_fixed_iters'==`g_degree_iters'+`g_art_iters'+`g_bridge_iters' & ///
        `capstruct'==160 & `capabi'==1 & `capschema'==3 &            ///
        `capsupported'==1 & `capreason'==0 & `capprofile'==4 &       ///
        `capalgorithm'==`algreq' & `capdeletion'==`delcode' &       ///
        `capnuisance'==`nuiscode' & `caproute'==0 &                ///
        `caprng'==`expected_capability_rng' &                       ///
        `capcontrols'==`p_controls' & `capfrequency'==`r_frequency' & ///
        `capengine'==`engreq' & `capbatch'==`expected_batch_mode' &  ///
        `capstayers'==`expected_stayers_mode' &                      ///
        `captarget'==`r_target_mode' &                               ///
        `capdelsource'==`r_deletion_source' & `capprobeorder'==0 &   ///
        `capwallsup'==`wallsup' & `capphysical'==`physicallimit' &   ///
        `capsignaturehi'==`r_signature_hi' &                         ///
        `capsignaturelo'==`r_signature_lo' &                         ///
        `caplevmode'==`expected_batch_mode' &                        ///
        `captgtmode'==`expected_batch_mode' & `capfallback'==1 &     ///
        `capalgdefer'==`expected_algorithm_deferred' &               ///
        `capengdefer'==`expected_engine_deferred' &                 ///
        `caproutedefer'==`expected_route_deferred' &                ///
        `caplevdefer'==("`batchrequested'"=="auto") &               ///
        `captgtdefer'==("`batchrequested'"=="auto") &               ///
        `capwalladvisory'==1 & `capwallseconds'==`wallvalue' &       ///
        `r_cap_schema'==3 & `r_cap_profile'==4 &                    ///
        `r_algorithm_req'==`algreq' & `r_algorithm_sel'==1 &        ///
        `r_engine_req'==`engreq' & `r_engine_sel'==3 &              ///
        `r_result_controls'==`p_controls' & `r_rhs_rows'==0 &       ///
        `r_rhs_copy'==0 & `r_solve_peak'==`r_plan_mem_command' &    ///
        `r_plan_applicability'==1 & `r_plan_resolved'==1 &          ///
        `r_plan_frozen'==1 &                                       ///
        `r_plan_alg_reason'==cond(`algreq'==0,3,1) &                ///
        `r_plan_eng_reason'==1 & `r_plan_comp_elig'==0 &            ///
        `r_plan_complexity'==`p_workers'+`p_firms'-1+`p_controls' & ///
        `r_plan_exact_limit'==`exactlimit' &                         ///
        `r_ctr_complete'==1 &                                       ///
        `r_pre_rng_hi'==0 & `r_pre_rng_lo'==0 &                     ///
        `nscope'>=`ncomplete' & `ncomplete'>=`retained_count' &     ///
        `nstayers'>=0 & `nstayerrows'>=0
    if `context_ok' & `delcode'==1 {
        local context_ok = `g_keep_edges'==`p_units' &               ///
            (`g_bridge_iters'==0)==(`g_bridge_units'==0) &           ///
            (`g_bridge_iters'==0)==(`g_bridge_rows'==0)
    }
    if `context_ok' & `delcode'==2 {
        local context_ok = `p_units'==`retained_physical' &          ///
            `g_bridge_units'==0 & `g_bridge_rows'==0 & `g_bridge_iters'==0
    }
    if !`context_ok' {
        capture quietly _vckss_rust_public_call release `handle'
        capture quietly vckss_rust clear
        quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"       ///
            "The planned exact public context did not reconcile with the validated V7 result."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "exact_context_reconcile"
        exit 498
    }

    capture quietly _vckss_rust_public_call release `handle'
    if _rc {
        local failure_rc = _rc
        capture noisily _vckss_rust_abort, rc(`failure_rc')          ///
            handle(`handle') phase(release) norelease
        exit _rc
    }
    capture quietly vckss_rust snapshot
    if _rc | r(state)!=0 | r(handle)!=0 {
        capture quietly vckss_rust clear
        quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"       ///
            "The planned exact release did not return the native engine to idle state."
        exit 498
    }

    tempname plugin correction corrected kss_return mcse decomposition
    matrix colnames `raw_results' = worker_variance firm_variance    ///
        worker_firm_covariance total_variance
    matrix rownames `raw_results' = plugin bias_correction corrected ///
        numerical_mcse
    matrix `plugin' = `raw_results'[1,1..4]
    matrix `correction' = `raw_results'[2,1..4]
    matrix `corrected' = `raw_results'[3,1..4]
    matrix `kss_return' = `corrected'
    matrix `mcse' = `raw_results'[4,1..4]
    foreach matrix_name in plugin correction corrected kss_return mcse {
        matrix colnames ``matrix_name'' = worker_variance firm_variance ///
            worker_firm_covariance total_variance
    }

    local target_outcome_variance = .
    local regression_outcome_variance = .
    tempvar target_sq frequency_sq
    quietly summarize `depvar' [aw=`target'] if `touse' & `target' > 0, meanonly
    if !_rc & !missing(r(mean)) {
        local target_mean = r(mean)
        quietly generate double `target_sq' =                       ///
            `target'*(`depvar'-`target_mean')^2 if `touse'
        quietly summarize `target_sq' if `touse', meanonly
        local target_outcome_variance = r(sum)/`retained_target'
    }
    quietly summarize `depvar' [aw=`frequency'] if `touse', meanonly
    if !_rc & !missing(r(mean)) {
        local frequency_mean = r(mean)
        quietly generate double `frequency_sq' =                    ///
            `frequency'*(`depvar'-`frequency_mean')^2 if `touse'
        quietly summarize `frequency_sq' if `touse', meanonly
        local regression_outcome_variance = r(sum)/`retained_physical'
    }
    local residual_variance = `r_rss'/`retained_physical'
    local explained_variance = `regression_outcome_variance'-`residual_variance'
    local explained_share = .
    if `regression_outcome_variance' > 0 {
        local explained_share = `explained_variance'/`regression_outcome_variance'
    }
    matrix `decomposition' =                                      ///
        (`raw_results'[1,1],`raw_results'[2,1],`raw_results'[3,1] \ ///
         `raw_results'[1,2],`raw_results'[2,2],`raw_results'[3,2] \ ///
         2*`raw_results'[1,3],2*`raw_results'[2,3],               ///
            2*`raw_results'[3,3] \                               ///
         `raw_results'[1,4],`raw_results'[2,4],`raw_results'[3,4])
    matrix `decomposition' = `decomposition', J(4,4,.)
    if `target_outcome_variance' > 0 {
        forvalues component = 1/4 {
            matrix `decomposition'[`component',4] =                 ///
                `decomposition'[`component',1]/`target_outcome_variance'
            matrix `decomposition'[`component',5] =                 ///
                `decomposition'[`component',3]/`target_outcome_variance'
        }
    }
    if `decomposition'[4,1] > 0 {
        forvalues component = 1/4 {
            matrix `decomposition'[`component',6] =                 ///
                `decomposition'[`component',1]/`decomposition'[4,1]
        }
    }
    if `decomposition'[4,3] > 0 {
        forvalues component = 1/4 {
            matrix `decomposition'[`component',7] =                 ///
                `decomposition'[`component',3]/`decomposition'[4,3]
        }
    }
    matrix rownames `decomposition' = worker_variance firm_variance ///
        sorting_2covariance total_worker_firm
    matrix colnames `decomposition' = plugin bias_correction corrected ///
        plugin_share_outcome corrected_share_outcome                ///
        plugin_share_worker_firm corrected_share_worker_firm

    tempname graph_receipt memory_receipt preparation_receipt       ///
        exact_memory_receipt capability_receipt
    matrix `graph_receipt' = (`g_input_rows',`g_keep_rows',          ///
        `g_input_mass',`g_keep_mass',`g_init_comp',`g_max_comp',     ///
        `g_init_rows',`g_mover_rows',`g_init_edges',`g_keep_edges',  ///
        `g_degree_removed',`g_art_removed',`g_bridge_units',         ///
        `g_bridge_rows',`g_degree_iters',`g_art_iters',              ///
        `g_bridge_iters',`g_fixed_iters')
    matrix colnames `graph_receipt' = input_rows retained_rows input_mass ///
        retained_mass initial_components maximum_components initial_component_rows ///
        mover_input_rows initial_deletion_edges retained_deletion_edges ///
        insufficient_workers_removed articulation_workers_removed    ///
        bridge_units_removed bridge_rows_removed degree_iterations   ///
        articulation_iterations bridge_iterations fixed_point_iterations
    matrix `memory_receipt' = (`r_mem_limit',`r_input_copy',0,       ///
        `r_prep_peak',`r_resident',0,0,0,`r_result_bytes',           ///
        `r_solve_peak',`r_command_peak')
    matrix colnames `memory_receipt' = limit caller_input_copy       ///
        caller_result_copy preparation_peak prepared_resident       ///
        solver_setup leverage_phase target_phase result solve_peak command_peak
    matrix `preparation_receipt' = (`p_input',`p_retained',          ///
        `p_workers',`p_firms',`p_cells',`p_units',`p_strata',`p_target')
    matrix colnames `preparation_receipt' = input_rows retained_rows ///
        workers firms cells deletion_units target_strata target_weight_sum
    matrix `exact_memory_receipt' = (`r_fit_peak',                   ///
        `r_correction_peak',`r_exact_peak',`r_prep_peak',            ///
        `r_command_peak',`r_mem_limit')
    matrix colnames `exact_memory_receipt' = fit correction exact_solve ///
        preparation command limit
    matrix `capability_receipt' = (`capstruct',`capabi',       ///
        `capschema',`capsupported',`capreason',`capprofile',             ///
        `capalgorithm',`capdeletion',`capnuisance',`caproute',`caprng',  ///
        `capcontrols',`capfrequency',`capengine',`capbatch',             ///
        `capstayers',`captarget',`capdelsource',`capprobeorder',         ///
        `capwallsup',`capphysical',`capsignaturehi',`capsignaturelo')
    matrix colnames `capability_receipt' = struct_size abi_version schema ///
        supported reason profile algorithm deletion nuisance route rng   ///
        controls frequency engine batch stayers target deletion_source  ///
        probeorder wall physical_limit signature_hi signature_lo

    tempname prep_boundary_counts
    local prep_deletion_groups = cond("`deletionmode'"=="observation",0,1)
    local exact_semantic_groups = (`p_controls'>0)
    matrix `prep_boundary_counts' = (2,`prep_deletion_groups',0,    ///
        `exact_semantic_groups',2,4,`p_input',2,`retained_count',0,0)
    matrix colnames `prep_boundary_counts' = initial_id_group_calls ///
        deletion_group_calls retained_id_group_calls semantic_group_calls ///
        stata_sort_calls graph_import_columns graph_import_rows     ///
        retained_map_columns retained_map_rows compression_import_columns ///
        compression_import_rows

    ereturn clear
    ereturn post `corrected', obs(`retained_physical') esample(`touse') ///
        depname(`depvar')
    ereturn matrix results = `raw_results'
    ereturn matrix plugin = `plugin'
    ereturn matrix correction = `correction'
    ereturn matrix kss = `kss_return'
    ereturn matrix numerical_mcse = `mcse'
    ereturn matrix decomposition = `decomposition'
    ereturn matrix rust_graph_receipt = `graph_receipt'
    ereturn matrix rust_memory_receipt = `memory_receipt'
    ereturn matrix rust_preparation_receipt = `preparation_receipt'
    ereturn matrix rust_exact_memory_receipt = `exact_memory_receipt'
    ereturn matrix rust_request_capability_receipt = `capability_receipt'
    ereturn matrix prep_boundary_counts = `prep_boundary_counts'
    ereturn local prep_boundary_counts_schema "PREP-BND-COUNTS-V1"
    ereturn scalar N_stored = `retained_count'
    ereturn scalar N_physical = `retained_physical'
    ereturn scalar N_requested = `nscope'
    ereturn scalar N_complete = `ncomplete'
    ereturn scalar N_retained = `retained_count'
    ereturn scalar N_mover_input = `g_mover_rows'
    ereturn scalar N_initial_component = `g_init_rows'
    ereturn scalar N_initial_component_dropped = `ncomplete'-`g_init_rows'
    ereturn scalar N_mover_dropped = `g_init_rows'-`g_mover_rows'
    ereturn scalar N_graph_dropped = `g_mover_rows'-`retained_count'
    ereturn scalar N_stayers = `nstayers'
    ereturn scalar N_stayer_rows = `nstayerrows'
    ereturn scalar worker_levels = `p_workers'
    ereturn scalar firm_levels = `p_firms'
    ereturn scalar parameters = `r_parameters'
    ereturn scalar full_parameters = `r_full_parameters'
    ereturn scalar correction_parameters = `r_correction_parameters'
    ereturn scalar controls_count = `p_controls'
    ereturn scalar coefficient_cells = `p_cells'
    ereturn scalar deletion_units = `p_units'
    ereturn scalar target_strata = `p_strata'
    ereturn scalar target_weight_sum = `p_target'
    ereturn scalar graph_edges = `g_init_edges'
    ereturn scalar graph_retained_edges = `g_keep_edges'
    ereturn scalar graph_initial_components = `g_init_comp'
    ereturn scalar graph_leaveout_components = `g_max_comp'
    ereturn scalar graph_initial_component_rows = `g_init_rows'
    ereturn scalar graph_insufficient_workers = `g_degree_removed'
    ereturn scalar graph_articulation_workers = `g_art_removed'
    ereturn scalar graph_bridge_units_removed = `g_bridge_units'
    ereturn scalar graph_bridge_rows_removed = `g_bridge_rows'
    ereturn scalar graph_pruning_iterations = `g_degree_iters'
    ereturn scalar graph_bridge_iterations = `g_bridge_iters'
    ereturn scalar graph_fixedpoint_iterations = `g_fixed_iters'
    ereturn scalar graph_final_bridge_units = 0
    ereturn scalar probes = 0
    ereturn scalar probes_requested = `probesrequested'
    ereturn scalar max_leverage = `r_max_lev'
    ereturn scalar information_rcond = `r_info_rcond'
    ereturn scalar inverse_relres = `r_inverse_relres'
    ereturn scalar solver_iterations = 0
    ereturn scalar solver_max_residual = `r_max_complete'
    ereturn scalar complete_residual_max = `r_max_complete'
    ereturn scalar correction_reciprocal_residual = `r_maker'
    ereturn scalar target_identity_residual = `r_actual_accounting'
    ereturn scalar weighted_rss = `r_rss'
    ereturn scalar target_outcome_variance = `target_outcome_variance'
    ereturn scalar regression_outcome_variance = `regression_outcome_variance'
    ereturn scalar residual_variance = `residual_variance'
    ereturn scalar full_model_explained_variance = `explained_variance'
    ereturn scalar full_model_explained_share = `explained_share'
    ereturn scalar tolerance = `tolerancerequested'
    ereturn scalar tolerance_requested = `tolerancerequested'
    ereturn scalar maxiter = 0
    ereturn scalar maxiter_requested = `maxiterrequested'
    ereturn scalar seed = 0
    ereturn scalar seed_requested = `seedrequested'
    ereturn scalar batch = 0
    ereturn scalar batch_requested_numeric = `batchnumeric'
    ereturn scalar leverage_batch = 0
    ereturn scalar target_batch = 0
    ereturn scalar memory_gib = `memorygib'
    ereturn scalar memory_forecast_bytes = `r_command_peak'
    ereturn scalar physical_limit = `physicallimit'
    ereturn scalar physical_limit_applied = 0
    ereturn scalar active_processors = c(processors)
    ereturn scalar route_code = 1
    ereturn scalar route_planned_rhs = 0
    ereturn scalar rust_requested_algorithm_code = `r_algorithm_req'
    ereturn scalar rust_selected_algorithm_code = `r_algorithm_sel'
    ereturn scalar rust_requested_engine_code = `r_engine_req'
    ereturn scalar rust_selected_engine_code = `r_engine_sel'
    ereturn scalar rust_plan_struct_size = `r_plan_struct'
    ereturn scalar rust_plan_schema = `r_plan_schema'
    ereturn scalar rust_plan_route_schema = `r_plan_route_schema'
    ereturn scalar rust_plan_resolved = `r_plan_resolved'
    ereturn scalar rust_plan_frozen = `r_plan_frozen'
    ereturn scalar rust_plan_applicability = `r_plan_applicability'
    ereturn scalar rust_plan_algorithm_requested = `r_plan_alg_req'
    ereturn scalar rust_plan_algorithm_selected = `r_plan_alg_sel'
    ereturn scalar rust_plan_algorithm_reason = `r_plan_alg_reason'
    ereturn scalar rust_plan_engine_requested = `r_plan_eng_req'
    ereturn scalar rust_plan_engine_selected = `r_plan_eng_sel'
    ereturn scalar rust_plan_engine_reason = `r_plan_eng_reason'
    ereturn scalar rust_plan_compressed_eligibility = `r_plan_comp_elig'
    ereturn scalar rust_plan_complexity = `r_plan_complexity'
    ereturn scalar rust_plan_exact_limit = `r_plan_exact_limit'
    ereturn scalar rust_plan_route_requested = `r_plan_route_req'
    ereturn scalar rust_plan_route_selected = `r_plan_route_sel'
    ereturn scalar rust_plan_route_fallback = `r_plan_route_fallback'
    ereturn scalar rust_plan_route_error = `r_plan_route_error'
    ereturn scalar rust_plan_rhs = `r_plan_rhs'
    ereturn scalar rust_plan_full_dimension = `r_plan_full_dim'
    ereturn scalar rust_plan_leverage_batch = `r_plan_lev_batch'
    ereturn scalar rust_plan_target_batch = `r_plan_tgt_batch'
    ereturn scalar rust_plan_solve_peak_bytes = `r_plan_mem_command'
    ereturn scalar rust_counter_plan_complete = `r_ctr_complete'
    ereturn scalar rust_pre_rng_hi = `r_pre_rng_hi'
    ereturn scalar rust_pre_rng_lo = `r_pre_rng_lo'
    ereturn scalar rust_wall_request_applicable = `r_wall_req_app'
    ereturn scalar rust_wallseconds_supplied = `r_wallseconds'
    ereturn scalar rust_wallseconds_requested = `r_wall_requested'
    ereturn scalar rust_wallseconds_forecast = `r_wall_forecast'
    ereturn scalar rust_wallseconds_advisory = `r_wall_advisory'
    ereturn scalar rust_wallseconds_margin = `r_wall_margin'
    ereturn scalar rust_batch_mode_code = `r_batch_mode'
    ereturn scalar rust_leverage_batch_mode_code = `r_lev_batch_mode'
    ereturn scalar rust_target_batch_mode_code = `r_tgt_batch_mode'
    ereturn scalar rust_requested_route = `r_req_route'
    ereturn scalar rust_selected_route = `r_sel_route'
    ereturn scalar rust_solver_dimension = `r_dimension'
    ereturn scalar residual_acceptance_tolerance = `r_full_tol'
    ereturn scalar rust_full_fit_route = `r_full_route'
    ereturn scalar rust_full_fit_iterations = `r_full_iter'
    ereturn scalar rust_full_fit_reduced_residual = `r_full_red'
    ereturn scalar rust_full_fit_complete_residual = `r_full_complete'
    ereturn scalar rust_working_fit_residual = `r_working_fit'
    ereturn scalar rust_max_reduced_residual = `r_max_red'
    ereturn scalar rust_max_complete_residual = `r_max_complete'
    ereturn scalar rust_max_reciprocal_residual = `r_max_recip'
    ereturn scalar rust_full_fit_zero_rhs = `r_full_zero'
    ereturn scalar rust_rank_tolerance = `r_rank_tol'
    ereturn scalar rust_block_tolerance = `r_block_tol'
    ereturn scalar rust_inverse_sqrt_relres = `r_inverse_sqrt'
    ereturn scalar rust_maker_relres = `r_maker'
    ereturn scalar rust_control_basis_relres = `r_control_relres'
    ereturn scalar rust_control_basis_forward_error = `r_control_forward'
    ereturn scalar rust_deletion_rank_gap = `r_rank_gap'
    ereturn scalar rust_firm_zero_sum_residual = `r_firm_zero'
    ereturn scalar rust_actual_accounting_residual = `r_actual_accounting'
    ereturn scalar rust_exact_diagnostic_flags = `r_exact_flags'
    ereturn scalar rust_exact_fit_peak_bytes = `r_fit_peak'
    ereturn scalar rust_exact_correction_peak_bytes =                ///
        `r_correction_peak'
    ereturn scalar rust_exact_peak_forecast_bytes = `r_exact_peak'
    ereturn scalar rust_core_ready_flags = `rustcoreflags'
    ereturn scalar rust_support_flags = `rustsupportflags'
    ereturn scalar rust_topology_checksum_hi = `r_top_hi'
    ereturn scalar rust_topology_checksum_lo = `r_top_lo'
    ereturn scalar rust_rng_contract_code = `r_rng'
    ereturn scalar rng_master_seed = `r_seed'
    ereturn scalar rust_cap_struct_size = `capstruct'
    ereturn scalar rust_cap_abi_version = `capabi'
    ereturn scalar rust_cap_schema = `capschema'
    ereturn scalar rust_cap_supported = `capsupported'
    ereturn scalar rust_cap_reason_code = `capreason'
    ereturn scalar rust_cap_profile_code = `capprofile'
    ereturn scalar rust_cap_signature_hi = `capsignaturehi'
    ereturn scalar rust_cap_signature_lo = `capsignaturelo'
    ereturn scalar rust_cap_algorithm_deferred = `capalgdefer'
    ereturn scalar rust_cap_engine_deferred = `capengdefer'
    ereturn scalar rust_cap_route_deferred = `caproutedefer'
    ereturn scalar rust_cap_leverage_batch_deferred = `caplevdefer'
    ereturn scalar rust_cap_target_batch_deferred = `captgtdefer'
    ereturn scalar numerical_mcse_available = 0
    ereturn scalar backend_option_supplied = `backendsupplied'
    ereturn scalar rng_option_supplied = `rngsupplied'
    ereturn scalar algorithm_option_supplied = `algorithmsupplied'
    ereturn scalar engine_option_supplied = `enginesupplied'
    ereturn scalar preconditioner_option_supplied = `preconditionsupplied'
    ereturn scalar batch_option_supplied = `batchsupplied'
    ereturn scalar stayers_option_supplied = `stayerssupplied'
    ereturn scalar deletionid_option_supplied = `deletionidsupplied'
    ereturn scalar targetweight_option_supplied = `targetweightsupplied'
    ereturn local cmd "vckss"
    ereturn local cmdline `"`cmdline'"'
    ereturn local version "0.4.0-alpha.1"
    ereturn local model "linear"
    ereturn local correction_method "kss"
    ereturn local backend_requested "rust"
    ereturn local backend_selected "rust"
    ereturn local backend_routing_reason = cond(`algreq'==0,        ///
        "explicit planned Rust algorithm-auto route selected exact; RNG is not applicable", ///
        "explicit Rust exact route selected the native exact result family; RNG is not applicable")
    ereturn local rng_requested "`rngrequested'"
    ereturn local rng_selected "NOT_APPLICABLE"
    ereturn local rng_contract "NOT_APPLICABLE"
    ereturn local rng_implementation "NOT_APPLICABLE"
    ereturn local rng_runtime "NOT_APPLICABLE"
    ereturn local rng_leverage_domain "NOT_APPLICABLE"
    ereturn local rng_target_domain "NOT_APPLICABLE"
    ereturn local algorithm_requested "`algorithmrequested'"
    ereturn local algorithm "exact"
    ereturn local stayers "`stayersmode'"
    ereturn local engine_requested "`enginerequested'"
    ereturn local engine_selected "NOT_APPLICABLE"
    ereturn local preconditioner_requested "`preconditionerrequested'"
    ereturn local preconditioner_selected "NOT_APPLICABLE"
    ereturn local routing_reason "EXACT_ALGORITHM"
    ereturn local fallback_status "NOT_APPLICABLE"
    ereturn local fallback_message "exact estimation does not use iterative fallback"
    ereturn local batch_requested "`batchrequested'"
    ereturn local batch_routing_reason "exact algorithm does not consume probe batches"
    ereturn local physical_limit_status "NOT_APPLICABLE_TO_EXACT"
    ereturn local rust_capability_profile "PLANNED_V1"
    ereturn local execution_plan_schema "VCKSS-EXECUTION-PLAN-V1"
    ereturn local route_api "VCKSS-NATIVE-EXACT-PLANNED-V4-V7"
    ereturn local result_family "exact"
    ereturn local rust_capability_reason "SUPPORTED"
    ereturn local rust_reduced_receipt_note                         ///
        "conservative mirrors of exact complete-fit residuals"
    ereturn local deletion "`deletionmode'"
    ereturn local nuisance "`nuisance'"
    ereturn local target_population = cond("`deletionmode'"=="match", ///
        "movers", "retained observations")
    ereturn local sample_selection = cond("`deletionmode'"=="match", ///
        "MOVERS_DELETION_MULTIGRAPH_FIXED_POINT",                    ///
        "MATLAB_LEAVEONEWORKER_COMPONENT")
    ereturn local connectedness_status = cond("`deletionmode'"=="match", ///
        "DELETION_UNIT_BRIDGE_FREE", "LEAVE_ONE_WORKER_CONNECTED")
    ereturn local frequency_convention "literal physical copies"
    ereturn local targetweight_convention                           ///
        "explicit stored-row mass; default physical-observation mass"
    ereturn local inference "not implemented"
    ereturn local numerical_error "deterministic dense numerical backend"
    ereturn local deletion_rank_certificate "dense Woodbury plus direct rank gate"
    ereturn local status "KSS_POINT_ESTIMATES_ONLY"
    if "`nodisplay'" == "" _vckss_display
end

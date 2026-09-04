*! fevc 0.5.0-alpha.1 31aug2026

program define fevc, eclass
    version 18.0

    if lower(strtrim(`"`0'"')) == ", version" {
        _vckss_impl `0'
        exit
    }

    // Routing metadata is command-local failure context.  Clear it before
    // any runtime work so an interrupted earlier command cannot leak into a
    // later typed failure.
    quietly _vckss_route_context_clear

    // A cached compressed design is command-local state.  Clear a current
    // scale runtime defensively at entry so no interrupted prior invocation
    // can leak state into this estimate.
    capture mata: assert(vckss_scale__api_level() == 6 &          ///
        vckss_scale__build_id() ==                               ///
        "vckss-scale-api6-prep-sem1-mata")
    if !_rc capture mata: vckss_scale_runtime__reset()

    capture mata: vckss_rng__api_level()
    local rng_runtime_loaded = (_rc == 0)
    capture mata: assert(vckss_rng__api_level() == 4 &              ///
        vckss_rng__build_id() ==                                   ///
        "vckss-rng-numeric-ranks-v4")
    if _rc {
        if `rng_runtime_loaded' {
            quietly _vckss_post_failure "STALE_RNG_RUNTIME"
            di as error "a different KSS RNG runtime is already loaded; restart Stata or run discard before retrying"
            _vckss_display_failure
            exit 498
        }
        capture findfile fevc_rng.mata
        if _rc {
            quietly _vckss_post_failure "RNG_RUNTIME_NOT_FOUND"
            di as error "fevc_rng.mata was not found on the Stata adopath"
            _vckss_display_failure
            exit 601
        }
        quietly do `"`r(fn)'"'
        capture mata: assert(vckss_rng__api_level() == 4 &          ///
            vckss_rng__build_id() ==                               ///
            "vckss-rng-numeric-ranks-v4")
        if _rc {
            quietly _vckss_post_failure "INVALID_RNG_RUNTIME"
            di as error "the installed KSS RNG runtime is incompatible with this command"
            _vckss_display_failure
            exit 498
        }
    }

    tempname outer_rng_guard_rc outer_rng_restore_rc
    mata: st_numscalar("`outer_rng_guard_rc'",vckss_rng__guard_begin())
    if scalar(`outer_rng_guard_rc') {
        quietly _vckss_post_failure "RNG_GUARD_FAILED"
        di as error "caller RNG and sort-jumbler state could not be captured"
        _vckss_display_failure
        exit 498
    }
    capture quietly _vckss_stage_timer_ids
    if _rc {
        mata: st_numscalar("`outer_rng_restore_rc'",               ///
            vckss_rng__guard_restore())
        quietly _vckss_post_failure "TIMER_RESERVATION_FAILED"
        di as error "two free command-stage timer IDs were not available"
        _vckss_display_failure
        exit 498
    }
    local stage_selection_timer = r(selection_timer)
    local stage_validation_timer = r(validation_timer)
    global VCKSS_STAGE_SELECTION_TIMER `stage_selection_timer'
    global VCKSS_STAGE_VALIDATION_TIMER `stage_validation_timer'
    capture noisily _vckss_impl `0'
    local command_rc = _rc
    local outer_scale_reset_rc = 0
    capture mata: assert(vckss_scale__api_level() == 6 &          ///
        vckss_scale__build_id() ==                               ///
        "vckss-scale-api6-prep-sem1-mata")
    if !_rc {
        capture mata: vckss_scale_runtime__reset()
        local outer_scale_reset_rc = _rc
    }
    mata: st_numscalar("`outer_rng_restore_rc'",vckss_rng__guard_restore())
    foreach stage_timer in `stage_selection_timer'                ///
        `stage_validation_timer' {
        capture quietly timer off `stage_timer'
        capture quietly timer clear `stage_timer'
    }
    macro drop VCKSS_STAGE_SELECTION_TIMER
    macro drop VCKSS_STAGE_VALIDATION_TIMER
    if scalar(`outer_rng_restore_rc') {
        quietly _vckss_post_failure "RNG_RESTORE_FAILED"
        quietly _vckss_route_context_clear
        di as error "caller RNG and sort-jumbler state could not be restored"
        _vckss_display_failure
        exit 498
    }
    if `outer_scale_reset_rc' & !`command_rc' {
        quietly _vckss_post_failure "SCALE_STATE_RELEASE_FAILED"
        quietly _vckss_route_context_clear
        di as error "cached compressed state could not be released"
        _vckss_display_failure
        exit 498
    }
    if !`command_rc' & "`e(cmd)'" == "fevc" &                  ///
        !inlist("`e(status)'", "WITHHELD", "ALPHA") {
        ereturn local estat_cmd "fevc_estat"
    }
    if `command_rc' & `"`e(status)'"' == "WITHHELD" {
        _vckss_display_failure
    }
    quietly _vckss_route_context_clear
    exit `command_rc'
end

program define _fevc_rust_generic, eclass sortpreserve
    version 18.0
    args depvar worker firm deletionvar frequency target touse nscope ///
        ncomplete nstayers nstayerrows probes batch seed tolerance    ///
        maxiter memorygib enginerequested backendsupplied rngsupplied ///
        deletionidsupplied enginesupplied algorithmsupplied          ///
        preconditionsupplied batchsupplied stayerssupplied           ///
        rustcoreflags                                                ///
        rustsupportflags nodisplay deletionmode nuisance exactlimit  ///
        ranktol blocktol blocksizelimit physicallimit controls       ///
        targetweightsupplied frequencyused cmdline

    foreach input in `depvar' `worker' `firm' `deletionvar'          ///
        `frequency' `target' `touse' `controls' {
        confirm numeric variable `input'
    }
    local control_count : word count `controls'
    local deletion_code = cond("`deletionmode'"=="match",1,2)
    local nuisance_code = cond("`nuisance'"=="joint",1,2)
    local frequency_code = `frequencyused'
    local target_code = `targetweightsupplied'
    local target_mode frequency
    if `target_code' local target_mode explicit
    local deletion_source cell
    local deletion_source_code = 1
    if "`deletionmode'" == "observation" {
        local deletion_source observation
        local deletion_source_code = 3
    }
    else if `deletionidsupplied' {
        local deletion_source matchid
        local deletion_source_code = 2
    }

    capture quietly fevc_rust clear
    if _rc {
        local failure_rc = _rc
        capture noisily _fevc_rust_abort, rc(`failure_rc') phase(clear_entry)
        exit _rc
    }

    capture quietly _fevc_rust_public_call requestcapability,       ///
        algorithm(jla) deletion(`deletionmode') nuisance(`nuisance') ///
        route(diagonal) rngcontract(counter_v1)                      ///
        controls(`control_count') frequencyused(`frequency_code')    ///
        engine(generic) batchmode(explicit) stayers(movers)          ///
        targetweightmode(`target_mode') deletionsource(`deletion_source') ///
        probeordersupplied(0) wallsecondssupplied(0)                 ///
        physicallimit(`physicallimit')
    if _rc {
        capture quietly fevc_rust clear
        global VCKSS_ROUTE_BACKEND_REASON                            ///
            "explicit generic-JLA route could not obtain a V2 capability receipt"
        quietly _vckss_post_failure "RUST_BACKEND_UNAVAILABLE"      ///
            "The generic-JLA request-capability query was unavailable; no native preparation was attempted."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "request_capability"
        ereturn local backend_selected ""
        ereturn local rng_selected ""
        ereturn scalar rust_core_ready_flags = `rustcoreflags'
        ereturn scalar rust_support_flags = `rustsupportflags'
        exit 498
    }
    foreach name in struct_size abi_version request_schema supported ///
        reason_code profile_code algorithm_code deletion_mode_code   ///
        nuisance_mode_code solver_route_code rng_contract_code       ///
        controls_count frequency_use_code engine_code batch_mode_code ///
        stayers_mode_code target_weight_mode_code deletion_source_code ///
        probeorder_supplied wallseconds_supplied physical_limit      ///
        request_signature_hi request_signature_lo {
        local cap_`name' = r(`name')
    }
    local cap_reason_name `"`r(reason)'"'
    local cap_profile_name `"`r(profile)'"'
    quietly _vckss_request_signature 2 2 `deletion_code'           ///
        `nuisance_code' 2 1 `control_count' `frequency_code' 2 1  ///
        1 `target_code' `deletion_source_code' 0 0 `physicallimit'
    local expected_signature_hi = r(signature_hi)
    local expected_signature_lo = r(signature_lo)
    local capability_ok = 1
    foreach name in struct_size abi_version request_schema supported ///
        reason_code profile_code algorithm_code deletion_mode_code   ///
        nuisance_mode_code solver_route_code rng_contract_code       ///
        controls_count frequency_use_code engine_code batch_mode_code ///
        stayers_mode_code target_weight_mode_code deletion_source_code ///
        probeorder_supplied wallseconds_supplied physical_limit      ///
        request_signature_hi request_signature_lo {
        if missing(`cap_`name'') | `cap_`name'' < 0 |               ///
            `cap_`name'' != floor(`cap_`name'') local capability_ok = 0
    }
    if `capability_ok' {
        local capability_ok =                                      ///
            `cap_struct_size' == 104 & `cap_abi_version' == 1 &    ///
            `cap_request_schema' == 2 &                            ///
            inlist(`cap_supported',0,1) &                          ///
            `cap_algorithm_code' == 2 &                            ///
            `cap_deletion_mode_code' == `deletion_code' &          ///
            `cap_nuisance_mode_code' == `nuisance_code' &          ///
            `cap_solver_route_code' == 2 & `cap_rng_contract_code' == 1 & ///
            `cap_controls_count' == `control_count' &              ///
            `cap_frequency_use_code' == `frequency_code' &         ///
            `cap_engine_code' == 2 & `cap_batch_mode_code' == 1 & ///
            `cap_stayers_mode_code' == 1 &                         ///
            `cap_target_weight_mode_code' == `target_code' &       ///
            `cap_deletion_source_code' == `deletion_source_code' & ///
            `cap_probeorder_supplied' == 0 &                       ///
            `cap_wallseconds_supplied' == 0 &                      ///
            `cap_physical_limit' == `physicallimit' &              ///
            `cap_request_signature_hi' == `expected_signature_hi' & ///
            `cap_request_signature_lo' == `expected_signature_lo' & ///
            `cap_request_signature_hi' <= 4294967295 &             ///
            `cap_request_signature_lo' <= 4294967295 &             ///
            (`cap_supported'==1) == (`cap_reason_code'==0) &        ///
            cond(`cap_supported'==1,`cap_profile_code'==3,          ///
                `cap_profile_code'==0)
    }
    if !`capability_ok' {
        capture quietly fevc_rust clear
        global VCKSS_ROUTE_BACKEND_REASON                            ///
            "explicit generic-JLA route rejected an inconsistent V2 capability receipt"
        quietly _vckss_post_failure "RUST_BACKEND_UNQUALIFIED"      ///
            "The V2 request-capability receipt did not reconcile with the materialized generic-JLA tuple."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "request_capability_reconcile"
        ereturn local backend_selected ""
        ereturn local rng_selected ""
        ereturn scalar rust_core_ready_flags = `rustcoreflags'
        ereturn scalar rust_support_flags = `rustsupportflags'
        exit 498
    }
    if !`cap_supported' {
        capture quietly fevc_rust clear
        global VCKSS_ROUTE_BACKEND_REASON                            ///
            "explicit generic-JLA request was declined by the V2 capability boundary"
        quietly _vckss_post_failure "RUST_OPTION_UNSUPPORTED"       ///
            "Rust generic-JLA capability declined the materialized tuple: `cap_reason_name'."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "request_capability"
        ereturn local backend_selected ""
        ereturn local rng_selected ""
        ereturn local rust_request_capability_reason "`cap_reason_name'"
        ereturn scalar rust_cap_reason_code = `cap_reason_code'
        ereturn scalar rust_core_ready_flags = `rustcoreflags'
        ereturn scalar rust_support_flags = `rustsupportflags'
        exit 498
    }

    tempvar rust_keep
    capture noisily _fevc_rust_public_call prepare `worker' `firm' ///
        `deletionvar' `depvar' `frequency' `target' `controls'      ///
        if `touse', cleanup generate(`rust_keep') memorygib(`memorygib') ///
        deletion(`deletionmode')
    if _rc {
        local failure_rc = _rc
        capture noisily _fevc_rust_abort, rc(`failure_rc') phase(prepare)
        exit _rc
    }
    local handle = r(handle)
    foreach pair in input_rows:p_input retained_rows:p_retained workers:p_workers ///
        firms:p_firms cells:p_cells deletion_units:p_units            ///
        target_strata:p_strata target_weight_sum:p_target             ///
        memory_limit_bytes:p_mem_limit caller_copy_bytes:p_input_copy ///
        preparation_peak_forecast_bytes:p_prep_peak                   ///
        prepared_resident_bytes:p_resident controls_count:p_controls  ///
        graph_input_rows:g_input_rows graph_retained_rows:g_keep_rows ///
        graph_input_physical_mass:g_input_mass                        ///
        graph_retained_physical_mass:g_keep_mass                      ///
        graph_initial_components:g_init_comp                         ///
        graph_maximum_components:g_max_comp                          ///
        graph_initial_component_rows:g_init_rows                     ///
        graph_mover_input_rows:g_mover_rows                          ///
        graph_initial_deletion_edges:g_init_edges                    ///
        graph_retained_deletion_edges:g_keep_edges                   ///
        graph_degree_workers_removed:g_degree_removed                ///
        graph_artic_workers_removed:g_art_removed                    ///
        graph_bridge_units_removed:g_bridge_units                    ///
        graph_bridge_rows_removed:g_bridge_rows                      ///
        graph_degree_iterations:g_degree_iters                       ///
        graph_articulation_iterations:g_art_iters                    ///
        graph_bridge_iterations:g_bridge_iters                       ///
        graph_fixed_point_iterations:g_fixed_iters {
        gettoken returned localname : pair, parse(":")
        local localname = substr("`localname'",2,.)
        local `localname' = r(`returned')
    }
    quietly summarize `frequency' if `touse', meanonly
    local input_physical = r(sum)
    quietly replace `touse' = `touse' & `rust_keep'
    quietly count if `touse'
    local retained_count = r(N)
    quietly summarize `frequency' if `touse', meanonly
    local retained_physical = r(sum)
    quietly summarize `target' if `touse', meanonly
    local retained_target = r(sum)

    local preparation_ok = 1
    foreach value in handle p_input p_retained p_workers p_firms p_cells ///
        p_units p_strata p_mem_limit p_input_copy p_prep_peak p_resident ///
        p_controls g_input_rows g_keep_rows g_input_mass g_keep_mass     ///
        g_init_comp g_max_comp g_init_rows g_mover_rows g_init_edges    ///
        g_keep_edges g_degree_removed g_art_removed g_bridge_units      ///
        g_bridge_rows g_degree_iters g_art_iters g_bridge_iters g_fixed_iters {
        if missing(``value'') | ``value'' < 0 |                        ///
            ``value'' != floor(``value'') local preparation_ok = 0
    }
    local expected_input_copy = `p_input'*(6+`control_count')*8
    local expected_prep_peak = `expected_input_copy'+               ///
        `p_input'*768+`p_input'*`control_count'*32+4096
    if missing(`p_target') | `p_target' <= 0 |                       ///
        missing(`retained_target') | `retained_target' <= 0 local preparation_ok = 0
    if `preparation_ok' {
        local preparation_ok =                                      ///
            `handle'>0 & `p_input'>0 & `p_retained'>0 &             ///
            `p_workers'>0 & `p_firms'>1 & `p_cells'>0 &             ///
            `p_units'>0 & `p_strata'>0 & `p_controls'==`control_count' & ///
            `p_input'==`ncomplete' & `p_retained'==`retained_count' & ///
            `p_input_copy'==`expected_input_copy' &                 ///
            `p_prep_peak'==`expected_prep_peak' & `p_resident'>0 & ///
            `p_input_copy'+`p_resident'<=`p_mem_limit' &            ///
            `p_prep_peak'<=`p_mem_limit' &                          ///
            `g_input_rows'==`ncomplete' & `g_keep_rows'==`retained_count' & ///
            `g_input_mass'==`input_physical' &                      ///
            `g_keep_mass'==`retained_physical' &                    ///
            `g_input_rows'>=`g_keep_rows' & `g_input_mass'>=`g_keep_mass' & ///
            `g_init_comp'>0 & `g_max_comp'>=`g_init_comp' &         ///
            `g_max_comp'<=`g_input_rows' & `g_init_rows'>0 &        ///
            `g_init_rows'<=`g_input_rows' & `g_init_rows'>=`g_mover_rows' & ///
            `g_mover_rows'>=`g_keep_rows' & `g_init_edges'>0 &      ///
            `g_init_edges'>=`g_keep_edges' &                        ///
            `g_degree_iters'<=`g_degree_removed' &                  ///
            `g_art_iters'<=`g_art_removed' &                        ///
            (`g_degree_iters'==0)==(`g_degree_removed'==0) &        ///
            (`g_art_iters'==0)==(`g_art_removed'==0) &              ///
            abs(`p_target'-`retained_target')<=1e-10*max(1,abs(`p_target'))
    }
    if `preparation_ok' & "`deletionmode'"=="match" {
        local preparation_ok =                                      ///
            `g_keep_edges'==`p_units' & `g_bridge_iters'<=`g_bridge_units' & ///
            (`g_bridge_iters'==0)==(`g_bridge_units'==0) &           ///
            (`g_bridge_iters'==0)==(`g_bridge_rows'==0) &            ///
            `g_bridge_units'<=`g_init_edges' &                       ///
            `g_bridge_rows'>=`g_bridge_units' & `g_bridge_rows'<=`g_mover_rows' & ///
            `g_fixed_iters'==`g_degree_iters'+`g_art_iters'+`g_bridge_iters'
    }
    if `preparation_ok' & "`deletionmode'"=="observation" {
        local preparation_ok =                                      ///
            `p_units'==`retained_physical' & `g_mover_rows'==`g_init_rows' & ///
            `g_bridge_units'==0 & `g_bridge_rows'==0 & `g_bridge_iters'==0 & ///
            `g_fixed_iters'==`g_degree_iters'+`g_art_iters'
    }
    if !`preparation_ok' {
        capture quietly fevc_rust release `handle'
        capture quietly fevc_rust clear
        quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"     ///
            "Generic-JLA preparation receipts did not reconcile with the validated Stata sample."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "preparation_reconcile"
        ereturn local backend_selected ""
        ereturn local rng_selected ""
        ereturn scalar rust_core_ready_flags = `rustcoreflags'
        ereturn scalar rust_support_flags = `rustsupportflags'
        exit 498
    }
    if `retained_physical' > `physicallimit' {
        capture quietly fevc_rust release `handle'
        capture quietly fevc_rust clear
        quietly _vckss_post_failure "PHYSICAL_COPY_LIMIT"          ///
            "Retained physical mass exceeds physical_limit() before generic-JLA RNG."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "physical_limit"
        ereturn local backend_selected ""
        ereturn local rng_selected ""
        ereturn scalar physical_limit = `physicallimit'
        ereturn scalar rust_core_ready_flags = `rustcoreflags'
        ereturn scalar rust_support_flags = `rustsupportflags'
        exit 498
    }

    capture noisily _fevc_rust_public_call solve `handle',         ///
        seed(`seed') probes(`probes') leveragebatch(`batch')         ///
        targetbatch(`batch') route(diagonal) tolerance(`tolerance') ///
        maxiter(`maxiter') algorithm(jla) deletion(`deletionmode')  ///
        nuisance(`nuisance') exactlimit(`exactlimit')               ///
        blocksizelimit(`blocksizelimit') ranktolerance(`ranktol')   ///
        blocktolerance(`blocktol') engine(generic) batchmode(explicit) ///
        stayers(movers) targetweightmode(`target_mode')             ///
        deletionsource(`deletion_source') probeordersupplied(0)     ///
        wallsecondssupplied(0) physicallimit(`physicallimit')       ///
        capabilityschema(2) capabilityprofile(3)                    ///
        frequencyused(`frequency_code')                             ///
        signaturehi(`cap_request_signature_hi')                     ///
        signaturelo(`cap_request_signature_lo')
    if _rc {
        local failure_rc = _rc
        capture noisily _fevc_rust_abort, rc(`failure_rc')         ///
            handle(`handle') phase(solve)
        exit _rc
    }
    capture noisily _fevc_rust_public_call result `handle'
    if _rc {
        local failure_rc = _rc
        capture noisily _fevc_rust_abort, rc(`failure_rc')         ///
            handle(`handle') phase(result_export)
        exit _rc
    }
    tempname raw_results rhs_native
    matrix `raw_results' = r(result)
    matrix `rhs_native' = r(rhs_receipts)
    tempname native_full_red native_full_complete native_full_tol native_working
    tempname native_cr_rcond native_cr_small native_cr_large native_cr_proj
    tempname native_cr_norm native_cr_fe native_cr_maxproj native_cr_tol
    tempname native_cr_pcg native_cr_gate
    scalar `native_full_red' = r(full_fit_reduced_residual)
    scalar `native_full_complete' = r(full_fit_complete_residual)
    scalar `native_full_tol' = r(full_residual_tolerance)
    scalar `native_working' = r(generic_working_fit_residual)
    scalar `native_cr_rcond' = r(control_rank_rcond)
    scalar `native_cr_small' = r(control_rank_smallest_lower)
    scalar `native_cr_large' = r(control_rank_largest_upper)
    scalar `native_cr_proj' = r(control_rank_projection_error)
    scalar `native_cr_norm' = r(control_rank_normalization_err)
    scalar `native_cr_fe' = r(control_rank_fe_info_lower)
    scalar `native_cr_maxproj' = r(control_rank_max_projection)
    scalar `native_cr_tol' = r(control_rank_effective_tolerance)
    scalar `native_cr_pcg' = r(control_rank_projection_pcg_tol)
    scalar `native_cr_gate' = r(control_rank_projection_gate)
    foreach pair in seed:r_seed probes:r_probes                       ///
        leverage_probes_accepted:r_lev_acc target_probes_accepted:r_tgt_acc ///
        requested_route:r_req_route selected_route:r_sel_route       ///
        solver_fallback:r_fallback solver_fallback_error:r_fallback_err ///
        solver_dimension:r_dimension leverage_batch_width:r_lev_batch ///
        target_batch_width:r_tgt_batch rank_tolerance:r_rank_tol      ///
        block_tolerance:r_block_tol full_residual_tolerance:r_full_tol ///
        full_fit_route:r_full_route full_fit_iterations:r_full_iter  ///
        full_fit_reduced_residual:r_full_red                         ///
        full_fit_complete_residual:r_full_complete full_fit_zero_rhs:r_full_zero ///
        leverage_rhs_count:r_lev_rhs target_rhs_count:r_tgt_rhs      ///
        max_reduced_residual:r_max_red max_complete_residual:r_max_complete ///
        max_leverage:r_max_lev max_reciprocal_residual:r_max_recip  ///
        accounting_residual:r_accounting topology_checksum_hi:r_top_hi ///
        topology_checksum_lo:r_top_lo rng_contract_code:r_rng        ///
        rhs_receipt_rows:r_rhs_rows caller_result_copy_bytes:r_rhs_copy ///
        requested_algorithm_code:r_algorithm_req selected_algorithm_code:r_algorithm_sel ///
        deletion_mode_code:r_deletion nuisance_mode_code:r_nuisance ///
        parameters:r_parameters full_parameters:r_full_parameters    ///
        correction_parameters:r_correction_parameters               ///
        information_rcond:r_native_info inverse_relative_residual:r_native_inverse ///
        exact_diagnostic_flags:r_exact_flags actual_accounting_residual:r_actual_accounting ///
        weighted_rss:r_rss memory_limit_bytes:r_mem_limit            ///
        caller_copy_bytes:r_input_copy preparation_peak_forecast_bytes:r_prep_peak ///
        prepared_resident_bytes:r_resident solver_setup_forecast_bytes:r_solver_setup ///
        leverage_phase_forecast_bytes:r_lev_phase target_phase_forecast_bytes:r_tgt_phase ///
        result_forecast_bytes:r_result_bytes solve_peak_forecast_bytes:r_solve_peak ///
        command_peak_forecast_bytes:r_command_peak rhs_receipt_schema:r_rhs_schema ///
        requested_engine_code:r_engine_req selected_engine_code:r_engine_sel ///
        generic_diagnostic_flags:r_generic_flags generic_controls_count:r_generic_controls ///
        control_projection_rhs_count:r_control_rhs control_rank_rcond:r_cr_rcond ///
        control_rank_smallest_lower:r_cr_small control_rank_largest_upper:r_cr_large ///
        control_rank_projection_error:r_cr_proj control_rank_normalization_err:r_cr_norm ///
        control_rank_fe_info_lower:r_cr_fe control_rank_max_projection:r_cr_maxproj ///
        control_rank_effective_tolerance:r_cr_tol control_rank_projection_pcg_tol:r_cr_pcg ///
        control_rank_projection_gate:r_cr_gate generic_control_basis_relres:r_control_basis ///
        generic_control_basis_fwd_err:r_control_forward generic_control_schur_rcond:r_schur_rcond ///
        generic_control_schur_relres:r_schur_relres generic_deletion_rank_gap:r_rank_gap ///
        full_joint_fit_complete_residual:r_full_joint generic_working_fit_residual:r_working_fit ///
        generic_maker_relative_residual:r_maker canonicalization_peak_bytes:r_canon_peak ///
        generic_fit_peak_forecast_bytes:r_fit_peak geometry_peak_forecast_bytes:r_geometry_peak ///
        generic_leverage_peak_bytes:r_generic_lev_peak generic_target_peak_bytes:r_generic_tgt_peak ///
        maker_peak_forecast_bytes:r_maker_peak generic_result_forecast_bytes:r_generic_result ///
        generic_peak_forecast_bytes:r_generic_peak rhs_v2_caller_copy_bytes:r_rhs_v2_copy ///
        capability_schema:r_cap_schema capability_profile:r_cap_profile ///
        batch_mode_code:r_batch_mode stayers_mode_code:r_stayers_mode ///
        target_weight_mode_code:r_target_mode deletion_source_code:r_deletion_source ///
        probeorder_supplied:r_probeorder wallseconds_supplied:r_wallseconds ///
        frequency_use_code:r_frequency physical_limit:r_physical_limit ///
        request_signature_hi:r_signature_hi request_signature_lo:r_signature_lo {
        gettoken returned localname : pair, parse(":")
        local localname = substr("`localname'",2,.)
        local `localname' = r(`returned')
    }

    local expected_full_parameters = `p_workers'+`p_firms'-1+`control_count'
    local expected_parameters = `expected_full_parameters'
    if "`nuisance'" == "fixedoffset" {
        local expected_parameters = `p_workers'+`p_firms'-1
    }
    local expected_rhs_rows = `control_count'+1+                ///
        (`control_count'>0 & "`nuisance'"=="fixedoffset")+3*`probes'
    local expected_full_tol = max(1e-11,10*`tolerance')
    local expected_flags = 126+(`control_count'>0)
    local maker_gate = max(1e-10,100*`ranktol')
    local roundoff_gate = 1e-12
    local results_ok = rowsof(`raw_results')==4 & colsof(`raw_results')==4 & ///
        rowsof(`rhs_native')==`expected_rhs_rows' & colsof(`rhs_native')==15
    local rhs_max_iterations = 0
    local rhs_max_reduced = 0
    local rhs_max_complete = 0
    local accounting_truth = 0
    tempname control_projection_max
    scalar `control_projection_max' = 0
    if `results_ok' {
        forvalues row = 1/4 {
            forvalues column = 1/4 {
                if missing(`raw_results'[`row',`column']) local results_ok = 0
            }
        }
        forvalues row = 1/3 {
            local row_identity = abs(`raw_results'[`row',4]-          ///
                `raw_results'[`row',1]-`raw_results'[`row',2]-       ///
                2*`raw_results'[`row',3])
            local accounting_truth = max(`accounting_truth',`row_identity')
        }
        forvalues column = 1/4 {
            if abs(`raw_results'[1,`column']-`raw_results'[2,`column']- ///
                `raw_results'[3,`column']) >                         ///
                1e-10*max(1,abs(`raw_results'[1,`column'])) local results_ok = 0
        }
        if `control_count' > 0 {
            forvalues row = 1/`control_count' {
                if `rhs_native'[`row',1]!=5 |                     ///
                    `rhs_native'[`row',2]!=`row'-1 |              ///
                    `rhs_native'[`row',3]!=0 |                    ///
                    `rhs_native'[`row',13]!=scalar(`native_cr_gate') | ///
                    `rhs_native'[`row',14]!=1 |                   ///
                    `rhs_native'[`row',15]!=`p_firms' local results_ok = 0
                scalar `control_projection_max' = max(            ///
                    scalar(`control_projection_max'),             ///
                    `rhs_native'[`row',7])
            }
        }
        local semantic_row = `control_count'+1
        if `rhs_native'[`semantic_row',1]!=1 |                    ///
            `rhs_native'[`semantic_row',2]!=-1 |                  ///
            `rhs_native'[`semantic_row',3]!=0 |                   ///
            `rhs_native'[`semantic_row',5]!=`r_full_iter' |       ///
            `rhs_native'[`semantic_row',6]!=scalar(`native_full_red') | ///
            `rhs_native'[`semantic_row',7]!=scalar(`native_full_complete') | ///
            `rhs_native'[`semantic_row',8]!=`r_full_zero' |       ///
            `rhs_native'[`semantic_row',13]!=scalar(`native_full_tol') | ///
            `rhs_native'[`semantic_row',14]!=2 |                  ///
            `rhs_native'[`semantic_row',15]!=`r_dimension' local results_ok = 0
        local semantic_row = `semantic_row'+1
        if `control_count'>0 & "`nuisance'"=="fixedoffset" {
            if `rhs_native'[`semantic_row',1]!=4 |                ///
                `rhs_native'[`semantic_row',2]!=-1 |              ///
                `rhs_native'[`semantic_row',3]!=0 |               ///
                `rhs_native'[`semantic_row',7]!=scalar(`native_working') | ///
                `rhs_native'[`semantic_row',13]!=scalar(`native_full_tol') | ///
                `rhs_native'[`semantic_row',14]!=1 |              ///
                `rhs_native'[`semantic_row',15]!=`p_firms' local results_ok = 0
            local semantic_row = `semantic_row'+1
        }
        forvalues probe = 0/`=`probes'-1' {
            if `rhs_native'[`semantic_row',1]!=2 |                ///
                `rhs_native'[`semantic_row',2]!=`probe' |         ///
                `rhs_native'[`semantic_row',3]!=0 |               ///
                `rhs_native'[`semantic_row',13]!=scalar(`native_full_tol') | ///
                `rhs_native'[`semantic_row',14]!=1 |              ///
                `rhs_native'[`semantic_row',15]!=`p_firms' local results_ok = 0
            local semantic_row = `semantic_row'+1
        }
        forvalues target_rhs = 0/`=2*`probes'-1' {
            if `rhs_native'[`semantic_row',1]!=3 |                ///
                `rhs_native'[`semantic_row',2]!=floor(`target_rhs'/2) | ///
                `rhs_native'[`semantic_row',3]!=1+mod(`target_rhs',2) | ///
                `rhs_native'[`semantic_row',13]!=scalar(`native_full_tol') | ///
                `rhs_native'[`semantic_row',14]!=                 ///
                    cond("`nuisance'"=="joint",2,1) |           ///
                `rhs_native'[`semantic_row',15]!=                 ///
                    cond("`nuisance'"=="joint",`r_dimension',`p_firms') ///
                local results_ok = 0
            local semantic_row = `semantic_row'+1
        }
        if `semantic_row' != `expected_rhs_rows'+1 local results_ok = 0
        forvalues row = 1/`expected_rhs_rows' {
            forvalues column = 1/15 {
                if missing(`rhs_native'[`row',`column']) local results_ok = 0
            }
            foreach column in 1 2 3 4 5 8 9 10 11 12 14 15 {
                if `rhs_native'[`row',`column'] !=                   ///
                    floor(`rhs_native'[`row',`column']) local results_ok = 0
            }
            if `rhs_native'[`row',4]!=2 | `rhs_native'[`row',5]<0 | ///
                `rhs_native'[`row',5]>`maxiter' |                   ///
                `rhs_native'[`row',6]<0 | `rhs_native'[`row',7]<0 | ///
                `rhs_native'[`row',7]>`rhs_native'[`row',13] |      ///
                !inlist(`rhs_native'[`row',8],0,1) |                ///
                !inlist(`rhs_native'[`row',9],1,2) |                ///
                `rhs_native'[`row',8] != (`rhs_native'[`row',9]==1) | ///
                `rhs_native'[`row',10]<0 | `rhs_native'[`row',11]<0 | ///
                `rhs_native'[`row',12]<0 | `rhs_native'[`row',13]<=0 | ///
                !inlist(`rhs_native'[`row',14],1,2) |               ///
                `rhs_native'[`row',15]<=0 local results_ok = 0
            if `rhs_native'[`row',8] &                              ///
                (`rhs_native'[`row',5]!=0 | `rhs_native'[`row',6]!=0 | ///
                 `rhs_native'[`row',10]!=0 | `rhs_native'[`row',11]!=0 | ///
                 `rhs_native'[`row',12]!=0)                         ///
                local results_ok = 0
            local rhs_max_iterations = max(`rhs_max_iterations',`rhs_native'[`row',5])
            local rhs_max_reduced = max(`rhs_max_reduced',`rhs_native'[`row',6])
            local rhs_max_complete = max(`rhs_max_complete',`rhs_native'[`row',7])
        }
    }
    local receipt_numbers r_seed r_probes r_lev_acc r_tgt_acc r_req_route ///
        r_sel_route r_fallback r_fallback_err r_dimension r_lev_batch     ///
        r_tgt_batch r_rank_tol r_block_tol r_full_tol r_full_route        ///
        r_full_iter r_full_red r_full_complete r_full_zero r_lev_rhs      ///
        r_tgt_rhs r_max_red r_max_complete r_max_lev r_max_recip          ///
        r_accounting r_top_hi r_top_lo r_rng r_rhs_rows r_rhs_copy        ///
        r_algorithm_req r_algorithm_sel r_deletion r_nuisance r_parameters ///
        r_full_parameters r_correction_parameters r_native_info           ///
        r_native_inverse r_exact_flags r_actual_accounting r_rss          ///
        r_mem_limit r_input_copy r_prep_peak r_resident r_solver_setup    ///
        r_lev_phase r_tgt_phase r_result_bytes r_solve_peak r_command_peak ///
        r_rhs_schema r_engine_req r_engine_sel r_generic_flags            ///
        r_generic_controls r_control_rhs r_cr_rcond r_cr_small r_cr_large ///
        r_cr_proj r_cr_norm r_cr_fe r_cr_maxproj r_cr_tol r_cr_pcg        ///
        r_cr_gate r_control_basis r_control_forward r_schur_rcond          ///
        r_schur_relres r_rank_gap r_full_joint r_working_fit r_maker      ///
        r_canon_peak r_fit_peak r_geometry_peak r_generic_lev_peak        ///
        r_generic_tgt_peak r_maker_peak r_generic_result r_generic_peak   ///
        r_rhs_v2_copy r_cap_schema r_cap_profile r_batch_mode             ///
        r_stayers_mode r_target_mode r_deletion_source r_probeorder       ///
        r_wallseconds r_frequency r_physical_limit r_signature_hi r_signature_lo
    foreach value of local receipt_numbers {
        if missing(``value'') local results_ok = 0
    }
    if `results_ok' {
        local results_ok =                                         ///
            `r_seed'==`seed' & `r_probes'==`probes' &              ///
            `r_lev_acc'==`probes' & `r_tgt_acc'==`probes' &        ///
            `r_req_route'==2 & `r_sel_route'==2 &                  ///
            `r_fallback'==0 & `r_fallback_err'==0 &                ///
            `r_dimension'==`p_firms'+`control_count' &             ///
            `r_lev_batch'==`batch' & `r_tgt_batch'==`batch' &      ///
            `r_rank_tol'==`ranktol' & `r_block_tol'==`blocktol' &  ///
            `r_full_tol'==`expected_full_tol' & `r_full_route'==2 & ///
            `r_full_iter'>=0 & `r_full_iter'<=`maxiter' &          ///
            `r_full_red'>=0 & `r_full_complete'>=0 &               ///
            `r_full_complete'<=`r_full_tol' & inlist(`r_full_zero',0,1) & ///
            `r_lev_rhs'==`probes' & `r_tgt_rhs'==2*`probes' &      ///
            `r_max_red'==`rhs_max_reduced' &                       ///
            `r_max_complete'==`rhs_max_complete' &                 ///
            `r_max_complete'<=`r_full_tol' &                       ///
            `r_max_lev'>=0 & `r_max_lev'<1 & `r_max_recip'>=0 &   ///
            `r_max_recip'<=`maker_gate' &                          ///
            `r_rng'==1 & `r_rhs_rows'==`expected_rhs_rows' &       ///
            `r_rhs_copy'==0 & `r_rhs_schema'==2 &                  ///
            `r_algorithm_req'==2 & `r_algorithm_sel'==2 &          ///
            `r_deletion'==`deletion_code' & `r_nuisance'==`nuisance_code' & ///
            `r_parameters'==`expected_parameters' &                ///
            `r_full_parameters'==`expected_full_parameters' &      ///
            `r_correction_parameters'==`expected_parameters' &     ///
            `r_native_info'==0 & `r_native_inverse'==0 &           ///
            `r_exact_flags'==256 & `r_engine_req'==2 & `r_engine_sel'==2 & ///
            `r_generic_flags'==`expected_flags' &                   ///
            `r_generic_controls'==`control_count' &                ///
            `r_control_rhs'==`control_count' &                      ///
            `r_full_joint'==`r_full_complete' &                    ///
            `r_working_fit'>=0 & `r_working_fit'<=`r_full_tol' &   ///
            `r_maker'==`r_max_recip' & `r_rank_gap'>0 &            ///
            `r_schur_rcond'>0 & `r_schur_rcond'<=1 &               ///
            `r_schur_relres'>=0 & `r_control_basis'>=0 &           ///
            `r_control_forward'>=0 &                               ///
            scalar(`native_cr_tol')==max(`ranktol',1e-12) &        ///
            scalar(`native_cr_pcg')==1e-13 &                      ///
            scalar(`native_cr_gate')==1e-11 &                     ///
            `r_cap_schema'==2 & `r_cap_profile'==3 &               ///
            `r_batch_mode'==1 & `r_stayers_mode'==1 &              ///
            `r_target_mode'==`target_code' &                       ///
            `r_deletion_source'==`deletion_source_code' &          ///
            `r_probeorder'==0 & `r_wallseconds'==0 &               ///
            `r_frequency'==`frequency_code' &                      ///
            `r_physical_limit'==`physicallimit' &                  ///
            `r_signature_hi'==`cap_request_signature_hi' &         ///
            `r_signature_lo'==`cap_request_signature_lo' &         ///
            `r_mem_limit'==`p_mem_limit' & `r_input_copy'==`p_input_copy' & ///
            `r_prep_peak'==`p_prep_peak' & `r_resident'==`p_resident' & ///
            `r_rhs_v2_copy'==216*`expected_rhs_rows' &             ///
            `r_result_bytes'==`r_generic_result' &                 ///
            `r_result_bytes'>=`r_rhs_v2_copy' &                    ///
            `r_solver_setup'==max(`r_canon_peak',`r_fit_peak',`r_geometry_peak') & ///
            `r_generic_peak'==max(`r_canon_peak',`r_fit_peak',     ///
                `r_geometry_peak',`r_generic_lev_peak',            ///
                `r_generic_tgt_peak',`r_maker_peak',`r_generic_result') & ///
            `r_solve_peak'==`r_generic_peak' &                     ///
            `r_command_peak'==max(`r_prep_peak',`r_generic_peak') & ///
            `r_command_peak'<=`r_mem_limit' &                      ///
            abs(`r_actual_accounting'-`accounting_truth')<=        ///
                `roundoff_gate'*max(1,abs(`accounting_truth')) &   ///
            abs(`r_accounting'-`r_actual_accounting')<=            ///
                `roundoff_gate'*max(1,abs(`r_actual_accounting'))
    }
    if `results_ok' & `control_count'>0 {
        local results_ok =                                         ///
            scalar(`native_cr_rcond')>scalar(`native_cr_tol') &   ///
            scalar(`native_cr_rcond')<=1 &                        ///
            scalar(`native_cr_small')>0 & scalar(`native_cr_large')>0 & ///
            scalar(`native_cr_small')<=scalar(`native_cr_large') & ///
            scalar(`native_cr_rcond')==                           ///
                scalar(`native_cr_small')/scalar(`native_cr_large') & ///
            scalar(`native_cr_proj')>=0 & scalar(`native_cr_norm')>=0 & ///
            scalar(`native_cr_norm')<.25 & scalar(`native_cr_fe')>0 & ///
            scalar(`native_cr_maxproj')>=0 &                      ///
            scalar(`native_cr_maxproj')==scalar(`control_projection_max') & ///
            scalar(`native_cr_maxproj')<=scalar(`native_cr_gate')
    }
    if `results_ok' & `control_count'==0 {
        local results_ok =                                         ///
            scalar(`native_cr_rcond')==1 & scalar(`native_cr_small')==1 & ///
            scalar(`native_cr_large')==1 & scalar(`native_cr_proj')==0 & ///
            scalar(`native_cr_norm')==0 & scalar(`native_cr_fe')==0 & ///
            scalar(`native_cr_maxproj')==0 &                       ///
            scalar(`control_projection_max')==0
    }
    if !`results_ok' {
        capture quietly fevc_rust release `handle'
        capture quietly fevc_rust clear
        quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"     ///
            "Generic-JLA V6/RHS-V2 result receipts did not reconcile with the submitted request."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "result_reconcile"
        ereturn local backend_selected ""
        ereturn local rng_selected ""
        ereturn scalar rust_core_ready_flags = `rustcoreflags'
        ereturn scalar rust_support_flags = `rustsupportflags'
        exit 498
    }

    capture quietly _fevc_rust_public_call release `handle'
    if _rc {
        local failure_rc = _rc
        capture noisily _fevc_rust_abort, rc(`failure_rc')         ///
            handle(`handle') phase(release) norelease
        exit _rc
    }
    capture quietly fevc_rust snapshot
    if _rc {
        local failure_rc = _rc
        capture noisily _fevc_rust_abort, rc(`failure_rc') phase(release_snapshot)
        exit _rc
    }
    if r(state)!=0 | r(handle)!=0 {
        capture quietly fevc_rust clear
        quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"     ///
            "Generic-JLA release succeeded without an idle native snapshot."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "release_snapshot"
        ereturn local backend_selected ""
        ereturn local rng_selected ""
        exit 498
    }

    tempname plugin correction corrected kss_return posted mcse decomposition
    matrix colnames `raw_results' = worker_variance firm_variance   ///
        worker_firm_covariance total_variance
    matrix rownames `raw_results' = plugin bias_correction corrected numerical_mcse
    matrix `plugin' = `raw_results'[1,1..4]
    matrix `correction' = `raw_results'[2,1..4]
    matrix `corrected' = `raw_results'[3,1..4]
    matrix `kss_return' = `corrected'
    matrix `posted' = `corrected'
    matrix `mcse' = `raw_results'[4,1..4]
    foreach matrix_name in plugin correction corrected kss_return mcse {
        matrix colnames ``matrix_name'' = worker_variance firm_variance ///
            worker_firm_covariance total_variance
    }

    local target_outcome_variance = .
    local regression_outcome_variance = .
    tempvar target_sq frequency_sq
    quietly summarize `depvar' [aw=`target'] if `touse' & `target'>0, meanonly
    if !_rc & !missing(r(mean)) {
        local target_mean = r(mean)
        quietly generate double `target_sq' = `target'*(`depvar'-`target_mean')^2 if `touse'
        quietly summarize `target_sq' if `touse', meanonly
        local target_outcome_variance = r(sum)/`retained_target'
    }
    quietly summarize `depvar' [aw=`frequency'] if `touse', meanonly
    if !_rc & !missing(r(mean)) {
        local frequency_mean = r(mean)
        quietly generate double `frequency_sq' =                 ///
            `frequency'*(`depvar'-`frequency_mean')^2 if `touse'
        quietly summarize `frequency_sq' if `touse', meanonly
        local regression_outcome_variance = r(sum)/`retained_physical'
    }
    local residual_variance = `r_rss'/`retained_physical'
    local explained_variance = `regression_outcome_variance'-`residual_variance'
    local explained_share = .
    if `regression_outcome_variance'>0 local explained_share =      ///
        `explained_variance'/`regression_outcome_variance'
    matrix `decomposition' =                                     ///
        (`raw_results'[1,1],`raw_results'[2,1],`raw_results'[3,1] \ ///
         `raw_results'[1,2],`raw_results'[2,2],`raw_results'[3,2] \ ///
         2*`raw_results'[1,3],2*`raw_results'[2,3],2*`raw_results'[3,3] \ ///
         `raw_results'[1,4],`raw_results'[2,4],`raw_results'[3,4])
    matrix `decomposition' = `decomposition',J(4,4,.)
    if `target_outcome_variance'>0 {
        forvalues component = 1/4 {
            matrix `decomposition'[`component',4] =                 ///
                `decomposition'[`component',1]/`target_outcome_variance'
            matrix `decomposition'[`component',5] =                 ///
                `decomposition'[`component',3]/`target_outcome_variance'
        }
    }
    if `decomposition'[4,1]>0 {
        forvalues component = 1/4 {
            matrix `decomposition'[`component',6] =                 ///
                `decomposition'[`component',1]/`decomposition'[4,1]
        }
    }
    if `decomposition'[4,3]>0 {
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

    tempname rhs_public graph_receipt memory_receipt preparation_receipt
    tempname capability_receipt generic_receipt control_rank_receipt
    matrix `rhs_public' = J(`expected_rhs_rows',6,.)
    forvalues row = 1/`expected_rhs_rows' {
        local phase = `rhs_native'[`row',1]
        local probe = `rhs_native'[`row',2]
        local side = `rhs_native'[`row',3]
        local stage = cond(`phase'==5,1,cond(`phase'==1,2,         ///
            cond(`phase'==4,3,cond(`phase'==2,4,5))))
        local logical_rhs = cond(`probe'<0,1,cond(`phase'==3,      ///
            2*`probe'+`side',`probe'+1))
        local batch_start = cond(`probe'<0,1,floor(`probe'/`batch')*`batch'+1)
        matrix `rhs_public'[`row',1] = `stage'
        matrix `rhs_public'[`row',2] = `batch_start'
        matrix `rhs_public'[`row',3] = `logical_rhs'
        matrix `rhs_public'[`row',4] = `rhs_native'[`row',5]
        matrix `rhs_public'[`row',5] = `rhs_native'[`row',7]
        matrix `rhs_public'[`row',6] = inlist(`rhs_native'[`row',9],1,2)
    }
    matrix colnames `rhs_public' = stage batch_start rhs iterations ///
        relative_residual converged
    matrix `graph_receipt' = (`g_input_rows',`g_keep_rows',`g_input_mass', ///
        `g_keep_mass',`g_init_comp',`g_max_comp',`g_init_rows',`g_mover_rows', ///
        `g_init_edges',`g_keep_edges',`g_degree_removed',`g_art_removed', ///
        `g_bridge_units',`g_bridge_rows',`g_degree_iters',`g_art_iters', ///
        `g_bridge_iters',`g_fixed_iters')
    matrix colnames `graph_receipt' = input_rows retained_rows input_mass ///
        retained_mass initial_components maximum_components initial_component_rows ///
        mover_input_rows initial_deletion_edges retained_deletion_edges   ///
        insufficient_workers_removed articulation_workers_removed        ///
        bridge_units_removed bridge_rows_removed degree_iterations       ///
        articulation_iterations bridge_iterations fixed_point_iterations
    matrix `memory_receipt' = (`r_mem_limit',`r_input_copy',0,`r_rhs_v2_copy', ///
        `r_prep_peak',`r_resident',`r_solver_setup',`r_lev_phase',`r_tgt_phase', ///
        `r_result_bytes',`r_solve_peak',`r_command_peak')
    matrix colnames `memory_receipt' = limit caller_input_copy legacy_result_copy ///
        rhs_v2_copy preparation_peak prepared_resident solver_setup leverage_phase ///
        target_phase result solve_peak command_peak
    matrix `preparation_receipt' = (`p_input',`p_retained',`p_workers', ///
        `p_firms',`p_cells',`p_units',`p_strata',`p_target',`p_controls')
    matrix colnames `preparation_receipt' = input_rows retained_rows workers ///
        firms cells deletion_units target_strata target_weight_sum controls
    matrix `capability_receipt' = (`cap_struct_size',`cap_abi_version', ///
        `cap_request_schema',`cap_supported',`cap_reason_code',`cap_profile_code', ///
        `cap_algorithm_code',`cap_deletion_mode_code',`cap_nuisance_mode_code', ///
        `cap_solver_route_code',`cap_rng_contract_code',`cap_controls_count', ///
        `cap_frequency_use_code',`cap_engine_code',`cap_batch_mode_code', ///
        `cap_stayers_mode_code',`cap_target_weight_mode_code',       ///
        `cap_deletion_source_code',`cap_probeorder_supplied',        ///
        `cap_wallseconds_supplied',`cap_physical_limit',             ///
        `cap_request_signature_hi',`cap_request_signature_lo')
    matrix colnames `capability_receipt' = struct_size abi_version schema ///
        supported reason profile algorithm deletion nuisance route rng controls ///
        frequency engine batch stayers target deletion_source probeorder wall ///
        physical_limit signature_hi signature_lo
    matrix `generic_receipt' = (`r_generic_flags',`r_control_basis', ///
        `r_control_forward',`r_schur_rcond',`r_schur_relres',`r_rank_gap', ///
        `r_full_joint',`r_working_fit',`r_maker',`r_actual_accounting')
    matrix colnames `generic_receipt' = flags control_basis_relres   ///
        control_basis_forward_error control_schur_rcond control_schur_relres ///
        deletion_rank_gap full_joint_fit working_fit maker_relres accounting
    matrix `control_rank_receipt' = (scalar(`native_cr_rcond'),      ///
        scalar(`native_cr_small'),scalar(`native_cr_large'),       ///
        scalar(`native_cr_proj'),scalar(`native_cr_norm'),         ///
        scalar(`native_cr_fe'),scalar(`native_cr_maxproj'),        ///
        scalar(`native_cr_tol'),scalar(`native_cr_pcg'),           ///
        scalar(`native_cr_gate'))
    matrix colnames `control_rank_receipt' = rcond smallest_lower largest_upper ///
        projection_error normalization_error fe_information_lower max_projection ///
        effective_tolerance projection_pcg_tolerance projection_gate

    tempname prep_boundary_counts
    local prep_deletion_groups = cond("`deletionmode'"=="observation",0,1)
    matrix `prep_boundary_counts' = (2,`prep_deletion_groups',0,1,2, ///
        4,`p_input',2,`retained_count',0,0)
    matrix colnames `prep_boundary_counts' = initial_id_group_calls ///
        deletion_group_calls retained_id_group_calls semantic_group_calls ///
        stata_sort_calls graph_import_columns graph_import_rows     ///
        retained_map_columns retained_map_rows compression_import_columns ///
        compression_import_rows

    ereturn clear
    ereturn post `posted', obs(`retained_physical') esample(`touse') depname(`depvar')
    ereturn matrix results = `raw_results'
    ereturn matrix plugin = `plugin'
    ereturn matrix correction = `correction'
    ereturn matrix kss = `kss_return'
    ereturn matrix numerical_mcse = `mcse'
    ereturn matrix decomposition = `decomposition'
    ereturn matrix solver_rhs_diagnostics = `rhs_public'
    ereturn matrix rust_rhs_receipts = `rhs_native'
    ereturn matrix rust_graph_receipt = `graph_receipt'
    ereturn matrix rust_memory_receipt = `memory_receipt'
    ereturn matrix rust_preparation_receipt = `preparation_receipt'
    ereturn matrix rust_request_capability_receipt = `capability_receipt'
    ereturn matrix rust_generic_receipt = `generic_receipt'
    ereturn matrix rust_control_rank_receipt = `control_rank_receipt'
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
    ereturn scalar controls_count = `control_count'
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
    ereturn scalar probes = `r_probes'
    ereturn scalar max_leverage = `r_max_lev'
    ereturn scalar information_rcond = .
    ereturn scalar inverse_relres = .
    ereturn scalar solver_iterations = `rhs_max_iterations'
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
    ereturn scalar tolerance = `tolerance'
    ereturn scalar maxiter = `maxiter'
    ereturn scalar seed = `r_seed'
    ereturn scalar batch = `batch'
    ereturn scalar leverage_batch = `r_lev_batch'
    ereturn scalar target_batch = `r_tgt_batch'
    ereturn scalar memory_gib = `memorygib'
    ereturn scalar memory_forecast_bytes = `r_command_peak'
    ereturn scalar residual_acceptance_tolerance = scalar(`native_full_tol')
    ereturn scalar physical_limit = `physicallimit'
    ereturn scalar physical_limit_applied = 1
    ereturn scalar active_processors = c(processors)
    ereturn scalar route_code = `r_sel_route'
    ereturn scalar route_planned_rhs = `r_rhs_rows'
    ereturn scalar rust_requested_route = `r_req_route'
    ereturn scalar rust_selected_route = `r_sel_route'
    ereturn scalar rust_solver_fallback = `r_fallback'
    ereturn scalar rust_solver_fallback_error = `r_fallback_err'
    ereturn scalar rust_solver_dimension = `r_dimension'
    ereturn scalar rust_full_fit_route = `r_full_route'
    ereturn scalar rust_full_fit_iterations = `r_full_iter'
    ereturn scalar rust_full_fit_reduced_residual = scalar(`native_full_red')
    ereturn scalar rust_full_fit_complete_residual = scalar(`native_full_complete')
    ereturn scalar rust_full_fit_zero_rhs = `r_full_zero'
    ereturn scalar rust_working_fit_residual = scalar(`native_working')
    ereturn scalar rust_leverage_rhs_count = `r_lev_rhs'
    ereturn scalar rust_target_rhs_count = `r_tgt_rhs'
    ereturn scalar rust_control_rhs_count = `r_control_rhs'
    ereturn scalar rust_rhs_receipt_schema = `r_rhs_schema'
    ereturn scalar rust_max_reduced_residual = `r_max_red'
    ereturn scalar rust_max_complete_residual = `r_max_complete'
    ereturn scalar rust_leverage_probes_accepted = `r_lev_acc'
    ereturn scalar rust_target_probes_accepted = `r_tgt_acc'
    ereturn scalar rust_rank_tolerance = `r_rank_tol'
    ereturn scalar rust_block_tolerance = `r_block_tol'
    ereturn scalar rust_maker_relres = `r_maker'
    ereturn scalar rust_control_basis_relres = `r_control_basis'
    ereturn scalar rust_control_basis_forward_error = `r_control_forward'
    ereturn scalar rust_control_schur_rcond = `r_schur_rcond'
    ereturn scalar rust_control_schur_relres = `r_schur_relres'
    ereturn scalar rust_deletion_rank_gap = `r_rank_gap'
    ereturn scalar rust_actual_accounting_residual = `r_actual_accounting'
    ereturn scalar rust_generic_diagnostic_flags = `r_generic_flags'
    ereturn scalar rust_canonical_peak_bytes = `r_canon_peak'
    ereturn scalar rust_generic_fit_peak_bytes = `r_fit_peak'
    ereturn scalar rust_geometry_peak_bytes = `r_geometry_peak'
    ereturn scalar rust_generic_leverage_peak = `r_generic_lev_peak'
    ereturn scalar rust_generic_target_peak = `r_generic_tgt_peak'
    ereturn scalar rust_maker_peak_bytes = `r_maker_peak'
    ereturn scalar rust_generic_result_bytes = `r_generic_result'
    ereturn scalar rust_generic_peak_bytes = `r_generic_peak'
    ereturn scalar rust_rhs_v2_copy_bytes = `r_rhs_v2_copy'
    ereturn scalar rust_core_ready_flags = `rustcoreflags'
    ereturn scalar rust_support_flags = `rustsupportflags'
    ereturn scalar rust_topology_checksum_hi = `r_top_hi'
    ereturn scalar rust_topology_checksum_lo = `r_top_lo'
    ereturn scalar rust_rng_contract_code = `r_rng'
    ereturn scalar rust_requested_engine_code = `r_engine_req'
    ereturn scalar rust_selected_engine_code = `r_engine_sel'
    ereturn scalar rust_batch_mode_code = `r_batch_mode'
    ereturn scalar rust_stayers_mode_code = `r_stayers_mode'
    ereturn scalar rust_target_weight_mode_code = `r_target_mode'
    ereturn scalar rust_deletion_source_code = `r_deletion_source'
    ereturn scalar rust_probeorder_supplied = `r_probeorder'
    ereturn scalar rust_wallseconds_supplied = `r_wallseconds'
    ereturn scalar rust_frequency_use_code = `r_frequency'
    ereturn scalar rust_solve_physical_limit = `r_physical_limit'
    ereturn scalar rust_result_cap_schema = `r_cap_schema'
    ereturn scalar rust_result_cap_profile = `r_cap_profile'
    ereturn scalar rust_solve_signature_hi = `r_signature_hi'
    ereturn scalar rust_solve_signature_lo = `r_signature_lo'
    ereturn scalar rng_master_seed = `r_seed'
    ereturn scalar rng_leverage_probe_first = 1
    ereturn scalar rng_leverage_probe_last = `r_probes'
    ereturn scalar rng_target_probe_first = 1
    ereturn scalar rng_target_probe_last = `r_probes'
    ereturn scalar rust_cap_struct_size = `cap_struct_size'
    ereturn scalar rust_cap_abi_version = `cap_abi_version'
    ereturn scalar rust_cap_schema = `cap_request_schema'
    ereturn scalar rust_cap_supported = `cap_supported'
    ereturn scalar rust_cap_reason_code = `cap_reason_code'
    ereturn scalar rust_cap_profile_code = `cap_profile_code'
    ereturn scalar rust_cap_signature_hi = `cap_request_signature_hi'
    ereturn scalar rust_cap_signature_lo = `cap_request_signature_lo'
    ereturn scalar numerical_mcse_available = 1
    ereturn scalar backend_option_supplied = `backendsupplied'
    ereturn scalar rng_option_supplied = `rngsupplied'
    ereturn scalar algorithm_option_supplied = `algorithmsupplied'
    ereturn scalar engine_option_supplied = `enginesupplied'
    ereturn scalar preconditioner_option_supplied = `preconditionsupplied'
    ereturn scalar batch_option_supplied = `batchsupplied'
    ereturn scalar stayers_option_supplied = `stayerssupplied'
    ereturn scalar deletionid_option_supplied = `deletionidsupplied'
    ereturn scalar targetweight_option_supplied = `targetweightsupplied'
    ereturn local cmd "fevc"
    ereturn local cmdline `"`cmdline'"'
    ereturn local version "0.5.0-alpha.1"
    ereturn local model "linear"
    ereturn local correction_method "kss"
    ereturn local backend_requested "rust"
    ereturn local backend_selected "rust"
    ereturn local backend_routing_reason "fully explicit public generic-JLA route"
    ereturn local rng_requested "counter_v1"
    ereturn local rng_selected "counter_v1"
    ereturn local rng_contract "VCKSS-COUNTER-V1"
    ereturn local rng_implementation "stateless canonical Counter-V1 atoms"
    ereturn local rng_call_shape "one canonical atom plan per logical probe"
    ereturn local rng_runtime "native Rust Counter-V1"
    ereturn local rng_leverage_domain "leverage"
    ereturn local rng_target_domain "target"
    ereturn local algorithm "jla"
    ereturn local engine_requested "generic"
    ereturn local engine_selected "generic"
    ereturn local preconditioner_requested "diagonal"
    ereturn local preconditioner_selected "DIAGONAL"
    ereturn local routing_reason "explicit generic diagonal PCG route"
    ereturn local fallback_status "NOT_NEEDED"
    ereturn local fallback_message "generic-JLA completed on the requested route without fallback"
    ereturn local batch_requested "`batch'"
    ereturn local batch_routing_reason "caller supplied the required explicit batch width"
    ereturn local deletion "`deletionmode'"
    ereturn local nuisance "`nuisance'"
    ereturn local stayers "movers"
    ereturn local target_population = cond("`deletionmode'"=="match", ///
        "movers","retained observations")
    ereturn local sample_selection = cond("`deletionmode'"=="match", ///
        "MOVERS_DELETION_MULTIGRAPH_FIXED_POINT","MATLAB_LEAVEONEWORKER_COMPONENT")
    ereturn local connectedness_status = cond("`deletionmode'"=="match", ///
        "DELETION_UNIT_BRIDGE_FREE","LEAVE_ONE_WORKER_CONNECTED")
    ereturn local frequency_convention "literal physical copies"
    ereturn local targetweight_convention                           ///
        "explicit stored-row mass; default physical-observation mass"
    ereturn local probe_order                                     ///
        "observed IDs, outcome, controls, and per-copy target mass"
    ereturn local residual_normalization "complete weighted model residual"
    ereturn local quotient_convention "full_firm_zero_sum"
    ereturn local grounding_convention                              ///
        "last_firm_zero_after_quotient_with_complete residual checked"
    ereturn local inference "not implemented"
    ereturn local numerical_error "conditional probe MCSE and certified solver residuals"
    ereturn local inverse_diagnostics "NOT_APPLICABLE"
    ereturn local deletion_rank_certificate "generic maker/control-Schur rank gates"
    ereturn local route_api "VCKSS-NATIVE-GENERIC-V3-V6"
    ereturn local rust_capability_profile "JLA_GENERIC_COUNTER_V1"
    ereturn local rust_capability_reason "SUPPORTED"
    ereturn local status "KSS_POINT_ESTIMATES_ONLY"
    if "`nodisplay'" == "" _fevc_display
end

program define _vckss_proj_result_ok, rclass
    version 18.0
    args projection_b projection_V projection_V_naive             ///
        projection_columns prr_schema prr_columns prr_effect       ///
        prr_weight pr_effect pr_weight prr_cov_min prr_cov_max     ///
        prr_psd prr_proxy_min prr_proxy_max prr_max_iter           ///
        prr_max_reduced prr_max_complete expected_max_iter         ///
        expected_max_reduced expected_max_complete prr_full_tol    ///
        expected_full_tol prr_peak expected_peak memory_limit      ///
        prr_bytes expected_bytes

    local ok = rowsof(`projection_b')==1 &                        ///
        colsof(`projection_b')==`projection_columns' &             ///
        rowsof(`projection_V')==`projection_columns' &             ///
        colsof(`projection_V')==`projection_columns' &             ///
        rowsof(`projection_V_naive')==`projection_columns' &       ///
        colsof(`projection_V_naive')==`projection_columns'
    local projection_scale = 1e-30
    if `ok' {
        forvalues row = 1/`projection_columns' {
            if missing(`projection_b'[1,`row']) |                  ///
                missing(`projection_V'[`row',`row']) |             ///
                missing(`projection_V_naive'[`row',`row']) |       ///
                `projection_V'[`row',`row']<=0 |                   ///
                `projection_V_naive'[`row',`row']<0 local ok = 0
            local projection_scale = max(`projection_scale',      ///
                abs(`projection_V'[`row',`row']))
            forvalues column = 1/`projection_columns' {
                if missing(`projection_V'[`row',`column']) |       ///
                    missing(`projection_V_naive'[`row',`column']) | ///
                    abs(`projection_V'[`row',`column']-             ///
                        `projection_V'[`column',`row'])>            ///
                        1e-12*max(1,abs(`projection_V'[`row',`column'])) | ///
                    abs(`projection_V_naive'[`row',`column']-       ///
                        `projection_V_naive'[`column',`row'])>      ///
                        1e-12*max(1,abs(`projection_V_naive'[`row',`column'])) ///
                    local ok = 0
            }
        }
    }
    if !`ok' | `prr_schema'!=1 | `prr_columns'!=`projection_columns' | ///
        `prr_effect'!=`pr_effect' | `prr_weight'!=`pr_weight' |    ///
        `prr_cov_min' < -1e-8*`projection_scale' |                 ///
        `prr_cov_max' < `prr_cov_min' |                            ///
        `prr_psd'<0 | `prr_psd'>1e-8*`projection_scale' |         ///
        `prr_proxy_min'>`prr_proxy_max' |                          ///
        `prr_max_iter'!=`expected_max_iter' |                      ///
        `prr_max_reduced'!=`expected_max_reduced' |                ///
        `prr_max_complete'!=`expected_max_complete' |              ///
        `prr_max_complete'>`prr_full_tol' |                        ///
        `prr_full_tol'!=`expected_full_tol' |                      ///
        `prr_peak'!=`expected_peak' | `prr_peak'>`memory_limit' |  ///
        `prr_bytes'!=`expected_bytes' local ok = 0
    return scalar ok = `ok'
end

program define _fevc_rust_generic_planned, eclass sortpreserve
    version 18.0
    args depvar worker firm deletionvar frequency target touse nscope ///
        ncomplete nstayers nstayerrows probes batch seed tolerance    ///
        maxiter memorygib algorithm_requested enginerequested       ///
        backendsupplied rngsupplied                                  ///
        deletionidsupplied enginesupplied algorithmsupplied          ///
        preconditionsupplied batchsupplied stayerssupplied           ///
        rustcoreflags                                                ///
        rustsupportflags nodisplay deletionmode nuisance exactlimit  ///
        ranktol blocktol blocksizelimit physicallimit controls       ///
        targetweightsupplied frequencyused cmdline                  ///
        preconditionerrequested batchrequested wallsecondssupplied       ///
        wallseconds probeorder stayersmode originalstayer hybridcomplete ///
        rngrequested fullcmg tolerancesupplied                      ///
        project projecteffect projectweight level inference inferencemodel ///
        inferencesimulations inferenceseed

    if "`stayersmode'"=="" local stayersmode movers
    if "`rngrequested'"=="" local rngrequested counter_v1
    if "`fullcmg'"=="" local fullcmg = 0
    if "`tolerancesupplied'"=="" local tolerancesupplied = 0
    local projection_requested = (strtrim(`"`project'"') != "")
    local component_requested = ("`inference'" != "none" &       ///
        inlist(lower(strtrim("`inferencemodel'")),                 ///
            "structured_common", "structured_leverage"))
    tempvar native_input_order
    quietly generate double `native_input_order' = _n
    local implicit_match = (`fullcmg' == 1)
    foreach input in `depvar' `worker' `firm' `deletionvar'          ///
        `frequency' `target' `touse' `controls' `project' {
        confirm numeric variable `input'
    }
    if `projection_requested' &                              ///
        !("`algorithm_requested'"=="jla" &                   ///
          inlist("`deletionmode'","match","observation") &  ///
          ("`deletionmode'"=="match" |                       ///
              lower(strtrim("`stayersmode'"))=="movers") &   ///
          inlist(lower(strtrim("`stayersmode'")),"movers","both") & ///
          inlist(lower(strtrim("`projecteffect'")),"worker","firm") & ///
          inlist(lower(strtrim("`projectweight'")),"frequency","target")) {
        quietly _vckss_post_failure "RUST_OPTION_UNSUPPORTED" ///
            "The sparse project() route requires JLA; observation deletion remains mover-only, while match deletion supports movers or both."
        exit 498
    }
    if lower(strtrim("`stayersmode'"))=="both" {
        confirm numeric variable `originalstayer'
        confirm numeric variable `hybridcomplete'
    }
    local control_count : word count `controls'
    local probeorder_supplied_code = (strtrim("`probeorder'")!="")
    if `probeorder_supplied_code' confirm numeric variable `probeorder'
    local algorithm_requested = lower(strtrim("`algorithm_requested'"))
    local algorithm_expected_code = cond("`algorithm_requested'"=="auto",0, ///
        cond("`algorithm_requested'"=="exact",1,2))
    local algorithm_defer_expected = cond("`algorithm_requested'"=="auto",1,0)
    local engine_requested = lower(strtrim("`enginerequested'"))
    local engine_expected_code = cond("`engine_requested'"=="auto",0,2)
    local engine_defer_expected = cond("`algorithm_requested'"=="auto" | ///
        ("`algorithm_requested'"=="jla" & "`engine_requested'"=="auto"),1,0)
    local planned_engine_admissible = inlist("`engine_requested'","generic","auto")
    local deletion_code = cond("`deletionmode'"=="match",1,2)
    local nuisance_code = cond("`nuisance'"=="joint",1,2)
    local frequency_code = `frequencyused'
    local stayers_mode = lower(strtrim("`stayersmode'"))
    local stayers_code = cond("`stayers_mode'"=="both",2,             ///
        cond("`stayers_mode'"=="movers",1,.))
    local native_stayers = cond(`stayers_code'==2,"all","movers")
    local native_rng_contract = cond("`algorithm_requested'"=="exact", ///
        "none","counter_v1")
    local capability_rng_code = cond("`algorithm_requested'"=="exact",0,1)
    local target_code = `targetweightsupplied'
    local target_mode frequency
    if `target_code' local target_mode explicit
    local deletion_source cell
    local deletion_source_code = 1
    if "`deletionmode'" == "observation" {
        local deletion_source observation
        local deletion_source_code = 3
    }
    else if `deletionidsupplied' {
        local deletion_source matchid
        local deletion_source_code = 2
    }
    local preconditioner_requested = lower(strtrim("`preconditionerrequested'"))
    local batch_request = lower(strtrim("`batchrequested'"))
    local phase_batch_mode = cond("`batch_request'"=="auto","auto","explicit")
    local phase_batch_code = cond("`phase_batch_mode'"=="auto",0,1)
    local solve_batch = cond("`phase_batch_mode'"=="auto",0,`batch')
    local fallback_allowed = cond("`preconditioner_requested'"=="auto",1,0)
    local route_expected_code = cond("`preconditioner_requested'"=="auto",0, ///
        cond("`preconditioner_requested'"=="cmg",3,2))
    local wallseconds_supplied_code = real("`wallsecondssupplied'")
    local wallseconds_value = cond(`wallseconds_supplied_code',real("`wallseconds'"),0)
    if !inlist("`algorithm_requested'","jla","auto","exact") |   ///
        ("`algorithm_requested'"=="auto" &                         ///
            "`preconditioner_requested'"!="auto") |                ///
        ("`algorithm_requested'"=="exact" &                        ///
            ("`preconditioner_requested'"!="auto" |               ///
             "`batch_request'"!="auto" | `probeorder_supplied_code' | ///
             `wallseconds_supplied_code')) |                          ///
        !inlist("`engine_requested'","generic","auto") |          ///
        !`planned_engine_admissible' |                               ///
        !inlist("`preconditioner_requested'","auto","diagonal","cmg") | ///
        !inlist("`phase_batch_mode'","auto","explicit") |            ///
        !inlist(`wallseconds_supplied_code',0,1) |                   ///
        (`wallseconds_supplied_code' &                               ///
            (missing(`wallseconds_value') | `wallseconds_value'<=0)) | ///
        (!`wallseconds_supplied_code' & `wallseconds_value'!=0) |      ///
        !inlist(`stayers_code',1,2) |                                 ///
        (`stayers_code'==2 &                                         ///
            ("`deletionmode'"!="match" |                          ///
             `probeorder_supplied_code' | `wallseconds_supplied_code')) {
        quietly _vckss_post_failure "INVALID_TUNING"                ///
            "The planned Rust route received an invalid engine, route, batch, or wall tuple."
        exit 198
    }

    capture quietly fevc_rust clear
    if _rc {
        local failure_rc = _rc
        capture noisily _fevc_rust_abort, rc(`failure_rc') phase(clear_entry)
        exit _rc
    }

    capture quietly _fevc_rust_public_call requestcapability,       ///
        algorithm(`algorithm_requested') deletion(`deletionmode') nuisance(`nuisance') ///
        route(`preconditioner_requested') rngcontract(`native_rng_contract') ///
        controls(`control_count') frequencyused(`frequency_code')    ///
        engine(`engine_requested') batchmode(`phase_batch_mode')                ///
        leveragebatchmode(`phase_batch_mode')                        ///
        targetbatchmode(`phase_batch_mode') stayers(`native_stayers') ///
        targetweightmode(`target_mode') deletionsource(`deletion_source') ///
        probeordersupplied(`probeorder_supplied_code')               ///
        wallsecondssupplied(`wallseconds_supplied_code') ///
        fallback(`fallback_allowed') wallseconds(`wallseconds_value') ///
        physicallimit(`physicallimit')
    if _rc {
        capture quietly fevc_rust clear
        global VCKSS_ROUTE_BACKEND_REASON                            ///
            "planned Rust route could not obtain a V3 capability receipt"
        quietly _vckss_post_failure "RUST_BACKEND_UNAVAILABLE"      ///
            "The planned Rust capability query was unavailable; no native preparation was attempted."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "request_capability"
        ereturn local backend_selected ""
        ereturn local rng_selected ""
        ereturn scalar rust_core_ready_flags = `rustcoreflags'
        ereturn scalar rust_support_flags = `rustsupportflags'
        exit 498
    }
    foreach name in struct_size abi_version request_schema supported ///
        reason_code profile_code algorithm_code deletion_mode_code   ///
        nuisance_mode_code solver_route_code rng_contract_code       ///
        controls_count frequency_use_code engine_code batch_mode_code ///
        stayers_mode_code target_weight_mode_code deletion_source_code ///
        probeorder_supplied wallseconds_supplied physical_limit      ///
        request_signature_hi request_signature_lo                    ///
        leverage_batch_mode_code target_batch_mode_code              ///
        automatic_fallback_allowed wallseconds wall_advisory_only {
        local cap_`name' = r(`name')
    }
    // Stata local-macro names are capped at 32 characters.  Keep the
    // externally frozen receipt field names but store the longest fields in
    // short, explicit aliases rather than constructing cap_<field> names.
    local cap_alg_defer = r(algorithm_resolution_deferred)
    local cap_eng_defer = r(engine_resolution_deferred)
    local cap_route_defer = r(route_resolution_deferred)
    local cap_lev_defer = r(leverage_batch_deferred)
    local cap_tgt_defer = r(target_batch_resolution_deferred)
    local cap_reason_name `"`r(reason)'"'
    local cap_profile_name `"`r(profile)'"'
    local capability_ok = 1
    foreach name in struct_size abi_version request_schema supported ///
        reason_code profile_code algorithm_code deletion_mode_code   ///
        nuisance_mode_code solver_route_code rng_contract_code       ///
        controls_count frequency_use_code engine_code batch_mode_code ///
        stayers_mode_code target_weight_mode_code deletion_source_code ///
        probeorder_supplied wallseconds_supplied physical_limit      ///
        request_signature_hi request_signature_lo                    ///
        leverage_batch_mode_code target_batch_mode_code              ///
        automatic_fallback_allowed wall_advisory_only {
        if missing(`cap_`name'') | `cap_`name'' < 0 |               ///
            `cap_`name'' != floor(`cap_`name'') local capability_ok = 0
    }
    foreach value in cap_alg_defer cap_eng_defer cap_route_defer     ///
        cap_lev_defer cap_tgt_defer {
        if missing(``value'') | ``value'' < 0 |                     ///
            ``value'' != floor(``value'') local capability_ok = 0
    }
    if missing(`cap_wallseconds') | `cap_wallseconds'<0 local capability_ok = 0
    if `capability_ok' {
        local capability_ok =                                      ///
            `cap_struct_size'==160 & `cap_abi_version'==1 &        ///
            `cap_request_schema'==3 & `cap_supported'==1 &         ///
            `cap_reason_code'==0 & `cap_profile_code'==4 &         ///
            `cap_algorithm_code'==`algorithm_expected_code' &                              ///
            `cap_deletion_mode_code'==`deletion_code' &            ///
            `cap_nuisance_mode_code'==`nuisance_code' &            ///
            `cap_solver_route_code'==`route_expected_code' &       ///
            `cap_rng_contract_code'==`capability_rng_code' &       ///
            `cap_controls_count'==`control_count' &                ///
            `cap_frequency_use_code'==`frequency_code' &           ///
            `cap_engine_code'==`engine_expected_code' &            ///
            `cap_batch_mode_code'==`phase_batch_code' &            ///
            `cap_stayers_mode_code'==`stayers_code' &              ///
            `cap_target_weight_mode_code'==`target_code' &         ///
            `cap_deletion_source_code'==`deletion_source_code' &   ///
            `cap_probeorder_supplied'==`probeorder_supplied_code' & ///
            `cap_wallseconds_supplied'==`wallseconds_supplied_code' & ///
            `cap_physical_limit'==`physicallimit' &                ///
            `cap_leverage_batch_mode_code'==`phase_batch_code' &   ///
            `cap_target_batch_mode_code'==`phase_batch_code' &     ///
            `cap_automatic_fallback_allowed'==`fallback_allowed' & ///
            `cap_wallseconds'==`wallseconds_value' &               ///
            `cap_alg_defer'==`algorithm_defer_expected' &               ///
            `cap_eng_defer'==`engine_defer_expected' &             ///
            `cap_route_defer'==                                    ///
                ("`algorithm_requested'"!="exact" &               ///
                 "`preconditioner_requested'"=="auto") &           ///
            `cap_lev_defer'==                        ///
                ("`phase_batch_mode'"=="auto") &                   ///
            `cap_tgt_defer'==               ///
                ("`phase_batch_mode'"=="auto") &                   ///
            `cap_wall_advisory_only'==1 &                          ///
            `cap_request_signature_hi'<=4294967295 &               ///
            `cap_request_signature_lo'<=4294967295
    }
    if !`capability_ok' {
        capture quietly fevc_rust clear
        global VCKSS_ROUTE_BACKEND_REASON                            ///
            "planned Rust route rejected an inconsistent V3 capability receipt"
        quietly _vckss_post_failure "RUST_BACKEND_UNQUALIFIED"      ///
            "The V3 request-capability receipt did not reconcile with the materialized planned Rust tuple."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "request_capability_reconcile"
        ereturn local backend_selected ""
        ereturn local rng_selected ""
        ereturn scalar rust_core_ready_flags = `rustcoreflags'
        ereturn scalar rust_support_flags = `rustsupportflags'
        exit 498
    }

    tempvar rust_keep
    local probeorder_option
    if `probeorder_supplied_code' local probeorder_option probeorder(`probeorder')
    local implicit_match_option
    if `implicit_match' local implicit_match_option implicitmatch
    capture noisily _fevc_rust_public_call prepare `worker' `firm' ///
        `deletionvar' `depvar' `frequency' `target' `controls'      ///
        if `touse', cleanup generate(`rust_keep') memorygib(`memorygib') ///
        deletion(`deletionmode') `probeorder_option' `implicit_match_option'
    if _rc {
        local failure_rc = _rc
        capture noisily _fevc_rust_abort, rc(`failure_rc') phase(prepare)
        exit _rc
    }
    local handle = r(handle)
    foreach pair in input_rows:p_input retained_rows:p_retained workers:p_workers ///
        firms:p_firms cells:p_cells deletion_units:p_units            ///
        target_strata:p_strata target_weight_sum:p_target             ///
        memory_limit_bytes:p_mem_limit caller_copy_bytes:p_input_copy ///
        preparation_peak_forecast_bytes:p_prep_peak                   ///
        prepared_resident_bytes:p_resident controls_count:p_controls  ///
        graph_input_rows:g_input_rows graph_retained_rows:g_keep_rows ///
        graph_input_physical_mass:g_input_mass                        ///
        graph_retained_physical_mass:g_keep_mass                      ///
        graph_initial_components:g_init_comp                         ///
        graph_maximum_components:g_max_comp                          ///
        graph_initial_component_rows:g_init_rows                     ///
        graph_mover_input_rows:g_mover_rows                          ///
        graph_initial_deletion_edges:g_init_edges                    ///
        graph_retained_deletion_edges:g_keep_edges                   ///
        graph_degree_workers_removed:g_degree_removed                ///
        graph_artic_workers_removed:g_art_removed                    ///
        graph_bridge_units_removed:g_bridge_units                    ///
        graph_bridge_rows_removed:g_bridge_rows                      ///
        graph_degree_iterations:g_degree_iters                       ///
        graph_articulation_iterations:g_art_iters                    ///
        graph_bridge_iterations:g_bridge_iters                       ///
        graph_fixed_point_iterations:g_fixed_iters {
        gettoken returned localname : pair, parse(":")
        local localname = substr("`localname'",2,.)
        local `localname' = r(`returned')
    }
    quietly summarize `frequency' if `touse', meanonly
    local input_physical = r(sum)
    quietly replace `touse' = `touse' & `rust_keep'
    quietly count if `touse'
    local retained_count = r(N)
    quietly summarize `frequency' if `touse', meanonly
    local retained_physical = r(sum)
    quietly summarize `target' if `touse', meanonly
    local retained_target = r(sum)

    local preparation_ok = 1
    foreach value in handle p_input p_retained p_workers p_firms p_cells ///
        p_units p_strata p_mem_limit p_input_copy p_prep_peak p_resident ///
        p_controls g_input_rows g_keep_rows g_input_mass g_keep_mass     ///
        g_init_comp g_max_comp g_init_rows g_mover_rows g_init_edges    ///
        g_keep_edges g_degree_removed g_art_removed g_bridge_units      ///
        g_bridge_rows g_degree_iters g_art_iters g_bridge_iters g_fixed_iters {
        if missing(``value'') | ``value'' < 0 |                        ///
            ``value'' != floor(``value'') local preparation_ok = 0
    }
    local expected_input_copy = `p_input' *                         ///
        (6+`control_count'+`probeorder_supplied_code')*8
    local expected_prep_peak = `expected_input_copy'+               ///
        `p_input'*768+`p_input'*`control_count'*32+4096
    if `probeorder_supplied_code' local expected_prep_peak =         ///
        `expected_prep_peak' + `p_input'*16
    if `implicit_match' local expected_prep_peak =                  ///
        `expected_prep_peak' + `p_input'*16
    if missing(`p_target') | `p_target' <= 0 |                       ///
        missing(`retained_target') | `retained_target' <= 0 local preparation_ok = 0
    if `preparation_ok' {
        local preparation_ok =                                      ///
            `handle'>0 & `p_input'>0 & `p_retained'>0 &             ///
            `p_workers'>0 & `p_firms'>1 & `p_cells'>0 &             ///
            `p_units'>0 & `p_strata'>0 & `p_controls'==`control_count' & ///
            `p_input'==`ncomplete' & `p_retained'==`retained_count' & ///
            `p_input_copy'==`expected_input_copy' &                 ///
            `p_prep_peak'==`expected_prep_peak' & `p_resident'>0 & ///
            `p_input_copy'+`p_resident'<=`p_mem_limit' &            ///
            `p_prep_peak'<=`p_mem_limit' &                          ///
            `g_input_rows'==`ncomplete' & `g_keep_rows'==`retained_count' & ///
            `g_input_mass'==`input_physical' &                      ///
            `g_keep_mass'==`retained_physical' &                    ///
            `g_input_rows'>=`g_keep_rows' & `g_input_mass'>=`g_keep_mass' & ///
            `g_init_comp'>0 & `g_max_comp'>=`g_init_comp' &         ///
            `g_max_comp'<=`g_input_rows' & `g_init_rows'>0 &        ///
            `g_init_rows'<=`g_input_rows' & `g_init_rows'>=`g_mover_rows' & ///
            `g_mover_rows'>=`g_keep_rows' & `g_init_edges'>0 &      ///
            `g_init_edges'>=`g_keep_edges' &                        ///
            `g_degree_iters'<=`g_degree_removed' &                  ///
            `g_art_iters'<=`g_art_removed' &                        ///
            (`g_degree_iters'==0)==(`g_degree_removed'==0) &        ///
            (`g_art_iters'==0)==(`g_art_removed'==0) &              ///
            abs(`p_target'-`retained_target')<=1e-10*max(1,abs(`p_target'))
    }
    if `preparation_ok' & "`deletionmode'"=="match" {
        local preparation_ok =                                      ///
            `g_keep_edges'==`p_units' & `g_bridge_iters'<=`g_bridge_units' & ///
            (`g_bridge_iters'==0)==(`g_bridge_units'==0) &           ///
            (`g_bridge_iters'==0)==(`g_bridge_rows'==0) &            ///
            `g_bridge_units'<=`g_init_edges' &                       ///
            `g_bridge_rows'>=`g_bridge_units' & `g_bridge_rows'<=`g_mover_rows' & ///
            `g_fixed_iters'==`g_degree_iters'+`g_art_iters'+`g_bridge_iters'
    }
    if `preparation_ok' & "`deletionmode'"=="observation" {
        local preparation_ok =                                      ///
            `p_units'==`retained_physical' & `g_mover_rows'==`g_init_rows' & ///
            `g_bridge_units'==0 & `g_bridge_rows'==0 & `g_bridge_iters'==0 & ///
            `g_fixed_iters'==`g_degree_iters'+`g_art_iters'
    }
    if `preparation_ok' & `implicit_match' &                       ///
        !(`g_init_rows'==`g_mover_rows' & `g_degree_removed'==0 & ///
          `p_cells'==`p_units') {
        capture quietly fevc_rust release `handle'
        capture quietly fevc_rust clear
        quietly _vckss_post_failure "RUST_OPTION_UNSUPPORTED"     ///
            "CMG_FULL_V2 implicit-match preparation requires an all-mover sample with one deletion unit per worker-firm cell."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "implicit_match_reconcile"
        ereturn local backend_selected ""
        ereturn local rng_selected ""
        ereturn scalar rust_core_ready_flags = `rustcoreflags'
        ereturn scalar rust_support_flags = `rustsupportflags'
        exit 498
    }
    if !`preparation_ok' {
        capture quietly fevc_rust release `handle'
        capture quietly fevc_rust clear
        quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"     ///
            "Planned Rust preparation receipts did not reconcile with the validated Stata sample."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "preparation_reconcile"
        ereturn local backend_selected ""
        ereturn local rng_selected ""
        ereturn scalar rust_core_ready_flags = `rustcoreflags'
        ereturn scalar rust_support_flags = `rustsupportflags'
        exit 498
    }
    tempname projection_aug_ctx
    local projection_columns = 0
    local projection_persistent = 0
    local projection_augmentation_peak = 0
    local exact_family_possible =                                ///
        "`algorithm_requested'"=="exact" |                       ///
        ("`algorithm_requested'"=="auto" &                        ///
         "`engine_requested'"=="auto" &                           ///
         "`preconditioner_requested'"=="auto" &                   ///
         "`batch_request'"=="auto" &                              ///
         !`wallseconds_supplied_code')
    local solve_resident = `p_resident'
    local N_hybrid_stayers = 0
    local N_hybrid_stayer_rows = 0
    local N_hyb_singleton_drop = 0
    local N_hyb_unattached = 0
    local hybrid_stayer_physical = 0
    local hybrid_total_physical = `retained_physical'
    local hybrid_mover_target_mass = `retained_target'
    local hybrid_stayer_target_mass = 0
    local hybrid_target_mass = `retained_target'
    tempname stayer_aug_ctx
    if `stayers_code'==2 {
        tempvar retained_firm_member stayer_physical_total hybrid_stayer ///
            hybrid_touse hybrid_stayer_worker hybrid_firm hybrid_stayer_tag ///
            complete_worker_tag
        quietly egen byte `retained_firm_member' = max(`touse')       ///
            if `hybridcomplete', by(`firm')
        quietly egen double `stayer_physical_total' =                 ///
            total(`frequency') if `hybridcomplete', by(`worker')
        quietly generate byte `hybrid_stayer' =                       ///
            `originalstayer' & `retained_firm_member' &               ///
            `stayer_physical_total'>=2 if `hybridcomplete'
        quietly generate byte `hybrid_touse' = `touse' | `hybrid_stayer'
        quietly egen long `hybrid_firm' = group(`firm') if `hybrid_touse'
        quietly egen long `hybrid_stayer_worker' = group(`worker')    ///
            if `hybrid_stayer'
        quietly egen byte `hybrid_stayer_tag' = tag(`worker')         ///
            if `hybrid_stayer'
        quietly egen byte `complete_worker_tag' = tag(`worker')       ///
            if `hybridcomplete'
        quietly count if `hybrid_stayer_tag'
        local N_hybrid_stayers = r(N)
        quietly count if `hybrid_stayer'
        local N_hybrid_stayer_rows = r(N)
        quietly count if `complete_worker_tag' & `originalstayer' &   ///
            `retained_firm_member' & `stayer_physical_total'<2
        local N_hyb_singleton_drop = r(N)
        quietly count if `complete_worker_tag' & `originalstayer' &   ///
            !`retained_firm_member'
        local N_hyb_unattached = r(N)
        quietly summarize `frequency' if `hybrid_stayer', meanonly
        local hybrid_stayer_physical = r(sum)
        local hybrid_total_physical =                              ///
            `retained_physical'+`hybrid_stayer_physical'
        quietly summarize `target' if `hybrid_stayer', meanonly
        local hybrid_stayer_target_mass = r(sum)
        local hybrid_target_mass =                                 ///
            `retained_target'+`hybrid_stayer_target_mass'
        local hybrid_exact_dimension = `p_workers'+                 ///
            `N_hybrid_stayers'+`p_firms'-1+`control_count'
        if "`algorithm_requested'"=="exact" &                       ///
            `hybrid_exact_dimension'>`exactlimit' {
            capture quietly _fevc_rust_public_call release `handle'
            local failure_rc = _rc
            if `failure_rc' {
                capture noisily _fevc_rust_abort, rc(`failure_rc') ///
                    handle(`handle') phase(stayer_exact_limit_release) norelease
                exit _rc
            }
            capture quietly fevc_rust clear
            local failure_rc = _rc
            if `failure_rc' {
                capture noisily _fevc_rust_abort, rc(`failure_rc') ///
                    phase(stayer_exact_limit_clear) norelease
                exit _rc
            }
            quietly _vckss_post_failure "EXACT_SIZE_LIMIT"         ///
                "The combined mover-stayer identified coefficient dimension exceeds exact_limit()."
            ereturn scalar exact_identified_dimension =             ///
                `hybrid_exact_dimension'
            ereturn scalar exact_limit = `exactlimit'
            ereturn scalar rust_core_ready_flags = `rustcoreflags'
            ereturn scalar rust_support_flags = `rustsupportflags'
            exit 198
        }

        capture noisily _fevc_rust_public_call augmentstayers     ///
            `hybrid_firm' `hybrid_stayer_worker' `depvar'          ///
            `frequency' `target' `controls' if `hybrid_stayer',    ///
            handle(`handle')
        if _rc {
            local failure_rc = _rc
            capture noisily _fevc_rust_abort, rc(`failure_rc')     ///
                handle(`handle') phase(stayer_augmentation)
            exit _rc
        }
        foreach pair in struct_size:a_struct schema_version:a_schema ///
            mover_stored_rows:a_mover_rows stayer_stored_rows:a_stayer_rows ///
            combined_stored_rows:a_total_rows mover_physical_mass:a_mover_mass ///
            stayer_physical_mass:a_stayer_mass combined_physical_mass:a_total_mass ///
            mover_workers:a_mover_workers stayer_workers:a_stayer_workers ///
            combined_workers:a_total_workers firms:a_firms           ///
            mover_deletion_units:a_mover_del stayer_deletion_units:a_stayer_del ///
            combined_deletion_units:a_total_del mover_target_mass:a_mover_target ///
            stayer_target_mass:a_stayer_target combined_target_mass:a_total_target ///
            topology_checksum_hi:a_top_hi topology_checksum_lo:a_top_lo ///
            memory_limit_bytes:a_mem_limit caller_copy_bytes:a_copy   ///
            augmentation_peak_forecast_bytes:a_peak                   ///
            augmented_resident_bytes:a_resident                       ///
            total_prepared_resident_bytes:a_prepared {
            gettoken returned localname : pair, parse(":")
            local localname = substr("`localname'",2,.)
            local `localname' = r(`returned')
        }
        local expected_stayer_copy = `N_hybrid_stayer_rows' *       ///
            (5+`control_count')*8
        local augmentation_ok = 1
        foreach value in a_struct a_schema a_mover_rows a_stayer_rows ///
            a_total_rows a_mover_mass a_stayer_mass a_total_mass     ///
            a_mover_workers a_stayer_workers a_total_workers a_firms ///
            a_mover_del a_stayer_del a_total_del a_top_hi a_top_lo   ///
            a_mem_limit a_copy a_peak a_resident a_prepared {
            if missing(``value'') | ``value''<0 |                    ///
                ``value''!=floor(``value'') local augmentation_ok = 0
        }
        foreach value in a_mover_target a_stayer_target a_total_target {
            if missing(``value'') | ``value''<0 local augmentation_ok = 0
        }
        if `augmentation_ok' {
            local augmentation_ok = `a_struct'==192 & `a_schema'==1 & ///
                `a_mover_rows'==`p_retained' &                       ///
                `a_stayer_rows'==`N_hybrid_stayer_rows' &            ///
                `a_total_rows'==`p_retained'+`N_hybrid_stayer_rows' & ///
                `a_mover_mass'==`retained_physical' &                ///
                `a_stayer_mass'==`hybrid_stayer_physical' &          ///
                `a_total_mass'==`hybrid_total_physical' &            ///
                `a_mover_workers'==`p_workers' &                     ///
                `a_stayer_workers'==`N_hybrid_stayers' &             ///
                `a_total_workers'==`p_workers'+`N_hybrid_stayers' &   ///
                `a_firms'==`p_firms' & `a_mover_del'==`p_units' &    ///
                `a_stayer_del'==`hybrid_stayer_physical' &           ///
                `a_total_del'==`p_units'+`hybrid_stayer_physical' &   ///
                abs(`a_mover_target'-`retained_target')<=            ///
                    1e-10*max(1,abs(`retained_target')) &             ///
                abs(`a_stayer_target'-`hybrid_stayer_target_mass')<= ///
                    1e-10*max(1,abs(`hybrid_stayer_target_mass')) &   ///
                abs(`a_total_target'-`hybrid_target_mass')<=         ///
                    1e-10*max(1,abs(`hybrid_target_mass')) &          ///
                `a_mem_limit'==`p_mem_limit' & `a_copy'==`expected_stayer_copy' & ///
                `a_peak'<=`a_mem_limit' & `a_resident'>=0 &          ///
                `a_resident'<=`a_prepared' & `a_prepared'<=`a_mem_limit'
        }
        if !`augmentation_ok' {
            capture quietly fevc_rust release `handle'
            capture quietly fevc_rust clear
            quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"   ///
                "Rust stayer-augmentation receipts did not reconcile with the frozen complete-case sample."
            ereturn scalar native_error_code = .
            ereturn local native_error_phase "stayer_augmentation_reconcile"
            ereturn local backend_selected ""
            ereturn local rng_selected ""
            exit 498
        }
        local solve_resident = `a_prepared'
        matrix `stayer_aug_ctx' = (`a_struct',`a_schema',`a_mover_rows', ///
            `a_stayer_rows',`a_total_rows',`a_mover_mass',`a_stayer_mass', ///
            `a_total_mass',`a_mover_workers',`a_stayer_workers',       ///
            `a_total_workers',`a_firms',`a_mover_del',`a_stayer_del',  ///
            `a_total_del',`a_mover_target',`a_stayer_target',          ///
            `a_total_target',`a_top_hi',`a_top_lo',`a_mem_limit',      ///
            `a_copy',`a_peak',`a_resident',`a_prepared')
        matrix colnames `stayer_aug_ctx' = struct_size schema mover_rows ///
            stayer_rows total_rows mover_mass stayer_mass total_mass    ///
            mover_workers stayer_workers total_workers firms mover_del  ///
            stayer_del total_del mover_target stayer_target total_target ///
            topology_hi topology_lo memory_limit caller_copy augmentation_peak ///
            augmented_resident total_prepared_resident
    }
    local result_stored = `retained_count'
    local result_physical = `retained_physical'
    local result_workers = `p_workers'
    local result_cells = `p_cells'
    local result_units = `p_units'
    local result_strata = `p_strata'
    local result_target = `retained_target'
    local result_touse `touse'
    if `stayers_code'==2 {
        local result_stored = `retained_count'+`N_hybrid_stayer_rows'
        local result_physical = `hybrid_total_physical'
        local result_workers = `p_workers'+`N_hybrid_stayers'
        local result_cells = `p_cells'+`N_hybrid_stayers'
        local result_units = `p_units'+`hybrid_stayer_physical'
        local result_strata = `p_strata'+`hybrid_stayer_physical'
        local result_target = `hybrid_target_mass'
        local result_touse `hybrid_touse'
    }
    if `projection_requested' {
        local project_count : word count `project'
        local expected_projection_columns = `project_count' + 1
        local expected_projection_copy = `result_stored' * `project_count' * 8
        local expected_projection_persistent =                    ///
            (`result_workers'+`p_firms')*`expected_projection_columns'*8
        local expected_projection_square =                        ///
            `expected_projection_columns'^2*8
        local expected_projection_peak = `solve_resident' +       ///
            2*`expected_projection_copy' +                         ///
            4*`expected_projection_persistent' +                  ///
            8*`expected_projection_square' + 4096
        if `expected_projection_peak' > `p_mem_limit' {
            capture quietly fevc_rust release `handle'
            capture quietly fevc_rust clear
            quietly _vckss_post_failure "RESOURCE_LIMIT"          ///
                "The sparse projection augmentation exceeds memory_gib() before native projection work."
            ereturn local native_error_phase "projection_augmentation_memory"
            exit 498
        }
        if `stayers_code'==2 sort `hybrid_stayer' `native_input_order'
        else sort `native_input_order'
        capture noisily _fevc_rust_public_call augmentprojection `project' ///
            if `result_touse', handle(`handle') projecteffect(`projecteffect') ///
            projectweight(`projectweight') ranktolerance(`ranktol')
        if _rc {
            local failure_rc = _rc
            capture noisily _fevc_rust_abort, rc(`failure_rc')    ///
                handle(`handle') phase(projection_augmentation)
            exit _rc
        }
        foreach pair in schema_version:pr_schema rows:pr_rows      ///
            columns:pr_columns effect_code:pr_effect weight_code:pr_weight ///
            caller_copy_bytes:pr_copy                              ///
            augmentation_peak_forecast_bytes:pr_peak              ///
            projection_persistent_bytes:pr_persistent             ///
            total_prepared_resident_bytes:pr_prepared             ///
            gram_rcond:pr_gram_rcond gram_relres:pr_gram_relres   ///
            gram_original_relres:pr_gram_orig {
            gettoken returned localname : pair, parse(":")
            local localname = substr("`localname'",2,.)
            local `localname' = r(`returned')
        }
        local projection_attach_ok = 1
        foreach value in pr_schema pr_rows pr_columns pr_effect pr_weight ///
            pr_copy pr_peak pr_persistent pr_prepared {
            if missing(``value'') | ``value'' < 0 |                ///
                ``value'' != floor(``value'') local projection_attach_ok = 0
        }
        foreach value in pr_gram_rcond pr_gram_relres pr_gram_orig {
            if missing(``value'') | ``value'' < 0 local projection_attach_ok = 0
        }
        local expected_projection_effect =                          ///
            cond(lower("`projecteffect'")=="worker",1,2)
        local expected_projection_weight =                          ///
            cond(lower("`projectweight'")=="frequency",1,2)
        if `projection_attach_ok' {
            local projection_attach_ok = `pr_schema'==1 &          ///
                `pr_rows'==`result_stored' &                        ///
                `pr_columns'==`expected_projection_columns' &       ///
                `pr_effect'==`expected_projection_effect' &        ///
                `pr_weight'==`expected_projection_weight' &        ///
                `pr_copy'==`expected_projection_copy' &            ///
                `pr_persistent'==`expected_projection_persistent' & ///
                `pr_prepared'==`solve_resident'+`pr_persistent' &   ///
                `pr_peak'==`expected_projection_peak' &            ///
                `pr_peak'<=`p_mem_limit' & `pr_prepared'<=`p_mem_limit' & ///
                `pr_gram_rcond'>`ranktol' &                        ///
                `pr_gram_relres'<=max(1e-11,100*`ranktol') &       ///
                `pr_gram_orig'<=max(1e-11,100*`ranktol')
        }
        if !`projection_attach_ok' {
            capture quietly fevc_rust release `handle'
            capture quietly fevc_rust clear
            quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED" ///
                "Rust projection-augmentation receipts did not reconcile with the retained sample."
            ereturn local native_error_phase "projection_augmentation_reconcile"
            exit 498
        }
        local projection_columns = `pr_columns'
        local projection_persistent = `pr_persistent'
        local projection_augmentation_peak = `pr_peak'
        local solve_resident = `pr_prepared'
        local p_prep_peak = max(`p_prep_peak',`pr_peak')
        matrix `projection_aug_ctx' = (`pr_schema',`pr_rows',`pr_columns', ///
            `pr_effect',`pr_weight',`pr_copy',`pr_peak',`pr_persistent', ///
            `pr_prepared',`pr_gram_rcond',`pr_gram_relres',`pr_gram_orig')
        matrix colnames `projection_aug_ctx' = schema rows columns effect ///
            weight caller_copy augmentation_peak persistent prepared     ///
            gram_rcond gram_relres gram_original_relres
    }
    tempname component_aug_ctx
    local component_augmentation_peak = 0
    if `component_requested' {
        local component_model = lower(strtrim("`inferencemodel'"))
        local component_reference = cond("`inference'"=="q1","q1","q0")
        capture noisily _fevc_rust_component_attach `handle' `result_stored' ///
            `solve_resident' `p_mem_limit' `component_model'              ///
            `component_reference' `inferencesimulations' `batch'          ///
            `inferenceseed' `level' `ranktol' `component_aug_ctx'
        if _rc exit _rc
        local component_augmentation_peak = r(peak)
        local p_prep_peak = max(`p_prep_peak',r(peak))
    }
    local exact_selected_pre_rng = (`exact_family_possible') &   ///
        (`result_workers'+`p_firms'-1+`control_count'<=`exactlimit')
    local exact_plan_complexity =                              ///
        `result_workers'+`p_firms'-1+`control_count'
    if (`result_physical' > `physicallimit') &                    ///
        !(`exact_selected_pre_rng') {
        capture quietly fevc_rust release `handle'
        capture quietly fevc_rust clear
        quietly _vckss_post_failure "PHYSICAL_COPY_LIMIT"          ///
            "The combined retained physical mass exceeds physical_limit() before the planned native solve."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "physical_limit"
        ereturn local backend_selected ""
        ereturn local rng_selected ""
        ereturn scalar physical_limit = `physicallimit'
        ereturn scalar rust_core_ready_flags = `rustcoreflags'
        ereturn scalar rust_support_flags = `rustsupportflags'
        exit 498
    }
    local compressed_family_possible =                           ///
        "`engine_requested'"=="auto" & "`deletionmode'"=="match" & ///
        `control_count'==0
    tempname exact_prep_ctx exact_graph_ctx exact_cap_ctx
    if `exact_family_possible' {
        matrix `exact_prep_ctx' = (`p_input',`p_retained',`p_workers', ///
            `p_firms',`p_cells',`p_units',`p_strata',`p_target',`p_controls', ///
            `p_mem_limit',`p_input_copy',`p_prep_peak',`solve_resident')
        matrix colnames `exact_prep_ctx' = input_rows retained_rows ///
            workers firms cells deletion_units target_strata target_weight_sum ///
            controls memory_limit caller_input_copy preparation_peak          ///
            prepared_resident
        matrix `exact_graph_ctx' = (`g_input_rows',`g_keep_rows',      ///
            `g_input_mass',`g_keep_mass',`g_init_comp',`g_max_comp',   ///
            `g_init_rows',`g_mover_rows',`g_init_edges',`g_keep_edges', ///
            `g_degree_removed',`g_art_removed',`g_bridge_units',       ///
            `g_bridge_rows',`g_degree_iters',`g_art_iters',            ///
            `g_bridge_iters',`g_fixed_iters')
        matrix colnames `exact_graph_ctx' = input_rows retained_rows   ///
            input_mass retained_mass initial_components maximum_components ///
            initial_component_rows mover_input_rows initial_deletion_edges ///
            retained_deletion_edges insufficient_workers_removed           ///
            articulation_workers_removed bridge_units_removed              ///
            bridge_rows_removed degree_iterations articulation_iterations  ///
            bridge_iterations fixed_point_iterations
        matrix `exact_cap_ctx' = (`cap_struct_size',`cap_abi_version', ///
            `cap_request_schema',`cap_supported',`cap_reason_code',   ///
            `cap_profile_code',`cap_algorithm_code',                 ///
            `cap_deletion_mode_code',`cap_nuisance_mode_code',       ///
            `cap_solver_route_code',`cap_rng_contract_code',         ///
            `cap_controls_count',`cap_frequency_use_code',           ///
            `cap_engine_code',`cap_batch_mode_code',                 ///
            `cap_stayers_mode_code',`cap_target_weight_mode_code',   ///
            `cap_deletion_source_code',`cap_probeorder_supplied',    ///
            `cap_wallseconds_supplied',`cap_physical_limit',         ///
            `cap_request_signature_hi',`cap_request_signature_lo',   ///
            `cap_leverage_batch_mode_code',                          ///
            `cap_target_batch_mode_code',                            ///
            `cap_automatic_fallback_allowed',`cap_alg_defer',        ///
            `cap_eng_defer',`cap_route_defer',`cap_lev_defer',       ///
            `cap_tgt_defer',`cap_wall_advisory_only',`cap_wallseconds')
        matrix colnames `exact_cap_ctx' = struct_size abi_version schema ///
            supported reason profile algorithm deletion nuisance route rng ///
            controls frequency engine batch stayers target deletion_source ///
            probeorder wall physical_limit signature_hi signature_lo       ///
            leverage_batch target_batch fallback algorithm_deferred        ///
            engine_deferred route_deferred leverage_deferred target_deferred ///
            wall_advisory wallseconds
    }
    tempname compressed_prep_ctx compressed_graph_ctx compressed_cap_ctx
    if `compressed_family_possible' {
        matrix `compressed_prep_ctx' = (`p_input',`p_retained',`p_workers', ///
            `p_firms',`p_cells',`p_units',`p_strata',`p_target',`p_controls', ///
            `p_mem_limit',`p_input_copy',`p_prep_peak',`solve_resident')
        matrix colnames `compressed_prep_ctx' = input_rows retained_rows ///
            workers firms cells deletion_units target_strata target_weight_sum ///
            controls memory_limit caller_input_copy preparation_peak          ///
            prepared_resident
        matrix `compressed_graph_ctx' = (`g_input_rows',`g_keep_rows',      ///
            `g_input_mass',`g_keep_mass',`g_init_comp',`g_max_comp',       ///
            `g_init_rows',`g_mover_rows',`g_init_edges',`g_keep_edges',    ///
            `g_degree_removed',`g_art_removed',`g_bridge_units',           ///
            `g_bridge_rows',`g_degree_iters',`g_art_iters',                ///
            `g_bridge_iters',`g_fixed_iters')
        matrix colnames `compressed_graph_ctx' = input_rows retained_rows   ///
            input_mass retained_mass initial_components maximum_components ///
            initial_component_rows mover_input_rows initial_deletion_edges ///
            retained_deletion_edges insufficient_workers_removed           ///
            articulation_workers_removed bridge_units_removed              ///
            bridge_rows_removed degree_iterations articulation_iterations  ///
            bridge_iterations fixed_point_iterations
        matrix `compressed_cap_ctx' = (`cap_struct_size',`cap_abi_version', ///
            `cap_request_schema',`cap_supported',`cap_reason_code',         ///
            `cap_profile_code',`cap_algorithm_code',                       ///
            `cap_deletion_mode_code',`cap_nuisance_mode_code',             ///
            `cap_solver_route_code',`cap_rng_contract_code',               ///
            `cap_controls_count',`cap_frequency_use_code',`cap_engine_code', ///
            `cap_batch_mode_code',`cap_stayers_mode_code',                 ///
            `cap_target_weight_mode_code',`cap_deletion_source_code',      ///
            `cap_probeorder_supplied',`cap_wallseconds_supplied',          ///
            `cap_physical_limit',`cap_request_signature_hi',               ///
            `cap_request_signature_lo',`cap_eng_defer')
        matrix colnames `compressed_cap_ctx' = struct_size abi_version schema ///
            supported reason profile algorithm deletion nuisance route rng ///
            controls frequency engine batch stayers target deletion_source ///
            probeorder wall physical_limit signature_hi signature_lo       ///
            engine_deferred
    }

    capture noisily _fevc_rust_public_call solve `handle',         ///
        seed(`seed') probes(`probes') leveragebatch(`solve_batch')   ///
        targetbatch(`solve_batch') route(`preconditioner_requested') ///
        tolerance(`tolerance') maxiter(`maxiter') algorithm(`algorithm_requested')    ///
        deletion(`deletionmode') nuisance(`nuisance')               ///
        exactlimit(`exactlimit') blocksizelimit(`blocksizelimit')   ///
        ranktolerance(`ranktol') blocktolerance(`blocktol')          ///
        engine(`engine_requested') batchmode(`phase_batch_mode')                ///
        leveragebatchmode(`phase_batch_mode')                        ///
        targetbatchmode(`phase_batch_mode') stayers(`native_stayers') ///
        targetweightmode(`target_mode') deletionsource(`deletion_source') ///
        probeordersupplied(`probeorder_supplied_code')               ///
        wallsecondssupplied(`wallseconds_supplied_code') ///
        physicallimit(`physicallimit') capabilityschema(3)          ///
        capabilityprofile(4) frequencyused(`frequency_code')        ///
        signaturehi(`cap_request_signature_hi')                     ///
        signaturelo(`cap_request_signature_lo')                     ///
        fallback(`fallback_allowed') wallseconds(`wallseconds_value') ///
        fullcmg(`fullcmg') threads(`=c(processors)')                 ///
        tolerancesupplied(`tolerancesupplied')
    if _rc {
        local failure_rc = _rc
        local solve_failure_phase = cond(`exact_selected_pre_rng', ///
            "solve_exact","solve_jla")
        capture noisily _fevc_rust_abort, rc(`failure_rc')         ///
            handle(`handle') phase(`solve_failure_phase')
        exit _rc
    }

    local component_result_probes = 0
    if `component_requested' local component_result_probes = `inferencesimulations'
    capture noisily _fevc_rust_public_call result `handle',          ///
        componentinference(`component_requested')                    ///
        componentprobes(`component_result_probes')
    local result_export_rc = _rc
    if `result_export_rc' {
        local failure_rc = `result_export_rc'
        capture noisily _fevc_rust_abort, rc(`failure_rc')         ///
            handle(`handle') phase(result_export)
        exit _rc
    }

    local native_result_engine = r(selected_engine_code)
    local native_result_rhs_schema = r(rhs_receipt_schema)
    local native_perf_schema = r(performance_schema)
    local native_perf_flags = r(performance_flags)
    tempname rust_phase_profile
    matrix `rust_phase_profile' =                              ///
        (r(performance_ingest_ns)/1e9,                         ///
         r(performance_canonicalize_ns)/1e9,                   ///
         r(performance_graph_ns)/1e9,                          ///
         r(performance_compress_ns)/1e9,                       ///
         r(performance_plan_ns)/1e9,                           ///
         r(performance_stayer_ns)/1e9,                         ///
         r(performance_solve_ns)/1e9,                          ///
         r(performance_total_ns)/1e9)
    matrix colnames `rust_phase_profile' = ingest canonicalize graph ///
        compress plan stayer_augmentation solve native_total

    tempname component_point_snapshot
    if `component_requested' {
        matrix `component_point_snapshot' = r(result)[3,1..4]
    }
    tempname component_V_primitive component_V component_trace_mcse
    tempname component_spectrum component_q1_raw component_variance_summary
    tempname component_fold_diagnostics component_cv_diagnostics
    tempname component_inference_results component_q1_results
    tempname component_inference_receipt
    matrix `component_inference_receipt' = J(1,21,0)
    local ci_result_peak = 0
    if `component_requested' {
        capture noisily _fevc_rust_component_fetch `handle'             ///
            `component_reference' `component_model' `inferencesimulations' ///
            `p_mem_limit' `component_point_snapshot' `level'              ///
            `component_V_primitive' `component_V' `component_trace_mcse'  ///
            `component_spectrum' `component_q1_raw'                       ///
            `component_variance_summary' `component_fold_diagnostics'     ///
            `component_cv_diagnostics' `component_inference_receipt'      ///
            `component_inference_results' `component_q1_results'
        if _rc exit _rc
        local ci_result_peak = r(peak)
        // componentresult replaces r(). Re-export the immutable solved
        // result before any generic-result reconciliation or posting.
        capture noisily _fevc_rust_public_call result `handle',           ///
            componentinference(1) componentprobes(`inferencesimulations')
        if _rc {
            local failure_rc = _rc
            capture noisily _fevc_rust_abort, rc(`failure_rc')            ///
                handle(`handle') phase(component_result_refresh)
            exit _rc
        }
    }

    local full_cmg_active = (`fullcmg' == 1)
    local full_cmg_pre_reconciled = 0
    if `full_cmg_active' {
        if `native_result_engine'!=1 | `native_result_rhs_schema'!=1 {
            capture quietly fevc_rust release `handle'
            capture quietly fevc_rust clear
            quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED" ///
                "CMG_FULL_V2 returned an unexpected statistical result family."
            ereturn local native_error_phase "full_cmg_result_family"
            exit 498
        }
        capture quietly _fevc_rust_reconcile_comp_v7 `probes' `seed' ///
            `maxiter' `tolerance' `p_workers' `p_firms' `ranktol'    ///
            `blocktol' `algorithm_expected_code' `nuisance_code' `route_expected_code' ///
            `fallback_allowed' `phase_batch_code' `solve_batch'     ///
            `solve_batch' `target_code' `deletion_source_code'      ///
            `frequency_code' `p_mem_limit' `p_input_copy'           ///
            `p_prep_peak' `solve_resident' `cap_request_signature_hi' ///
            `cap_request_signature_lo' `physicallimit'              ///
            `probeorder_supplied_code' `wallseconds_supplied_code'  ///
            `wallseconds_value' 1 `tolerancesupplied' `stayers_code'
        local compressed_reconcile_rc = _rc
        if !`compressed_reconcile_rc' {
            local compressed_reconcile_ok = r(ok)
            local compressed_reconcile_detail `"`r(detail)'"'
        }
        if `compressed_reconcile_rc' | `compressed_reconcile_ok'!=1 {
            capture quietly fevc_rust release `handle'
            capture quietly fevc_rust clear
            quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED" ///
                "CMG_FULL_V2 compressed-result reconciliation failed: `compressed_reconcile_detail'."
            ereturn local native_error_phase "full_cmg_result_reconcile"
            exit 498
        }
        local full_cmg_pre_reconciled = 1
    }
    tempname full_cmg_receipt
    if `full_cmg_active' {
        capture noisily _fevc_rust_public_call fullcmgreceipt `handle'
        local full_cmg_receipt_rc = _rc
        if `full_cmg_receipt_rc' {
            local failure_rc = `full_cmg_receipt_rc'
            capture noisily _fevc_rust_abort, rc(`failure_rc')     ///
                handle(`handle') phase(full_cmg_receipt)
            exit _rc
        }
        foreach pair in generation:cmg_generation                  ///
            backend_identity:cmg_backend_identity                  ///
            platform_os:cmg_platform_os platform_arch:cmg_platform_arch ///
            batch_strategy_mask:cmg_batch_mask                     ///
            threads_requested:cmg_threads_requested                ///
            threads_used:cmg_threads_used                          ///
            maximum_concurrency:cmg_max_concurrency vertices:cmg_vertices ///
            edges:cmg_edges hierarchy_levels:cmg_levels            ///
            terminal_vertices:cmg_terminal graph_copy_bytes:cmg_graph_bytes ///
            hierarchy_bytes:cmg_hierarchy_bytes plan_bytes:cmg_plan_bytes ///
            workspace_bytes_each:cmg_ws_each workspace_pool_bytes:cmg_ws_pool ///
            admitted_peak_bytes:cmg_admitted_peak                  ///
            fit_effective_tolerance:cmg_fit_tol                    ///
            probe_effective_tolerance:cmg_probe_tol                ///
            fit_initial_inner_tolerance:cmg_fit_inner              ///
            probe_initial_inner_tolerance:cmg_probe_inner          ///
            refinement_attempts:cmg_refine_attempts                ///
            refined_columns:cmg_refined_columns batch_calls:cmg_batch_calls ///
            rhs_count:cmg_rhs_count serial_batches:cmg_serial_batches ///
            planned_batches:cmg_planned_batches                    ///
            across_rhs_batches:cmg_across_batches                  ///
            total_iterations:cmg_iterations                        ///
            total_operator_applications:cmg_operator_apps          ///
            total_preconditioner_apps:cmg_preconditioner_apps        ///
            maximum_reduced_residual:cmg_max_reduced               ///
            maximum_complete_residual:cmg_max_complete graph_ns:cmg_graph_ns ///
            hierarchy_plan_ns:cmg_hierarchy_ns rhs_ns:cmg_rhs_ns   ///
            solve_ns:cmg_solve_ns extraction_ns:cmg_extraction_ns ///
            preparation_peak_bytes:cmg_prep_peak                 ///
            prepared_persistent_bytes:cmg_prepared_bytes         ///
            non_cmg_command_peak_bytes:cmg_non_cmg_peak          ///
            pre_rng_forecast_bytes:cmg_pre_rng_forecast          ///
            actual_retained_bytes:cmg_actual_retained            ///
            allocator_allowance_bytes:cmg_allocator_allowance    ///
            maximum_batch_rhs:cmg_max_batch_rhs                  ///
            workspace_count:cmg_workspace_count {
            gettoken returned localname : pair, parse(":")
            local localname = substr("`localname'",2,.)
            local `localname' = r(`returned')
        }
        local cmg_backend `"`r(cmg_backend)'"'
        local cmg_source_commit `"`r(cmg_source_commit)'"'
        local expected_cmg_fit_tol = `tolerance'
        local expected_cmg_probe_tol = cond(`tolerancesupplied',`tolerance',1e-6)
        local full_cmg_receipt_ok =                              ///
            `cmg_generation'==`handle' & `cmg_backend_identity'==2 & ///
            `"`cmg_backend'"'=="CMG_FULL_V2" &                 ///
            `"`cmg_source_commit'"'==                           ///
                "92a12f2d572ca56b30a035220953f9dd4bced999" &  ///
            `cmg_threads_requested'==c(processors) &              ///
            `cmg_threads_used'==c(processors) &                    ///
            `cmg_fit_tol'==`expected_cmg_fit_tol' &                ///
            `cmg_probe_tol'==`expected_cmg_probe_tol' &            ///
            `cmg_fit_inner'==0.01*`expected_cmg_fit_tol' &         ///
            `cmg_probe_inner'==`expected_cmg_probe_tol' &          ///
            `cmg_admitted_peak'<=`p_mem_limit' &                   ///
            `cmg_pre_rng_forecast'<=`p_mem_limit' &                ///
            `cmg_admitted_peak'<=`cmg_pre_rng_forecast' &          ///
            `cmg_prepared_bytes'<=`cmg_non_cmg_peak' &             ///
            `cmg_actual_retained'>0 &                              ///
            `cmg_allocator_allowance'==floor(`cmg_actual_retained'/5) & ///
            `cmg_admitted_peak'==max(`cmg_prep_peak',              ///
                `cmg_non_cmg_peak'+`cmg_actual_retained'+          ///
                `cmg_allocator_allowance') &                       ///
            `cmg_max_batch_rhs'==64 &                              ///
            `cmg_workspace_count'>0 &                              ///
            `cmg_workspace_count'<=`cmg_threads_used' &            ///
            `cmg_rhs_count'==1+3*`probes' &                        ///
            `cmg_max_complete'<=max(1e-11,10*max(                  ///
                `expected_cmg_fit_tol',`expected_cmg_probe_tol'))
        if !`full_cmg_receipt_ok' {
            capture quietly fevc_rust release `handle'
            capture quietly fevc_rust clear
            quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED" ///
                "The CMG_FULL_V2 receipt did not reconcile with the explicit request."
            ereturn local native_error_phase "full_cmg_reconcile"
            ereturn local backend_selected ""
            ereturn local rng_selected ""
            exit 498
        }
        matrix `full_cmg_receipt' = (`cmg_backend_identity',       ///
            `cmg_platform_os',`cmg_platform_arch',`cmg_batch_mask', ///
            `cmg_threads_requested',`cmg_threads_used',            ///
            `cmg_max_concurrency',`cmg_vertices',`cmg_edges',      ///
            `cmg_levels',`cmg_terminal',`cmg_graph_bytes',         ///
            `cmg_hierarchy_bytes',`cmg_plan_bytes',`cmg_ws_each',  ///
            `cmg_ws_pool',`cmg_admitted_peak',`cmg_fit_tol',       ///
            `cmg_probe_tol',`cmg_fit_inner',`cmg_probe_inner',     ///
            `cmg_refine_attempts',`cmg_refined_columns',           ///
            `cmg_batch_calls',`cmg_rhs_count',`cmg_serial_batches', ///
            `cmg_planned_batches',`cmg_across_batches',            ///
            `cmg_iterations',`cmg_operator_apps',                  ///
            `cmg_preconditioner_apps',`cmg_max_reduced',           ///
            `cmg_max_complete',`cmg_graph_ns',`cmg_hierarchy_ns',  ///
            `cmg_rhs_ns',`cmg_solve_ns',`cmg_extraction_ns',       ///
            `cmg_prep_peak',`cmg_prepared_bytes',`cmg_non_cmg_peak', ///
            `cmg_pre_rng_forecast',`cmg_actual_retained',          ///
            `cmg_allocator_allowance',`cmg_max_batch_rhs',         ///
            `cmg_workspace_count')
        matrix colnames `full_cmg_receipt' = backend_id platform_os ///
            platform_arch batch_mask threads_requested threads_used ///
            maximum_concurrency vertices edges hierarchy_levels     ///
            terminal_vertices graph_bytes hierarchy_bytes plan_bytes ///
            workspace_each workspace_pool admitted_peak fit_tolerance ///
            probe_tolerance fit_inner probe_inner refinement_attempts ///
            refined_columns batch_calls rhs_count serial_batches    ///
            planned_batches across_rhs_batches iterations operator_apps ///
            preconditioner_apps max_reduced max_complete graph_ns   ///
            hierarchy_ns rhs_ns solve_ns extraction_ns preparation_peak ///
            prepared_persistent non_cmg_peak pre_rng_forecast       ///
            actual_retained allocator_allowance maximum_batch_rhs   ///
            workspace_count
    }

    if missing(`native_result_engine') | missing(`native_result_rhs_schema') | ///
        `native_perf_schema'!=1 | missing(`native_perf_flags') |              ///
        mod(`native_perf_flags',4)!=3 {
        capture quietly fevc_rust release `handle'
        capture quietly fevc_rust clear
        quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"       ///
            "The planned V4 result omitted its engine-family receipt."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "result_family"
        ereturn local backend_selected ""
        ereturn local rng_selected ""
        ereturn scalar rust_core_ready_flags = `rustcoreflags'
        ereturn scalar rust_support_flags = `rustsupportflags'
        exit 498
    }
    if `native_result_engine'==3 {
        if !`exact_family_possible' | `native_result_rhs_schema'!=0 {
            capture quietly fevc_rust release `handle'
            capture quietly fevc_rust clear
            quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"   ///
                "The native planner selected an inadmissible exact result family."
            ereturn scalar native_error_code = .
            ereturn local native_error_phase "result_family"
            ereturn local backend_selected ""
            ereturn local rng_selected ""
            ereturn scalar rust_core_ready_flags = `rustcoreflags'
            ereturn scalar rust_support_flags = `rustsupportflags'
            exit 498
        }
        capture quietly _fevc_rust_reconcile_exact_v7              ///
            `algorithm_expected_code' `engine_expected_code'         ///
            `deletion_code' `nuisance_code' `p_workers' `p_firms'    ///
            `control_count' `ranktol' `blocktol' `tolerance'         ///
            `exactlimit' `p_mem_limit' `p_input_copy' `p_prep_peak'  ///
            `solve_resident' `cap_request_signature_hi'              ///
            `cap_request_signature_lo' `physicallimit'               ///
            `wallseconds_supplied_code' `wallseconds_value'          ///
            `target_code' `deletion_source_code' `frequency_code'    ///
            `stayers_code' `exact_plan_complexity'
        local exact_reconcile_rc = _rc
        if `exact_reconcile_rc' {
            capture quietly fevc_rust release `handle'
            capture quietly fevc_rust clear
            quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"   ///
                "The exact V7 result reconciler was unavailable."
            ereturn scalar native_error_code = .
            ereturn local native_error_phase "exact_reconcile"
            ereturn local backend_selected ""
            ereturn local rng_selected ""
            ereturn scalar rust_core_ready_flags = `rustcoreflags'
            ereturn scalar rust_support_flags = `rustsupportflags'
            exit 498
        }
        local exact_reconcile_ok = r(ok)
        local exact_reconcile_detail `"`r(detail)'"'
        local exact_reconcile_family `"`r(result_family)'"'
        local exact_reconcile_schema `"`r(execution_plan_schema)'"'
        if `exact_reconcile_ok'!=1 |                               ///
            `"`exact_reconcile_family'"'!="exact" |                ///
            `"`exact_reconcile_schema'"'!="VCKSS-EXECUTION-PLAN-V1" {
            capture quietly fevc_rust release `handle'
            capture quietly fevc_rust clear
            quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"   ///
                "Exact V7 result reconciliation failed: `exact_reconcile_detail'."
            ereturn scalar native_error_code = .
            ereturn local native_error_phase "exact_reconcile"
            ereturn local backend_selected ""
            ereturn local rng_selected ""
            ereturn scalar rust_core_ready_flags = `rustcoreflags'
            ereturn scalar rust_support_flags = `rustsupportflags'
            exit 498
        }
        tempname hybrid_result_ctx hybrid_raw_ctx hybrid_source_ctx
        if `stayers_code'==2 {
            capture noisily _fevc_rust_capture_stayers             ///
                `handle' `control_count' `nuisance' `tolerance'      ///
                `ranktol' `blocktol' `stayer_aug_ctx'
            local stayer_capture_rc = _rc
            if `stayer_capture_rc' {
                capture noisily _fevc_rust_abort, rc(`stayer_capture_rc') ///
                    handle(`handle') phase(stayer_result_export)
                exit _rc
            }
            local stayer_capture_ok = r(ok)
            local stayer_capture_detail `"`r(detail)'"'
            if `stayer_capture_ok'!=1 {
                capture quietly fevc_rust release `handle'
                capture quietly fevc_rust clear
                quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED" ///
                    "Exact stayer-hybrid result reconciliation failed: `stayer_capture_detail'."
                ereturn scalar native_error_code = .
                ereturn local native_error_phase "stayer_result_reconcile"
                ereturn local backend_selected ""
                ereturn local rng_selected ""
                exit 498
            }
            matrix `hybrid_result_ctx' = r(receipt)
            matrix `hybrid_raw_ctx' = r(result)
            matrix `hybrid_source_ctx' = r(correction_source)
        }
        local exact_nodisplay `"`nodisplay'"'
        if `stayers_code'==2 local exact_nodisplay nodisplay
        capture noisily _fevc_rust_post_exact_v7 `handle' `depvar' ///
            `frequency' `target' `touse' `nscope' `ncomplete'       ///
            `nstayers' `nstayerrows' `probes' `batch' `seed'        ///
            `tolerance' `maxiter' `memorygib' `algorithm_requested' ///
            `engine_requested' `backendsupplied' `rngsupplied'      ///
            `rngrequested' `deletionidsupplied' `enginesupplied'    ///
            `algorithmsupplied' `preconditionsupplied' `batchsupplied' ///
            `stayerssupplied' `rustcoreflags' `rustsupportflags'    ///
            `"`exact_nodisplay'"' `deletionmode' `nuisance' `ranktol' ///
            `blocktol' `exactlimit' `physicallimit'                 ///
            `preconditioner_requested' `batch_request'              ///
            `targetweightsupplied' `"`cmdline'"'                    ///
            `wallseconds_supplied_code' `wallseconds_value'         ///
            `exact_prep_ctx' `exact_graph_ctx' `exact_cap_ctx'      ///
            `stayers_mode' `exact_plan_complexity'
        local exact_post_rc = _rc
        if !`exact_post_rc' & `stayers_code'==2 {
            capture noisily _fevc_rust_post_stayer_hybrid `depvar' ///
                `frequency' `target' `hybrid_touse' `nuisance'       ///
                `N_hybrid_stayers' ///
                `N_hybrid_stayer_rows' `N_hyb_singleton_drop'       ///
                `N_hyb_unattached' `stayer_aug_ctx' `hybrid_result_ctx' ///
                `hybrid_raw_ctx' `hybrid_source_ctx' `"`nodisplay'"'
            local exact_post_rc = _rc
            if `exact_post_rc' ereturn clear
        }
        if !`exact_post_rc' {
            ereturn matrix rust_phase_profile = `rust_phase_profile'
            ereturn local rust_phase_profile_schema "VCKSS-NATIVE-PHASE-PERF-V1"
            ereturn local rust_phase_profile_units "seconds"
            ereturn scalar rust_phase_profile_flags = `native_perf_flags'
        }
        exit `exact_post_rc'
    }
    if `native_result_engine'==1 {
        if !`compressed_family_possible' | `native_result_rhs_schema'!=1 {
            capture quietly fevc_rust release `handle'
            capture quietly fevc_rust clear
            quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"   ///
                "The native planner selected an inadmissible compressed result family."
            ereturn scalar native_error_code = .
            ereturn local native_error_phase "result_family"
            ereturn local backend_selected ""
            ereturn local rng_selected ""
            ereturn scalar rust_core_ready_flags = `rustcoreflags'
            ereturn scalar rust_support_flags = `rustsupportflags'
            exit 498
        }
        // fullcmgreceipt must be consumed while the handle is still live, but
        // it necessarily replaces r(). Re-export the immutable solved result
        // so the compressed reconciler remains immediately adjacent to its
        // poster, exactly as on every legacy compressed path.
        if `full_cmg_active' {
            capture noisily _fevc_rust_public_call result `handle'
            local full_cmg_result_refresh_rc = _rc
            if `full_cmg_result_refresh_rc' {
                local failure_rc = `full_cmg_result_refresh_rc'
                capture noisily _fevc_rust_abort, rc(`failure_rc') ///
                    handle(`handle') phase(full_cmg_result_refresh)
                exit _rc
            }
        }
        if `full_cmg_active' | !`full_cmg_pre_reconciled' {
            capture quietly _fevc_rust_reconcile_comp_v7 `probes' `seed' ///
                `maxiter' `tolerance' `p_workers' `p_firms' `ranktol' ///
                `blocktol' `algorithm_expected_code' `nuisance_code' `route_expected_code' ///
                `fallback_allowed' `phase_batch_code' `solve_batch' ///
                `solve_batch' `target_code' `deletion_source_code'  ///
                `frequency_code' `p_mem_limit' `p_input_copy'       ///
                `p_prep_peak' `solve_resident' `cap_request_signature_hi' ///
                `cap_request_signature_lo' `physicallimit'          ///
                `probeorder_supplied_code' `wallseconds_supplied_code' ///
                `wallseconds_value' `full_cmg_active' `tolerancesupplied' ///
                `stayers_code'
            local compressed_reconcile_rc = _rc
        }
        if `compressed_reconcile_rc' {
            capture quietly fevc_rust release `handle'
            capture quietly fevc_rust clear
            quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"   ///
                "The compressed V7 result reconciler was unavailable."
            ereturn scalar native_error_code = .
            ereturn local native_error_phase "compressed_reconcile"
            ereturn local backend_selected ""
            ereturn local rng_selected ""
            ereturn scalar rust_core_ready_flags = `rustcoreflags'
            ereturn scalar rust_support_flags = `rustsupportflags'
            exit 498
        }
        local compressed_reconcile_ok = r(ok)
        local compressed_reconcile_detail `"`r(detail)'"'
        if `compressed_reconcile_ok'!=1 {
            capture quietly fevc_rust release `handle'
            capture quietly fevc_rust clear
            quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"   ///
                "Compressed V7 result reconciliation failed: `compressed_reconcile_detail'."
            ereturn scalar native_error_code = .
            ereturn local native_error_phase "compressed_reconcile"
            ereturn local backend_selected ""
            ereturn local rng_selected ""
            ereturn scalar rust_core_ready_flags = `rustcoreflags'
            ereturn scalar rust_support_flags = `rustsupportflags'
            exit 498
        }
        capture noisily _fevc_rust_post_comp_v7 `handle' `depvar'   ///
            `frequency' `target' `touse' `nscope' `ncomplete'       ///
            `nstayers' `nstayerrows' `tolerance' `maxiter'          ///
            `memorygib' `engine_requested' `backendsupplied'        ///
            `rngsupplied' `deletionidsupplied' `enginesupplied'     ///
            `algorithmsupplied' `preconditionsupplied' `batchsupplied' ///
            `stayerssupplied' `rustcoreflags' `rustsupportflags'    ///
            `"`nodisplay'"' `nuisance' `physicallimit'              ///
            `targetweightsupplied' `"`cmdline'"'                    ///
            `preconditioner_requested' `batch_request'              ///
            `compressed_prep_ctx' `compressed_graph_ctx'            ///
            `compressed_cap_ctx' `fullcmg' `stayers_mode'
        local compressed_post_rc = _rc
        if !`compressed_post_rc' & `probeorder_supplied_code' {
            ereturn local probe_order                               ///
                "observed IDs, outcome, controls, target mass, and optional tie-breaker"
        }
        if !`compressed_post_rc' {
            // A request for the combined default can still select the
            // compressed mover engine when sample construction finds no
            // eligible attached stayers.  The numerical result is then
            // exactly the mover result, but its public metadata must retain
            // the requested combined-population contract.
            if `stayers_code'==2 {
                tempname zero_hybrid_plugin zero_hybrid_correction
                tempname zero_hybrid_kss zero_hybrid_results
                tempname zero_hybrid_decomposition zero_hybrid_source
                tempname zero_hybrid_accounting
                matrix `zero_hybrid_plugin' = e(plugin)
                matrix `zero_hybrid_correction' = e(correction)
                matrix `zero_hybrid_kss' = e(kss)
                matrix `zero_hybrid_results' = e(results)
                matrix `zero_hybrid_decomposition' = e(decomposition)
                matrix `zero_hybrid_source' = J(2,4,.)
                matrix rownames `zero_hybrid_source' = mover_match ///
                    stayer_observation
                matrix colnames `zero_hybrid_source' = worker_variance ///
                    firm_variance worker_firm_covariance total_variance
                matrix `zero_hybrid_accounting' =                     ///
                    (`retained_count',`retained_physical',`p_workers', ///
                        `retained_target',`p_units' \                   ///
                     0,0,0,0,0 \                                      ///
                     `retained_count',`retained_physical',`p_workers', ///
                        `retained_target',`p_units')
                matrix rownames `zero_hybrid_accounting' = mover stayer total
                matrix colnames `zero_hybrid_accounting' = stored_rows ///
                    physical_observations worker_levels target_weight_mass ///
                    deletion_units
                ereturn matrix stayer_hybrid_plugin = `zero_hybrid_plugin'
                ereturn matrix stayer_hybrid_correction =             ///
                    `zero_hybrid_correction'
                ereturn matrix stayer_hybrid_kss = `zero_hybrid_kss'
                ereturn matrix stayer_hybrid_results = `zero_hybrid_results'
                ereturn matrix stayer_hybrid_decomposition =          ///
                    `zero_hybrid_decomposition'
                ereturn matrix stayer_hybrid_correction_source =      ///
                    `zero_hybrid_source'
                ereturn matrix stayer_hybrid_sample_accounting =      ///
                    `zero_hybrid_accounting'
                ereturn scalar stayer_hybrid_N_stored = e(N_stored)
                ereturn scalar stayer_hybrid_N_physical = e(N_physical)
                ereturn scalar stayer_hybrid_worker_levels = e(worker_levels)
                ereturn scalar stayer_hybrid_firm_levels = e(firm_levels)
                ereturn scalar stayer_hybrid_parameters = e(parameters)
                ereturn scalar stayer_hybrid_full_parameters =       ///
                    e(full_parameters)
                ereturn scalar stayer_hybrid_corr_parameters =       ///
                    e(correction_parameters)
                ereturn scalar stayer_hybrid_deletion_units = e(deletion_units)
                ereturn scalar stayer_hybrid_target_mass = e(target_weight_sum)
                ereturn scalar stayer_hybrid_max_leverage = e(max_leverage)
                ereturn scalar stayer_hybrid_information_rcond =     ///
                    e(information_rcond)
                ereturn scalar stayer_hybrid_inverse_relres = e(inverse_relres)
                ereturn scalar stayer_hybrid_weighted_rss = e(weighted_rss)
                ereturn scalar stayer_hybrid_target_y_variance =     ///
                    e(target_outcome_variance)
                ereturn scalar stayer_hybrid_N_stayers = 0
                ereturn scalar stayer_hybrid_N_stayer_rows = 0
                ereturn scalar stayer_hybrid_N_stayer_physical = 0
                ereturn scalar stayer_hybrid_N_singleton_drop =      ///
                    `N_hyb_singleton_drop'
                ereturn scalar stayer_hybrid_N_unattached = `N_hyb_unattached'
                ereturn scalar stayer_hybrid_mover_target_mass =     ///
                    `retained_target'
                ereturn scalar stayer_hybrid_stayer_target_mass = 0
                ereturn local stayers "both"
                ereturn local target_population                      ///
                    "retained movers plus eligible attached stayers"
                ereturn local sample_selection                       ///
                    "MOVERS_FIXED_POINT_PLUS_ELIGIBLE_ATTACHED_STAYERS"
                ereturn local stayer_hybrid_status "CONVERGED"
                ereturn local stayer_hybrid_target_population        ///
                    "retained movers plus eligible original one-firm stayers attached to retained mover firms"
                ereturn local stayer_hybrid_deletion                 ///
                    "mover matches plus stayer physical observations"
                ereturn local stayer_hybrid_assumption               ///
                    "mover correction is match-robust; stayer correction is not match-robust"
                ereturn local stayer_hybrid_sample_rule              ///
                    "original one-firm stayers; retained mover firm; physical T>=2; graph-dropped movers excluded"
                ereturn local stayer_hybrid_esample                  ///
                    "e(sample) marks retained movers plus eligible attached stayers"
                ereturn local stayer_hybrid_targetweight             ///
                    "pooled stored-row target mass; explicit mass is not multiplied by frequency"
                ereturn local stayer_hybrid_nuisance "`nuisance'"
            }
            ereturn matrix rust_phase_profile = `rust_phase_profile'
            ereturn local rust_phase_profile_schema "VCKSS-NATIVE-PHASE-PERF-V1"
            ereturn local rust_phase_profile_units "seconds"
            ereturn scalar rust_phase_profile_flags = `native_perf_flags'
            if `full_cmg_active' {
                ereturn matrix full_cmg_receipt = `full_cmg_receipt'
                ereturn scalar resource_peak_bytes = `cmg_pre_rng_forecast'
                ereturn scalar memory_forecast_bytes = `cmg_pre_rng_forecast'
                ereturn local cmg_backend "`cmg_backend'"
                ereturn local cmg_source_commit "`cmg_source_commit'"
                ereturn scalar cmg_threads_requested = `cmg_threads_requested'
                ereturn scalar cmg_threads_used = `cmg_threads_used'
                ereturn scalar cmg_admitted_peak_bytes = `cmg_admitted_peak'
                ereturn scalar cmg_max_complete_residual = `cmg_max_complete'
                ereturn scalar cmg_refinement_attempts = `cmg_refine_attempts'
                ereturn scalar cmg_refined_columns = `cmg_refined_columns'
            }
        }
        exit `compressed_post_rc'
    }
    if `native_result_engine'!=2 | `native_result_rhs_schema'!=2 {
        capture quietly fevc_rust release `handle'
        capture quietly fevc_rust clear
        quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"       ///
            "The planned V4 result returned an unknown engine/result family."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "result_family"
        ereturn local backend_selected ""
        ereturn local rng_selected ""
        ereturn scalar rust_core_ready_flags = `rustcoreflags'
        ereturn scalar rust_support_flags = `rustsupportflags'
        exit 498
    }

    tempname raw_results rhs_native
    matrix `raw_results' = r(result)
    matrix `rhs_native' = r(rhs_receipts)
    tempname native_full_red native_full_complete native_full_tol native_working
    tempname native_cr_rcond native_cr_small native_cr_large native_cr_proj
    tempname native_cr_norm native_cr_fe native_cr_maxproj native_cr_tol
    tempname native_cr_pcg native_cr_gate
    scalar `native_full_red' = r(full_fit_reduced_residual)
    scalar `native_full_complete' = r(full_fit_complete_residual)
    scalar `native_full_tol' = r(full_residual_tolerance)
    scalar `native_working' = r(generic_working_fit_residual)
    scalar `native_cr_rcond' = r(control_rank_rcond)
    scalar `native_cr_small' = r(control_rank_smallest_lower)
    scalar `native_cr_large' = r(control_rank_largest_upper)
    scalar `native_cr_proj' = r(control_rank_projection_error)
    scalar `native_cr_norm' = r(control_rank_normalization_err)
    scalar `native_cr_fe' = r(control_rank_fe_info_lower)
    scalar `native_cr_maxproj' = r(control_rank_max_projection)
    scalar `native_cr_tol' = r(control_rank_effective_tolerance)
    scalar `native_cr_pcg' = r(control_rank_projection_pcg_tol)
    scalar `native_cr_gate' = r(control_rank_projection_gate)
    foreach pair in seed:r_seed probes:r_probes                       ///
        leverage_probes_accepted:r_lev_acc target_probes_accepted:r_tgt_acc ///
        requested_route:r_req_route selected_route:r_sel_route       ///
        solver_fallback:r_fallback solver_fallback_error:r_fallback_err ///
        solver_dimension:r_dimension leverage_batch_width:r_lev_batch ///
        target_batch_width:r_tgt_batch rank_tolerance:r_rank_tol      ///
        block_tolerance:r_block_tol full_residual_tolerance:r_full_tol ///
        full_fit_route:r_full_route full_fit_iterations:r_full_iter  ///
        full_fit_reduced_residual:r_full_red                         ///
        full_fit_complete_residual:r_full_complete full_fit_zero_rhs:r_full_zero ///
        leverage_rhs_count:r_lev_rhs target_rhs_count:r_tgt_rhs      ///
        max_reduced_residual:r_max_red max_complete_residual:r_max_complete ///
        max_leverage:r_max_lev max_reciprocal_residual:r_max_recip  ///
        accounting_residual:r_accounting topology_checksum_hi:r_top_hi ///
        topology_checksum_lo:r_top_lo rng_contract_code:r_rng        ///
        rhs_receipt_rows:r_rhs_rows caller_result_copy_bytes:r_rhs_copy ///
        requested_algorithm_code:r_algorithm_req selected_algorithm_code:r_algorithm_sel ///
        deletion_mode_code:r_deletion nuisance_mode_code:r_nuisance ///
        parameters:r_parameters full_parameters:r_full_parameters    ///
        correction_parameters:r_correction_parameters               ///
        information_rcond:r_native_info inverse_relative_residual:r_native_inverse ///
        exact_diagnostic_flags:r_exact_flags actual_accounting_residual:r_actual_accounting ///
        weighted_rss:r_rss memory_limit_bytes:r_mem_limit            ///
        caller_copy_bytes:r_input_copy preparation_peak_forecast_bytes:r_prep_peak ///
        prepared_resident_bytes:r_resident solver_setup_forecast_bytes:r_solver_setup ///
        leverage_phase_forecast_bytes:r_lev_phase target_phase_forecast_bytes:r_tgt_phase ///
        result_forecast_bytes:r_result_bytes solve_peak_forecast_bytes:r_solve_peak ///
        command_peak_forecast_bytes:r_command_peak rhs_receipt_schema:r_rhs_schema ///
        requested_engine_code:r_engine_req selected_engine_code:r_engine_sel ///
        generic_diagnostic_flags:r_generic_flags generic_controls_count:r_generic_controls ///
        control_projection_rhs_count:r_control_rhs control_rank_rcond:r_cr_rcond ///
        control_rank_smallest_lower:r_cr_small control_rank_largest_upper:r_cr_large ///
        control_rank_projection_error:r_cr_proj control_rank_normalization_err:r_cr_norm ///
        control_rank_fe_info_lower:r_cr_fe control_rank_max_projection:r_cr_maxproj ///
        control_rank_effective_tolerance:r_cr_tol control_rank_projection_pcg_tol:r_cr_pcg ///
        control_rank_projection_gate:r_cr_gate generic_control_basis_relres:r_control_basis ///
        generic_control_basis_fwd_err:r_control_forward generic_control_schur_rcond:r_schur_rcond ///
        generic_control_schur_relres:r_schur_relres generic_deletion_rank_gap:r_rank_gap ///
        full_joint_fit_complete_residual:r_full_joint generic_working_fit_residual:r_working_fit ///
        generic_maker_relative_residual:r_maker canonicalization_peak_bytes:r_canon_peak ///
        generic_fit_peak_forecast_bytes:r_fit_peak geometry_peak_forecast_bytes:r_geometry_peak ///
        generic_leverage_peak_bytes:r_generic_lev_peak generic_target_peak_bytes:r_generic_tgt_peak ///
        maker_peak_forecast_bytes:r_maker_peak generic_result_forecast_bytes:r_generic_result ///
        generic_peak_forecast_bytes:r_generic_peak rhs_v2_caller_copy_bytes:r_rhs_v2_copy ///
        capability_schema:r_cap_schema capability_profile:r_cap_profile ///
        batch_mode_code:r_batch_mode stayers_mode_code:r_stayers_mode ///
        target_weight_mode_code:r_target_mode deletion_source_code:r_deletion_source ///
        probeorder_supplied:r_probeorder wallseconds_supplied:r_wallseconds ///
        frequency_use_code:r_frequency physical_limit:r_physical_limit ///
        request_signature_hi:r_signature_hi request_signature_lo:r_signature_lo ///
        projection_columns:r_proj_columns                            ///
        projection_peak_forecast_bytes:r_proj_peak                  ///
        batch_lev_mode:r_lev_batch_mode                              ///
        batch_tgt_mode:r_tgt_batch_mode plan_struct:r_plan_struct    ///
        plan_schema:r_plan_schema plan_route_schema:r_plan_route_schema ///
        plan_resolved:r_plan_resolved plan_frozen:r_plan_frozen      ///
        plan_applicability:r_plan_applicability                      ///
        plan_alg_req:r_plan_alg_req plan_alg_sel:r_plan_alg_sel      ///
        plan_eng_req:r_plan_eng_req plan_eng_sel:r_plan_eng_sel      ///
        plan_route_req:r_plan_route_req plan_route_sel:r_plan_route_sel ///
        plan_route_fallback:r_plan_route_fallback                    ///
        plan_route_error:r_plan_route_error plan_rhs:r_plan_rhs      ///
        plan_full_dim:r_plan_full_dim batch_lev_sel:r_batch_lev_sel  ///
        batch_tgt_sel:r_batch_tgt_sel batch_command:r_batch_command  ///
        batch_nonbatched:r_batch_nonbatched                          ///
        batch_lev_onebytes:r_batch_lev_onebytes                      ///
        batch_lev_selbytes:r_batch_lev_selbytes                      ///
        batch_tgt_onebytes:r_batch_tgt_onebytes                      ///
        batch_tgt_selbytes:r_batch_tgt_selbytes                      ///
        ctr_complete:r_ctr_complete plan_res_rng_hi:r_pre_rng_hi     ///
        plan_res_rng_lo:r_pre_rng_lo                                ///
        wall_requested:r_wall_requested_value                       ///
        wall_forecast:r_wall_forecast_value wall_advisory:r_wall_advisory_value ///
        wall_margin:r_wall_margin_value mem_command:r_plan_mem_command {
        gettoken returned localname : pair, parse(":")
        local localname = substr("`localname'",2,.)
        local `localname' = r(`returned')
    }

    tempname projection_b projection_V projection_V_naive projection_results
    tempname projection_diagnostics projection_solver_rhs
    foreach value in schema columns effect weight cov_min cov_max psd      ///
        proxy_min proxy_max max_iter max_reduced max_complete full_tol peak bytes {
        local prr_`value' = 0
    }
    if `projection_requested' {
        capture noisily _fevc_rust_public_call projectionresult `handle', ///
            columns(`projection_columns')
        if _rc {
            local failure_rc = _rc
            capture noisily _fevc_rust_abort, rc(`failure_rc')    ///
                handle(`handle') phase(projection_result_export)
            exit _rc
        }
        matrix `projection_b' = r(coefficients)
        matrix `projection_V' = r(covariance)
        matrix `projection_V_naive' = r(naive_covariance)
        foreach pair in schema_version:prr_schema columns:prr_columns ///
            effect_code:prr_effect weight_code:prr_weight          ///
            covariance_minimum_eigenvalue:prr_cov_min              ///
            covariance_maximum_eigenvalue:prr_cov_max psd_cleanup:prr_psd ///
            proxy_minimum:prr_proxy_min proxy_maximum:prr_proxy_max ///
            maximum_iterations:prr_max_iter                        ///
            maximum_reduced_residual:prr_max_reduced               ///
            maximum_complete_residual:prr_max_complete             ///
            full_residual_tolerance:prr_full_tol                   ///
            projection_peak_forecast_bytes:prr_peak result_bytes:prr_bytes {
            gettoken returned localname : pair, parse(":")
            local localname = substr("`localname'",2,.)
            local `localname' = r(`returned')
        }
    }

    local expected_full_parameters = `result_workers'+`p_firms'-1+`control_count'
    local expected_parameters = `expected_full_parameters'
    if "`nuisance'" == "fixedoffset" {
        local expected_parameters = `result_workers'+`p_firms'-1
    }
    local expected_rhs_rows = `control_count'+1+                ///
        (`control_count'>0 & "`nuisance'"=="fixedoffset")+3*`probes' + ///
        `projection_columns'
    local expected_full_tol = max(1e-11,10*`tolerance')
    local expected_flags = 126+(`control_count'>0)
    local maker_gate = max(1e-10,100*`ranktol')
    local roundoff_gate = 1e-12
    local results_ok = rowsof(`raw_results')==4 & colsof(`raw_results')==4 & ///
        rowsof(`rhs_native')==`expected_rhs_rows' & colsof(`rhs_native')==15
    local rhs_max_iterations = 0
    local rhs_max_reduced = 0
    local rhs_max_complete = 0
    local projection_rhs_max_iterations = 0
    local projection_rhs_max_reduced = 0
    local projection_rhs_max_complete = 0
    local accounting_truth = 0
    tempname control_projection_max
    scalar `control_projection_max' = 0
    if `results_ok' {
        forvalues row = 1/4 {
            forvalues column = 1/4 {
                if missing(`raw_results'[`row',`column']) local results_ok = 0
            }
        }
        forvalues row = 1/3 {
            local row_identity = abs(`raw_results'[`row',4]-          ///
                `raw_results'[`row',1]-`raw_results'[`row',2]-       ///
                2*`raw_results'[`row',3])
            local accounting_truth = max(`accounting_truth',`row_identity')
        }
        forvalues column = 1/4 {
            if abs(`raw_results'[1,`column']-`raw_results'[2,`column']- ///
                `raw_results'[3,`column']) >                         ///
                1e-10*max(1,abs(`raw_results'[1,`column'])) local results_ok = 0
        }
        if `control_count' > 0 {
            forvalues row = 1/`control_count' {
                if `rhs_native'[`row',1]!=5 |                     ///
                    `rhs_native'[`row',2]!=`row'-1 |              ///
                    `rhs_native'[`row',3]!=0 |                    ///
                    `rhs_native'[`row',13]!=scalar(`native_cr_gate') | ///
                    `rhs_native'[`row',14]!=1 |                   ///
                    `rhs_native'[`row',15]!=`p_firms' local results_ok = 0
                scalar `control_projection_max' = max(            ///
                    scalar(`control_projection_max'),             ///
                    `rhs_native'[`row',7])
            }
        }
        local semantic_row = `control_count'+1
        if `rhs_native'[`semantic_row',1]!=1 |                    ///
            `rhs_native'[`semantic_row',2]!=-1 |                  ///
            `rhs_native'[`semantic_row',3]!=0 |                   ///
            `rhs_native'[`semantic_row',5]!=`r_full_iter' |       ///
            `rhs_native'[`semantic_row',6]!=scalar(`native_full_red') | ///
            `rhs_native'[`semantic_row',7]!=scalar(`native_full_complete') | ///
            `rhs_native'[`semantic_row',8]!=`r_full_zero' |       ///
            `rhs_native'[`semantic_row',13]!=scalar(`native_full_tol') | ///
            `rhs_native'[`semantic_row',14]!=2 |                  ///
            `rhs_native'[`semantic_row',15]!=`r_dimension' local results_ok = 0
        local semantic_row = `semantic_row'+1
        if `control_count'>0 & "`nuisance'"=="fixedoffset" {
            if `rhs_native'[`semantic_row',1]!=4 |                ///
                `rhs_native'[`semantic_row',2]!=-1 |              ///
                `rhs_native'[`semantic_row',3]!=0 |               ///
                `rhs_native'[`semantic_row',7]!=scalar(`native_working') | ///
                `rhs_native'[`semantic_row',13]!=scalar(`native_full_tol') | ///
                `rhs_native'[`semantic_row',14]!=1 |              ///
                `rhs_native'[`semantic_row',15]!=`p_firms' local results_ok = 0
            local semantic_row = `semantic_row'+1
        }
        forvalues probe = 0/`=`probes'-1' {
            if `rhs_native'[`semantic_row',1]!=2 |                ///
                `rhs_native'[`semantic_row',2]!=`probe' |         ///
                `rhs_native'[`semantic_row',3]!=0 |               ///
                `rhs_native'[`semantic_row',13]!=scalar(`native_full_tol') | ///
                `rhs_native'[`semantic_row',14]!=1 |              ///
                `rhs_native'[`semantic_row',15]!=`p_firms' local results_ok = 0
            local semantic_row = `semantic_row'+1
        }
        forvalues target_rhs = 0/`=2*`probes'-1' {
            if `rhs_native'[`semantic_row',1]!=3 |                ///
                `rhs_native'[`semantic_row',2]!=floor(`target_rhs'/2) | ///
                `rhs_native'[`semantic_row',3]!=1+mod(`target_rhs',2) | ///
                `rhs_native'[`semantic_row',13]!=scalar(`native_full_tol') | ///
                `rhs_native'[`semantic_row',14]!=                 ///
                    cond("`nuisance'"=="joint",2,1) |           ///
                `rhs_native'[`semantic_row',15]!=                 ///
                    cond("`nuisance'"=="joint",`r_dimension',`p_firms') ///
                local results_ok = 0
            local semantic_row = `semantic_row'+1
        }
        if `projection_columns' > 0 {
            local projection_first_row = `semantic_row'
            forvalues projection_rhs = 0/`=`projection_columns'-1' {
                if `rhs_native'[`semantic_row',1]!=6 |            ///
                    `rhs_native'[`semantic_row',2]!=`projection_rhs' | ///
                    `rhs_native'[`semantic_row',3]!=0 |           ///
                    `rhs_native'[`semantic_row',13]!=scalar(`native_full_tol') | ///
                    `rhs_native'[`semantic_row',14]!=              ///
                        cond("`nuisance'"=="joint",2,1) |          ///
                    `rhs_native'[`semantic_row',15]!=              ///
                        cond("`nuisance'"=="joint",`r_dimension',`p_firms') {
                    local results_ok = 0
                }
                local projection_rhs_max_iterations = max(       ///
                    `projection_rhs_max_iterations',              ///
                    `rhs_native'[`semantic_row',5])
                local projection_rhs_max_reduced = max(          ///
                    `projection_rhs_max_reduced',                 ///
                    `rhs_native'[`semantic_row',6])
                local projection_rhs_max_complete = max(         ///
                    `projection_rhs_max_complete',                ///
                    `rhs_native'[`semantic_row',7])
                local semantic_row = `semantic_row'+1
            }
        }
        if `semantic_row' != `expected_rhs_rows'+1 local results_ok = 0
        forvalues row = 1/`expected_rhs_rows' {
            forvalues column = 1/15 {
                if missing(`rhs_native'[`row',`column']) local results_ok = 0
            }
            foreach column in 1 2 3 4 5 8 9 10 11 12 14 15 {
                if `rhs_native'[`row',`column'] !=                   ///
                    floor(`rhs_native'[`row',`column']) local results_ok = 0
            }
            // Column four belongs to the frozen V2 RHS prefix and
            // therefore remains diagonal.  The additive V7 plan fields above
            // are authoritative for the actual selected route.
            if `rhs_native'[`row',4]!=2 |                         ///
                `rhs_native'[`row',5]<0 |                         ///
                `rhs_native'[`row',5]>`maxiter' |                   ///
                `rhs_native'[`row',6]<0 | `rhs_native'[`row',7]<0 | ///
                `rhs_native'[`row',7]>`rhs_native'[`row',13] |      ///
                !inlist(`rhs_native'[`row',8],0,1) |                ///
                !inlist(`rhs_native'[`row',9],1,2) |                ///
                `rhs_native'[`row',8] != (`rhs_native'[`row',9]==1) | ///
                `rhs_native'[`row',10]<0 | `rhs_native'[`row',11]<0 | ///
                `rhs_native'[`row',12]<0 | `rhs_native'[`row',13]<=0 | ///
                !inlist(`rhs_native'[`row',14],1,2) |               ///
                `rhs_native'[`row',15]<=0 local results_ok = 0
            if `rhs_native'[`row',8] &                              ///
                (`rhs_native'[`row',5]!=0 | `rhs_native'[`row',6]!=0 | ///
                 `rhs_native'[`row',10]!=0 | `rhs_native'[`row',11]!=0 | ///
                 `rhs_native'[`row',12]!=0)                         ///
                local results_ok = 0
            local rhs_max_iterations = max(`rhs_max_iterations',`rhs_native'[`row',5])
            local rhs_max_reduced = max(`rhs_max_reduced',`rhs_native'[`row',6])
            local rhs_max_complete = max(`rhs_max_complete',`rhs_native'[`row',7])
        }
    }
    local receipt_numbers r_seed r_probes r_lev_acc r_tgt_acc r_req_route ///
        r_sel_route r_fallback r_fallback_err r_dimension r_lev_batch     ///
        r_tgt_batch r_rank_tol r_block_tol r_full_tol r_full_route        ///
        r_full_iter r_full_red r_full_complete r_full_zero r_lev_rhs      ///
        r_tgt_rhs r_max_red r_max_complete r_max_lev r_max_recip          ///
        r_accounting r_top_hi r_top_lo r_rng r_rhs_rows r_rhs_copy        ///
        r_algorithm_req r_algorithm_sel r_deletion r_nuisance r_parameters ///
        r_full_parameters r_correction_parameters r_native_info           ///
        r_native_inverse r_exact_flags r_actual_accounting r_rss          ///
        r_mem_limit r_input_copy r_prep_peak r_resident r_solver_setup    ///
        r_lev_phase r_tgt_phase r_result_bytes r_solve_peak r_command_peak ///
        r_rhs_schema r_engine_req r_engine_sel r_generic_flags            ///
        r_generic_controls r_control_rhs r_cr_rcond r_cr_small r_cr_large ///
        r_cr_proj r_cr_norm r_cr_fe r_cr_maxproj r_cr_tol r_cr_pcg        ///
        r_cr_gate r_control_basis r_control_forward r_schur_rcond          ///
        r_schur_relres r_rank_gap r_full_joint r_working_fit r_maker      ///
        r_canon_peak r_fit_peak r_geometry_peak r_generic_lev_peak        ///
        r_generic_tgt_peak r_maker_peak r_generic_result r_generic_peak   ///
        r_rhs_v2_copy r_cap_schema r_cap_profile r_batch_mode             ///
        r_stayers_mode r_target_mode r_deletion_source r_probeorder       ///
        r_wallseconds r_frequency r_physical_limit r_signature_hi r_signature_lo ///
        r_lev_batch_mode r_tgt_batch_mode r_plan_struct             ///
        r_plan_schema r_plan_route_schema r_plan_resolved r_plan_frozen ///
        r_plan_applicability r_plan_alg_req r_plan_alg_sel          ///
        r_plan_eng_req r_plan_eng_sel r_plan_route_req r_plan_route_sel ///
        r_plan_route_fallback r_plan_route_error r_plan_rhs         ///
        r_plan_full_dim r_batch_lev_sel r_batch_tgt_sel             ///
        r_batch_command r_batch_nonbatched r_batch_lev_onebytes     ///
        r_batch_lev_selbytes r_batch_tgt_onebytes                   ///
        r_batch_tgt_selbytes r_ctr_complete r_pre_rng_hi r_pre_rng_lo ///
        r_wall_requested_value r_wall_forecast_value                ///
        r_wall_advisory_value r_wall_margin_value r_plan_mem_command ///
        r_proj_columns r_proj_peak
    foreach value of local receipt_numbers {
        if missing(``value'') local results_ok = 0
    }
    // The V6 full-fit route field is frozen as diagonal.  V7
    // plan_route_sel is authoritative for the actual selected route.
    local route_result_ok =                                      ///
        `r_req_route'==`route_expected_code' &                   ///
        inlist(`r_sel_route',2,3) &                              ///
        (`route_expected_code'==0 | `r_sel_route'==`route_expected_code') & ///
        `r_full_route'==2 &                                      ///
        inlist(`r_fallback',0,1) &                               ///
        (`fallback_allowed' | `r_fallback'==0) &                 ///
        (`r_fallback' | `r_fallback_err'==0) &                   ///
        `r_plan_route_req'==`r_req_route' &                      ///
        `r_plan_route_sel'==`r_sel_route' &                      ///
        `r_plan_route_fallback'==`r_fallback' &                  ///
        `r_plan_route_error'==`r_fallback_err'
    local plan_result_ok =                                       ///
        `r_plan_struct'==1000 & `r_plan_schema'==1 &             ///
        `r_plan_route_schema'==2 & `r_plan_resolved'==1 &        ///
        `r_plan_frozen'==1 & `r_plan_applicability'==3 &         ///
        `r_plan_alg_req'==`algorithm_expected_code' &            ///
        `r_plan_alg_sel'==2 &                                    ///
        `r_plan_eng_req'==`engine_expected_code' &               ///
        `r_plan_eng_sel'==2 &                                    ///
        `r_plan_rhs'==`expected_rhs_rows'+                        ///
            `component_requested'*(`inferencesimulations'+3) &    ///
        `r_plan_full_dim'==`r_dimension' &                        ///
        `r_batch_lev_sel'==`r_lev_batch' &                        ///
        `r_batch_tgt_sel'==`r_tgt_batch' &                        ///
        `r_plan_mem_command'==`r_batch_command'+                  ///
            `component_requested'*`ci_result_peak' &              ///
        `r_batch_command'==max(`r_batch_nonbatched',              ///
            `r_batch_lev_selbytes',`r_batch_tgt_selbytes') &      ///
        `r_batch_lev_onebytes'<=`r_batch_lev_selbytes' &          ///
        `r_batch_tgt_onebytes'<=`r_batch_tgt_selbytes' &          ///
        `r_ctr_complete'==1 & `r_pre_rng_hi'==0 & `r_pre_rng_lo'==0
    local expected_phase_batch = cond(`phase_batch_code'==0,0,   ///
        min(`batch',`probes'))
    local batch_result_ok =                                      ///
        `r_lev_batch'>=1 & `r_lev_batch'<=`probes' &             ///
        `r_tgt_batch'>=1 & `r_tgt_batch'<=`probes' &             ///
        (`phase_batch_code'==0 |                                 ///
            (`r_lev_batch'==`expected_phase_batch' &             ///
             `r_tgt_batch'==`expected_phase_batch')) &           ///
        `r_lev_batch_mode'==`phase_batch_code' &                 ///
        `r_tgt_batch_mode'==`phase_batch_code'
    local capability_result_ok =                                 ///
        `r_cap_schema'==3 & `r_cap_profile'==4 &                 ///
        `r_batch_mode'==`phase_batch_code' &                     ///
        `r_stayers_mode'==`stayers_code' &                       ///
        `r_target_mode'==`target_code' &                          ///
        `r_deletion_source'==`deletion_source_code' &            ///
        `r_probeorder'==`probeorder_supplied_code' &             ///
        `r_wallseconds'==`wallseconds_supplied_code' &           ///
        `r_frequency'==`frequency_code' &                        ///
        `r_physical_limit'==`physicallimit' &                    ///
        `r_plan_schema'==1 & `r_plan_route_schema'==2 &          ///
        `r_wall_requested_value'==`wallseconds_value'
    local expected_generic_base_peak = max(`r_canon_peak',`r_fit_peak', ///
        `r_geometry_peak',`r_generic_lev_peak',`r_generic_tgt_peak',    ///
        `r_proj_peak',`r_maker_peak',`r_generic_result')
    local memory_result_ok =                                     ///
        `r_result_bytes'==`r_generic_result' &                   ///
        `r_result_bytes'>=`r_rhs_v2_copy' &                      ///
        `r_solver_setup'==max(`r_canon_peak',`r_fit_peak',`r_geometry_peak') & ///
        `r_generic_peak'==`expected_generic_base_peak'+          ///
            `component_requested'*`ci_result_peak' &             ///
        `r_solve_peak'==`r_generic_peak' &                       ///
        `r_plan_mem_command'==`expected_generic_base_peak'+      ///
            `component_requested'*`ci_result_peak' &             ///
        `r_command_peak'==max(`r_prep_peak',`r_solve_peak') &    ///
        `r_command_peak'<=`r_mem_limit'
    local fit_receipt_ok =                                         ///
        `r_seed'==`seed' & `r_probes'==`probes' &                  ///
        `r_lev_acc'==`probes' & `r_tgt_acc'==`probes' &            ///
        `route_result_ok' & `plan_result_ok' &                     ///
        `r_dimension'==`p_firms'+`control_count' &                 ///
        `batch_result_ok' &                                       ///
        `r_rank_tol'==`ranktol' & `r_block_tol'==`blocktol' &      ///
        `r_full_tol'==`expected_full_tol' &                        ///
        `r_full_iter'>=0 & `r_full_iter'<=`maxiter' &              ///
        `r_full_red'>=0 & `r_full_complete'>=0 &                   ///
        `r_full_complete'<=`r_full_tol' & inlist(`r_full_zero',0,1) & ///
        `r_lev_rhs'==`probes' & `r_tgt_rhs'==2*`probes' &          ///
        `r_proj_columns'==`projection_columns' &                   ///
        `r_proj_peak'==cond(`projection_columns'>0,`prr_peak',0)
    local component_max_complete = `component_inference_receipt'[1,14]
    local residual_receipt_ok =                                    ///
        `r_max_red'==`rhs_max_reduced' &                           ///
        `r_max_complete'==max(`rhs_max_complete',                  ///
            `component_requested'*`component_max_complete') &      ///
        `r_max_complete'<=`r_full_tol' &                           ///
        `r_max_lev'>=0 & `r_max_lev'<1 & `r_max_recip'>=0 &       ///
        `r_max_recip'<=`maker_gate'
    local schema_receipt_ok =                                      ///
        `r_rng'==1 & `r_rhs_rows'==`expected_rhs_rows' &           ///
        `r_rhs_copy'==0 & `r_rhs_schema'==2 &                      ///
        `r_algorithm_req'==`algorithm_expected_code' &             ///
        `r_algorithm_sel'==2 & `r_deletion'==`deletion_code' &     ///
        `r_nuisance'==`nuisance_code' &                            ///
        `r_parameters'==`expected_parameters' &                    ///
        `r_full_parameters'==`expected_full_parameters' &          ///
        `r_correction_parameters'==`expected_parameters' &         ///
        `r_native_info'==0 & `r_native_inverse'==0 &               ///
        `r_exact_flags'==256 & `r_engine_req'==`engine_expected_code' & ///
        `r_engine_sel'==2 & `r_generic_flags'==`expected_flags' &  ///
        `r_generic_controls'==`control_count' &                    ///
        `r_control_rhs'==`control_count'
    local generic_numeric_ok =                                     ///
        `r_full_joint'==`r_full_complete' &                        ///
        `r_working_fit'>=0 & `r_working_fit'<=`r_full_tol' &       ///
        `r_maker'==`r_max_recip' & `r_rank_gap'>0 &                ///
        `r_schur_rcond'>0 & `r_schur_rcond'<=1 &                   ///
        `r_schur_relres'>=0 & `r_control_basis'>=0 &               ///
        `r_control_forward'>=0 &                                   ///
        scalar(`native_cr_tol')==max(`ranktol',1e-12) &            ///
        scalar(`native_cr_pcg')==1e-13 &                           ///
        scalar(`native_cr_gate')==1e-11
    local submission_receipt_ok =                                  ///
        `capability_result_ok' &                                   ///
        `r_signature_hi'==`cap_request_signature_hi' &             ///
        `r_signature_lo'==`cap_request_signature_lo' &             ///
        `r_mem_limit'==`p_mem_limit' & `r_input_copy'==`p_input_copy' & ///
        `r_prep_peak'==`p_prep_peak' & `r_resident'==`solve_resident' & ///
        `r_rhs_v2_copy'==216*`expected_rhs_rows' &                 ///
        `memory_result_ok' &                                       ///
        abs(`r_actual_accounting'-`accounting_truth')<=            ///
            `roundoff_gate'*max(1,abs(`accounting_truth')) &       ///
        abs(`r_accounting'-`r_actual_accounting')<=                ///
            `roundoff_gate'*max(1,abs(`r_actual_accounting'))
    if `results_ok' {
        local results_ok = `fit_receipt_ok' & `residual_receipt_ok' & ///
            `schema_receipt_ok' & `generic_numeric_ok' &           ///
            `submission_receipt_ok'
    }
    if `results_ok' & `control_count'>0 {
        local results_ok =                                         ///
            scalar(`native_cr_rcond')>scalar(`native_cr_tol') &   ///
            scalar(`native_cr_rcond')<=1 &                        ///
            scalar(`native_cr_small')>0 & scalar(`native_cr_large')>0 & ///
            scalar(`native_cr_small')<=scalar(`native_cr_large') & ///
            scalar(`native_cr_rcond')==                           ///
                scalar(`native_cr_small')/scalar(`native_cr_large') & ///
            scalar(`native_cr_proj')>=0 & scalar(`native_cr_norm')>=0 & ///
            scalar(`native_cr_norm')<.25 & scalar(`native_cr_fe')>0 & ///
            scalar(`native_cr_maxproj')>=0 &                      ///
            scalar(`native_cr_maxproj')==scalar(`control_projection_max') & ///
            scalar(`native_cr_maxproj')<=scalar(`native_cr_gate')
    }
    if `results_ok' & `control_count'==0 {
        local results_ok =                                         ///
            scalar(`native_cr_rcond')==1 & scalar(`native_cr_small')==1 & ///
            scalar(`native_cr_large')==1 & scalar(`native_cr_proj')==0 & ///
            scalar(`native_cr_norm')==0 & scalar(`native_cr_fe')==0 & ///
            scalar(`native_cr_maxproj')==0 &                       ///
            scalar(`control_projection_max')==0
    }
    if `results_ok' & `projection_requested' {
        local expected_proj_result_bytes =                       ///
            (`projection_columns'+2*`projection_columns'^2)*8
        quietly _vckss_proj_result_ok                             ///
            `projection_b' `projection_V' `projection_V_naive'    ///
            `projection_columns' `prr_schema' `prr_columns'       ///
            `prr_effect' `prr_weight' `pr_effect' `pr_weight'     ///
            `prr_cov_min' `prr_cov_max' `prr_psd' `prr_proxy_min' ///
            `prr_proxy_max' `prr_max_iter' `prr_max_reduced'      ///
            `prr_max_complete' `projection_rhs_max_iterations'    ///
            `projection_rhs_max_reduced'                          ///
            `projection_rhs_max_complete' `prr_full_tol'          ///
            `=scalar(`native_full_tol')' `prr_peak' `r_proj_peak' ///
            `r_mem_limit' `prr_bytes' `expected_proj_result_bytes'
        local results_ok = r(ok)
    }
    if !`results_ok' {
        capture quietly fevc_rust release `handle'
        capture quietly fevc_rust clear
        quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"     ///
            "Generic-JLA V6/RHS-V2 result receipts did not reconcile with the submitted request."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "result_reconcile"
        ereturn local backend_selected ""
        ereturn local rng_selected ""
        ereturn scalar rust_core_ready_flags = `rustcoreflags'
        ereturn scalar rust_support_flags = `rustsupportflags'
        exit 498
    }

    capture quietly _fevc_rust_public_call release `handle'
    if _rc {
        local failure_rc = _rc
        capture noisily _fevc_rust_abort, rc(`failure_rc')         ///
            handle(`handle') phase(release) norelease
        exit _rc
    }
    capture quietly fevc_rust snapshot
    if _rc {
        local failure_rc = _rc
        capture noisily _fevc_rust_abort, rc(`failure_rc') phase(release_snapshot)
        exit _rc
    }
    if r(state)!=0 | r(handle)!=0 {
        capture quietly fevc_rust clear
        quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"     ///
            "Generic-JLA release succeeded without an idle native snapshot."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "release_snapshot"
        ereturn local backend_selected ""
        ereturn local rng_selected ""
        exit 498
    }

    tempname plugin correction corrected kss_return posted mcse decomposition
    matrix colnames `raw_results' = worker_variance firm_variance   ///
        worker_firm_covariance total_variance
    matrix rownames `raw_results' = plugin bias_correction corrected numerical_mcse
    matrix `plugin' = `raw_results'[1,1..4]
    matrix `correction' = `raw_results'[2,1..4]
    matrix `corrected' = `raw_results'[3,1..4]
    matrix `kss_return' = `corrected'
    matrix `posted' = `corrected'
    matrix `mcse' = `raw_results'[4,1..4]
    foreach matrix_name in plugin correction corrected kss_return mcse {
        matrix colnames ``matrix_name'' = worker_variance firm_variance ///
            worker_firm_covariance total_variance
    }

    local target_outcome_variance = .
    local regression_outcome_variance = .
    tempvar target_sq frequency_sq
    quietly summarize `depvar' [aw=`target'] if `result_touse' & `target'>0, meanonly
    if !_rc & !missing(r(mean)) {
        local target_mean = r(mean)
        quietly generate double `target_sq' = `target'*(`depvar'-`target_mean')^2 if `result_touse'
        quietly summarize `target_sq' if `result_touse', meanonly
        local target_outcome_variance = r(sum)/`result_target'
    }
    quietly summarize `depvar' [aw=`frequency'] if `result_touse', meanonly
    if !_rc & !missing(r(mean)) {
        local frequency_mean = r(mean)
        quietly generate double `frequency_sq' =                 ///
            `frequency'*(`depvar'-`frequency_mean')^2 if `result_touse'
        quietly summarize `frequency_sq' if `result_touse', meanonly
        local regression_outcome_variance = r(sum)/`result_physical'
    }
    local residual_variance = `r_rss'/`result_physical'
    local explained_variance = `regression_outcome_variance'-`residual_variance'
    local explained_share = .
    if `regression_outcome_variance'>0 local explained_share =      ///
        `explained_variance'/`regression_outcome_variance'
    matrix `decomposition' =                                     ///
        (`raw_results'[1,1],`raw_results'[2,1],`raw_results'[3,1] \ ///
         `raw_results'[1,2],`raw_results'[2,2],`raw_results'[3,2] \ ///
         2*`raw_results'[1,3],2*`raw_results'[2,3],2*`raw_results'[3,3] \ ///
         `raw_results'[1,4],`raw_results'[2,4],`raw_results'[3,4])
    matrix `decomposition' = `decomposition',J(4,4,.)
    if `target_outcome_variance'>0 {
        forvalues component = 1/4 {
            matrix `decomposition'[`component',4] =                 ///
                `decomposition'[`component',1]/`target_outcome_variance'
            matrix `decomposition'[`component',5] =                 ///
                `decomposition'[`component',3]/`target_outcome_variance'
        }
    }
    if `decomposition'[4,1]>0 {
        forvalues component = 1/4 {
            matrix `decomposition'[`component',6] =                 ///
                `decomposition'[`component',1]/`decomposition'[4,1]
        }
    }
    if `decomposition'[4,3]>0 {
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

    if `projection_requested' {
        local projection_names _cons `project'
        matrix colnames `projection_b' = `projection_names'
        matrix rownames `projection_V' = `projection_names'
        matrix colnames `projection_V' = `projection_names'
        matrix rownames `projection_V_naive' = `projection_names'
        matrix colnames `projection_V_naive' = `projection_names'
        matrix `projection_results' = J(`projection_columns',7,.)
        local projection_zcrit = invnormal(1-(100-`level')/200)
        forvalues row = 1/`projection_columns' {
            local projection_estimate = `projection_b'[1,`row']
            local projection_se = sqrt(`projection_V'[`row',`row'])
            local projection_z = `projection_estimate'/`projection_se'
            matrix `projection_results'[`row',1] = `projection_estimate'
            matrix `projection_results'[`row',2] = `projection_se'
            matrix `projection_results'[`row',3] = `projection_z'
            matrix `projection_results'[`row',4] = 2*normal(-abs(`projection_z'))
            matrix `projection_results'[`row',5] =                  ///
                `projection_estimate'-`projection_zcrit'*`projection_se'
            matrix `projection_results'[`row',6] =                  ///
                `projection_estimate'+`projection_zcrit'*`projection_se'
            matrix `projection_results'[`row',7] =                  ///
                sqrt(`projection_V_naive'[`row',`row'])
        }
        matrix rownames `projection_results' = `projection_names'
        matrix colnames `projection_results' = estimate se z p lb ub naive_se
        matrix `projection_diagnostics' = (`prr_schema',`prr_columns', ///
            `prr_effect',`prr_weight',`pr_gram_rcond',`pr_gram_relres', ///
            `pr_gram_orig',`prr_cov_min',`prr_cov_max',`prr_psd',    ///
            `prr_proxy_min',`prr_proxy_max',`prr_peak',`prr_bytes',  ///
            `pr_persistent',`pr_peak')
        matrix colnames `projection_diagnostics' = schema columns effect ///
            weight gram_rcond gram_relres gram_original_relres covariance_min ///
            covariance_max psd_cleanup proxy_min proxy_max solve_peak result_bytes ///
            prepared_persistent augmentation_peak
        matrix `projection_solver_rhs' = `rhs_native'[             ///
            `projection_first_row'..`=`projection_first_row'+`projection_columns'-1',1..15]
        matrix rownames `projection_solver_rhs' = `projection_names'
        matrix colnames `projection_solver_rhs' = phase probe side route iterations ///
            reduced_residual complete_residual zero_rhs status replacements ///
            operator_applications preconditioner_applications tolerance ///
            residual_space solver_dimension
    }

    tempname rhs_public graph_receipt memory_receipt preparation_receipt
    tempname capability_receipt generic_receipt control_rank_receipt
    tempname route_diagnostics
    matrix `rhs_public' = J(`expected_rhs_rows',6,.)
    forvalues row = 1/`expected_rhs_rows' {
        local phase = `rhs_native'[`row',1]
        local probe = `rhs_native'[`row',2]
        local side = `rhs_native'[`row',3]
        local stage = cond(`phase'==5,1,cond(`phase'==1,2,         ///
            cond(`phase'==4,3,cond(`phase'==2,4,                 ///
            cond(`phase'==6,6,5)))))
        local logical_rhs = cond(`probe'<0,1,cond(`phase'==3,      ///
            2*`probe'+`side',`probe'+1))
        local active_batch = cond(`phase'==2,`r_lev_batch',`r_tgt_batch')
        local batch_start = cond(`probe'<0,1,                       ///
            floor(`probe'/`active_batch')*`active_batch'+1)
        matrix `rhs_public'[`row',1] = `stage'
        matrix `rhs_public'[`row',2] = `batch_start'
        matrix `rhs_public'[`row',3] = `logical_rhs'
        matrix `rhs_public'[`row',4] = `rhs_native'[`row',5]
        matrix `rhs_public'[`row',5] = `rhs_native'[`row',7]
        matrix `rhs_public'[`row',6] = inlist(`rhs_native'[`row',9],1,2)
    }
    matrix colnames `rhs_public' = stage batch_start rhs iterations ///
        relative_residual converged
    matrix `graph_receipt' = (`g_input_rows',`g_keep_rows',`g_input_mass', ///
        `g_keep_mass',`g_init_comp',`g_max_comp',`g_init_rows',`g_mover_rows', ///
        `g_init_edges',`g_keep_edges',`g_degree_removed',`g_art_removed', ///
        `g_bridge_units',`g_bridge_rows',`g_degree_iters',`g_art_iters', ///
        `g_bridge_iters',`g_fixed_iters')
    matrix colnames `graph_receipt' = input_rows retained_rows input_mass ///
        retained_mass initial_components maximum_components initial_component_rows ///
        mover_input_rows initial_deletion_edges retained_deletion_edges   ///
        insufficient_workers_removed articulation_workers_removed        ///
        bridge_units_removed bridge_rows_removed degree_iterations       ///
        articulation_iterations bridge_iterations fixed_point_iterations
    matrix `memory_receipt' = (`r_mem_limit',`r_input_copy',0,`r_rhs_v2_copy', ///
        `r_prep_peak',`r_resident',`r_solver_setup',`r_lev_phase',`r_tgt_phase', ///
        `r_result_bytes',`r_solve_peak',`r_command_peak')
    matrix colnames `memory_receipt' = limit caller_input_copy legacy_result_copy ///
        rhs_v2_copy preparation_peak prepared_resident solver_setup leverage_phase ///
        target_phase result solve_peak command_peak
    matrix `preparation_receipt' = (`p_input',`p_retained',`p_workers', ///
        `p_firms',`p_cells',`p_units',`p_strata',`p_target',`p_controls')
    matrix colnames `preparation_receipt' = input_rows retained_rows workers ///
        firms cells deletion_units target_strata target_weight_sum controls
    matrix `capability_receipt' = (`cap_struct_size',`cap_abi_version', ///
        `cap_request_schema',`cap_supported',`cap_reason_code',`cap_profile_code', ///
        `cap_algorithm_code',`cap_deletion_mode_code',`cap_nuisance_mode_code', ///
        `cap_solver_route_code',`cap_rng_contract_code',`cap_controls_count', ///
        `cap_frequency_use_code',`cap_engine_code',`cap_batch_mode_code', ///
        `cap_stayers_mode_code',`cap_target_weight_mode_code',       ///
        `cap_deletion_source_code',`cap_probeorder_supplied',        ///
        `cap_wallseconds_supplied',`cap_physical_limit',             ///
        `cap_request_signature_hi',`cap_request_signature_lo')
    matrix colnames `capability_receipt' = struct_size abi_version schema ///
        supported reason profile algorithm deletion nuisance route rng controls ///
        frequency engine batch stayers target deletion_source probeorder wall ///
        physical_limit signature_hi signature_lo
    matrix `generic_receipt' = (`r_generic_flags',`r_control_basis', ///
        `r_control_forward',`r_schur_rcond',`r_schur_relres',`r_rank_gap', ///
        `r_full_joint',`r_working_fit',`r_maker',`r_actual_accounting')
    matrix colnames `generic_receipt' = flags control_basis_relres   ///
        control_basis_forward_error control_schur_rcond control_schur_relres ///
        deletion_rank_gap full_joint_fit working_fit maker_relres accounting
    matrix `control_rank_receipt' = (scalar(`native_cr_rcond'),      ///
        scalar(`native_cr_small'),scalar(`native_cr_large'),       ///
        scalar(`native_cr_proj'),scalar(`native_cr_norm'),         ///
        scalar(`native_cr_fe'),scalar(`native_cr_maxproj'),        ///
        scalar(`native_cr_tol'),scalar(`native_cr_pcg'),           ///
        scalar(`native_cr_gate'))
    matrix colnames `control_rank_receipt' = rcond smallest_lower largest_upper ///
        projection_error normalization_error fe_information_lower max_projection ///
        effective_tolerance projection_pcg_tolerance projection_gate

    local native_batch = max(`r_lev_batch',`r_tgt_batch')
    local native_batch_scratch = max(`r_batch_lev_selbytes',       ///
        `r_batch_tgt_selbytes')
    local native_batch_column = `native_batch_scratch'/`native_batch'
    matrix `route_diagnostics' = J(1,26,.)
    matrix `route_diagnostics'[1,1] = `r_plan_rhs'
    matrix `route_diagnostics'[1,2] = `r_plan_mem_command'
    matrix `route_diagnostics'[1,13] = `r_sel_route'
    matrix `route_diagnostics'[1,25] = `r_plan_mem_command'
    matrix colnames `route_diagnostics' = planned_rhs memory_bytes ///
        setup_seconds hierarchy_levels edge_complexity             ///
        vertex_complexity structural_bytes dense_factor_bytes      ///
        workers firms hybrid_vertices hybrid_edges route_code      ///
        predicted_vertices predicted_edges predicted_structural_bytes ///
        predicted_scratch_bytes reserved18 reserved19 reserved20   ///
        reserved21 hierarchy_seconds reserved23 reserved24         ///
        forecast_peak_bytes terminal_vertices

    tempname hybrid_source hybrid_accounting
    if `stayers_code'==2 {
        matrix `hybrid_source' = J(2,4,.)
        matrix rownames `hybrid_source' = mover_match stayer_observation
        matrix colnames `hybrid_source' = worker_variance firm_variance ///
            worker_firm_covariance total_variance
        matrix `hybrid_accounting' =                           ///
            (`retained_count',`retained_physical',`p_workers',  ///
                `retained_target',`p_units' \                   ///
             `N_hybrid_stayer_rows',`hybrid_stayer_physical',  ///
                `N_hybrid_stayers',`hybrid_stayer_target_mass', ///
                `hybrid_stayer_physical' \                     ///
             `result_stored',`result_physical',`result_workers', ///
                `result_target',`result_units')
        matrix rownames `hybrid_accounting' = mover stayer total
        matrix colnames `hybrid_accounting' = stored_rows       ///
            physical_observations worker_levels target_weight_mass ///
            deletion_units
    }

    tempname prep_boundary_counts
    local prep_deletion_groups = cond("`deletionmode'"=="observation",0,1)
    matrix `prep_boundary_counts' = (2,`prep_deletion_groups',0,1,2, ///
        4,`p_input',2,`retained_count',0,0)
    matrix colnames `prep_boundary_counts' = initial_id_group_calls ///
        deletion_group_calls retained_id_group_calls semantic_group_calls ///
        stata_sort_calls graph_import_columns graph_import_rows     ///
        retained_map_columns retained_map_rows compression_import_columns ///
        compression_import_rows

    ereturn clear
    if `component_requested' {
        ereturn post `posted' `component_V', obs(`result_physical')      ///
            esample(`result_touse') depname(`depvar')
    }
    else ereturn post `posted', obs(`result_physical')                  ///
        esample(`result_touse') depname(`depvar')
    ereturn matrix results = `raw_results'
    ereturn matrix plugin = `plugin'
    ereturn matrix correction = `correction'
    ereturn matrix kss = `kss_return'
    ereturn matrix numerical_mcse = `mcse'
    ereturn matrix decomposition = `decomposition'
    if `projection_requested' {
        ereturn matrix projection_b = `projection_b'
        ereturn matrix projection_V = `projection_V'
        ereturn matrix projection_V_naive = `projection_V_naive'
        ereturn matrix projection_results = `projection_results'
        ereturn matrix projection_diagnostics = `projection_diagnostics'
        ereturn matrix projection_solver_diagnostics = `projection_solver_rhs'
        ereturn matrix projection_augmentation_receipt = `projection_aug_ctx'
        ereturn scalar level = `level'
        ereturn scalar projection_columns = `projection_columns'
        ereturn scalar projection_psd_cleanup = `prr_psd'
        ereturn scalar projection_covariance_min = `prr_cov_min'
        ereturn scalar projection_covariance_max = `prr_cov_max'
        ereturn scalar projection_proxy_min = `prr_proxy_min'
        ereturn scalar projection_proxy_max = `prr_proxy_max'
        ereturn scalar projection_solver_iterations = `prr_max_iter'
        ereturn scalar projection_solver_max_reduced = `prr_max_reduced'
        ereturn scalar projection_solver_max_complete = `prr_max_complete'
        ereturn scalar projection_peak_forecast_bytes = `prr_peak'
        ereturn scalar projection_result_bytes = `prr_bytes'
        ereturn local projection_effect "`projecteffect'"
        ereturn local projection_weight "`projectweight'"
        ereturn local projection_variables "`project'"
        ereturn local projection_constant                       ///
            "automatic; normalization-dependent"
    }
    ereturn matrix solver_rhs_diagnostics = `rhs_public'
    ereturn matrix rust_rhs_receipts = `rhs_native'
    ereturn matrix rust_graph_receipt = `graph_receipt'
    ereturn matrix rust_memory_receipt = `memory_receipt'
    ereturn matrix rust_preparation_receipt = `preparation_receipt'
    ereturn matrix rust_request_capability_receipt = `capability_receipt'
    ereturn matrix rust_generic_receipt = `generic_receipt'
    ereturn matrix rust_control_rank_receipt = `control_rank_receipt'
    ereturn matrix route_diagnostics = `route_diagnostics'
    ereturn matrix prep_boundary_counts = `prep_boundary_counts'
    if `stayers_code'==2 {
        tempname hybrid_plugin_alias hybrid_correction_alias
        tempname hybrid_kss_alias hybrid_results_alias
        tempname hybrid_decomposition_alias
        matrix `hybrid_plugin_alias' = e(plugin)
        matrix `hybrid_correction_alias' = e(correction)
        matrix `hybrid_kss_alias' = e(kss)
        matrix `hybrid_results_alias' = e(results)
        matrix `hybrid_decomposition_alias' = e(decomposition)
        ereturn matrix stayer_hybrid_plugin = `hybrid_plugin_alias'
        ereturn matrix stayer_hybrid_correction = `hybrid_correction_alias'
        ereturn matrix stayer_hybrid_kss = `hybrid_kss_alias'
        ereturn matrix stayer_hybrid_results = `hybrid_results_alias'
        ereturn matrix stayer_hybrid_decomposition =              ///
            `hybrid_decomposition_alias'
        ereturn matrix stayer_hybrid_correction_source = `hybrid_source'
        ereturn matrix stayer_hybrid_sample_accounting = `hybrid_accounting'
    }
    ereturn matrix rust_phase_profile = `rust_phase_profile'
    ereturn local rust_phase_profile_schema "VCKSS-NATIVE-PHASE-PERF-V1"
    ereturn local rust_phase_profile_units "seconds"
    ereturn scalar rust_phase_profile_flags = `native_perf_flags'
    ereturn local prep_boundary_counts_schema "PREP-BND-COUNTS-V1"
    ereturn scalar N_stored = `result_stored'
    ereturn scalar N_physical = `result_physical'
    ereturn scalar N_requested = `nscope'
    ereturn scalar N_complete = `ncomplete'
    ereturn scalar N_retained = `result_stored'
    ereturn scalar N_mover_input = `g_mover_rows'
    ereturn scalar N_initial_component = `g_init_rows'
    ereturn scalar N_initial_component_dropped = `ncomplete'-`g_init_rows'
    ereturn scalar N_mover_dropped = `g_init_rows'-`g_mover_rows'
    ereturn scalar N_graph_dropped = `g_mover_rows'-`retained_count'
    ereturn scalar N_stayers = `nstayers'
    ereturn scalar N_stayer_rows = `nstayerrows'
    ereturn scalar worker_levels = `result_workers'
    ereturn scalar firm_levels = `p_firms'
    ereturn scalar parameters = `r_parameters'
    ereturn scalar full_parameters = `r_full_parameters'
    ereturn scalar correction_parameters = `r_correction_parameters'
    ereturn scalar controls_count = `control_count'
    ereturn scalar coefficient_cells = `result_cells'
    ereturn scalar deletion_units = `result_units'
    ereturn scalar target_strata = `result_strata'
    ereturn scalar target_weight_sum = `result_target'
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
    ereturn scalar probes = `r_probes'
    ereturn scalar max_leverage = `r_max_lev'
    ereturn scalar information_rcond = .
    ereturn scalar inverse_relres = .
    ereturn scalar solver_iterations = `rhs_max_iterations'
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
    if `stayers_code'==2 {
        ereturn scalar stayer_hybrid_N_stored = `result_stored'
        ereturn scalar stayer_hybrid_N_physical = `result_physical'
        ereturn scalar stayer_hybrid_worker_levels = `result_workers'
        ereturn scalar stayer_hybrid_firm_levels = `p_firms'
        ereturn scalar stayer_hybrid_parameters = `r_parameters'
        ereturn scalar stayer_hybrid_full_parameters = `r_full_parameters'
        ereturn scalar stayer_hybrid_corr_parameters =             ///
            `r_correction_parameters'
        ereturn scalar stayer_hybrid_deletion_units = `result_units'
        ereturn scalar stayer_hybrid_target_mass = `result_target'
        ereturn scalar stayer_hybrid_max_leverage = `r_max_lev'
        ereturn scalar stayer_hybrid_information_rcond = .
        ereturn scalar stayer_hybrid_inverse_relres = .
        ereturn scalar stayer_hybrid_weighted_rss = `r_rss'
        ereturn scalar stayer_hybrid_target_y_variance =           ///
            `target_outcome_variance'
        ereturn scalar stayer_hybrid_N_stayers = `N_hybrid_stayers'
        ereturn scalar stayer_hybrid_N_stayer_rows =               ///
            `N_hybrid_stayer_rows'
        ereturn scalar stayer_hybrid_N_stayer_physical =           ///
            `hybrid_stayer_physical'
        ereturn scalar stayer_hybrid_N_singleton_drop =            ///
            `N_hyb_singleton_drop'
        ereturn scalar stayer_hybrid_N_unattached = `N_hyb_unattached'
        ereturn scalar stayer_hybrid_mover_target_mass = `retained_target'
        ereturn scalar stayer_hybrid_stayer_target_mass =          ///
            `hybrid_stayer_target_mass'
    }
    ereturn scalar tolerance = `tolerance'
    ereturn scalar maxiter = `maxiter'
    ereturn scalar seed = `r_seed'
    ereturn scalar batch = `native_batch'
    ereturn scalar leverage_batch = `r_lev_batch'
    ereturn scalar target_batch = `r_tgt_batch'
    ereturn scalar batch_memory_budget_bytes = `r_mem_limit'
    ereturn scalar batch_column_forecast_bytes = `native_batch_column'
    ereturn scalar batch_physical_column_bytes = 0
    ereturn scalar batch_scratch_forecast_bytes = `native_batch_scratch'
    ereturn scalar memory_gib = `memorygib'
    ereturn scalar memory_forecast_bytes = `r_command_peak'
    ereturn scalar residual_acceptance_tolerance = scalar(`native_full_tol')
    ereturn scalar physical_limit = `physicallimit'
    ereturn scalar physical_limit_applied = 1
    ereturn scalar active_processors = c(processors)
    ereturn scalar route_code = `r_sel_route'
    ereturn scalar route_planned_rhs = `r_rhs_rows'
    ereturn scalar route_forecast_peak_bytes = `r_plan_mem_command'
    ereturn scalar route_hierarchy_levels = cond(`r_sel_route'==3,.,0)
    ereturn scalar route_hybrid_vertices = cond(`r_sel_route'==3,.,0)
    ereturn scalar route_hybrid_edges = cond(`r_sel_route'==3,.,0)
    ereturn scalar route_terminal_vertices = cond(`r_sel_route'==3,.,0)
    ereturn scalar rust_requested_algorithm_code = `r_algorithm_req'
    ereturn scalar rust_selected_algorithm_code = `r_algorithm_sel'
    ereturn scalar rust_requested_route = `r_req_route'
    ereturn scalar rust_selected_route = `r_sel_route'
    ereturn scalar rust_solver_fallback = `r_fallback'
    ereturn scalar rust_solver_fallback_error = `r_fallback_err'
    ereturn scalar rust_solver_dimension = `r_dimension'
    ereturn scalar rust_full_fit_route = `r_full_route'
    ereturn scalar rust_full_fit_iterations = `r_full_iter'
    ereturn scalar rust_full_fit_reduced_residual = scalar(`native_full_red')
    ereturn scalar rust_full_fit_complete_residual = scalar(`native_full_complete')
    ereturn scalar rust_full_fit_zero_rhs = `r_full_zero'
    ereturn scalar rust_working_fit_residual = scalar(`native_working')
    ereturn scalar rust_leverage_rhs_count = `r_lev_rhs'
    ereturn scalar rust_target_rhs_count = `r_tgt_rhs'
    ereturn scalar rust_control_rhs_count = `r_control_rhs'
    ereturn scalar rust_rhs_receipt_schema = `r_rhs_schema'
    ereturn scalar rust_max_reduced_residual = `r_max_red'
    ereturn scalar rust_max_complete_residual = `r_max_complete'
    ereturn scalar rust_leverage_probes_accepted = `r_lev_acc'
    ereturn scalar rust_target_probes_accepted = `r_tgt_acc'
    ereturn scalar rust_rank_tolerance = `r_rank_tol'
    ereturn scalar rust_block_tolerance = `r_block_tol'
    ereturn scalar rust_maker_relres = `r_maker'
    ereturn scalar rust_control_basis_relres = `r_control_basis'
    ereturn scalar rust_control_basis_forward_error = `r_control_forward'
    ereturn scalar rust_control_schur_rcond = `r_schur_rcond'
    ereturn scalar rust_control_schur_relres = `r_schur_relres'
    ereturn scalar rust_deletion_rank_gap = `r_rank_gap'
    ereturn scalar rust_actual_accounting_residual = `r_actual_accounting'
    ereturn scalar rust_generic_diagnostic_flags = `r_generic_flags'
    ereturn scalar rust_canonical_peak_bytes = `r_canon_peak'
    ereturn scalar rust_generic_fit_peak_bytes = `r_fit_peak'
    ereturn scalar rust_geometry_peak_bytes = `r_geometry_peak'
    ereturn scalar rust_generic_leverage_peak = `r_generic_lev_peak'
    ereturn scalar rust_generic_target_peak = `r_generic_tgt_peak'
    ereturn scalar rust_maker_peak_bytes = `r_maker_peak'
    ereturn scalar rust_generic_result_bytes = `r_generic_result'
    ereturn scalar rust_generic_peak_bytes = `r_generic_peak'
    ereturn scalar rust_rhs_v2_copy_bytes = `r_rhs_v2_copy'
    ereturn scalar rust_core_ready_flags = `rustcoreflags'
    ereturn scalar rust_support_flags = `rustsupportflags'
    ereturn scalar rust_topology_checksum_hi = `r_top_hi'
    ereturn scalar rust_topology_checksum_lo = `r_top_lo'
    ereturn scalar rust_rng_contract_code = `r_rng'
    ereturn scalar rust_requested_engine_code = `r_engine_req'
    ereturn scalar rust_selected_engine_code = `r_engine_sel'
    ereturn scalar rust_batch_mode_code = `r_batch_mode'
    ereturn scalar rust_leverage_batch_mode_code = `r_lev_batch_mode'
    ereturn scalar rust_target_batch_mode_code = `r_tgt_batch_mode'
    ereturn scalar rust_plan_struct_size = `r_plan_struct'
    ereturn scalar rust_plan_schema = `r_plan_schema'
    ereturn scalar rust_plan_route_schema = `r_plan_route_schema'
    ereturn scalar rust_plan_resolved = `r_plan_resolved'
    ereturn scalar rust_plan_frozen = `r_plan_frozen'
    ereturn scalar rust_plan_applicability = `r_plan_applicability'
    ereturn scalar rust_plan_algorithm_requested = `r_plan_alg_req'
    ereturn scalar rust_plan_algorithm_selected = `r_plan_alg_sel'
    ereturn scalar rust_plan_engine_requested = `r_plan_eng_req'
    ereturn scalar rust_plan_engine_selected = `r_plan_eng_sel'
    ereturn scalar rust_plan_route_requested = `r_plan_route_req'
    ereturn scalar rust_plan_route_selected = `r_plan_route_sel'
    ereturn scalar rust_plan_route_fallback = `r_plan_route_fallback'
    ereturn scalar rust_plan_route_error = `r_plan_route_error'
    ereturn scalar rust_plan_rhs = `r_plan_rhs'
    ereturn scalar rust_plan_full_dimension = `r_plan_full_dim'
    ereturn scalar rust_plan_leverage_batch = `r_batch_lev_sel'
    ereturn scalar rust_plan_target_batch = `r_batch_tgt_sel'
    ereturn scalar rust_plan_nonbatched_peak_bytes = `r_batch_nonbatched'
    ereturn scalar rust_plan_lev_one_bytes = `r_batch_lev_onebytes'
    ereturn scalar rust_plan_lev_selected_bytes = `r_batch_lev_selbytes'
    ereturn scalar rust_plan_tgt_one_bytes = `r_batch_tgt_onebytes'
    ereturn scalar rust_plan_tgt_selected_bytes = `r_batch_tgt_selbytes'
    ereturn scalar rust_counter_plan_complete = `r_ctr_complete'
    ereturn scalar rust_pre_rng_hi = `r_pre_rng_hi'
    ereturn scalar rust_pre_rng_lo = `r_pre_rng_lo'
    ereturn scalar rust_wallseconds_requested = `r_wall_requested_value'
    ereturn scalar rust_wallseconds_forecast = `r_wall_forecast_value'
    ereturn scalar rust_wallseconds_advisory = `r_wall_advisory_value'
    ereturn scalar rust_wallseconds_margin = `r_wall_margin_value'
    ereturn scalar rust_plan_solve_peak_bytes = `r_plan_mem_command'
    ereturn scalar rust_stayers_mode_code = `r_stayers_mode'
    ereturn scalar rust_target_weight_mode_code = `r_target_mode'
    ereturn scalar rust_deletion_source_code = `r_deletion_source'
    ereturn scalar rust_probeorder_supplied = `r_probeorder'
    ereturn scalar rust_wallseconds_supplied = `r_wallseconds'
    ereturn scalar rust_frequency_use_code = `r_frequency'
    ereturn scalar rust_solve_physical_limit = `r_physical_limit'
    ereturn scalar rust_result_cap_schema = `r_cap_schema'
    ereturn scalar rust_result_cap_profile = `r_cap_profile'
    ereturn scalar rust_solve_signature_hi = `r_signature_hi'
    ereturn scalar rust_solve_signature_lo = `r_signature_lo'
    ereturn scalar rng_master_seed = `r_seed'
    ereturn scalar rng_leverage_probe_first = 1
    ereturn scalar rng_leverage_probe_last = `r_probes'
    ereturn scalar rng_target_probe_first = 1
    ereturn scalar rng_target_probe_last = `r_probes'
    ereturn scalar rust_cap_struct_size = `cap_struct_size'
    ereturn scalar rust_cap_abi_version = `cap_abi_version'
    ereturn scalar rust_cap_schema = `cap_request_schema'
    ereturn scalar rust_cap_supported = `cap_supported'
    ereturn scalar rust_cap_reason_code = `cap_reason_code'
    ereturn scalar rust_cap_profile_code = `cap_profile_code'
    ereturn scalar rust_cap_signature_hi = `cap_request_signature_hi'
    ereturn scalar rust_cap_signature_lo = `cap_request_signature_lo'
    ereturn scalar rust_cap_algorithm_deferred = `cap_alg_defer'
    ereturn scalar rust_cap_engine_deferred = `cap_eng_defer'
    ereturn scalar numerical_mcse_available = 1
    ereturn scalar backend_option_supplied = `backendsupplied'
    ereturn scalar rng_option_supplied = `rngsupplied'
    ereturn scalar algorithm_option_supplied = `algorithmsupplied'
    ereturn scalar engine_option_supplied = `enginesupplied'
    ereturn scalar preconditioner_option_supplied = `preconditionsupplied'
    ereturn scalar batch_option_supplied = `batchsupplied'
    ereturn scalar stayers_option_supplied = `stayerssupplied'
    ereturn scalar deletionid_option_supplied = `deletionidsupplied'
    ereturn scalar targetweight_option_supplied = `targetweightsupplied'
    ereturn local cmd "fevc"
    ereturn local cmdline `"`cmdline'"'
    ereturn local version "0.5.0-alpha.1"
    ereturn local model "linear"
    ereturn local correction_method "kss"
    ereturn local backend_requested "rust"
    ereturn local backend_selected "rust"
    ereturn local backend_routing_reason "explicit planned public generic-JLA route"
    ereturn local rng_requested "counter_v1"
    ereturn local rng_selected "counter_v1"
    ereturn local rng_contract "VCKSS-COUNTER-V1"
    ereturn local rng_implementation "stateless canonical Counter-V1 atoms"
    ereturn local rng_call_shape "one canonical atom plan per logical probe"
    ereturn local rng_runtime "native Rust Counter-V1"
    ereturn local rng_leverage_domain "leverage"
    ereturn local rng_target_domain "target"
    ereturn local algorithm "jla"
    ereturn local engine_requested "`engine_requested'"
    ereturn local engine_selected "generic"
    ereturn local preconditioner_requested "`preconditioner_requested'"
    ereturn local preconditioner_selected = cond(`r_sel_route'==3,"CMG","DIAGONAL")
    ereturn local routing_reason "native planned generic-JLA route"
    ereturn local fallback_status = cond(`r_fallback',"CMG_TO_DIAGONAL", ///
        "NOT_NEEDED")
    ereturn local fallback_message = cond(`r_fallback',              ///
        "CMG setup failed before RNG and the permitted diagonal fallback completed", ///
        "generic-JLA completed on the selected native route")
    ereturn local batch_requested "`batch_request'"
    ereturn local batch_routing_reason = cond("`phase_batch_mode'"=="auto", ///
        "native planner selected independent phase widths",          ///
        "caller supplied the shared explicit phase width")
    ereturn local fastpath_status = cond(`stayers_code'==2,       ///
        "FASTPATH_MIXED_DELETION",cond("`engine_requested'"=="generic", ///
        "FASTPATH_BYPASSED",cond("`deletionmode'"=="observation", ///
        "FASTPATH_OBSERVATION_DELETION",cond(`control_count'>0,   ///
        "FASTPATH_CONTROLS","FASTPATH_BYPASSED"))))
    ereturn local deletion "`deletionmode'"
    ereturn local nuisance "`nuisance'"
    ereturn local stayers "`stayers_mode'"
    ereturn local target_population = cond(`stayers_code'==2,     ///
        "retained movers plus eligible attached stayers",       ///
        cond("`deletionmode'"=="match","movers",             ///
            "retained observations"))
    ereturn local sample_selection = cond(`stayers_code'==2,      ///
        "MOVERS_FIXED_POINT_PLUS_ELIGIBLE_ATTACHED_STAYERS",    ///
        cond("`deletionmode'"=="match",                        ///
            "MOVERS_DELETION_MULTIGRAPH_FIXED_POINT",           ///
            "MATLAB_LEAVEONEWORKER_COMPONENT"))
    ereturn local connectedness_status = cond("`deletionmode'"=="match", ///
        "DELETION_UNIT_BRIDGE_FREE","LEAVE_ONE_WORKER_CONNECTED")
    ereturn local frequency_convention "literal physical copies"
    ereturn local targetweight_convention                           ///
        "explicit stored-row mass; default physical-observation mass"
    ereturn local probe_order = cond(`probeorder_supplied_code',    ///
        "observed IDs, outcome, controls, target mass, and optional tie-breaker", ///
        "canonical observed inputs and Counter-V1 domains")
    ereturn local residual_normalization "complete weighted model residual"
    ereturn local quotient_convention "full_firm_zero_sum"
    ereturn local grounding_convention                              ///
        "last_firm_zero_after_quotient_with_complete residual checked"
    ereturn local inference = cond(`projection_requested',"none","not implemented")
    ereturn local inference_method = cond(`projection_requested',       ///
        "sparse JLA block cross-fit projection","not requested")
    ereturn local inference_deletion = cond(`projection_requested',     ///
        "qualified JLA `deletionmode' deletion; stayers `stayers_mode'", ///
        "not requested")
    ereturn local inference_covariance = cond(`projection_requested',   ///
        "symmetrized block covariance","not posted")
    ereturn local inference_rng = cond(`projection_requested',          ///
        "Counter-V1 JLA proxy with deterministic projection solves",  ///
        "not requested")
    ereturn local numerical_error "conditional probe MCSE and certified solver residuals"
    ereturn local inverse_diagnostics "NOT_APPLICABLE"
    ereturn local deletion_rank_certificate "generic maker/control-Schur rank gates"
    if `stayers_code'==2 {
        ereturn local stayer_hybrid_status "CONVERGED"
        ereturn local stayer_hybrid_target_population             ///
            "retained movers plus eligible original one-firm stayers attached to retained mover firms"
        ereturn local stayer_hybrid_deletion                       ///
            "mover matches plus stayer physical observations"
        ereturn local stayer_hybrid_assumption                     ///
            "mover correction is match-robust; stayer correction is not match-robust"
        ereturn local stayer_hybrid_sample_rule                    ///
            "original one-firm stayers; retained mover firm; physical T>=2; graph-dropped movers excluded"
        ereturn local stayer_hybrid_esample                        ///
            "e(sample) marks retained movers plus eligible attached stayers"
        ereturn local stayer_hybrid_targetweight                   ///
            "pooled stored-row target mass; explicit mass is not multiplied by frequency"
        ereturn local stayer_hybrid_nuisance "`nuisance'"
    }
    ereturn local route_api "VCKSS-NATIVE-GENERIC-PLANNED-V4-V7"
    ereturn local rust_capability_profile "PLANNED_V1"
    ereturn local execution_plan_schema "VCKSS-EXECUTION-PLAN-V1"
    ereturn local rust_capability_reason "SUPPORTED"
    ereturn local status = cond(`projection_requested',               ///
        "KSS_PROJECTION_INFERENCE","KSS_POINT_ESTIMATES_ONLY")
    if `component_requested' {
        quietly _fevc_rust_component_post `component_model' `inference' ///
            `inferencesimulations' `inferenceseed' `level'               ///
            `component_V_primitive' `component_inference_results'        ///
            `component_trace_mcse' `component_spectrum'                  ///
            `component_variance_summary' `component_fold_diagnostics'    ///
            `component_cv_diagnostics' `component_inference_receipt'     ///
            `component_aug_ctx' `component_q1_results' `component_q1_raw'
    }
    if "`nodisplay'" == "" _fevc_display
end

program define _vckss_descriptive_variances, rclass
    version 18.0
    args depvar frequency target touse physical
    foreach variable in `depvar' `frequency' `target' `touse' {
        confirm numeric variable `variable'
    }
    return scalar target = .
    return scalar regression = .
    tempvar target_sq frequency_sq
    quietly summarize `depvar' [aw=`target'] if `touse' & `target'>0, ///
        meanonly
    if !_rc & !missing(r(mean)) {
        local target_mean = r(mean)
        quietly generate double `target_sq' =                        ///
            `target'*(`depvar'-`target_mean')^2 if `touse'
        quietly count if `touse' & `target'>0 & missing(`target_sq')
        if r(N)==0 {
            quietly summarize `target_sq' if `touse', meanonly
            local target_ss = r(sum)
            quietly summarize `target' if `touse', meanonly
            if r(sum)>0 return scalar target = `target_ss'/r(sum)
        }
    }
    quietly summarize `depvar' [aw=`frequency'] if `touse', meanonly
    if !_rc & !missing(r(mean)) {
        local frequency_mean = r(mean)
        quietly generate double `frequency_sq' =                    ///
            `frequency'*(`depvar'-`frequency_mean')^2 if `touse'
        quietly count if `touse' & missing(`frequency_sq')
        if r(N)==0 & `physical'>0 {
            quietly summarize `frequency_sq' if `touse', meanonly
            return scalar regression = r(sum)/`physical'
        }
    }
end

program define _vckss_mata_stayer_hybrid, rclass sortpreserve
    version 18.0
    args depvar worker firm controls frequency target deletion stayer ///
        touse nuisance ranktol blocktol exactlimit blocklimit probeorder
    tempname raw diagnostics source
    tempvar semantic_target
    quietly generate double `semantic_target' = `target'/`frequency' ///
        if `touse'
    local key `worker' `firm' `deletion' `semantic_target' `depvar' `controls'
    if "`probeorder'"!="" local key `key' `probeorder'
    sort `key' `worker' `firm' `deletion'
    capture noisily mata: vckss__stata_exact_stayer_hybrid(          ///
        "`depvar'", "`worker'", "`firm'", `"`controls'"',       ///
        "`frequency'", "`target'", "`deletion'", "`stayer'",   ///
        "`touse'", "`nuisance'", `ranktol', `blocktol',           ///
        `exactlimit', `blocklimit', "`raw'", "hybrid_status",     ///
        "hybrid_message", "`diagnostics'", "`source'")
    if _rc {
        local rc = _rc
        quietly _vckss_post_failure "MATA_RUNTIME_FAILED"           ///
            "The combined mover-stayer Mata calculation stopped unexpectedly."
        exit `rc'
    }
    if "`hybrid_status'"!="CONVERGED" {
        local rc = cond(inlist("`hybrid_status'", "EXACT_SIZE_LIMIT", ///
            "INVALID_INPUT", "INVALID_FREQUENCY",                  ///
            "INVALID_TARGET_WEIGHT", "INVALID_IDENTIFIER",         ///
            "INVALID_NUISANCE", "INVALID_TOLERANCE",               ///
            "BLOCK_SIZE_LIMIT", "CROSS_COORDINATE_MATCH"),198,498)
        quietly _vckss_post_failure "`hybrid_status'"                ///
            `"Stayer hybrid failed: `hybrid_message'."'
        di as error `"stayer hybrid failed: `hybrid_message'"'
        exit `rc'
    }
    return matrix raw = `raw'
    return matrix diagnostics = `diagnostics'
    return matrix source = `source'
end

program define _vckss_impl, eclass sortpreserve
    version 18.0
    local cmdline `"fevc `0'"'

    if lower(strtrim(`"`0'"')) == ", version" {
        ereturn clear
        ereturn local cmd "fevc"
        ereturn local version "0.5.0-alpha.1"
        ereturn local model "linear"
        ereturn local correction "kss"
        ereturn local status "ALPHA"
        di as txt "fevc 0.5.0-alpha.1 (30aug2026)"
        exit
    }

    capture noisily syntax varlist(numeric fv min=1) [if] [in] [fw], ///
        WORKER(varname) FIRM(varname) [                          ///
        DELETION(string) DELETIONID(varname)                     ///
        ALGORITHM(string) NUISANCE(string)                       ///
        TARGETWeight(varname numeric) STAYERS(string)            ///
        PROBEOrder(varname numeric)                              ///
        PROBES(integer 200) BATCH(string)                        ///
        ENGINE(string) BACKEND(string) RNG(string) WALLSeconds(string) ///
        PREConditioner(string) MEMory_gib(real 4)                ///
        SEED(integer 8675309) TOLerance(string)                  ///
        MAXIter(integer 10000) EXACT_limit(integer 500)          ///
        RANK_tolerance(real 1e-10) BLOCK_tolerance(real 1e-10)   ///
        BLOCKSIZE_limit(integer 5000)                            ///
        PHYSICAL_limit(integer 50000000)                         ///
        INFERence(string) Level(cilevel)                          ///
        INFERENCEMOdel(string)                                    ///
        INFERENCESIMulations(integer 1000)                        ///
        INFERENCESeed(integer 8675309)                            ///
        INFERENCEBins(integer 1000)                               ///
        PROJECT(varlist numeric) PROJECTEffect(string)            ///
        PROJECTWeight(string) NODISPlay                           ///
    ]
    local syntax_rc = _rc
    if `syntax_rc' {
        quietly _vckss_post_failure "INVALID_INPUT"              ///
            "Stata rejected the command syntax, variable list, weight, qualifier, or option."
        di as error "check the required worker() and firm() options and the documented syntax"
        exit `syntax_rc'
    }
    local tolerance_supplied = (strtrim(`"`tolerance'"') != "")
    if !`tolerance_supplied' local tolerance = 1e-10
    else local tolerance = real(strtrim(`"`tolerance'"'))

    /* Preflight is split out to keep Stata's compiled-program limit stable. */
    _fevc_component_model_route `"`inference'"' `"`inferencemodel'"' ///
        `"`project'"' `"`projecteffect'"' `"`projectweight'"'      ///
        `inferencesimulations' `inferencebins' `inferenceseed'       ///
        `"`backend'"' `"`rng'"' `"`algorithm'"' `"`engine'"'     ///
        `"`preconditioner'"' `"`batch'"' `"`stayers'"'            ///
        `"`deletionid'"' `"`targetweight'"' `"`deletion'"'       ///
        `"`nuisance'"' `"`weight'"'
    local backend_fallback = 0
    local backend_fallback_reason ""
    local backend_fallback_phase ""

    // The scalable project() surface is deliberately explicit and narrow.
    // Its qualified planned-generic solvers are diagonal PCG and forced CMG;
    // automatic route selection remains outside this public tuple.  All other
    // projection/inference combinations retain the exact Mata route and its
    // established guards.
    local scalable_project_requested = `project_supplied' &       ///
        "`inference'" == "none" & "`backend_requested'" == "rust" & ///
        "`rng_requested'" == "counter_v1" & `algorithm_supplied' & ///
        lower(strtrim(`"`algorithm'"')) == "jla" &                 ///
        ((inlist(lower(strtrim(`"`deletion'"')),"","match") &    ///
            inlist(lower(strtrim(`"`stayers'"')),"","movers","both")) | ///
         (lower(strtrim(`"`deletion'"'))=="observation" &          ///
            inlist(lower(strtrim(`"`stayers'"')),"","movers"))) & ///
        `preconditioner_supplied' &                                ///
        inlist(lower(strtrim(`"`preconditioner'"')),               ///
            "diagonal", "cmg") &                                 ///
        inlist(lower(strtrim(`"`engine'"')), "", "auto", "generic")

    if `inference_requested' & "`backend_requested'" == "rust" & ///
        !(`scalable_project_requested' | `scalable_component_requested') {
        if `project_supplied' {
            quietly _vckss_post_failure "RUST_INFERENCE_UNSUPPORTED" ///
                "Rust project() requires the explicit qualified JLA generic-engine tuple with observation or match deletion and diagonal PCG or forced CMG."
            di as error "backend(rust) project() is outside the qualified sparse tuple"
        }
        else {
            quietly _vckss_post_failure "RUST_INFERENCE_UNSUPPORTED" ///
                "Rust component inference requires explicit inferencemodel(structured_common|structured_leverage), deletion(observation), algorithm(jla), rng(counter_v1), joint nuisance handling, mover-only data, and diagonal PCG or forced CMG."
            di as error "backend(rust) component inference is outside the explicit structured tuple"
        }
        exit 498
    }
    if `inference_requested' & "`rng_requested'" == "counter_v1" & ///
        !(`scalable_project_requested' | `scalable_component_requested') {
        if `project_supplied' {
            quietly _vckss_post_failure "COUNTER_INFERENCE_UNSUPPORTED" ///
                "Counter-V1 project() requires the explicit qualified JLA generic-engine tuple with observation or match deletion and diagonal PCG or forced CMG."
            di as error "rng(counter_v1) project() is outside the qualified sparse tuple"
        }
        else {
            quietly _vckss_post_failure "COUNTER_INFERENCE_UNSUPPORTED" ///
                "Counter-V1 component inference is available only for the explicit structured Rust tuple."
            di as error "rng(counter_v1) component inference is outside the explicit structured tuple"
        }
        exit 498
    }

    global VCKSS_ROUTE_METADATA_READY 1
    global VCKSS_ROUTE_BACKEND_REQUESTED "`backend_requested'"
    global VCKSS_ROUTE_BACKEND_SELECTED ""
    global VCKSS_ROUTE_BACKEND_REASON "routing validation has not completed"
    global VCKSS_ROUTE_BACKEND_SUPPLIED `backend_supplied'
    global VCKSS_ROUTE_RNG_REQUESTED "`rng_requested'"
    global VCKSS_ROUTE_RNG_SELECTED ""
    global VCKSS_ROUTE_RNG_SUPPLIED `rng_supplied'
    global VCKSS_ROUTE_FALLBACK 0
    global VCKSS_ROUTE_FB_REASON ""
    global VCKSS_ROUTE_FB_PHASE ""

    if !inlist("`backend_requested'", "auto", "mata", "rust") {
        global VCKSS_ROUTE_BACKEND_REASON "invalid backend() value"
        quietly _vckss_post_failure "INVALID_BACKEND"            ///
            "backend() must be auto, mata, or rust."
        ereturn local backend_requested `"`backend_requested'"'
        ereturn local backend_selected ""
        ereturn local backend_routing_reason "invalid backend() value"
        ereturn scalar backend_option_supplied = `backend_supplied'
        di as error "backend() must be auto, mata, or rust"
        exit 198
    }
    if !inlist("`rng_requested'", "auto", "stata", "counter_v1") {
        global VCKSS_ROUTE_BACKEND_REASON "invalid rng() value"
        quietly _vckss_post_failure "INVALID_RNG"               ///
            "rng() must be auto, stata, or counter_v1."
        di as error "rng() must be auto, stata, or counter_v1"
        exit 198
    }

    local rust_exact_requested =                              ///
        lower(strtrim(`"`algorithm'"')) == "exact"
    local rust_generic_requested =                            ///
        lower(strtrim(`"`algorithm'"')) == "jla" &          ///
        lower(strtrim(`"`engine'"')) == "generic"
    local rust_strict = "`backend_requested'" == "rust" |   ///
        "`rng_requested'" == "counter_v1"
    local rust_public = 0
    local backend_selected mata
    local rng_selected stata
    if "`backend_requested'" == "rust" & "`rng_requested'" == "stata" {
        global VCKSS_ROUTE_BACKEND_REASON ///
            "strict Rust rejected because rng(stata) selects the Mata runtime"
        quietly _vckss_post_failure "RUST_RNG_BACKEND_MISMATCH" ///
            "backend(rust) cannot be combined with rng(stata)."
        ereturn local backend_requested "rust"
        ereturn local backend_selected ""
        ereturn local rng_requested "`rng_requested'"
        ereturn local rng_selected ""
        ereturn scalar backend_option_supplied = `backend_supplied'
        ereturn scalar rng_option_supplied = `rng_supplied'
        ereturn scalar algorithm_option_supplied = `algorithm_supplied'
        ereturn scalar engine_option_supplied = `engine_supplied'
        ereturn scalar preconditioner_option_supplied = `preconditioner_supplied'
        ereturn scalar batch_option_supplied = `batch_supplied'
        ereturn scalar stayers_option_supplied = `stayers_supplied'
        ereturn local algorithm = lower(strtrim(`"`algorithm'"'))
        ereturn local engine_requested = lower(strtrim(`"`engine'"'))
        ereturn local preconditioner_requested =                  ///
            lower(strtrim(`"`preconditioner'"'))
        ereturn scalar backend_fallback = 0
        di as error "backend(rust) cannot be combined with rng(stata)"
        exit 498
    }
    if "`backend_requested'" == "mata" & "`rng_requested'" == "counter_v1" {
        global VCKSS_ROUTE_BACKEND_REASON ///
            "Counter-V1 rejected because backend(mata) was explicitly selected"
        quietly _vckss_post_failure "COUNTER_RNG_BACKEND_MISMATCH" ///
            "rng(counter_v1) cannot be combined with backend(mata)."
        ereturn local backend_requested "`backend_requested'"
        ereturn local backend_selected ""
        ereturn local rng_requested "`rng_requested'"
        ereturn local rng_selected ""
        ereturn scalar backend_option_supplied = `backend_supplied'
        ereturn scalar rng_option_supplied = `rng_supplied'
        ereturn scalar backend_fallback = 0
        di as error "rng(counter_v1) cannot be combined with backend(mata)"
        exit 498
    }
    if `inference_requested' &                              ///
        !(`scalable_project_requested' | `scalable_component_requested') {
        local rust_public = 0
        local backend_selected mata
        local rng_selected stata
        local backend_routing_reason                         ///
            "explicit inference request selected the exact Mata inference runtime"
    }
    else if "`backend_requested'" == "mata" | "`rng_requested'" == "stata" {
        if "`backend_requested'" == "mata" {
            local backend_routing_reason "backend(mata) explicitly selected"
        }
        else {
            local backend_routing_reason ///
                "rng(stata) selected Mata before native preflight"
        }
    }
    else {
        local rust_public = 1
        local backend_selected rust
        if `rust_exact_requested' {
            local rng_selected NOT_APPLICABLE
        }
        else local rng_selected counter_v1
        if `rust_strict' {
            local backend_routing_reason ///
                "strict Rust route selected before capability preflight"
        }
        else {
            local backend_routing_reason ///
                "automatic backend prefers Rust pending capability preflight"
        }
    }
    if `rust_public' {
        global VCKSS_ROUTE_BACKEND_SELECTED ""
        global VCKSS_ROUTE_RNG_SELECTED ""
    }
    else {
        global VCKSS_ROUTE_BACKEND_SELECTED "`backend_selected'"
        global VCKSS_ROUTE_RNG_SELECTED "`rng_selected'"
    }
    global VCKSS_ROUTE_BACKEND_REASON `"`backend_routing_reason'"'

    local wallseconds_supplied = ("`wallseconds'" != "")
    if `wallseconds_supplied' local wallseconds = real("`wallseconds'")
    else local wallseconds = .

    gettoken depvar controls : varlist
    capture confirm numeric variable `depvar'
    if _rc {
        quietly _vckss_post_failure "INVALID_DEPVAR"
        di as error "the dependent variable must be a numeric variable"
        exit 109
    }
    if "`worker'" == "`firm'" {
        quietly _vckss_post_failure "INVALID_IDENTIFIER"
        di as error "worker() and firm() must identify distinct dimensions"
        exit 198
    }

    if "`deletion'" == "" local deletion match
    local deletion = lower(strtrim("`deletion'"))
    if !inlist("`deletion'", "match", "observation") {
        quietly _vckss_post_failure "UNSUPPORTED_DELETION"
        di as error "deletion() must be match or observation"
        exit 198
    }
    if "`deletion'" == "observation" & "`deletionid'" != "" {
        quietly _vckss_post_failure "UNSUPPORTED_DELETION_ID"
        di as error "deletionid() is not allowed with deletion(observation)"
        exit 198
    }
    if "`inference'" != "none" & "`deletion'" != "observation" {
        quietly _vckss_post_failure "INFERENCE_DELETION_UNSUPPORTED" ///
            "The registered inference surface requires deletion(observation)."
        di as error "inference requires deletion(observation)"
        exit 498
    }

    if "`algorithm'" == "" {
        if `inference_requested' local algorithm exact
        else local algorithm jla
    }
    local algorithm = lower(strtrim("`algorithm'"))
    if !inlist("`algorithm'", "auto", "exact", "jla") {
        quietly _vckss_post_failure "UNSUPPORTED_ALGORITHM"
        di as error "algorithm() must be auto, exact, or jla"
        exit 198
    }
    if `inference_requested' & "`algorithm'" == "jla" &         ///
        !(`scalable_project_requested' | `scalable_component_requested') {
        quietly _vckss_post_failure "JLA_INFERENCE_UNSUPPORTED"  ///
            "Inference is not available from randomized diagonal approximations."
        di as error "inference does not support algorithm(jla)"
        exit 498
    }
    if `inference_requested' & "`algorithm'" == "auto" {
        local algorithm exact
    }
    if "`nuisance'" == "" local nuisance joint
    local nuisance = lower(strtrim("`nuisance'"))
    if !inlist("`nuisance'", "joint", "fixedoffset") {
        quietly _vckss_post_failure "INVALID_NUISANCE"
        di as error "nuisance() must be joint or fixedoffset"
        exit 198
    }
    if "`stayers'" == "" {
        if "`deletion'" == "match" local stayers both
        else local stayers movers
    }
    local stayers = lower(strtrim("`stayers'"))
    if !inlist("`stayers'", "movers", "both") {
        quietly _vckss_post_failure "INVALID_STAYER_CONVENTION"
        di as error "stayers() must be movers or both"
        exit 198
    }
    if "`deletion'" == "observation" & "`stayers'" == "both" {
        quietly _vckss_post_failure "STAYER_HYBRID_DELETION_UNSUPPORTED" ///
            "The mixed-deletion stayer hybrid is defined only for a mover-match headline."
        di as error "stayers(both) requires deletion(match)"
        exit 498
    }
    if "`inference'" != "none" & "`stayers'" != "movers" {
        quietly _vckss_post_failure "INFERENCE_STAYER_UNSUPPORTED" ///
            "Inference is defined for the single retained observation-deletion population."
        di as error "inference requires stayers(movers)"
        exit 498
    }
    if "`preconditioner'" == "" local preconditioner auto
    local preconditioner = lower(strtrim("`preconditioner'"))
    if !inlist("`preconditioner'", "auto", "diagonal", "cmg") {
        quietly _vckss_post_failure "INVALID_PRECONDITIONER"
        di as error "preconditioner() must be auto, diagonal, or cmg"
        exit 198
    }
    if `memory_gib' <= 0 | missing(`memory_gib') {
        quietly _vckss_post_failure "INVALID_MEMORY_ENVELOPE"
        di as error "memory_gib() must be positive"
        exit 198
    }
    if "`engine'" == "" local engine auto
    local engine_requested = lower(strtrim("`engine'"))
    if !inlist("`engine_requested'", "auto", "compressed", "generic") {
        quietly _vckss_post_failure "INVALID_ENGINE"
        di as error "engine() must be auto, compressed, or generic"
        exit 198
    }
    if `wallseconds_supplied' &                                ///
        (missing(`wallseconds') | `wallseconds' <= 0) {
        quietly _vckss_post_failure "INVALID_WALL_ENVELOPE"
        di as error "wallseconds() must be positive when supplied"
        exit 198
    }
    if "`batch'" == "" local batch auto
    local batch_requested = lower(strtrim("`batch'"))
    local batch_routing_reason "caller supplied an explicit batch width"
    local batch_memory_budget_bytes = .
    local batch_column_forecast_bytes = .
    local batch_scratch_forecast_bytes = .
    if "`batch_requested'" == "auto" {
        // The production policy is finalized after the retained sample and
        // parameter dimensions are known.  Eight is the safe small-sample
        // default and the minimum production matrix-matrix width.
        local batch 8
        local batch_routing_reason "small-sample automatic batch floor"
    }
    else {
        capture confirm integer number `batch_requested'
        if _rc {
            quietly _vckss_post_failure "INVALID_TUNING"
            di as error "batch() must be auto or a positive integer"
            exit 198
        }
        local batch = real("`batch_requested'")
    }
    local active_processors = c(processors)

    if `probes' < 2 {
        quietly _vckss_post_failure "INVALID_TUNING"
        di as error "probes() must be at least two"
        exit 198
    }
    if `batch' < 1 {
        quietly _vckss_post_failure "INVALID_TUNING"
        di as error "batch() must be positive"
        exit 198
    }
    if `seed' < 0 | `seed' > 2147483646 {
        quietly _vckss_post_failure "INVALID_TUNING"
        di as error "seed() must be between zero and 2,147,483,646"
        exit 198
    }
    if `tolerance' < 1e-15 | `tolerance' > 1e-4 {
        quietly _vckss_post_failure "INVALID_TUNING"
        di as error "tolerance() must lie in [1e-15,1e-4]"
        exit 198
    }
    if `maxiter' < 1 {
        quietly _vckss_post_failure "INVALID_TUNING"
        di as error "maxiter() must be positive"
        exit 198
    }
    if `exact_limit' < 2 | `exact_limit' > 2000 {
        quietly _vckss_post_failure "INVALID_TUNING"
        di as error "exact_limit() must be between 2 and 2,000"
        exit 198
    }
    if `rank_tolerance' < 1e-14 | `rank_tolerance' >= 0.1 {
        quietly _vckss_post_failure "INVALID_TUNING"
        di as error "rank_tolerance() must lie in [1e-14,0.1)"
        exit 198
    }
    if `block_tolerance' < 1e-14 | `block_tolerance' >= 1 {
        quietly _vckss_post_failure "INVALID_TUNING"
        di as error "block_tolerance() must lie in [1e-14,1)"
        exit 198
    }
    if `blocksize_limit' < 1 | `blocksize_limit' > 1000000 {
        quietly _vckss_post_failure "INVALID_TUNING"
        di as error "blocksize_limit() must be between 1 and 1,000,000"
        exit 198
    }
    if `physical_limit' < 1 | `physical_limit' > 1000000000 {
        quietly _vckss_post_failure "INVALID_TUNING"
        di as error "physical_limit() must be between 1 and 1,000,000,000"
        exit 198
    }

    // Freeze the qualified CMG_FULL_V2 cell before any sample transformation.
    // Explicit Rust retains its strict consent tuple; automatic routing may
    // select the same effective cell only on the qualified macOS/Linux builds.
    // The one combined bit controls both raw implicit-match preparation and
    // the V5 solve request, so the two paths cannot drift.
    local rust_full_cmg_platform =                         ///
        strpos(lower(`"`c(machine_type)'"'),"mac") > 0 | ///
        `"`c(os)'"' == "Unix"
    local rust_full_cmg_common =                           ///
        `rust_public' & `rust_full_cmg_platform' &         ///
        "`algorithm'"=="jla" &                           ///
        "`engine_requested'"=="auto" &                  ///
        "`preconditioner'"=="auto" &                   ///
        "`batch_requested'"=="auto" &                  ///
        "`deletion'"=="match" & "`nuisance'"=="joint" & ///
        "`stayers'"=="movers" & "`probeorder'"!="" &  ///
        strtrim(`"`controls'"')=="" & !`targetweight_supplied' & ///
        !`deletionid_supplied' & strtrim("`weight'")==""
    local rust_full_cmg_explicit =                         ///
        `rust_full_cmg_common' &                           ///
        "`backend_requested'"=="rust" & `backend_supplied' & ///
        "`rng_requested'"=="counter_v1" & `rng_supplied' & ///
        `algorithm_supplied' & `engine_supplied' &          ///
        `preconditioner_supplied' & `batch_supplied'
    local rust_full_cmg_auto =                             ///
        `rust_full_cmg_common' &                           ///
        "`backend_requested'"=="auto" &                  ///
        "`rng_requested'"=="auto"
    local rust_full_cmg_eligible =                         ///
        `rust_full_cmg_explicit' | `rust_full_cmg_auto'
    local implicit_match = `rust_full_cmg_eligible'

    global VCKSS_ROUTE_ALGORITHM_REQUESTED "`algorithm'"
    global VCKSS_ROUTE_ENGINE_REQUESTED "`engine_requested'"
    global VCKSS_ROUTE_PRECOND_REQUESTED "`preconditioner'"
    global VCKSS_ROUTE_DELETION_REQUESTED "`deletion'"
    global VCKSS_ROUTE_NUISANCE_REQUESTED "`nuisance'"
    global VCKSS_ROUTE_ALGORITHM_SUPPLIED `algorithm_supplied'
    global VCKSS_ROUTE_ENGINE_SUPPLIED `engine_supplied'
    global VCKSS_ROUTE_PRECOND_SUPPLIED `preconditioner_supplied'
    global VCKSS_ROUTE_BATCH_SUPPLIED `batch_supplied'
    global VCKSS_ROUTE_STAYERS_SUPPLIED `stayers_supplied'
    global VCKSS_ROUTE_DELETIONID_SUPPLIED `deletionid_supplied'

    if `rust_public' {
        // The helper converts GiB to an exact integer byte ceiling before
        // native preparation.  Mirror that validation here so malformed
        // Rust-route limits fail structurally without probing the plugin.
        local rust_memory_bytes = floor(`memory_gib' * 1073741824)
        if `rust_memory_bytes' <= 0 |                         ///
            `rust_memory_bytes' > 9007199254740992 {
            if "`algorithm'" == "exact" {
                global VCKSS_ROUTE_BACKEND_REASON ///
                    "explicit Rust exact route rejected by memory-envelope validation"
            }
            else {
                global VCKSS_ROUTE_BACKEND_REASON ///
                    "explicit strict Rust route rejected by memory-envelope validation"
            }
            quietly _vckss_post_failure "INVALID_MEMORY_ENVELOPE" ///
                "memory_gib() must convert to an integer byte limit in (0,2^53]."
            ereturn local backend_requested "rust"
            ereturn local backend_selected ""
            ereturn local rng_requested "`rng_requested'"
            ereturn local rng_selected ""
            ereturn scalar backend_option_supplied = 1
            ereturn scalar rng_option_supplied = `rng_supplied'
            di as error "memory_gib() is outside the exactly representable byte range"
            exit 198
        }
        local rust_legacy_jla_supported =                      ///
            `algorithm_supplied' & "`algorithm'" == "jla" & ///
            `preconditioner_supplied' &                        ///
            "`preconditioner'" == "diagonal" &              ///
            `batch_supplied' & "`batch_requested'" != "auto" & ///
            "`deletion'" == "match" & "`nuisance'" == "joint" & ///
            "`stayers'" == "movers" & strtrim(`"`controls'"') == "" & ///
            "`probeorder'" == "" & !`wallseconds_supplied' & ///
            inlist("`engine_requested'", "auto", "compressed") & ///
            `rank_tolerance' == 1e-10 & `block_tolerance' == 1e-10 & ///
            `exact_limit' == 500 & `blocksize_limit' == 5000 & ///
            `physical_limit' == 50000000
        local rust_generic_supported =                         ///
            `algorithm_supplied' & "`algorithm'" == "jla" & ///
            `engine_supplied' & "`engine_requested'" == "generic" & ///
            `preconditioner_supplied' &                        ///
            "`preconditioner'" == "diagonal" &              ///
            `batch_supplied' & "`batch_requested'" != "auto" & ///
            `rng_supplied' & "`rng_requested'" == "counter_v1" & ///
            "`stayers'" == "movers" &                          ///
            "`probeorder'" == "" & !`wallseconds_supplied'
        local rust_auto_engine_generic =                        ///
            "`engine_requested'"=="auto" &                         ///
            ("`deletion'"=="observation" |                        ///
                strtrim(`"`controls'"')!="")
        local rust_auto_engine_compressed =                     ///
            "`engine_requested'"=="auto" &                         ///
            "`deletion'"=="match" &                              ///
            strtrim(`"`controls'"')==""
        local rust_planned_generic_supported =                 ///
            ( `scalable_project_requested' |                    ///
            `scalable_component_requested' |                   ///
            ("`algorithm'" == "jla" &                           ///
            ("`engine_requested'"=="generic" |                   ///
                `rust_auto_engine_generic' |                       ///
                `rust_auto_engine_compressed') &                   ///
            (inlist("`preconditioner'","auto","cmg") |           ///
                ("`preconditioner'"=="diagonal" &                 ///
                    ("`batch_requested'"=="auto" |                ///
                        `wallseconds_supplied' |                    ///
                        "`probeorder'"!="" |                      ///
                        "`rng_requested'"=="auto" |                ///
                        !`algorithm_supplied' | !`engine_supplied' | ///
                        !`preconditioner_supplied' | !`batch_supplied' | ///
                        "`stayers'"=="both"))) &                  ///
            inlist("`deletion'","match","observation") &          ///
            inlist("`nuisance'","joint","fixedoffset") &          ///
            ("`stayers'" == "movers" |                             ///
                ("`stayers'"=="both" & "`deletion'"=="match" & ///
                    "`probeorder'"=="" & !`wallseconds_supplied'))) )
        local rust_auto_exact_supported =                      ///
            "`algorithm'" == "auto" &                            ///
            "`engine_requested'" == "auto" &                     ///
            "`preconditioner'" == "auto" &                       ///
            "`batch_requested'" == "auto" &                      ///
            inlist("`deletion'","match","observation") &          ///
            inlist("`nuisance'","joint","fixedoffset") &          ///
            ("`stayers'" == "movers" |                             ///
                ("`stayers'"=="both" & "`deletion'"=="match")) & ///
            "`probeorder'" == "" &                                  ///
            !`wallseconds_supplied'
        local rust_exact_supported =                           ///
            `algorithm_supplied' & "`algorithm'" == "exact" & ///
            "`stayers'" == "movers" & "`probeorder'" == "" & ///
            !`wallseconds_supplied' &                          ///
            "`preconditioner'" == "auto" &                   ///
            inlist("`engine_requested'", "auto", "generic")
        local rust_exact_stayer_supported =                    ///
            `algorithm_supplied' & "`algorithm'" == "exact" & ///
            "`stayers'" == "both" & "`deletion'" == "match" & ///
            "`probeorder'" == "" & !`wallseconds_supplied' &  ///
            "`preconditioner'" == "auto" &                   ///
            "`batch_requested'" == "auto" &                  ///
            inlist("`engine_requested'", "auto", "generic")
        local rust_options_supported =                         ///
            `rust_legacy_jla_supported' | `rust_generic_supported' | ///
            `rust_planned_generic_supported' |                       ///
            `rust_auto_exact_supported' | `rust_exact_supported' |   ///
            `rust_exact_stayer_supported'
        if !`rust_options_supported' {
            if !`rust_strict' {
                local rust_public = 0
                local backend_selected mata
                local rng_selected stata
                local backend_fallback = 1
                local backend_fallback_reason "RUST_OPTION_UNSUPPORTED"
                local backend_fallback_phase "preflight"
                local backend_routing_reason ///
                    "Rust preflight declined the effective request; fell back to Mata before preparation and estimator RNG"
                global VCKSS_ROUTE_BACKEND_SELECTED "mata"
                global VCKSS_ROUTE_RNG_SELECTED "stata"
                global VCKSS_ROUTE_BACKEND_REASON `"`backend_routing_reason'"'
                global VCKSS_ROUTE_FALLBACK 1
                global VCKSS_ROUTE_FB_REASON ///
                    "RUST_OPTION_UNSUPPORTED"
                global VCKSS_ROUTE_FB_PHASE "preflight"
            }
            else if "`algorithm'" == "exact" {
                global VCKSS_ROUTE_BACKEND_REASON ///
                    "explicit Rust exact route rejected an unsupported option combination"
            }
            else {
                global VCKSS_ROUTE_BACKEND_REASON ///
                    "explicit strict Rust route rejected an unsupported option combination"
            }
            if `rust_strict' {
                quietly _vckss_post_failure "RUST_OPTION_UNSUPPORTED" ///
                    "The effective request is outside the supported Rust subset."
                ereturn local backend_requested "`backend_requested'"
                ereturn local backend_selected ""
                ereturn local rng_requested "`rng_requested'"
                ereturn local rng_selected ""
                ereturn scalar backend_option_supplied = `backend_supplied'
                ereturn scalar rng_option_supplied = `rng_supplied'
                ereturn scalar backend_fallback = 0
                di as error "the requested option combination is outside the supported Rust subset"
                exit 498
            }
        }
        if `rust_public' {
        capture quietly fevc_rust probe
        local rust_probe_rc = _rc
        if `rust_probe_rc' {
            if !`rust_strict' {
                capture quietly fevc_rust clear
                local rust_public = 0
                local backend_selected mata
                local rng_selected stata
                local backend_fallback = 1
                local backend_fallback_reason "RUST_BACKEND_UNAVAILABLE"
                local backend_fallback_phase "preflight"
                local backend_routing_reason ///
                    "Rust preflight unavailable; fell back to Mata before preparation and estimator RNG"
                global VCKSS_ROUTE_BACKEND_SELECTED "mata"
                global VCKSS_ROUTE_RNG_SELECTED "stata"
                global VCKSS_ROUTE_BACKEND_REASON `"`backend_routing_reason'"'
                global VCKSS_ROUTE_FALLBACK 1
                global VCKSS_ROUTE_FB_REASON ///
                    "RUST_BACKEND_UNAVAILABLE"
                global VCKSS_ROUTE_FB_PHASE "preflight"
            }
            else if "`algorithm'" == "exact" {
                global VCKSS_ROUTE_BACKEND_REASON ///
                    "explicit Rust exact route could not load or probe the native backend"
            }
            else {
                global VCKSS_ROUTE_BACKEND_REASON ///
                    "explicit strict Rust route could not load or probe the native backend"
            }
            if `rust_strict' {
                quietly _vckss_post_failure "RUST_BACKEND_UNAVAILABLE" ///
                    "The Rust plugin could not be loaded or probed."
                ereturn local backend_requested "`backend_requested'"
                ereturn local backend_selected ""
                ereturn local rng_requested "`rng_requested'"
                ereturn local rng_selected ""
                ereturn scalar backend_option_supplied = `backend_supplied'
                ereturn scalar rng_option_supplied = `rng_supplied'
                ereturn scalar backend_fallback = 0
                di as error "the Rust backend plugin is unavailable"
                exit 498
            }
        }
        if `rust_public' {
        local rust_abi_compiled = r(abi_compiled)
        local rust_abi_runtime = r(abi_runtime)
        local rust_core_flags = r(core_ready_flags)
        local rust_support_flags = r(support_flags)
        local rust_deterministic = r(deterministic_parallelism)
        local rust_core_required =                              ///
            mod(floor(`rust_core_flags'/1),2) == 1
        // All Rust routes on a qualified full-CMG platform require the new
        // runtime readiness bit, so a pre-V2 plugin cannot be mixed with this
        // ado build. Windows omits the bit and retains its existing routes.
        if `rust_full_cmg_platform' {
            local rust_core_required = `rust_core_required' &   ///
                mod(floor(`rust_core_flags'/256),2) == 1
        }
        if `scalable_project_requested' {
            local rust_core_required = `rust_core_required' &   ///
                mod(floor(`rust_core_flags'/512),2) == 1
        }
        if "`algorithm'" == "exact" & "`stayers'" == "movers" {
            local rust_core_required = `rust_core_required' &   ///
                mod(floor(`rust_core_flags'/2),2) == 1
        }
        else {
            local rust_core_required = `rust_core_required' &   ///
                mod(floor(`rust_core_flags'/4),2) == 1 &        ///
                mod(floor(`rust_core_flags'/8),2) == 1 &        ///
                mod(floor(`rust_core_flags'/32),2) == 1 &       ///
                mod(floor(`rust_core_flags'/64),2) == 1 &       ///
                mod(floor(`rust_core_flags'/128),2) == 1
            if `rust_auto_exact_supported' | `rust_exact_stayer_supported' {
                local rust_core_required = `rust_core_required' & ///
                    mod(floor(`rust_core_flags'/2),2) == 1
            }
        }
        local rust_transport_valid =                           ///
            !missing(`rust_abi_compiled') &                    ///
            !missing(`rust_abi_runtime') &                     ///
            !missing(`rust_deterministic') &                   ///
            !missing(`rust_core_flags') &                      ///
            `rust_core_flags' == floor(`rust_core_flags') &    ///
            !missing(`rust_support_flags') &                   ///
            `rust_support_flags' == floor(`rust_support_flags')
        // The frozen flat mask remains the legacy transport receipt.  Request
        // combinations are authorized only by requestcapability below.
        local rust_support_required = (`rust_support_flags' == 38)
        if !`rust_transport_valid' | `rust_abi_compiled' != 1 | ///
            `rust_abi_runtime' != 1 | `rust_deterministic' != 1 | ///
            !`rust_core_required' | !`rust_support_required' {
            if "`algorithm'" == "exact" {
                global VCKSS_ROUTE_BACKEND_REASON ///
                    "explicit Rust exact route rejected an invalid native transport receipt"
            }
            else {
                global VCKSS_ROUTE_BACKEND_REASON ///
                    "explicit strict Rust route rejected an unqualified native capability receipt"
            }
            quietly _vckss_post_failure "RUST_BACKEND_UNQUALIFIED" ///
                "The loaded Rust plugin did not satisfy the public transport and core-readiness contract."
            ereturn local backend_requested "rust"
            ereturn local backend_selected ""
            ereturn local rng_requested "`rng_requested'"
            ereturn local rng_selected ""
            ereturn scalar backend_option_supplied = 1
            ereturn scalar rng_option_supplied = `rng_supplied'
            ereturn scalar rust_core_ready_flags = `rust_core_flags'
            ereturn scalar rust_support_flags = `rust_support_flags'
            di as error "the loaded Rust backend is not qualified for this request"
            exit 498
        }
        if `rust_strict' {
            local backend_routing_reason ///
                "strict Rust request accepted by complete transport preflight"
        }
        else {
            local backend_routing_reason ///
                "automatic backend selected Rust after complete transport preflight"
        }
        global VCKSS_ROUTE_BACKEND_REASON `"`backend_routing_reason'"'
        }
        }
    }

    // A missing runtime or unsupported capability may have moved an
    // automatic request to Mata during preflight. Native implicit-match
    // preparation is meaningful only while the qualified Rust cell remains
    // selected; every fallback must re-enter the ordinary Mata preparation.
    local implicit_match = `rust_public' & `rust_full_cmg_eligible'

    /* PREP-BND-PERF-V1 observes command-boundary work only.  These
       diagnostics never participate in routing, RNG, or acceptance. */
    local prep_mark_validate_seconds = 0
    local prep_initial_group_seconds = 0
    local prep_runtime_setup_seconds = 0
    local prep_graph_setup_io_seconds = 0
    local prep_retained_map_seconds = 0
    local prep_semantic_group_calls = 0
    local prep_sort_calls = 0
    local prep_compression_import_columns = 0
    local prep_compression_import_rows = 0
    quietly timer on $VCKSS_STAGE_SELECTION_TIMER
    tempvar requested touse hybrid_complete
    mark `requested' `if' `in'
    quietly count if `requested'
    local N_scope = r(N)
    local weightvar
    if "`weight'" != "" {
        local weightvar = substr("`exp'",2,.)
        capture confirm numeric variable `weightvar'
        if _rc {
            quietly _vckss_post_failure "INVALID_FREQUENCY"
            di as error "the frequency weight must be a numeric variable"
            exit 198
        }
        quietly count if `requested' & missing(`weightvar')
        if r(N) {
            quietly _vckss_post_failure "INVALID_FREQUENCY"
            di as error "frequency weights must be finite and strictly positive"
            exit 198
        }
        quietly summarize `weightvar' if `requested', meanonly
        if r(min) <= 0 | r(max) >= . {
            quietly _vckss_post_failure "INVALID_FREQUENCY"
            di as error "frequency weights must be finite and strictly positive"
            exit 198
        }
        quietly count if `weightvar' != floor(`weightvar') & `requested'
        if r(N) {
            quietly _vckss_post_failure "INVALID_FREQUENCY"
            di as error "frequency weights must be positive integers"
            exit 198
        }
    }

    if "`targetweight'" != "" {
        quietly count if `requested' & missing(`targetweight')
        if r(N) {
            quietly _vckss_post_failure "INVALID_TARGET_WEIGHT"
            di as error "targetweight() must be finite, nonnegative, and have positive total mass"
            exit 198
        }
        quietly summarize `targetweight' if `requested', meanonly
        if r(min) < 0 | r(max) >= . | r(sum) <= 0 {
            quietly _vckss_post_failure "INVALID_TARGET_WEIGHT"
            di as error "targetweight() must be finite, nonnegative, and have positive total mass"
            exit 198
        }
    }

    mark `touse' `if' `in'
    markout `touse' `depvar'
    markout `touse' `worker' `firm', strok
    if "`deletionid'" != "" markout `touse' `deletionid', strok
    if `project_supplied' markout `touse' `project'
    if "`probeorder'" != "" {
        quietly count if `requested' & missing(`probeorder')
        if r(N) {
            quietly _vckss_post_failure "INVALID_PROBE_ORDER"
            di as error "probeorder() must be complete on the requested sample"
            exit 198
        }
    }

    local controlvars
    if strtrim(`"`controls'"') != "" {
        capture quietly fvexpand `controls' if `touse'
        if _rc {
            quietly _vckss_post_failure "INVALID_CONTROLS"
            di as error "controls could not be expanded into numeric columns"
            exit _rc
        }
        local expanded_terms `r(varlist)'
        capture quietly fvrevar `expanded_terms' if `touse'
        if _rc {
            quietly _vckss_post_failure "INVALID_CONTROLS"
            di as error "controls could not be materialized as numeric columns"
            exit _rc
        }
        local expanded_controls `r(varlist)'
        local term_count : word count `expanded_terms'
        local control_count_expanded : word count `expanded_controls'
        if `term_count' != `control_count_expanded' {
            quietly _vckss_post_failure "INVALID_CONTROLS"
            di as error "factor-variable expansion did not preserve control metadata"
            exit 498
        }
        forvalues control_index = 1/`term_count' {
            local term : word `control_index' of `expanded_terms'
            local control : word `control_index' of `expanded_controls'
            capture quietly _ms_parse_parts `term'
            if _rc {
                quietly _vckss_post_failure "INVALID_CONTROLS"
                di as error "expanded control metadata could not be parsed"
                exit 498
            }
            // Only Stata-designated omitted factor terms are removed.  A
            // user-supplied zero or collinear regressor remains in the design
            // and must be rejected by the registered rank gates.
            if !r(omit) local controlvars `controlvars' `control'
        }
        if strtrim(`"`controlvars'"') != "" markout `touse' `controlvars'
    }

    quietly generate byte `hybrid_complete' = `touse'

    quietly count if `touse'
    local N_complete = r(N)
    if "`deletion'" == "match" & `N_complete' != `N_scope' {
        local missing_rows = `N_scope' - `N_complete'
        quietly _vckss_post_failure "MATCH_INPUT_MISSING"
        di as error "match deletion requires complete frozen inputs; `missing_rows' requested row(s) are incomplete"
        exit 459
    }
    if `N_complete' == 0 {
        quietly _vckss_post_failure "NO_USABLE_OBSERVATIONS"
        error 2000
    }

    // Flat support bits are a frozen legacy receipt, not a compositional
    // authorization surface.  Exact requests cross the typed capability
    // boundary only after factor controls have become concrete columns.
    if `rust_public' & "`algorithm'" == "exact" {
        local rust_cap_control_count : word count `controlvars'
        local rust_cap_frequency_used = ("`weightvar'" != "")
        local rust_cap_deletion_expected =                       ///
            cond("`deletion'" == "match",1,2)
        local rust_cap_nuisance_expected =                       ///
            cond("`nuisance'" == "joint",1,2)
        capture quietly fevc_rust requestcapability,      ///
            algorithm(exact) deletion(`deletion') nuisance(`nuisance') ///
            route(exact) rngcontract(none)                       ///
            controls(`rust_cap_control_count')                   ///
            frequencyused(`rust_cap_frequency_used')
        local rust_capability_rc = _rc
        if `rust_capability_rc' {
            capture quietly fevc_rust clear
            global VCKSS_ROUTE_BACKEND_REASON                    ///
                "explicit Rust exact route could not obtain a request-capability receipt"
            quietly _vckss_post_failure "RUST_BACKEND_UNAVAILABLE" ///
                "The Rust request-capability query was unavailable; no native preparation was attempted."
            ereturn scalar native_error_code = .
            ereturn local native_error_phase "request_capability"
            ereturn scalar rust_core_ready_flags = `rust_core_flags'
            ereturn scalar rust_support_flags = `rust_support_flags'
            exit 498
        }
        local rust_cap_struct = r(struct_size)
        local rust_cap_abi = r(abi_version)
        local rust_cap_schema = r(request_schema)
        local rust_cap_supported = r(supported)
        local rust_cap_reason = r(reason_code)
        local rust_cap_profile = r(profile_code)
        local rust_cap_algorithm = r(algorithm_code)
        local rust_cap_deletion = r(deletion_mode_code)
        local rust_cap_nuisance = r(nuisance_mode_code)
        local rust_cap_route = r(solver_route_code)
        local rust_cap_rng = r(rng_contract_code)
        local rust_cap_controls = r(controls_count)
        local rust_cap_frequency = r(frequency_use_code)
        local rust_cap_signature_hi = r(request_signature_hi)
        local rust_cap_signature_lo = r(request_signature_lo)
        local rust_cap_reason_name `"`r(reason)'"'
        local rust_cap_profile_name `"`r(profile)'"'
        quietly _vckss_request_signature 1 1                       ///
            `rust_cap_deletion_expected' `rust_cap_nuisance_expected' ///
            1 0 `rust_cap_control_count' `rust_cap_frequency_used'
        local rust_cap_expected_signature_hi = r(signature_hi)
        local rust_cap_expected_signature_lo = r(signature_lo)
        local rust_capability_receipt_ok = 1
        foreach receipt in rust_cap_struct rust_cap_abi rust_cap_schema ///
            rust_cap_supported rust_cap_reason rust_cap_profile       ///
            rust_cap_algorithm rust_cap_deletion rust_cap_nuisance    ///
            rust_cap_route rust_cap_rng rust_cap_controls             ///
            rust_cap_frequency rust_cap_signature_hi                  ///
            rust_cap_signature_lo {
            if missing(``receipt'') | ``receipt'' < 0 |              ///
                ``receipt'' != floor(``receipt'') {
                local rust_capability_receipt_ok = 0
            }
        }
        if `rust_capability_receipt_ok' {
            local rust_capability_receipt_ok =                       ///
                `rust_cap_struct' == 64 & `rust_cap_abi' == 1 &     ///
                `rust_cap_schema' == 1 &                            ///
                inlist(`rust_cap_supported',0,1) &                  ///
                `rust_cap_algorithm' == 1 &                         ///
                `rust_cap_deletion' == `rust_cap_deletion_expected' & ///
                `rust_cap_nuisance' == `rust_cap_nuisance_expected' & ///
                `rust_cap_route' == 1 & `rust_cap_rng' == 0 &       ///
                `rust_cap_controls' == `rust_cap_control_count' &   ///
                `rust_cap_frequency' == `rust_cap_frequency_used' & ///
                `rust_cap_signature_hi' ==                          ///
                    `rust_cap_expected_signature_hi' &              ///
                `rust_cap_signature_lo' ==                          ///
                    `rust_cap_expected_signature_lo' &              ///
                `rust_cap_signature_hi' <= 4294967295 &             ///
                `rust_cap_signature_lo' <= 4294967295 &             ///
                (`rust_cap_supported' == 1) == (`rust_cap_reason' == 0) & ///
                cond(`rust_cap_supported' == 1,                     ///
                    `rust_cap_profile' == 1, `rust_cap_profile' == 0)
        }
        if !`rust_capability_receipt_ok' {
            capture quietly fevc_rust clear
            global VCKSS_ROUTE_BACKEND_REASON                    ///
                "explicit Rust exact route rejected an inconsistent request-capability receipt"
            quietly _vckss_post_failure "RUST_BACKEND_UNQUALIFIED" ///
                "The Rust request-capability receipt did not reconcile with the exact request tuple."
            ereturn scalar native_error_code = .
            ereturn local native_error_phase "request_capability_reconcile"
            ereturn scalar rust_core_ready_flags = `rust_core_flags'
            ereturn scalar rust_support_flags = `rust_support_flags'
            exit 498
        }
        if !`rust_cap_supported' {
            if !`rust_strict' {
                capture quietly fevc_rust clear
                local rust_public = 0
                local backend_selected mata
                local rng_selected stata
                local backend_fallback = 1
                local backend_fallback_reason "RUST_OPTION_UNSUPPORTED"
                local backend_fallback_phase "preflight"
                local backend_routing_reason ///
                    "Rust capability preflight declined the materialized request; fell back to Mata before native preparation and estimator RNG"
                global VCKSS_ROUTE_BACKEND_SELECTED "mata"
                global VCKSS_ROUTE_RNG_SELECTED "stata"
                global VCKSS_ROUTE_BACKEND_REASON `"`backend_routing_reason'"'
                global VCKSS_ROUTE_FALLBACK 1
                global VCKSS_ROUTE_FB_REASON ///
                    "RUST_OPTION_UNSUPPORTED"
                global VCKSS_ROUTE_FB_PHASE "preflight"
            }
            else {
                capture quietly fevc_rust clear
                global VCKSS_ROUTE_BACKEND_REASON                    ///
                    "explicit Rust exact request was declined by the request-capability boundary"
                quietly _vckss_post_failure "RUST_OPTION_UNSUPPORTED" ///
                    "Rust exact request capability declined the materialized tuple: `rust_cap_reason_name'."
                ereturn local rust_request_capability_reason         ///
                    "`rust_cap_reason_name'"
                ereturn scalar rust_cap_reason_code = ///
                    `rust_cap_reason'
                ereturn scalar rust_core_ready_flags = `rust_core_flags'
                ereturn scalar rust_support_flags = `rust_support_flags'
                exit 498
            }
        }
    }

    tempvar frequency target
    local rust_frequency_used = ("`weightvar'" != "")
    if "`weightvar'" == "" quietly generate double `frequency' = 1 if `touse'
    else quietly generate double `frequency' = `weightvar' if `touse'
    if "`targetweight'" == "" quietly generate double `target' = `frequency' if `touse'
    else quietly generate double `target' = `targetweight' if `touse'

    quietly timer off $VCKSS_STAGE_SELECTION_TIMER
    quietly timer list $VCKSS_STAGE_SELECTION_TIMER
    local prep_mark_validate_seconds =                       ///
        r(t$VCKSS_STAGE_SELECTION_TIMER)
    quietly timer on $VCKSS_STAGE_SELECTION_TIMER

    tempvar initial_worker initial_firm pair_first firm_count worker_tag
    tempvar original_stayer
    if `implicit_match' {
        capture confirm numeric variable `worker'
        if !_rc capture confirm numeric variable `firm'
        if _rc {
            quietly _vckss_post_failure "INVALID_IDENTIFIER"      ///
                "CMG_FULL_V2 implicit-match preparation requires numeric worker and firm identifiers."
            di as error "CMG_FULL_V2 implicit-match preparation requires numeric worker() and firm() identifiers"
            exit 198
        }
        quietly count if `touse' &                                ///
            (`worker' != floor(`worker') |                          ///
             abs(`worker') > 9007199254740992 |                     ///
             `firm' != floor(`firm') |                              ///
             abs(`firm') > 9007199254740992)
        if r(N) {
            quietly _vckss_post_failure "INVALID_IDENTIFIER"      ///
                "CMG_FULL_V2 implicit-match preparation requires signed exact binary64 integer identifiers."
            di as error "CMG_FULL_V2 worker and firm IDs must be exact integers between -2^53 and 2^53"
            exit 198
        }
        local initial_worker `worker'
        local initial_firm `firm'
        quietly generate byte `original_stayer' = 0 if `touse'
        local N_stayers = 0
        local N_stayer_rows = 0
    }
    else {
        quietly egen long `initial_worker' = group(`worker') if `touse'
        quietly egen long `initial_firm' = group(`firm') if `touse'
        sort `initial_worker' `initial_firm'
        local prep_sort_calls = `prep_sort_calls' + 1
        quietly by `initial_worker' `initial_firm': generate byte `pair_first' = ///
            (_n == 1) if `touse'
        quietly by `initial_worker': egen long `firm_count' = total(`pair_first') ///
            if `touse'
        quietly egen byte `worker_tag' = tag(`initial_worker') if `touse'
        quietly generate byte `original_stayer' = (`firm_count' == 1) if `touse'
        quietly count if `worker_tag' & `firm_count' == 1 & `touse'
        local N_stayers = r(N)
        quietly count if `firm_count' == 1 & `touse'
        local N_stayer_rows = r(N)
    }

    if `rust_public' {
        tempvar rust_deletion rust_block_rows
        if `implicit_match' {
            // Preserve the six-column input boundary. Rust replaces this
            // placeholder with the certified implicit worker-firm match key.
            quietly generate double `rust_deletion' = `worker' if `touse'
        }
        else if "`deletion'" == "observation" {
            quietly generate long `rust_deletion' = _n if `touse'
        }
        else if "`deletionid'" != "" {
            quietly egen long `rust_deletion' = group(`deletionid') if `touse'
        }
        else {
            quietly egen long `rust_deletion' = group(`initial_worker' `initial_firm') if `touse'
        }
        if "`deletion'" == "match" & !`implicit_match' {
            quietly bysort `rust_deletion': generate long `rust_block_rows' = _N if `touse'
            quietly summarize `rust_block_rows' if `touse', meanonly
            if r(max) > `blocksize_limit' {
                quietly _vckss_post_failure "BLOCK_SIZE_LIMIT"       ///
                    "A requested deletion block exceeds blocksize_limit()."
                di as error "a deletion block exceeds blocksize_limit()"
                exit 198
            }
        }
        // Do not let a prior estimation receipt be mistaken for a failure
        // posted by this prepared lifecycle if an unexpected Stata error or
        // UserBreak unwinds the inner program.
        ereturn clear
        if "`algorithm'" == "exact" & "`stayers'" == "movers" {
            capture noisily _vckss_rexact `depvar'                  ///
                `initial_worker' `initial_firm' `rust_deletion'     ///
                `frequency' `target' `touse' `N_scope' `N_complete' ///
                `N_stayers' `N_stayer_rows' `probes' `batch' `seed' ///
                `tolerance' `maxiter' `memory_gib'                  ///
                `engine_requested' `backend_supplied' `rng_supplied' ///
                `rng_requested' `deletionid_supplied' `rust_core_flags' ///
                `rust_support_flags' `"`nodisplay'"' `deletion'     ///
                `nuisance' `exact_limit' `rank_tolerance'           ///
                `block_tolerance' `blocksize_limit' `physical_limit' ///
                `preconditioner' `batch_requested' `"`controlvars'"' ///
                `rust_cap_struct' `rust_cap_abi' `rust_cap_schema'  ///
                `rust_cap_supported' `rust_cap_reason'              ///
                `rust_cap_profile' `rust_cap_algorithm'             ///
                `rust_cap_deletion' `rust_cap_nuisance'             ///
                `rust_cap_route' `rust_cap_rng' `rust_cap_controls' ///
                `rust_cap_frequency' `rust_cap_signature_hi'        ///
                `rust_cap_signature_lo'
        }
        else if `rust_planned_generic_supported' |              ///
            `rust_auto_exact_supported' | `rust_exact_stayer_supported' {
            local rust_planned_wallseconds = cond(`wallseconds_supplied', ///
                `wallseconds',0)
            capture noisily _fevc_rust_generic_planned `depvar'  ///
                `initial_worker' `initial_firm' `rust_deletion'   ///
                `frequency' `target' `touse' `N_scope' `N_complete' ///
                `N_stayers' `N_stayer_rows' `probes' `batch' `seed' ///
                `tolerance' `maxiter' `memory_gib'                ///
                `algorithm' `engine_requested' `backend_supplied' `rng_supplied' ///
                `deletionid_supplied' `engine_supplied'           ///
                `algorithm_supplied' `preconditioner_supplied'     ///
                `batch_supplied' `stayers_supplied'                ///
                `rust_core_flags' `rust_support_flags' `"`nodisplay'"' ///
                `deletion' `nuisance' `exact_limit'               ///
                `rank_tolerance' `block_tolerance'                ///
                `blocksize_limit' `physical_limit' `"`controlvars'"' ///
                `targetweight_supplied' `rust_frequency_used' `"`cmdline'"' ///
                `preconditioner' `batch_requested'                 ///
                `wallseconds_supplied' `rust_planned_wallseconds' ///
                `"`probeorder'"' `stayers' `original_stayer'    ///
                `hybrid_complete' `rng_requested'                  ///
                `rust_full_cmg_eligible' `tolerance_supplied'      ///
                `"`project'"' `"`projecteffect'"' `"`projectweight'"' `level' ///
                `"`inference'"' `"`inferencemodel'"' `inferencesimulations' ///
                `inferenceseed'
        }
        else if `rust_generic_requested' {
            capture noisily _fevc_rust_generic `depvar'          ///
                `initial_worker' `initial_firm' `rust_deletion'   ///
                `frequency' `target' `touse' `N_scope' `N_complete' ///
                `N_stayers' `N_stayer_rows' `probes' `batch' `seed' ///
                `tolerance' `maxiter' `memory_gib'                ///
                `engine_requested' `backend_supplied' `rng_supplied' ///
                `deletionid_supplied' `engine_supplied'           ///
                `algorithm_supplied' `preconditioner_supplied'     ///
                `batch_supplied' `stayers_supplied'                ///
                `rust_core_flags' `rust_support_flags' `"`nodisplay'"' ///
                `deletion' `nuisance' `exact_limit'               ///
                `rank_tolerance' `block_tolerance'                ///
                `blocksize_limit' `physical_limit' `"`controlvars'"' ///
                `targetweight_supplied' `rust_frequency_used' `"`cmdline'"'
        }
        else {
            capture noisily _fevc_rust_public `depvar' `initial_worker' ///
                `initial_firm' `rust_deletion' `frequency' `target' `touse' ///
                `N_scope' `N_complete' `N_stayers' `N_stayer_rows'      ///
                `probes' `batch' `seed' `tolerance' `maxiter' `memory_gib' ///
                `engine_requested' `backend_supplied' `rng_supplied'    ///
                `deletionid_supplied' `rust_core_flags'                ///
                `rust_support_flags' `nodisplay'
        }
        local rust_public_rc = _rc
        local rust_failure_cmd `"`e(cmd)'"'
        local rust_failure_status `"`e(status)'"'
        local rust_typed_failure =                              ///
            `"`rust_failure_cmd'"' == "fevc" &          ///
            `"`rust_failure_status'"' == "WITHHELD"

        // This outer guard runs after every return from the prepared public
        // lifecycle, including unexpected Stata errors and UserBreak.  The
        // inner path normally releases before posting e(), while this final
        // guard makes any unanticipated exit fail-safe and idempotent.
        capture quietly _fevc_rust_finally
        local rust_finally_rc = _rc
        if `rust_typed_failure' {
            ereturn local backend_requested "`backend_requested'"
            ereturn scalar backend_option_supplied = `backend_supplied'
            ereturn local rng_requested "`rng_requested'"
            ereturn scalar rng_option_supplied = `rng_supplied'
            ereturn local backend_routing_reason ///
                `"`backend_routing_reason'"'
            ereturn scalar backend_fallback = 0
            ereturn local backend_fallback_reason ""
            ereturn local backend_fallback_phase ""
        }
        if `rust_public_rc' {
            if `rust_public_rc' == 1 | !`rust_typed_failure' {
                ereturn clear
            }
            exit `rust_public_rc'
        }
        if `rust_finally_rc' {
            ereturn clear
            quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED" ///
                "The Rust lifecycle completed, but its outer cleanup guard could not certify idle native state."
            ereturn local backend_requested "rust"
            ereturn local backend_selected ""
            ereturn local rng_requested "`rng_requested'"
            ereturn local rng_selected ""
            ereturn scalar backend_option_supplied = 1
            ereturn scalar rng_option_supplied = `rng_supplied'
            exit 498
        }
        ereturn local backend_requested "`backend_requested'"
        ereturn local backend_selected "rust"
        ereturn local backend_routing_reason `"`backend_routing_reason'"'
        ereturn scalar backend_option_supplied = `backend_supplied'
        ereturn local rng_requested "`rng_requested'"
        ereturn scalar rng_option_supplied = `rng_supplied'
        ereturn local stayers "`stayers'"
        ereturn scalar stayers_option_supplied = `stayers_supplied'
        ereturn scalar backend_fallback = 0
        ereturn local backend_fallback_reason ""
        ereturn local backend_fallback_phase ""
        exit 0
    }

    quietly timer off $VCKSS_STAGE_SELECTION_TIMER
    quietly timer list $VCKSS_STAGE_SELECTION_TIMER
    local prep_initial_group_seconds =                      ///
        r(t$VCKSS_STAGE_SELECTION_TIMER) -                  ///
        `prep_mark_validate_seconds'
    quietly timer on $VCKSS_STAGE_SELECTION_TIMER

    local expected_mata_build "vckss-api21-stayer-hybrid"
    capture mata: vckss__api_level()
    local mata_runtime_loaded = (_rc == 0)
    capture mata: assert(vckss__api_level() == 21 &                 ///
        vckss__version() == "0.5.0-alpha.1" &                         ///
        vckss__build_id() == "`expected_mata_build'")
    if _rc {
        if `mata_runtime_loaded' {
            quietly _vckss_post_failure "STALE_MATA_RUNTIME"
            di as error "a different fevc Mata runtime is already loaded; restart Stata or run discard before retrying"
            exit 498
        }
        capture findfile fevc.mata
        if _rc {
            quietly _vckss_post_failure "MATA_RUNTIME_NOT_FOUND"
            di as error "fevc.mata was not found on the Stata adopath"
            exit 601
        }
        quietly do `"`r(fn)'"'
        capture mata: assert(vckss__api_level() == 21 &             ///
            vckss__version() == "0.5.0-alpha.1" &                     ///
            vckss__build_id() == "`expected_mata_build'")
        if _rc {
            quietly _vckss_post_failure "INVALID_MATA_RUNTIME"
            di as error "the loaded fevc Mata runtime does not match this command build"
            exit 498
        }
    }

    // Ordinary binary64 summation cannot distinguish every integer total
    // above 2^53.  Reject such inputs before graph ranking so component mass,
    // observation-unit counts, and e(N_physical) remain literal integers.
    tempname exact_physical_total
    mata: st_numscalar("`exact_physical_total'",                  ///
        vckss__exact_physical_total(                              ///
        st_data(., "`frequency'", "`touse'")))
    if missing(scalar(`exact_physical_total')) {
        quietly _vckss_post_failure "PHYSICAL_TOTAL_LIMIT"
        di as error "literal frequency total exceeds the exact binary64 integer range (2^53)"
        exit 498
    }

    capture mata: assert(vckss_graph__api_level() == 21 &          ///
        vckss_graph__build_id() ==                                 ///
        "vckss-graph-api21-prep-map1-retained")
    if _rc {
        capture findfile fevc_graph.mata
        if _rc {
            quietly _vckss_post_failure "GRAPH_RUNTIME_NOT_FOUND"
            di as error "fevc_graph.mata was not found on the Stata adopath"
            exit 601
        }
        quietly do `"`r(fn)'"'
        capture mata: assert(vckss_graph__api_level() == 21 &      ///
            vckss_graph__build_id() ==                             ///
            "vckss-graph-api21-prep-map1-retained")
        if _rc {
            quietly _vckss_post_failure "INVALID_GRAPH_RUNTIME"
            di as error "the loaded graph runtime does not match this command build"
            exit 498
        }
    }

    quietly timer off $VCKSS_STAGE_SELECTION_TIMER
    quietly timer list $VCKSS_STAGE_SELECTION_TIMER
    local prep_runtime_setup_seconds =                      ///
        r(t$VCKSS_STAGE_SELECTION_TIMER) -                  ///
        `prep_mark_validate_seconds' -                      ///
        `prep_initial_group_seconds'
    quietly timer on $VCKSS_STAGE_SELECTION_TIMER

    /* The complete-case worker and firm maps above are exactly the graph's
       input maps.  The graph returns retained dense maps derived only from
       those Stata-generated numeric codes, preserving Stata's original
       string/numeric ordering oracle without two post-pruning egen passes. */
    local graph_worker `initial_worker'
    local graph_firm `initial_firm'
    tempvar graph_deletion graph_keep id_worker id_firm
    if "`deletion'" == "observation" {
        quietly generate double `graph_deletion' = _n if `touse'
    }
    else if "`deletionid'" != "" {
        quietly egen long `graph_deletion' = group(`deletionid') if `touse'
    }
    else {
        quietly egen long `graph_deletion' = group(`graph_worker' `graph_firm') ///
            if `touse'
    }
    quietly generate byte `graph_keep' = 0
    quietly generate long `id_worker' = .
    quietly generate long `id_firm' = .
    tempname graph_diagnostics
    local graph_status
    local graph_message
    local graph_worker_levels
    local graph_firm_levels
    capture noisily mata: vckss_graph__stata_prune(                ///
        "`graph_worker'", "`graph_firm'", "`frequency'",       ///
        "`graph_deletion'", "`touse'", "`deletion'",          ///
        "`graph_keep'", "`graph_diagnostics'",                 ///
        "graph_status", "graph_message", "`id_worker'",       ///
        "`id_firm'", "graph_worker_levels",                    ///
        "graph_firm_levels")
    if _rc {
        local graph_rc = _rc
        quietly _vckss_post_failure "GRAPH_RUNTIME_FAILED"
        di as error "leave-out graph construction stopped unexpectedly"
        exit `graph_rc'
    }
    if "`graph_status'" != "CONVERGED" {
        local failure_status `graph_status'
        local failure_message `"`graph_message'"'
        quietly _vckss_post_failure "`failure_status'"           ///
            `"`failure_message'"'
        di as error `"`failure_message'"'
        if inlist("`failure_status'", "INVALID_GRAPH_INPUT",       ///
            "CROSS_COORDINATE_MATCH", "UNSUPPORTED_DELETION") exit 198
        if "`failure_status'" == "NO_MOVER_SAMPLE" exit 2000
        exit 498
    }
    quietly replace `touse' = `touse' & `graph_keep'
    quietly count if `touse'
    local N_retained = r(N)
    if `N_retained' == 0 {
        quietly _vckss_post_failure "NO_LEAVEOUT_COMPONENT"
        di as error "no observations remain in the leave-out-connected component"
        exit 498
    }
    tempname retained_physical_total
    mata: st_numscalar("`retained_physical_total'",               ///
        vckss__exact_physical_total(                              ///
        st_data(., "`frequency'", "`touse'")))
    if missing(scalar(`retained_physical_total')) {
        quietly _vckss_post_failure "MATA_RUNTIME_FAILED"
        di as error "retained literal frequency total could not be certified"
        exit 498
    }
    local retained_physical = scalar(`retained_physical_total')
    local N_mover_input = `graph_diagnostics'[1,8]
    local N_initial_component = `graph_diagnostics'[1,9]
    local N_initial_component_dropped = `N_complete' - `N_initial_component'
    local N_mover_dropped = `N_initial_component' - `N_mover_input'
    local N_graph_dropped = `N_mover_input' - `N_retained'
    local N_mover_retained = `N_retained'
    tempvar mover_deletion_tag
    quietly egen byte `mover_deletion_tag' = tag(`graph_deletion') if `touse'
    quietly count if `mover_deletion_tag'
    local N_mover_deletion_units = r(N)

    /* The combined match target is defined from the frozen complete-case
       worker histories, never from rows dropped by the mover graph.  Only
       original one-firm stayers attached to a firm in the final mover sample
       enter, and one-copy histories are ineligible for physical leave-one-out. */
    local N_hybrid_stayers = 0
    local N_hybrid_stayer_rows = 0
    local N_hyb_singleton_drop = 0
    local N_hyb_unattached = 0
    local hybrid_mover_physical = `retained_physical'
    local hybrid_stayer_physical = 0
    local hybrid_total_physical = `retained_physical'
    local hybrid_mover_target_mass = .
    local hybrid_stayer_target_mass = 0
    local hybrid_target_mass = .
    tempvar retained_firm_member stayer_physical_total
    tempvar hybrid_stayer hybrid_touse hybrid_worker hybrid_firm
    tempvar hybrid_stayer_tag
    if "`stayers'" == "both" {
        quietly egen byte `retained_firm_member' = max(`touse')   ///
            if `hybrid_complete', by(`initial_firm')
        quietly egen double `stayer_physical_total' =            ///
            total(`frequency') if `hybrid_complete', by(`initial_worker')
        quietly generate byte `hybrid_stayer' =                  ///
            `original_stayer' & `retained_firm_member' &         ///
            `stayer_physical_total' >= 2 if `hybrid_complete'
        quietly generate byte `hybrid_touse' =                   ///
            `touse' | `hybrid_stayer'
        quietly egen long `hybrid_worker' = group(`initial_worker') ///
            if `hybrid_touse'
        quietly egen long `hybrid_firm' = group(`initial_firm')  ///
            if `hybrid_touse'
        quietly egen byte `hybrid_stayer_tag' =                  ///
            tag(`initial_worker') if `hybrid_stayer'
        quietly count if `hybrid_stayer_tag'
        local N_hybrid_stayers = r(N)
        quietly count if `hybrid_stayer'
        local N_hybrid_stayer_rows = r(N)
        quietly count if `worker_tag' & `original_stayer' &      ///
            `retained_firm_member' & `stayer_physical_total' < 2
        local N_hyb_singleton_drop = r(N)
        quietly count if `worker_tag' & `original_stayer' &      ///
            !`retained_firm_member'
        local N_hyb_unattached = r(N)
        quietly summarize `frequency' if `hybrid_stayer', meanonly
        local hybrid_stayer_physical = r(sum)
        local hybrid_total_physical =                          ///
            `hybrid_mover_physical' + `hybrid_stayer_physical'
        quietly summarize `target' if `touse', meanonly
        local hybrid_mover_target_mass = r(sum)
        quietly summarize `target' if `hybrid_stayer', meanonly
        local hybrid_stayer_target_mass = r(sum)
        local hybrid_target_mass =                             ///
            `hybrid_mover_target_mass' + `hybrid_stayer_target_mass'
    }

    /* Outcome-variance summaries are descriptive display quantities, not
       additional KSS targets.  The target-weighted variance uses the same
       retained target mass as the four quadratic forms.  The regression-
       weighted variance uses literal frequency mass and therefore supports
       the ordinary full-model explained-variance identity below.  A
       nonfinite descriptive moment is left missing rather than withholding
       an otherwise valid KSS calculation. */
    quietly _vckss_descriptive_variances `depvar' `frequency'     ///
        `target' `touse' `retained_physical'
    local target_outcome_variance = r(target)
    local regression_outcome_variance = r(regression)
    local hybrid_target_outcome_variance = `target_outcome_variance'
    local hybrid_regression_variance =                            ///
        `regression_outcome_variance'
    if "`stayers'" == "both" {
        quietly _vckss_descriptive_variances `depvar' `frequency' ///
            `target' `hybrid_touse' `hybrid_total_physical'
        local hybrid_target_outcome_variance = r(target)
        local hybrid_regression_variance = r(regression)
    }

    quietly timer off $VCKSS_STAGE_SELECTION_TIMER
    quietly timer list $VCKSS_STAGE_SELECTION_TIMER
    local prep_graph_boundary_elapsed =                     ///
        r(t$VCKSS_STAGE_SELECTION_TIMER) -                  ///
        `prep_mark_validate_seconds' -                      ///
        `prep_initial_group_seconds' -                      ///
        `prep_runtime_setup_seconds'
    local prep_graph_setup_io_seconds = max(0,              ///
        `prep_graph_boundary_elapsed' -                     ///
        `graph_diagnostics'[1,12] -                         ///
        `graph_diagnostics'[1,19])
    quietly timer on $VCKSS_STAGE_SELECTION_TIMER

    /* Deletion IDs need equality and canonical order, not consecutive values.
       Reuse the graph-stage map after pruning; gaps left by removed units are
       harmless and the compressed constructor independently densifies them. */
    local deletion_id `graph_deletion'
    tempvar hybrid_deletion
    if "`stayers'" == "both" {
        quietly generate double `hybrid_deletion' = `graph_deletion' ///
            if `hybrid_touse'
        quietly summarize `graph_deletion' if `touse', meanonly
        local mover_deletion_max = r(max)
        quietly replace `hybrid_deletion' = `mover_deletion_max' + _n ///
            if `hybrid_stayer'
    }

    local worker_levels = real("`graph_worker_levels'")
    local firm_levels = real("`graph_firm_levels'")
    if missing(`worker_levels') | missing(`firm_levels') |       ///
        `worker_levels' < 1 | `firm_levels' < 2 {
        quietly _vckss_post_failure "INVALID_GRAPH_INPUT"
        di as error "retained worker or firm maps are invalid"
        exit 498
    }
    local control_count : word count `controlvars'
    local mover_worker_levels = `worker_levels'
    local mover_parameters = `worker_levels' + `firm_levels' - 1 + `control_count'
    local parameters = `mover_parameters'
    if "`stayers'" == "both" local parameters =                    ///
        `mover_parameters' + `N_hybrid_stayers'
    quietly timer off $VCKSS_STAGE_SELECTION_TIMER
    quietly timer list $VCKSS_STAGE_SELECTION_TIMER
    local sample_selection_seconds =                          ///
        r(t$VCKSS_STAGE_SELECTION_TIMER)
    local prep_retained_map_seconds =                       ///
        `graph_diagnostics'[1,19] + max(0,                   ///
        `sample_selection_seconds' -                         ///
        `prep_mark_validate_seconds' -                       ///
        `prep_initial_group_seconds' -                       ///
        `prep_runtime_setup_seconds' -                       ///
        `prep_graph_boundary_elapsed')
    local selected_algorithm `algorithm'
    if "`selected_algorithm'" == "auto" {
        if `parameters' <= `exact_limit' local selected_algorithm exact
        else local selected_algorithm jla
    }

    local solve_touse `touse'
    local solve_worker `id_worker'
    local solve_firm `id_firm'
    local solve_deletion `deletion_id'
    local solve_stayer_mask
    if "`selected_algorithm'" == "jla" & "`stayers'" == "both" & ///
        `N_hybrid_stayers' > 0 {
        local solve_touse `hybrid_touse'
        local solve_worker `hybrid_worker'
        local solve_firm `hybrid_firm'
        local solve_deletion `hybrid_deletion'
        local solve_stayer_mask `hybrid_stayer'
        local N_retained = `N_retained' + `N_hybrid_stayer_rows'
        local worker_levels = `mover_worker_levels' + `N_hybrid_stayers'
        scalar `retained_physical_total' = `hybrid_total_physical'
        local retained_physical = `hybrid_total_physical'
    }

    // Construct the canonical order from observed dense identifiers and the
    // statistical row content.  Row order, batching, and solver route cannot
    // change it.  A caller who relabels IDs may obtain a different valid draw;
    // arbitrary relabel invariance is not part of the user-facing RNG contract.
    local semantic_order_seconds = 0
    tempvar semantic_target semantic_rank
    local prep_stata_semantic_ready = 0
    local prep_mata_semantic = (                                  ///
        "`selected_algorithm'" == "jla" &                         ///
        "`deletion'" == "match" & `control_count' == 0 &          ///
        scalar(`retained_physical_total') < 2^53 &                 ///
        "`engine_requested'" != "generic" &                       ///
        ("`stayers'" == "movers" | `N_hybrid_stayers' == 0))
    if "`selected_algorithm'" == "jla" & !`prep_mata_semantic' {
        quietly timer clear $VCKSS_STAGE_SELECTION_TIMER
        quietly timer on $VCKSS_STAGE_SELECTION_TIMER
        quietly generate double `semantic_target' =               ///
            `target'/`frequency' if `solve_touse'
        local semantic_key `solve_worker' `solve_firm'
        if "`deletion'" == "match" local semantic_key             ///
            `semantic_key' `solve_deletion'
        local semantic_key `semantic_key' `semantic_target'       ///
            `depvar' `controlvars'
        if "`probeorder'" != "" local semantic_key                ///
            `semantic_key' `probeorder'
        quietly egen double `semantic_rank' =                     ///
            group(`semantic_key') if `solve_touse'
        local prep_semantic_group_calls =                         ///
            `prep_semantic_group_calls' + 1
        if "`deletion'" == "match" {
            sort `semantic_key' `solve_worker' `solve_firm' `solve_deletion'
        }
        else sort `semantic_key' `solve_worker' `solve_firm'
        local prep_sort_calls = `prep_sort_calls' + 1
        quietly timer off $VCKSS_STAGE_SELECTION_TIMER
        quietly timer list $VCKSS_STAGE_SELECTION_TIMER
        local semantic_order_seconds = r(t$VCKSS_STAGE_SELECTION_TIMER)
        local prep_stata_semantic_ready = 1
    }

    local engine_selected generic
    local fastpath_eligible = 0
    local fastpath_status NOT_APPLICABLE
    local fastpath_message "selected algorithm does not use randomized probes"
    local coefficient_cells = `N_retained'
    local diagnostic_coefficient_cells = .
    local diagnostic_deletion_units = .
    local target_strata = `N_retained'
    local leverage_rng_calls_per_probe = 1
    local target_rng_calls_per_probe = 1
    local leverage_batch = `batch'
    local target_batch = `batch'
    local resource_peak_phase NOT_APPLICABLE
    local resource_status NOT_APPLICABLE
    local resource_message
    local compression_seconds = 0
    local life_mem_before_bytes = .
    tempname scale_prepare_diagnostics resource_components resource_forecasts
    tempname resource_scaling

    if "`selected_algorithm'" == "jla" {
        // Measure the row-resident caller state before constructing any
        // cached compressed arrays.  The resource model reconciles this
        // stage separately from transition, numerical, and restoration use.
        capture program list _fevc_lifecycle_memory
        if _rc {
            capture findfile _fevc_lifecycle.ado
            if _rc {
                quietly _vckss_post_failure "LIFECYCLE_RUNTIME_NOT_FOUND"
                di as error "_fevc_lifecycle.ado was not found on the Stata adopath"
                exit 601
            }
            quietly do `"`r(fn)'"'
        }
        quietly _fevc_lifecycle_memory, stage(selection)
        local life_mem_before_bytes = r(total_alloc_bytes)
        if missing(`life_mem_before_bytes') | `life_mem_before_bytes' <= 0 {
            quietly _vckss_post_failure "RAW_MEMORY_MEASUREMENT_FAILED"
            di as error "Stata did not provide a finite resident-memory measurement for resource admission"
            exit 498
        }

        capture mata: vckss_scale__api_level()
        local scale_runtime_loaded = (_rc == 0)
        capture mata: assert(vckss_scale__api_level() == 6 &       ///
            vckss_scale__build_id() ==                            ///
            "vckss-scale-api6-prep-sem1-mata")
        if _rc {
            if `scale_runtime_loaded' {
                quietly _vckss_post_failure "STALE_SCALE_RUNTIME"
                di as error "a different compressed-design runtime is already loaded; restart Stata or run discard before retrying"
                exit 498
            }
            capture findfile fevc_scale.mata
            if _rc {
                quietly _vckss_post_failure "SCALE_RUNTIME_NOT_FOUND"
                di as error "fevc_scale.mata was not found on the Stata adopath"
                exit 601
            }
            quietly do `"`r(fn)'"'
            capture mata: assert(vckss_scale__api_level() == 6 &   ///
                vckss_scale__build_id() ==                        ///
                "vckss-scale-api6-prep-sem1-mata")
            if _rc {
                quietly _vckss_post_failure "INVALID_SCALE_RUNTIME"
                di as error "the installed compressed-design runtime is incompatible with this command"
                exit 498
            }
        }

        // Known-ineligible and explicitly generic calls never construct the
        // compressed state.  Eligible auto/explicit calls build the canonical
        // cell/unit/stratum representation exactly once and reuse it through
        // preserve/clear and all numerical work.
        if "`deletion'" != "match" {
            local fastpath_status FASTPATH_OBSERVATION_DELETION
            local fastpath_message "compressed leverage sums are not valid for observation deletion"
        }
        else if `control_count' > 0 {
            local fastpath_status FASTPATH_CONTROLS
            local fastpath_message "the experimental compressed engine does not support nuisance controls"
        }
        else if "`stayers'" == "both" & `N_hybrid_stayers' > 0 {
            local fastpath_status FASTPATH_MIXED_DELETION
            local fastpath_message "the mover/stayer hybrid requires the generic mixed-deletion engine"
        }
        else if scalar(`retained_physical_total') >= 2^53 {
            local fastpath_status BINOMIAL_CONTRACT_UNSUPPORTED
            local fastpath_message "compressed binomial trials require total physical mass below 2^53"
        }
        else if "`engine_requested'" == "generic" {
            // Graph selection has already certified that every match lies in
            // one coefficient coordinate.  This is sufficient to retain the
            // exact semantic-atom RNG path without building compressed arrays.
            local fastpath_eligible = 1
            local fastpath_status FASTPATH_BYPASSED
            local fastpath_message "caller explicitly selected the generic engine"
        }
        else {
            quietly _fevc_lifecycle_timer_ids
            local compression_timer = r(transition_timer)
            local compression_aux_timer_1 = r(work_timer)
            local compression_aux_timer_2 = r(restore_timer)
            foreach compression_timer_id in `compression_timer'   ///
                `compression_aux_timer_1' `compression_aux_timer_2' {
                quietly timer clear `compression_timer_id'
            }
            capture quietly _fevc_lifecycle_phase,              ///
                phase(compression_transition)
            if _rc {
                quietly _fevc_lifecycle_release_timers,         ///
                    timers(`compression_timer'                    ///
                        `compression_aux_timer_1'                  ///
                        `compression_aux_timer_2')
                quietly _vckss_post_failure "PHASE_MARKER_FAILED"
                di as error "the compression-transition phase marker could not be written"
                exit 498
            }
            quietly timer on `compression_timer'
            local prep_compression_import_columns =               ///
                cond(`prep_mata_semantic',                        ///
                    6 + ("`probeorder'" != ""), 7)
            local prep_compression_import_rows = `N_retained'
            capture noisily mata: vckss_srt__prepare(             ///
                "`depvar'", "`id_worker'", "`id_firm'",       ///
                "`deletion_id'", "`frequency'", "`target'",  ///
                "`semantic_rank'", "`probeorder'",              ///
                `prep_mata_semantic', "`touse'",                 ///
                `rank_tolerance',                                 ///
                "`scale_prepare_diagnostics'",                   ///
                "scale_prepare_status", "scale_prepare_message")
            local scale_prepare_rc = _rc
            quietly timer off `compression_timer'
            quietly timer list `compression_timer'
            local compression_seconds = r(t`compression_timer')
            local scale_semantic_seconds = 0
            if !`scale_prepare_rc' {
                capture confirm matrix `scale_prepare_diagnostics'
                if !_rc &                                        ///
                    rowsof(`scale_prepare_diagnostics') == 1 &    ///
                    colsof(`scale_prepare_diagnostics') >= 16 &   ///
                    `scale_prepare_diagnostics'[1,16] < . {
                    local scale_semantic_seconds =                ///
                        `scale_prepare_diagnostics'[1,16]
                }
            }
            local semantic_order_seconds =                        ///
                `semantic_order_seconds' + `scale_semantic_seconds'
            local compression_seconds = max(0,                    ///
                `compression_seconds' - `scale_semantic_seconds')
            quietly _fevc_lifecycle_release_timers,             ///
                timers(`compression_timer'                        ///
                    `compression_aux_timer_1'                      ///
                    `compression_aux_timer_2')
            if !`scale_prepare_rc' &                              ///
                "`scale_prepare_status'" == "PREPARED" {
                local diagnostic_coefficient_cells =              ///
                    `scale_prepare_diagnostics'[1,5]
                local diagnostic_deletion_units =                 ///
                    `scale_prepare_diagnostics'[1,6]
                local target_strata = `scale_prepare_diagnostics'[1,7]
                local leverage_rng_calls_per_probe =              ///
                    `scale_prepare_diagnostics'[1,14]
                local target_rng_calls_per_probe =                ///
                    `scale_prepare_diagnostics'[1,15]
                local coefficient_cells =                        ///
                    `diagnostic_coefficient_cells'
                local fastpath_eligible = 1
                local fastpath_status ELIGIBLE
                local fastpath_message "no-control match design has exact cell, deletion-unit, and target-stratum compression"
            }
            else {
                capture mata: vckss_scale_runtime__reset()
                if `scale_prepare_rc' {
                    local fastpath_status SCALE_PREPARATION_FAILED
                    local fastpath_message "compressed-state construction stopped unexpectedly"
                }
                else {
                    local fastpath_status `scale_prepare_status'
                    local fastpath_message `"`scale_prepare_message'"'
                }
            }
        }

        if "`engine_requested'" == "compressed" & !`fastpath_eligible' {
            quietly _vckss_post_failure "`fastpath_status'"
            ereturn scalar N_retained = `N_retained'
            ereturn scalar N_physical = scalar(`retained_physical_total')
            ereturn scalar coefficient_cells = `coefficient_cells'
            ereturn scalar deletion_units = `diagnostic_deletion_units'
            ereturn scalar target_strata = `target_strata'
            ereturn local engine_requested "`engine_requested'"
            ereturn local engine_selected "WITHHELD"
            ereturn local fastpath_status "`fastpath_status'"
            ereturn local fastpath_message `"`fastpath_message'"'
            di as error `"`fastpath_message'"'
            exit 498
        }
        if "`engine_requested'" == "compressed" |                 ///
            ("`engine_requested'" == "auto" & `fastpath_eligible') {
            local engine_selected compressed
        }
        if "`engine_selected'" == "generic" {
            capture mata: vckss_scale_runtime__reset()
            if _rc {
                quietly _vckss_post_failure "SCALE_STATE_RELEASE_FAILED"
                di as error "unused compressed command state could not be released"
                exit 498
            }
            // An eligible auto call can reach the generic route only after
            // compressed preparation declines the design.  Construct the
            // unchanged Stata semantic oracle before invoking that route.
            if !`prep_stata_semantic_ready' {
                quietly timer clear $VCKSS_STAGE_SELECTION_TIMER
                quietly timer on $VCKSS_STAGE_SELECTION_TIMER
                quietly generate double `semantic_target' =       ///
                    `target'/`frequency' if `solve_touse'
                local semantic_key `solve_worker' `solve_firm'    ///
                    `solve_deletion' `semantic_target' `depvar'
                if "`probeorder'" != "" local semantic_key       ///
                    `semantic_key' `probeorder'
                quietly egen double `semantic_rank' =             ///
                    group(`semantic_key') if `solve_touse'
                local prep_semantic_group_calls =                 ///
                    `prep_semantic_group_calls' + 1
                sort `semantic_key' `solve_worker' `solve_firm'   ///
                    `solve_deletion'
                local prep_sort_calls = `prep_sort_calls' + 1
                quietly timer off $VCKSS_STAGE_SELECTION_TIMER
                quietly timer list $VCKSS_STAGE_SELECTION_TIMER
                local semantic_order_seconds =                    ///
                    `semantic_order_seconds' +                    ///
                    r(t$VCKSS_STAGE_SELECTION_TIMER)
                local prep_stata_semantic_ready = 1
            }
        }

        if "`engine_selected'" == "compressed" &                 ///
            "`batch_requested'" == "auto" {
            local leverage_batch = 8
            local target_batch = 8
            local scale_batch_budget = floor(`memory_gib'*1024^3*.25)
            foreach candidate in 16 32 64 {
                local leverage_candidate_bytes = 8*`candidate'*(   ///
                    6*`diagnostic_deletion_units'+                ///
                    4*`diagnostic_coefficient_cells'+3*`parameters')
                if `candidate' <= `probes' & `candidate' <= 32 &   ///
                    `leverage_candidate_bytes' <= `scale_batch_budget' {
                    local leverage_batch = `candidate'
                }
                local target_candidate_bytes = 8*`candidate'*(     ///
                    2*`target_strata'+                             ///
                    8*`diagnostic_coefficient_cells'+6*`parameters')
                if `candidate' <= `probes' & `candidate' <= 32 &   ///
                    `target_candidate_bytes' <= `scale_batch_budget' {
                    local target_batch = `candidate'
                }
            }
            local batch = `leverage_batch'
            local batch_routing_reason "separate largest cell/unit/stratum widths within the compressed 25% scratch reservation"
        }
        else {
            local leverage_batch = `batch'
            local target_batch = `batch'
        }
        if "`engine_selected'" == "generic" &                    ///
            "`batch_requested'" == "auto" {
            local generic_batch_budget = floor(`memory_gib'*1024^3*.35)
            local generic_physical_column =                       ///
                8*scalar(`retained_physical_total')
            local generic_column_bytes =                         ///
                8*(14*`N_retained'+12*`parameters')+              ///
                `generic_physical_column'
            local generic_processor_cap = cond(`active_processors'>=8,64,32)
            if `N_retained' >= 10000 {
                foreach candidate in 16 32 64 {
                    if `candidate' <= `generic_processor_cap' &   ///
                        `candidate' <= `probes' &                 ///
                        `candidate'*`generic_column_bytes' <=     ///
                        `generic_batch_budget' local batch = `candidate'
                }
            }
            local leverage_batch = `batch'
            local target_batch = `batch'
        }

        capture mata: vckss_resource__api_level()
        local resource_runtime_loaded = (_rc == 0)
        capture mata: assert(vckss_resource__api_level() == 10 &   ///
            vckss_resource__build_id() ==                         ///
            "vckss-resource-api10-fe-buf1-buffered")
        if _rc {
            if `resource_runtime_loaded' {
                quietly _vckss_post_failure "STALE_RESOURCE_RUNTIME"
                di as error "a different resource-admission runtime is already loaded; restart Stata or run discard before retrying"
                exit 498
            }
            capture findfile fevc_resource.mata
            if _rc {
                quietly _vckss_post_failure "RESOURCE_RUNTIME_NOT_FOUND"
                di as error "fevc_resource.mata was not found on the Stata adopath"
                exit 601
            }
            quietly do `"`r(fn)'"'
            capture mata: assert(vckss_resource__api_level() == 10 & ///
                vckss_resource__build_id() ==                     ///
                "vckss-resource-api10-fe-buf1-buffered")
            if _rc {
                quietly _vckss_post_failure "INVALID_RESOURCE_RUNTIME"
                di as error "the installed resource-admission runtime is incompatible with this command"
                exit 498
            }
        }

        local resource_cells = `coefficient_cells'
        local resource_units = `diagnostic_deletion_units'
        if missing(`resource_units') local resource_units = `N_retained'
        local resource_strata = `target_strata'
        if missing(`resource_strata') local resource_strata = `N_retained'
        capture noisily mata: vckss_resource__stata_model(         ///
            `N_retained', `retained_physical',                    ///
            `resource_cells', `resource_units', `resource_strata', ///
            `worker_levels', `firm_levels', `parameters',         ///
            `probes', `leverage_batch', `target_batch',           ///
            `leverage_rng_calls_per_probe',                       ///
            `target_rng_calls_per_probe',                         ///
            vckss_resource__rng_call_upper(),                     ///
            `life_mem_before_bytes',                              ///
            `memory_gib'*1024^3, `wallseconds',                   ///
            "`resource_components'", "`resource_forecasts'",   ///
            "`resource_scaling'", "resource_model_status",     ///
            "resource_model_message", "compressed_resource_status", ///
            "compressed_resource_message", "generic_resource_status", ///
            "generic_resource_message", "compressed_peak_phase", ///
            "generic_peak_phase")
        if _rc | "`resource_model_status'" != "MODELED" {
            quietly _vckss_post_failure "RESOURCE_MODEL_FAILED"
            di as error "resource admission could not be constructed before probes"
            exit 498
        }
        local resource_row = cond("`engine_selected'" == "compressed",1,2)
        if `resource_row' == 1 {
            local resource_status `compressed_resource_status'
            local resource_message `"`compressed_resource_message'"'
            local resource_peak_phase `compressed_peak_phase'
        }
        else {
            local resource_status `generic_resource_status'
            local resource_message `"`generic_resource_message'"'
            local resource_peak_phase `generic_peak_phase'
        }
        local resource_non_solver_bytes =                         ///
            `resource_components'[`resource_row',2]+              ///
            `resource_components'[`resource_row',3]+              ///
            `resource_components'[`resource_row',4]+              ///
            `resource_components'[`resource_row',6]+              ///
            `resource_components'[`resource_row',8]+              ///
            `resource_components'[`resource_row',9]+              ///
            `resource_components'[`resource_row',11]
        if `resource_row' == 2 local resource_non_solver_bytes =  ///
            `resource_non_solver_bytes'+                          ///
            `resource_components'[`resource_row',1]
        local resource_selection_peak =                          ///
            `resource_forecasts'[`resource_row',1]
        local resource_transition_peak =                         ///
            `resource_forecasts'[`resource_row',2]
        if `resource_row' == 1 local resource_non_solver_bytes = ///
            max(`resource_non_solver_bytes',                    ///
                `resource_transition_peak'+                     ///
                `resource_components'[`resource_row',6]+        ///
                `resource_components'[`resource_row',8])
        local resource_restoration_peak =                        ///
            `resource_forecasts'[`resource_row',4]
        local resource_wall_forecast =                           ///
            `resource_forecasts'[`resource_row',8]
        local resource_hard_memory =                             ///
            `resource_forecasts'[`resource_row',11]
        local resource_hard_wall =                               ///
            `resource_forecasts'[`resource_row',12]
        // The model's provisional solver component is planning evidence,
        // not an admission decision.  Before routing, reject only direct
        // allocations that are unavoidable for every solver route, including
        // the minimum FE/base design.  The solver then reconciles the selected
        // DIAGONAL or CMG allocation against the remaining memory before
        // estimator RNG begins.
        local resource_min_solver_rows = `N_retained'
        if `resource_row' == 1 local resource_min_solver_rows =   ///
            `resource_cells'
        local resource_min_solver_bytes = 8*(                    ///
            8*`resource_min_solver_rows'+                        ///
            5*(`worker_levels'+`firm_levels'))
        local resource_min_numerical_peak =                      ///
            `resource_non_solver_bytes'+`resource_min_solver_bytes'
        local resource_unavoidable_peak = max(                   ///
            `resource_selection_peak',                           ///
            `resource_transition_peak',                          ///
            `resource_restoration_peak',                         ///
            `resource_min_numerical_peak')
        local resource_unavoidable_phase numerical
        if `resource_unavoidable_peak' ==                        ///
            `resource_selection_peak' local resource_unavoidable_phase selection
        else if `resource_unavoidable_peak' ==                   ///
            `resource_transition_peak' local resource_unavoidable_phase transition
        else if `resource_unavoidable_peak' ==                   ///
            `resource_restoration_peak' local resource_unavoidable_phase restoration
        if `resource_unavoidable_peak' > `resource_hard_memory' {
            if `resource_row' == 1 local resource_status         ///
                RESOURCE_ADMISSION_FAILED
            else local resource_status                           ///
                GENERIC_RESOURCE_ADMISSION_FAILED
            local resource_message                               ///
                "unavoidable non-solver direct allocation exceeds or exhausts the declared memory envelope"
            local resource_peak_phase `resource_unavoidable_phase'
            quietly _vckss_post_failure "`resource_status'"
            ereturn scalar N_retained = `N_retained'
            ereturn scalar N_physical = scalar(`retained_physical_total')
            ereturn scalar coefficient_cells = `coefficient_cells'
            ereturn scalar deletion_units = `resource_units'
            ereturn scalar target_strata = `resource_strata'
            ereturn scalar resource_peak_bytes =                 ///
                `resource_unavoidable_peak'
            ereturn scalar resource_mem_admit_bytes =            ///
                ceil(1.30*`resource_unavoidable_peak')
            ereturn scalar resource_wall_upper_seconds =          ///
                `resource_forecasts'[`resource_row',8]
            ereturn scalar resource_wall_admit_seconds =          ///
                `resource_forecasts'[`resource_row',10]
            ereturn scalar resource_hard_mem_bytes =              ///
                `resource_forecasts'[`resource_row',11]
            ereturn scalar resource_hard_wall_seconds =           ///
                `resource_forecasts'[`resource_row',12]
            ereturn local engine_requested "`engine_requested'"
            ereturn local engine_selected "`engine_selected'"
            ereturn local resource_status "`resource_status'"
            ereturn local resource_message `"`resource_message'"'
            ereturn local resource_peak_phase "`resource_peak_phase'"
            di as error `"`resource_message'"'
            exit 498
        }
    }
    else if "`engine_requested'" == "compressed" {
        quietly _vckss_post_failure "FASTPATH_REQUIRES_JLA"
        ereturn local engine_requested "`engine_requested'"
        ereturn local engine_selected "WITHHELD"
        ereturn local fastpath_status "FASTPATH_REQUIRES_JLA"
        di as error "engine(compressed) requires algorithm(jla)"
        exit 498
    }

    // Forecast the largest estimator scratch family conservatively as
    // fourteen retained-row vectors, twelve coefficient vectors, and one
    // literal-physical-mass vector per simultaneous probe. The percentage
    // and processor thresholds choose a practical automatic width. They do
    // not replace the complete direct-peak allocation check below.
    local batch_memory_budget_bytes = floor(`memory_gib'*1024^3*.35)
    local batch_physical_column_bytes =                            ///
        8*scalar(`retained_physical_total')
    local batch_column_forecast_bytes = ///
        8*(14*`N_retained'+12*`parameters')+ ///
        `batch_physical_column_bytes'
    if "`batch_requested'" == "auto" & "`selected_algorithm'" == "jla" & ///
        "`engine_selected'" == "generic" {
        local batch_processor_cap = cond(`active_processors'>=8,64,32)
        if `N_retained' >= 10000 {
            foreach candidate in 16 32 64 {
                if `candidate' <= `batch_processor_cap' & ///
                    `candidate' <= `probes' & ///
                    `candidate'*`batch_column_forecast_bytes' <= ///
                    `batch_memory_budget_bytes' local batch = `candidate'
            }
            local batch_routing_reason ///
                "largest evidence-backed width within probe, processor, and 35% memory gates"
        }
    }
    if "`selected_algorithm'" == "jla" {
        if "`engine_selected'" == "compressed" {
            local batch_scratch_forecast_bytes =                  ///
                `resource_components'[1,6]
            local batch_memory_budget_bytes =                     ///
                floor(`memory_gib'*1024^3*.25)
            local batch_column_forecast_bytes =                   ///
                `batch_scratch_forecast_bytes'/max(1,`batch')
        }
        else local batch_scratch_forecast_bytes =                 ///
            `batch'*`batch_column_forecast_bytes'
    }
    else {
        local batch_scratch_forecast_bytes = 0
        local batch_routing_reason "exact algorithm does not consume probe batches"
    }

    // The memory envelope is a fail-closed direct-allocation contract.
    // Percentage batch budgets select a practical automatic width. They are
    // heuristics, not independent rejection gates; the complete direct-peak
    // resource model below is the authoritative allocation check.

    if "`selected_algorithm'" == "jla" &                         ///
        "`engine_selected'" == "generic" {
        if scalar(`retained_physical_total') > `physical_limit' {
            quietly _vckss_post_failure "PHYSICAL_COPY_LIMIT"
            di as error "JLA physical copies exceed physical_limit()"
            exit 498
        }
    }
    if "`selected_algorithm'" != "jla" & `control_count' > 0 {
        // Exact controlled calculations use the same observed-ID row order.
        // probeorder() is an optional tie-breaker, never a uniqueness gate.
        tempvar semantic_target semantic_rank
        quietly generate double `semantic_target' = ///
            `target'/`frequency' if `touse'
        local semantic_key `id_worker' `id_firm'
        if "`deletion'" == "match" local semantic_key ///
            `semantic_key' `deletion_id'
        local semantic_key `semantic_key' `semantic_target' ///
            `depvar' `controlvars'
        if "`probeorder'" != "" local semantic_key ///
            `semantic_key' `probeorder'
        quietly egen double `semantic_rank' = group(`semantic_key') if `touse'
        local prep_semantic_group_calls =                         ///
            `prep_semantic_group_calls' + 1
        if "`deletion'" == "match" {
            sort `semantic_key' `id_worker' `id_firm' `deletion_id'
        }
        else sort `semantic_key' `id_worker' `id_firm'
        local prep_sort_calls = `prep_sort_calls' + 1
    }
    else if "`selected_algorithm'" != "jla" & "`deletion'" == "match" {
        sort `id_worker' `id_firm' `deletion_id' `depvar' ///
            `frequency' `target'
        local prep_sort_calls = `prep_sort_calls' + 1
    }
    else if "`selected_algorithm'" != "jla" {
        sort `id_worker' `id_firm' `depvar' `frequency' `target'
        local prep_sort_calls = `prep_sort_calls' + 1
    }
    tempname raw_results diagnostics solver_rhs_diagnostics route_diagnostics
    tempname inference_V_primitive inference_V inference_highrank
    tempname inference_q1 projection_b projection_V projection_V_naive
    tempname inference_q1_status
    tempname projection_results inference_diagnostics
    tempname decomposition
    tempname hybrid_raw_results hybrid_diagnostics
    tempname hybrid_correction_source hybrid_decomposition
    tempname hybrid_plugin hybrid_correction hybrid_corrected hybrid_kss
    tempname mover_plugin mover_correction mover_kss
    tempname hybrid_sample_accounting
    tempname mover_raw_results mover_diagnostics
    tempname pilot_diagnostics scale_receipt
    tempname plugin correction
    tempname corrected kss_return mcse prep_profile rhs_profile work_counters
    tempname prep_boundary_profile prep_boundary_counts
    tempname fe_buffer_profile
    local mata_status
    local mata_message
    local hybrid_mata_status
    local hybrid_mata_message
    local selected_preconditioner NOT_APPLICABLE
    local routing_reason EXACT_ALGORITHM
    local fallback_status NOT_APPLICABLE
    local fallback_message
    local pilot_status
    local pilot_failure_reason
    local rng_contract NOT_APPLICABLE
    local rng_implementation NOT_APPLICABLE
    local rng_call_shape NOT_APPLICABLE
    local rng_runtime `"`c(stata_version)'"'
    local rng_leverage_domain NOT_APPLICABLE
    local rng_target_domain NOT_APPLICABLE
    local rng_leverage_probe_first = .
    local rng_leverage_probe_last = .
    local rng_target_probe_first = .
    local rng_target_probe_last = .
    local life_transition_seconds = 0
    local life_work_seconds = 0
    local life_restore_seconds = 0
    local life_sample_restored = 1
    local life_preserve_forced_disk = 0
    local life_mem_cleared_bytes = .
    local life_mem_work_bytes = .
    local life_mem_restored_bytes = .
    local life_method RAW_RESIDENT
    local mata_call_rc = 0
    local resource_receipt_rc = 0
    local resource_gate_applied NOT_APPLIED
    if "`selected_algorithm'" == "exact" {
        capture noisily mata: vckss__stata_exact(                  ///
            "`depvar'", "`id_worker'", "`id_firm'",             ///
            `"`controlvars'"', "`frequency'", "`target'",      ///
            "`deletion_id'", "`touse'", "`deletion'",         ///
            "`nuisance'", `rank_tolerance', `block_tolerance',  ///
            `exact_limit', `blocksize_limit', "`raw_results'",  ///
            "mata_status", "mata_message", "`diagnostics'")
    }
    else {
        local rng_call_shape                            ///
            vector-parameter-or-scalar-chunk-canonical-atoms-v2
        capture mata: vckss_rng__api_level()
        local rng_runtime_loaded = (_rc == 0)
        capture mata: assert(vckss_rng__api_level() == 4 &         ///
            vckss_rng__build_id() ==                              ///
            "vckss-rng-numeric-ranks-v4")
        if _rc {
            if `rng_runtime_loaded' {
                quietly _vckss_post_failure "STALE_RNG_RUNTIME"
                di as error "a different KSS RNG runtime is already loaded; restart Stata or run discard before retrying"
                exit 498
            }
            capture findfile fevc_rng.mata
            if _rc {
                quietly _vckss_post_failure "RNG_RUNTIME_NOT_FOUND"
                di as error "fevc_rng.mata was not found on the Stata adopath"
                exit 601
            }
            quietly do `"`r(fn)'"'
            capture mata: assert(vckss_rng__api_level() == 4 &     ///
                vckss_rng__build_id() ==                          ///
                "vckss-rng-numeric-ranks-v4")
            if _rc {
                quietly _vckss_post_failure "INVALID_RNG_RUNTIME"
                di as error "the installed RNG runtime is incompatible with this command"
                exit 498
            }
        }
        capture mata: assert(vckss_rng__production_contract() != "")
        if _rc {
            quietly _vckss_post_failure "RNG_RUNTIME_UNREGISTERED"
            ereturn local rng_runtime `"`c(stata_version)'"'
            di as error "the current Stata runtime has no registered KSS probe contract"
            exit 498
        }
        capture mata: vckss_cmg__api_level()
        local cmg_runtime_loaded = (_rc == 0)
        local expected_cmg_design ///
            "gpl-cmg-mata-degree3-hybrid-v8-vckss-component"
        capture mata: assert(vckss_cmg__api_level() == 8 &        ///
            vckss_cmg__numeric_mode() == "off" &                 ///
            vckss_cmg__design_label() == "`expected_cmg_design'")
        if _rc {
            if `cmg_runtime_loaded' {
                quietly _vckss_post_failure "STALE_CMG_RUNTIME"
                di as error "a different CMG runtime is already loaded; restart Stata or run discard before retrying"
                exit 498
            }
            capture findfile fevc_cmg.mata
            if _rc {
                quietly _vckss_post_failure "CMG_RUNTIME_NOT_FOUND"
                di as error "fevc_cmg.mata was not found on the Stata adopath"
                exit 601
            }
            quietly do `"`r(fn)'"'
            capture mata: assert(vckss_cmg__api_level() == 8 &    ///
                vckss_cmg__numeric_mode() == "off" &             ///
                vckss_cmg__design_label() ==                     ///
                "`expected_cmg_design'")
            if _rc {
                quietly _vckss_post_failure "INVALID_CMG_RUNTIME"
                di as error "the installed CMG runtime is incompatible with this command"
                exit 498
            }
        }
        capture mata: vckss_solver__api_level()
        local solver_runtime_loaded = (_rc == 0)
        capture mata: assert(vckss_solver__api_level() == 26 &     ///
            vckss_solver__route_api() == 1 &                      ///
            vckss_solver__build_id() ==                           ///
            "vckss-solver-api26-gpl-mata-cmg")
        if _rc {
            if `solver_runtime_loaded' {
                quietly _vckss_post_failure "STALE_SOLVER_RUNTIME"
                di as error "a different KSS solver adapter is already loaded; restart Stata or run discard before retrying"
                exit 498
            }
            capture findfile fevc_solver.mata
            if _rc {
                quietly _vckss_post_failure "SOLVER_RUNTIME_NOT_FOUND"
                di as error "fevc_solver.mata was not found on the Stata adopath"
                exit 601
            }
            quietly do `"`r(fn)'"'
            capture mata: assert(vckss_solver__api_level() == 26 & ///
                vckss_solver__route_api() == 1 &                  ///
                vckss_solver__build_id() ==                       ///
                "vckss-solver-api26-gpl-mata-cmg")
            if _rc {
                quietly _vckss_post_failure "INVALID_SOLVER_RUNTIME"
                di as error "the installed KSS solver adapter is incompatible with this command"
                exit 498
            }
        }
        if "`engine_selected'" == "compressed" {
            capture mata: vckss_scale_engine__api_level()
            local scale_engine_loaded = (_rc == 0)
            capture mata: assert(                                 ///
                vckss_scale_engine__api_level() == 4 &            ///
                vckss_scale_engine__build_id() ==                 ///
                "vckss-scale-engine-api4-fe-buf1-buffered")
            if _rc {
                if `scale_engine_loaded' {
                    quietly _vckss_post_failure "STALE_SCALE_ENGINE"
                    di as error "a different compressed estimator runtime is already loaded; restart Stata or run discard before retrying"
                    exit 498
                }
                capture findfile fevc_scale_engine.mata
                if _rc {
                    quietly _vckss_post_failure "SCALE_ENGINE_NOT_FOUND"
                    di as error "fevc_scale_engine.mata was not found on the Stata adopath"
                    exit 601
                }
                quietly do `"`r(fn)'"'
                capture mata: assert(                             ///
                    vckss_scale_engine__api_level() == 4 &        ///
                    vckss_scale_engine__build_id() ==             ///
                    "vckss-scale-engine-api4-fe-buf1-buffered")
                if _rc {
                    quietly _vckss_post_failure "INVALID_SCALE_ENGINE"
                    di as error "the compressed estimator runtime is incompatible with this command"
                    exit 498
                }
            }

            capture mata: vckss_scale_runtime__api_level()
            local scale_bridge_loaded = (_rc == 0)
            capture mata: assert(                                 ///
                vckss_scale_runtime__api_level() == 3 &           ///
                vckss_scale_runtime__build_id() ==                ///
                "vckss-scale-runtime-api3-fe-buf1-buffered")
            if _rc {
                if `scale_bridge_loaded' {
                    quietly _vckss_post_failure "STALE_SCALE_BRIDGE"
                    di as error "a different compressed lifecycle bridge is already loaded; restart Stata or run discard before retrying"
                    exit 498
                }
                capture findfile fevc_scale_runtime.mata
                if _rc {
                    quietly _vckss_post_failure "SCALE_BRIDGE_NOT_FOUND"
                    di as error "fevc_scale_runtime.mata was not found on the Stata adopath"
                    exit 601
                }
                quietly do `"`r(fn)'"'
                capture mata: assert(                             ///
                    vckss_scale_runtime__api_level() == 3 &       ///
                    vckss_scale_runtime__build_id() ==            ///
                    "vckss-scale-runtime-api3-fe-buf1-buffered")
                if _rc {
                    quietly _vckss_post_failure "INVALID_SCALE_BRIDGE"
                    di as error "the compressed lifecycle bridge is incompatible with this command"
                    exit 498
                }
            }

            quietly _fevc_lifecycle_timer_ids
            local transition_timer = r(transition_timer)
            local work_timer = r(work_timer)
            local restore_timer = r(restore_timer)
            foreach lifecycle_timer in `transition_timer' `work_timer' ///
                `restore_timer' {
                quietly timer clear `lifecycle_timer'
            }
            quietly count if `touse'
            local lifecycle_sample_N = r(N)
            quietly _datasignature `touse', nonames
            local lifecycle_sample_signature `"`r(datasignature)'"'

            capture mata: assert(                                 ///
                vckss_scale_runtime__status() == "PREPARED")
            if _rc {
                capture mata: vckss_scale_runtime__reset()
                quietly _fevc_lifecycle_release_timers,         ///
                    timers(`transition_timer' `work_timer' `restore_timer')
                quietly _vckss_post_failure "FASTPATH_STATE_UNAVAILABLE"
                di as error "the canonical compressed command state is unavailable"
                exit 498
            }

            local caller_max_preservemem = c(max_preservemem)
            local old_preservemem_text : display %21.0f ///
                `caller_max_preservemem'
            local old_preservemem_text = strtrim("`old_preservemem_text'")
            local transition_rc = 0
            local preserve_active = 0
            quietly timer on `transition_timer'
            capture quietly set max_preservemem 0
            if _rc local transition_rc = _rc
            if !`transition_rc' {
                capture quietly preserve
                if _rc local transition_rc = _rc
                else local preserve_active = 1
            }
            capture quietly set max_preservemem `old_preservemem_text'
            if _rc & !`transition_rc' local transition_rc = _rc
            if !`transition_rc' {
                capture quietly clear
                if _rc local transition_rc = _rc
            }
            quietly timer off `transition_timer'
            quietly timer list `transition_timer'
            local life_transition_seconds = r(t`transition_timer')
            local life_preserve_forced_disk = 1
            local life_method PRESERVE_DISK
            if `transition_rc' {
                capture mata: vckss_scale_runtime__reset()
                if `preserve_active' capture quietly restore
                quietly _fevc_lifecycle_release_timers,         ///
                    timers(`transition_timer' `work_timer' `restore_timer')
                quietly _vckss_post_failure "DATA_LIFECYCLE_FAILED"
                di as error "disk-backed Stata preserve/clear transition failed"
                exit `transition_rc'
            }

            quietly _fevc_lifecycle_memory, stage(cleared)
            local life_mem_cleared_bytes = r(total_alloc_bytes)
            capture quietly _fevc_lifecycle_phase, phase(numerical)
            local numerical_marker_rc = _rc
            if `numerical_marker_rc' {
                capture mata: vckss_scale_runtime__reset()
                capture quietly clear
                capture quietly restore
                quietly _fevc_lifecycle_release_timers,         ///
                    timers(`transition_timer' `work_timer' `restore_timer')
                quietly _vckss_post_failure "PHASE_MARKER_FAILED"
                di as error "the numerical phase marker could not be written"
                exit 498
            }
            capture mata: assert(vckss_solver__resource_config(  ///
                "compressed", `resource_non_solver_bytes',       ///
                `resource_selection_peak',                       ///
                `resource_transition_peak',                      ///
                `resource_restoration_peak',                     ///
                `resource_wall_forecast',                        ///
                `resource_hard_memory', `resource_hard_wall') == 0)
            local resource_gate_config_rc = _rc
            if `resource_gate_config_rc' {
                capture mata: vckss_solver__resource_clear()
                capture mata: vckss_scale_runtime__reset()
                capture quietly clear
                capture quietly restore
                quietly _fevc_lifecycle_release_timers,         ///
                    timers(`transition_timer' `work_timer' `restore_timer')
                quietly _vckss_post_failure                     ///
                    "RESOURCE_GATE_CONFIGURATION_FAILED"
                di as error "the final routed resource gate could not be configured"
                exit 498
            }
            quietly timer on `work_timer'
            capture noisily mata: vckss_scale_runtime__stata_run(   ///
                `probes', `leverage_batch', `target_batch',         ///
                `seed', `tolerance', `maxiter', `rank_tolerance',   ///
                `block_tolerance', `blocksize_limit',               ///
                "`preconditioner'", `memory_gib'*1024^3,           ///
                "`raw_results'", "`diagnostics'",                ///
                "`solver_rhs_diagnostics'",                       ///
                "`route_diagnostics'", "`pilot_diagnostics'",   ///
                "`scale_receipt'", "mata_status",               ///
                "mata_message", "selected_preconditioner",      ///
                "routing_reason", "fallback_status",            ///
                "fallback_message", "pilot_status",             ///
                "pilot_failure_reason", "rng_contract",         ///
                "rng_implementation", "rng_runtime")
            local mata_call_rc = _rc
            quietly timer off `work_timer'
            quietly timer list `work_timer'
            local life_work_seconds = r(t`work_timer')
            capture mata: vckss_solver__stata_res_rcpt(          ///
                "`resource_components'",                        ///
                "`resource_forecasts'", `resource_row',          ///
                "resource_route_status",                        ///
                "resource_route_message",                       ///
                "resource_route_phase",                         ///
                "resource_gate_applied")
            local resource_receipt_rc = _rc
            if `resource_receipt_rc' {
                capture mata: vckss_solver__resource_clear()
            }
            else if "`resource_gate_applied'" == "APPLIED" {
                local resource_status `resource_route_status'
                local resource_message `"`resource_route_message'"'
                local resource_peak_phase `resource_route_phase'
            }
            else if !`mata_call_rc' & "`mata_status'" == "CONVERGED" {
                local resource_receipt_rc = 498
            }
            quietly _fevc_lifecycle_memory, stage(after_work)
            local life_mem_work_bytes = r(total_alloc_bytes)

            quietly timer on `restore_timer'
            capture mata: vckss_scale_runtime__reset()
            local scale_release_rc = _rc
            capture quietly clear
            capture quietly _fevc_lifecycle_phase, phase(restoration)
            local restoration_marker_rc = _rc
            capture quietly restore
            local restore_rc = _rc
            quietly timer off `restore_timer'
            quietly timer list `restore_timer'
            local life_restore_seconds = r(t`restore_timer')
            quietly _fevc_lifecycle_memory, stage(restored)
            local life_mem_restored_bytes = r(total_alloc_bytes)
            local life_sample_restored = 1
            capture confirm numeric variable `touse'
            if _rc local life_sample_restored = 0
            if `life_sample_restored' {
                quietly count if `touse'
                if r(N) != `lifecycle_sample_N' ///
                    local life_sample_restored = 0
            }
            if `life_sample_restored' {
                quietly _datasignature `touse', nonames
                if `"`r(datasignature)'"' !=                       ///
                    `"`lifecycle_sample_signature'"'               ///
                    local life_sample_restored = 0
            }
            quietly _fevc_lifecycle_release_timers,             ///
                timers(`transition_timer' `work_timer' `restore_timer')
            if `scale_release_rc' | `restore_rc' |                 ///
                !`life_sample_restored' {
                quietly _vckss_post_failure "DATA_RESTORATION_FAILED"
                di as error "caller data or estimation-sample restoration failed"
                exit 498
            }
            if `restoration_marker_rc' {
                quietly _vckss_post_failure "PHASE_MARKER_FAILED"
                di as error "the restoration phase marker could not be written"
                exit 498
            }
            if `resource_receipt_rc' {
                quietly _vckss_post_failure "RESOURCE_RECEIPT_FAILED"
                di as error "the final routed resource receipt could not be recorded"
                exit 498
            }
        }
        else {
            capture mata: assert(vckss_solver__resource_config(  ///
                "generic", `resource_non_solver_bytes',          ///
                `resource_selection_peak',                       ///
                `resource_transition_peak',                      ///
                `resource_restoration_peak',                     ///
                `resource_wall_forecast',                        ///
                `resource_hard_memory', `resource_hard_wall') == 0)
            if _rc {
                capture mata: vckss_solver__resource_clear()
                quietly _vckss_post_failure                     ///
                    "RESOURCE_GATE_CONFIGURATION_FAILED"
                di as error "the final routed resource gate could not be configured"
                exit 498
            }
            capture noisily mata: vckss__stata_jla_routed(         ///
                "`depvar'", "`solve_worker'", "`solve_firm'",   ///
                `"`controlvars'"', "`frequency'", "`target'",   ///
                "`solve_deletion'", "`solve_touse'",             ///
                "`semantic_rank'", `fastpath_eligible',           ///
                "`deletion'",                                    ///
                "`nuisance'", `probes', `batch', `seed',          ///
                `tolerance', `maxiter', `rank_tolerance',          ///
                `block_tolerance', `blocksize_limit',              ///
                "`preconditioner'", `memory_gib'*1024^3,         ///
                "`raw_results'", "mata_status",                 ///
                "mata_message", "`diagnostics'",                ///
                "`solver_rhs_diagnostics'",                      ///
                "`route_diagnostics'",                           ///
                "selected_preconditioner", "routing_reason",   ///
                "fallback_status", "fallback_message",         ///
                "`pilot_diagnostics'", "pilot_status",          ///
                "pilot_failure_reason", "`solve_stayer_mask'")
            local mata_call_rc = _rc
            capture mata: vckss_solver__stata_res_rcpt(          ///
                "`resource_components'",                        ///
                "`resource_forecasts'", `resource_row',          ///
                "resource_route_status",                        ///
                "resource_route_message",                       ///
                "resource_route_phase",                         ///
                "resource_gate_applied")
            local resource_receipt_rc = _rc
            if `resource_receipt_rc' {
                capture mata: vckss_solver__resource_clear()
                quietly _vckss_post_failure "RESOURCE_RECEIPT_FAILED"
                di as error "the final routed resource receipt could not be recorded"
                exit 498
            }
            if "`resource_gate_applied'" == "APPLIED" {
                local resource_status `resource_route_status'
                local resource_message `"`resource_route_message'"'
                local resource_peak_phase `resource_route_phase'
            }
            else if !`mata_call_rc' & "`mata_status'" == "CONVERGED" {
                quietly _vckss_post_failure "RESOURCE_RECEIPT_FAILED"
                di as error "the solver returned without applying the final routed resource gate"
                exit 498
            }
            mata: st_local("rng_contract",                       ///
                vckss_rng__production_contract())
            if `fastpath_eligible' local rng_implementation       ///
                per_domain_stream_semantic_atoms
            else local rng_implementation                         ///
                per_domain_stream_physical_copies
        }
        local rng_leverage_domain leverage
        local rng_target_domain target
        local rng_leverage_probe_first = 1
        local rng_leverage_probe_last = `probes'
        local rng_target_probe_first = 1
        local rng_target_probe_last = `probes'
        if "`fallback_status'" == "" local fallback_status NOT_NEEDED
    }
    if "`selected_algorithm'" == "jla" & `mata_call_rc' {
        local mata_rc = `mata_call_rc'
        quietly _vckss_post_failure "MATA_RUNTIME_FAILED"
        di as error "the Mata backend stopped unexpectedly"
        exit `mata_rc'
    }
    if "`mata_status'" != "CONVERGED" {
        local failure_status `mata_status'
        local failure_message `"`mata_message'"'
        quietly _vckss_post_failure "`failure_status'"           ///
            `"`failure_message'"'
        if "`selected_algorithm'" == "jla" {
            ereturn local engine_requested "`engine_requested'"
            ereturn local engine_selected "`engine_selected'"
            ereturn local fastpath_status "`fastpath_status'"
            ereturn local fastpath_message `"`fastpath_message'"'
            ereturn local resource_status "`resource_status'"
            ereturn local resource_message `"`resource_message'"'
            ereturn local resource_peak_phase "`resource_peak_phase'"
            ereturn local rng_contract "`rng_contract'"
            ereturn local rng_implementation "`rng_implementation'"
            ereturn local rng_call_shape "`rng_call_shape'"
            ereturn local rng_runtime "`rng_runtime'"
            ereturn local rng_leverage_domain "`rng_leverage_domain'"
            ereturn local rng_target_domain "`rng_target_domain'"
            ereturn local life_method "`life_method'"
            ereturn scalar coefficient_cells =                     ///
                `diagnostic_coefficient_cells'
            ereturn scalar deletion_units = `diagnostic_deletion_units'
            ereturn scalar target_strata = `target_strata'
            ereturn scalar life_transition_seconds =               ///
                `life_transition_seconds'
            ereturn scalar life_work_seconds = `life_work_seconds'
            ereturn scalar life_restore_seconds = `life_restore_seconds'
            ereturn scalar life_sample_restored = `life_sample_restored'
            ereturn scalar life_mem_before_bytes = `life_mem_before_bytes'
            ereturn scalar life_mem_cleared_bytes =                ///
                `life_mem_cleared_bytes'
            ereturn scalar life_mem_work_bytes = `life_mem_work_bytes'
            ereturn scalar life_mem_restored_bytes =               ///
                `life_mem_restored_bytes'
            ereturn scalar resource_peak_bytes =                   ///
                `resource_forecasts'[`resource_row',5]
            ereturn scalar resource_mem_admit_bytes =              ///
                `resource_forecasts'[`resource_row',7]
            ereturn scalar resource_wall_upper_seconds =           ///
                `resource_forecasts'[`resource_row',8]
            ereturn scalar resource_wall_admit_seconds =           ///
                `resource_forecasts'[`resource_row',10]
            ereturn scalar resource_hard_mem_bytes =               ///
                `resource_forecasts'[`resource_row',11]
            ereturn scalar resource_hard_wall_seconds =            ///
                `resource_forecasts'[`resource_row',12]
            ereturn local preconditioner_requested "`preconditioner'"
            ereturn local preconditioner_selected ///
                "`selected_preconditioner'"
            ereturn local routing_reason `"`routing_reason'"'
            ereturn local fallback_status "`fallback_status'"
            ereturn local fallback_message `"`fallback_message'"'
            ereturn local batch_requested "`batch_requested'"
            ereturn local batch_routing_reason `"`batch_routing_reason'"'
            ereturn scalar batch = `batch'
            ereturn scalar batch_memory_budget_bytes = ///
                `batch_memory_budget_bytes'
            ereturn scalar batch_column_forecast_bytes = ///
                `batch_column_forecast_bytes'
            ereturn scalar batch_scratch_forecast_bytes = ///
                `batch_scratch_forecast_bytes'
            ereturn scalar memory_gib = `memory_gib'
            ereturn scalar active_processors = `active_processors'
            matrix colnames `route_diagnostics' = planned_rhs memory_bytes ///
                setup_seconds hierarchy_levels edge_complexity ///
                vertex_complexity structural_bytes dense_factor_bytes ///
                workers firms hybrid_vertices hybrid_edges route_code ///
                predicted_vertices predicted_edges ///
                predicted_structural_bytes predicted_scratch_bytes ///
                reserved18 reserved19 reserved20 reserved21 ///
                hierarchy_seconds reserved23 reserved24 forecast_peak_bytes ///
                terminal_vertices
            ereturn scalar route_planned_rhs = `route_diagnostics'[1,1]
            ereturn scalar route_forecast_peak_bytes = ///
                `route_diagnostics'[1,25]
            ereturn scalar route_hierarchy_levels = ///
                `route_diagnostics'[1,4]
            ereturn scalar route_hybrid_vertices = ///
                `route_diagnostics'[1,11]
            ereturn scalar route_hybrid_edges = ///
                `route_diagnostics'[1,12]
            ereturn scalar route_terminal_vertices = ///
                `route_diagnostics'[1,26]
            ereturn scalar memory_forecast_bytes =                 ///
                `resource_forecasts'[`resource_row',5]
            ereturn matrix route_diagnostics = `route_diagnostics'
            ereturn local route_api "KSS-ROUTE-STRUCTURAL-V1"
        }
        di as error `"`failure_message'"'
        if inlist("`failure_status'", "EXACT_SIZE_LIMIT", "INVALID_INPUT", ///
            "INVALID_FREQUENCY", "INVALID_TARGET_WEIGHT",                 ///
            "INVALID_IDENTIFIER", "UNSUPPORTED_DELETION") exit 198
        if inlist("`failure_status'", "INVALID_NUISANCE",                  ///
            "INVALID_TOLERANCE", "BLOCK_SIZE_LIMIT",                     ///
            "CROSS_COORDINATE_MATCH") exit 198
        exit 498
    }

    /* Exact first computes the mover result used by its block kernel, then the
       combined MATLAB-style target. JLA estimates the combined target in one
       routed solve and therefore needs no second estimator pass. */
    if "`stayers'" == "both" & "`selected_algorithm'" == "exact" {
        capture noisily _vckss_mata_stayer_hybrid `depvar'       ///
            `hybrid_worker' `hybrid_firm' `"`controlvars'"'     ///
            `frequency' `target' `deletion_id' `hybrid_stayer'   ///
            `hybrid_touse' `nuisance' `rank_tolerance'           ///
            `block_tolerance' `exact_limit' `blocksize_limit'    ///
            `probeorder'
        if _rc exit _rc
        matrix `hybrid_raw_results' = r(raw)
        matrix `hybrid_diagnostics' = r(diagnostics)
        matrix `hybrid_correction_source' = r(source)
    }

    if "`stayers'" == "both" {
        if "`selected_algorithm'" == "exact" {
            matrix `mover_raw_results' = `raw_results'
            matrix `mover_diagnostics' = `diagnostics'
            matrix `raw_results' = `hybrid_raw_results'
            matrix `diagnostics' = `hybrid_diagnostics'
        }
        else {
            matrix `hybrid_raw_results' = `raw_results'
            matrix `hybrid_diagnostics' = `diagnostics'
            matrix `hybrid_correction_source' = J(2,4,.)
        }
        local N_retained = `diagnostics'[1,1]
        local worker_levels = `diagnostics'[1,3]
        local firm_levels = `diagnostics'[1,4]
        local parameters = `diagnostics'[1,5]
        local target_outcome_variance = `hybrid_target_outcome_variance'
        local regression_outcome_variance =                       ///
            `hybrid_regression_variance'
    }

    quietly timer on $VCKSS_STAGE_VALIDATION_TIMER
    local generic_identity_resid = .
    local generic_complete_resid = .
    if "`selected_algorithm'" == "jla" &                         ///
        "`engine_selected'" == "generic" {
        tempname generic_identity_residual generic_complete_residual
        capture mata: st_numscalar("`generic_identity_residual'", ///
            vckss_solver__target_id_resid(                        ///
                st_matrix("`raw_results'")[1..3,.]))
        if _rc | missing(scalar(`generic_identity_residual')) {
            quietly _vckss_post_failure "TARGET_IDENTITY_FAILED"
            di as error "generic target accounting identity is nonfinite"
            exit 498
        }
        local generic_identity_resid =                            ///
            scalar(`generic_identity_residual')
        if `generic_identity_resid' >                             ///
            4096*2.2204460492503131e-16 {
            quietly _vckss_post_failure "TARGET_IDENTITY_FAILED"
            di as error "generic target accounting identity failed"
            exit 498
        }
        capture mata: st_numscalar("`generic_complete_residual'", ///
            vckss_solver__rhs_resid_max(                          ///
                st_matrix("`solver_rhs_diagnostics'")))
        if _rc | missing(scalar(`generic_complete_residual')) {
            quietly _vckss_post_failure "SOLVER_DIAGNOSTICS_INVALID"
            di as error "generic complete-RHS residual diagnostics are invalid"
            exit 498
        }
        local generic_complete_resid =                            ///
            scalar(`generic_complete_residual')
    }

    matrix colnames `raw_results' = worker_variance firm_variance ///
        worker_firm_covariance total_variance
    matrix rownames `raw_results' = plugin bias_correction corrected ///
        numerical_mcse
    matrix `plugin' = `raw_results'[1,1..4]
    matrix `correction' = `raw_results'[2,1..4]
    matrix `corrected' = `raw_results'[3,1..4]
    matrix `kss_return' = `corrected'
    matrix `mcse' = `raw_results'[4,1..4]
    matrix colnames `plugin' = worker_variance firm_variance ///
        worker_firm_covariance total_variance
    matrix colnames `correction' = worker_variance firm_variance ///
        worker_firm_covariance total_variance
    matrix colnames `corrected' = worker_variance firm_variance ///
        worker_firm_covariance total_variance
    matrix colnames `mcse' = worker_variance firm_variance ///
        worker_firm_covariance total_variance

    if `inference_requested' {
        local inference_runtime_loaded = 0
        capture mata: vckss_inference__api_level()
        if !_rc local inference_runtime_loaded = 1
        capture mata: assert(vckss_inference__api_level() == 2 & ///
            vckss_inference__build_id() ==                       ///
            "vckss-inference-api2-q1-target-status")
        if _rc {
            if `inference_runtime_loaded' {
                quietly _vckss_post_failure "STALE_INFERENCE_RUNTIME" ///
                    "A different fevc inference runtime is already loaded."
                di as error "restart Stata or run discard before loading this inference runtime"
                exit 498
            }
            capture findfile fevc_inference.mata
            if _rc {
                quietly _vckss_post_failure "INFERENCE_RUNTIME_NOT_FOUND" ///
                    "fevc_inference.mata was not found on the Stata adopath."
                di as error "fevc_inference.mata was not found"
                exit 601
            }
            quietly do `"`r(fn)'"'
            capture mata: assert(vckss_inference__api_level() == 2 & ///
                vckss_inference__build_id() ==                   ///
                "vckss-inference-api2-q1-target-status")
            if _rc {
                quietly _vckss_post_failure "INVALID_INFERENCE_RUNTIME" ///
                    "The installed fevc inference runtime is incompatible with this command."
                di as error "the installed inference runtime is incompatible"
                exit 498
            }
        }
        capture noisily mata: vckss_inference__stata(            ///
            "`depvar'", "`id_worker'", "`id_firm'",          ///
            `"`controlvars'"', "`frequency'", "`target'",     ///
            "`deletion_id'", "`touse'", "`hybrid_worker'",   ///
            "`hybrid_firm'", "`hybrid_deletion'",             ///
            "`hybrid_stayer'", "`hybrid_touse'",              ///
            "`deletion'", "`stayers'", "`nuisance'",        ///
            "`inference'",                                     ///
            `level', `inferencesimulations', `inferenceseed',    ///
            `inferencebins', `"`project'"', "`projecteffect'", ///
            "`projectweight'", `rank_tolerance',                ///
            `block_tolerance', `blocksize_limit', "`corrected'", ///
            "`inference_V_primitive'", "`inference_V'",        ///
            "`inference_highrank'", "`inference_q1'",          ///
            "`inference_q1_status'",                           ///
            "`projection_b'", "`projection_V'",                ///
            "`projection_V_naive'", "`projection_results'",    ///
            "`inference_diagnostics'", "inference_status",     ///
            "inference_message")
        local inference_rc = _rc
        if `inference_rc' {
            quietly _vckss_post_failure "INFERENCE_RUNTIME_FAILED" ///
                "The point estimate converged, but requested inference stopped unexpectedly; no partial result was posted."
            di as error "the inference runtime stopped unexpectedly"
            exit `inference_rc'
        }
        if "`inference_status'" != "CONVERGED" {
            quietly _vckss_post_failure "`inference_status'"    ///
                `"`inference_message'"'
            di as error `"`inference_message'"'
            exit 498
        }
        if "`inference'" != "none" {
            matrix rownames `inference_V_primitive' =            ///
                worker_variance firm_variance worker_firm_covariance
            matrix colnames `inference_V_primitive' =            ///
                worker_variance firm_variance worker_firm_covariance
            matrix rownames `inference_V' = worker_variance      ///
                firm_variance worker_firm_covariance total_variance
            matrix colnames `inference_V' = worker_variance      ///
                firm_variance worker_firm_covariance total_variance
            matrix rownames `inference_highrank' = worker_variance ///
                firm_variance worker_firm_covariance total_variance
            matrix colnames `inference_highrank' = estimate se lb ub
        }
        if "`inference'" == "q1" {
            matrix rownames `inference_q1_status' = worker_variance ///
                firm_variance worker_firm_covariance total_variance
            matrix colnames `inference_q1_status' = status standardized_determinant ///
                remainder_influence_variance remainder_trace_variance recenter_var_b1 ///
                remainder_identity_error reserved
            matrix rownames `inference_q1' = worker_variance     ///
                firm_variance worker_firm_covariance total_variance
            matrix colnames `inference_q1' = estimate highrank_se ///
                wald_lb wald_ub am_lb am_ub lambda1 eigen_share   ///
                max_weight_sq var_b1 cov_b1_theta1 var_theta1    ///
                b1 theta1 F curvature critical_value
        }
        if `project_supplied' {
            matrix colnames `projection_b' = _cons `project'
            matrix rownames `projection_V' = _cons `project'
            matrix colnames `projection_V' = _cons `project'
            matrix rownames `projection_V_naive' = _cons `project'
            matrix colnames `projection_V_naive' = _cons `project'
            matrix rownames `projection_results' = _cons `project'
            matrix colnames `projection_results' = estimate se z p lb ub naive_se
        }
        matrix colnames `inference_diagnostics' = simulations seed ///
            bins level psd_cleanup raw_variance_min raw_variance_max ///
            mover_rows stayer_rows projection_psd_cleanup          ///
            variance_floor_count projection_variance_min           ///
            projection_variance_max
    }

    /* e(results) retains the four scientific targets, including the raw
       covariance.  e(decomposition) is an applied-user view whose sorting
       row is twice that covariance, so its first three rows add to the
       worker-plus-firm total in every level and share column. */
    matrix `decomposition' =                                      ///
        (`raw_results'[1,1], `raw_results'[2,1], `raw_results'[3,1] \ ///
         `raw_results'[1,2], `raw_results'[2,2], `raw_results'[3,2] \ ///
         2*`raw_results'[1,3], 2*`raw_results'[2,3],               ///
             2*`raw_results'[3,3] \                               ///
         `raw_results'[1,4], `raw_results'[2,4], `raw_results'[3,4])
    matrix `decomposition' = `decomposition', J(4,4,.)
    if !missing(`target_outcome_variance') &                      ///
        `target_outcome_variance' > 0 {
        forvalues component = 1/4 {
            matrix `decomposition'[`component',4] =               ///
                `decomposition'[`component',1]/`target_outcome_variance'
            matrix `decomposition'[`component',5] =               ///
                `decomposition'[`component',3]/`target_outcome_variance'
        }
    }
    if !missing(`decomposition'[4,1]) & `decomposition'[4,1] > 0 {
        forvalues component = 1/4 {
            matrix `decomposition'[`component',6] =               ///
                `decomposition'[`component',1]/`decomposition'[4,1]
        }
    }
    if !missing(`decomposition'[4,3]) & `decomposition'[4,3] > 0 {
        forvalues component = 1/4 {
            matrix `decomposition'[`component',7] =               ///
                `decomposition'[`component',3]/`decomposition'[4,3]
        }
    }
    matrix rownames `decomposition' = worker_variance firm_variance ///
        sorting_2covariance total_worker_firm
    matrix colnames `decomposition' = plugin bias_correction       ///
        corrected plugin_share_outcome corrected_share_outcome    ///
        plugin_share_worker_firm corrected_share_worker_firm

    if "`stayers'" == "both" {
        matrix colnames `hybrid_raw_results' = worker_variance     ///
            firm_variance worker_firm_covariance total_variance
        matrix rownames `hybrid_raw_results' = plugin             ///
            bias_correction corrected numerical_mcse
        matrix `hybrid_plugin' = `hybrid_raw_results'[1,1..4]
        matrix `hybrid_correction' = `hybrid_raw_results'[2,1..4]
        matrix `hybrid_corrected' = `hybrid_raw_results'[3,1..4]
        matrix `hybrid_kss' = `hybrid_corrected'
        foreach hybrid_matrix in hybrid_plugin hybrid_correction  ///
            hybrid_corrected hybrid_kss {
            matrix colnames ``hybrid_matrix'' = worker_variance   ///
                firm_variance worker_firm_covariance total_variance
        }

        matrix `hybrid_decomposition' =                           ///
            (`hybrid_raw_results'[1,1], `hybrid_raw_results'[2,1], ///
                `hybrid_raw_results'[3,1] \                       ///
             `hybrid_raw_results'[1,2], `hybrid_raw_results'[2,2], ///
                `hybrid_raw_results'[3,2] \                       ///
             2*`hybrid_raw_results'[1,3],                         ///
                2*`hybrid_raw_results'[2,3],                      ///
                2*`hybrid_raw_results'[3,3] \                     ///
             `hybrid_raw_results'[1,4], `hybrid_raw_results'[2,4], ///
                `hybrid_raw_results'[3,4])
        matrix `hybrid_decomposition' = `hybrid_decomposition', J(4,4,.)
        if !missing(`hybrid_target_outcome_variance') &           ///
            `hybrid_target_outcome_variance' > 0 {
            forvalues component = 1/4 {
                matrix `hybrid_decomposition'[`component',4] =    ///
                    `hybrid_decomposition'[`component',1] /       ///
                    `hybrid_target_outcome_variance'
                matrix `hybrid_decomposition'[`component',5] =    ///
                    `hybrid_decomposition'[`component',3] /       ///
                    `hybrid_target_outcome_variance'
            }
        }
        if !missing(`hybrid_decomposition'[4,1]) &                ///
            `hybrid_decomposition'[4,1] > 0 {
            forvalues component = 1/4 {
                matrix `hybrid_decomposition'[`component',6] =    ///
                    `hybrid_decomposition'[`component',1] /       ///
                    `hybrid_decomposition'[4,1]
            }
        }
        if !missing(`hybrid_decomposition'[4,3]) &                ///
            `hybrid_decomposition'[4,3] > 0 {
            forvalues component = 1/4 {
                matrix `hybrid_decomposition'[`component',7] =    ///
                    `hybrid_decomposition'[`component',3] /       ///
                    `hybrid_decomposition'[4,3]
            }
        }
        matrix rownames `hybrid_decomposition' = worker_variance  ///
            firm_variance sorting_2covariance total_worker_firm
        matrix colnames `hybrid_decomposition' = plugin           ///
            bias_correction corrected plugin_share_outcome        ///
            corrected_share_outcome plugin_share_worker_firm      ///
            corrected_share_worker_firm

        matrix rownames `hybrid_correction_source' = mover_match  ///
            stayer_observation
        matrix colnames `hybrid_correction_source' =              ///
            worker_variance firm_variance worker_firm_covariance  ///
            total_variance
        if "`selected_algorithm'" == "exact" {
            matrix `mover_plugin' = `mover_raw_results'[1,1..4]
            matrix `mover_correction' = `mover_raw_results'[2,1..4]
            matrix `mover_kss' = `mover_raw_results'[3,1..4]
            foreach mover_matrix in mover_plugin mover_correction mover_kss {
                matrix colnames ``mover_matrix'' = worker_variance ///
                    firm_variance worker_firm_covariance total_variance
            }
        }
        matrix `hybrid_sample_accounting' =                       ///
            (`N_mover_retained', `hybrid_mover_physical',         ///
                `mover_worker_levels', `hybrid_mover_target_mass', ///
                `N_mover_deletion_units' \                         ///
             `N_hybrid_stayer_rows', `hybrid_stayer_physical',    ///
                `N_hybrid_stayers', `hybrid_stayer_target_mass',  ///
                `hybrid_stayer_physical' \                        ///
             `N_mover_retained'+`N_hybrid_stayer_rows',           ///
                `hybrid_total_physical', `hybrid_diagnostics'[1,3], ///
                `hybrid_target_mass', `hybrid_diagnostics'[1,6])
        matrix rownames `hybrid_sample_accounting' = mover stayer total
        matrix colnames `hybrid_sample_accounting' = stored_rows  ///
            physical_observations worker_levels target_weight_mass ///
            deletion_units
    }

    local residual_variance =                                    ///
        `diagnostics'[1,14]/`diagnostics'[1,2]
    local full_model_explained_variance = .
    local full_model_explained_share = .
    if !missing(`regression_outcome_variance') &                  ///
        !missing(`residual_variance') {
        local full_model_explained_variance =                     ///
            `regression_outcome_variance'-`residual_variance'
        if `regression_outcome_variance' > 0 {
            local full_model_explained_share =                    ///
                `full_model_explained_variance'/                  ///
                `regression_outcome_variance'
        }
    }

    /* PREP-RHS-PERF-V1 is diagnostic only.  Stage times are exclusive:
       selection_other subtracts graph pruning from the enclosing selection
       timer; semantic ordering and compressed preparation are timed
       separately; lifecycle transition/restore sit outside the numerical
       engine.  No profile value is consulted by routing or scientific gates. */
    local prep_selection_other = max(0,                         ///
        `sample_selection_seconds'-`graph_diagnostics'[1,12])
    local prep_observed_total = `prep_selection_other' +        ///
        `graph_diagnostics'[1,12] + `semantic_order_seconds' +  ///
        `compression_seconds' + `life_transition_seconds' +    ///
        `life_restore_seconds'
    matrix `prep_profile' = (`prep_selection_other',             ///
        `graph_diagnostics'[1,12], `semantic_order_seconds',     ///
        `compression_seconds', `life_transition_seconds',       ///
        `life_restore_seconds', `prep_observed_total')
    matrix colnames `prep_profile' = selection_other graph_prune ///
        semantic_order compression_prepare lifecycle_transition ///
        lifecycle_restore observed_total

    /* The boundary profile preserves the established prep_profile and adds
       enough exclusive detail to attribute later scan/group/map changes.
       Counts describe executed operations; they are not performance gates. */
    local prep_boundary_observed_total =                        ///
        `prep_mark_validate_seconds' +                          ///
        `prep_initial_group_seconds' +                          ///
        `prep_runtime_setup_seconds' +                          ///
        `prep_graph_setup_io_seconds' +                         ///
        `graph_diagnostics'[1,12] +                             ///
        `prep_retained_map_seconds' +                           ///
        `semantic_order_seconds' + `compression_seconds' +     ///
        `life_transition_seconds' + `life_restore_seconds'
    matrix `prep_boundary_profile' = (                          ///
        `prep_mark_validate_seconds',                           ///
        `prep_initial_group_seconds',                           ///
        `prep_runtime_setup_seconds',                           ///
        `prep_graph_setup_io_seconds',                          ///
        `graph_diagnostics'[1,12],                              ///
        `prep_retained_map_seconds',                            ///
        `semantic_order_seconds', `compression_seconds',       ///
        `life_transition_seconds', `life_restore_seconds',     ///
        `prep_boundary_observed_total')
    matrix colnames `prep_boundary_profile' = mark_validate     ///
        initial_group runtime_setup graph_setup_io graph_prune  ///
        retained_map semantic_order compression_prepare         ///
        lifecycle_transition lifecycle_restore observed_total

    local prep_deletion_group_calls =                           ///
        cond("`deletion'" == "observation",0,1)
    matrix `prep_boundary_counts' = (2,                         ///
        `prep_deletion_group_calls', 0,                         ///
        `prep_semantic_group_calls', `prep_sort_calls',         ///
        4, `N_complete', 2, `N_mover_retained',                 ///
        `prep_compression_import_columns',                      ///
        `prep_compression_import_rows')
    matrix colnames `prep_boundary_counts' =                    ///
        initial_id_group_calls deletion_group_calls             ///
        retained_id_group_calls semantic_group_calls            ///
        stata_sort_calls graph_import_columns graph_import_rows ///
        retained_map_columns retained_map_rows                  ///
        compression_import_columns compression_import_rows

    matrix `rhs_profile' = (`diagnostics'[1,15],                 ///
        `diagnostics'[1,16], `diagnostics'[1,17],                ///
        `diagnostics'[1,18], `diagnostics'[1,25],                ///
        `diagnostics'[1,26], `diagnostics'[1,27],                ///
        `diagnostics'[1,28])
    matrix colnames `rhs_profile' = fit leverage target          ///
        correction_nested schur preconditioner_apply pcg        ///
        solver_backend

    local work_scatter_plans = .
    local work_scatter_calls = .
    local work_scatter_sorts = .
    local work_logical_rhs = .
    local work_physical_rhs = .
    local work_solver_calls = .
    local work_leverage_batches = .
    local work_target_batches = .
    if "`selected_algorithm'" == "jla" {
        /* Packed execution sends exactly the active logical columns to each
           Schur/preconditioner batch; the paired counters make any future
           inactive-column work visible without timing-based routing. */
        local work_logical_rhs = `diagnostics'[1,29] +            ///
            `diagnostics'[1,31]
        local work_physical_rhs = `work_logical_rhs'
        local work_leverage_batches = ceil(`probes'/`leverage_batch')
        local work_target_batches = ceil(`probes'/`target_batch')
        local work_solver_calls = 1 + `work_leverage_batches' +  ///
            `work_target_batches'
        if "`engine_selected'" == "compressed" {
            local work_scatter_plans = 2
            local work_scatter_calls = 5 +                      ///
                `work_leverage_batches' + `work_target_batches'
            local work_scatter_sorts = 0
        }
        else {
            local work_scatter_plans = 0
            local work_scatter_calls = 0
            local work_scatter_sorts = 0
        }
    }
    matrix `work_counters' = (`work_scatter_plans',              ///
        `work_scatter_calls', `work_scatter_sorts',              ///
        `work_logical_rhs', `work_physical_rhs',                 ///
        `work_solver_calls', `diagnostics'[1,29],                ///
        `diagnostics'[1,30], `diagnostics'[1,31],                ///
        `diagnostics'[1,32], `work_leverage_batches',            ///
        `work_target_batches')
    matrix colnames `work_counters' = scatter_plan_builds        ///
        scatter_apply_calls scatter_sort_rebuilds                ///
        logical_operator_columns physical_operator_columns       ///
        solver_calls                                             ///
        schur_actions schur_batches preconditioner_applications  ///
        preconditioner_batches leverage_batches target_batches

    /* FE-BUF-PERF-V1 is diagnostic-only.  The measurement baseline records
       every Schur batch on the legacy path; the candidate replaces these
       fields with solve-local buffer usage.  No profile value participates
       in scientific, routing, RNG, or resource-admission decisions. */
    if "`selected_algorithm'" == "jla" {
        matrix `fe_buffer_profile' = (`diagnostics'[1,33],       ///
            `diagnostics'[1,34], `diagnostics'[1,35],           ///
            `diagnostics'[1,36], `diagnostics'[1,37],           ///
            `diagnostics'[1,38], `diagnostics'[1,39],           ///
            `diagnostics'[1,40], `diagnostics'[1,41],           ///
            `diagnostics'[1,42])
    }
    else matrix `fe_buffer_profile' = (0,0,0,0,0,0,0,0,0,0)
    matrix colnames `fe_buffer_profile' = applicable            ///
        workspace_builds buffered_schur_batches                 ///
        legacy_schur_batches buffered_operator_columns          ///
        legacy_operator_columns packed_fallback_batches         ///
        max_buffer_width modeled_workspace_peak_bytes           ///
        modeled_cell_bytes_avoided

    local headline_touse `touse'
    if "`stayers'" == "both" local headline_touse `hybrid_touse'
    quietly summarize `frequency' if `headline_touse', meanonly
    local N_physical = r(sum)
    ereturn clear
    if "`inference'" != "none" {
        ereturn post `corrected' `inference_V', obs(`N_physical') ///
            esample(`headline_touse') depname(`depvar')
    }
    else {
        ereturn post `corrected', obs(`N_physical') esample(`headline_touse') ///
            depname(`depvar')
    }
    ereturn matrix plugin = `plugin'
    ereturn matrix correction = `correction'
    ereturn matrix kss = `kss_return'
    ereturn matrix numerical_mcse = `mcse'
    ereturn matrix results = `raw_results'
    ereturn matrix decomposition = `decomposition'
    if `inference_requested' {
        ereturn matrix inference_diagnostics = `inference_diagnostics'
        ereturn scalar level = `level'
        if "`inference'" != "none" {
            ereturn scalar inference_simulations = `inferencesimulations'
            ereturn scalar inference_seed = `inferenceseed'
            ereturn scalar inference_bins = `inferencebins'
            ereturn matrix V_primitive = `inference_V_primitive'
            ereturn matrix component_inference = `inference_highrank'
        }
        if "`inference'" == "q1" {
            ereturn matrix q1_inference = `inference_q1'
            tempname q1status
            matrix `q1status' = `inference_q1_status'[1..4,1]
            ereturn matrix q1_status = `q1status'
            ereturn local q1_status_codes "0 computed; 1 nonpositive variance; 2 singular covariance; 3 interval failure; 4 unidentified mode; 5 target variance fit invalid"
            local q1computed = 0
            forvalues row = 1/4 {
                local q1computed = `q1computed'+(`inference_q1_status'[`row',1]==0)
            }
            ereturn scalar q1_computed_targets = `q1computed'
            ereturn matrix q1_failure_diagnostics = `inference_q1_status'
        }
        if `project_supplied' {
            ereturn matrix projection_b = `projection_b'
            ereturn matrix projection_V = `projection_V'
            ereturn matrix projection_V_naive = `projection_V_naive'
            ereturn matrix projection_results = `projection_results'
            ereturn local projection_effect "`projecteffect'"
            ereturn local projection_weight "`projectweight'"
            ereturn local projection_variables "`project'"
            ereturn local projection_constant                     ///
                "automatic; normalization-dependent"
        }
    }
    if "`stayers'" == "both" {
        if "`selected_algorithm'" == "exact" {
            ereturn matrix mover_plugin = `mover_plugin'
            ereturn matrix mover_correction = `mover_correction'
            ereturn matrix mover_kss = `mover_kss'
            ereturn matrix mover_results = `mover_raw_results'
        }
        ereturn matrix stayer_hybrid_plugin = `hybrid_plugin'
        ereturn matrix stayer_hybrid_correction = `hybrid_correction'
        ereturn matrix stayer_hybrid_kss = `hybrid_kss'
        ereturn matrix stayer_hybrid_results = `hybrid_raw_results'
        ereturn matrix stayer_hybrid_decomposition =              ///
            `hybrid_decomposition'
        ereturn matrix stayer_hybrid_correction_source =          ///
            `hybrid_correction_source'
        ereturn matrix stayer_hybrid_sample_accounting =          ///
            `hybrid_sample_accounting'
        ereturn scalar stayer_hybrid_N_stored =                   ///
            `hybrid_diagnostics'[1,1]
        ereturn scalar stayer_hybrid_N_physical =                 ///
            `hybrid_diagnostics'[1,2]
        ereturn scalar stayer_hybrid_worker_levels =              ///
            `hybrid_diagnostics'[1,3]
        ereturn scalar stayer_hybrid_firm_levels =                ///
            `hybrid_diagnostics'[1,4]
        ereturn scalar stayer_hybrid_parameters =                 ///
            `hybrid_diagnostics'[1,5]
        ereturn scalar stayer_hybrid_full_parameters =            ///
            `hybrid_diagnostics'[1,23]
        ereturn scalar stayer_hybrid_corr_parameters =            ///
            `hybrid_diagnostics'[1,24]
        ereturn scalar stayer_hybrid_deletion_units =             ///
            `hybrid_diagnostics'[1,6]
        ereturn scalar stayer_hybrid_target_mass =                ///
            `hybrid_diagnostics'[1,7]
        ereturn scalar stayer_hybrid_max_leverage =               ///
            `hybrid_diagnostics'[1,8]
        ereturn scalar stayer_hybrid_information_rcond =          ///
            `hybrid_diagnostics'[1,9]
        ereturn scalar stayer_hybrid_inverse_relres =             ///
            `hybrid_diagnostics'[1,10]
        ereturn scalar stayer_hybrid_weighted_rss =               ///
            `hybrid_diagnostics'[1,14]
        ereturn scalar stayer_hybrid_target_y_variance =          ///
            `hybrid_target_outcome_variance'
        ereturn scalar stayer_hybrid_N_stayers = `N_hybrid_stayers'
        ereturn scalar stayer_hybrid_N_stayer_rows =              ///
            `N_hybrid_stayer_rows'
        ereturn scalar stayer_hybrid_N_stayer_physical =          ///
            `hybrid_stayer_physical'
        ereturn scalar stayer_hybrid_N_singleton_drop =           ///
            `N_hyb_singleton_drop'
        ereturn scalar stayer_hybrid_N_unattached =               ///
            `N_hyb_unattached'
        ereturn scalar stayer_hybrid_mover_target_mass =          ///
            `hybrid_mover_target_mass'
        ereturn scalar stayer_hybrid_stayer_target_mass =         ///
            `hybrid_stayer_target_mass'
        ereturn local stayer_hybrid_status "CONVERGED"
        ereturn local stayer_hybrid_target_population             ///
            "retained movers plus eligible original one-firm stayers attached to retained mover firms"
        ereturn local stayer_hybrid_deletion                      ///
            "mover matches plus stayer physical observations"
        ereturn local stayer_hybrid_assumption                    ///
            "mover correction is match-robust; stayer correction is not match-robust"
        ereturn local stayer_hybrid_sample_rule                   ///
            "original one-firm stayers; retained mover firm; physical T>=2; graph-dropped movers excluded"
        ereturn local stayer_hybrid_esample                       ///
            "e(sample) marks retained movers plus eligible attached stayers"
        ereturn local stayer_hybrid_targetweight                  ///
            "pooled stored-row target mass; explicit mass is not multiplied by frequency"
        ereturn local stayer_hybrid_nuisance "`nuisance'"
    }
    ereturn matrix prep_profile = `prep_profile'
    ereturn matrix prep_boundary_profile = `prep_boundary_profile'
    ereturn matrix prep_boundary_counts = `prep_boundary_counts'
    ereturn matrix rhs_profile = `rhs_profile'
    ereturn matrix work_counters = `work_counters'
    ereturn matrix fe_buffer_profile = `fe_buffer_profile'
    ereturn local fe_buffer_profile_schema "FE-BUF-PERF-V1"
    ereturn local prep_boundary_profile_schema "PREP-BND-PERF-V1"
    ereturn local prep_boundary_counts_schema "PREP-BND-COUNTS-V1"
    ereturn scalar N_stored = `diagnostics'[1,1]
    ereturn scalar N_physical = `diagnostics'[1,2]
    ereturn scalar N_requested = `N_scope'
    ereturn scalar N_complete = `N_complete'
    ereturn scalar N_retained = `N_retained'
    ereturn scalar N_mover_input = `N_mover_input'
    ereturn scalar N_initial_component = `N_initial_component'
    ereturn scalar N_initial_component_dropped = ///
        `N_initial_component_dropped'
    ereturn scalar N_mover_dropped = `N_mover_dropped'
    ereturn scalar N_graph_dropped = `N_graph_dropped'
    ereturn scalar graph_edges = `graph_diagnostics'[1,5]
    ereturn scalar graph_articulation_workers = ///
        `graph_diagnostics'[1,6]
    ereturn scalar graph_leaveout_components = `graph_diagnostics'[1,7]
    ereturn scalar graph_initial_component_rows = `graph_diagnostics'[1,9]
    ereturn scalar graph_insufficient_workers = ///
        `graph_diagnostics'[1,10]
    ereturn scalar graph_pruning_iterations = `graph_diagnostics'[1,11]
    ereturn scalar graph_seconds = `graph_diagnostics'[1,12]
    ereturn scalar graph_retained_edges = `graph_diagnostics'[1,13]
    ereturn scalar graph_bridge_units_removed = ///
        `graph_diagnostics'[1,14]
    ereturn scalar graph_bridge_rows_removed = ///
        `graph_diagnostics'[1,15]
    ereturn scalar graph_bridge_iterations = `graph_diagnostics'[1,16]
    ereturn scalar graph_fixedpoint_iterations = ///
        `graph_diagnostics'[1,17]
    ereturn scalar graph_final_bridge_units = `graph_diagnostics'[1,18]
    ereturn scalar N_stayers = `N_stayers'
    ereturn scalar N_stayer_rows = `N_stayer_rows'
    ereturn scalar worker_levels = `diagnostics'[1,3]
    ereturn scalar firm_levels = `diagnostics'[1,4]
    ereturn scalar parameters = `diagnostics'[1,5]
    ereturn scalar full_parameters = `diagnostics'[1,23]
    ereturn scalar correction_parameters = `diagnostics'[1,24]
    ereturn scalar deletion_units = `diagnostics'[1,6]
    ereturn scalar target_weight_sum = `diagnostics'[1,7]
    ereturn scalar max_leverage = `diagnostics'[1,8]
    ereturn scalar information_rcond = `diagnostics'[1,9]
    ereturn scalar inverse_relres = `diagnostics'[1,10]
    ereturn scalar solver_iterations = `diagnostics'[1,11]
    ereturn scalar solver_max_residual = `diagnostics'[1,12]
    ereturn scalar probes = `diagnostics'[1,13]
    ereturn scalar weighted_rss = `diagnostics'[1,14]
    ereturn scalar target_outcome_variance = `target_outcome_variance'
    ereturn scalar regression_outcome_variance =                  ///
        `regression_outcome_variance'
    ereturn scalar residual_variance = `residual_variance'
    ereturn scalar full_model_explained_variance =                ///
        `full_model_explained_variance'
    ereturn scalar full_model_explained_share =                   ///
        `full_model_explained_share'
    ereturn scalar fit_seconds = `diagnostics'[1,15]
    ereturn scalar leverage_seconds = `diagnostics'[1,16]
    ereturn scalar target_seconds = `diagnostics'[1,17]
    ereturn scalar correction_seconds = `diagnostics'[1,18]
    ereturn scalar setup_seconds = `diagnostics'[1,19]
    ereturn scalar preconditioner_seconds = `diagnostics'[1,19]
    ereturn scalar preconditioner_ratio = `diagnostics'[1,20]
    ereturn scalar control_schur_rcond = `diagnostics'[1,21]
    ereturn scalar deletion_rank_gap = `diagnostics'[1,22]
    if "`selected_algorithm'" == "jla" {
        matrix colnames `solver_rhs_diagnostics' = stage batch_start rhs ///
            iterations relative_residual converged
        ereturn matrix solver_rhs_diagnostics = `solver_rhs_diagnostics'
        matrix colnames `route_diagnostics' = planned_rhs memory_bytes ///
            setup_seconds hierarchy_levels edge_complexity ///
            vertex_complexity structural_bytes dense_factor_bytes ///
            workers firms hybrid_vertices hybrid_edges route_code ///
            predicted_vertices predicted_edges ///
            predicted_structural_bytes predicted_scratch_bytes ///
            reserved18 reserved19 reserved20 reserved21 ///
            hierarchy_seconds reserved23 reserved24 forecast_peak_bytes ///
            terminal_vertices
        ereturn scalar route_code = `route_diagnostics'[1,13]
        ereturn scalar route_planned_rhs = `route_diagnostics'[1,1]
        ereturn scalar route_forecast_peak_bytes = ///
            `route_diagnostics'[1,25]
        ereturn scalar route_hierarchy_levels = ///
            `route_diagnostics'[1,4]
        ereturn scalar route_hybrid_vertices = ///
            `route_diagnostics'[1,11]
        ereturn scalar route_hybrid_edges = ///
            `route_diagnostics'[1,12]
        ereturn scalar route_terminal_vertices = ///
            `route_diagnostics'[1,26]
        ereturn scalar memory_forecast_bytes = ///
            `route_diagnostics'[1,25]+`batch_scratch_forecast_bytes'
        ereturn matrix route_diagnostics = `route_diagnostics'
        ereturn local route_api "KSS-ROUTE-STRUCTURAL-V1"
        ereturn scalar schur_seconds = `diagnostics'[1,25]
        ereturn scalar preconditioner_apply_seconds = `diagnostics'[1,26]
        ereturn scalar pcg_seconds = `diagnostics'[1,27]
        ereturn scalar solver_backend_seconds = `diagnostics'[1,28]
        ereturn scalar solver_schur_actions = `diagnostics'[1,29]
        ereturn scalar solver_schur_batches = `diagnostics'[1,30]
        ereturn scalar solver_precond_applications = ///
            `diagnostics'[1,31]
        ereturn scalar solver_precond_batches = `diagnostics'[1,32]

        matrix colnames `resource_components' = raw_stata_bytes      ///
            cell_bytes deletion_unit_bytes target_stratum_bytes      ///
            cmg_hierarchy_bytes phase_scratch_bytes                  ///
            sorting_compression_bytes solve_ahead_bytes              ///
            output_certificate_bytes preservation_transition_bytes  ///
            runtime_resident_bytes
        matrix rownames `resource_components' = compressed generic
        matrix colnames `resource_forecasts' = select_peak_bytes     ///
            transition_peak_bytes numerical_peak_bytes               ///
            restore_peak_bytes peak_bytes memory_headroom_fraction   ///
            memory_admit_bytes wall_upper_seconds                    ///
            wall_headroom_fraction wall_admit_seconds                ///
            hard_mem_bytes hard_wall_seconds memory_admitted         ///
            wall_admitted admitted
        matrix rownames `resource_forecasts' = compressed generic
        matrix colnames `resource_scaling' = row_scale               ///
            structure_scale compressed_wall_upper                    ///
            generic_wall_upper physical_scale physical_observations  ///
            leverage_rng_calls target_rng_calls rng_call_seconds     ///
            rng_total_calls rng_wall_upper
        ereturn scalar resource_raw_stata_bytes =                    ///
            `resource_components'[`resource_row',1]
        ereturn scalar resource_cell_bytes =                         ///
            `resource_components'[`resource_row',2]
        ereturn scalar resource_deletion_unit_bytes =                ///
            `resource_components'[`resource_row',3]
        ereturn scalar resource_target_stratum_bytes =               ///
            `resource_components'[`resource_row',4]
        ereturn scalar resource_cmg_hierarchy_bytes =                ///
            `resource_components'[`resource_row',5]
        ereturn scalar resource_routed_solver_bytes =                ///
            `resource_components'[`resource_row',5]
        ereturn scalar resource_phase_scratch_bytes =                ///
            `resource_components'[`resource_row',6]
        ereturn scalar resource_sort_temp_bytes =                    ///
            `resource_components'[`resource_row',7]
        ereturn scalar resource_solve_ahead_bytes =                  ///
            `resource_components'[`resource_row',8]
        ereturn scalar resource_output_bytes =                       ///
            `resource_components'[`resource_row',9]
        ereturn scalar resource_preserve_bytes =                     ///
            `resource_components'[`resource_row',10]
        ereturn scalar resource_runtime_resident_bytes =             ///
            `resource_components'[`resource_row',11]
        ereturn scalar resource_select_peak_bytes =                 ///
            `resource_forecasts'[`resource_row',1]
        ereturn scalar resource_transition_peak_bytes =             ///
            `resource_forecasts'[`resource_row',2]
        ereturn scalar resource_numerical_peak_bytes =              ///
            `resource_forecasts'[`resource_row',3]
        ereturn scalar resource_restore_peak_bytes =                ///
            `resource_forecasts'[`resource_row',4]
        ereturn scalar resource_peak_bytes =                        ///
            `resource_forecasts'[`resource_row',5]
        ereturn scalar resource_memory_headroom =                   ///
            `resource_forecasts'[`resource_row',6]
        ereturn scalar resource_mem_admit_bytes =                   ///
            `resource_forecasts'[`resource_row',7]
        ereturn scalar resource_wall_upper_seconds =                ///
            `resource_forecasts'[`resource_row',8]
        ereturn scalar resource_wall_headroom =                     ///
            `resource_forecasts'[`resource_row',9]
        ereturn scalar resource_wall_admit_seconds =                ///
            `resource_forecasts'[`resource_row',10]
        ereturn scalar resource_hard_mem_bytes =                    ///
            `resource_forecasts'[`resource_row',11]
        ereturn scalar resource_hard_wall_seconds =                 ///
            `resource_forecasts'[`resource_row',12]
        ereturn scalar resource_memory_admitted =                   ///
            `resource_forecasts'[`resource_row',13]
        ereturn scalar resource_wall_admitted =                     ///
            `resource_forecasts'[`resource_row',14]
        ereturn scalar resource_admitted =                          ///
            `resource_forecasts'[`resource_row',15]
        ereturn scalar resource_physical_scale =                    ///
            `resource_scaling'[1,5]
        ereturn scalar resource_rng_lev_calls =                     ///
            `resource_scaling'[1,7]
        ereturn scalar resource_rng_target_calls =                  ///
            `resource_scaling'[1,8]
        ereturn scalar resource_rng_call_seconds =                  ///
            `resource_scaling'[1,9]
        ereturn scalar resource_rng_total_calls =                   ///
            `resource_scaling'[1,10]
        ereturn scalar resource_rng_wall_seconds =                  ///
            `resource_scaling'[1,11]
        ereturn scalar memory_forecast_bytes =                      ///
            `resource_forecasts'[`resource_row',5]
        ereturn scalar coefficient_cells = `diagnostic_coefficient_cells'
        ereturn scalar target_strata = `target_strata'
        ereturn scalar row_cell_compression =                       ///
            `N_retained'/`diagnostic_coefficient_cells'
        ereturn scalar units_per_cell =                             ///
            `diagnostic_deletion_units'/`diagnostic_coefficient_cells'
        ereturn scalar leverage_batch = `leverage_batch'
        ereturn scalar target_batch = `target_batch'
        ereturn scalar compression_seconds = `compression_seconds'
        ereturn scalar life_transition_seconds = `life_transition_seconds'
        ereturn scalar life_work_seconds = `life_work_seconds'
        ereturn scalar life_restore_seconds = `life_restore_seconds'
        ereturn scalar life_sample_restored = `life_sample_restored'
        ereturn scalar life_preserve_forced_disk =                  ///
            `life_preserve_forced_disk'
        ereturn scalar life_mem_before_bytes = `life_mem_before_bytes'
        ereturn scalar life_mem_cleared_bytes = `life_mem_cleared_bytes'
        ereturn scalar life_mem_work_bytes = `life_mem_work_bytes'
        ereturn scalar life_mem_restored_bytes = `life_mem_restored_bytes'
        ereturn scalar rng_master_seed = `seed'
        ereturn scalar rng_leverage_probe_first =                   ///
            `rng_leverage_probe_first'
        ereturn scalar rng_leverage_probe_last =                    ///
            `rng_leverage_probe_last'
        ereturn scalar rng_target_probe_first =                     ///
            `rng_target_probe_first'
        ereturn scalar rng_target_probe_last =                      ///
            `rng_target_probe_last'
        ereturn scalar residual_acceptance_tolerance =              ///
            max(1e-11,10*`tolerance')
        ereturn matrix resource_components = `resource_components'
        ereturn matrix resource_forecasts = `resource_forecasts'
        ereturn matrix resource_scaling = `resource_scaling'
        if "`engine_selected'" == "compressed" {
            matrix colnames `scale_receipt' = coefficient_cells     ///
                deletion_units target_strata reciprocal_residual    ///
                residual_gate target_identity_residual              ///
                complete_residual stored_rows physical_observations ///
                numerical_seconds rng_seconds
            ereturn scalar correction_reciprocal_residual =        ///
                `scale_receipt'[1,4]
            ereturn scalar target_identity_residual =              ///
                `scale_receipt'[1,6]
            ereturn scalar complete_residual_max =                 ///
                `scale_receipt'[1,7]
            ereturn scalar rng_seconds = `scale_receipt'[1,11]
            ereturn matrix scale_receipt = `scale_receipt'
        }
        else {
            ereturn scalar correction_reciprocal_residual = .
            ereturn scalar target_identity_residual =              ///
                `generic_identity_resid'
            ereturn scalar complete_residual_max =                ///
                `generic_complete_resid'
            ereturn scalar rng_seconds = .
        }
        ereturn local engine_requested "`engine_requested'"
        ereturn local engine_selected "`engine_selected'"
        ereturn local fastpath_status "`fastpath_status'"
        ereturn local fastpath_message `"`fastpath_message'"'
        ereturn local resource_status "`resource_status'"
        ereturn local resource_message `"`resource_message'"'
        ereturn local resource_peak_phase "`resource_peak_phase'"
        ereturn local life_method "`life_method'"
        ereturn local rng_contract "`rng_contract'"
        ereturn local rng_implementation "`rng_implementation'"
        ereturn local rng_call_shape "`rng_call_shape'"
        ereturn local rng_runtime "`rng_runtime'"
        ereturn local rng_leverage_domain "`rng_leverage_domain'"
        ereturn local rng_target_domain "`rng_target_domain'"
        ereturn local residual_normalization                        ///
            "l2_rhs_or_absolute_zero_rhs"
        ereturn local quotient_convention "full_firm_zero_sum"
        ereturn local grounding_convention                          ///
            "last_firm_zero_after_quotient_with_grounded_equation_checked"
        ereturn local scale_status = cond(                          ///
            "`engine_selected'" == "compressed",                 ///
            "EXPERIMENTAL_SCALE_ENGINE", "GENERAL_ENGINE")
    }
    else {
        ereturn scalar memory_forecast_bytes = 0
        ereturn scalar schur_seconds = 0
        ereturn scalar preconditioner_apply_seconds = 0
        ereturn scalar pcg_seconds = 0
        ereturn scalar solver_backend_seconds = 0
        ereturn scalar solver_schur_actions = 0
        ereturn scalar solver_schur_batches = 0
        ereturn scalar solver_precond_applications = 0
        ereturn scalar solver_precond_batches = 0
    }
    ereturn scalar batch = `batch'
    ereturn scalar batch_memory_budget_bytes = ///
        `batch_memory_budget_bytes'
    ereturn scalar batch_column_forecast_bytes = ///
        `batch_column_forecast_bytes'
    ereturn scalar batch_physical_column_bytes = ///
        `batch_physical_column_bytes'
    ereturn scalar batch_scratch_forecast_bytes = ///
        `batch_scratch_forecast_bytes'
    ereturn scalar memory_gib = `memory_gib'
    ereturn scalar active_processors = `active_processors'
    ereturn scalar seed = `seed'
    ereturn scalar tolerance = `tolerance'
    ereturn scalar maxiter = `maxiter'
    ereturn scalar physical_limit = `physical_limit'
    ereturn scalar numerical_mcse_available = ///
        ("`selected_algorithm'" == "jla")
    ereturn local cmd "fevc"
    ereturn local cmdline `"fevc `0'"'
    ereturn local version "0.5.0-alpha.1"
    ereturn local model "linear"
    ereturn local correction_method "kss"
    ereturn local backend_requested "`backend_requested'"
    ereturn local backend_selected "`backend_selected'"
    ereturn local backend_routing_reason `"`backend_routing_reason'"'
    ereturn scalar backend_option_supplied = `backend_supplied'
    ereturn scalar backend_fallback = `backend_fallback'
    ereturn local backend_fallback_reason "`backend_fallback_reason'"
    ereturn local backend_fallback_phase "`backend_fallback_phase'"
    ereturn local rng_requested "`rng_requested'"
    ereturn local rng_selected "`rng_selected'"
    ereturn scalar rng_option_supplied = `rng_supplied'
    ereturn local algorithm "`selected_algorithm'"
    ereturn local batch_requested "`batch_requested'"
    ereturn local batch_routing_reason `"`batch_routing_reason'"'
    ereturn local preconditioner_requested "`preconditioner'"
    ereturn local preconditioner_selected "`selected_preconditioner'"
    ereturn local routing_reason `"`routing_reason'"'
    ereturn local fallback_status "`fallback_status'"
    ereturn local fallback_message `"`fallback_message'"'
    ereturn local deletion "`deletion'"
    ereturn local nuisance "`nuisance'"
    ereturn local stayers "`stayers'"
    ereturn local target_population = cond("`deletion'" == "match", ///
        cond("`stayers'"=="both",                                 ///
            "retained movers plus eligible attached stayers",      ///
            "movers"), "retained observations")
    ereturn local sample_selection = cond("`deletion'" == "match", ///
        "MOVERS_DELETION_MULTIGRAPH_FIXED_POINT",                  ///
        "MATLAB_LEAVEONEWORKER_COMPONENT")
    ereturn local connectedness_status = cond("`deletion'" == "match", ///
        "DELETION_UNIT_BRIDGE_FREE", "LEAVE_ONE_WORKER_CONNECTED")
    ereturn local frequency_convention "literal physical copies"
    ereturn local probe_order = cond("`probeorder'" == "", ///
        "observed IDs, outcome, controls, and per-copy target mass", ///
        "observed IDs, outcome, controls, target mass, and optional tie-breaker")
    ereturn local targetweight_convention ///
        "explicit stored-row mass; default physical-observation mass"
    ereturn local inference = cond(`inference_requested',        ///
        "`inference'", "not implemented")
    ereturn local inference_method = cond("`inference'"=="none", ///
        cond(`project_supplied',                                 ///
            "exact block cross-fit projection",                  ///
            "not requested"),                                  ///
        "MATLAB-compatible target-specific binned local-linear variance approximation")
    _fevc_exact_inference_model_post `inference' `inferencemodel_supplied'
    ereturn local inference_deletion = cond(`inference_requested', ///
        cond(`project_supplied',                                  ///
            "exact `deletion' deletion; stayers `stayers'",      ///
            "exact observation deletion"), "not requested")
    ereturn local inference_covariance = cond("`inference'"=="none", ///
        cond(`project_supplied',                                  ///
            "symmetrized block covariance",                      ///
            "not posted"), "full joint component covariance")
    ereturn local inference_rng = cond("`inference'"!="none",   ///
        "guarded Stata RNG with caller state restoration",       ///
        cond(`project_supplied',"not used; deterministic exact projection", ///
            "not requested"))
    ereturn local numerical_error = cond("`selected_algorithm'" == "exact", ///
        "deterministic dense numerical backend", "conditional probe MCSE")
    ereturn local performance_profile_api "PREP-RHS-PERF-V1"
    ereturn local deletion_rank_certificate =                    ///
        cond("`selected_algorithm'"=="exact",                    ///
            "dense Woodbury plus direct rank gate",              ///
            cond(`control_count'>0,                               ///
                "full-fit within-cell trace and direct factor gates", ///
                "FE graph and spectral JLA gate"))
    ereturn local status = cond("`inference'"=="q1",              ///
        cond(`project_supplied',"KSS_Q1_AND_PROJECTION_INFERENCE", ///
            "KSS_Q1_INFERENCE"),                                  ///
        cond("`inference'"=="highrank",                           ///
            cond(`project_supplied',                               ///
                "KSS_HIGHRANK_AND_PROJECTION_INFERENCE",          ///
                "KSS_HIGHRANK_INFERENCE"),                        ///
            cond(`project_supplied',"KSS_PROJECTION_INFERENCE",   ///
                cond("`engine_selected'"=="compressed" &          ///
                    "`selected_algorithm'"=="jla",                ///
                    "KSS_SCALE_EXPERIMENTAL_POINT_ESTIMATES",     ///
                    "KSS_POINT_ESTIMATES_ONLY"))))
    if "`inference'"=="q1" {
        if `q1computed'<4 ereturn local status "KSS_Q1_PARTIAL"
    }

    quietly timer off $VCKSS_STAGE_VALIDATION_TIMER
    quietly timer list $VCKSS_STAGE_VALIDATION_TIMER
    local validation_seconds = r(t$VCKSS_STAGE_VALIDATION_TIMER)
    ereturn scalar sample_selection_seconds = `sample_selection_seconds'
    ereturn scalar validation_seconds = `validation_seconds'

    if "`nodisplay'" == "" _fevc_display
end

program define _fevc_rust_abort, eclass
    version 18.0
    syntax , RC(integer) [HANDLE(real 0) PHASE(string) NORELEASE]

    if `rc' == 1 {
        if `handle' > 0 & "`norelease'" == "" {
            capture quietly fevc_rust release `handle'
        }
        capture quietly fevc_rust clear
        ereturn clear
        exit 1
    }

    capture quietly fevc_rust lasterror
    if _rc {
        local native_code = .
        local native_status "NATIVE_ERROR_UNAVAILABLE"
        local native_detail "The Rust failure occurred, but its structural error receipt could not be retrieved."
    }
    else {
        local native_code = r(native_error_code)
        local native_status `"`r(native_error_status)'"'
        local native_detail `"`r(native_error_detail)'"'
    }
    local public_rc = `rc'
    // Preserve the established public Mata status for a deletion identifier
    // that crosses worker-firm coordinates.  The native canonicalizer uses
    // the broader typed INVALID_IDENTIFIER family internally.
    if `"`native_status'"'=="INVALID_IDENTIFIER" &                 ///
        strpos(`"`native_detail'"',"deletion identifier")>0 {
        local native_status "CROSS_COORDINATE_MATCH"
    }
    if `"`native_status'"'=="RESOURCE_LIMIT" &                    ///
        strpos(`"`native_detail'"',"exact_limit()")>0 {
        local native_status "EXACT_SIZE_LIMIT"
        local public_rc = 198
    }
    if `"`native_status'"'=="RESOURCE_LIMIT" &                    ///
        strpos(`"`native_detail'"',"deletion block")>0 {
        local native_status "BLOCK_SIZE_LIMIT"
        local public_rc = 198
    }
    if `"`native_status'"'=="RESOURCE_LIMIT" &                    ///
        strpos(`"`native_detail'"',"physical-frequency total")>0 {
        local native_status "PHYSICAL_TOTAL_LIMIT"
        local public_rc = 498
    }
    if `"`native_status'"'=="SINGULAR_INFORMATION" &              ///
        "`phase'"=="solve_jla" {
        local native_status "SINGULAR_NUISANCE_BLOCK"
    }
    if `"`native_status'"'=="INVERSE_RESIDUAL_FAILED" &           ///
        strpos(`"`native_detail'"',"control_basis_gram")>0 {
        local native_status "AMBIGUOUS_CONTROL_BASIS"
    }
    if `"`native_status'"'=="GRAPH_UNIDENTIFIED" &                ///
        strpos(`"`native_detail'"',"tie for largest-component")>0 {
        local native_status "AMBIGUOUS_LARGEST_COMPONENT"
    }
    // A Stata-side failure can occur after native preparation (for example,
    // while copying the retained mask or returned matrices).  In that case
    // the native error slot is still the cleared OK receipt.  Preserve the
    // original Stata rc, clean the registry, and do not manufacture a typed
    // native failure with status OK.
    if !missing(`native_code') & `native_code' == 0 &          ///
        "`native_status'" == "OK" {
        if `handle' > 0 & "`norelease'" == "" {
            capture quietly fevc_rust release `handle'
        }
        capture quietly fevc_rust clear
        ereturn clear
        exit `rc'
    }
    if `handle' > 0 & "`norelease'" == "" {
        capture quietly fevc_rust release `handle'
    }
    capture quietly fevc_rust clear
    quietly _vckss_post_failure "`native_status'" `"`native_detail'"'
    ereturn scalar native_error_code = `native_code'
    ereturn local native_error_phase "`phase'"
    ereturn local backend_requested "rust"
    ereturn local backend_selected ""
    ereturn local backend_routing_reason                       ///
        "explicit Rust lifecycle failed; no fallback is permitted"
    ereturn local rng_requested `"${VCKSS_ROUTE_RNG_REQUESTED}"'
    ereturn local rng_selected ""
    ereturn scalar backend_option_supplied = 1
    ereturn scalar rng_option_supplied =                       ///
        real("${VCKSS_ROUTE_RNG_SUPPLIED}")
    exit `public_rc'
end

program define _fevc_rust_finally
    version 18.0

    local active_handle = 0
    capture quietly fevc_rust snapshot
    if !_rc local active_handle = r(handle)
    if `active_handle' > 0 {
        capture quietly fevc_rust release `active_handle'
    }
    capture quietly fevc_rust clear
    capture quietly fevc_rust snapshot
    if _rc exit _rc
    if r(state) != 0 | r(handle) != 0 exit 498
end

program define _vckss_request_signature, rclass
    version 18.0
    args request_schema algorithm_code deletion_code nuisance_code ///
        route_code rng_code controls_count frequency_code engine_code ///
        batch_code stayers_code target_code deletionsource_code       ///
        probeorder_code wallseconds_code physical_limit

    // Reproduce the native fixed-domain FNV-1a identity receipt with two
    // exact 32-bit limbs.  Stata doubles represent every intermediate below
    // exactly; no rounded 64-bit integer is formed.
    local signature_hi = 3421674724
    local signature_lo = 2216829733
    local signature_bytes 86 67 75 83 83 45 82 69 81 85 69 83 84 ///
        45 67 65 80 65 66 73 76 73 84 89 45 86
    if `request_schema' == 2 local signature_bytes `signature_bytes' 50
    else local signature_bytes `signature_bytes' 49
    local signature_values `request_schema' `algorithm_code'       ///
        `deletion_code' `nuisance_code' `route_code' `rng_code'    ///
        `controls_count' `frequency_code'
    if `request_schema' == 2 {
        local signature_values `signature_values' `engine_code'    ///
            `batch_code' `stayers_code' `target_code'              ///
            `deletionsource_code' `probeorder_code' `wallseconds_code'
    }
    foreach value in `signature_values' {
        forvalues byte_index = 0/3 {
            local signature_bytes `signature_bytes' ///
                `=mod(floor(`value'/(256^`byte_index')),256)'
        }
    }
    if `request_schema' == 2 {
        forvalues byte_index = 0/7 {
            local signature_bytes `signature_bytes'              ///
                `=mod(floor(`physical_limit'/(256^`byte_index')),256)'
        }
    }
    foreach byte of local signature_bytes {
        local low_byte = mod(`signature_lo',256)
        local xor_byte = 0
        forvalues bit = 0/7 {
            local left_bit = mod(floor(`low_byte'/(2^`bit')),2)
            local right_bit = mod(floor(`byte'/(2^`bit')),2)
            if `left_bit' != `right_bit' {
                local xor_byte = `xor_byte' + 2^`bit'
            }
        }
        local xor_lo = `signature_lo' - `low_byte' + `xor_byte'
        local low_product = `xor_lo' * 435
        local signature_hi = mod(`signature_hi'*435 +              ///
            floor(`low_product'/4294967296) +                      ///
            mod(`xor_lo'*256,4294967296),4294967296)
        local signature_lo = mod(`low_product',4294967296)
    }
    return scalar signature_hi = `signature_hi'
    return scalar signature_lo = `signature_lo'
end

program define _vckss_rexact, eclass sortpreserve
    version 18.0
    args depvar worker firm deletionvar frequency target touse nscope   ///
        ncomplete nstayers nstayerrows probesrequested batchnumeric   ///
        seedrequested tolerancerequested maxiterrequested memorygib   ///
        enginerequested backendsupplied rngsupplied rngrequested      ///
        deletionidsupplied rustcoreflags rustsupportflags nodisplay   ///
        deletionmode nuisance exactlimit ranktol blocktol             ///
        blocksizelimit physicallimit preconditionerrequested          ///
        batchrequested controls capstruct capabi capschema            ///
        capsupported capreason capprofile capalgorithm capdeletion    ///
        capnuisance caproute caprng capcontrols capfrequency          ///
        capsignaturehi capsignaturelo

    foreach input in `depvar' `worker' `firm' `deletionvar'           ///
        `frequency' `target' `touse' `controls' {
        confirm numeric variable `input'
    }
    local control_count : word count `controls'

    capture quietly fevc_rust clear
    if _rc {
        local failure_rc = _rc
        capture noisily _fevc_rust_abort, rc(`failure_rc') phase(clear_entry)
        exit _rc
    }

    tempvar rust_keep
    capture noisily _fevc_rust_public_call prepare `worker' `firm'    ///
        `deletionvar' `depvar' `frequency' `target' `controls'        ///
        if `touse', cleanup generate(`rust_keep') memorygib(`memorygib') ///
        deletion(`deletionmode')
    if _rc {
        local failure_rc = _rc
        capture noisily _fevc_rust_abort, rc(`failure_rc') phase(prepare)
        exit _rc
    }
    local handle = r(handle)
    local p_input = r(input_rows)
    local p_retained = r(retained_rows)
    local p_workers = r(workers)
    local p_firms = r(firms)
    local p_cells = r(cells)
    local p_units = r(deletion_units)
    local p_strata = r(target_strata)
    local p_target = r(target_weight_sum)
    local p_mem_limit = r(memory_limit_bytes)
    local p_input_copy = r(caller_copy_bytes)
    local p_prep_peak = r(preparation_peak_forecast_bytes)
    local p_resident = r(prepared_resident_bytes)
    local p_controls = r(controls_count)
    local g_input_rows = r(graph_input_rows)
    local g_keep_rows = r(graph_retained_rows)
    local g_input_mass = r(graph_input_physical_mass)
    local g_keep_mass = r(graph_retained_physical_mass)
    local g_init_comp = r(graph_initial_components)
    local g_max_comp = r(graph_maximum_components)
    local g_init_rows = r(graph_initial_component_rows)
    local g_mover_rows = r(graph_mover_input_rows)
    local g_init_edges = r(graph_initial_deletion_edges)
    local g_keep_edges = r(graph_retained_deletion_edges)
    local g_degree_removed = r(graph_degree_workers_removed)
    local g_art_removed = r(graph_artic_workers_removed)
    local g_bridge_units = r(graph_bridge_units_removed)
    local g_bridge_rows = r(graph_bridge_rows_removed)
    local g_degree_iters = r(graph_degree_iterations)
    local g_art_iters = r(graph_articulation_iterations)
    local g_bridge_iters = r(graph_bridge_iterations)
    local g_fixed_iters = r(graph_fixed_point_iterations)

    quietly summarize `frequency' if `touse', meanonly
    local input_physical = r(sum)
    quietly replace `touse' = `touse' & `rust_keep'
    quietly count if `touse'
    local retained_count = r(N)
    quietly summarize `frequency' if `touse', meanonly
    local retained_physical = r(sum)
    quietly summarize `target' if `touse', meanonly
    local retained_target = r(sum)

    local preparation_receipts_ok = 1
    foreach receipt in p_input p_retained p_workers p_firms p_cells   ///
        p_units p_strata p_mem_limit p_input_copy p_prep_peak        ///
        p_resident p_controls g_input_rows g_keep_rows g_input_mass  ///
        g_keep_mass g_init_comp g_max_comp g_init_rows g_mover_rows  ///
        g_init_edges g_keep_edges g_degree_removed g_art_removed     ///
        g_bridge_units g_bridge_rows g_degree_iters g_art_iters      ///
        g_bridge_iters g_fixed_iters {
        if missing(``receipt'') | ``receipt'' < 0 |                  ///
            ``receipt'' != floor(``receipt'') {
            local preparation_receipts_ok = 0
        }
    }
    local expected_input_copy = `p_input'*(6+`control_count')*8
    local expected_prep_peak = `expected_input_copy' +               ///
        `p_input'*768 + `p_input'*`control_count'*32 + 4096
    if missing(`handle') | `handle' <= 0 | `handle' != floor(`handle') | ///
        missing(`p_target') | `p_target' <= 0 |                      ///
        missing(`retained_target') | `retained_target' <= 0 {
        local preparation_receipts_ok = 0
    }
    if `preparation_receipts_ok' {
        local preparation_receipts_ok =                              ///
            `p_input' > 0 & `p_retained' > 0 &                      ///
            `p_workers' > 0 & `p_firms' > 1 & `p_cells' > 0 &      ///
            `p_units' > 0 & `p_strata' > 0 &                        ///
            `p_controls' == `control_count' &                       ///
            `p_input_copy' == `expected_input_copy' &               ///
            `p_prep_peak' == `expected_prep_peak' &                 ///
            `p_resident' > 0 & `p_input' == `ncomplete' &           ///
            `p_input_copy'+`p_resident' <= `p_mem_limit' &          ///
            `p_prep_peak' <= `p_mem_limit' &                        ///
            `g_input_rows' == `ncomplete' &                         ///
            `g_input_mass' == `input_physical' &                    ///
            `p_retained' == `retained_count' &                      ///
            `g_keep_rows' == `retained_count' &                     ///
            `g_keep_mass' == `retained_physical' &                  ///
            `g_input_rows' >= `g_keep_rows' &                       ///
            `g_input_mass' >= `g_keep_mass' &                       ///
            `g_init_comp' > 0 & `g_max_comp' >= `g_init_comp' &    ///
            `g_max_comp' <= `g_input_rows' &                        ///
            `g_init_rows' > 0 & `g_init_rows' <= `g_input_rows' &  ///
            `g_init_rows' >= `g_mover_rows' &                       ///
            `g_mover_rows' >= `g_keep_rows' &                       ///
            `g_init_edges' > 0 & `g_init_edges' >= `g_keep_edges' & ///
            `g_degree_iters' <= `g_degree_removed' &                ///
            `g_art_iters' <= `g_art_removed' &                      ///
            (`g_degree_iters' == 0) == (`g_degree_removed' == 0) &  ///
            (`g_art_iters' == 0) == (`g_art_removed' == 0) &        ///
            abs(`p_target'-`retained_target') <=                    ///
                1e-10*max(1,abs(`p_target'))
    }
    if `preparation_receipts_ok' & "`deletionmode'" == "match" {
        local preparation_receipts_ok =                              ///
            `g_keep_edges' == `p_units' &                           ///
            `g_bridge_iters' <= `g_bridge_units' &                  ///
            (`g_bridge_iters' == 0) == (`g_bridge_units' == 0) &    ///
            (`g_bridge_iters' == 0) == (`g_bridge_rows' == 0) &     ///
            `g_bridge_units' <= `g_init_edges' &                    ///
            `g_bridge_rows' >= `g_bridge_units' &                   ///
            `g_bridge_rows' <= `g_mover_rows' &                     ///
            `g_fixed_iters' == `g_degree_iters'+`g_art_iters'+      ///
                `g_bridge_iters'
    }
    if `preparation_receipts_ok' & "`deletionmode'" == "observation" {
        local preparation_receipts_ok =                              ///
            `p_units' == `retained_physical' &                      ///
            `g_mover_rows' == `g_init_rows' &                       ///
            `g_bridge_units' == 0 & `g_bridge_rows' == 0 &          ///
            `g_bridge_iters' == 0 &                                ///
            `g_fixed_iters' == `g_degree_iters'+`g_art_iters'
    }
    if !`preparation_receipts_ok' {
        capture quietly fevc_rust release `handle'
        capture quietly fevc_rust clear
        quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"      ///
            "Rust exact preparation receipts did not reconcile with the validated Stata sample."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "preparation_reconcile"
        ereturn local backend_requested "rust"
        ereturn local backend_selected ""
        ereturn local rng_requested "`rngrequested'"
        ereturn local rng_selected ""
        ereturn scalar rust_core_ready_flags = `rustcoreflags'
        ereturn scalar rust_support_flags = `rustsupportflags'
        exit 498
    }
    capture noisily _fevc_rust_public_call solve `handle',          ///
        seed(`seedrequested') probes(`probesrequested')              ///
        leveragebatch(`batchnumeric') targetbatch(`batchnumeric')    ///
        route(exact) tolerance(`tolerancerequested')                 ///
        maxiter(`maxiterrequested') algorithm(exact)                 ///
        deletion(`deletionmode') nuisance(`nuisance')                ///
        exactlimit(`exactlimit') blocksizelimit(`blocksizelimit')    ///
        ranktolerance(`ranktol') blocktolerance(`blocktol')
    if _rc {
        local failure_rc = _rc
        capture noisily _fevc_rust_abort, rc(`failure_rc')          ///
            handle(`handle') phase(solve)
        exit _rc
    }

    capture noisily _fevc_rust_public_call result `handle'
    if _rc {
        local failure_rc = _rc
        capture noisily _fevc_rust_abort, rc(`failure_rc')          ///
            handle(`handle') phase(result_export)
        exit _rc
    }
    tempname raw_results
    matrix `raw_results' = r(result)
    local r_seed = r(seed)
    local r_probes = r(probes)
    local r_lev_acc = r(leverage_probes_accepted)
    local r_tgt_acc = r(target_probes_accepted)
    local r_req_route = r(requested_route)
    local r_sel_route = r(selected_route)
    local r_fallback = r(solver_fallback)
    local r_fallback_err = r(solver_fallback_error)
    local r_dimension = r(solver_dimension)
    local r_lev_batch = r(leverage_batch_width)
    local r_tgt_batch = r(target_batch_width)
    local r_rank_tol = r(rank_tolerance)
    local r_block_tol = r(block_tolerance)
    local r_full_tol = r(full_residual_tolerance)
    local r_full_route = r(full_fit_route)
    local r_full_iter = r(full_fit_iterations)
    local r_full_red = r(full_fit_reduced_residual)
    local r_full_complete = r(full_fit_complete_residual)
    local r_full_zero = r(full_fit_zero_rhs)
    local r_lev_rhs = r(leverage_rhs_count)
    local r_tgt_rhs = r(target_rhs_count)
    local r_max_red = r(max_reduced_residual)
    local r_max_complete = r(max_complete_residual)
    local r_max_lev = r(max_leverage)
    local r_max_recip = r(max_reciprocal_residual)
    local r_accounting = r(accounting_residual)
    local r_top_hi = r(topology_checksum_hi)
    local r_top_lo = r(topology_checksum_lo)
    local r_rng = r(rng_contract_code)
    local r_rhs_rows = r(rhs_receipt_rows)
    local r_rhs_copy = r(caller_result_copy_bytes)
    local r_algorithm_req = r(requested_algorithm_code)
    local r_algorithm_sel = r(selected_algorithm_code)
    local r_deletion = r(deletion_mode_code)
    local r_nuisance = r(nuisance_mode_code)
    local r_parameters = r(parameters)
    local r_full_parameters = r(full_parameters)
    local r_correction_parameters = r(correction_parameters)
    local r_info_rcond = r(information_rcond)
    local r_inverse_relres = r(inverse_relative_residual)
    local r_exact_peak = r(exact_peak_forecast_bytes)
    local r_exact_flags = r(exact_diagnostic_flags)
    local r_working_fit = r(working_fit_complete_residual)
    local r_inverse_sqrt = r(inverse_sqrt_relative_residual)
    local r_maker = r(maker_relative_residual)
    local r_control_relres = r(control_basis_relative_residual)
    local r_control_forward = r(control_basis_forward_error)
    local r_rank_gap = r(deletion_rank_gap)
    local r_firm_zero = r(firm_zero_sum_residual)
    local r_fit_peak = r(fit_peak_forecast_bytes)
    local r_correction_peak = r(correction_peak_forecast_bytes)
    local r_actual_accounting = r(actual_accounting_residual)
    local r_rss = r(weighted_rss)
    local r_mem_limit = r(memory_limit_bytes)
    local r_input_copy = r(caller_copy_bytes)
    local r_prep_peak = r(preparation_peak_forecast_bytes)
    local r_resident = r(prepared_resident_bytes)
    local r_solver_setup = r(solver_setup_forecast_bytes)
    local r_lev_phase = r(leverage_phase_forecast_bytes)
    local r_tgt_phase = r(target_phase_forecast_bytes)
    local r_result_bytes = r(result_forecast_bytes)
    local r_solve_peak = r(solve_peak_forecast_bytes)
    local r_command_peak = r(command_peak_forecast_bytes)

    local expected_full_parameters = `p_workers'+`p_firms'-1+`control_count'
    local expected_parameters = `expected_full_parameters'
    if "`nuisance'" == "fixedoffset" {
        local expected_parameters = `expected_parameters'-`control_count'
    }
    local expected_deletion_code = cond("`deletionmode'"=="match",1,2)
    local expected_nuisance_code = cond("`nuisance'"=="joint",1,2)
    local expected_fit_tolerance = max(1e-11,10*`tolerancerequested')
    local expected_exact_flags = 497
    if "`deletionmode'" == "match" {
        local expected_exact_flags = `expected_exact_flags' + 6
    }
    if `control_count' > 0 {
        local expected_exact_flags = `expected_exact_flags' + 8
    }
    local inverse_gate = max(1e-10,100*`ranktol')
    local receipt_epsilon = 4096*c(epsdouble)
    local result_receipts_ok = 1
    foreach receipt in r_seed r_probes r_lev_acc r_tgt_acc r_req_route ///
        r_sel_route r_fallback r_fallback_err r_dimension r_lev_batch ///
        r_tgt_batch r_full_route r_full_iter r_full_zero r_lev_rhs    ///
        r_tgt_rhs r_top_hi r_top_lo r_rng                             ///
        r_rhs_rows r_rhs_copy r_algorithm_req r_algorithm_sel         ///
        r_deletion r_nuisance r_parameters r_full_parameters          ///
        r_correction_parameters r_exact_peak r_exact_flags r_fit_peak ///
        r_correction_peak r_mem_limit r_input_copy                    ///
        r_prep_peak r_resident r_solver_setup r_lev_phase r_tgt_phase ///
        r_result_bytes r_solve_peak r_command_peak {
        if missing(``receipt'') | ``receipt'' < 0 |                  ///
            ``receipt'' != floor(``receipt'') {
            local result_receipts_ok = 0
        }
    }
    foreach receipt in r_rank_tol r_block_tol r_full_tol r_full_red   ///
        r_full_complete r_working_fit r_max_red r_max_complete        ///
        r_max_lev r_max_recip r_accounting r_info_rcond               ///
        r_inverse_relres r_inverse_sqrt r_maker r_control_relres      ///
        r_control_forward r_rank_gap r_firm_zero                      ///
        r_actual_accounting r_rss {
        if missing(``receipt'') local result_receipts_ok = 0
    }
    if rowsof(`raw_results') != 4 | colsof(`raw_results') != 4 {
        local result_receipts_ok = 0
    }
    if `result_receipts_ok' {
        local result_receipts_ok =                                  ///
            `r_seed' == 0 & `r_probes' == 0 &                       ///
            `r_lev_acc' == 0 & `r_tgt_acc' == 0 &                  ///
            `r_lev_batch' == 0 & `r_tgt_batch' == 0 &              ///
            `r_req_route' == 1 & `r_sel_route' == 1 &              ///
            `r_fallback' == 0 & `r_fallback_err' == 0 &            ///
            `r_dimension' == `expected_parameters' &               ///
            `r_rank_tol' == `ranktol' & `r_block_tol' == `blocktol' & ///
            abs(`r_full_tol'-`expected_fit_tolerance') <=           ///
                `receipt_epsilon'*max(1,`expected_fit_tolerance') & ///
            `r_full_route' == 1 & `r_full_iter' == 0 &              ///
            `r_full_zero' == 0 &                                   ///
            `r_lev_rhs' == 0 & `r_tgt_rhs' == 0 &                  ///
            `r_rng' == 0 & `r_rhs_rows' == 0 & `r_rhs_copy' == 0 & ///
            `r_algorithm_req' == 1 & `r_algorithm_sel' == 1 &      ///
            `r_deletion' == `expected_deletion_code' &             ///
            `r_nuisance' == `expected_nuisance_code' &             ///
            `r_parameters' == `expected_parameters' &              ///
            `r_full_parameters' == `expected_full_parameters' &    ///
            `r_correction_parameters' == `expected_parameters' &   ///
            `r_input_copy' == `p_input_copy' &                      ///
            `r_prep_peak' == `p_prep_peak' &                       ///
            `r_resident' == `p_resident' &                         ///
            `r_mem_limit' == `p_mem_limit' &                       ///
            `r_solver_setup' == 0 & `r_lev_phase' == 0 &           ///
            `r_tgt_phase' == 0 & `r_result_bytes' > 0 &            ///
            `r_exact_flags' == `expected_exact_flags' &             ///
            `r_fit_peak' > 0 & `r_correction_peak' > 0 &            ///
            `r_exact_peak' == max(`r_fit_peak',`r_correction_peak') & ///
            `r_solve_peak' == `r_exact_peak' &                      ///
            `r_command_peak' == max(`r_prep_peak',`r_solve_peak') & ///
            `r_command_peak' <= `r_mem_limit' &                    ///
            `r_top_hi' <= 4294967295 & `r_top_lo' <= 4294967295 & ///
            `r_info_rcond' > 0 & `r_info_rcond' <= 1 &             ///
            `r_inverse_relres' >= 0 &                              ///
            `r_inverse_relres' <= `inverse_gate' &                 ///
            `r_full_red' >= 0 & `r_full_complete' >= 0 &           ///
            `r_working_fit' >= 0 &                                ///
            `r_full_complete' <= `r_full_tol' &                    ///
            `r_working_fit' <= `r_full_tol' &                     ///
            abs(`r_full_red'-`r_full_complete') <=                 ///
                `receipt_epsilon'*max(1,abs(`r_full_complete')) &  ///
            abs(`r_max_complete'-max(`r_full_complete',            ///
                `r_working_fit')) <= `receipt_epsilon'*            ///
                max(1,abs(`r_max_complete')) &                     ///
            abs(`r_max_red'-`r_max_complete') <=                   ///
                `receipt_epsilon'*max(1,abs(`r_max_complete')) &   ///
            `r_max_lev' >= 0 & `r_max_lev' < 1 &                  ///
            `r_rank_gap' > `blocktol' & `r_firm_zero' >= 0 &      ///
            `r_firm_zero' <= `inverse_gate' &                     ///
            `r_accounting' == 0 & `r_actual_accounting' >= 0 &    ///
            `r_rss' >= 0
    }
    if `result_receipts_ok' & "`deletionmode'" == "match" {
        local result_receipts_ok =                                 ///
            `r_inverse_sqrt' >= 0 & `r_maker' >= 0 &              ///
            `r_maker' <= `inverse_gate' &                         ///
            abs(`r_max_recip'-`r_maker') <=                       ///
                `receipt_epsilon'*max(1,abs(`r_maker'))
    }
    if `result_receipts_ok' & "`deletionmode'" == "observation" {
        local result_receipts_ok =                                 ///
            `r_inverse_sqrt' == 0 & `r_maker' == 0 &               ///
            `r_max_recip' == 0
    }
    if `result_receipts_ok' & `control_count' > 0 {
        local result_receipts_ok =                                 ///
            `r_control_relres' >= 0 & `r_control_forward' >= 0 &  ///
            `r_control_forward' < 0.25
    }
    if `result_receipts_ok' & `control_count' == 0 {
        local result_receipts_ok =                                 ///
            `r_control_relres' == 0 & `r_control_forward' == 0
    }
    local result_gate = max(1e-10,4096*c(epsdouble))
    if `result_receipts_ok' {
        local result_identity_scale = 1
        local computed_actual_accounting = 0
        forvalues row = 1/4 {
            forvalues column = 1/4 {
                if missing(`raw_results'[`row',`column']) {
                    local result_receipts_ok = 0
                }
                else {
                    local result_identity_scale = max(              ///
                        `result_identity_scale',                    ///
                        abs(`raw_results'[`row',`column']))
                }
            }
        }
        forvalues row = 1/3 {
            local row_accounting = abs(`raw_results'[`row',4] -     ///
                `raw_results'[`row',1] - `raw_results'[`row',2] -  ///
                2*`raw_results'[`row',3])
            local computed_actual_accounting = max(                 ///
                `computed_actual_accounting',`row_accounting')
        }
        if `computed_actual_accounting' >                          ///
                `result_gate'*`result_identity_scale' |            ///
            `r_actual_accounting' >                                ///
                `result_gate'*`result_identity_scale' |            ///
            abs(`r_actual_accounting'-`computed_actual_accounting') > ///
                `receipt_epsilon'*`result_identity_scale' {
            local result_receipts_ok = 0
        }
        forvalues column = 1/4 {
            if `raw_results'[4,`column'] != 0 |                      ///
                abs(`raw_results'[1,`column']-                      ///
                    `raw_results'[2,`column']-                      ///
                    `raw_results'[3,`column']) >                    ///
                    `result_gate'*max(1,abs(`raw_results'[1,`column'])) {
                local result_receipts_ok = 0
            }
        }
    }
    if !`result_receipts_ok' {
        capture quietly fevc_rust release `handle'
        capture quietly fevc_rust clear
        quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"      ///
            "Rust exact result receipts did not reconcile with the submitted solve request."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "result_reconcile"
        ereturn local backend_requested "rust"
        ereturn local backend_selected ""
        ereturn local rng_requested "`rngrequested'"
        ereturn local rng_selected ""
        ereturn scalar rust_core_ready_flags = `rustcoreflags'
        ereturn scalar rust_support_flags = `rustsupportflags'
        exit 498
    }

    capture quietly _fevc_rust_public_call release `handle'
    if _rc {
        local failure_rc = _rc
        capture noisily _fevc_rust_abort, rc(`failure_rc')          ///
            handle(`handle') phase(release) norelease
        exit _rc
    }
    capture quietly fevc_rust snapshot
    if _rc | r(state) != 0 | r(handle) != 0 {
        capture quietly fevc_rust clear
        quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"      ///
            "Rust exact release did not return the native engine to idle state."
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
    matrix `capability_receipt' = (`capstruct',`capabi',`capschema', ///
        `capsupported',`capreason',`capprofile',`capalgorithm',      ///
        `capdeletion',`capnuisance',`caproute',`caprng',             ///
        `capcontrols',`capfrequency',`capsignaturehi',`capsignaturelo')
    matrix colnames `capability_receipt' = struct_size abi_version  ///
        request_schema supported reason profile algorithm deletion  ///
        nuisance route rng controls frequency signature_hi signature_lo

    tempname prep_boundary_counts
    local prep_deletion_groups = cond("`deletionmode'"=="observation",0,1)
    local exact_semantic_groups = (`control_count'>0)
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
    ereturn scalar numerical_mcse_available = 0
    ereturn scalar backend_option_supplied = `backendsupplied'
    ereturn scalar rng_option_supplied = `rngsupplied'
    ereturn scalar deletionid_option_supplied = `deletionidsupplied'
    ereturn local cmd "fevc"
    ereturn local version "0.5.0-alpha.1"
    ereturn local model "linear"
    ereturn local correction_method "kss"
    ereturn local backend_requested "rust"
    ereturn local backend_selected "rust"
    ereturn local backend_routing_reason                       ///
        "explicit Rust exact route; RNG is not applicable"
    ereturn local rng_requested "`rngrequested'"
    ereturn local rng_selected "NOT_APPLICABLE"
    ereturn local rng_contract "NOT_APPLICABLE"
    ereturn local rng_implementation "NOT_APPLICABLE"
    ereturn local rng_runtime "NOT_APPLICABLE"
    ereturn local rng_leverage_domain "NOT_APPLICABLE"
    ereturn local rng_target_domain "NOT_APPLICABLE"
    ereturn local algorithm "exact"
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
    ereturn local rust_capability_profile "EXACT_V1"
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
    if "`nodisplay'" == "" _fevc_display
end

program define _fevc_rust_public, eclass sortpreserve
    version 18.0
    args depvar worker firm deletion frequency target touse nscope ///
        ncomplete nstayers nstayerrows probes batch seed tolerance ///
        maxiter memorygib enginerequested backendsupplied          ///
        rngsupplied deletionidsupplied rustcoreflags               ///
        rustsupportflags nodisplay
    foreach input in `depvar' `worker' `firm' `deletion'          ///
        `frequency' `target' `touse' {
        confirm numeric variable `input'
    }

    capture quietly fevc_rust clear
    if _rc {
        local failure_rc = _rc
        capture noisily _fevc_rust_abort, rc(`failure_rc') phase(clear_entry)
        exit _rc
    }

    tempvar rust_keep
    capture noisily _fevc_rust_public_call prepare `worker' `firm' `deletion' ///
        `depvar' `frequency' `target' if `touse', cleanup             ///
        generate(`rust_keep') memorygib(`memorygib')
    if _rc {
        local failure_rc = _rc
        capture noisily _fevc_rust_abort, rc(`failure_rc') phase(prepare)
        exit _rc
    }
    local handle = r(handle)
    local p_input = r(input_rows)
    local p_retained = r(retained_rows)
    local p_workers = r(workers)
    local p_firms = r(firms)
    local p_cells = r(cells)
    local p_units = r(deletion_units)
    local p_strata = r(target_strata)
    local p_target = r(target_weight_sum)
    local p_mem_limit = r(memory_limit_bytes)
    local p_input_copy = r(caller_copy_bytes)
    local p_prep_peak = r(preparation_peak_forecast_bytes)
    local p_resident = r(prepared_resident_bytes)
    local g_input_rows = r(graph_input_rows)
    local g_keep_rows = r(graph_retained_rows)
    local g_input_mass = r(graph_input_physical_mass)
    local g_keep_mass = r(graph_retained_physical_mass)
    local g_init_comp = r(graph_initial_components)
    local g_max_comp = r(graph_maximum_components)
    local g_init_rows = r(graph_initial_component_rows)
    local g_mover_rows = r(graph_mover_input_rows)
    local g_init_edges = r(graph_initial_deletion_edges)
    local g_keep_edges = r(graph_retained_deletion_edges)
    local g_degree_removed = r(graph_degree_workers_removed)
    local g_art_removed = r(graph_artic_workers_removed)
    local g_bridge_units = r(graph_bridge_units_removed)
    local g_bridge_rows = r(graph_bridge_rows_removed)
    local g_degree_iters = r(graph_degree_iterations)
    local g_art_iters = r(graph_articulation_iterations)
    local g_bridge_iters = r(graph_bridge_iterations)
    local g_fixed_iters = r(graph_fixed_point_iterations)

    quietly summarize `frequency' if `touse', meanonly
    local input_physical = r(sum)
    quietly replace `touse' = `touse' & `rust_keep'
    quietly count if `touse'
    local retained_count = r(N)
    quietly summarize `frequency' if `touse', meanonly
    local retained_physical = r(sum)
    quietly summarize `target' if `touse', meanonly
    local retained_target = r(sum)
    local preparation_receipts_ok = 1
    foreach receipt in p_input p_retained p_workers p_firms p_cells ///
        p_units p_strata p_mem_limit p_input_copy p_prep_peak       ///
        p_resident g_input_rows g_keep_rows g_input_mass g_keep_mass ///
        g_init_comp g_max_comp g_init_rows g_mover_rows g_init_edges ///
        g_keep_edges g_degree_removed g_art_removed g_bridge_units   ///
        g_bridge_rows g_degree_iters g_art_iters g_bridge_iters      ///
        g_fixed_iters {
        if missing(``receipt'') | ``receipt'' < 0 |             ///
            ``receipt'' != floor(``receipt'') {
            local preparation_receipts_ok = 0
        }
    }
    if missing(`handle') | `handle' <= 0 | `handle' != floor(`handle') | ///
        missing(`p_target') | `p_target' <= 0 |                    ///
        missing(`retained_target') | `retained_target' <= 0 {
        local preparation_receipts_ok = 0
    }
    if `preparation_receipts_ok' {
        local preparation_receipts_ok =                         ///
            `p_input' > 0 & `p_retained' > 0 &                 ///
            `p_workers' > 0 & `p_firms' > 1 & `p_cells' > 0 & ///
            `p_units' > 0 & `p_strata' > 0 &                   ///
            `p_mem_limit' > 0 &                                ///
            `p_input_copy' == `p_input'*6*8 &                  ///
            `p_prep_peak' == `p_input_copy'+`p_input'*768+4096 & ///
            `p_resident' > 0 & `p_input' == `ncomplete' &      ///
            `p_input_copy'+`p_resident' <= `p_mem_limit' &     ///
            `p_prep_peak' <= `p_mem_limit' &                   ///
            `g_input_rows' == `ncomplete' &                    ///
            `g_input_mass' == `input_physical' &               ///
            `p_retained' == `retained_count' &                 ///
            `g_keep_rows' == `retained_count' &                ///
            `g_keep_mass' == `retained_physical' &             ///
            `g_input_rows' >= `g_keep_rows' &                  ///
            `g_input_mass' >= `g_keep_mass' &                  ///
            `g_init_comp' > 0 &                                ///
            `g_max_comp' >= `g_init_comp' &                    ///
            `g_max_comp' <= `g_input_rows' &                   ///
            `g_init_rows' > 0 & `g_init_rows' <= `g_input_rows' & ///
            `g_init_rows' >= `g_mover_rows' &                  ///
            `g_mover_rows' >= `g_keep_rows' &                  ///
            `g_init_edges' > 0 &                               ///
            `g_init_edges' >= `g_keep_edges' &                 ///
            `g_keep_edges' == `p_units' &                      ///
            `g_degree_iters' <= `g_degree_removed' &           ///
            `g_art_iters' <= `g_art_removed' &                 ///
            `g_bridge_iters' <= `g_bridge_units' &             ///
            (`g_degree_iters' == 0) == (`g_degree_removed' == 0) & ///
            (`g_art_iters' == 0) == (`g_art_removed' == 0) &   ///
            (`g_bridge_iters' == 0) == (`g_bridge_units' == 0) & ///
            (`g_bridge_iters' == 0) == (`g_bridge_rows' == 0) & ///
            `g_bridge_units' <= `g_init_edges' &               ///
            `g_bridge_rows' >= `g_bridge_units' &              ///
            `g_bridge_rows' <= `g_mover_rows' &                ///
            `g_fixed_iters' == `g_degree_iters'+`g_art_iters'+ ///
                `g_bridge_iters' &                             ///
            `g_fixed_iters' <= `g_input_rows'+`g_init_edges'+1 & ///
            abs(`p_target'-`retained_target') <=               ///
                1e-10*max(1,abs(`p_target'))
    }
    if !`preparation_receipts_ok' {
        capture quietly fevc_rust release `handle'
        capture quietly fevc_rust clear
        quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED" ///
            "Rust preparation receipts did not reconcile with the validated Stata sample."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "preparation_reconcile"
        ereturn local backend_requested "rust"
        ereturn local backend_selected ""
        ereturn local rng_requested "counter_v1"
        ereturn local rng_selected ""
        ereturn scalar backend_option_supplied = 1
        ereturn scalar rng_option_supplied = 1
        ereturn scalar rust_core_ready_flags = `rustcoreflags'
        ereturn scalar rust_support_flags = `rustsupportflags'
        exit 498
    }
    if `retained_physical' > 50000000 {
        capture quietly fevc_rust release `handle'
        capture quietly fevc_rust clear
        quietly _vckss_post_failure "PHYSICAL_TOTAL_LIMIT"       ///
            "Retained Rust physical mass exceeds the qualified public default."
        ereturn local backend_requested "rust"
        ereturn local backend_selected ""
        ereturn local backend_routing_reason                       ///
            "explicit Rust request exceeded the qualified physical-mass limit"
        ereturn local rng_requested "counter_v1"
        ereturn local rng_selected ""
        ereturn scalar backend_option_supplied = 1
        ereturn scalar rng_option_supplied = 1
        ereturn scalar rust_core_ready_flags = `rustcoreflags'
        ereturn scalar rust_support_flags = `rustsupportflags'
        exit 498
    }

    capture noisily _fevc_rust_public_call solve `handle', seed(`seed') ///
        probes(`probes') leveragebatch(`batch') targetbatch(`batch') ///
        route(diagonal) tolerance(`tolerance') maxiter(`maxiter')
    if _rc {
        local failure_rc = _rc
        capture noisily _fevc_rust_abort, rc(`failure_rc')       ///
            handle(`handle') phase(solve)
        exit _rc
    }

    capture noisily _fevc_rust_public_call result `handle'
    if _rc {
        local failure_rc = _rc
        capture noisily _fevc_rust_abort, rc(`failure_rc')       ///
            handle(`handle') phase(result_export)
        exit _rc
    }
    tempname raw_results rhs_native
    matrix `raw_results' = r(result)
    matrix `rhs_native' = r(rhs_receipts)
    local r_seed = r(seed)
    local r_probes = r(probes)
    local r_lev_acc = r(leverage_probes_accepted)
    local r_tgt_acc = r(target_probes_accepted)
    local r_req_route = r(requested_route)
    local r_sel_route = r(selected_route)
    local r_fallback = r(solver_fallback)
    local r_fallback_err = r(solver_fallback_error)
    local r_dimension = r(solver_dimension)
    local r_lev_batch = r(leverage_batch_width)
    local r_tgt_batch = r(target_batch_width)
    local r_rank_tol = r(rank_tolerance)
    local r_block_tol = r(block_tolerance)
    local r_full_tol = r(full_residual_tolerance)
    local r_full_route = r(full_fit_route)
    local r_full_iter = r(full_fit_iterations)
    local r_full_red = r(full_fit_reduced_residual)
    local r_full_complete = r(full_fit_complete_residual)
    local r_full_zero = r(full_fit_zero_rhs)
    local r_lev_rhs = r(leverage_rhs_count)
    local r_tgt_rhs = r(target_rhs_count)
    local r_max_red = r(max_reduced_residual)
    local r_max_complete = r(max_complete_residual)
    local r_max_lev = r(max_leverage)
    local r_max_recip = r(max_reciprocal_residual)
    local r_accounting = r(accounting_residual)
    local r_top_hi = r(topology_checksum_hi)
    local r_top_lo = r(topology_checksum_lo)
    local r_rng = r(rng_contract_code)
    local r_rhs_rows = r(rhs_receipt_rows)
    local r_rhs_copy = r(caller_result_copy_bytes)
    local r_rss = r(weighted_rss)
    local r_mem_limit = r(memory_limit_bytes)
    local r_input_copy = r(caller_copy_bytes)
    local r_prep_peak = r(preparation_peak_forecast_bytes)
    local r_resident = r(prepared_resident_bytes)
    local r_solver_setup = r(solver_setup_forecast_bytes)
    local r_lev_phase = r(leverage_phase_forecast_bytes)
    local r_tgt_phase = r(target_phase_forecast_bytes)
    local r_result_bytes = r(result_forecast_bytes)
    local r_solve_peak = r(solve_peak_forecast_bytes)
    local r_command_peak = r(command_peak_forecast_bytes)

    local result_receipts_ok = 1
    local roundoff_gate = 4096*c(epsdouble)
    local expected_full_tolerance = max(1e-11,10*`tolerance')

    foreach receipt in r_seed r_probes r_lev_acc r_tgt_acc r_req_route ///
        r_sel_route r_fallback r_fallback_err r_dimension r_lev_batch ///
        r_tgt_batch r_full_route r_full_iter r_full_zero r_lev_rhs    ///
        r_tgt_rhs r_top_hi r_top_lo r_rng r_rhs_rows r_rhs_copy       ///
        r_mem_limit r_input_copy r_prep_peak r_resident r_solver_setup ///
        r_lev_phase r_tgt_phase r_result_bytes r_solve_peak r_command_peak {
        if missing(``receipt'') | ``receipt'' < 0 |             ///
            ``receipt'' != floor(``receipt'') {
            local result_receipts_ok = 0
        }
    }
    foreach receipt in r_rank_tol r_block_tol r_full_tol r_full_red ///
        r_full_complete r_max_red r_max_complete r_max_lev        ///
        r_max_recip r_accounting r_rss {
        if missing(``receipt'') local result_receipts_ok = 0
    }
    if rowsof(`raw_results') != 4 | colsof(`raw_results') != 4 | ///
        rowsof(`rhs_native') != 1+3*`probes' |                   ///
        colsof(`rhs_native') != 8 {
        local result_receipts_ok = 0
    }
    if `result_receipts_ok' {
        local result_receipts_ok =                            ///
            `r_seed' == `seed' & `r_probes' == `probes' &    ///
            `r_lev_acc' == `probes' & `r_tgt_acc' == `probes' & ///
            `r_req_route' == 2 & `r_sel_route' == 2 &        ///
            `r_fallback' == 0 & `r_fallback_err' == 0 &      ///
            `r_dimension' == `p_firms'-1 &                   ///
            `r_lev_batch' == `batch' & `r_tgt_batch' == `batch' & ///
            `r_rank_tol' == 1e-10 & `r_block_tol' == 1e-10 & ///
            `r_full_tol' == `expected_full_tolerance' &      ///
            `r_full_route' == 2 & `r_full_iter' <= `maxiter' & ///
            inlist(`r_full_zero',0,1) &                      ///
            `r_lev_rhs' == `probes' & `r_tgt_rhs' == 2*`probes' & ///
            `r_rng' == 1 & `r_rhs_rows' == 1+3*`probes' &    ///
            `r_rhs_copy' == `r_rhs_rows'*14*8 &              ///
            `r_input_copy' == `p_input_copy' &               ///
            `r_prep_peak' == `p_prep_peak' &                 ///
            `r_resident' == `p_resident' &                   ///
            `r_mem_limit' == `p_mem_limit' &                 ///
            `r_solver_setup' > 0 &                            ///
            `r_lev_phase' > 0 & `r_tgt_phase' > 0 &          ///
            `r_result_bytes' >= `r_rhs_copy' &                ///
            `r_solve_peak' >= `r_resident'+`r_solver_setup'+ ///
                `r_result_bytes'+max(`r_lev_phase',`r_tgt_phase') & ///
            `r_command_peak' == max(`r_prep_peak',`r_solve_peak') & ///
            `r_command_peak' <= `r_mem_limit' &              ///
            `r_top_hi' <= 4294967295 & `r_top_lo' <= 4294967295 & ///
            `r_full_red' >= 0 & `r_full_red' <= `tolerance' & ///
            `r_full_complete' >= 0 &                         ///
            `r_full_complete' <= `r_full_tol' &              ///
            `r_max_red' >= 0 & `r_max_red' <= `tolerance' &  ///
            `r_max_complete' >= 0 & `r_max_complete' <= `r_full_tol' & ///
            `r_max_lev' >= 0 & `r_max_lev' < 1 &             ///
            `r_max_recip' >= 0 &                             ///
            `r_max_recip' <= max(1e-10,100*`r_rank_tol') &   ///
            `r_accounting' >= 0 & `r_accounting' <= `roundoff_gate' & ///
            `r_rss' >= 0
    }

    if `result_receipts_ok' {
        forvalues row = 1/4 {
            forvalues column = 1/4 {
                if missing(`raw_results'[`row',`column']) {
                    local result_receipts_ok = 0
                }
            }
        }
        forvalues column = 1/4 {
            if `raw_results'[4,`column'] < 0 {
                local result_receipts_ok = 0
            }
        }
    }

    local public_identity_max = 0
    if `result_receipts_ok' {
        forvalues row = 1/3 {
            local component_scale = max(1,abs(`raw_results'[`row',1]), ///
                abs(`raw_results'[`row',2]),abs(`raw_results'[`row',3]), ///
                abs(`raw_results'[`row',4]))
            local identity_residual = abs(`raw_results'[`row',4] - ///
                `raw_results'[`row',1] - `raw_results'[`row',2] - ///
                2*`raw_results'[`row',3])/`component_scale'
            local public_identity_max = max(`public_identity_max', ///
                `identity_residual')
            if `identity_residual' > `roundoff_gate' {
                local result_receipts_ok = 0
            }
        }
        forvalues column = 1/4 {
            local subtract_scale = max(1,abs(`raw_results'[1,`column']), ///
                abs(`raw_results'[2,`column']),abs(`raw_results'[3,`column']))
            if abs(`raw_results'[1,`column']-`raw_results'[2,`column']- ///
                `raw_results'[3,`column']) >                         ///
                `roundoff_gate'*`subtract_scale' {
                local result_receipts_ok = 0
            }
        }
        if `public_identity_max' > `r_accounting'+`roundoff_gate' {
            local result_receipts_ok = 0
        }
    }

    local rhs_max_iterations = 0
    local rhs_max_reduced = 0
    local rhs_max_complete = 0
    if `result_receipts_ok' {
        forvalues row = 1/`r_rhs_rows' {
            forvalues column = 1/8 {
                if missing(`rhs_native'[`row',`column']) {
                    local result_receipts_ok = 0
                }
            }
            if `rhs_native'[`row',1] != floor(`rhs_native'[`row',1]) | ///
                `rhs_native'[`row',2] != floor(`rhs_native'[`row',2]) | ///
                `rhs_native'[`row',3] != floor(`rhs_native'[`row',3]) | ///
                `rhs_native'[`row',4] != floor(`rhs_native'[`row',4]) | ///
                `rhs_native'[`row',5] != floor(`rhs_native'[`row',5]) | ///
                `rhs_native'[`row',8] != floor(`rhs_native'[`row',8]) | ///
                `rhs_native'[`row',5] < 0 |                         ///
                `rhs_native'[`row',5] > `maxiter' |                 ///
                `rhs_native'[`row',6] < 0 |                         ///
                `rhs_native'[`row',6] > `tolerance' |               ///
                `rhs_native'[`row',7] < 0 |                         ///
                `rhs_native'[`row',7] > `r_full_tol' |              ///
                !inlist(`rhs_native'[`row',8],0,1) |                 ///
                (`rhs_native'[`row',8] == 1 &                       ///
                    (`rhs_native'[`row',5] != 0 |                   ///
                     `rhs_native'[`row',6] != 0 |                   ///
                     `rhs_native'[`row',7] != 0)) {
                local result_receipts_ok = 0
            }
            local rhs_max_iterations = max(`rhs_max_iterations', ///
                `rhs_native'[`row',5])
            local rhs_max_reduced = max(`rhs_max_reduced',       ///
                `rhs_native'[`row',6])
            local rhs_max_complete = max(`rhs_max_complete',     ///
                `rhs_native'[`row',7])
        }
    }
    if `result_receipts_ok' {
        local result_receipts_ok =                          ///
            `rhs_native'[1,1] == 1 & `rhs_native'[1,2] == -1 & ///
            `rhs_native'[1,3] == 0 & `rhs_native'[1,4] == 2 & ///
            `rhs_native'[1,5] == `r_full_iter' &            ///
            `rhs_native'[1,8] == `r_full_zero' &            ///
            abs(`rhs_native'[1,6]-`r_full_red') <=          ///
                `roundoff_gate'*max(1,abs(`r_full_red')) &  ///
            abs(`rhs_native'[1,7]-`r_full_complete') <=     ///
                `roundoff_gate'*max(1,abs(`r_full_complete'))
    }
    if `result_receipts_ok' {
        forvalues probe = 0/`=`probes'-1' {
            local leverage_row = 2+`probe'
            local worker_row = 2+`probes'+2*`probe'
            local firm_row = `worker_row'+1
            local result_receipts_ok = `result_receipts_ok' & ///
                `rhs_native'[`leverage_row',1] == 2 &         ///
                `rhs_native'[`leverage_row',2] == `probe' &   ///
                `rhs_native'[`leverage_row',3] == 0 &         ///
                `rhs_native'[`leverage_row',4] == 2 &         ///
                `rhs_native'[`worker_row',1] == 3 &           ///
                `rhs_native'[`worker_row',2] == `probe' &     ///
                `rhs_native'[`worker_row',3] == 1 &           ///
                `rhs_native'[`worker_row',4] == 2 &           ///
                `rhs_native'[`firm_row',1] == 3 &             ///
                `rhs_native'[`firm_row',2] == `probe' &       ///
                `rhs_native'[`firm_row',3] == 2 &             ///
                `rhs_native'[`firm_row',4] == 2
        }
    }
    if `result_receipts_ok' {
        local result_receipts_ok =                          ///
            abs(`rhs_max_reduced'-`r_max_red') <=          ///
                `roundoff_gate'*max(1,abs(`r_max_red')) &  ///
            abs(`rhs_max_complete'-`r_max_complete') <=    ///
                `roundoff_gate'*max(1,abs(`r_max_complete'))
    }
    if !`result_receipts_ok' {
        capture quietly fevc_rust release `handle'
        capture quietly fevc_rust clear
        quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED" ///
            "Rust result receipts did not reconcile with the submitted solve request."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "result_reconcile"
        ereturn local backend_requested "rust"
        ereturn local backend_selected ""
        ereturn local rng_requested "counter_v1"
        ereturn local rng_selected ""
        ereturn scalar backend_option_supplied = 1
        ereturn scalar rng_option_supplied = 1
        ereturn scalar rust_core_ready_flags = `rustcoreflags'
        ereturn scalar rust_support_flags = `rustsupportflags'
        exit 498
    }

    capture quietly _fevc_rust_public_call release `handle'
    if _rc {
        local failure_rc = _rc
        capture noisily _fevc_rust_abort, rc(`failure_rc')       ///
            handle(`handle') phase(release) norelease
        exit _rc
    }
    capture quietly fevc_rust snapshot
    if _rc {
        local failure_rc = _rc
        capture noisily _fevc_rust_abort, rc(`failure_rc')       ///
            phase(release_snapshot)
        exit _rc
    }
    if r(state) != 0 | r(handle) != 0 {
        capture quietly fevc_rust clear
        quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED" ///
            "Rust release returned success but the engine did not return to idle state."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "release_snapshot"
        ereturn local backend_requested "rust"
        ereturn local backend_selected ""
        ereturn local rng_requested "counter_v1"
        ereturn local rng_selected ""
        ereturn scalar backend_option_supplied = 1
        ereturn scalar rng_option_supplied = 1
        ereturn scalar rust_core_ready_flags = `rustcoreflags'
        ereturn scalar rust_support_flags = `rustsupportflags'
        exit 498
    }

    tempname plugin correction corrected kss_return mcse decomposition
    matrix colnames `raw_results' = worker_variance firm_variance ///
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
        quietly generate double `target_sq' =                    ///
            `target'*(`depvar'-`target_mean')^2 if `touse'
        quietly summarize `target_sq' if `touse', meanonly
        local target_outcome_variance = r(sum)/`retained_target'
    }
    quietly summarize `depvar' [aw=`frequency'] if `touse', meanonly
    if !_rc & !missing(r(mean)) {
        local frequency_mean = r(mean)
        quietly generate double `frequency_sq' =                 ///
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

    matrix `decomposition' =                                    ///
        (`raw_results'[1,1], `raw_results'[2,1], `raw_results'[3,1] \ ///
         `raw_results'[1,2], `raw_results'[2,2], `raw_results'[3,2] \ ///
         2*`raw_results'[1,3], 2*`raw_results'[2,3], 2*`raw_results'[3,3] \ ///
         `raw_results'[1,4], `raw_results'[2,4], `raw_results'[3,4])
    matrix `decomposition' = `decomposition', J(4,4,.)
    if `target_outcome_variance' > 0 {
        forvalues component = 1/4 {
            matrix `decomposition'[`component',4] =              ///
                `decomposition'[`component',1]/`target_outcome_variance'
            matrix `decomposition'[`component',5] =              ///
                `decomposition'[`component',3]/`target_outcome_variance'
        }
    }
    if `decomposition'[4,1] > 0 {
        forvalues component = 1/4 {
            matrix `decomposition'[`component',6] =              ///
                `decomposition'[`component',1]/`decomposition'[4,1]
        }
    }
    if `decomposition'[4,3] > 0 {
        forvalues component = 1/4 {
            matrix `decomposition'[`component',7] =              ///
                `decomposition'[`component',3]/`decomposition'[4,3]
        }
    }
    matrix rownames `decomposition' = worker_variance firm_variance ///
        sorting_2covariance total_worker_firm
    matrix colnames `decomposition' = plugin bias_correction corrected ///
        plugin_share_outcome corrected_share_outcome              ///
        plugin_share_worker_firm corrected_share_worker_firm

    tempname rhs_public graph_receipt memory_receipt preparation_receipt
    matrix `rhs_public' = J(`r_rhs_rows',6,.)
    forvalues row = 1/`r_rhs_rows' {
        local native_phase = `rhs_native'[`row',1]
        local probe_zero = `rhs_native'[`row',2]
        local public_stage = cond(`native_phase'==1,2,cond(`native_phase'==2,4,5))
        local logical_rhs = cond(`probe_zero'<0,1,             ///
            cond(`native_phase'==3,2*`probe_zero'+             ///
                `rhs_native'[`row',3],`probe_zero'+1))
        local batch_start = cond(`probe_zero'<0,1,             ///
            floor(`probe_zero'/`batch')*`batch'+1)
        matrix `rhs_public'[`row',1] = `public_stage'
        matrix `rhs_public'[`row',2] = `batch_start'
        matrix `rhs_public'[`row',3] = `logical_rhs'
        matrix `rhs_public'[`row',4] = `rhs_native'[`row',5]
        matrix `rhs_public'[`row',5] = `rhs_native'[`row',7]
        local rhs_converged =                                 ///
            `rhs_native'[`row',4] == 2 &                     ///
            `rhs_native'[`row',5] >= 0 &                     ///
            `rhs_native'[`row',5] <= `maxiter' &             ///
            `rhs_native'[`row',6] >= 0 &                     ///
            `rhs_native'[`row',6] <= `tolerance' &           ///
            `rhs_native'[`row',7] >= 0 &                     ///
            `rhs_native'[`row',7] <= `r_full_tol' &          ///
            inlist(`rhs_native'[`row',8],0,1)
        matrix `rhs_public'[`row',6] = `rhs_converged'
    }
    matrix colnames `rhs_public' = stage batch_start rhs iterations ///
        relative_residual converged
    matrix `graph_receipt' = (`g_input_rows', `g_keep_rows',      ///
        `g_input_mass', `g_keep_mass', `g_init_comp', `g_max_comp', ///
        `g_init_rows', `g_mover_rows', `g_init_edges', `g_keep_edges', ///
        `g_degree_removed', `g_art_removed', `g_bridge_units',    ///
        `g_bridge_rows', `g_degree_iters', `g_art_iters',         ///
        `g_bridge_iters', `g_fixed_iters')
    matrix colnames `graph_receipt' = input_rows retained_rows input_mass ///
        retained_mass initial_components maximum_components initial_component_rows ///
        mover_input_rows initial_deletion_edges retained_deletion_edges ///
        insufficient_workers_removed articulation_workers_removed      ///
        bridge_units_removed bridge_rows_removed degree_iterations     ///
        articulation_iterations bridge_iterations fixed_point_iterations
    matrix `memory_receipt' = (`r_mem_limit', `r_input_copy', `r_rhs_copy', ///
        `r_prep_peak', `r_resident', `r_solver_setup', `r_lev_phase', ///
        `r_tgt_phase', `r_result_bytes', `r_solve_peak', `r_command_peak')
    matrix colnames `memory_receipt' = limit caller_input_copy caller_result_copy ///
        preparation_peak prepared_resident solver_setup leverage_phase ///
        target_phase result solve_peak command_peak
    matrix `preparation_receipt' = (`p_input', `p_retained',      ///
        `p_workers', `p_firms', `p_cells', `p_units', `p_strata', `p_target')
    matrix colnames `preparation_receipt' = input_rows retained_rows workers ///
        firms cells deletion_units target_strata target_weight_sum

    tempname prep_boundary_counts
    matrix `prep_boundary_counts' = (2,1,0,0,1,4,`p_input',2,     ///
        `retained_count',6,`retained_count')
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
    ereturn matrix solver_rhs_diagnostics = `rhs_public'
    ereturn matrix rust_rhs_receipts = `rhs_native'
    ereturn matrix rust_graph_receipt = `graph_receipt'
    ereturn matrix rust_memory_receipt = `memory_receipt'
    ereturn matrix rust_preparation_receipt = `preparation_receipt'
    ereturn matrix prep_boundary_counts = `prep_boundary_counts'
    ereturn local prep_boundary_counts_schema "PREP-BND-COUNTS-V1"
    ereturn scalar N_stored = `retained_count'
    ereturn scalar N_physical = `retained_physical'
    ereturn scalar N_requested = `nscope'
    ereturn scalar N_complete = `ncomplete'
    ereturn scalar N_retained = `retained_count'
    ereturn scalar N_mover_input = `g_mover_rows'
    ereturn scalar N_initial_component = `g_init_rows'
    ereturn scalar N_initial_component_dropped =                 ///
        `ncomplete'-`g_init_rows'
    ereturn scalar N_mover_dropped =                             ///
        `g_init_rows'-`g_mover_rows'
    ereturn scalar N_graph_dropped = `g_mover_rows'-`retained_count'
    ereturn scalar N_stayers = `nstayers'
    ereturn scalar N_stayer_rows = `nstayerrows'
    ereturn scalar worker_levels = `p_workers'
    ereturn scalar firm_levels = `p_firms'
    ereturn scalar parameters = `p_workers'+`p_firms'-1
    ereturn scalar full_parameters = `p_workers'+`p_firms'-1
    ereturn scalar correction_parameters = `p_workers'+`p_firms'-1
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
    ereturn scalar probes = `r_probes'
    ereturn scalar max_leverage = `r_max_lev'
    ereturn scalar inverse_relres = `r_max_recip'
    ereturn scalar solver_iterations = `rhs_max_iterations'
    ereturn scalar solver_max_residual = `r_max_complete'
    ereturn scalar complete_residual_max = `r_max_complete'
    ereturn scalar correction_reciprocal_residual = `r_max_recip'
    ereturn scalar target_identity_residual = `r_accounting'
    ereturn scalar weighted_rss = `r_rss'
    ereturn scalar target_outcome_variance = `target_outcome_variance'
    ereturn scalar regression_outcome_variance = `regression_outcome_variance'
    ereturn scalar residual_variance = `residual_variance'
    ereturn scalar full_model_explained_variance = `explained_variance'
    ereturn scalar full_model_explained_share = `explained_share'
    ereturn scalar tolerance = `tolerance'
    ereturn scalar maxiter = `maxiter'
    ereturn scalar seed = `seed'
    ereturn scalar batch = `batch'
    ereturn scalar leverage_batch = `batch'
    ereturn scalar target_batch = `batch'
    ereturn scalar memory_gib = `memorygib'
    ereturn scalar memory_forecast_bytes = `r_command_peak'
    ereturn scalar residual_acceptance_tolerance = `r_full_tol'
    ereturn scalar physical_limit = 50000000
    ereturn scalar active_processors = c(processors)
    ereturn scalar route_code = `r_sel_route'
    ereturn scalar route_planned_rhs = `r_rhs_rows'
    ereturn scalar rust_requested_route = `r_req_route'
    ereturn scalar rust_selected_route = `r_sel_route'
    ereturn scalar rust_solver_fallback = `r_fallback'
    ereturn scalar rust_solver_fallback_error = `r_fallback_err'
    ereturn scalar rust_solver_dimension = `r_dimension'
    ereturn scalar rust_full_fit_route = `r_full_route'
    ereturn scalar rust_full_fit_iterations = `r_full_iter'
    ereturn scalar rust_full_fit_reduced_residual = `r_full_red'
    ereturn scalar rust_full_fit_complete_residual = `r_full_complete'
    ereturn scalar rust_leverage_rhs_count = `r_lev_rhs'
    ereturn scalar rust_target_rhs_count = `r_tgt_rhs'
    ereturn scalar rust_max_reduced_residual = `r_max_red'
    ereturn scalar rust_leverage_probes_accepted = `r_lev_acc'
    ereturn scalar rust_target_probes_accepted = `r_tgt_acc'
    ereturn scalar rust_rank_tolerance = `r_rank_tol'
    ereturn scalar rust_block_tolerance = `r_block_tol'
    ereturn scalar rng_master_seed = `r_seed'
    ereturn scalar rng_leverage_probe_first = 1
    ereturn scalar rng_leverage_probe_last = `r_probes'
    ereturn scalar rng_target_probe_first = 1
    ereturn scalar rng_target_probe_last = `r_probes'
    ereturn scalar rust_core_ready_flags = `rustcoreflags'
    ereturn scalar rust_support_flags = `rustsupportflags'
    ereturn scalar rust_topology_checksum_hi = `r_top_hi'
    ereturn scalar rust_topology_checksum_lo = `r_top_lo'
    ereturn scalar rust_full_fit_zero_rhs = `r_full_zero'
    ereturn scalar rust_rng_contract_code = `r_rng'
    ereturn scalar numerical_mcse_available = 1
    ereturn scalar backend_option_supplied = `backendsupplied'
    ereturn scalar rng_option_supplied = `rngsupplied'
    ereturn scalar deletionid_option_supplied = `deletionidsupplied'
    ereturn local cmd "fevc"
    ereturn local version "0.5.0-alpha.1"
    ereturn local model "linear"
    ereturn local correction_method "kss"
    ereturn local backend_requested "rust"
    ereturn local backend_selected "rust"
    ereturn local backend_routing_reason "explicit strict Rust route with Counter-V1 RNG"
    ereturn local rng_requested "counter_v1"
    ereturn local rng_selected "counter_v1"
    ereturn local rng_contract "VCKSS-COUNTER-V1"
    ereturn local rng_implementation "stateless canonical Counter-V1 atoms"
    ereturn local rng_call_shape "one canonical atom plan per logical probe"
    ereturn local rng_runtime "native Rust Counter-V1"
    ereturn local rng_leverage_domain "leverage"
    ereturn local rng_target_domain "target"
    ereturn local algorithm "jla"
    ereturn local engine_requested "`enginerequested'"
    ereturn local engine_selected "compressed"
    ereturn local preconditioner_requested "diagonal"
    ereturn local preconditioner_selected "DIAGONAL"
    ereturn local routing_reason "explicit qualified diagonal PCG route"
    ereturn local fallback_status "NOT_NEEDED"
    ereturn local fallback_message "requested diagonal PCG route completed without fallback"
    ereturn local batch_requested "`batch'"
    ereturn local batch_routing_reason "caller supplied an explicit batch width"
    ereturn local deletion "match"
    ereturn local nuisance "joint"
    ereturn local target_population "movers"
    ereturn local sample_selection "MOVERS_DELETION_MULTIGRAPH_FIXED_POINT"
    ereturn local connectedness_status "DELETION_UNIT_BRIDGE_FREE"
    ereturn local frequency_convention "literal physical copies"
    ereturn local targetweight_convention ///
        "explicit stored-row mass; default physical-observation mass"
    ereturn local probe_order ///
        "observed IDs, outcome, controls, and per-copy target mass"
    ereturn local residual_normalization "l2_rhs_or_absolute_zero_rhs"
    ereturn local quotient_convention "full_firm_zero_sum"
    ereturn local grounding_convention ///
        "last_firm_zero_after_quotient_with_grounded_equation_checked"
    ereturn local inference "not implemented"
    ereturn local numerical_error "conditional probe MCSE"
    ereturn local deletion_rank_certificate "FE graph and spectral JLA gate"
    ereturn local route_api "VCKSS-NATIVE-ROUTE-V1"
    ereturn local status "KSS_SCALE_EXPERIMENTAL_POINT_ESTIMATES"
    if "`nodisplay'" == "" _fevc_display
end

program define _vckss_post_failure, eclass
    version 18.0
    args failure_status failure_detail
    quietly _fevc_failure_guidance "`failure_status'"
    local failure_reason `"`r(reason)'"'
    local failure_suggestion `"`r(suggestion)'"'
    if `"`failure_detail'"' == "" local failure_detail `"`failure_reason'"'
    ereturn clear
    ereturn local cmd "fevc"
    ereturn local version "0.5.0-alpha.1"
    ereturn local model "linear"
    ereturn local correction_method "kss"
    ereturn local status "WITHHELD"
    ereturn local withholding_status "`failure_status'"
    ereturn local withholding_detail `"`failure_detail'"'
    ereturn local withholding_reason `"`failure_reason'"'
    ereturn local withholding_suggestion `"`failure_suggestion'"'
    if "${VCKSS_ROUTE_METADATA_READY}" == "1" {
        ereturn local backend_requested ///
            `"${VCKSS_ROUTE_BACKEND_REQUESTED}"'
        ereturn local backend_selected ///
            `"${VCKSS_ROUTE_BACKEND_SELECTED}"'
        ereturn local backend_routing_reason ///
            `"${VCKSS_ROUTE_BACKEND_REASON}"'
        ereturn scalar backend_option_supplied = ///
            real("${VCKSS_ROUTE_BACKEND_SUPPLIED}")
        ereturn local rng_requested `"${VCKSS_ROUTE_RNG_REQUESTED}"'
        ereturn local rng_selected `"${VCKSS_ROUTE_RNG_SELECTED}"'
        ereturn scalar rng_option_supplied = ///
            real("${VCKSS_ROUTE_RNG_SUPPLIED}")
        ereturn scalar backend_fallback = ///
            real("${VCKSS_ROUTE_FALLBACK}")
        ereturn local backend_fallback_reason ///
            `"${VCKSS_ROUTE_FB_REASON}"'
        ereturn local backend_fallback_phase ///
            `"${VCKSS_ROUTE_FB_PHASE}"'
        if "${VCKSS_ROUTE_ALGORITHM_REQUESTED}" != "" {
            ereturn local algorithm "${VCKSS_ROUTE_ALGORITHM_REQUESTED}"
            ereturn local engine_requested "${VCKSS_ROUTE_ENGINE_REQUESTED}"
            ereturn local preconditioner_requested ///
                "${VCKSS_ROUTE_PRECOND_REQUESTED}"
            ereturn local deletion "${VCKSS_ROUTE_DELETION_REQUESTED}"
            ereturn local nuisance "${VCKSS_ROUTE_NUISANCE_REQUESTED}"
            ereturn scalar algorithm_option_supplied =             ///
                real("${VCKSS_ROUTE_ALGORITHM_SUPPLIED}")
            ereturn scalar engine_option_supplied =                ///
                real("${VCKSS_ROUTE_ENGINE_SUPPLIED}")
            ereturn scalar preconditioner_option_supplied =        ///
                real("${VCKSS_ROUTE_PRECOND_SUPPLIED}")
            ereturn scalar batch_option_supplied =                 ///
                real("${VCKSS_ROUTE_BATCH_SUPPLIED}")
            ereturn scalar stayers_option_supplied =               ///
                real("${VCKSS_ROUTE_STAYERS_SUPPLIED}")
            ereturn scalar deletionid_option_supplied =            ///
                real("${VCKSS_ROUTE_DELETIONID_SUPPLIED}")
        }
    }
end

program define _vckss_route_context_clear
    version 18.0
    foreach route_global in VCKSS_ROUTE_METADATA_READY          ///
        VCKSS_ROUTE_BACKEND_REQUESTED VCKSS_ROUTE_BACKEND_SELECTED ///
        VCKSS_ROUTE_BACKEND_REASON VCKSS_ROUTE_BACKEND_SUPPLIED ///
        VCKSS_ROUTE_FALLBACK VCKSS_ROUTE_FB_REASON               ///
        VCKSS_ROUTE_FB_PHASE                                     ///
        VCKSS_ROUTE_RNG_REQUESTED VCKSS_ROUTE_RNG_SELECTED      ///
        VCKSS_ROUTE_RNG_SUPPLIED VCKSS_ROUTE_ALGORITHM_REQUESTED ///
        VCKSS_ROUTE_ENGINE_REQUESTED VCKSS_ROUTE_PRECOND_REQUESTED ///
        VCKSS_ROUTE_DELETION_REQUESTED VCKSS_ROUTE_NUISANCE_REQUESTED ///
        VCKSS_ROUTE_ALGORITHM_SUPPLIED VCKSS_ROUTE_ENGINE_SUPPLIED ///
        VCKSS_ROUTE_PRECOND_SUPPLIED VCKSS_ROUTE_BATCH_SUPPLIED ///
        VCKSS_ROUTE_STAYERS_SUPPLIED VCKSS_ROUTE_DELETIONID_SUPPLIED {
        capture macro drop `route_global'
    }
end

program define _vckss_display_failure
    version 18.0
    di as error _newline "fevc could not compute the requested decomposition."
    di as error "Reason: " as text `"`e(withholding_reason)'"'
    if `"`e(withholding_detail)'"' != `"`e(withholding_reason)'"' {
        di as error "Detail: " as text `"`e(withholding_detail)'"'
    }
    di as error "What to try: " as text `"`e(withholding_suggestion)'"'
    di as error "Technical status: " as result `"`e(withholding_status)'"'
    di as text "See "                                           ///
        `"{help fevc##troubleshooting:help fevc, troubleshooting}."'
end

program define _vckss_stage_timer_ids, rclass
    version 18.0
    local timers
    forvalues id = 31/50 {
        quietly capture timer list `id'
        if missing(r(t`id')) local timers `timers' `id'
        local timer_count : word count `timers'
        if `timer_count' == 2 continue, break
    }
    local timer_count : word count `timers'
    if `timer_count' != 2 exit 498
    return scalar selection_timer = real(word("`timers'",1))
    return scalar validation_timer = real(word("`timers'",2))
end

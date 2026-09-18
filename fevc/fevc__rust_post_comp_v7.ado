*! version 0.5.0-rc.1 05sep2026
program define fevc__rust_post_comp_v7, eclass sortpreserve
    version 18.0
    args handle depvar frequency target touse nscope ncomplete nstayers  ///
        nstayerrows tolerance maxiter memorygib engine_requested       ///
        backendsupplied rngsupplied deletionidsupplied enginesupplied  ///
        algorithmsupplied preconditionsupplied batchsupplied           ///
        stayerssupplied rustcoreflags rustsupportflags nodisplay       ///
        nuisance physicallimit targetweightsupplied cmdline            ///
        preconditioner_requested batch_requested prepctx graphctx capctx fullcmg ///
        stayersmode
    if "`fullcmg'"=="" local fullcmg = 0
    if "`stayersmode'"=="" local stayersmode movers
    local expected_stayers = cond(lower(strtrim("`stayersmode'"))=="both",2,1)

    // The caller invokes this program immediately after the compressed V7
    // reconciler. Copy the complete validated return before any r-class work.
    local h_ok = r(ok)
    local h_detail `"`r(detail)'"'
    local h_family `"`r(result_family)'"'
    local h_exec_schema `"`r(execution_plan_schema)'"'
    foreach pair in expected_rhs_rows:h_rows rhs_max_iterations:h_itermax ///
        rhs_max_reduced:h_redmax rhs_max_complete:h_compmax             ///
        accounting_truth:h_accttruth seed:h_seed probes:h_probes        ///
        leverage_probes_accepted:h_levacc                               ///
        target_probes_accepted:h_tgtacc                                 ///
        requested_algorithm_code:h_algreq selected_algorithm_code:h_algsel ///
        requested_engine_code:h_engreq selected_engine_code:h_engsel    ///
        requested_route:h_rtreq selected_route:h_rtsel                  ///
        solver_fallback:h_fb solver_fallback_error:h_fberr              ///
        solver_dimension:h_dim leverage_batch_width:h_levbatch          ///
        target_batch_width:h_tgtbatch rng_contract_code:h_rng           ///
        rhs_receipt_rows:h_rhsrows caller_result_copy_bytes:h_resultcopy ///
        memory_limit_bytes:h_memlimit caller_copy_bytes:h_inputcopy      ///
        preparation_peak_bytes:h_preppeak                               ///
        prepared_resident_bytes:h_resident solver_setup_bytes:h_setup   ///
        leverage_phase_bytes:h_levphase target_phase_bytes:h_tgtphase   ///
        result_forecast_bytes:h_resultbytes solve_peak_bytes:h_solvepeak ///
        command_peak_bytes:h_cmdpeak full_fit_route:h_fullroute         ///
        full_fit_iterations:h_fulliter full_fit_reduced_residual:h_fullred ///
        full_fit_complete_residual:h_fullcomplete                       ///
        full_fit_zero_rhs:h_fullzero leverage_rhs_count:h_levrhs        ///
        target_rhs_count:h_tgtrhs max_reduced_residual:h_maxred         ///
        max_complete_residual:h_maxcomplete max_leverage:h_maxlev       ///
        max_reciprocal_residual:h_maxrecip accounting_residual:h_accounting ///
        actual_accounting_residual:h_actualacct weighted_rss:h_rss      ///
        rank_tolerance:h_ranktol block_tolerance:h_blocktol             ///
        full_residual_tolerance:h_fulltol deletion_mode_code:h_delcode  ///
        nuisance_mode_code:h_nuiscode parameters:h_parameters          ///
        full_parameters:h_fullparams correction_parameters:h_corrparams ///
        topology_checksum_hi:h_tophi topology_checksum_lo:h_toplo      ///
        rhs_receipt_schema:h_rhsschema capability_schema:h_capschema    ///
        capability_profile:h_capprof request_signature_hi:h_sighi      ///
        request_signature_lo:h_siglo batch_mode_code:h_batchmode       ///
        leverage_batch_mode_code:h_levmode target_batch_mode_code:h_tgtmode ///
        stayers_mode_code:h_staymode target_weight_mode_code:h_targetmode ///
        deletion_source_code:h_delsource probeorder_supplied:h_probeorder ///
        wallseconds_supplied:h_wallsup frequency_use_code:h_frequse    ///
        physical_limit:h_physlimit rhs_v2_copy_bytes:h_rhsv2copy       ///
        plan_struct:h_planstruct plan_schema:h_planschema              ///
        plan_route_schema:h_planrouteschema plan_resolved:h_planresolved ///
        plan_frozen:h_planfrozen plan_applicability:h_planapp          ///
        plan_algorithm_requested:h_planalgreq                          ///
        plan_algorithm_selected:h_planalgsel                           ///
        plan_engine_requested:h_planengreq plan_engine_selected:h_planengsel ///
        plan_route_requested:h_planrtreq plan_route_selected:h_planrtsel ///
        plan_route_fallback:h_planfb plan_route_error:h_planerr        ///
        plan_rhs:h_planrhs plan_full_dimension:h_planfulldim           ///
        plan_leverage_batch:h_planlevbatch plan_target_batch:h_plantgtbatch ///
        plan_batch_command:h_planbatchcmd plan_memory_command:h_planmemcmd ///
        counter_complete:h_counter pre_rng_hi:h_prernghi pre_rng_lo:h_prernglo ///
        wall_request_applicable:h_wallapp wallseconds_requested:h_wallreq ///
        wallseconds_forecast:h_wallforecast                            ///
        wallseconds_advisory:h_walladvisory wallseconds_margin:h_wallmargin {
        gettoken source localname : pair, parse(":")
        local localname = substr("`localname'",2,.)
        local `localname' = r(`source')
    }
    tempname raw_results rhs_native prep_receipt graph_receipt cap_all
    matrix `raw_results' = r(result)
    matrix `rhs_native' = r(rhs_receipts)
    matrix `prep_receipt' = `prepctx'
    matrix `graph_receipt' = `graphctx'
    matrix `cap_all' = `capctx'

    local context_ok = `h_ok'==1 & `"`h_family'"'=="compressed" &    ///
        `"`h_exec_schema'"'=="VCKSS-EXECUTION-PLAN-V1" &             ///
        rowsof(`prep_receipt')==1 & colsof(`prep_receipt')==13 &    ///
        rowsof(`graph_receipt')==1 & colsof(`graph_receipt')==18 &  ///
        rowsof(`cap_all')==1 & colsof(`cap_all')==24
    if !`context_ok' {
        capture quietly fevc__rust_public_call release `handle'
        capture quietly fevc_rust clear
        quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"       ///
            "The compressed V7 poster did not receive a complete validated context."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "compressed_context"
        ereturn local backend_selected ""
        ereturn local rng_selected ""
        ereturn scalar rust_core_ready_flags = `rustcoreflags'
        ereturn scalar rust_support_flags = `rustsupportflags'
        exit 498
    }

    local p_input       = `prep_receipt'[1,1]
    local p_retained    = `prep_receipt'[1,2]
    local p_workers     = `prep_receipt'[1,3]
    local p_firms       = `prep_receipt'[1,4]
    local p_cells       = `prep_receipt'[1,5]
    local p_units       = `prep_receipt'[1,6]
    local p_strata      = `prep_receipt'[1,7]
    local p_target      = `prep_receipt'[1,8]
    local p_controls    = `prep_receipt'[1,9]
    local p_memlimit    = `prep_receipt'[1,10]
    local p_inputcopy   = `prep_receipt'[1,11]
    local p_preppeak    = `prep_receipt'[1,12]
    local p_resident    = `prep_receipt'[1,13]

    local g_input_rows      = `graph_receipt'[1,1]
    local g_keep_rows       = `graph_receipt'[1,2]
    local g_input_mass      = `graph_receipt'[1,3]
    local g_keep_mass       = `graph_receipt'[1,4]
    local g_init_comp       = `graph_receipt'[1,5]
    local g_max_comp        = `graph_receipt'[1,6]
    local g_init_rows       = `graph_receipt'[1,7]
    local g_mover_rows      = `graph_receipt'[1,8]
    local g_init_edges      = `graph_receipt'[1,9]
    local g_keep_edges      = `graph_receipt'[1,10]
    local g_degree_removed  = `graph_receipt'[1,11]
    local g_art_removed     = `graph_receipt'[1,12]
    local g_bridge_units    = `graph_receipt'[1,13]
    local g_bridge_rows     = `graph_receipt'[1,14]
    local g_degree_iters    = `graph_receipt'[1,15]
    local g_art_iters       = `graph_receipt'[1,16]
    local g_bridge_iters    = `graph_receipt'[1,17]
    local g_fixed_iters     = `graph_receipt'[1,18]

    local cap_struct     = `cap_all'[1,1]
    local cap_abi        = `cap_all'[1,2]
    local cap_schema     = `cap_all'[1,3]
    local cap_supported  = `cap_all'[1,4]
    local cap_reason     = `cap_all'[1,5]
    local cap_profile    = `cap_all'[1,6]
    local cap_algorithm  = `cap_all'[1,7]
    local cap_deletion   = `cap_all'[1,8]
    local cap_nuisance   = `cap_all'[1,9]
    local cap_route      = `cap_all'[1,10]
    local cap_rng        = `cap_all'[1,11]
    local cap_controls   = `cap_all'[1,12]
    local cap_frequency  = `cap_all'[1,13]
    local cap_engine     = `cap_all'[1,14]
    local cap_batch      = `cap_all'[1,15]
    local cap_stayers    = `cap_all'[1,16]
    local cap_target     = `cap_all'[1,17]
    local cap_delsource  = `cap_all'[1,18]
    local cap_probeorder = `cap_all'[1,19]
    local cap_wallsup    = `cap_all'[1,20]
    local cap_physical   = `cap_all'[1,21]
    local cap_sighi      = `cap_all'[1,22]
    local cap_siglo      = `cap_all'[1,23]
    local cap_engdefer   = `cap_all'[1,24]

    confirm numeric variable `depvar'
    confirm numeric variable `frequency'
    confirm numeric variable `target'
    confirm numeric variable `touse'
    quietly count if `touse'
    local retained_count = r(N)
    quietly summarize `frequency' if `touse', meanonly
    local retained_physical = r(sum)
    quietly summarize `target' if `touse', meanonly
    local retained_target = r(sum)
    local nuisance_code = cond("`nuisance'"=="joint",1,2)
    local expected_input_copy = `p_input'*(6+`cap_probeorder')*8
    local expected_prep_peak = `expected_input_copy'+`p_input'*768+4096
    if `cap_probeorder'==1 local expected_prep_peak =               ///
        `expected_prep_peak' + `p_input'*16
    if `fullcmg'==1 local expected_prep_peak =                      ///
        `expected_prep_peak' + `p_input'*16
    local receipt_eps = 4096*c(epsdouble)

    local context_ok = inlist(`fullcmg',0,1) & `h_ok'==1 &          ///
        "`engine_requested'"=="auto" &                             ///
        inlist("`nuisance'","joint","fixedoffset") &                 ///
        `h_delcode'==1 & `h_nuiscode'==`nuisance_code' &            ///
        `h_engreq'==0 & `h_engsel'==1 & `h_rhsschema'==1 &          ///
        `h_capschema'==3 & `h_capprof'==4 &                          ///
        `h_parameters'==`p_workers'+`p_firms'-1 &                   ///
        `h_fullparams'==`h_parameters' & `h_corrparams'==`h_parameters' & ///
        `h_memlimit'==`p_memlimit' & `h_inputcopy'==`p_inputcopy' & ///
        `h_preppeak'==`p_preppeak' & `h_resident'==`p_resident' &   ///
        `h_physlimit'==`physicallimit' &                            ///
        `h_planapp'==2 & `h_planengreq'==0 & `h_planengsel'==1 &   ///
        `h_planrhs'==`h_rhsrows' & `h_planfulldim'==`h_dim' &       ///
        `h_planlevbatch'==`h_levbatch' & `h_plantgtbatch'==`h_tgtbatch' & ///
        `h_planbatchcmd'==`h_planmemcmd' & `h_planmemcmd'==`h_solvepeak' & ///
        `h_counter'==1 & `h_prernghi'==0 & `h_prernglo'==0 &       ///
        `p_controls'==0 & `p_input'==`ncomplete' &                  ///
        `p_retained'==`retained_count' & `p_workers'>0 & `p_firms'>1 & ///
        `p_cells'>0 & `p_units'>0 & `p_strata'>0 &                 ///
        `p_memlimit'==`h_memlimit' & `p_inputcopy'==`expected_input_copy' & ///
        (`p_preppeak'==`expected_prep_peak' | ///
            ("$VCKSS_MEMORY_ACTIVE"=="1" & `p_preppeak'>=`p_inputcopy')) & `p_resident'>0 &       ///
        `g_input_rows'==`ncomplete' & `g_keep_rows'==`retained_count' & ///
        `g_input_mass'>=`g_keep_mass' & `g_keep_mass'==`retained_physical' & ///
        `g_init_comp'>0 & `g_max_comp'>=`g_init_comp' &             ///
        `g_init_rows'>=`g_mover_rows' & `g_mover_rows'>=`g_keep_rows' & ///
        `g_init_edges'>=`g_keep_edges' & `g_keep_edges'==`p_units' & ///
        `g_degree_iters'<=`g_degree_removed' &                      ///
        `g_art_iters'<=`g_art_removed' & `g_bridge_iters'<=`g_bridge_units' & ///
        `g_fixed_iters'==`g_degree_iters'+`g_art_iters'+`g_bridge_iters' & ///
        `p_target'>0 & `retained_target'>0 &                        ///
        abs(`p_target'-`retained_target')<=                          ///
            1e-10*max(1,abs(`p_target')) &                          ///
        `nscope'>=`ncomplete' & `ncomplete'>=`retained_count' &     ///
        `nstayers'>=0 & `nstayerrows'>=0 &                          ///
        `cap_struct'==160 & `cap_abi'==1 & `cap_schema'==3 &        ///
        `cap_supported'==1 & `cap_reason'==0 & `cap_profile'==4 &   ///
        `cap_algorithm'==`h_algreq' & `cap_deletion'==1 &                    ///
        `cap_nuisance'==`nuisance_code' & `cap_route'==`h_rtreq' & ///
        `cap_rng'==1 & `cap_controls'==0 &                          ///
        `cap_frequency'==`h_frequse' & `cap_engine'==0 &            ///
        `cap_batch'==`h_batchmode' &                                ///
        `cap_stayers'==`expected_stayers' &                         ///
        `h_staymode'==`expected_stayers' &                          ///
        `cap_target'==`h_targetmode' & `cap_delsource'==`h_delsource' & ///
        inlist(`cap_probeorder',0,1) &                              ///
        `cap_probeorder'==`h_probeorder' &                           ///
        `cap_wallsup'==`h_wallsup' &                                ///
        `cap_physical'==`physicallimit' & `cap_sighi'==`h_sighi' &  ///
        `cap_siglo'==`h_siglo' & `cap_engdefer'==1
    if !`context_ok' {
        capture quietly fevc__rust_public_call release `handle'
        capture quietly fevc_rust clear
        quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"       ///
            "Compressed V7 public context did not reconcile with the validated result family."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "compressed_context_reconcile"
        ereturn local backend_selected ""
        ereturn local rng_selected ""
        ereturn scalar rust_core_ready_flags = `rustcoreflags'
        ereturn scalar rust_support_flags = `rustsupportflags'
        exit 498
    }

    capture quietly fevc__rust_public_call release `handle'
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
        quietly _vckss_post_failure "INTERNAL_INVARIANT_FAILED"       ///
            "Compressed V7 release succeeded without an idle native snapshot."
        ereturn scalar native_error_code = .
        ereturn local native_error_phase "release_snapshot"
        ereturn local backend_selected ""
        ereturn local rng_selected ""
        exit 498
    }

    tempname plugin correction corrected kss_return posted mcse decomposition
    matrix colnames `raw_results' = worker_variance firm_variance      ///
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
        quietly generate double `target_sq' =                         ///
            `target'*(`depvar'-`target_mean')^2 if `touse'
        quietly summarize `target_sq' if `touse', meanonly
        local target_outcome_variance = r(sum)/`retained_target'
    }
    quietly summarize `depvar' [aw=`frequency'] if `touse', meanonly
    if !_rc & !missing(r(mean)) {
        local frequency_mean = r(mean)
        quietly generate double `frequency_sq' =                      ///
            `frequency'*(`depvar'-`frequency_mean')^2 if `touse'
        quietly summarize `frequency_sq' if `touse', meanonly
        local regression_outcome_variance = r(sum)/`retained_physical'
    }
    local residual_variance = `h_rss'/`retained_physical'
    local explained_variance = `regression_outcome_variance'-`residual_variance'
    local explained_share = .
    if `regression_outcome_variance'>0 local explained_share =       ///
        `explained_variance'/`regression_outcome_variance'

    matrix `decomposition' =                                          ///
        (`raw_results'[1,1],`raw_results'[2,1],`raw_results'[3,1] \  ///
         `raw_results'[1,2],`raw_results'[2,2],`raw_results'[3,2] \  ///
         2*`raw_results'[1,3],2*`raw_results'[2,3],2*`raw_results'[3,3] \ ///
         `raw_results'[1,4],`raw_results'[2,4],`raw_results'[3,4])
    matrix `decomposition' = `decomposition',J(4,4,.)
    if `target_outcome_variance'>0 {
        forvalues component = 1/4 {
            matrix `decomposition'[`component',4] =                   ///
                `decomposition'[`component',1]/`target_outcome_variance'
            matrix `decomposition'[`component',5] =                   ///
                `decomposition'[`component',3]/`target_outcome_variance'
        }
    }
    if `decomposition'[4,1]>0 {
        forvalues component = 1/4 {
            matrix `decomposition'[`component',6] =                   ///
                `decomposition'[`component',1]/`decomposition'[4,1]
        }
    }
    if `decomposition'[4,3]>0 {
        forvalues component = 1/4 {
            matrix `decomposition'[`component',7] =                   ///
                `decomposition'[`component',3]/`decomposition'[4,3]
        }
    }
    matrix rownames `decomposition' = worker_variance firm_variance   ///
        sorting_2covariance total_worker_firm
    matrix colnames `decomposition' = plugin bias_correction corrected ///
        plugin_share_outcome corrected_share_outcome                  ///
        plugin_share_worker_firm corrected_share_worker_firm

    tempname rhs_public memory_receipt capability_receipt compressed_receipt
    tempname route_diagnostics
    matrix colnames `rhs_native' = phase probe side route iterations   ///
        reduced_residual complete_residual zero_rhs
    matrix `rhs_public' = J(`h_rows',6,.)
    forvalues row = 1/`h_rows' {
        local phase = `rhs_native'[`row',1]
        local probe = `rhs_native'[`row',2]
        local side = `rhs_native'[`row',3]
        local stage = cond(`phase'==1,2,cond(`phase'==2,4,5))
        local logical_rhs = cond(`probe'<0,1,cond(`phase'==3,          ///
            2*`probe'+`side',`probe'+1))
        local active_batch = cond(`phase'==2,`h_levbatch',            ///
            cond(`phase'==3,`h_tgtbatch',1))
        local batch_start = cond(`probe'<0,1,                         ///
            floor(`probe'/`active_batch')*`active_batch'+1)
        matrix `rhs_public'[`row',1] = `stage'
        matrix `rhs_public'[`row',2] = `batch_start'
        matrix `rhs_public'[`row',3] = `logical_rhs'
        matrix `rhs_public'[`row',4] = `rhs_native'[`row',5]
        matrix `rhs_public'[`row',5] = `rhs_native'[`row',7]
        matrix `rhs_public'[`row',6] = 1
    }
    matrix colnames `rhs_public' = stage batch_start rhs iterations  ///
        relative_residual converged

    matrix `memory_receipt' = (`h_memlimit',`h_inputcopy',`h_resultcopy', ///
        `h_rhsv2copy',`h_preppeak',`h_resident',`h_setup',`h_levphase', ///
        `h_tgtphase',`h_resultbytes',`h_solvepeak',`h_cmdpeak')
    matrix colnames `memory_receipt' = limit caller_input_copy result_copy ///
        rhs_v2_copy preparation_peak prepared_resident solver_setup  ///
        leverage_phase target_phase result solve_peak command_peak
    matrix `capability_receipt' = `cap_all'[1,1..23]
    matrix colnames `capability_receipt' = struct_size abi_version schema ///
        supported reason profile algorithm deletion nuisance route rng controls ///
        frequency engine batch stayers target deletion_source probeorder wall ///
        physical_limit signature_hi signature_lo
    matrix `compressed_receipt' = (`h_engreq',`h_engsel',`h_rtreq',`h_rtsel', ///
        `h_dim',`h_fullcomplete',`h_maxcomplete',`h_maxrecip',        ///
        `h_actualacct',`h_planapp',`h_counter')
    matrix colnames `compressed_receipt' = requested_engine selected_engine ///
        requested_route selected_route dimension full_fit_complete   ///
        max_complete max_reciprocal accounting plan_applicability counter_complete

    local native_batch = max(`h_levbatch',`h_tgtbatch')
    local native_batch_scratch = max(`h_levphase',`h_tgtphase')
    local native_batch_column = `native_batch_scratch'/`native_batch'
    matrix `route_diagnostics' = J(1,26,.)
    matrix `route_diagnostics'[1,1] = `h_planrhs'
    matrix `route_diagnostics'[1,2] = `h_planmemcmd'
    matrix `route_diagnostics'[1,13] = `h_rtsel'
    matrix `route_diagnostics'[1,25] = `h_planmemcmd'
    matrix colnames `route_diagnostics' = planned_rhs memory_bytes ///
        setup_seconds hierarchy_levels edge_complexity             ///
        vertex_complexity structural_bytes dense_factor_bytes      ///
        workers firms hybrid_vertices hybrid_edges route_code      ///
        predicted_vertices predicted_edges predicted_structural_bytes ///
        predicted_scratch_bytes reserved18 reserved19 reserved20   ///
        reserved21 hierarchy_seconds reserved23 reserved24         ///
        forecast_peak_bytes terminal_vertices

    local compression_columns = 6+`h_probeorder'
    tempname prep_boundary_counts
    matrix `prep_boundary_counts' = (2,1,0,0,1,4,`p_input',2,     ///
        `retained_count',`compression_columns',`retained_count')
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
    ereturn matrix rust_preparation_receipt = `prep_receipt'
    ereturn matrix rust_request_capability_receipt = `capability_receipt'
    ereturn matrix rust_compressed_receipt = `compressed_receipt'
    ereturn matrix route_diagnostics = `route_diagnostics'
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
    ereturn scalar parameters = `h_parameters'
    ereturn scalar full_parameters = `h_fullparams'
    ereturn scalar correction_parameters = `h_corrparams'
    ereturn scalar controls_count = 0
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
    ereturn scalar probes = `h_probes'
    ereturn scalar max_leverage = `h_maxlev'
    ereturn scalar information_rcond = .
    ereturn scalar inverse_relres = .
    ereturn scalar solver_iterations = `h_itermax'
    ereturn scalar solver_max_residual = `h_maxcomplete'
    ereturn scalar complete_residual_max = `h_maxcomplete'
    ereturn scalar correction_reciprocal_residual = `h_maxrecip'
    ereturn scalar target_identity_residual = `h_actualacct'
    ereturn scalar weighted_rss = `h_rss'
    ereturn scalar target_outcome_variance = `target_outcome_variance'
    ereturn scalar regression_outcome_variance = `regression_outcome_variance'
    ereturn scalar residual_variance = `residual_variance'
    ereturn scalar full_model_explained_variance = `explained_variance'
    ereturn scalar full_model_explained_share = `explained_share'
    ereturn scalar tolerance = `tolerance'
    ereturn scalar maxiter = `maxiter'
    ereturn scalar seed = `h_seed'
    ereturn scalar batch = `native_batch'
    ereturn scalar leverage_batch = `h_levbatch'
    ereturn scalar target_batch = `h_tgtbatch'
    ereturn scalar batch_memory_budget_bytes = `h_memlimit'
    ereturn scalar batch_column_forecast_bytes = `native_batch_column'
    ereturn scalar batch_physical_column_bytes = 0
    ereturn scalar batch_scratch_forecast_bytes = `native_batch_scratch'
    ereturn scalar memory_gib = `memorygib'
    ereturn scalar resource_peak_bytes = `h_cmdpeak'
    ereturn scalar memory_forecast_bytes = `h_cmdpeak'
    ereturn scalar residual_acceptance_tolerance = `h_fulltol'
    ereturn scalar physical_limit = `physicallimit'
    ereturn scalar physical_limit_applied = 1
    ereturn scalar active_processors = c(processors)
    ereturn scalar route_code = `h_rtsel'
    ereturn scalar route_planned_rhs = `h_rhsrows'
    ereturn scalar route_forecast_peak_bytes = `h_planmemcmd'
    ereturn scalar rust_requested_algorithm_code = `h_algreq'
    ereturn scalar rust_selected_algorithm_code = `h_algsel'
    ereturn scalar rust_requested_route = `h_rtreq'
    ereturn scalar rust_selected_route = `h_rtsel'
    ereturn scalar rust_solver_fallback = `h_fb'
    ereturn scalar rust_solver_fallback_error = `h_fberr'
    ereturn scalar rust_solver_dimension = `h_dim'
    ereturn scalar rust_full_fit_route = `h_fullroute'
    ereturn scalar rust_full_fit_iterations = `h_fulliter'
    ereturn scalar rust_full_fit_reduced_residual = `h_fullred'
    ereturn scalar rust_full_fit_complete_residual = `h_fullcomplete'
    ereturn scalar rust_full_fit_zero_rhs = `h_fullzero'
    ereturn scalar rust_leverage_rhs_count = `h_levrhs'
    ereturn scalar rust_target_rhs_count = `h_tgtrhs'
    ereturn scalar rust_rhs_receipt_schema = `h_rhsschema'
    ereturn scalar rust_result_copy_bytes = `h_resultcopy'
    ereturn scalar rust_max_reduced_residual = `h_maxred'
    ereturn scalar rust_max_complete_residual = `h_maxcomplete'
    ereturn scalar rust_leverage_probes_accepted = `h_levacc'
    ereturn scalar rust_target_probes_accepted = `h_tgtacc'
    ereturn scalar rust_rank_tolerance = `h_ranktol'
    ereturn scalar rust_block_tolerance = `h_blocktol'
    ereturn scalar rust_actual_accounting_residual = `h_actualacct'
    ereturn scalar rust_rhs_v2_copy_bytes = `h_rhsv2copy'
    ereturn scalar rust_core_ready_flags = `rustcoreflags'
    ereturn scalar rust_support_flags = `rustsupportflags'
    ereturn scalar rust_topology_checksum_hi = `h_tophi'
    ereturn scalar rust_topology_checksum_lo = `h_toplo'
    ereturn scalar rust_rng_contract_code = `h_rng'
    ereturn scalar rust_requested_engine_code = `h_engreq'
    ereturn scalar rust_selected_engine_code = `h_engsel'
    ereturn scalar rust_batch_mode_code = `h_batchmode'
    ereturn scalar rust_leverage_batch_mode_code = `h_levmode'
    ereturn scalar rust_target_batch_mode_code = `h_tgtmode'
    ereturn scalar rust_plan_struct_size = `h_planstruct'
    ereturn scalar rust_plan_schema = `h_planschema'
    ereturn scalar rust_plan_route_schema = `h_planrouteschema'
    ereturn scalar rust_plan_resolved = `h_planresolved'
    ereturn scalar rust_plan_frozen = `h_planfrozen'
    ereturn scalar rust_plan_applicability = `h_planapp'
    ereturn scalar rust_plan_algorithm_requested = `h_planalgreq'
    ereturn scalar rust_plan_algorithm_selected = `h_planalgsel'
    ereturn scalar rust_plan_engine_requested = `h_planengreq'
    ereturn scalar rust_plan_engine_selected = `h_planengsel'
    ereturn scalar rust_plan_route_requested = `h_planrtreq'
    ereturn scalar rust_plan_route_selected = `h_planrtsel'
    ereturn scalar rust_plan_route_fallback = `h_planfb'
    ereturn scalar rust_plan_route_error = `h_planerr'
    ereturn scalar rust_plan_rhs = `h_planrhs'
    ereturn scalar rust_plan_full_dimension = `h_planfulldim'
    ereturn scalar rust_plan_leverage_batch = `h_planlevbatch'
    ereturn scalar rust_plan_target_batch = `h_plantgtbatch'
    ereturn scalar rust_plan_solve_peak_bytes = `h_planmemcmd'
    ereturn scalar rust_counter_plan_complete = `h_counter'
    ereturn scalar rust_pre_rng_hi = `h_prernghi'
    ereturn scalar rust_pre_rng_lo = `h_prernglo'
    ereturn scalar rust_wall_request_applicable = `h_wallapp'
    ereturn scalar rust_wallseconds_supplied = `h_wallsup'
    ereturn scalar rust_wallseconds_requested = `h_wallreq'
    ereturn scalar rust_wallseconds_forecast = `h_wallforecast'
    ereturn scalar rust_wallseconds_advisory = `h_walladvisory'
    ereturn scalar rust_wallseconds_margin = `h_wallmargin'
    ereturn scalar rust_stayers_mode_code = `h_staymode'
    ereturn scalar rust_target_weight_mode_code = `h_targetmode'
    ereturn scalar rust_deletion_source_code = `h_delsource'
    ereturn scalar rust_probeorder_supplied = `h_probeorder'
    ereturn scalar rust_frequency_use_code = `h_frequse'
    ereturn scalar rust_solve_physical_limit = `h_physlimit'
    ereturn scalar rust_result_cap_schema = `h_capschema'
    ereturn scalar rust_result_cap_profile = `h_capprof'
    ereturn scalar rust_solve_signature_hi = `h_sighi'
    ereturn scalar rust_solve_signature_lo = `h_siglo'
    ereturn scalar rng_master_seed = `h_seed'
    ereturn scalar rng_leverage_probe_first = 1
    ereturn scalar rng_leverage_probe_last = `h_probes'
    ereturn scalar rng_target_probe_first = 1
    ereturn scalar rng_target_probe_last = `h_probes'
    ereturn scalar rust_cap_struct_size = `cap_struct'
    ereturn scalar rust_cap_abi_version = `cap_abi'
    ereturn scalar rust_cap_schema = `cap_schema'
    ereturn scalar rust_cap_supported = `cap_supported'
    ereturn scalar rust_cap_reason_code = `cap_reason'
    ereturn scalar rust_cap_profile_code = `cap_profile'
    ereturn scalar rust_cap_signature_hi = `cap_sighi'
    ereturn scalar rust_cap_signature_lo = `cap_siglo'
    ereturn scalar rust_cap_engine_deferred = `cap_engdefer'
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
    ereturn local version "0.5.0-rc.1"
    ereturn local model "linear"
    ereturn local correction_method "kss"
    ereturn local backend_requested "rust"
    ereturn local backend_selected "rust"
    ereturn local backend_routing_reason                         ///
        "explicit planned public engine-auto route selected compressed"
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
    ereturn local engine_selected "compressed"
    ereturn local preconditioner_requested "`preconditioner_requested'"
    ereturn local preconditioner_selected = cond(`h_rtsel'==1,"EXACT", ///
        cond(`h_rtsel'==3,"CMG","DIAGONAL"))
    ereturn local routing_reason "native planned compressed-JLA route"
    ereturn local fallback_status = cond(`h_fb',"CMG_TO_DIAGONAL", ///
        "NOT_NEEDED")
    ereturn local fallback_message = cond(`h_fb',                    ///
        "CMG setup failed before RNG and the permitted diagonal fallback completed", ///
        "compressed-JLA completed on the selected native route")
    ereturn local batch_requested "`batch_requested'"
    ereturn local batch_routing_reason = cond(`h_batchmode'==0,      ///
        "native planner selected independent phase widths",          ///
        "caller supplied the shared explicit phase width")
    ereturn local fastpath_status "ELIGIBLE"
    ereturn local deletion "match"
    ereturn local nuisance "`nuisance'"
    ereturn local target_population "movers"
    ereturn local sample_selection "MOVERS_DELETION_MULTIGRAPH_FIXED_POINT"
    ereturn local connectedness_status "DELETION_UNIT_BRIDGE_FREE"
    ereturn local frequency_convention "literal physical copies"
    ereturn local targetweight_convention                           ///
        "explicit stored-row mass; default physical-observation mass"
    ereturn local probe_order = cond(`h_probeorder',               ///
        "observed IDs, outcome, controls, target mass, and optional tie-breaker", ///
        "observed IDs, outcome, controls, and per-copy target mass")
    ereturn local residual_normalization "complete weighted model residual"
    ereturn local quotient_convention "full_firm_zero_sum"
    ereturn local grounding_convention                              ///
        "last_firm_zero_after_quotient_with_complete residual checked"
    ereturn local inference "not implemented"
    ereturn local numerical_error "conditional probe MCSE and certified solver residuals"
    ereturn local inverse_diagnostics "NOT_APPLICABLE"
    ereturn local deletion_rank_certificate                         ///
        "compressed match graph, quotient, and complete-model residual gates"
    ereturn local route_api "VCKSS-NATIVE-COMPRESSED-PLANNED-V4-V7"
    ereturn local result_family "compressed"
    ereturn local rust_capability_profile "PLANNED_V1"
    ereturn local execution_plan_schema "`h_exec_schema'"
    ereturn local rust_capability_reason "SUPPORTED"
    ereturn local status "KSS_SCALE_EXPERIMENTAL_POINT_ESTIMATES"
    if "`nodisplay'" == "" fevc__display
end

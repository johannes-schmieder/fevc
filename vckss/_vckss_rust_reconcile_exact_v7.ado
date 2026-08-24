*! version 0.4.0-dev 24aug2026
program define _vckss_rust_reconcile_exact_v7, rclass
    version 18.0
    args algreq engreq delcode nuiscode workers firms controls ranktol ///
        blocktol tolerance exactlimit memlimit inputcopy preppeak resident ///
        sighi                                                            ///
        siglo physlimit wallsup wallvalue targetmode delsource frequse

    tempname raw
    matrix `raw' = r(result)
    local schema `"`r(receipt_schema)'"'
    foreach pair in seed:r_seed probes:r_probes                         ///
        leverage_probes_accepted:r_lacc target_probes_accepted:r_tacc  ///
        requested_algorithm_code:r_algreq selected_algorithm_code:r_algsel ///
        requested_engine_code:r_engreq selected_engine_code:r_engsel   ///
        requested_route:r_rtreq selected_route:r_rtsel                 ///
        solver_fallback:r_fb solver_fallback_error:r_fberr             ///
        solver_dimension:r_dim leverage_batch_width:r_lb               ///
        target_batch_width:r_tb rng_contract_code:r_rng                ///
        rhs_receipt_rows:r_rhsrows caller_result_copy_bytes:r_resultcopy ///
        memory_limit_bytes:r_memlimit caller_copy_bytes:r_inputcopy     ///
        preparation_peak_forecast_bytes:r_preppeak                     ///
        prepared_resident_bytes:r_resident                             ///
        solver_setup_forecast_bytes:r_setup                            ///
        leverage_phase_forecast_bytes:r_lphase                         ///
        target_phase_forecast_bytes:r_tphase                           ///
        result_forecast_bytes:r_resultbytes                            ///
        solve_peak_forecast_bytes:r_solvepeak                         ///
        command_peak_forecast_bytes:r_cmdpeak                          ///
        full_fit_route:r_fullroute full_fit_iterations:r_fulliter      ///
        full_fit_reduced_residual:r_fullred                            ///
        full_fit_complete_residual:r_fullcomp                          ///
        full_fit_zero_rhs:r_fullzero leverage_rhs_count:r_lrhs         ///
        target_rhs_count:r_trhs max_reduced_residual:r_maxred          ///
        max_complete_residual:r_maxcomp max_leverage:r_maxlev          ///
        max_reciprocal_residual:r_maxrecip accounting_residual:r_acct  ///
        actual_accounting_residual:r_aacct weighted_rss:r_rss          ///
        rank_tolerance:r_rank block_tolerance:r_block                  ///
        full_residual_tolerance:r_fulltol deletion_mode_code:r_del     ///
        nuisance_mode_code:r_nuis parameters:r_params                  ///
        full_parameters:r_fullparams correction_parameters:r_corrparams ///
        information_rcond:r_info inverse_relative_residual:r_inv       ///
        exact_peak_forecast_bytes:r_exactpeak                          ///
        exact_diagnostic_flags:r_xflags                               ///
        working_fit_complete_residual:r_workfit                       ///
        inverse_sqrt_relative_residual:r_invsqrt                       ///
        maker_relative_residual:r_maker                               ///
        control_basis_relative_residual:r_cbrel                        ///
        control_basis_forward_error:r_cbfwd deletion_rank_gap:r_rankgap ///
        firm_zero_sum_residual:r_fzero fit_peak_forecast_bytes:r_fitpeak ///
        correction_peak_forecast_bytes:r_corrpeak                     ///
        rhs_receipt_schema:r_rhsschema capability_schema:r_capschema  ///
        capability_profile:r_capprof request_signature_hi:r_sighi     ///
        request_signature_lo:r_siglo batch_mode_code:r_batchmode      ///
        stayers_mode_code:r_staymode target_weight_mode_code:r_targetmode ///
        deletion_source_code:r_delsource probeorder_supplied:r_probeorder ///
        wallseconds_supplied:r_wallsup frequency_use_code:r_frequse   ///
        physical_limit:r_phys generic_controls_count:r_resultcontrols ///
        batch_lev_mode:r_levmode batch_tgt_mode:r_tgtmode             ///
        plan_struct:r_planstruct plan_schema:r_planschema             ///
        plan_route_schema:r_prouteschema plan_resolved:r_presolved    ///
        plan_frozen:r_pfrozen plan_applicability:r_papp               ///
        plan_alg_req:r_palgreq plan_alg_sel:r_palgsel                 ///
        plan_alg_reason:r_palgreason plan_eng_req:r_pengreq           ///
        plan_eng_sel:r_pengsel plan_eng_reason:r_pengreason           ///
        plan_comp_elig:r_pcompelig plan_complexity:r_pcomplexity      ///
        plan_exact_limit:r_pexactlimit                               ///
        plan_route_req:r_proutereq plan_route_sel:r_proutesel         ///
        plan_route_fallback:r_pfb plan_route_error:r_pfberr           ///
        plan_rhs:r_prhs plan_full_dim:r_pfulldim                      ///
        batch_lev_sel:r_plev batch_tgt_sel:r_ptgt                     ///
        batch_command:r_pbatchcmd mem_command:r_pmemcmd               ///
        ctr_complete:r_ctr plan_res_rng_hi:r_prnghi                   ///
        plan_res_rng_lo:r_prnglo wall_req_app:r_wallapp               ///
        wall_requested:r_wallreq wall_forecast:r_wallforecast         ///
        wall_advisory:r_walladv wall_margin:r_wallmargin {
        gettoken source target : pair, parse(":")
        gettoken colon target : target, parse(":")
        local `target' = r(`source')
    }

    local ok = 1
    local detail
    foreach value in algreq engreq delcode nuiscode workers firms controls ///
        ranktol blocktol tolerance exactlimit memlimit inputcopy preppeak  ///
        resident sighi siglo physlimit wallsup wallvalue targetmode       ///
        delsource frequse {
        if missing(``value'') {
            local ok = 0
            if `"`detail'"' == "" local detail "missing expected exact-V7 argument `value'"
        }
    }
    if `ok' & (!inlist(`algreq',0,1) | !inlist(`engreq',0,2) |      ///
        !inlist(`delcode',1,2) | !inlist(`nuiscode',1,2) |          ///
        `workers'<=0 | `workers'!=floor(`workers') | `firms'<=1 |   ///
        `firms'!=floor(`firms') | `controls'<0 |                    ///
        `controls'!=floor(`controls') | `ranktol'<=0 | `blocktol'<=0 | ///
        `tolerance'<=0 | `exactlimit'<2 | `exactlimit'>2000 |       ///
        `exactlimit'!=floor(`exactlimit') | `memlimit'<=0 |           ///
        `memlimit'!=floor(`memlimit') |                               ///
        `inputcopy'<0 | `inputcopy'!=floor(`inputcopy') |            ///
        `preppeak'<0 | `preppeak'!=floor(`preppeak') |               ///
        `resident'<=0 | `resident'!=floor(`resident') |              ///
        `sighi'<0 | `sighi'>4294967295 | `sighi'!=floor(`sighi') |   ///
        `siglo'<0 | `siglo'>4294967295 | `siglo'!=floor(`siglo') |   ///
        `physlimit'<=0 | `physlimit'>9007199254740992 |              ///
        `physlimit'!=floor(`physlimit') | !inlist(`wallsup',0,1) |   ///
        (`wallsup'==0 & `wallvalue'!=0) | (`wallsup'==1 & `wallvalue'<=0) | ///
        !inlist(`targetmode',0,1) | !inlist(`delsource',1,2,3) |     ///
        !inlist(`frequse',0,1)) {
        local ok = 0
        local detail "invalid expected exact-V7 tuple"
    }

    local fullparams = `workers'+`firms'-1+`controls'
    local params = `fullparams'
    if `nuiscode'==2 local params = `workers'+`firms'-1
    local fulltol = max(1e-11,10*`tolerance')
    local xflags = 497 + 6*(`delcode'==1) + 8*(`controls'>0)
    local invgate = max(1e-10,100*`ranktol')
    local eps = 4096*c(epsdouble)

    local receipt_names r_seed r_probes r_lacc r_tacc r_algreq r_algsel ///
        r_engreq r_engsel r_rtreq r_rtsel r_fb r_fberr r_dim r_lb r_tb ///
        r_rng r_rhsrows r_resultcopy r_memlimit r_inputcopy r_preppeak  ///
        r_resident r_setup r_lphase r_tphase r_resultbytes r_solvepeak  ///
        r_cmdpeak r_fullroute r_fulliter r_fullzero r_lrhs r_trhs       ///
        r_rhsschema r_capschema r_capprof r_batchmode r_staymode       ///
        r_targetmode r_delsource r_probeorder r_wallsup r_frequse      ///
        r_phys r_resultcontrols r_levmode r_tgtmode r_planstruct       ///
        r_planschema r_prouteschema r_presolved r_pfrozen r_papp       ///
        r_palgreq r_palgsel r_palgreason r_pengreq r_pengsel          ///
        r_pengreason r_pcompelig r_pcomplexity r_pexactlimit          ///
        r_proutereq r_proutesel                                      ///
        r_pfb r_pfberr r_prhs r_pfulldim r_plev r_ptgt r_pbatchcmd     ///
        r_pmemcmd r_ctr r_prnghi r_prnglo r_wallapp
    foreach value of local receipt_names {
        if missing(``value'') | ``value''<0 | ``value''!=floor(``value'') {
            local ok = 0
            if `"`detail'"'=="" local detail "missing or nonintegral exact-V7 receipt `value'"
        }
    }
    foreach value in r_fullred r_fullcomp r_maxred r_maxcomp r_maxlev ///
        r_maxrecip r_acct r_aacct r_rss r_rank r_block r_fulltol     ///
        r_info r_inv r_workfit r_invsqrt r_maker r_cbrel r_cbfwd    ///
        r_rankgap r_fzero r_wallreq r_wallforecast r_walladv r_wallmargin {
        if missing(``value'') {
            local ok = 0
            if `"`detail'"'=="" local detail "missing exact-V7 numerical receipt `value'"
        }
    }
    if rowsof(`raw')!=4 | colsof(`raw')!=4 {
        local ok = 0
        local detail "exact-V7 result matrix is not 4 by 4"
    }
    if `ok' & `"`schema'"'!="VCKSS-EXECUTION-PLAN-V1" {
        local ok = 0
        local detail "exact result omitted the V7 execution-plan schema"
    }

    local accttruth = 0
    local scale = 1
    if `ok' {
        forvalues row=1/4 {
            forvalues col=1/4 {
                if missing(`raw'[`row',`col']) local ok = 0
                else local scale = max(`scale',abs(`raw'[`row',`col']))
            }
        }
        forvalues row=1/3 {
            local resid = abs(`raw'[`row',4]-`raw'[`row',1]-         ///
                `raw'[`row',2]-2*`raw'[`row',3])
            local accttruth = max(`accttruth',`resid')
            if `resid'>1e-10*`scale' local ok = 0
        }
        forvalues col=1/4 {
            if `raw'[4,`col']!=0 | abs(`raw'[1,`col']-              ///
                `raw'[2,`col']-`raw'[3,`col'])>1e-10*max(1,abs(`raw'[1,`col'])) ///
                local ok = 0
        }
        if !`ok' & `"`detail'"'=="" local detail "exact result accounting identities failed"
    }

    if `ok' {
        local ok = `r_seed'==0 & `r_probes'==0 & `r_lacc'==0 & `r_tacc'==0 & ///
            `r_algreq'==`algreq' & `r_algsel'==1 &                  ///
            `r_engreq'==`engreq' & `r_engsel'==3 &                 ///
            `r_rtreq'==1 & `r_rtsel'==1 & `r_fb'==0 & `r_fberr'==0 & ///
            `r_dim'==`params' & `r_lb'==0 & `r_tb'==0 & `r_rng'==0 & ///
            `r_rhsrows'==0 & `r_resultcopy'==0 & `r_rhsschema'==0 & ///
            `r_fullroute'==1 & `r_fulliter'==0 & `r_fullzero'==0 &  ///
            `r_lrhs'==0 & `r_trhs'==0 & `r_del'==`delcode' &       ///
            `r_nuis'==`nuiscode' & `r_params'==`params' &          ///
            `r_fullparams'==`fullparams' & `r_corrparams'==`params'
        if !`ok' local detail "exact V4/V7 request, route, dimension, or zero-RNG receipt mismatch"
    }
    if `ok' {
        local ok = `r_rank'==`ranktol' & `r_block'==`blocktol' &    ///
            abs(`r_fulltol'-`fulltol')<=`eps'*max(1,`fulltol') &    ///
            `r_fullred'>=0 & `r_fullcomp'>=0 & `r_fullcomp'<=`r_fulltol' & ///
            `r_workfit'>=0 & `r_workfit'<=`r_fulltol' &             ///
            abs(`r_fullred'-`r_fullcomp')<=`eps'*max(1,abs(`r_fullcomp')) & ///
            abs(`r_maxcomp'-max(`r_fullcomp',`r_workfit'))<=        ///
                `eps'*max(1,abs(`r_maxcomp')) &                     ///
            abs(`r_maxred'-`r_maxcomp')<=`eps'*max(1,abs(`r_maxcomp')) & ///
            `r_maxlev'>=0 & `r_maxlev'<1 & `r_rankgap'>`blocktol' & ///
            `r_fzero'>=0 & `r_fzero'<=`invgate' & `r_info'>0 & `r_info'<=1 & ///
            `r_inv'>=0 & `r_inv'<=`invgate' & `r_acct'==0 &        ///
            `r_aacct'>=0 & abs(`r_aacct'-`accttruth')<=`eps'*`scale' & ///
            `r_rss'>=0 & `r_xflags'==`xflags'
        if !`ok' local detail "exact V4/V7 residual, accounting, rank, or diagnostic receipt mismatch"
    }
    if `ok' {
        local ok = `r_memlimit'==`memlimit' & `r_inputcopy'==`inputcopy' & ///
            `r_preppeak'==`preppeak' & `r_resident'==`resident' &   ///
            `r_setup'==0 & `r_lphase'==0 & `r_tphase'==0 &         ///
            `r_resultbytes'>0 & `r_fitpeak'>0 & `r_corrpeak'>0 &   ///
            `r_exactpeak'==max(`r_fitpeak',`r_corrpeak') &          ///
            `r_solvepeak'==`r_exactpeak' &                          ///
            `r_cmdpeak'==max(`r_preppeak',`r_solvepeak') &          ///
            `r_cmdpeak'<=`r_memlimit'
        if !`ok' local detail "exact V4/V7 direct-memory receipt mismatch"
    }
    if `ok' {
        local ok = `r_capschema'==3 & `r_capprof'==4 &              ///
            `r_sighi'==`sighi' & `r_siglo'==`siglo' &              ///
            `r_batchmode'==0 & `r_staymode'==1 &                    ///
            `r_targetmode'==`targetmode' & `r_delsource'==`delsource' & ///
            `r_probeorder'==0 & `r_wallsup'==`wallsup' &            ///
            `r_frequse'==`frequse' & `r_phys'==`physlimit' &        ///
            `r_resultcontrols'==`controls' & `r_levmode'==3 & `r_tgtmode'==3
        if !`ok' local detail "exact V4/V7 capability or semantic-context receipt mismatch"
    }
    if `ok' & `delcode'==1 {
        local ok = `r_invsqrt'>=0 & `r_maker'>=0 & `r_maker'<=`invgate' & ///
            abs(`r_maxrecip'-`r_maker')<=`eps'*max(1,abs(`r_maker'))
        if !`ok' local detail "exact match-deletion inverse/maker receipt mismatch"
    }
    if `ok' & `delcode'==2 {
        local ok = `r_invsqrt'==0 & `r_maker'==0 & `r_maxrecip'==0
        if !`ok' local detail "exact observation-deletion inverse/maker receipt mismatch"
    }
    if `ok' & `controls'>0 {
        local ok = `r_cbrel'>=0 & `r_cbfwd'>=0 & `r_cbfwd'<.25
        if !`ok' local detail "exact control-basis receipt mismatch"
    }
    if `ok' & `controls'==0 {
        local ok = `r_cbrel'==0 & `r_cbfwd'==0
        if !`ok' local detail "exact zero-control receipt mismatch"
    }
    if `ok' {
        local ok = `r_planstruct'==1000 & `r_planschema'==1 &       ///
            `r_prouteschema'>=1 & `r_presolved'==1 & `r_pfrozen'==1 & ///
            `r_papp'==1 & `r_palgreq'==`algreq' & `r_palgsel'==1 & ///
            `r_palgreason'==cond(`algreq'==0,3,1) &                 ///
            `r_pengreq'==`engreq' & `r_pengsel'==3 &               ///
            `r_pengreason'==1 & `r_pcompelig'==0 &                  ///
            `r_pcomplexity'==`fullparams' &                         ///
            `r_pexactlimit'==`exactlimit' &                         ///
            `r_proutereq'==4 & `r_proutesel'==4 &                  ///
            `r_pfb'==0 & `r_pfberr'==0 & `r_prhs'==0 &             ///
            `r_pfulldim'==0 & `r_plev'==0 & `r_ptgt'==0 &          ///
            `r_pbatchcmd'==0 & `r_pmemcmd'==`r_solvepeak' &        ///
            `r_ctr'==1 & `r_prnghi'==0 & `r_prnglo'==0 &           ///
            `r_wallapp'==`wallsup' & `r_wallreq'==`wallvalue' &    ///
            `r_wallforecast'>=0 & `r_walladv'>=0 & `r_wallmargin'>=0
        if !`ok' local detail "exact frozen execution-plan receipt mismatch"
    }
    if !`ok' & `"`detail'"'=="" local detail "planned exact V7 receipt reconciliation failed"
    if `ok' local detail "planned exact V7 result and execution plan reconciled"

    return clear
    return scalar ok = `ok'
    return local detail `"`detail'"'
    return local result_family "exact"
    return local execution_plan_schema `"`schema'"'
    return matrix result = `raw'
    return scalar algorithm_requested = `r_algreq'
    return scalar algorithm_selected = `r_algsel'
    return scalar engine_requested = `r_engreq'
    return scalar engine_selected = `r_engsel'
    return scalar plan_algorithm_requested = `r_palgreq'
    return scalar plan_algorithm_selected = `r_palgsel'
    return scalar plan_algorithm_reason = `r_palgreason'
    return scalar plan_engine_requested = `r_pengreq'
    return scalar plan_engine_selected = `r_pengsel'
    return scalar plan_engine_reason = `r_pengreason'
    return scalar plan_compressed_eligibility = `r_pcompelig'
    return scalar plan_complexity = `r_pcomplexity'
    return scalar plan_exact_limit = `r_pexactlimit'
    return scalar plan_applicability = `r_papp'
    return scalar plan_resolved = `r_presolved'
    return scalar plan_frozen = `r_pfrozen'
    return scalar plan_route_requested = `r_proutereq'
    return scalar plan_route_selected = `r_proutesel'
    return scalar counter_complete = `r_ctr'
    return scalar pre_rng_hi = `r_prnghi'
    return scalar pre_rng_lo = `r_prnglo'
    return scalar solve_peak_bytes = `r_solvepeak'
    return scalar plan_memory_bytes = `r_pmemcmd'
    return scalar full_fit_complete_residual = `r_fullcomp'
    return scalar residual_tolerance = `r_fulltol'
    return scalar actual_accounting_residual = `r_aacct'
end

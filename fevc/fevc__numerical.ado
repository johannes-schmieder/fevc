*! fevc all-point-probe numerical attachment V1 29sep2026
program define fevc__numerical, eclass
    version 18.0
    gettoken action 0 : 0
    macro shift
    if "`action'" == "init" {
        args cond lev raw all se meta rhs
        foreach field in MODE STATUS FAILURE COND LEV RAW ALL SE META RHS ERR DETAIL EXECUTED POINT_RHS REQUESTED EXPLICIT UNAVAILABLE {
            capture macro drop VCKSS_NMC_`field'
        }
        global VCKSS_NMC_MODE all
        foreach field in cond lev raw all se meta rhs {
            local key = upper("`field'")
            global VCKSS_NMC_`key' ``field''
        }
        exit
    }
    if "`action'" == "clear" {
        foreach field in MODE STATUS FAILURE COND LEV RAW ALL SE META RHS ERR DETAIL EXECUTED POINT_RHS REQUESTED EXPLICIT UNAVAILABLE {
            capture macro drop VCKSS_NMC_`field'
        }
        exit
    }
    if "`action'" == "request" {
        args mode inference project alias
        if strtrim(`"`mode'"')!="" & strtrim(`"`alias'"')!="" {
            quietly _fevc_numerical_failure "INVALID_NUMERICAL_MCSE"
            di as error "specify mcse() or its numericalmcse() alias, not both"
            exit 198
        }
        global VCKSS_NMC_EXPLICIT = (strtrim(`"`mode'`alias'"')!="")
        if "`mode'"=="" local mode `alias'
        local mode=lower(strtrim("`mode'"))
        if "`mode'"=="" local mode all
        if !inlist("`mode'","conditional","all","off") {
            quietly _fevc_numerical_failure "INVALID_NUMERICAL_MCSE"
            di as error "mcse() must be all or off"
            exit 198
        }
        global VCKSS_NMC_MODE `mode'
        global VCKSS_NMC_REQUESTED `mode'
        fevc__numerical preflight
        exit
    }
    if "`action'" == "native" {
        args native algorithm
        if "$VCKSS_NMC_MODE"=="all" & `native' & "`algorithm'"!="exact" {
            quietly fevc_rust probe
            if r(numerical_api)!=2 {
                if "$VCKSS_NMC_EXPLICIT"!="1" {
                    global VCKSS_NMC_MODE off
                    global VCKSS_NMC_UNAVAILABLE unavailable_capability
                    exit
                }
                quietly _fevc_numerical_failure "STALE_NUMERICAL_RUNTIME"
                di as error "the loaded plugin lacks numerical V2; install a matching qualified candidate"
                exit 498
            }
        }
        exit
    }
    if "`action'" == "fetch" {
        args handle probes
        fevc_rust numericalresult `handle', probes(`probes')
        foreach pair in COND:conditional LEV:leverage RAW:raw ALL:usable SE:mcse META:metadata {
            gettoken dest source : pair, parse(":")
            local source=substr("`source'",2,.)
            matrix ${VCKSS_NMC_`dest'} = r(`source')
        }
        if r(replay_rhs_count)>0 matrix $VCKSS_NMC_RHS = r(replay_rhs)
        else mata: st_matrix(st_global("VCKSS_NMC_RHS"),J(0,3,.))
        global VCKSS_NMC_STATUS `r(status)'
        global VCKSS_NMC_FAILURE
        if r(replay_error_code)>0 {
            local code=r(replay_error_code)
            if !inlist(`code',51,60,61,62,63,64) {
                quietly _fevc_numerical_failure "NUMERICAL_TRANSPORT_FAILED"
                exit 498
            }
            local name=cond(`code'==51,"CMG_APPLY_FAILED",cond(`code'==60,"PCG_BREAKDOWN_CURVATURE", ///
                cond(`code'==61,"PCG_BREAKDOWN_PRECONDITIONER",cond(`code'==62,"PCG_STAGNATION", ///
                cond(`code'==63,"PCG_MAXITER","FULL_RESIDUAL_FAILED")))))
            global VCKSS_NMC_FAILURE `name'
        }
        global VCKSS_NMC_EXECUTED `r(executed_replay_rhs)'
        global VCKSS_NMC_POINT_RHS `r(point_rhs)'
        exit
    }
    if "`action'" == "reserve" {
        if "$VCKSS_NMC_MODE"!="all" exit
        args components row physical stored probes units cells
        local bound=192*`physical'+640*`stored'+512*`probes'+4096
        if `row'==1 local bound=576*`units'+640*`cells'+512*`probes'+4096
        if missing(`bound') | `bound'>9007199254740992 {
            quietly _fevc_numerical_failure "NUMERICAL_ALLOCATION_OVERFLOW"
            exit 498
        }
        // Phase scratch includes derivative state during the moment overlap.
        matrix `components'[`row',6]=`components'[`row',6]+`bound'
        exit
    }
    if "`action'" == "runtime" {
        if "$VCKSS_NMC_MODE"!="all" exit
        args runtime
        if "`runtime'"=="solver" capture mata: assert(vckss_nmc__schema()==1 & vckss_solver__numerical_schema()==1)
        else capture mata: assert(vckss_scale_eng__nmc_api()==1 & vckss_scale_runtime__nmc_api()==1)
        if _rc {
            quietly _fevc_numerical_failure "STALE_NUMERICAL_RUNTIME"
            di as error "the loaded Mata runtime lacks numerical V1; restart Stata or discard"
            exit 498
        }
        exit
    }
    if "`action'" == "preflight" {
        if "$VCKSS_NMC_MODE"!="all" exit
        capture mata: vckss_nmc__module_api()
        if _rc {
            // An older monolithic runtime must be rejected before redefining its types.
            capture mata: vckss_nmc__schema()
            if !_rc {
                quietly _fevc_numerical_failure "STALE_NUMERICAL_RUNTIME"
                di as error "the loaded numerical runtime is stale; restart Stata or discard"
                exit 498
            }
            quietly findfile fevc_numerical.mata
            quietly do `"`r(fn)'"'
        }
        capture mata: assert(vckss_nmc__module_api()==4 & ///
            vckss_nmc__build_id()=="vckss-numerical-api4-vector-replay8-centering1" & vckss_nmc__schema()==1)
        if _rc {
            quietly _fevc_numerical_failure "STALE_NUMERICAL_RUNTIME"
            di as error "the loaded numerical runtime is stale; restart Stata or discard"
            exit 498
        }
        exit
    }
    if "`action'" != "post" exit 198
    // Direct executor tests use frozen point receipts without a public request.
    if "$VCKSS_NMC_REQUESTED"=="" exit
    // Success routes and the outer wrapper both post; normalize each fit once.
    if "`e(mcse_method)'" != "" exit
    tempname conditional
    capture confirm matrix e(numerical_mcse)
    if _rc matrix `conditional'=J(1,4,.)
    else matrix `conditional'=e(numerical_mcse)
    matrix colnames `conditional'=worker_variance firm_variance worker_firm_covariance total_variance
    local conditional_available = (e(numerical_mcse_available)==1 | "`e(algorithm)'"=="exact")
    ereturn local mcse_mode "$VCKSS_NMC_REQUESTED"
    if "$VCKSS_NMC_MODE" == "off" {
        tempname omitted
        matrix `omitted'=J(1,4,.)
        matrix colnames `omitted'=worker_variance firm_variance worker_firm_covariance total_variance
        ereturn matrix numerical_mcse=`omitted'
        ereturn scalar numerical_mcse_available=0
        ereturn scalar numerical_mcse_all_available=0
        ereturn local numerical_mc_status "off"
        if "$VCKSS_NMC_UNAVAILABLE"!="" ereturn local numerical_mc_status "$VCKSS_NMC_UNAVAILABLE"
        quietly _fevc_post_mcse
        exit
    }
    ereturn matrix mcse_conditional=`conditional'
    ereturn scalar mcse_conditional_available=`conditional_available'
    if "$VCKSS_NMC_MODE" != "all" {
        ereturn local numerical_mc_status "conditional"
        if "`e(algorithm)'"=="exact" ereturn local numerical_mc_status "exact_zero"
        quietly _fevc_post_mcse
        exit
    }
    if "`e(algorithm)'" == "exact" {
        foreach field in COND LEV RAW ALL {
            matrix ${VCKSS_NMC_`field'} = J(3,3,0)
        }
        matrix $VCKSS_NMC_SE = J(1,4,0)
        matrix $VCKSS_NMC_META = (0,0,0,0,0,0,0,0,0,.,.,.,0,0,.)
        mata: st_matrix(st_global("VCKSS_NMC_RHS"),J(0,3,.))
        global VCKSS_NMC_STATUS exact_zero
    }
    tempname valid gate
    scalar `gate' = max(1e-11,10*e(tolerance))
    // Replay reuses the selected point route and must pass that route's own
    // complete-residual gate. CMG_FULL_V2 registers 10*max(fit,probe) phase
    // tolerances, which exceed 10*tolerance() for its default probe phase.
    if "`e(cmg_backend)'"=="CMG_FULL_V2" & !missing(e(residual_acceptance_tolerance)) {
        scalar `gate' = max(scalar(`gate'),e(residual_acceptance_tolerance))
    }
    capture mata: st_numscalar("`valid'",vckss_nmc__stata_validate( ///
        st_numscalar("e(probes)"),st_numscalar("`gate'")))
    if _rc {
        quietly _fevc_numerical_failure "NUMERICAL_TRANSPORT_FAILED" ///
            "The numerical attachment could not be validated."
        exit 498
    }
    if !scalar(`valid') {
        quietly _fevc_numerical_failure "NUMERICAL_TRANSPORT_FAILED" ///
            "The numerical matrices, status or work receipts did not reconcile."
        exit 498
    }
    foreach field in COND LEV RAW ALL {
        matrix rownames ${VCKSS_NMC_`field'} = var_worker var_firm cov_worker_firm
        matrix colnames ${VCKSS_NMC_`field'} = var_worker var_firm cov_worker_firm
    }
    matrix colnames $VCKSS_NMC_SE = worker_variance firm_variance worker_firm_covariance total_variance
    ereturn matrix numerical_mccov_cond = $VCKSS_NMC_COND
    ereturn matrix numerical_mccov_leverage = $VCKSS_NMC_LEV
    ereturn matrix numerical_mccov_all_raw = $VCKSS_NMC_RAW
    ereturn matrix numerical_mccov_all = $VCKSS_NMC_ALL
    ereturn matrix numerical_mcse_all = $VCKSS_NMC_SE
    ereturn local numerical_mc_method "crossfit_if_v1"
    ereturn local numerical_mc_status "$VCKSS_NMC_STATUS"
    ereturn local numerical_mc_scope "local_unfiltered_point_probes"
    ereturn local numerical_mc_failure "$VCKSS_NMC_FAILURE"
    ereturn local numerical_mc_conditioning "all observed inputs and deterministic preparation"
    ereturn local numerical_mc_excludes "finite-probe bias; solver/floating-point error; sampling uncertainty"
    local names mc_leverage_probes mc_target_probes mc_target_fold_a ///
        mc_target_fold_b mc_replay_rhs mc_replay_attempted_rhs mc_replay_generator_work ///
        mc_allocation_bound_bytes mc_psd_adjustment mc_min_constrained ///
        mc_min_residual_margin mc_sensitivity_ratio mc_replay_max_residual ///
        mc_replay_seconds mc_failed_replay_probe
    local i = 0
    foreach name of local names {
        local ++i
        ereturn scalar `name' = el($VCKSS_NMC_META,1,`i')
    }
    if "$VCKSS_NMC_EXECUTED"!="" ereturn scalar mc_replay_executed_rhs = real("$VCKSS_NMC_EXECUTED")
    else ereturn scalar mc_replay_executed_rhs = e(mc_replay_rhs)
    if e(mc_replay_rhs)>0 {
        matrix colnames $VCKSS_NMC_RHS = phase probe complete_residual
        ereturn matrix numerical_replay_rhs = $VCKSS_NMC_RHS
    }
    ereturn scalar numerical_mcse_all_available = ///
        inlist("$VCKSS_NMC_STATUS","exact_zero","ok_local","ok_local_psd_adjusted")
    quietly _fevc_post_mcse
end

program define _fevc_post_mcse, eclass
    version 18.0
    tempname se raw usable map results legacy hybrid
    local targets worker_variance firm_variance worker_firm_covariance total_variance
    matrix `se'=J(1,4,.)
    matrix `raw'=J(4,4,.)
    matrix `usable'=J(4,4,.)
    local available=0
    local method none
    if "`e(mcse_mode)'"=="conditional" {
        matrix `se'=e(mcse_conditional)
        local available=e(mcse_conditional_available)
        local method conditional_target_v1
        // This developer diagnostic has no full covariance attachment.
    }
    if "`e(mcse_mode)'"=="all" {
        local method crossfit_if_v1
        capture confirm matrix e(numerical_mcse_all)
        if !_rc {
            matrix `se'=e(numerical_mcse_all)
            local available=e(numerical_mcse_all_available)
            // Total variance is exactly vw+vf+2*cov, including cross terms.
            matrix `map'=(1,0,0 \ 0,1,0 \ 0,0,1 \ 1,1,2)
            matrix `raw'=`map'*e(numerical_mccov_all_raw)*`map''
            matrix `usable'=`map'*e(numerical_mccov_all)*`map''
        }
    }
    if "`e(numerical_mc_status)'"=="exact_zero" local method exact
    matrix colnames `se'=`targets'
    foreach m in raw usable {
        matrix rownames ``m''=`targets'
        matrix colnames ``m''=`targets'
    }
    // Legacy general names alias the selected diagnostic, never a fallback.
    matrix `legacy'=`se'
    ereturn matrix numerical_mcse=`legacy'
    ereturn scalar numerical_mcse_available=`available'
    ereturn matrix mcse=`se'
    ereturn matrix mcse_cov_raw=`raw'
    ereturn matrix mcse_cov=`usable'
    ereturn scalar mcse_available=`available'
    ereturn local mcse_status "`e(numerical_mc_status)'"
    ereturn local mcse_method "`method'"
    capture confirm matrix e(results)
    if !_rc {
        matrix `results'=e(results)
        matrix `results'[4,1]=e(mcse)
        matrix rownames `results'=plugin bias_correction corrected mcse
        ereturn matrix results=`results'
    }
    // This is an alias for the same combined headline population.
    capture confirm matrix e(stayer_hybrid_results)
    if !_rc {
        matrix `hybrid'=e(stayer_hybrid_results)
        matrix `hybrid'[4,1]=e(mcse)
        matrix rownames `hybrid'=plugin bias_correction corrected mcse
        ereturn matrix stayer_hybrid_results=`hybrid'
    }
end

program define _fevc_numerical_failure
    version 18.0
    args status detail
    global VCKSS_NMC_ERR `status'
    global VCKSS_NMC_DETAIL `"`detail'"'
end

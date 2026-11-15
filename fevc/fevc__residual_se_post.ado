*! Posts the description, assumptions, and diagnostics of leave-out
*! residual standard errors, replacing the structured-variance metadata.

program define fevc__residual_se_post, eclass
    version 15.0
    // Repeat-measurement standard errors with leading directions decide the
    // reported interval of every component before anything is posted; the
    // decision lives in this file and is reached through these two actions.
    if "`1'"=="regime_decide" {
        _fevc_se_regime_decide `2' `3' `4'
        exit
    }
    if "`1'"=="regime_clear" {
        capture matrix drop __fevc_se_leading
        capture matrix drop __fevc_se_intervals
        capture macro drop VCKSS_SE_ANY_Q1
        exit
    }
    // draws: pseudo-noise draws. omitted: 1 when no control term was needed
    // because there are no controls. route: rust or mata.
    args draws deletion omitted route
    tempname spectrum
    matrix `spectrum' = e(component_spectrum)
    ereturn scalar inference_draws = `draws'
    // Matrix-free route: one fit and three kernel images per draw, plus
    // three solves for the kernel image of the outcome.
    if "`route'"=="rust" ereturn scalar inference_simulations = 4*`draws'
    else ereturn scalar inference_simulations = `draws'
    ereturn local inference_variance_fit "leaveout_residual"
    ereturn local inference_method                              ///
        "leave-out residual standard errors: squared-residual signal term, pseudo-noise trace and calibration"
    ereturn local inference_rng = cond("`route'"=="rust",       ///
        "Counter-V1 Gaussian pseudo-noise draws",                 ///
        "Stata Gaussian pseudo-noise draws")
    ereturn local inference_reference                            ///
        "normal approximation; effective rank of the squared kernel reported"
    ereturn local inference_reference_selected "q0 normal"
    ereturn local inference_variance_response                    ///
        "squared residual divided by the residual-maker diagonal (HC2)"
    ereturn local inference_variance_conditioning                ///
        "working variance for pseudo-noise only: within-type midranks of leverage and unit mass, linear and squared"
    ereturn local inference_crossfit "not applicable; no model selection"
    ereturn local inference_ordering "not applicable"
    ereturn local inference_positivity                           ///
        "signal term truncated to positive semidefinite; working-variance floor 1e-8 times median squared residual over the maker diagonal"
    ereturn local inference_kss_scope                            ///
        "FEVC extension of the KSS variance formula; not the KSS split-sample variance estimator"
    ereturn local inference_spectral_rule                        ///
        "leading squared-eigenvalue share lies between 1/r_eff and 1/sqrt(r_eff); no eigenvector is computed"
    ereturn local inference_support_status "approximate_residual_based"
    ereturn local inference_qualification                         ///
        "development evidence from a Python prototype and Rust dense oracles; not a qualified release claim"
    ereturn local inference_gram_method "not applicable"
    ereturn local inference_variance_warning                      ///
        "assumes similar error variances among units that share a worker or a firm; not robust to arbitrary heteroskedasticity"
    ereturn local inference_reference_guarantee                   ///
        "normal intervals need no dominant direction and no dominant unit; see e(inference_effective_rank)"
    ereturn local inference_q_condition                           ///
        "effective rank well above one for every component"
    ereturn local inference_execution_scope                       ///
        "successful computation does not establish the variance assumption or the normal approximation"
    ereturn local inference_covariance                            ///
        "four-component covariance of rank three; total is worker + firm + 2 x covariance"
    ereturn local inference_capability = cond("`route'"=="rust", ///
        "leave-out residual standard errors; generic JLA; fixed-offset unit geometry under either nuisance mode", ///
        "leave-out residual standard errors; Mata exact kernels or linear solves; fixed-offset unit geometry")
    // Pseudo-noise is centred exactly as the estimate is, so the observed
    // mean is not held fixed.
    ereturn scalar inference_mean_omitted = 0
    ereturn local inference_centering = cond("$VCKSS_CENTERING"=="none", ///
        "uncentered", "mean-centred estimate; pseudo-noise centred identically")
    if "$VCKSS_CENTERING"=="corrected" {
        ereturn local inference_centering                         ///
            "standard error of the mean-centred estimate; corrected-centering increment treated as fixed"
    }
    tempname rank
    matrix `rank' = J(4,3,.)
    forvalues row = 1/4 {
        local share = `spectrum'[`row',7]
        matrix `rank'[`row',1] = cond(`share'>0 & `share'<1, 1/`share'^2, 1)
        matrix `rank'[`row',2] = cond(`share'>0 & `share'<1, `share'^2, 1)
        matrix `rank'[`row',3] = min(1,`share')
    }
    matrix rownames `rank' = worker_variance firm_variance worker_firm_covariance total_variance
    matrix colnames `rank' = effective_rank leading_share_lb leading_share_ub
    local minimum_rank = min(`rank'[1,1],`rank'[2,1],`rank'[3,1])
    ereturn matrix inference_effective_rank = `rank'
    ereturn scalar inference_min_effective_rank = `minimum_rank'
    ereturn scalar inference_max_influence_share = max(`spectrum'[1,15], ///
        `spectrum'[2,15],`spectrum'[3,15],`spectrum'[4,15])
    // Fewer than about ten effective directions is the bottleneck regime in
    // which normal intervals under-cover whatever variance is used.
    ereturn scalar inference_low_rank_warning = (`minimum_rank' < 10)
    ereturn scalar inference_nuisance_omitted = `omitted'
    ereturn local inference_deletion                              ///
        "`deletion' deletion; one unit per mover deletion unit and per stayer observation"
    if "`deletion'"=="observation" {
        ereturn local inference_deletion "observation deletion; one unit per physical observation"
    }
    // Solves spent on the fixed-offset geometry beside a joint point estimate.
    if "`route'"=="rust" {
        ereturn scalar inference_offset_pass_solves =             ///
            e(inference_solver_columns)-4*`draws'-3
        // Receipt columns 39-64 hold the native diagnostics when the plugin
        // sends them: 41-42 working-variance terms, 44 signal truncation,
        // 45-60 the row-major model-based covariance.
        tempname native
        matrix `native' = e(component_inference_receipt)
        if colsof(`native')==64 {
            tempname modelbased modelse terms
            local names worker_variance firm_variance worker_firm_covariance total_variance
            matrix `modelbased' = J(4,4,.)
            matrix `modelse' = J(1,4,.)
            forvalues row = 1/4 {
                forvalues column = 1/4 {
                    matrix `modelbased'[`row',`column'] =         ///
                        `native'[1,44+4*(`row'-1)+`column']
                }
                matrix `modelse'[1,`row'] = sqrt(`modelbased'[`row',`row'])
            }
            matrix rownames `modelbased' = `names'
            matrix colnames `modelbased' = `names'
            matrix rownames `modelse' = se
            matrix colnames `modelse' = `names'
            matrix `terms' = (`native'[1,41],`native'[1,42])
            matrix rownames `terms' = terms
            matrix colnames `terms' = mover_units stayer_observations
            ereturn matrix V_modelbased = `modelbased'
            ereturn matrix se_modelbased = `modelse'
            ereturn matrix inference_working_terms = `terms'
            ereturn scalar inference_signal_truncation = `native'[1,44]
            ereturn scalar inference_controls = `native'[1,40]
        }
    }
    if "`deletion'"=="match" {
        ereturn local inference_method                            ///
            "leave-out residual standard errors on collapsed deletion units"
        ereturn local inference_independence                      ///
            "independence across deletion units; within-unit dependence enters the unit variance; each stayer observation is its own unit"
        ereturn local inference_frequency                         ///
            "mover regression mass; literal independent stayer copies"
        ereturn local inference_offset_warning = cond(`omitted'==0, ///
            "control-coefficient estimation enters as a first-order term", ///
            "no controls: no nuisance offset is estimated")
    }
    else {
        ereturn local inference_independence "independence across observations"
        // The native route keeps controls in the operator; the Mata route
        // subtracts the fitted control index and adds the first-order term.
        ereturn local inference_offset_warning = cond("`route'"=="rust", ///
            "controls stay in the operator; their estimation is part of the kernel", ///
            "control-coefficient estimation enters as a first-order term")
    }
    ereturn local status "FEVC_LEAVEOUT_RESIDUAL_SE"
    if e(inference_computed_targets)<4 ereturn local status "FEVC_LEAVEOUT_RESIDUAL_SE_PARTIAL"
    if e(inference_computed_targets)==0 ereturn local status "FEVC_STRUCTURED_INFERENCE_UNAVAILABLE"
    if "$VCKSS_SE_MODEL"=="repeat_measurement" _fevc_repeat_se_post `route'
end

// Repeat-measurement standard errors share the pass above and replace the
// description of the variances, the assumptions, and the status.
program define _fevc_repeat_se_post, eclass
    version 15.0
    args route
    ereturn local inference_variance_fit "repeat_measurement"
    ereturn local inference_method                              ///
        "repeat-measurement standard errors: sample variance of every repeat-measurement set (the deletion units of one worker-firm cell), two-step fitted model for the remaining units, high-rank covariance with a pair term for the sets"
    ereturn local inference_variance_response                    ///
        "sets: sample variance of the outcomes of the cell; remaining units: leave-out products"
    ereturn local inference_variance_conditioning                ///
        "units outside sets: scale on midranks of leverage and unit mass (linear and squared) times a quadratic in the standardised estimated worker and firm effects, limited to three standard deviations"
    ereturn local inference_positivity                           ///
        "covariance reported as computed, no truncation; fitted scale floored at five per cent of the mean leave-out product of the fitted units; variances floored at 1e-8 times the median squared residual over the maker diagonal"
    ereturn local inference_support_status "fitted_model_for_movers"
    ereturn local inference_qualification                         ///
        "development evidence from a Python prototype, simulations on one data set, and Rust dense oracles; not a qualified release claim"
    ereturn local inference_variance_warning                      ///
        "as good as the fitted variance model for movers; errors assumed independent across the units of a worker-firm cell with one variance per cell"
    ereturn local inference_capability = cond("`route'"=="rust", ///
        "repeat-measurement standard errors; generic JLA; fixed-offset unit geometry, match or observation deletion", ///
        "repeat-measurement standard errors; Mata exact kernels or linear solves; fixed-offset unit geometry, match or observation deletion")
    // The flag is 0 exactly when controls entered with their first-order term.
    if e(inference_nuisance_omitted)==0 {
        ereturn local inference_method                            ///
            "`e(inference_method)'; estimation of the control coefficients included to first order"
        ereturn local inference_offset_warning                    ///
            "control-coefficient estimation enters as a first-order term"
    }
    else ereturn local inference_offset_warning                   ///
        "no controls: no nuisance offset is estimated"
    // Shape of the variances: from the native receipt (columns 65-78) or
    // from the Mata runtime.
    tempname shape native
    local shape_ok = 0
    if "`route'"=="rust" {
        capture matrix `native' = e(component_inference_receipt)
        if !_rc {
            if colsof(`native')==78 {
                matrix `shape' = `native'[1,65..78]
                local shape_ok = 1
            }
        }
    }
    else if `: word count $VCKSS_SE_SHAPE'==14 {
        matrix `shape' = J(1,14,.)
        forvalues index = 1/14 {
            matrix `shape'[1,`index'] = real("`: word `index' of $VCKSS_SE_SHAPE'")
        }
        local shape_ok = 1
    }
    if `shape_ok' {
        matrix colnames `shape' = scale_terms profile_terms scale_floor ///
            scale_floored sets set_units set_floored pair_term ///
            profile_1 profile_2 profile_3 profile_4 profile_5 profile_6
        matrix rownames `shape' = repeat_measurement
        ereturn scalar inference_repeat_sets = `shape'[1,5]
        ereturn scalar inference_pair_term = `shape'[1,8]
        ereturn matrix inference_repeat = `shape'
    }
    ereturn local status "FEVC_REPEAT_MEASUREMENT_SE"
    if e(inference_computed_targets)<4 ereturn local status "FEVC_REPEAT_MEASUREMENT_SE_PARTIAL"
    if e(inference_computed_targets)==0 ereturn local status "FEVC_STRUCTURED_INFERENCE_UNAVAILABLE"
    // Leading directions and the interval reported for every component.
    _fevc_se_regime_post
    capture confirm scalar e(inference_leading_steps)
    if !_rc {
        if "`route'"=="rust" {
            // The search, four remainder influences and four remainder
            // kernels per draw are not part of the offset pass.
            ereturn scalar inference_offset_pass_solves =            ///
                e(inference_offset_pass_solves)-e(inference_leading_steps)-4- ///
                4*e(inference_draws)
        }
        if e(inference_q1_components)>0 {
            ereturn local status "FEVC_REPEAT_MEASUREMENT_Q1"
        }
    }
    fevc__residual_se_post regime_clear
end

// The rule. A component is reported with the q = 1 interval when its
// leading direction carries more than `share_threshold' of the estimated
// sampling variance, or when the high-rank variance estimate is not
// positive; under inference(q1) wherever that interval exists. The variance
// of the sum has no dominant direction (its nonzero eigenvalues are all
// equal) and keeps the normal interval under inference(auto).
//
// V: 4 x 4 high-rank covariance. b: 1 x 4 estimates. level: confidence level
// in per cent. Reads __fevc_se_leading (4 x 28, the layout of the native
// export) and $VCKSS_SE_LEADING (auto or q1). Leaves __fevc_se_intervals:
//   1 estimate   2 highrank_se   3 normal_lb   4 normal_ub   5 q1_lb
//   6 q1_ub      7 regime (0 normal, 1 q = 1, missing: none)  8 lb  9 ub
//  10 variance_share  11 curvature  12 critical_value  13 eigen_share
//  14 max_weight_sq   15 converged  16 q1_status
program define _fevc_se_regime_decide
    version 15.0
    args V b level
    local share_threshold .1
    global VCKSS_SE_ANY_Q1 0
    capture matrix drop __fevc_se_intervals
    capture confirm matrix __fevc_se_leading
    if _rc | !inlist("$VCKSS_SE_LEADING","auto","q1") exit
    if rowsof(__fevc_se_leading)!=4 | colsof(__fevc_se_leading)!=28 {
        di as error "repeat-measurement leading directions have the wrong shape"
        exit 498
    }
    tempname table leading
    matrix `leading' = __fevc_se_leading
    matrix `table' = J(4,16,.)
    local z = invnormal(.5+`level'/200)
    local any = 0
    forvalues t = 1/4 {
        local estimate = `b'[1,`t']
        local variance = `V'[`t',`t']
        local lambda = `leading'[`t',5]
        local score = `leading'[`t',11]
        local leadvar = `leading'[`t',14]
        local q1_ok = (`leading'[`t',1]==0 & !missing(`leading'[`t',26]) & ///
            !missing(`leading'[`t',27]))
        local normal_ok = (`variance'>0 & !missing(`variance'))
        local share = .
        if `normal_ok' {
            // Parentheses: a negative macro value must be squared as a whole.
            local share = (4*(`lambda')^2*max((`score')^2-(`leadvar'),0)*(`leadvar')+ ///
                2*(`lambda')^2*(`leadvar')^2)/`variance'
        }
        local regime = .
        if "$VCKSS_SE_LEADING"=="q1" {
            if `q1_ok' local regime = 1
            else if `normal_ok' local regime = 0
        }
        else {
            if `q1_ok' & `t'<4 & (!`normal_ok' | `share'>`share_threshold') local regime = 1
            else if `normal_ok' local regime = 0
        }
        matrix `table'[`t',1] = `estimate'
        if `normal_ok' {
            matrix `table'[`t',2] = sqrt(`variance')
            matrix `table'[`t',3] = `estimate'-`z'*sqrt(`variance')
            matrix `table'[`t',4] = `estimate'+`z'*sqrt(`variance')
        }
        if `q1_ok' {
            // The decomposition is that of the standard-error pass. Its
            // estimate differs from the posted one by projection noise on the
            // matrix-free Mata route and by the estimator of the controls
            // under joint nuisance; the interval moves with the posted one.
            matrix `table'[`t',5] = `leading'[`t',26]+`estimate'-`leading'[`t',10]
            matrix `table'[`t',6] = `leading'[`t',27]+`estimate'-`leading'[`t',10]
        }
        matrix `table'[`t',7] = `regime'
        if `regime'==1 {
            matrix `table'[`t',8] = `table'[`t',5]
            matrix `table'[`t',9] = `table'[`t',6]
            local any = 1
        }
        else if `regime'==0 {
            matrix `table'[`t',8] = `table'[`t',3]
            matrix `table'[`t',9] = `table'[`t',4]
        }
        matrix `table'[`t',10] = `share'
        matrix `table'[`t',11] = `leading'[`t',24]
        matrix `table'[`t',12] = `leading'[`t',25]
        matrix `table'[`t',13] = `leading'[`t',8]
        matrix `table'[`t',14] = `leading'[`t',9]
        matrix `table'[`t',15] = `leading'[`t',2]
        matrix `table'[`t',16] = `leading'[`t',1]
    }
    matrix rownames `table' = worker_variance firm_variance ///
        worker_firm_covariance total_variance
    matrix colnames `table' = estimate highrank_se normal_lb normal_ub ///
        q1_lb q1_ub regime lb ub variance_share curvature critical_value ///
        eigen_share max_weight_sq converged q1_status
    matrix __fevc_se_intervals = `table'
    global VCKSS_SE_ANY_Q1 `any'
    global VCKSS_SE_SHARE_THRESHOLD `share_threshold'
end

program define _fevc_se_regime_post, eclass
    version 15.0
    capture confirm matrix __fevc_se_intervals
    if _rc exit
    tempname table leading
    matrix `table' = __fevc_se_intervals
    matrix `leading' = __fevc_se_leading
    matrix rownames `leading' = worker_variance firm_variance ///
        worker_firm_covariance total_variance
    matrix colnames `leading' = status converged steps critical_draws ///
        lambda1 lambda2 residual eigen_share max_weight_sq estimate b1 ///
        var_b1_leaveout var_b1_fitted var_b1 theta_r identity_error ///
        cov_b1_theta_r var_theta_r influence_var trace_var trace_mcse ///
        pair_term determinant curvature critical_value q1_lb q1_ub F
    local q1_components = 0
    local unresolved = 0
    local steps = 0
    forvalues t = 1/4 {
        if `table'[`t',7]==1 local ++q1_components
        if `leading'[`t',2]!=1 local ++unresolved
        local steps = `steps'+`leading'[`t',3]
    }
    ereturn scalar inference_q1_components = `q1_components'
    ereturn scalar inference_leading_unresolved = `unresolved'
    ereturn scalar inference_leading_steps = `steps'
    ereturn scalar inference_share_threshold = $VCKSS_SE_SHARE_THRESHOLD
    ereturn local inference_leading_request "$VCKSS_SE_LEADING"
    ereturn local inference_regime_rule = cond("$VCKSS_SE_LEADING"=="q1", ///
        "inference(q1): the q = 1 interval wherever it exists", ///
        "q = 1 interval when the leading direction carries more than " + ///
        strofreal($VCKSS_SE_SHARE_THRESHOLD) + ///
        " of the estimated sampling variance or the high-rank variance estimate is not positive; normal interval otherwise")
    ereturn local inference_reference_selected = cond(`q1_components'==0, ///
        "q0 normal","q0 normal or q1 by component")
    ereturn local inference_reference                             ///
        "normal approximation, or the Andrews-Mikusheva interval along one leading direction (KSS q = 1), by component"
    ereturn matrix inference_leading = `leading'
    ereturn matrix inference_intervals = `table'
end

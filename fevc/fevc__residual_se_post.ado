*! Posts the description, assumptions, and diagnostics of leave-out
*! residual standard errors, replacing the structured-variance metadata.

program define fevc__residual_se_post, eclass
    version 18.0
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
end

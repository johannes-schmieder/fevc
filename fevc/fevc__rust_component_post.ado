program define fevc__rust_component_post, eclass
    version 18.0
    args model inference simulations inferenceseed level primitive results ///
        mcse spectrum summaries folds cv receipt augmentation q1results q1raw ///
        units deletion
    local q1computed = 4
    local joint_status = `receipt'[1,24]
    local variance_fit = `receipt'[1,25]
    local computed_targets = `receipt'[1,34]
    tempname q0status
    matrix `q0status' = (`receipt'[1,35..38])'
    matrix rownames `q0status' = worker_variance firm_variance worker_firm_covariance total_variance
    matrix colnames `q0status' = status
    ereturn matrix q0_status = `q0status'
    ereturn local q0_status_codes "0 computed; 1 nonpositive variance; 4 no positive linear influence; 6 spectrum not certified"
    ereturn scalar inference_joint_status = `joint_status'
    ereturn scalar inference_joint_available = (`joint_status'==0)
    ereturn local inference_joint_status_codes "0 admissible; 1 nonpositive diagonal; 2 materially indefinite"
    ereturn scalar inference_computed_targets = `computed_targets'
    if `joint_status'==0 ereturn matrix V_primitive = `primitive'
    ereturn matrix component_inference = `results'
    ereturn matrix component_trace_mcse = `mcse'
    ereturn matrix component_spectrum = `spectrum'
    if `variance_fit'==1 {
        ereturn matrix structured_variance_summary = `summaries'
        ereturn matrix structured_variance_folds = `folds'
        ereturn matrix structured_variance_cv = `cv'
    }
    else {
        tempname fit_diagnostics
        matrix `fit_diagnostics' = `receipt'[1,26..33]
        ereturn matrix residual_moment_diagnostics = `fit_diagnostics'
        ereturn scalar inference_gram_probes = `receipt'[1,26]
        ereturn local inference_gram_method "direct_residual_covariance"
        ereturn scalar variance_gram_rcond = `receipt'[1,28]
        ereturn scalar variance_floor_share = `receipt'[1,32]/`units'[1,3]
    }
    if "`inference'"=="q1" {
        tempname q1status
        matrix `q1status' = `q1raw'[1..4,17]
        matrix colnames `q1status' = status
        ereturn matrix q1_status = `q1status'
        local q1computed = 0
        forvalues row = 1/4 {
            local q1computed = `q1computed'+(`q1raw'[`row',17]==0)
        }
        ereturn scalar q1_computed_targets = `q1computed'
        ereturn local q1_status_codes "0 computed; 1 nonpositive variance; 2 singular covariance; 3 interval failure; 6 leading mode not certified"
        ereturn matrix q1_inference = `q1results'
        ereturn matrix component_q1_diagnostics = `q1raw'
    }
    ereturn scalar level = `level'
    ereturn scalar inference_simulations = `simulations'
    ereturn scalar inference_seed = `inferenceseed'
    ereturn scalar inference_model_option_supplied = 1
    ereturn scalar inference_spectrum_probes = e(component_spectrum)[1,13]
    ereturn scalar inference_spectrum_iterations = e(component_spectrum)[1,14]
    local inference_solver_columns = `receipt'[1,22]
    ereturn scalar inference_solver_columns = `inference_solver_columns'
    ereturn scalar inference_critical_draws = `receipt'[1,23]
    ereturn scalar inference_psd_cleanup = `receipt'[1,8]
    ereturn scalar inference_covariance_min_raw = `receipt'[1,9]
    ereturn scalar inference_covariance_max_raw = `receipt'[1,10]
    ereturn scalar inference_point_identity_error = `receipt'[1,11]
    ereturn scalar inference_counter_atoms = `receipt'[1,5]
    ereturn scalar inference_counter_words = `receipt'[1,6]
    ereturn scalar inference_peak_forecast_bytes = `receipt'[1,7]
    ereturn scalar inference_solver_iterations = `receipt'[1,12]
    ereturn scalar inference_solver_max_reduced = `receipt'[1,13]
    ereturn scalar inference_solver_max_complete = `receipt'[1,14]
    ereturn scalar inference_solver_tolerance = `receipt'[1,15]
    ereturn scalar variance_median_abs_log_ratio = `receipt'[1,16]
    ereturn scalar variance_p90_abs_log_ratio = `receipt'[1,17]
    ereturn scalar variance_max_abs_log_ratio = `receipt'[1,18]
    ereturn scalar variance_log_correlation = `receipt'[1,19]
    ereturn matrix component_inference_receipt = `receipt'
    ereturn matrix component_augmentation_receipt = `augmentation'
    ereturn scalar inference_independent_units = `units'[1,3]
    ereturn scalar inference_nuisance_omitted = `units'[1,4]
    ereturn local result_family "generic"
    ereturn local inference "`inference'"
    ereturn local inference_method                             ///
        "matrix-free FEVC structured-variance component inference"
    ereturn local inference_deletion "`deletion' deletion; movers only"
    ereturn local inference_covariance                         ///
        "individual target intervals; separately checked joint covariance; no joint Gaussian Wald covariance for q1"
    ereturn local inference_variance_fit = cond(`variance_fit'==2,"residual_moments","cross_fitted_structured")
    ereturn local inference_rng "Counter-V1 Gaussian covariance and spectrum probes"
    ereturn local inference_model "`model'"
    ereturn local inference_reference = cond("`inference'"=="q1", ///
        "one estimated leading mode plus Gaussian remainder",     ///
        "q=0 Gaussian approximation with reported spectrum")
    ereturn local inference_variance_response                    ///
        "cross-fitted y_i times observation-deletion residual"
    ereturn local inference_variance_conditioning = cond(        ///
        "`model'"=="structured_common",                         ///
        "normalized midranks of leverage and three primitive target diagonals; polynomial interactions", ///
        "normalized midrank of leverage; quadratic polynomial")
    ereturn local inference_crossfit                             ///
        "five outcome-free outer folds; four-fold inner ridge selection; variance regression only"
    if `variance_fit'==2 {
        ereturn local inference_variance_response "full-model squared residual moments, adjusted for residual projection"
        ereturn local inference_crossfit "not applicable; residual-moment Gram fit without ridge or model selection"
        ereturn local inference_ordering "FEVC-OBSERVATION-DESIGN-ORDER-V1; design-only rows with distinct copy addresses"
    }
    ereturn local inference_positivity                           ///
        "floor 1e-8 times median maker-adjusted squared residual"
    ereturn local inference_kss_scope                            ///
        "pragmatic FEVC extension; not unrestricted-heteroskedastic KSS variance-product inference"
    ereturn local inference_spectral_rule                        ///
        "diagnostic only; no universal automatic q=0/q=1 cutoff"
    ereturn local inference_support_status "approximate_model_based"
    ereturn local inference_qualification "model-based approximation with source-specific development evidence and calibration limitations"
    ereturn local inference_capability                            ///
        "structured observation deletion; mover-only; unit frequency; generic JLA"
    ereturn local inference_population "movers"
    ereturn local inference_deletion_requested "`deletion'"
    ereturn local inference_deletion_selected "`deletion'"
    ereturn local inference_nuisance_requested "`e(nuisance)'"
    ereturn local inference_nuisance_selected "`e(nuisance)'"
    ereturn local inference_population_requested "movers"
    ereturn local inference_population_selected "movers"
    ereturn local inference_model_requested "`model'"
    ereturn local inference_model_selected "`model'"
    ereturn local inference_backend_requested "rust"
    ereturn local inference_backend_selected "rust"
    ereturn local inference_solver_requested "`e(preconditioner_requested)'"
    ereturn local inference_solver_selected "`e(preconditioner_selected)'"
    ereturn local inference_family_requested "generic"
    ereturn local inference_family_selected "generic"
    ereturn local inference_reference_requested = cond(          ///
        "`inference'"=="q1", "q1", "q0")
    ereturn local inference_reference_selected = cond(           ///
        "`inference'"=="q1", "q1 one-mode", "q0 diffuse")
    ereturn local inference_q_condition = cond(                  ///
        "`inference'"=="q1",                                   ///
        "one leading mode removed; remaining kernel and influence must be diffuse", ///
        "full kernel and influence must be diffuse")
    ereturn local inference_execution_scope                       ///
        "successful computation does not establish the target-specific asymptotic condition"
    ereturn local inference_variance_warning                      ///
        "structured variance assumptions; omitted variance drivers can invalidate SEs and intervals without changing component point estimates"
    ereturn local inference_reference_guarantee = cond(          ///
        "`inference'"=="q1",                                   ///
        "asymptotic at-least-nominal under the reference assumptions; potentially conservative; fitted-variance validity is an additional requirement", ///
        "Gaussian q=0 approximation requires strong identification")
    if "`deletion'"=="match" {
        ereturn local inference_method "Fixed-offset approximate match inference, ignoring nuisance-control estimation uncertainty."
        ereturn local inference_capability "structured match deletion; fixedoffset; mover-only; generic JLA"
        ereturn local inference_variance_response "squared collapsed fixed-offset residual moments, adjusted for the match-level FE projection"
        ereturn local inference_ordering "FEVC-MATCH-DESIGN-ORDER-V1; one numerical probe per independent match"
        ereturn local inference_variance_conditioning = cond(    ///
            "`model'"=="structured_common",                     ///
            "normalized midranks of match leverage, primitive target diagonals and regression mass; polynomial interactions", ///
            "normalized midrank of match leverage; quadratic polynomial")
        ereturn local inference_independence "independence across declared matches; within-match dependence enters aggregate-match variance"
        ereturn local inference_frequency "algebraic regression mass; one inferential unit per declared match"
        ereturn local inference_offset_warning "Control-estimation uncertainty is omitted; few controls do not guarantee negligible uncertainty or conditional validity."
        ereturn scalar inference_effective_matches = `units'[1,5]
        ereturn scalar inference_largest_mass_share = `units'[1,6]
        ereturn scalar inference_largest_leverage = `units'[1,7]
        ereturn scalar inference_smallest_maker = `units'[1,8]
    }
    ereturn matrix component_unit_receipt = `units'
    ereturn local status = cond("`inference'"=="q1",             ///
        "FEVC_STRUCTURED_Q1_INFERENCE","FEVC_STRUCTURED_Q0_INFERENCE")
    if "`inference'"=="q1" & `q1computed'<4 {
        ereturn local status "FEVC_STRUCTURED_Q1_PARTIAL"
    }
    if "`inference'"=="highrank" & `computed_targets'<4 {
        ereturn local status "FEVC_STRUCTURED_Q0_PARTIAL"
    }
    if `computed_targets'==0 ereturn local status "FEVC_STRUCTURED_INFERENCE_UNAVAILABLE"
end

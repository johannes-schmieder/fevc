program define _fevc_rust_component_post, eclass
    version 18.0
    args model inference simulations inferenceseed level primitive results ///
        mcse spectrum summaries folds cv receipt augmentation q1results q1raw
    ereturn matrix V_primitive = `primitive'
    ereturn matrix component_inference = `results'
    ereturn matrix component_trace_mcse = `mcse'
    ereturn matrix component_spectrum = `spectrum'
    ereturn matrix structured_variance_summary = `summaries'
    ereturn matrix structured_variance_folds = `folds'
    ereturn matrix structured_variance_cv = `cv'
    if "`inference'"=="q1" {
        ereturn matrix q1_inference = `q1results'
        ereturn matrix component_q1_diagnostics = `q1raw'
    }
    ereturn scalar level = `level'
    ereturn scalar inference_simulations = `simulations'
    ereturn scalar inference_seed = `inferenceseed'
    ereturn scalar inference_model_option_supplied = 1
    ereturn scalar inference_spectrum_probes = 128
    ereturn scalar inference_spectrum_iterations = 128
    local inference_solver_columns = 3+`simulations'+5*128+16*128+18
    if "`inference'"=="q1" local inference_solver_columns = `inference_solver_columns'+4
    ereturn scalar inference_solver_columns = `inference_solver_columns'
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
    ereturn local inference "`inference'"
    ereturn local inference_method                             ///
        "matrix-free FEVC structured-variance component inference"
    ereturn local inference_deletion "observation deletion; movers only"
    ereturn local inference_covariance                         ///
        "full joint primitive covariance with exact three-to-four map"
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
    ereturn local inference_positivity                           ///
        "floor 1e-8 times median maker-adjusted squared residual"
    ereturn local inference_kss_scope                            ///
        "pragmatic FEVC extension; not unrestricted-heteroskedastic KSS variance-product inference"
    ereturn local inference_spectral_rule                        ///
        "diagnostic only; no universal automatic q=0/q=1 cutoff"
    ereturn local status = cond("`inference'"=="q1",             ///
        "FEVC_STRUCTURED_Q1_INFERENCE","FEVC_STRUCTURED_Q0_INFERENCE")
end

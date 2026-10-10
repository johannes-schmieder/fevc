program define fevc__exact_inference_model_post, eclass
    version 18.0
    gettoken action : 0
    if "`action'"=="results" {
        gettoken action 0 : 0
        _fevc_exact_results_post `0'
        exit
    }
    args inference supplied model exact_structured_requested inference_joint_available
    if "`inference'"!="none" {
        if "`model'"=="" local model target_lowess
        ereturn local inference_model "`model'"
        ereturn local inference_kss_scope                         ///
            "target-specific MATLAB approximation; not the paper's unrestricted variance-product construction"
    }
    if inlist("`model'","structured_common","structured_leverage") {
        ereturn local inference_kss_scope "structured variance-model approximation; fixed nuisance offsets; estimation uncertainty omitted"
    }
    ereturn scalar inference_model_option_supplied = `supplied'
    if "`exact_structured_requested'"=="1" {
        ereturn local inference_method "joint exact residual-moment variance model with separate mover and stayer coefficients"
        ereturn local inference_covariance = cond("`inference'"=="highrank" & `inference_joint_available', ///
            "full joint component covariance","withheld; individual target results retained")
        ereturn local inference_independence "mutually independent mover units and physical stayer observations"
    }
    fevc__centering inference
end

program define _fevc_exact_results_post, eclass
    version 18.0
    args inference_requested inference_diagnostics level ///
        inference inferencesimulations inferenceseed ///
        inferencebins exact_structured_requested inference_joint_available ///
        inference_V_primitive inference_V inference_highrank ///
        inference_q1 inference_q1_status project_supplied ///
        projection_b projection_V projection_V_naive ///
        projection_results projecteffect projectweight ///
        project inference_varfit inference_spectrum
    if `inference_requested' {
        ereturn matrix inference_diagnostics = `inference_diagnostics'
        ereturn scalar level = `level'
        if "`inference'" != "none" {
            ereturn scalar inference_simulations = `inferencesimulations'
            ereturn scalar inference_seed = `inferenceseed'
            ereturn scalar inference_bins = `inferencebins'
            if !`exact_structured_requested' | ("`inference'"=="highrank" & `inference_joint_available') {
                ereturn matrix V_primitive = `inference_V_primitive'
            }
            if "`exact_structured_requested'"=="1" {
                if "`inference'"=="highrank" & `inference_joint_available' matrix `inference_V' = e(V)
                ereturn matrix component_raw_covariance = `inference_V'
                ereturn scalar component_joint_available = `inference_joint_available'
                ereturn scalar inference_joint_available = `inference_joint_available'
                ereturn scalar inference_joint_posted = ("`inference'"=="highrank" & `inference_joint_available')
                ereturn local inference_variance_fit "exact_residual_moments"
                ereturn local inference_gram_method "exact_coefficient_contraction"
                ereturn scalar inference_gram_probes = 0
                ereturn scalar variance_gram_rcond = `inference_varfit'[1,2]
                ereturn scalar variance_floor_share = `inference_varfit'[1,6]/`inference_varfit'[1,7]
                matrix colnames `inference_varfit' = active_terms gram_rcond gram_relres fit_relres positivity_floor floored units min_raw
                ereturn matrix residual_moment_diagnostics = `inference_varfit'
                matrix rownames `inference_spectrum' = worker_variance firm_variance worker_firm_covariance total_variance
                matrix colnames `inference_spectrum' = lambda1 lambda2 trace2_raw trace2 trace2_mcse trace_reconciliation leading_share leading_share_mcse remainder_leading_share max_mode_weight_sq leading_residual second_residual probes iterations max_influence_share
                ereturn matrix component_spectrum = `inference_spectrum'
                tempname q0status
                matrix `q0status' = J(4,1,0)
                forvalues row=1/4 {
                    if missing(`inference_highrank'[`row',2]) matrix `q0status'[`row',1] = 1
                }
                matrix rownames `q0status' = worker_variance firm_variance worker_firm_covariance total_variance
                matrix colnames `q0status' = status
                ereturn matrix q0_status = `q0status'
                ereturn local q0_status_codes "0 computed; 1 nonpositive variance"
                ereturn local inference_qualification "approximate model-based inference; see source-bound assessment"

                ereturn scalar inference_nuisance_omitted = 1
                ereturn scalar inference_mover_units = el(e(inference_diagnostics),1,8)
                ereturn scalar inference_stayer_units = el(e(inference_diagnostics),1,9)
                ereturn scalar inference_independent_units = el(e(inference_diagnostics),1,8)+el(e(inference_diagnostics),1,9)
            }
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
end

*! fevc display 0.5.0-alpha.1 02sep2026

program define _fevc_display
    version 18.0
    syntax [, DECOMPOSITIONonly FULL]

    if "`decompositiononly'" != "" {
        if "`full'" != "" _fevc_display_full
        else _fevc_display_decomp
        exit
    }

    local engine = lower(strtrim(`"`e(engine_selected)'"'))
    if "`engine'" == "" | upper("`engine'") == "NOT_APPLICABLE" {
        local engine "not applicable"
    }
    local preconditioner = lower(strtrim(`"`e(preconditioner_selected)'"'))
    if "`preconditioner'" == "" |                         ///
        upper("`preconditioner'") == "NOT_APPLICABLE" {
        local preconditioner "not applicable"
    }
    local backend = lower(strtrim(`"`e(backend_selected)'"'))
    if "`backend'" == "" local backend "not selected"

    local retained_percent = 100*e(N_retained)/e(N_requested)
    di as txt _newline "KSS leave-out variance decomposition"
    di as txt "Sample: " as result %12.0fc e(N_retained)          ///
        as txt " of " as result %12.0fc e(N_requested)           ///
        as txt " requested rows retained ("                      ///
        as result %5.1f `retained_percent' as txt "%)"
    if e(N_physical) != e(N_retained) {
        di as txt "Physical observations: " as result %12.0fc e(N_physical)
    }
    di as txt "Dimensions: " as result %10.0fc e(worker_levels)  ///
        as txt " workers; " as result %10.0fc e(firm_levels)     ///
        as txt " firms; " as result %10.0fc e(deletion_units)    ///
        as txt " deletion units"
    di as txt "Target: " as result "`e(target_population)'"
    di as txt "Deletion: " as result "`e(deletion)'"            ///
        as txt "; nuisance=" as result "`e(nuisance)'"
    di as txt "Method: " as result "`e(algorithm)'"              ///
        as txt "; backend=" as result "`backend'"
    if "`e(algorithm)'" == "jla" {
        di as txt "Numerics: engine=" as result "`engine'"       ///
            as txt "; preconditioner=" as result "`preconditioner'"
        di as txt "JLA work: probes=" as result %9.0fc e(probes) ///
            as txt "; seed=" as result %12.0fc e(seed)
    }

    if e(N_retained) < e(N_requested) {
        di as txt "Note: sample construction removed " as result ///
            %12.0fc e(N_requested)-e(N_retained)                  ///
            as txt " requested rows; type " as result            ///
            "estat sample" as txt " for the stage accounting."
    }
    if e(backend_fallback) == 1 {
        di as txt "Note: backend(auto) fell back to " as result  ///
            "`backend'" as txt " at " as result                  ///
            "`e(backend_fallback_phase)'" as txt " ("            ///
            as result "`e(backend_fallback_reason)'" as txt ")."
    }
    if upper(strtrim(`"`e(fallback_status)'"')) == "CMG_TO_DIAGONAL" {
        di as txt "Note: CMG setup fell back to diagonal PCG; "  ///
            as result "`e(fallback_message)'"
    }
    if "`e(stayers)'" == "both" & e(N_stayers) > 0 {
        di as txt "Note: movers use match blocks; stayers use observation deletion."
        di as txt "      Stayer correction is not match-robust."
    }

    _fevc_display_decomp
    _fevc_display_inference

    if inlist("`e(inference)'", "highrank", "q1") {
        di as txt _newline "Inference: component e(V) is posted. " ///
            "Numerical MCSE is computational only."
    }
    else if "`e(projection_effect)'" != "" {
        di as txt _newline "Projection covariance: " as result  ///
            "e(projection_V)" as txt "; component e(V) is not posted."
    }
    else if e(numerical_mcse_available) {
        di as txt _newline "Inference: not requested; e(V) not posted; " ///
            "MCSE is numerical, not sampling."
    }
    else {
        di as txt _newline "Inference: not requested; e(V) is not posted."
    }
    di as txt "Details: " as result "estat decomposition, full" ///
        as txt "; " as result "estat sample"
    di as txt "         " as result "estat computation" as txt ///
        "; " as result "estat diagnostics"
end

program define _fevc_display_decomp
    version 18.0
    tempname decomposition mcse
    matrix `decomposition' = e(decomposition)

    di as txt _newline "Additive worker-firm decomposition"
    di as txt "{hline 79}"
    di as txt %-22s "Component" %14s "Plug-in"                  ///
        %15s "Estimated bias" %14s "KSS corrected"             ///
        %14s "% of Var(Y)"
    di as txt "{hline 79}"
    forvalues row = 1/4 {
        if `row' == 1 local row_label "Worker variance"
        else if `row' == 2 local row_label "Firm variance"
        else if `row' == 3 local row_label "Sorting (2 x cov.)"
        else local row_label "Total worker-firm var."
        di as txt %-22s "`row_label'" as result                   ///
            %14.7g `decomposition'[`row',1]                       ///
            %15.7g `decomposition'[`row',2]                       ///
            %14.7g `decomposition'[`row',3]                       ///
            %14.2f 100*`decomposition'[`row',5]
    }
    di as txt "{hline 79}"
    di as txt "KSS corrected = plug-in - estimated bias; "       ///
        "sorting is 2 x covariance."
    di as txt "Target-weighted Var(Y): " as result               ///
        %13.7g e(target_outcome_variance)

    if e(numerical_mcse_available) {
        matrix `mcse' = e(numerical_mcse)
        di as txt "JLA numerical MCSE: worker=" as result         ///
            %10.5g `mcse'[1,1] as txt "; firm=" as result        ///
            %10.5g `mcse'[1,2]
        di as txt "                    sorting=" as result         ///
            %10.5g 2*`mcse'[1,3] as txt "; total=" as result    ///
            %10.5g `mcse'[1,4]
        di as txt "  Conditional on the leverage sketch; not sampling standard errors."
    }
end

program define _fevc_display_full
    version 18.0
    tempname levels decomposition shares
    matrix `levels' = (e(plugin)' , e(correction)' , e(kss)')
    matrix `decomposition' = e(decomposition)
    matrix `shares' = 100*e(decomposition)[1..4,4..7]

    di as txt _newline "Full variance-component accounting"
    di as txt _newline "Quadratic-form targets (raw covariance)"
    di as txt "{hline 78}"
    di as txt %-26s "Component" %17s "Plug-in"                  ///
        %17s "Estimated bias" %17s "KSS corrected"
    di as txt "{hline 78}"
    forvalues row = 1/4 {
        if `row' == 1 local row_label "Worker variance"
        else if `row' == 2 local row_label "Firm variance"
        else if `row' == 3 local row_label "Worker-firm covariance"
        else local row_label "Total worker-firm variance"
        di as txt %-26s "`row_label'" as result                  ///
            %17.7g `levels'[`row',1] %17.7g `levels'[`row',2]   ///
            %17.7g `levels'[`row',3]
    }
    di as txt "{hline 78}"

    di as txt _newline "Additive decomposition"
    di as txt "{hline 78}"
    di as txt %-26s "Component" %17s "Plug-in"                  ///
        %17s "Estimated bias" %17s "KSS corrected"
    di as txt "{hline 78}"
    forvalues row = 1/4 {
        if `row' == 1 local row_label "Worker variance"
        else if `row' == 2 local row_label "Firm variance"
        else if `row' == 3 local row_label "Sorting: 2 x covariance"
        else local row_label "Total worker-firm variance"
        di as txt %-26s "`row_label'" as result                  ///
            %17.7g `decomposition'[`row',1]                       ///
            %17.7g `decomposition'[`row',2]                       ///
            %17.7g `decomposition'[`row',3]
    }
    di as txt "{hline 78}"

    di as txt _newline "Shares (percent; missing when a denominator is nonpositive)"
    di as txt "{hline 78}"
    di as txt %-26s "" %26s "Target-weighted Var(Y)"            ///
        %26s "Worker-firm total"
    di as txt %-26s "Component" %13s "Plug-in" %13s "Corrected" ///
        %13s "Plug-in" %13s "Corrected"
    di as txt "{hline 78}"
    forvalues row = 1/4 {
        if `row' == 1 local row_label "Worker variance"
        else if `row' == 2 local row_label "Firm variance"
        else if `row' == 3 local row_label "Sorting: 2 x covariance"
        else local row_label "Total worker-firm variance"
        di as txt %-26s "`row_label'" as result                  ///
            %13.2f `shares'[`row',1] %13.2f `shares'[`row',2]   ///
            %13.2f `shares'[`row',3] %13.2f `shares'[`row',4]
    }
    di as txt "{hline 78}"

    di as txt _newline "Outcome variance and descriptive fit"
    di as txt "Target-weighted Var(Y): " as result               ///
        %13.7g e(target_outcome_variance)
    di as txt "KSS-corrected worker-firm total: " as result      ///
        %13.7g e(kss)[1,4]
    di as txt "Frequency-weighted Var(Y): " as result            ///
        %13.7g e(regression_outcome_variance)
    di as txt "Full-model explained variance (includes controls): " ///
        as result %13.7g e(full_model_explained_variance)
    di as txt "Full-model explained share of Var(Y): " as result ///
        %9.2f 100*e(full_model_explained_share) as txt "%"
end

program define _fevc_display_inference
    version 18.0
    tempname component_inference q1_inference projection_inference
    tempname component_spectrum variance_summary q1_diagnostics

    if inlist("`e(inference)'", "highrank", "q1") {
        matrix `component_inference' = e(component_inference)
        di as txt _newline "Econometric component inference ("    ///
            as result "`e(inference)'" as txt "; "               ///
            as result %4.1f e(level) as txt "% confidence level)"
        di as txt "{hline 78}"
        di as txt %-18s "Component" %12s "Estimate" %12s "Std. err." ///
            %10s "P>|z|" %13s "Lower" %13s "Upper"
        di as txt "{hline 78}"
        forvalues row = 1/4 {
            if `row' == 1 local row_label "Worker variance"
            else if `row' == 2 local row_label "Firm variance"
            else if `row' == 3 local row_label "Worker-firm cov."
            else local row_label "Total worker-firm"
            local pvalue = 2*normal(-abs(                         ///
                `component_inference'[`row',1]/                   ///
                `component_inference'[`row',2]))
            di as txt %-18s "`row_label'" as result              ///
                %12.6g `component_inference'[`row',1]             ///
                %12.6g `component_inference'[`row',2]             ///
                %10.4f `pvalue'                                   ///
                %13.6g `component_inference'[`row',3]             ///
                %13.6g `component_inference'[`row',4]
        }
        di as txt "{hline 78}"
    }
    if "`e(inference)'" == "q1" {
        matrix `q1_inference' = e(q1_inference)
        di as txt _newline "Rank-one weak-identification intervals"
        di as txt "{hline 78}"
        di as txt %-26s "Component" %13s "AM lower" %13s "AM upper" ///
            %13s "F" %13s "Curvature"
        di as txt "{hline 78}"
        forvalues row = 1/4 {
            if `row' == 1 local row_label "Worker variance"
            else if `row' == 2 local row_label "Firm variance"
            else if `row' == 3 local row_label "Worker-firm covariance"
            else local row_label "Total worker-firm variance"
            di as txt %-26s "`row_label'" as result              ///
                %13.6g `q1_inference'[`row',5]                    ///
                %13.6g `q1_inference'[`row',6]                    ///
                %13.6g `q1_inference'[`row',15]                   ///
                %13.6g `q1_inference'[`row',16]
        }
        di as txt "{hline 78}"
    }
    if inlist("`e(inference_model)'", "structured_common", "structured_leverage") {
        matrix `component_spectrum' = e(component_spectrum)
        di as txt _newline "Structured-model credibility diagnostics"
        di as txt "{hline 78}"
        if "`e(inference)'" == "q1" {
            di as txt %-18s "Component" %12s "Lead share" %12s "Trace MCSE" ///
                %12s "Remainder" %12s "Max mode" %12s "Max infl."
            matrix `q1_diagnostics' = e(component_q1_diagnostics)
        }
        else {
            di as txt %-18s "Component" %15s "Lead share" %15s "Trace MCSE" ///
                %15s "Max mode" %15s "Max infl."
        }
        di as txt "{hline 78}"
        forvalues row = 1/4 {
            if `row' == 1 local row_label "Worker variance"
            else if `row' == 2 local row_label "Firm variance"
            else if `row' == 3 local row_label "Worker-firm cov."
            else local row_label "Total worker-firm"
            if "`e(inference)'" == "q1" {
                di as txt %-18s "`row_label'" as result             ///
                    %12.5f `component_spectrum'[`row',7]            ///
                    %12.3g `component_spectrum'[`row',8]            ///
                    %12.5f `component_spectrum'[`row',9]            ///
                    %12.5f `component_spectrum'[`row',10]           ///
                    %12.5f `q1_diagnostics'[`row',14]
            }
            else {
                di as txt %-18s "`row_label'" as result             ///
                    %15.5f `component_spectrum'[`row',7]            ///
                    %15.3g `component_spectrum'[`row',8]            ///
                    %15.5f `component_spectrum'[`row',10]           ///
                    %15.5f `component_spectrum'[`row',15]
            }
        }
        di as txt "{hline 78}"
        matrix `variance_summary' = e(structured_variance_summary)
        local model_row = cond("`e(inference_model)'"=="structured_common",1,2)
        di as txt "Variance model: " as result "`e(inference_model)'" as txt  ///
            "; floor share=" as result %7.4f `variance_summary'[`model_row',7] ///
            as txt "; boundary share=" as result %7.4f                    ///
            `variance_summary'[`model_row',9]
        di as txt "Assumption: " as result "structured conditional variance" ///
            as txt "; not unrestricted-KSS variance-product inference."
        di as txt "No universal spectral cutoff is imposed; assess concentration " ///
            "for the intended asymptotic sequence."
    }
    if "`e(projection_effect)'" != "" {
        matrix `projection_inference' = e(projection_results)
        local projection_rows : rownames `projection_inference'
        di as txt _newline "KSS projection of " as result        ///
            "`e(projection_effect)'" as txt " effects ("         ///
            as result "`e(projection_weight)'" as txt " weights; " ///
            as result %4.1f e(level) as txt "% confidence level)"
        di as txt "Deletion: " as result "`e(inference_deletion)'"
        di as txt "{hline 78}"
        di as txt %-18s "Term" %12s "Estimate" %12s "KSS SE"   ///
            %10s "P>|z|" %13s "Lower" %13s "Upper"
        di as txt "{hline 78}"
        forvalues row = 1/`=rowsof(`projection_inference')' {
            local row_label : word `row' of `projection_rows'
            di as txt %-18s "`row_label'" as result              ///
                %12.6g `projection_inference'[`row',1]            ///
                %12.6g `projection_inference'[`row',2]            ///
                %10.4f `projection_inference'[`row',4]            ///
                %13.6g `projection_inference'[`row',5]            ///
                %13.6g `projection_inference'[`row',6]
        }
        di as txt "{hline 78}"
        di as txt "Constant: normalization-dependent. "          ///
            "Projection slopes: location-invariant."
    }
end

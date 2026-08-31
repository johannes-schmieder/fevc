program define _fevc_display
    version 18.0
    tempname levels additive shares mcse hybrid_levels
    tempname component_inference q1_inference projection_inference
    local engine `"`e(engine_selected)'"'
    if `"`engine'"' == "" |                                   ///
        upper(strtrim(`"`engine'"')) == "NOT_APPLICABLE" {
        local engine "not applicable"
    }
    local preconditioner `"`e(preconditioner_selected)'"'
    if `"`preconditioner'"' == "" |                            ///
        upper(strtrim(`"`preconditioner'"')) == "NOT_APPLICABLE" {
        local preconditioner "not applicable"
    }
    else local preconditioner = lower(strtrim(`"`preconditioner'"'))

    di as txt _newline "KSS leave-out variance decomposition"
    di as txt "Sample: " as result %12.0fc e(N_retained)          ///
        as txt " stored rows; " as result %12.0fc e(N_physical)  ///
        as txt " physical observations"
    di as txt "Dimensions: " as result %10.0fc e(worker_levels)  ///
        as txt " worker levels; " as result %10.0fc e(firm_levels) ///
        as txt " firm levels"
    di as txt "Graph: " as result %10.0fc e(deletion_units)      ///
        as txt " deletion units"
    di as txt "Design: deletion=" as result "`e(deletion)'"      ///
        as txt "  nuisance=" as result "`e(nuisance)'"          ///
        as txt "  target=" as result "`e(target_population)'"
    di as txt "Computation: algorithm=" as result "`e(algorithm)'" ///
        as txt "  engine=" as result "`engine'"
    di as txt "Solver: preconditioner=" as result "`preconditioner'"

    matrix `levels' = (e(plugin)' , e(correction)' , e(kss)')
    di as txt _newline "Quadratic-form targets"
    di as txt "{hline 78}"
    di as txt %-26s "Component" %17s "Plug-in"                  ///
        %17s "Bias correction" %17s "KSS corrected"
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

    if "`e(stayers)'" == "both" {
        di as txt _newline "Mixed-deletion population"
        di as txt "Deletion: " as result                         ///
            "`e(stayer_hybrid_deletion)'"
        di as txt "Caution: " as result                          ///
            "not match-robust for stayers"
    }

    matrix `additive' = (e(decomposition)[1..4,1],                ///
        e(decomposition)[1..4,3])
    di as txt _newline "Additive worker-firm decomposition"
    di as txt "(worker variance + firm variance + 2 x covariance = total)"
    di as txt "{hline 60}"
    di as txt %-26s "Component" %17s "Plug-in" %17s "KSS corrected"
    di as txt "{hline 60}"
    forvalues row = 1/4 {
        if `row' == 1 local row_label "Worker variance"
        else if `row' == 2 local row_label "Firm variance"
        else if `row' == 3 local row_label "Sorting: 2 x covariance"
        else local row_label "Total worker-firm variance"
        di as txt %-26s "`row_label'" as result                  ///
            %17.7g `additive'[`row',1] %17.7g `additive'[`row',2]
    }
    di as txt "{hline 60}"

    matrix `shares' = 100*e(decomposition)[1..4,4..7]
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

    di as txt _newline "Variance and fit summary"
    di as txt "Target-weighted Var(Y): " as result               ///
        %13.6g e(target_outcome_variance)
    di as txt "KSS-corrected worker-firm total: " as result      ///
        %13.6g e(kss)[1,4]
    di as txt "Frequency-weighted Var(Y): " as result            ///
        %13.6g e(regression_outcome_variance)
    di as txt "Descriptive full-model fit (frequency weighted; includes controls)"
    di as txt "  Explained variance: " as result                ///
        %13.6g e(full_model_explained_variance)
    di as txt "  Explained share of Var(Y): " as result         ///
        %9.2f 100*e(full_model_explained_share) as txt "%"
    if e(numerical_mcse_available) {
        matrix `mcse' = e(numerical_mcse)'
        di as txt _newline "JLA numerical MCSE, conditional on the leverage sketch"
        di as txt "{hline 47}"
        di as txt %-26s "Component" %20s "Numerical MCSE"
        di as txt "{hline 47}"
        forvalues row = 1/4 {
            if `row' == 1 local row_label "Worker variance"
            else if `row' == 2 local row_label "Firm variance"
            else if `row' == 3 local row_label "Worker-firm covariance"
            else local row_label "Total worker-firm variance"
            di as txt %-26s "`row_label'" as result              ///
                %20.7g `mcse'[`row',1]
        }
        di as txt "{hline 47}"
    }
    if inlist("`e(inference)'", "highrank", "q1") {
        matrix `component_inference' = e(component_inference)
        di as txt _newline "Econometric component inference ("        ///
            as result "`e(inference)'" as txt "; "                    ///
            as result %4.1f e(level) as txt "% level)"
        di as txt "{hline 78}"
        di as txt %-26s "Component" %13s "Estimate" %13s "Std. err." ///
            %13s "Lower" %13s "Upper"
        di as txt "{hline 78}"
        forvalues row = 1/4 {
            if `row' == 1 local row_label "Worker variance"
            else if `row' == 2 local row_label "Firm variance"
            else if `row' == 3 local row_label "Worker-firm covariance"
            else local row_label "Total worker-firm variance"
            di as txt %-26s "`row_label'" as result                  ///
                %13.6g `component_inference'[`row',1]                ///
                %13.6g `component_inference'[`row',2]                ///
                %13.6g `component_inference'[`row',3]                ///
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
            di as txt %-26s "`row_label'" as result                  ///
                %13.6g `q1_inference'[`row',5]                       ///
                %13.6g `q1_inference'[`row',6]                       ///
                %13.6g `q1_inference'[`row',15]                      ///
                %13.6g `q1_inference'[`row',16]
        }
        di as txt "{hline 78}"
    }
    if "`e(projection_effect)'" != "" {
        matrix `projection_inference' = e(projection_results)
        local projection_rows : rownames `projection_inference'
        di as txt _newline "KSS projection of " as result          ///
            "`e(projection_effect)'" as txt " effects"
        di as txt "{hline 78}"
        di as txt %-26s "Term" %13s "Estimate" %13s "KSS SE"     ///
            %13s "Lower" %13s "Upper"
        di as txt "{hline 78}"
        forvalues row = 1/`=rowsof(`projection_inference')' {
            local row_label : word `row' of `projection_rows'
            di as txt %-26s "`row_label'" as result                  ///
                %13.6g `projection_inference'[`row',1]               ///
                %13.6g `projection_inference'[`row',2]               ///
                %13.6g `projection_inference'[`row',5]               ///
                %13.6g `projection_inference'[`row',6]
        }
        di as txt "{hline 78}"
    }
    if inlist("`e(inference)'", "highrank", "q1") {
        di as txt _newline "Component e(V) is posted; numerical MCSE remains " ///
            "a separate computational diagnostic."
    }
    else if "`e(projection_effect)'" != "" {
        di as txt _newline "Projection covariance is posted separately; " ///
            "component e(V) is not posted."
    }
    else {
        di as txt _newline "Point estimates only; numerical MCSE is not " ///
            "econometric inference."
        di as txt "e(V) is not posted."
    }
end

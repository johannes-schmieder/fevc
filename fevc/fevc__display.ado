*! Displays fevc estimates, inference, and numerical diagnostics from e().

program define fevc__display
    version 15.0
    syntax [, DECOMPOSITIONonly FULL INFERENCEdiagnosticsonly NUMERICALdiagnosticsonly]

    quietly fevc__stayer_population_post

    if "`numericaldiagnosticsonly'"!="" {
        _fevc_display_numerical
        exit
    }
    // estat inference: the details of the standard errors of the components.
    if "`inferencediagnosticsonly'" != "" {
        if "`e(inference_variance_fit)'"=="repeat_measurement" _fevc_display_inference_details
        else if "`e(inference_variance_fit)'"=="leaveout_residual" _fevc_display_residual_se
        else _fevc_display_structured
        exit
    }

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
    if "`e(deletion)'" == "match" & "`e(stayers)'" == "both" & e(N_stayers) > 0 {
        di as txt "Note: movers use match blocks; stayers use observation deletion."
        di as txt "      Stayer correction is not match-robust."
    }

    _fevc_display_decomp
    _fevc_display_inference

    local se_details 0
    if "`e(inference_variance_fit)'"=="repeat_measurement" {
        _fevc_display_interval_notes
        local se_details 1
    }
    else if "`e(inference_variance_fit)'"=="leaveout_residual" {
        _fevc_display_residual_se
    }
    else if "`e(inference_variance_fit)'"!="" & e(inference_joint_posted)==0 {
        di as txt _newline "Inference: use the reported individual intervals; component e(V) is not posted."
    }
    else if inlist("`e(inference)'","highrank","q1") {
        di as txt _newline "Inference: component e(V) is posted. " ///
            "Numerical MCSE is computational only."
    }
    else if "`e(projection_effect)'" != "" {
        di as txt _newline "Projection covariance: " as result  ///
            "e(projection_V)" as txt "; component e(V) is not posted."
    }
    else if "$VCKSS_SE_AUTO"=="1" | (e(inference_auto)==1 & `"`e(inference_auto_reason)'"'!="") {
        local reason `"$VCKSS_SE_AUTO_REASON"'
        if `"`reason'"'=="" local reason `"`e(inference_auto_reason)'"'
        di as txt _newline "{p 0 2 2}Standard errors: not computed; `reason'.{p_end}"
        di as txt "e(V) is not posted. Numerical MCSE is computational, not sampling."
    }
    else if e(mcse_available)==1 {
        di as txt _newline "Inference: not requested; e(V) not posted; " ///
            "MCSE is numerical, not sampling."
    }
    else {
        di as txt _newline "Inference: not requested; e(V) is not posted."
    }
    if `se_details' {
        di as txt "Details: " as result "estat inference" as txt "; "  ///
            as result "estat decomposition, full" as txt "; "        ///
            as result "estat sample" as txt ";"
    }
    else {
        di as txt "Details: " as result "estat decomposition, full" ///
            as txt "; " as result "estat sample"
    }
    di as txt "         " as result "estat computation" as txt ///
        "; " as result "estat diagnostics"
end

program define _fevc_display_decomp
    version 15.0
    tempname decomposition
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

    _fevc_display_mcse_line

end

program define _fevc_display_full
    version 15.0
    tempname levels decomposition shares
    matrix `levels' = (e(plugin)' , e(correction)' , e(kss)')
    matrix `decomposition' = e(decomposition)
    matrix `shares' = e(decomposition)
    matrix `shares' = 100*`shares'[1..4,4..7]

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
        %13.7g el(e(kss),1,4)
    di as txt "Frequency-weighted Var(Y): " as result            ///
        %13.7g e(regression_outcome_variance)
    di as txt "Full-model explained variance (includes controls): " ///
        as result %13.7g e(full_model_explained_variance)
    di as txt "Full-model explained share of Var(Y): " as result ///
        %9.2f 100*e(full_model_explained_share) as txt "%"
end

program define _fevc_display_inference
    version 15.0
    tempname projection_inference

    // Standard errors of the components: one table in every mode, one
    // interval per component (normal or q = 1).
    capture confirm matrix e(inference_intervals)
    local by_component = (_rc==0)
    if !`by_component' & inlist("`e(inference)'","highrank","q1") & ///
        !inlist("`e(inference_model)'","structured_common","structured_leverage") {
        capture confirm matrix e(component_inference)
        local by_component = (_rc==0)
    }
    if `by_component' _fevc_display_intervals
    if inlist("`e(inference_model)'", "structured_common", "structured_leverage") {
        _fevc_display_structured
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

// The 4 x 16 layout of e(inference_intervals) (see fevc__residual_se_post),
// or the same layout from the normal intervals of e(component_inference)
// when no leading direction was computed. Leaves the request (auto,
// highrank or q1) in the caller's local request.
program define _fevc_interval_table
    version 15.0
    args table
    capture confirm matrix e(inference_intervals)
    if !_rc {
        matrix `table' = e(inference_intervals)
        c_local request "`e(inference_leading_request)'"
        exit
    }
    tempname normal
    matrix `normal' = e(component_inference)
    matrix `table' = J(4,16,.)
    forvalues t = 1/4 {
        forvalues j = 1/4 {
            matrix `table'[`t',`j'] = `normal'[`t',`j']
        }
        if !missing(`normal'[`t',2]) {
            matrix `table'[`t',7] = 0
            matrix `table'[`t',8] = `normal'[`t',3]
            matrix `table'[`t',9] = `normal'[`t',4]
        }
    }
    c_local request "`e(inference)'"
end

// A share as the tables print it: a percentage, >100% for a ratio of two
// estimates above one, . when there is none.
program define _fevc_share_text
    version 15.0
    args share
    local text "."
    if !missing(`share') {
        if `share'>1 local text ">100%"
        else if `share'<.0005 local text "<0.1%"
        else {
            local text : display %5.1f 100*`share'
            local text = strtrim("`text'")+"%"
        }
    }
    c_local share_text "`text'"
end

// The rule's threshold as a percentage, e.g. 10.
program define _fevc_threshold_text
    version 15.0
    local threshold = e(inference_share_threshold)
    if missing(`threshold') local threshold .1
    local text : display %9.0g 100*`threshold'
    c_local threshold_text = strtrim("`text'")
end

// One interval per component: the normal interval with its standard error,
// or the q = 1 interval, which has none; the same columns in every mode.
// 78 columns: label 17, four numbers of 11, interval 10, share 7.
program define _fevc_display_intervals
    version 15.0
    tempname table
    _fevc_interval_table `table'
    _fevc_threshold_text
    local level : display %9.0g e(level)
    local level = strtrim("`level'")
    if "`request'"=="highrank" {
        local title "(* normal interval requested by inference(highrank))"
    }
    else if "`request'"=="q1" {
        local title "(* q = 1 interval requested by inference(q1))"
    }
    else local title "(q = 1 interval when the lead share exceeds `threshold_text'%)"
    di as txt _newline "Component inference `title'"
    di as txt "{hline 78}"
    di as txt %71s "" %7s "Lead"
    di as txt %-17s "Component" %11s "Estimate" %11s "Std. err." ///
        %22s "[`level'% conf. interval]" %10s "Interval" %7s "share"
    di as txt "{hline 78}"
    forvalues row = 1/4 {
        if `row' == 1 local row_label "Worker variance"
        else if `row' == 2 local row_label "Firm variance"
        else if `row' == 3 local row_label "Worker-firm cov."
        else local row_label "Total worker-firm"
        local regime = `table'[`row',7]
        local kind = cond(`regime'==1,"q = 1",cond(`regime'==0,"normal","none"))
        if ("`request'"=="highrank" & `regime'==0) | ///
            ("`request'"=="q1" & `regime'==1) local kind "`kind'*"
        // The rule does not apply to the variance of the sum.
        local share_text "."
        if `row'<4 _fevc_share_text `=`table'[`row',10]'
        if `regime'==0 {
            di as txt %-17s "`row_label'" as result                 ///
                "  " %9.0g `table'[`row',1] "  " %9.0g `table'[`row',2] ///
                "  " %9.0g `table'[`row',8] "  " %9.0g `table'[`row',9] ///
                as txt %10s "`kind'" as result %7s "`share_text'"
        }
        else {
            di as txt %-17s "`row_label'" as result                 ///
                "  " %9.0g `table'[`row',1] as txt %11s ""             ///
                as result "  " %9.0g `table'[`row',8] "  " %9.0g `table'[`row',9] ///
                as txt %10s "`kind'" as result %7s "`share_text'"
        }
    }
    di as txt "{hline 78}"
end

// At most a few lines under the table: what the share and the q = 1
// interval are, whether e(V) is posted and why not. The method, the
// assumptions and every diagnostic are in estat inference.
program define _fevc_display_interval_notes
    version 15.0
    tempname table
    capture confirm matrix e(inference_intervals)
    if _rc {
        capture confirm matrix e(component_inference)
        if _rc {
            if e(inference_joint_posted)==1 di as txt "e(V) is posted for the four components."
            else di as txt "e(V) is not posted; see estat inference."
            exit
        }
    }
    _fevc_interval_table `table'
    _fevc_threshold_text
    local threshold = e(inference_share_threshold)
    if missing(`threshold') local threshold .1
    local shares 0
    local q1 0
    local none 0
    local missing_se 0
    local weighty 0
    local unavailable 0
    local would 0
    local unrequested 0
    forvalues row = 1/4 {
        local regime = `table'[`row',7]
        local share = `table'[`row',10]
        local status = `table'[`row',16]
        if `row'<4 & !missing(`share') local shares 1
        if `regime'==1 local ++q1
        if missing(`regime') local ++none
        if missing(`table'[`row',2]) local missing_se 1
        if `regime'==1 & `table'[`row',14]>.1 & !missing(`table'[`row',14]) local weighty 1
        if "`request'"=="auto" & `row'<4 & `regime'==0 & `status'!=0 & ///
            !missing(`share') & `share'>`threshold' local unavailable 1
        // What the default rule would have reported under inference(highrank).
        if "`request'"=="highrank" & `row'<4 & `status'==0 &       ///
            (missing(`table'[`row',2]) | (!missing(`share') & `share'>`threshold')) {
            local ++would
        }
        if "`request'"=="q1" & `regime'==0 local unrequested 1
    }
    if `shares' {
        di as txt "Lead share: leading direction's share of the estimated sampling variance."
    }
    if `q1' {
        di as txt "q = 1: Andrews-Mikusheva interval (KSS 2020, Sec. 7.1); no standard error."
    }
    if "`request'"=="highrank" & `shares' {
        if `would'==0 {
            di as txt "The default rule (lead share above `threshold_text'%) gives the same intervals."
        }
        else {
            local components = cond(`would'==1,"1 component would get","`would' components would get")
            di as txt "Under the default rule (lead share above `threshold_text'%), `components' q = 1."
        }
    }
    if `unrequested' {
        di as txt "normal (no *): that component has no q = 1 interval; see estat inference."
    }
    if `none' {
        di as txt "none: no interval; the variance estimates are not positive."
    }
    if `unavailable' {
        di as txt "Warning: q = 1 needed but unavailable for a component; normal may under-cover."
    }
    if `weighty' {
        di as txt "Warning: a q = 1 direction rests on few deletion units; see estat inference."
    }
    if "`e(centering)'"=="corrected" | "$VCKSS_CENTERING"=="corrected" {
        di as txt "Standard errors treat the centering(corrected) increment as fixed."
    }
    if e(inference_joint_posted)==1 {
        di as txt "e(V) is posted: every component has a normal interval."
    }
    else if `q1' di as txt "e(V) is not posted: a component has a q = 1 interval."
    else if `missing_se' di as txt "e(V) is not posted: a variance estimate is not positive."
    else di as txt "e(V) is not posted: the estimated covariance is not admissible."
end

// estat inference: everything about the standard errors of the components
// that the main display leaves out, component by component, with the method
// and its assumptions. 78 columns: label 26, four components of 13.
program define _fevc_display_inference_details
    version 15.0
    tempname table lead rank
    _fevc_interval_table `table'
    _fevc_threshold_text
    local threshold = e(inference_share_threshold)
    if missing(`threshold') local threshold .1
    local has_lead 0
    capture confirm matrix e(inference_leading)
    if !_rc {
        matrix `lead' = e(inference_leading)
        local has_lead 1
    }
    local has_rank 0
    capture confirm matrix e(inference_effective_rank)
    if !_rc {
        matrix `rank' = e(inference_effective_rank)
        local has_rank 1
    }
    local level : display %9.0g e(level)
    local level = strtrim("`level'")
    local draws : display %12.0fc e(inference_draws)
    local units : display %15.0fc e(inference_independent_units)
    di as txt _newline "Standard errors of the components"
    di as txt "Method: " as result "repeat-measurement" as txt "; "   ///
        as result strtrim("`draws'") as txt " pseudo-noise draws; " ///
        as result strtrim("`units'") as txt " deletion units; level " ///
        as result "`level'%"
    if "`request'"=="highrank" {
        di as txt "Request: " as result "inference(highrank)" as txt ///
            ": the normal interval of every component"
    }
    else if "`request'"=="q1" {
        di as txt "Request: " as result "inference(q1)" as txt   ///
            ": the q = 1 interval wherever it exists"
    }
    else {
        di as txt "Request: " as result "inference(auto)" as txt ///
            ": q = 1 interval when the lead share exceeds `threshold_text'%"
    }
    di as txt "{hline 78}"
    di as txt %-26s "" %13s "Worker" %13s "Firm" %13s "Worker-firm" %13s "Total"
    di as txt %-26s "" %13s "variance" %13s "variance" %13s "covariance" ///
        %13s "worker-firm"
    di as txt "{hline 78}"
    _fevc_detail_numbers "Estimate" `table' 1
    _fevc_detail_numbers "Std. err. (normal form)" `table' 2
    _fevc_detail_numbers "Normal interval, lower" `table' 3
    _fevc_detail_numbers "                 upper" `table' 4
    _fevc_detail_numbers "q = 1 interval, lower" `table' 5
    _fevc_detail_numbers "                upper" `table' 6
    _fevc_detail_numbers "q = 1 curvature" `table' 11
    _fevc_detail_numbers "q = 1 critical value" `table' 12
    forvalues t = 1/4 {
        local status = `table'[`t',16]
        local text`t' = cond(`status'==0,"computed",cond(`status'==1,"var. <= 0", ///
            cond(`status'==2,"singular",cond(`status'==3,"failed",       ///
            cond(missing(`status'),".","code `status'")))))
    }
    _fevc_detail_row "q = 1 status" "`text1'" "`text2'" "`text3'" "`text4'"
    di as txt "{hline 78}"
    forvalues t = 1/4 {
        local regime = `table'[`t',7]
        local share = `table'[`t',10]
        local normal_ok = !missing(`table'[`t',2])
        local q1_ok = (`table'[`t',16]==0)
        local kind`t' = cond(`regime'==1,"q = 1",cond(`regime'==0,"normal","none"))
        if ("`request'"=="highrank" & `regime'==0) | ///
            ("`request'"=="q1" & `regime'==1) local kind`t' "`kind`t''*"
        // Why the component has the interval it has.
        if missing(`regime') local why`t' "none exists"
        else if inlist("`request'","highrank","q1") {
            local why`t' = cond(`regime'==1 | "`request'"=="highrank","requested","no q = 1")
        }
        else if `t'==4 local why`t' "sum: normal"
        else if `regime'==1 & !`normal_ok' local why`t' "var. <= 0"
        else if `regime'==1 local why`t' "share > `threshold_text'%"
        else if !missing(`share') & `share'>`threshold' & !`q1_ok' local why`t' "no q = 1"
        else local why`t' "share <= `threshold_text'%"
        _fevc_share_text `=`table'[`t',10]'
        local share`t' "`share_text'"
        local converged`t' = cond(`table'[`t',15]==1,"yes",cond(missing(`table'[`t',15]),".","no"))
    }
    _fevc_detail_row "Reported interval" "`kind1'" "`kind2'" "`kind3'" "`kind4'"
    _fevc_detail_row "Reason" "`why1'" "`why2'" "`why3'" "`why4'"
    _fevc_detail_row "Lead share" "`share1'" "`share2'" "`share3'" "`share4'"
    if `has_lead' {
        _fevc_detail_numbers "Leading eigenvalue lambda1" `lead' 5
        _fevc_detail_numbers "Second eigenvalue lambda2" `lead' 6
    }
    _fevc_detail_numbers "KSS's Lindeberg max w_i1^2" `table' 14
    if `has_rank' _fevc_detail_numbers "Effective rank" `rank' 1
    _fevc_detail_row "Search converged" "`converged1'" "`converged2'" ///
        "`converged3'" "`converged4'"
    if `has_lead' _fevc_detail_numbers "Search solves" `lead' 3
    di as txt "{hline 78}"
    // Each paragraph is one display: SMCL wraps it at the line size.
    if "`request'"=="highrank" {
        di as txt "{p 0 2 2}* Requested. The default reports the q = 1 interval "   ///
            "when the lead share exceeds `threshold_text'% or the normal-form "   ///
            "variance is not positive, and the normal interval otherwise.{p_end}"
    }
    else if "`request'"=="q1" {
        di as txt "{p 0 2 2}* Requested. A component without a q = 1 interval "     ///
            "keeps the normal interval if it has one.{p_end}"
    }
    else {
        di as txt "{p 0 2 2}Reason: the default reports the q = 1 interval when "   ///
            "the lead share exceeds `threshold_text'% or the normal-form variance " ///
            "is not positive (var. <= 0), and the normal interval otherwise; the " ///
            "variance of the sum always keeps the normal interval.{p_end}"
    }
    di as txt "{p 0 2 2}Lead share: the leading direction's share of the "         ///
        "estimated sampling variance, (4 lambda1^2 max(b^2 - V[b], 0) V[b] + "      ///
        "2 lambda1^2 V[b]^2) / V, with b the leading score, V[b] its variance and " ///
        "V the normal-form variance; a ratio of two estimates, it can exceed one.{p_end}"
    _fevc_display_residual_se
    // Warnings about the components as reported.
    local weighty 0
    local unavailable 0
    local none 0
    local normal_low 0
    local low 0
    forvalues t = 1/4 {
        local regime = `table'[`t',7]
        if `regime'==1 & `table'[`t',14]>.1 & !missing(`table'[`t',14]) local weighty 1
        if "`request'"=="auto" & `t'<4 & `regime'==0 & `table'[`t',16]!=0 & ///
            !missing(`table'[`t',10]) & `table'[`t',10]>`threshold' local unavailable 1
        if missing(`regime') local none 1
        if `has_rank' & `t'<4 {
            if `rank'[`t',1]<10 {
                local low 1
                if `regime'==0 local normal_low 1
            }
        }
    }
    // The effective rank is that of the normal approximation: a low one
    // matters for a component that keeps the normal interval.
    if (`low' | e(inference_low_rank_warning)==1) & `normal_low' {
        di as txt "Warning: a low effective rank means few directions carry a component;"
        di as txt "         normal confidence intervals can then under-cover."
    }
    else if `low' {
        di as txt "{p 0 6 2}Note: every component with an effective rank below 10 has "  ///
            "the q = 1 interval.{p_end}"
    }
    if `weighty' {
        di as txt "{p 0 9 2}Warning: a leading direction rests on few deletion units "   ///
            "(a squared weight above 0.1); the q = 1 interval assumes that no unit "     ///
            "dominates it.{p_end}"
    }
    if e(inference_leading_unresolved)>0 & !missing(e(inference_leading_unresolved)) {
        di as txt "{p 0 6 2}Note: " as result e(inference_leading_unresolved) as txt  ///
            " leading direction(s) were not resolved within the cap on linear "        ///
            "solves; the largest eigenvalues are then close, and no single "           ///
            "direction dominates.{p_end}"
    }
    if `unavailable' {
        di as txt "{p 0 9 2}Warning: a component calls for the q = 1 interval, but "     ///
            "that interval could not be formed; the normal interval shown for it "     ///
            "can under-cover.{p_end}"
    }
    if `none' {
        di as txt "Note: a component has neither interval: its variance estimates are not positive."
    }
    if e(inference_joint_posted)==1 {
        di as txt "e(V) is posted for the four components; " as result "test" ///
            as txt " and " as result "lincom" as txt " use it."
    }
    else di as txt "e(V) is not posted: some component lacks a normal interval."
    di as txt "{p 0 2 2}Stored: e(inference_intervals), e(inference_leading), "       ///
        "e(inference_effective_rank), e(component_inference).{p_end}"
end

// One row of estat inference: a label and four formatted cells.
program define _fevc_detail_row
    version 15.0
    args label c1 c2 c3 c4
    di as txt %-26s `"`label'"' as result %13s `"`c1'"' %13s `"`c2'"' ///
        %13s `"`c3'"' %13s `"`c4'"'
end

// One row of numbers: column `column' of the 4-row matrix `matrix'.
program define _fevc_detail_numbers
    version 15.0
    args label matrix column
    forvalues t = 1/4 {
        local cell`t' : display %9.0g `matrix'[`t',`column']
        local cell`t' = strtrim("`cell`t''")
    }
    _fevc_detail_row `"`label'"' "`cell1'" "`cell2'" "`cell3'" "`cell4'"
end

// What the standard errors assume and how they are formed: the method text
// of estat inference (repeat-measurement) and of the removed leave-out
// residual model.
program define _fevc_display_residual_se
    version 15.0
    local repeat = ("`e(inference_variance_fit)'"=="repeat_measurement")
    if `repeat' {
        di as txt _newline "Method and assumptions"
        di as txt "{p 0 2 2}Deletion units that share a worker-firm cell (a "         ///
            "stayer's observations; under observation deletion also a mover's match " ///
            "of two or more observations) are repeat measurements and get the "       ///
            "sample variance of their outcomes; the remaining units of movers use a " ///
            "fitted model (leverage, match size, estimated effects).{p_end}"
        local controls = cond(e(inference_nuisance_omitted)==0, ///
            " Estimated controls enter to first order.","")
        di as txt "{p 0 2 2}Assumes independent deletion units. "                     ///
            "The standard error is as good as that model for movers.`controls'{p_end}"
        di as txt "{p 0 2 2}q = 1: the Andrews-Mikusheva interval of KSS (2020, "     ///
            "Section 7.1) along the leading direction, the range of the estimate "    ///
            "over a confidence ellipse for the leading score and the remainder, with " ///
            "a critical value set by the curvature. It need not be symmetric around "  ///
            "the estimate and has no standard error. It assumes that no deletion "     ///
            "unit dominates the direction, so that KSS's Lindeberg statistic "         ///
            "max w_i1^2 is small.{p_end}"
        if "`e(centering)'"=="corrected" | "$VCKSS_CENTERING"=="corrected" {
            di as txt "{p 0 2 2}Standard errors describe the mean-centred estimate; "  ///
                "the corrected-centering increment is treated as fixed.{p_end}"
        }
        exit
    }
    tempname rank
    matrix `rank' = e(inference_effective_rank)
    local units : display %15.0fc e(inference_independent_units)
    di as txt _newline "Standard errors: leave-out residual (" ///
        as result e(inference_draws) as txt " pseudo-noise draws; " ///
        as result strtrim("`units'") as txt " deletion units)."
    if e(inference_joint_posted)==1 {
        di as txt "e(V) is posted for the four components. Numerical MCSE is computational only."
    }
    else {
        di as txt "e(V) is not posted; use the available individual intervals."
    }
    di as txt "Assumes independent deletion units and similar error variances among units"
    di as txt "that share a worker or a firm. Not robust to arbitrary heteroskedasticity."
    di as txt "Effective rank (worker, firm, covariance): " as result ///
        %6.1f `rank'[1,1] %6.1f `rank'[2,1] %6.1f `rank'[3,1]
    di as txt "Largest unit influence share: " as result %6.4f e(inference_max_influence_share)
    if e(inference_low_rank_warning)==1 {
        di as txt "Warning: a low effective rank means few directions carry a component;"
        di as txt "         normal confidence intervals can then under-cover."
    }
end

program define _fevc_display_structured
    version 15.0
    if !inlist("`e(inference_model)'", "structured_common", "structured_leverage") {
        exit
    }

    if "`e(inference_variance_fit)'"=="exact_residual_moments" {
        di as txt _newline "Structured variance model: " as result "`e(inference_model)'"
        di as txt "Exact residual-moment fit; separate mover and stayer coefficients."
        di as txt "Independent units: " as result %12.0fc e(inference_mover_units) ///
            as txt " mover blocks and " as result %12.0fc e(inference_stayer_units) as txt " stayer observations."
        di as txt "Gram reciprocal condition: " as result %10.3g e(variance_gram_rcond) ///
            as txt "; variance floor share: " as result %7.4f e(variance_floor_share)
        di as txt "Fixed nuisance offsets: estimation uncertainty omitted."
        if e(inference_mean_omitted)==1 di as txt "Observed mean treated as fixed; mean-estimation uncertainty omitted."
        if e(inference_joint_posted)==0 di as txt "Joint Gaussian covariance withheld; use available individual intervals."
        di as txt "Structured variance assumptions and the target's q=0/q=1 conditions remain necessary."
        exit
    }
    tempname component_spectrum variance_summary q1_diagnostics
    matrix `component_spectrum' = e(component_spectrum)
    di as txt _newline "Explicit structured-model diagnostics"
    if e(inference_mean_omitted)==1 {
        di as txt "Observed mean treated as fixed; mean-estimation uncertainty omitted."
    }
    if "`e(inference_deletion_selected)'"=="match" {
        di as txt "`e(inference_method)'"
        di as txt "Independent mover blocks: " as result %12.0fc e(inference_mover_units) ///
            as txt "; effective by regression mass: " as result %12.2fc e(inference_effective_matches)
        if e(inference_stayer_units)>0 di as txt "Independent stayer observations: " as result %12.0fc e(inference_stayer_units)
        di as txt "Largest mass share: " as result %9.5f e(inference_largest_mass_share) ///
            as txt "; largest leverage: " as result %9.5f e(inference_largest_leverage) ///
            as txt "; minimum maker denominator: " as result %9.5f e(inference_smallest_maker)
        di as txt "`e(inference_offset_warning)'"
    }
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
    if "`e(inference_variance_fit)'"=="residual_moments" {
        di as txt "Variance model: " as result "`e(inference_model)'" ///
            as txt "; residual-moment fit; Gram probes=" as result %9.0fc e(inference_gram_probes)
        di as txt "Gram reciprocal condition=" as result %10.3g e(variance_gram_rcond) ///
            as txt "; covariance-fit floor share=" as result %7.4f e(variance_floor_share)
    }
    else {
        matrix `variance_summary' = e(structured_variance_summary)
        local model_row = cond("`e(inference_model)'"=="structured_common",1,2)
        di as txt "Variance model: " as result "`e(inference_model)'" as txt  ///
        "; floor share=" as result %7.4f `variance_summary'[`model_row',7] ///
        as txt "; boundary share=" as result %7.4f                    ///
        `variance_summary'[`model_row',9]
    }
    if "`e(inference_variance_fit)'"!="" & e(inference_joint_available)==0 {
        di as txt "Joint covariance unavailable: individual intervals retain their own checks."
    }
    if "`e(inference)'"=="q1" & "`e(inference_variance_fit)'"!="" {
        di as txt "Use the reported q1 intervals; ordinary Gaussian Wald postestimation is not supplied."
    }
    di as txt "Warning: " as result "structured conditional variance assumptions" ///
        as txt "; not unrestricted-KSS variance-product inference."
    di as txt "Omitted variance drivers can invalidate SEs and intervals; " ///
        "component point estimates are unchanged."
    if "`e(inference)'" == "q1" {
        di as txt "q=1 removes one leading mode and requires a diffuse remainder."
        di as txt "The q1 reference is asymptotically at least nominal under its assumptions; " ///
            "this does not validate the fitted variance model."
        di as txt "A computed interval does not establish that this target is one-mode; " ///
            "multi-mode targets are outside the confirmed coverage claim."
    }
    else {
        di as txt "q=0 requires strong identification and diffuse kernel and " ///
            "influence contributions."
    }
    di as txt "Qualification: " as result "`e(inference_qualification)'"
    di as txt "No universal spectral cutoff or automatic q selection is imposed; " ///
        "successful computation does not establish the asymptotic condition."
end

// Numerical MCSE in full, for estat diagnostics: mode, method, status, the
// four values, the developer's conditional values and what the MCSE leaves
// out. It is computational precision, not a sampling standard error.
program define _fevc_display_numerical
    version 15.0
    tempname mcse
    if "`e(mcse_status)'"=="unavailable_capability" {
        di as txt _newline "Approximate numerical MCSE unavailable: installed plugin lacks numerical V2."
    }
    if inlist("`e(mcse_mode)'","all","conditional") {
        if "`e(mcse_mode)'"=="all" di as txt _newline "Approximate numerical MCSE (all main point probes)"
        else di as txt _newline "Numerical MCSE (developer: conditional on leverage sketch)"
        di as txt "  MCSE method: " as result "`e(mcse_method)'" ///
            as txt "; status: " as result "`e(mcse_status)'"  ///
            as txt "; available: " as result e(mcse_available)
        if "`e(algorithm)'"=="jla" {
            local probes : display %12.0fc e(probes)
            di as txt "  Projections: " as result strtrim("`probes'") ///
                as txt "; centering: " as result "`e(mcse_centering)'"
        }
        if e(mcse_available)==1 {
            matrix `mcse' = e(mcse)
            di as txt "  worker=" as result %10.5g `mcse'[1,1] ///
                as txt "; firm=" as result %10.5g `mcse'[1,2] ///
                as txt "; covariance=" as result %10.5g `mcse'[1,3] ///
                as txt "; total=" as result %10.5g `mcse'[1,4]
        }
        else di as txt "  Unavailable: " as result "`e(mcse_status)'"
        // The developer's narrower calculation holds the leverage directions
        // fixed; under mcse(all) it is reported beside the main values.
        if "`e(mcse_mode)'"=="all" & e(mcse_conditional_available)==1 {
            capture confirm matrix e(mcse_conditional)
            if !_rc {
                matrix `mcse' = e(mcse_conditional)
                if colsof(`mcse')==4 {
                    di as txt "  Conditional on the leverage sketch (developer):"
                    di as txt "  worker=" as result %10.5g `mcse'[1,1] ///
                        as txt "; firm=" as result %10.5g `mcse'[1,2] ///
                        as txt "; covariance=" as result %10.5g `mcse'[1,3] ///
                        as txt "; total=" as result %10.5g `mcse'[1,4]
                }
            }
        }
        di as txt "  Main point estimates only; excludes projection/inference calculations."
        di as txt "  Excludes finite-probe bias, solver/floating-point error and sampling"
        di as txt "  uncertainty. More projections (probes()) reduce it."
    }
    else if "`e(mcse_mode)'"=="off" {
        di as txt _newline "Numerical MCSE: off (mcse(off)); method: none; available: 0."
    }
end

// Two significant digits, keeping trailing zeros: .00020, .018, 1.2, 12.
program define _fevc_sig2
    version 15.0
    args value
    local text "."
    if !missing(`value') {
        if `value'==0 local text "0"
        else {
            local digits = min(15,max(0,1-floor(log10(abs(`value')))))
            local text : display %21.`digits'f `value'
            local text = strtrim("`text'")
            if substr("`text'",1,2)=="0." local text = substr("`text'",2,.)
            if substr("`text'",1,3)=="-0." local text = "-"+substr("`text'",3,.)
        }
    }
    c_local sig_text "`text'"
end

// The numerical MCSE under the decomposition table, in one line (two when
// it does not fit in 79 columns): its four values and, when the components
// have standard errors, its largest share of one. The share uses the
// normal-form standard error for a component reported with the q = 1
// interval. Above 10% the line warns and suggests more projections.
program define _fevc_display_mcse_line
    version 15.0
    if "`e(mcse_status)'"=="unavailable_capability" {
        di as txt "Numerical MCSE unavailable: the installed plugin lacks numerical V2."
        exit
    }
    if !inlist("`e(mcse_mode)'","all","conditional") exit
    if "`e(mcse_method)'"=="exact" | "`e(algorithm)'"=="exact" {
        di as txt "Exact leverages: no numerical MCSE from projections."
        exit
    }
    if e(mcse_available)!=1 {
        di as txt "Numerical MCSE unavailable (" as result "`e(mcse_status)'" ///
            as txt "); see " as result "estat diagnostics" as txt "."
        exit
    }
    tempname mcse se
    matrix `mcse' = e(mcse)
    local have_se 0
    capture confirm matrix e(inference_intervals)
    if !_rc {
        matrix `se' = e(inference_intervals)
        local have_se 1
    }
    else if "`e(inference_variance_fit)'"!="" {
        capture confirm matrix e(component_inference)
        if !_rc {
            matrix `se' = e(component_inference)
            local have_se 1
        }
    }
    local names worker firm cov sum
    local largest = .
    local flagged ""
    forvalues j = 1/4 {
        local name : word `j' of `names'
        _fevc_sig2 `=`mcse'[1,`j']'
        local part`j' "`name' `sig_text'"
        if `have_se' {
            local error = `se'[`j',2]
            if `error'>0 & !missing(`error') {
                local ratio = `mcse'[1,`j']/`error'
                if missing(`largest') | `ratio'>`largest' local largest = `ratio'
                if `ratio'>.1 {
                    if "`flagged'"=="" local flagged "`name'"
                    else local flagged "`flagged', `name'"
                }
            }
        }
    }
    local probes : display %12.0fc e(probes)
    local probes = strtrim("`probes'")
    local line "Numerical MCSE (`probes' projections):"
    if "`e(mcse_mode)'"=="conditional" {
        local line "Conditional numerical MCSE (`probes' projections):"
    }
    local tail ""
    if !missing(`largest') {
        local percent = max(1,ceil(100*`largest'))
        local tail "at most `percent'% of a standard error."
        local part4 "`part4';"
    }
    forvalues j = 1/4 {
        local separator = cond(`j'==1,1,2)
        if strlen("`line'")+`separator'+strlen("`part`j''")<=79 {
            if `j'==1 local line "`line' `part`j''"
            else local line "`line'  `part`j''"
        }
        else {
            di as txt "`line'"
            local line "  `part`j''"
        }
    }
    if "`tail'"!="" {
        if strlen("`line'")+1+strlen("`tail'")<=79 local line "`line' `tail'"
        else {
            di as txt "`line'"
            local line "  `tail'"
        }
    }
    di as txt "`line'"
    if "`flagged'"!="" {
        local more = max(2000,10*e(probes))
        local more : display %12.0f `more'
        local more = strtrim("`more'")
        di as txt "{p 0 9 2}Warning: numerical MCSE over 10% of the standard error " ///
            "for `flagged'; more projections reduce it: rerun with "            ///
            as result "probes(`more')" as txt " or more.{p_end}"
    }
end

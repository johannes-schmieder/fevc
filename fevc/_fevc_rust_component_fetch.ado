program define _fevc_rust_component_fetch, rclass
    version 18.0
    args handle reference model simulations memorylimit posted level      ///
        outprimitive outcovariance outmcse outspectrum outq1raw           ///
        outsummaries outfolds outcv outreceipt outresults outq1results      ///
        outunits deletion expectedunits expectedgramprobes
    local modelcode = cond("`model'"=="structured_common",1,2)
    local referencecode = ("`reference'"=="q1")
    capture noisily _fevc_rust_public_call componentresultv5 `handle', reference(`reference')
    if _rc {
        local failure_rc = _rc
        capture noisily _fevc_rust_abort, rc(`failure_rc') handle(`handle') ///
            phase(component_result_export)
        exit `failure_rc'
    }
    tempname primitive covariance mcse spectrum q1_raw summaries folds cv receipt
    tempname inference_results q1_results units target_status
    matrix `target_status' = r(target_variance_status)
    local joint_status = r(joint_status)
    local variance_fit = r(variance_fit)
    matrix `units' = (r(unit_schema),r(unit_deletion),r(independent_units), ///
        r(nuisance_uncertainty_omitted),r(effective_match_count),            ///
        r(largest_match_mass_share),r(largest_match_leverage),              ///
        r(smallest_maker_denominator))
    matrix colnames `units' = schema deletion independent_units nuisance_omitted ///
        effective_matches largest_mass_share largest_leverage smallest_maker
    matrix `primitive' = r(primitive_covariance)
    matrix `covariance' = r(covariance)
    matrix `mcse' = r(trace_mcse)
    matrix `spectrum' = r(spectrum)
    if "`reference'"=="q1" matrix `q1_raw' = r(q1)
    matrix `summaries' = r(variance_summaries)
    matrix `folds' = r(fold_diagnostics)
    matrix `cv' = r(cv_diagnostics)
    matrix `receipt' = (r(schema_version),r(variance_source),        ///
        r(reference_distribution),r(probes),r(counter_atoms),       ///
        r(counter_words),r(peak_forecast_bytes),r(psd_cleanup),     ///
        r(raw_eigen_min),r(raw_eigen_max),                          ///
        r(point_correction_identity_error),r(maximum_iterations),   ///
        r(maximum_reduced_residual),r(maximum_complete_residual),   ///
        r(full_residual_tolerance),r(median_absolute_log_ratio),    ///
        r(p90_absolute_log_ratio),r(maximum_absolute_log_ratio),    ///
        r(log_variance_correlation),r(critical_simulations),        ///
        r(maximum_remainder_identity_error),r(solver_columns),r(critical_draws), ///
        r(joint_status),r(variance_fit),r(gram_probes),r(ordering),r(gram_rcond), ///
        r(gram_inverse_relres),r(variance_fit_relres),r(positivity_floor), ///
        r(floored_predictions),r(nonpositive_predictions),r(computed_targets), ///
        `target_status'[1,2],`target_status'[2,2],`target_status'[3,2],`target_status'[4,2])
    matrix colnames `receipt' = schema model reference probes       ///
        counter_atoms counter_words peak psd_cleanup raw_eigen_min  ///
        raw_eigen_max point_identity_error max_iterations           ///
        max_reduced_residual max_complete_residual                  ///
        full_residual_tolerance median_abs_log_ratio p90_abs_log_ratio ///
        maximum_abs_log_ratio log_variance_correlation critical_simulations ///
        maximum_remainder_identity_error solver_columns critical_draws ///
        joint_status variance_fit gram_probes ordering gram_rcond gram_relres ///
        fit_relres positivity_floor floored nonpositive computed q0_worker q0_firm q0_covariance q0_total
    local ok = rowsof(`primitive')==3 & colsof(`primitive')==3 &   ///
        rowsof(`covariance')==4 & colsof(`covariance')==4 &        ///
        rowsof(`mcse')==3 & colsof(`mcse')==3 &                    ///
        rowsof(`spectrum')==4 & colsof(`spectrum')==15 &           ///
        rowsof(`summaries')==2 & colsof(`summaries')==12 &         ///
        rowsof(`folds')==10 & colsof(`folds')==15 &                ///
        rowsof(`cv')==70 & colsof(`cv')==7 &                       ///
        rowsof(`target_status')==4 & colsof(`target_status')==2 & ///
        inlist(`joint_status',0,1,2) &                           ///
        `variance_fit'==2 &                                    ///
        `receipt'[1,1]==5 & `receipt'[1,2]==`modelcode' &          ///
        `receipt'[1,3]==`referencecode' &                          ///
        `receipt'[1,4]==`simulations' & `receipt'[1,5]>0 &         ///
        `receipt'[1,6]>0 & `receipt'[1,7]>0 &                      ///
        `receipt'[1,7]<=`memorylimit' & `receipt'[1,8]>=0 &        ///
        `receipt'[1,11]<=1e-10 & `receipt'[1,12]>=0 &              ///
        `receipt'[1,13]>=0 & `receipt'[1,14]>=0 &                  ///
        `receipt'[1,15]>0 & `receipt'[1,14]<=`receipt'[1,15] &     ///
        `receipt'[1,20]==cond("`reference'"=="q1",max(100000,100*`simulations'),0) & ///
        `receipt'[1,21]>=0 & `receipt'[1,22]>0 & `receipt'[1,23]>=0 & ///
        `receipt'[1,23]<=4*`receipt'[1,20] &                         ///
        `units'[1,1]==1 & `units'[1,2]==cond("`deletion'"=="match",1,2) & ///
        `units'[1,3]==`expectedunits' & `units'[1,3]>0 &            ///
        `units'[1,4]==("`deletion'"=="match")
    if "`deletion'"=="match" local ok = `ok' &                     ///
        `units'[1,5]>=1 & `units'[1,5]<=`expectedunits'*(1+1e-12) &  ///
        `units'[1,6]>0 & `units'[1,6]<=1 &                         ///
        `units'[1,7]>=0 & `units'[1,7]<1 &                         ///
        `units'[1,8]>0 & `units'[1,8]<=1
    else local ok = `ok' & `units'[1,5]==0 & `units'[1,6]==0 &     ///
        `units'[1,7]==0 & `units'[1,8]==0
    if "`reference'"=="q1" local ok = `ok' & rowsof(`q1_raw')==4 & ///
        colsof(`q1_raw')==20
    if `ok' {
        foreach matrix_name in primitive covariance mcse summaries folds cv receipt units target_status {
            forvalues row = 1/`=rowsof(``matrix_name'')' {
                forvalues column = 1/`=colsof(``matrix_name'')' {
                    local expected_missing =                              ///
                        (`joint_status'>0 & inlist("`matrix_name'","primitive","covariance")) | ///
                        (`variance_fit'==2 & inlist("`matrix_name'","summaries","folds","cv")) | ///
                        ("`matrix_name'"=="receipt" & `variance_fit'==2 & inrange(`column',16,19)) | ///
                        ("`matrix_name'"=="receipt" & `variance_fit'==1 & inrange(`column',28,31))
                    if missing(``matrix_name''[`row',`column']) != `expected_missing' local ok = 0
                }
            }
        }
        if `variance_fit'==2 local ok = `ok' & `receipt'[1,26]==`expectedgramprobes' & ///
            `receipt'[1,27]==cond("`deletion'"=="match",3,2) & ///
            `receipt'[1,28]>0 & `receipt'[1,29]>=0 & ///
            `receipt'[1,30]>=0 & `receipt'[1,31]>0 & `receipt'[1,32]<`expectedunits' & ///
            `receipt'[1,33]<=`receipt'[1,32]
        else local ok = `ok' & `receipt'[1,26]==0 & `receipt'[1,27]==0 & ///
            `receipt'[1,32]==0 & `receipt'[1,33]==0
        local q0computed = 0
        forvalues row = 1/4 {
            local targetcode = `target_status'[`row',2]
            if !inlist(`targetcode',0,1,4,6) local ok = 0
            if `targetcode'==0 & `target_status'[`row',1]<=0 local ok = 0
            local q0computed = `q0computed'+(`targetcode'==0)
        }
        if "`reference'"=="q0" & `receipt'[1,34]!=`q0computed' local ok = 0
        if "`reference'"=="q1" {
            local expected_critical_draws = 0
            local q1computed = 0
            forvalues row = 1/4 {
                local targetstatus = `q1_raw'[`row',17]
                local q1computed = `q1computed'+(`targetstatus'==0)
                if !inlist(`targetstatus',0,1,2,3,6) local ok = 0
                if inlist(`targetstatus',0,3) local expected_critical_draws = ///
                    `expected_critical_draws'+`receipt'[1,20]
                forvalues column = 1/20 {
                    local optional = `targetstatus'>0 & ///
                        (inrange(`column',9,13) | `column'==18)
                    if !`optional' & missing(`q1_raw'[`row',`column']) local ok = 0
                }
                if `targetstatus'>0 & (!missing(`q1_raw'[`row',11]) | ///
                    !missing(`q1_raw'[`row',12])) local ok = 0
                if reldif(`q1_raw'[`row',1],`posted'[1,`row'])>1e-10 local ok = 0
                if abs(`q1_raw'[`row',19]-`q1_raw'[`row',20]-`q1_raw'[`row',7])> ///
                    1e-10*max(1,abs(`q1_raw'[`row',7])) local ok = 0
                if `targetstatus'==0 {
                    if `q1_raw'[`row',3]<=0 | `q1_raw'[`row',7]<=0 | ///
                        `q1_raw'[`row',18]<=0 | `q1_raw'[`row',18]>1 | ///
                        `q1_raw'[`row',10]<=0 | `q1_raw'[`row',11]>`q1_raw'[`row',12] local ok = 0
                    local determinant = 1-(`q1_raw'[`row',6]/sqrt(`q1_raw'[`row',3])/ ///
                        sqrt(`q1_raw'[`row',7]))^2
                    if abs(`determinant'-`q1_raw'[`row',18])>1e-10 local ok = 0
                    local curvature = 2*abs(`spectrum'[`row',1])*`q1_raw'[`row',3]/ ///
                        sqrt(`q1_raw'[`row',7]*`q1_raw'[`row',18])
                    if reldif(`curvature',`q1_raw'[`row',9])>1e-9 local ok = 0
                }
            }
            if `receipt'[1,23]!=`expected_critical_draws' local ok = 0
            if `receipt'[1,34]!=`q1computed' local ok = 0
        }
        if `receipt'[1,22]!=floor(`receipt'[1,22]) local ok = 0
        forvalues row = 1/4 {
            if `joint_status'==0 & reldif(`target_status'[`row',1],`covariance'[`row',`row'])>1e-12 local ok = 0
            if `spectrum'[`row',13]!=128 | `spectrum'[`row',14]!= ///
                cond("`deletion'"=="observation" | "`reference'"=="q0",512,128) local ok = 0
            local spectral_missing_allowed = (`target_status'[`row',2]==6)
            if "`reference'"=="q1" local spectral_missing_allowed = (`q1_raw'[`row',17]==6)
            forvalues column = 1/15 {
                if missing(`spectrum'[`row',`column']) & ///
                    (!`spectral_missing_allowed' | `column'>12) local ok = 0
            }
        }
    }
    if `ok' & `joint_status'==0 {
        local scale = max(1,abs(`covariance'[4,4]))
        forvalues row = 1/4 {
            forvalues column = 1/4 {
                if abs(`covariance'[`row',`column']-                 ///
                    `covariance'[`column',`row'])>1e-12*`scale' local ok = 0
            }
        }
        forvalues row = 1/3 {
            forvalues column = 1/3 {
                if reldif(`primitive'[`row',`column'],`covariance'[`row',`column'])>1e-12 local ok = 0
            }
            local mapped = `primitive'[`row',1]+`primitive'[`row',2]+ ///
                2*`primitive'[`row',3]
            if abs(`covariance'[`row',4]-`mapped')>1e-12*`scale' local ok = 0
        }
        local mapped44 = `primitive'[1,1]+`primitive'[2,2]+4*`primitive'[3,3]+ ///
            2*`primitive'[1,2]+4*`primitive'[1,3]+4*`primitive'[2,3]
        if abs(`covariance'[4,4]-`mapped44')>1e-12*`scale' local ok = 0
    }
    local names worker_variance firm_variance worker_firm_covariance total_variance
    matrix rownames `primitive' = worker_variance firm_variance worker_firm_covariance
    matrix colnames `primitive' = worker_variance firm_variance worker_firm_covariance
    matrix rownames `covariance' = `names'
    matrix colnames `covariance' = `names'
    matrix rownames `mcse' = worker_variance firm_variance worker_firm_covariance
    matrix colnames `mcse' = worker_variance firm_variance worker_firm_covariance
    matrix rownames `spectrum' = `names'
    matrix colnames `spectrum' = lambda1 lambda2 trace2_raw trace2 trace2_mcse ///
        trace_reconciliation leading_share leading_share_mcse                ///
        remainder_leading_share max_mode_weight_sq leading_residual          ///
        second_residual probes iterations max_influence_share
    matrix rownames `summaries' = structured_common structured_leverage
    matrix colnames `summaries' = model observations minimum median maximum ///
        floor_count floor_share boundary_count boundary_share                ///
        maximum_boundary_excess max_prediction_leverage minimum_fitted_rcond
    matrix rownames `folds' = common_fold0 common_fold1 common_fold2 common_fold3 ///
        common_fold4 leverage_fold0 leverage_fold1 leverage_fold2 leverage_fold3 ///
        leverage_fold4
    matrix colnames `folds' = model outer_fold training_observations         ///
        validation_observations active_terms selected_lambda selected_cv_mse ///
        variance_scale positivity_floor floored_predictions boundary_predictions ///
        max_boundary_excess max_prediction_leverage fitted_rcond fitted_relres
    matrix colnames `cv' = model outer_fold grid_index lambda                ///
        validation_observations mse available
    matrix `inference_results' = J(4,4,.)
    local zcrit = invnormal(1-(100-`level')/200)
    forvalues row = 1/4 {
        local estimate = `posted'[1,`row']
        local se = cond(`target_status'[`row',2]==0,sqrt(`target_status'[`row',1]),.)
        matrix `inference_results'[`row',1] = `estimate'
        matrix `inference_results'[`row',2] = `se'
        matrix `inference_results'[`row',3] = `estimate'-`zcrit'*`se'
        matrix `inference_results'[`row',4] = `estimate'+`zcrit'*`se'
    }
    matrix rownames `inference_results' = `names'
    matrix colnames `inference_results' = estimate se lb ub
    if "`reference'"=="q1" {
        matrix rownames `q1_raw' = `names'
        matrix colnames `q1_raw' = estimate leading_score leading_variance ///
            leading_component remainder_estimate leading_remainder_cov    ///
            remainder_variance remainder_trace_mcse curvature critical_value ///
            am_lb am_ub leading_F remainder_influence_share               ///
            recenter_var_b1 remainder_identity_error target_status ///
            standardized_determinant remainder_influence_variance remainder_trace_variance
        matrix `q1_results' = J(4,17,.)
        forvalues row = 1/4 {
            matrix `q1_results'[`row',1] = `posted'[1,`row']
            matrix `q1_results'[`row',2] = `inference_results'[`row',2]
            matrix `q1_results'[`row',3] = `inference_results'[`row',3]
            matrix `q1_results'[`row',4] = `inference_results'[`row',4]
            matrix `q1_results'[`row',5] = `q1_raw'[`row',11]
            matrix `q1_results'[`row',6] = `q1_raw'[`row',12]
            matrix `q1_results'[`row',7] = `spectrum'[`row',1]
            matrix `q1_results'[`row',8] = `spectrum'[`row',7]
            matrix `q1_results'[`row',9] = `spectrum'[`row',10]
            matrix `q1_results'[`row',10] = `q1_raw'[`row',3]
            matrix `q1_results'[`row',11] = `q1_raw'[`row',6]
            matrix `q1_results'[`row',12] = `q1_raw'[`row',7]
            matrix `q1_results'[`row',13] = `q1_raw'[`row',2]
            matrix `q1_results'[`row',14] = `q1_raw'[`row',5]
            matrix `q1_results'[`row',15] = `q1_raw'[`row',13]
            matrix `q1_results'[`row',16] = `q1_raw'[`row',9]
            matrix `q1_results'[`row',17] = `q1_raw'[`row',10]
        }
        matrix rownames `q1_results' = `names'
        matrix colnames `q1_results' = estimate highrank_se wald_lb wald_ub ///
            am_lb am_ub lambda1 eigen_share max_weight_sq var_b1            ///
            cov_b1_theta1 var_theta1 b1 theta1 F curvature critical_value
    }
    if !`ok' {
        capture noisily _fevc_rust_abort, rc(498) handle(`handle') ///
            phase(component_result_export)
        exit 498
    }
    matrix `outprimitive' = `primitive'
    matrix `outcovariance' = `covariance'
    matrix `outmcse' = `mcse'
    matrix `outspectrum' = `spectrum'
    matrix `outsummaries' = `summaries'
    matrix `outfolds' = `folds'
    matrix `outcv' = `cv'
    matrix `outreceipt' = `receipt'
    matrix `outunits' = `units'
    matrix `outresults' = `inference_results'
    if "`reference'"=="q1" {
        matrix `outq1raw' = `q1_raw'
        matrix `outq1results' = `q1_results'
    }
    return scalar peak = `receipt'[1,7]
end

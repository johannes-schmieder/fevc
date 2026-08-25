*! version 0.4.0-dev 24aug2026
program define _vckss_rust_capture_stayers, rclass
    version 18.0
    args handle controls nuisance tolerance ranktol blocktol augctx

    tempname aug raw source receipt
    matrix `aug' = `augctx'
    if rowsof(`aug')!=1 | colsof(`aug')!=25 {
        return scalar ok = 0
        return local detail "invalid stayer-augmentation context shape"
        exit
    }

    capture noisily _vckss_rust_public_call stayerresult `handle'
    if _rc exit _rc
    matrix `raw' = r(result)
    matrix `source' = r(correction_source)
    foreach pair in schema_version:h_schema weighted_rss:h_rss       ///
        parameters:h_params full_parameters:h_fullparams             ///
        correction_parameters:h_corrparams deletion_units:h_del      ///
        max_leverage:h_maxlev information_rcond:h_info               ///
        inverse_relative_residual:h_inv inverse_original_relres:h_invorig ///
        inverse_sqrt_relative_residual:h_invsqrt                      ///
        maker_relative_residual:h_maker                               ///
        full_fit_relative_residual:h_fullfit                          ///
        working_fit_relative_residual:h_workfit                       ///
        fit_residual_tolerance:h_fittol                               ///
        control_basis_relative_residual:h_ctrlrel                     ///
        control_basis_forward_error:h_ctrlfwd deletion_rank_gap:h_delgap ///
        firm_zero_sum_residual:h_fzero peak_forecast_bytes:h_peak     ///
        fit_peak_forecast_bytes:h_fitpeak                             ///
        correction_peak_forecast_bytes:h_corrpeak                    ///
        accounting_residual:h_acct source_accounting_residual:h_source {
        gettoken returned localname : pair, parse(":")
        local localname = substr("`localname'",2,.)
        local `localname' = r(`returned')
    }

    local a_total_workers = `aug'[1,11]
    local a_firms = `aug'[1,12]
    local a_total_del = `aug'[1,15]
    local a_top_hi = `aug'[1,19]
    local a_top_lo = `aug'[1,20]
    local a_mem_limit = `aug'[1,21]
    local expected_fullparams = `a_total_workers'+`a_firms'-1+`controls'
    local expected_params = `expected_fullparams'
    if lower(strtrim("`nuisance'"))=="fixedoffset" {
        local expected_params = `a_total_workers'+`a_firms'-1
    }
    local expected_fittol = max(1e-11,10*`tolerance')
    local invgate = max(1e-10,100*`ranktol')
    local scale = 1
    local accounting_truth = 0
    local source_truth = 0
    local ok = rowsof(`raw')==4 & colsof(`raw')==4 &              ///
        rowsof(`source')==2 & colsof(`source')==4
    if `ok' {
        forvalues row=1/4 {
            forvalues col=1/4 {
                if missing(`raw'[`row',`col']) local ok = 0
                else local scale = max(`scale',abs(`raw'[`row',`col']))
            }
        }
        forvalues row=1/3 {
            local resid = abs(`raw'[`row',4]-`raw'[`row',1]-       ///
                `raw'[`row',2]-2*`raw'[`row',3])
            local accounting_truth = max(`accounting_truth',`resid')
        }
        forvalues col=1/4 {
            if `raw'[4,`col']!=0 |                                ///
                abs(`raw'[1,`col']-`raw'[2,`col']-`raw'[3,`col'])> ///
                    1e-10*max(1,abs(`raw'[1,`col'])) local ok = 0
            local source_resid = abs(`raw'[2,`col']-               ///
                `source'[1,`col']-`source'[2,`col'])
            local source_truth = max(`source_truth',`source_resid')
        }
        if `accounting_truth'>1e-10*`scale' |                      ///
            `source_truth'>1e-10*`scale' local ok = 0
    }
    foreach value in h_schema h_params h_fullparams h_corrparams h_del ///
        h_peak h_fitpeak h_corrpeak {
        if missing(``value'') | ``value''<0 | ``value''!=floor(``value'') {
            local ok = 0
        }
    }
    foreach value in h_rss h_maxlev h_info h_inv h_invorig h_invsqrt ///
        h_maker h_fullfit h_workfit h_fittol h_ctrlrel h_ctrlfwd     ///
        h_delgap h_fzero h_acct h_source {
        if missing(``value'') local ok = 0
    }
    if `ok' {
        local ok = `h_schema'==1 & `h_params'==`expected_params' &  ///
            `h_fullparams'==`expected_fullparams' &                 ///
            `h_corrparams'==`expected_params' & `h_del'==`a_total_del' & ///
            `h_rss'>=0 & `h_maxlev'>=0 & `h_maxlev'<1 &            ///
            `h_info'>0 & `h_info'<=1 & `h_inv'>=0 & `h_inv'<=`invgate' & ///
            `h_invorig'>=0 & `h_invsqrt'>=0 & `h_maker'>=0 &       ///
            `h_maker'<=`invgate' & `h_fullfit'>=0 &                ///
            `h_fullfit'<=`h_fittol' & `h_workfit'>=0 &             ///
            `h_workfit'<=`h_fittol' &                              ///
            abs(`h_fittol'-`expected_fittol')<=                    ///
                4096*c(epsdouble)*max(1,`expected_fittol') &       ///
            `h_ctrlrel'>=0 & `h_ctrlfwd'>=0 & `h_ctrlfwd'<.25 &   ///
            `h_delgap'>`blocktol' & `h_fzero'>=0 &                 ///
            `h_fzero'<=`invgate' &                                ///
            `h_peak'==max(`h_fitpeak',`h_corrpeak') &              ///
            `h_peak'<=`a_mem_limit' &                              ///
            abs(`h_acct'-`accounting_truth')<=                     ///
                4096*c(epsdouble)*max(1,`scale') &                 ///
            abs(`h_source'-`source_truth')<=                       ///
                4096*c(epsdouble)*max(1,`scale') &                 ///
            `a_top_hi'>=0 & `a_top_hi'<=4294967295 &              ///
            `a_top_lo'>=0 & `a_top_lo'<=4294967295
    }
    if `ok' & `controls'==0 {
        local ok = `h_ctrlrel'==0 & `h_ctrlfwd'==0
    }
    if `ok' local detail "Rust exact stayer-hybrid result reconciled"
    else local detail "Rust exact stayer-hybrid result, residual, accounting, or memory receipt mismatch"

    matrix `receipt' = (`h_schema',`h_rss',`h_params',`h_fullparams', ///
        `h_corrparams',`h_del',`h_maxlev',`h_info',`h_inv',`h_invorig', ///
        `h_invsqrt',`h_maker',`h_fullfit',`h_workfit',`h_fittol',    ///
        `h_ctrlrel',`h_ctrlfwd',`h_delgap',`h_fzero',`h_peak',      ///
        `h_fitpeak',`h_corrpeak',`h_acct',`h_source')
    matrix colnames `receipt' = schema weighted_rss parameters       ///
        full_parameters correction_parameters deletion_units max_leverage ///
        information_rcond inverse_relres inverse_original_relres     ///
        inverse_sqrt_relres maker_relres full_fit_relres working_fit_relres ///
        fit_tolerance control_basis_relres control_basis_forward_error ///
        deletion_rank_gap firm_zero_sum_residual peak fit_peak correction_peak ///
        accounting_residual source_accounting_residual
    return clear
    return scalar ok = `ok'
    return local detail `"`detail'"'
    return matrix result = `raw'
    return matrix correction_source = `source'
    return matrix receipt = `receipt'
end

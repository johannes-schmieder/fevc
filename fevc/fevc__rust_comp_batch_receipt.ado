*! execution-only component batch reconciliation
program define fevc__rust_comp_batch_receipt, rclass
    version 18.0
    args handle covariance_probes gram_probes work native_threads
    if "`native_threads'"=="" local native_threads = c(processors)
    capture noisily fevc__rust_public_call componentbatchreceipt `handle'
    if _rc {
        local failure_rc = _rc
        capture noisily _fevc_rust_abort, rc(`failure_rc') handle(`handle') ///
            phase(component_batch_receipt)
        exit `failure_rc'
    }
    tempname batch
    matrix `batch' = r(receipt)
    local ok = rowsof(`batch')==1 & colsof(`batch')==13
    if `ok' {
        forvalues column = 1/13 {
            if missing(`batch'[1,`column']) | `batch'[1,`column']<0 | ///
                `batch'[1,`column']>9007199254740992 | ///
                `batch'[1,`column']!=floor(`batch'[1,`column']) local ok = 0
        }
    }
    if `ok' {
        local width = max(`batch'[1,8],`batch'[1,9])
        local expected_cap = min(max(`covariance_probes',128,`gram_probes'), ///
            max(32,8*`native_threads'))
        local ok = `batch'[1,1]==88 & `batch'[1,2]==1 & ///
            `batch'[1,3]==1 & inlist(`batch'[1,4],2,3,4) & ///
            `batch'[1,5]==`handle' & `batch'[1,6]==1 & `batch'[1,7]==1 & ///
            `batch'[1,8]==min(max(`covariance_probes',128),`width') & ///
            `batch'[1,9]==min(`gram_probes',`width') & ///
            `width'>=1 & `width'<=`expected_cap' & ///
            `batch'[1,10]==`expected_cap' & `batch'[1,11]==`native_threads' & ///
            `batch'[1,12]==`work'[1,9] & `batch'[1,13]==`work'[1,22]
    }
    if !`ok' {
        capture noisily _fevc_rust_abort, rc(498) handle(`handle') ///
            phase(component_batch_reconcile)
        exit 498
    }
    return matrix receipt = `batch'
    return scalar width = `width'
end

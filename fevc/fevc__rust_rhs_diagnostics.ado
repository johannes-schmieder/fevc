// Convert the already validated native ledger without Stata's legacy
// matrix-subscript write limit. Temporary Mata state is removed on failure too.
program define fevc__rust_rhs_diagnostics
    version 15.0
    tempname work row phase probe stage rhs start batch ok
    capture noisily _fevc_rust_rhs_map `0' `work' `row' `phase' `probe' ///
        `stage' `rhs' `start' `batch' `ok'
    local rc = _rc
    foreach name in `work' `row' `phase' `probe' `stage' `rhs' `start' `batch' `ok' {
        capture mata: mata drop `name'
    }
    exit `rc'
end

program define _fevc_rust_rhs_map
    version 15.0
    args source destination levbatch tgtbatch kind maxiter reltol fulltol ///
        work row phase probe stage rhs start batch ok
    mata: `work' = st_matrix("`source'")
    mata: for (`row'=1; `row'<=rows(`work'); `row'++) { ///
        `phase' = `work'[`row',1]; ///
        `probe' = `work'[`row',2]; ///
        `stage' = (`kind'>=3 ? (`phase'==1 ? 2 : (`phase'==2 ? 4 : 5)) : ///
            (`phase'==5 ? 1 : (`phase'==1 ? 2 : (`phase'==4 ? 3 : ///
            (`phase'==2 ? 4 : (`phase'==6 & `kind'==2 ? 6 : 5)))))); ///
        `rhs' = (`probe'<0 ? 1 : (`phase'==3 ? 2*`probe'+`work'[`row',3] : `probe'+1)); ///
        `batch' = (`phase'==2 ? `levbatch' : (`kind'==4 & `phase'!=3 ? 1 : `tgtbatch')); ///
        `start' = (`probe'<0 ? 1 : floor(`probe'/`batch')*`batch'+1); ///
        if (`kind'==4) `ok'=1; ///
        else if (`kind'==3) `ok' = (`work'[`row',4]==2 & ///
            `work'[`row',5]>=0 & `work'[`row',5]<=`maxiter' & ///
            `work'[`row',6]>=0 & `work'[`row',6]<=`reltol' & ///
            `work'[`row',7]>=0 & `work'[`row',7]<=`fulltol' & ///
            (`work'[`row',8]==0 | `work'[`row',8]==1)); ///
        else `ok' = (`work'[`row',9]==1 | `work'[`row',9]==2); ///
        `work'[`row',1..6] = (`stage',`start',`rhs', ///
            `work'[`row',5],`work'[`row',7],`ok'); ///
    }
    mata: `work' = `work'[.,1..6]
    mata: st_matrix("`destination'",`work')
end

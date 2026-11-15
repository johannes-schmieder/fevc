version 15.0
clear all
set more off
set varabbrev off
args pkgroot
if `"`pkgroot'"'=="" local pkgroot `"`c(pwd)'/fevc"'
adopath ++ `"`pkgroot'"'
quietly do `"`pkgroot'/fevc_rng.mata"'

* Compare future tied sorts, rather than only testing an echoed seed.
set obs 1000
generate long id = _n
generate byte key = mod(_n,3)
quietly query sortseed
local initial_sort `"`r(sortseed)'"'
sort key
mata: expected_order = st_data(.,"id")
sort id
quietly set sortseed `initial_sort'

program define _fevc_compat_results, rclass
    tempname m
    matrix `m' = (1,2\3,4)
    return scalar sentinel = 917
    return local sentinel_text "caller results"
    return matrix sentinel_matrix = `m'
end
_fevc_compat_results
mata: saved = vckss_rng__capture_full()
mata: assert(saved.status=="OK")
assert r(sentinel)==917
assert `"`r(sentinel_text)'"'=="caller results"
tempname retained_matrix
matrix `retained_matrix' = r(sentinel_matrix)
assert `retained_matrix'[2,1]==3
sort key
sort id
mata: assert(vckss_rng__restore_full(saved)==0)
sort key
mata: assert(st_data(.,"id")==expected_order)
quietly query sortseed
local final_sort `"`r(sortseed)'"'
sort id
quietly set sortseed `initial_sort'
sort key
quietly query sortseed
assert `"`r(sortseed)'"'==`"`final_sort'"'

* Each runtime has its own identity; unknown future runtimes remain closed.
mata: assert(vckss_rng__production_contract()== ///
    "KSS-MT64S-DOMAIN-CURSOR-V3-STATA"+ ///
    strofreal(floor(st_numscalar("c(stata_version)")),"%2.0f"))

* Fail before asking a native generation to allocate an oversized matrix.
capture quietly fevc_rust probe
if !_rc {
    if c(stata_version)>=16 {
        local capacity = cond(c(MP),65534,cond(c(SE),11000,800))
        capture local capacity = c(max_matdim)
    }
    else local capacity = c(matsize)
    capture noisily fevc_rust projectionresult 1, columns(`=`capacity'+1')
    assert _rc==198
}
* Diagnostic transport preserves literal row identities on all native routes.
tempname ledger actual expected
mata: st_matrix("`ledger'",((5,0,0 \ 1,-1,0 \ 4,-1,0 \ 2,5,0 \ 3,5,1 \ 3,5,2 \ 6,3,0),J(7,12,0)))
mata: compatibility_ledger=st_matrix("`ledger'")
mata: compatibility_ledger[.,4..9]=J(7,1,(2,7,1e-13,2e-13,0,1))
mata: st_matrix("`ledger'",compatibility_ledger)
foreach kind in 1 2 {
    fevc__rust_rhs_diagnostics `ledger' `actual' 2 3 `kind' 100 1e-10 1e-10
    matrix `expected'=(1,1,1 \ 2,1,1 \ 3,1,1 \ 4,5,6 \ 5,4,11 \ 5,4,12 \ 5,4,4)
    if `kind'==2 matrix `expected'[7,1]=6
    mata: assert(st_matrix("`actual'")[.,1..3]==st_matrix("`expected'"))
    mata: assert(st_matrix("`actual'")[.,4..6]==J(7,1,(7,2e-13,1)))
}
mata: st_matrix("`ledger'",compatibility_ledger[(2,4,5,6),1..8])
foreach kind in 3 4 {
    fevc__rust_rhs_diagnostics `ledger' `actual' 2 3 `kind' 100 1e-10 1e-10
    matrix `expected'=(2,1,1 \ 4,5,6 \ 5,4,11 \ 5,4,12)
    mata: assert(st_matrix("`actual'")[.,1..3]==st_matrix("`expected'"))
    mata: assert(st_matrix("`actual'")[.,4..6]==J(4,1,(7,2e-13,1)))
}
* Mata export supports complete ledgers beyond the legacy matsize ceiling.
mata: st_matrix("`ledger'",J(11002,1,compatibility_ledger[6,.]))
fevc__rust_rhs_diagnostics `ledger' `actual' 2 3 2 0 0 0
assert rowsof(`actual')==11002 & colsof(`actual')==6
mata: assert(st_matrix("`actual'")==J(11002,1,(5,4,12,7,2e-13,1)))
mata: mata drop compatibility_ledger

program drop _fevc_compat_results
display as result "PASS test_stata_compatibility.do"

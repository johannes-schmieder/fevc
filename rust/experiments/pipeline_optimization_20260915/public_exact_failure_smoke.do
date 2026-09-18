version 18.0
clear all
set more off
args package_dir
adopath ++ `"`package_dir'"'
quietly run `"`package_dir'/fevc.ado"'
set processors 4
set obs 96
generate long key = _n
generate long worker = floor((_n-1)/8)+1
generate int firm = mod(floor((_n-1)/2),4)+1
generate double y = .7*worker-.4*firm+.3*mod(key,2)+mod(7*key,5)/11
sort key
local rng `"`c(rngstate)'"'
local sortrng `"`c(sortrngstate)'"'
quietly _datasignature
local signature `"`r(datasignature)'"'
capture program drop _fevc_rust_public_call
program define _fevc_rust_public_call, rclass
    version 18.0
    gettoken command rest : 0
    if "`command'"=="solve" & "$FEVC_EXACT_FAULT"=="break" exit 1
    fevc_rust `command' `rest'
    tempname work
    local mutate = 0
    if "$FEVC_EXACT_FAULT"!="" & "$FEVC_EXACT_FAULT"!="break" {
        if "`command'"=="exactexecutionreceipt" {
            matrix `work' = r(receipt)
            local mutate = 1
        }
        if "`command'"=="result" & "$FEVC_EXACT_ALGORITHM"=="auto" {
            matrix `work' = r(exact_execution_receipt)
            local mutate = 2
        }
    }
    if `mutate' matrix `work'[1,real("$FEVC_EXACT_FAULT")] = -1
    return add
    if `mutate'==1 return matrix receipt = `work'
    if `mutate'==2 return matrix exact_execution_receipt = `work'
end
foreach deletion in observation match {
    foreach algorithm in exact auto {
        global FEVC_EXACT_ALGORITHM `algorithm'
        foreach fault in 1 2 3 4 5 6 7 8 9 10 break {
            global FEVC_EXACT_FAULT `fault'
            capture noisily fevc y, worker(worker) firm(firm) deletion(`deletion') ///
                stayers(movers) algorithm(`algorithm') backend(rust) nodisplay
            if "`fault'"=="break" assert _rc==1
            else assert _rc==498
            quietly fevc_rust snapshot
            assert r(state)==0 & r(handle)==0
            assert `"`c(rngstate)'"'==`"`rng'"'
            assert `"`c(sortrngstate)'"'==`"`sortrng'"'
            quietly _datasignature
            assert `"`r(datasignature)'"'==`"`signature'"'
        }
        macro drop FEVC_EXACT_FAULT
        quietly fevc y, worker(worker) firm(firm) deletion(`deletion') ///
            stayers(movers) algorithm(`algorithm') backend(rust) nodisplay
        assert "`e(rust_execution_mode)'"=="exact_parallel"
    }
}
macro drop FEVC_EXACT_ALGORITHM
display as result "PASS public_exact_failure_smoke.do"

version 18.0
clear all
set more off
set varabbrev off
args package_dir test_threads
if "`test_threads'"=="" local test_threads = 4
adopath ++ `"`package_dir'"'
quietly run `"`package_dir'/fevc.ado"'
set processors `test_threads'
set obs 2048
generate long observation_key = _n
generate long worker = ceil(_n/4)
generate byte slot = mod(_n-1,4)
generate int home_firm = mod(worker-1,256)+1
generate int firm = cond(slot<2,home_firm,mod(home_firm,256)+1)
generate double outcome = .31*worker-.23*firm+.07*slot+ ///
    mod(17*observation_key+3,29)/101
sort worker firm observation_key

set rng kiss32
set seed 20260826
set sortseed 20260826
local caller_rng `"`c(rng)'"'
local caller_stream = c(rngstream)
local caller_state `"`c(rngstate)'"'
local caller_sort_state `"`c(sortrngstate)'"'
local caller_sortedby : sortedby
quietly _datasignature
local caller_signature `"`r(datasignature)'"'

foreach policy in error warn off {
    capture noisily fevc outcome, worker(worker) firm(firm) deletion(match) ///
        stayers(movers) algorithm(jla) backend(rust) rng(counter_v1) ///
        engine(generic) preconditioner(cmg) batch(auto) probes(4) ///
        memory_gib(.000001) memorycheck(`policy') mcse(conditional) nodisplay
    if "`policy'"=="error" assert _rc!=0
    else {
        assert _rc==0
        assert "`e(cmg_backend)'"=="CMG_FULL_V2"
    }
    quietly fevc_rust snapshot
    assert r(state)==0 & r(handle)==0
    assert `"`c(rngstate)'"'==`"`caller_state'"'
    assert `"`c(sortrngstate)'"'==`"`caller_sort_state'"'
}
// Inject transport corruption and r(1) at the public boundary. Native callback
// cancellation and worker retirement are tested separately by the FFI suite.
capture program drop fevc__rust_public_call
program define fevc__rust_public_call, rclass
    version 18.0
    gettoken command rest : 0
    if "`command'"=="solve" & "$FEVC_CMG_FAULT"=="break" exit 1
    fevc_rust `command' `rest'
    return add
    if "`command'"=="fullcmgreceipt" & "$FEVC_CMG_FAULT"=="source" ///
        return local cmg_source_commit "0000000000000000000000000000000000000000"
    if "`command'"=="fullcmgreceipt" & "$FEVC_CMG_FAULT"=="threads" ///
        return scalar threads_used = 0
end
foreach fault in source threads break {
    global FEVC_CMG_FAULT `fault'
    capture noisily fevc outcome, worker(worker) firm(firm) deletion(match) ///
        stayers(movers) algorithm(jla) backend(rust) rng(counter_v1) ///
        engine(generic) preconditioner(cmg) batch(auto) probes(4) ///
        mcse(conditional) nodisplay
    if "`fault'"=="break" assert _rc==1
    else assert _rc==498
    if "`fault'"=="break" assert "`e(cmd)'"==""
    else assert "`e(status)'"=="WITHHELD"
    capture confirm matrix e(kss)
    assert _rc!=0
    quietly fevc_rust snapshot
    assert r(state)==0 & r(handle)==0
    assert `"`c(rng)'"'==`"`caller_rng'"'
    assert c(rngstream)==`caller_stream'
    assert `"`c(rngstate)'"'==`"`caller_state'"'
    assert `"`c(sortrngstate)'"'==`"`caller_sort_state'"'
    local restored_sortedby : sortedby
    assert `"`restored_sortedby'"'==`"`caller_sortedby'"'
    quietly _datasignature
    assert `"`r(datasignature)'"'==`"`caller_signature'"'
}
macro drop FEVC_CMG_FAULT
capture program drop fevc__rust_public_call
program define fevc__rust_public_call, rclass
    version 18.0
    fevc_rust `0'
    return add
end
quietly fevc outcome, worker(worker) firm(firm) deletion(match) ///
    stayers(movers) algorithm(jla) backend(rust) rng(counter_v1) ///
    engine(generic) preconditioner(cmg) batch(auto) probes(4) nodisplay
assert "`e(cmg_backend)'"=="CMG_FULL_V2"
assert e(complete_residual_max)<=e(residual_acceptance_tolerance)
quietly fevc_rust snapshot
assert r(state)==0 & r(handle)==0
do "fevc/tests/stata/test_rust_full_cmg_result_cache.do" `"`package_dir'"' `test_threads'
display as result "PASS test_rust_full_cmg_lifecycle.do"

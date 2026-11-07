version 18.0
clear all
set more off
set varabbrev off

args package_dir
if `"`package_dir'"' == "" {
    local package_dir = subinstr("`c(pwd)'","/tests/stata","",.)
}
adopath ++ `"`package_dir'"'
quietly run `"`package_dir'/fevc.ado"'
quietly fevc_rust clear

// An installed Windows candidate must execute the shared CMG runtime.
assert `"`c(os)'"' == "Windows"
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

foreach threads in 1 2 {
set processors `threads'
foreach route in auto cmg {
foreach mcse in all conditional off {
quietly fevc outcome, worker(worker) firm(firm) deletion(match) ///
    nuisance(joint) stayers(movers) probeorder(observation_key) ///
    backend(rust) rng(counter_v1) algorithm(jla) engine(auto)  ///
    preconditioner(`route') batch(auto) probes(4) seed(81227)  ///
    maxiter(10000) memory_gib(1) mcse(`mcse') nodisplay

assert `"`e(cmd)'"' == "fevc"
assert `"`e(backend_requested)'"' == "rust"
assert `"`e(backend_selected)'"' == "rust"
assert `"`e(rng_selected)'"' == "counter_v1"
assert `"`e(algorithm)'"' == "jla"
assert `"`e(engine_selected)'"' == "compressed"
assert `"`e(preconditioner_selected)'"' == "CMG"
assert `"`e(cmg_backend)'"' == "CMG_FULL_V2"
assert `"`e(cmg_source_commit)'"' == "e5613e81fb3b03c78de98658b372646fa802aabb"
assert e(full_cmg_receipt)[1,2] == 3
assert e(full_cmg_receipt)[1,3] == 2
assert e(cmg_threads_requested) == `threads' & e(cmg_threads_used) == `threads'
assert e(backend_fallback) == 0
assert e(complete_residual_max) <= e(residual_acceptance_tolerance)
assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_stream'
assert `"`c(rngstate)'"' == `"`caller_state'"'
assert `"`c(sortrngstate)'"' == `"`caller_sort_state'"'
local restored_sortedby : sortedby
assert `"`restored_sortedby'"' == `"`caller_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'
quietly fevc_rust snapshot
assert r(state) == 0 & r(handle) == 0
}
}
}
display as result "PASS test_windows_full_cmg_boundary.do"
exit 0

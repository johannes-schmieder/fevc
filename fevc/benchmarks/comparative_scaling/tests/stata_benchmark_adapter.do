version 18.0
clear all
set more off
set varabbrev off

args package_dir benchmark_ado
if `"`package_dir'"' == "" | `"`benchmark_ado'"' == "" exit 198
confirm file `"`package_dir'/vckss.mata"'
confirm file `"`benchmark_ado'"'
adopath ++ `"`package_dir'"'
quietly run `"`benchmark_ado'"'
quietly fevc_rust clear
set processors 4
assert c(processors) == 4

// This registered automatic full-CMG cell is deliberately small enough for
// a local gate while still proving that the native pool is not c(processors).
set obs 2048
generate long observation_key = _n
generate long worker = ceil(_n/4)
generate byte slot = mod(_n-1,4)
generate int home_firm = mod(worker-1,256)+1
generate int firm = cond(slot<2,home_firm,mod(home_firm,256)+1)
generate double outcome = .31*worker-.23*firm+.07*slot+ ///
    mod(17*observation_key+3,29)/101
sort worker firm observation_key

quietly fevc outcome, worker(worker) firm(firm) deletion(match) ///
    nuisance(joint) stayers(movers) probeorder(observation_key) ///
    backend(rust) rng(counter_v1) algorithm(jla) engine(auto)  ///
    preconditioner(auto) batch(auto) probes(4) seed(81227)     ///
    maxiter(10000) memory_gib(1) nodisplay

assert c(processors) == 4
assert `"`e(cmg_backend)'"' == "CMG_FULL_V2"
assert e(cmg_threads_requested) == 8
assert e(cmg_threads_used) == 8
assert e(full_cmg_receipt)[1,5] == 8
assert e(full_cmg_receipt)[1,6] == 8
quietly fevc_rust snapshot
assert r(state) == 0 & r(handle) == 0

di as result "FEVC BENCHMARK ADAPTER STATA PASS"
exit 0

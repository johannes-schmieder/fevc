version 18.0
clear all
set more off
set varabbrev off

args package_dir
if `"`package_dir'"' == "" {
    local package_dir = subinstr("`c(pwd)'","/tests/stata","",.)
}
adopath ++ `"`package_dir'"'
quietly run `"`package_dir'/vckss.ado"'
quietly vckss_rust clear

// A connected 256-firm mover cycle crosses the registered automatic-CMG
// threshold while remaining small enough for every installed-package gate.
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

quietly vckss outcome, worker(worker) firm(firm) deletion(match) ///
    nuisance(joint) stayers(movers) probeorder(observation_key) ///
    backend(rust) rng(counter_v1) algorithm(jla) engine(auto)  ///
    preconditioner(auto) batch(auto) probes(4) seed(81227)     ///
    maxiter(10000) memory_gib(1) nodisplay

assert `"`e(cmd)'"' == "vckss"
assert `"`e(version)'"' == "0.4.0-dev"
assert `"`e(backend_requested)'"' == "rust"
assert `"`e(backend_selected)'"' == "rust"
assert `"`e(rng_selected)'"' == "counter_v1"
assert `"`e(algorithm)'"' == "jla"
assert `"`e(engine_selected)'"' == "compressed"
assert `"`e(preconditioner_selected)'"' == "CMG"
assert `"`e(cmg_backend)'"' == "CMG_FULL_V2"
assert `"`e(cmg_source_commit)'"' == ///
    "dbefbc5e3b442c6dde6e7861a66d82fd5ed24f10"
assert e(cmg_threads_requested) == c(processors)
assert e(cmg_threads_used) == c(processors)
assert e(cmg_admitted_peak_bytes) > 0
assert e(cmg_admitted_peak_bytes) <= e(memory_limit_bytes)
assert e(cmg_max_complete_residual) <= e(residual_acceptance_tolerance)
assert e(cmg_refinement_attempts) >= 0
assert e(cmg_refined_columns) >= 0
assert rowsof(e(full_cmg_receipt)) == 1
assert colsof(e(full_cmg_receipt)) == 46
assert e(full_cmg_receipt)[1,1] == 2
assert e(full_cmg_receipt)[1,18] == 1e-10
assert e(full_cmg_receipt)[1,19] == 1e-6
assert e(full_cmg_receipt)[1,20] == 1e-12
assert e(full_cmg_receipt)[1,21] == 1e-6
assert e(full_cmg_receipt)[1,25] == 1+3*e(probes)
assert e(full_cmg_receipt)[1,39] > 0
assert e(full_cmg_receipt)[1,40] > 0
assert e(full_cmg_receipt)[1,41] >= e(full_cmg_receipt)[1,40]
assert e(full_cmg_receipt)[1,42] >= e(full_cmg_receipt)[1,17]
assert e(full_cmg_receipt)[1,43] > 0
assert e(full_cmg_receipt)[1,44] == floor(e(full_cmg_receipt)[1,43]/5)
assert e(full_cmg_receipt)[1,45] == 64
assert e(full_cmg_receipt)[1,46] > 0
assert e(rust_probeorder_supplied) == 1
assert e(rust_pre_rng_hi) == 0 & e(rust_pre_rng_lo) == 0
assert e(complete_residual_max) <= e(residual_acceptance_tolerance)
assert e(target_identity_residual) == e(rust_actual_accounting_residual)
capture confirm matrix e(V)
assert _rc != 0
quietly count if e(sample)
assert r(N) == e(N_retained)
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rng)'"' == `"`caller_rng'"'
assert c(rngstream) == `caller_stream'
assert `"`c(rngstate)'"' == `"`caller_state'"'
assert `"`c(sortrngstate)'"' == `"`caller_sort_state'"'
local restored_sortedby : sortedby
assert `"`restored_sortedby'"' == `"`caller_sortedby'"'
quietly _datasignature
assert `"`r(datasignature)'"' == `"`caller_signature'"'

// An explicit tolerance overrides both phase defaults without changing route.
quietly vckss outcome, worker(worker) firm(firm) deletion(match) ///
    nuisance(joint) stayers(movers) probeorder(observation_key) ///
    backend(rust) rng(counter_v1) algorithm(jla) engine(auto)  ///
    preconditioner(auto) batch(auto) probes(4) seed(81227)     ///
    tolerance(1e-8) maxiter(10000) memory_gib(1) nodisplay
assert `"`e(cmg_backend)'"' == "CMG_FULL_V2"
assert e(full_cmg_receipt)[1,18] == 1e-8
assert e(full_cmg_receipt)[1,19] == 1e-8
assert e(full_cmg_receipt)[1,20] == 1e-10
assert e(full_cmg_receipt)[1,21] == 1e-8
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rngstate)'"' == `"`caller_state'"'

// Before cross-platform qualification, the otherwise eligible automatic
// request retains the existing qualified Rust route and cannot claim V2.
quietly vckss outcome, worker(worker) firm(firm) deletion(match) ///
    nuisance(joint) stayers(movers) probeorder(observation_key) ///
    backend(auto) rng(auto) algorithm(jla) engine(auto)        ///
    preconditioner(auto) batch(auto) probes(4) seed(81227)     ///
    maxiter(10000) memory_gib(1) nodisplay
assert `"`e(backend_selected)'"' == "rust"
assert `"`e(cmg_backend)'"' == ""
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rngstate)'"' == `"`caller_state'"'

// Omitting the registered observation-key tie breaker retains the existing
// qualified Rust route and must not claim the production full-CMG identity.
quietly vckss outcome, worker(worker) firm(firm) deletion(match) ///
    nuisance(joint) stayers(movers) backend(rust) rng(counter_v1) ///
    algorithm(jla) engine(auto) preconditioner(auto) batch(auto) ///
    probes(4) seed(81227) maxiter(10000) memory_gib(1) nodisplay
assert `"`e(backend_selected)'"' == "rust"
assert `"`e(cmg_backend)'"' == ""
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rngstate)'"' == `"`caller_state'"'

di as result "PASS test_rust_full_cmg_v2.do"

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
assert `"`e(version)'"' == "0.4.0-alpha.1"
assert `"`e(backend_requested)'"' == "rust"
assert `"`e(backend_selected)'"' == "rust"
assert `"`e(rng_selected)'"' == "counter_v1"
assert `"`e(algorithm)'"' == "jla"
assert `"`e(engine_selected)'"' == "compressed"
assert `"`e(preconditioner_selected)'"' == "CMG"
assert `"`e(cmg_backend)'"' == "CMG_FULL_V2"
assert `"`e(cmg_source_commit)'"' == ///
    "98768722fa21800d2cb91cd2182406c9db3bf979"
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

// After macOS and SCC qualification, the same effective automatic cell
// selects the production identity and keeps requested/selected routing
// metadata truthful.
quietly vckss outcome, worker(worker) firm(firm) deletion(match) ///
    nuisance(joint) stayers(movers) probeorder(observation_key) ///
    backend(auto) rng(auto) algorithm(jla) engine(auto)        ///
    preconditioner(auto) batch(auto) probes(4) seed(81227)     ///
    maxiter(10000) memory_gib(1) nodisplay
assert `"`e(backend_selected)'"' == "rust"
assert `"`e(backend_requested)'"' == "auto"
assert `"`e(rng_requested)'"' == "auto"
assert `"`e(rng_selected)'"' == "counter_v1"
assert e(backend_option_supplied) == 1
assert e(rng_option_supplied) == 1
assert e(backend_fallback) == 0
assert `"`e(cmg_backend)'"' == "CMG_FULL_V2"
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rngstate)'"' == `"`caller_state'"'

// Omitted routing and tuning options resolve to the same qualified effective
// cell. The explicit semantic probe order remains mandatory.
quietly vckss outcome, worker(worker) firm(firm)            ///
    probeorder(observation_key) probes(4) seed(81227)       ///
    maxiter(10000) memory_gib(1) nodisplay
assert `"`e(backend_requested)'"' == "auto"
assert `"`e(backend_selected)'"' == "rust"
assert `"`e(rng_requested)'"' == "auto"
assert `"`e(rng_selected)'"' == "counter_v1"
assert e(backend_option_supplied) == 0
assert e(rng_option_supplied) == 0
assert e(algorithm_option_supplied) == 0
assert e(engine_option_supplied) == 0
assert e(preconditioner_option_supplied) == 0
assert e(batch_option_supplied) == 0
assert e(backend_fallback) == 0
assert `"`e(cmg_backend)'"' == "CMG_FULL_V2"
quietly vckss_rust snapshot
assert r(state) == 0 & r(handle) == 0
assert `"`c(rngstate)'"' == `"`caller_state'"'

// Once automatic routing selects full CMG, native admission failure is typed
// and fail-closed rather than a post-selection Mata fallback.
capture quietly vckss outcome, worker(worker) firm(firm)    ///
    probeorder(observation_key) probes(4) seed(81227)       ///
    maxiter(10000) memory_gib(.000001) nodisplay
assert _rc != 0
assert inlist(`"`e(withholding_status)'"',"RESOURCE_LIMIT", ///
    "ALLOCATION_FAILED")
assert e(backend_fallback) == 0
assert `"`e(backend_requested)'"' == "auto"
assert `"`e(backend_selected)'"' == ""
assert `"`e(rng_requested)'"' == "auto"
assert `"`e(rng_selected)'"' == ""
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

// Explicit Mata remains permanently available for the same statistical
// request and never claims a native CMG receipt.
quietly vckss outcome, worker(worker) firm(firm) deletion(match) ///
    nuisance(joint) stayers(movers) probeorder(observation_key) ///
    backend(mata) rng(stata) algorithm(jla) engine(auto)       ///
    preconditioner(auto) batch(auto) probes(4) seed(81227)     ///
    maxiter(10000) memory_gib(1) nodisplay
assert `"`e(backend_requested)'"' == "mata"
assert `"`e(backend_selected)'"' == "mata"
assert `"`e(rng_selected)'"' == "stata"
assert e(backend_fallback) == 0
assert `"`e(cmg_backend)'"' == ""
assert `"`c(rngstate)'"' == `"`caller_state'"'

di as result "PASS test_rust_full_cmg_v2.do"

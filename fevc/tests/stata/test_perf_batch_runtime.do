version 18.0
clear all
set more off
set varabbrev off

local oldpwd `"`c(pwd)'"'
capture confirm file "fevc/fevc.ado"
if _rc {
    capture confirm file "../../fevc.ado"
    if _rc exit 601
    quietly cd "../.."
    local pkgroot `"`c(pwd)'"'
}
else local pkgroot `"`c(pwd)'/fevc"'
adopath ++ `"`pkgroot'"'

local workers = 1200
local firms = 120
local probes = 24
set obs `=4*`workers''
generate long worker = floor((_n-1)/4)+1
generate byte period = mod(_n-1,4)+1
generate long home_firm = mod(worker-1,`firms')+1
generate long firm = cond(period<=2,home_firm,mod(home_firm,`firms')+1)
generate long match = 2*(worker-1)+(period>2)+1
generate long observation_key = _n
generate double target = .75+mod(_n,17)/17
generate double y = sin(worker/97)+cos(firm/31)+period/101

// This regression times and counts the Mata lockstep implementation. Rust
// complete-command benchmarking uses the registered benchmark harness.
timer clear 80
timer on 80
fevc y, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) targetweight(target) probeorder(observation_key) ///
    algorithm(jla) preconditioner(diagonal) memory_gib(4) probes(`probes') ///
    batch(1) seed(8675309) ///
    tolerance(1e-10) maxiter(20000) backend(mata) rng(stata) nodisplay
timer off 80
quietly timer list 80
scalar batch1_seconds = r(t80)
matrix batch1_results = e(results)
scalar batch1_schur_batches = e(solver_schur_batches)
scalar batch1_precond_batches = e(solver_precond_batches)
local batch1_rng `"`c(rngstate)'"'
assert e(setup_seconds) >= 0
assert e(fit_seconds) >= 0
assert e(leverage_seconds) >= 0
assert e(target_seconds) >= 0
assert e(setup_seconds)+e(fit_seconds)+e(leverage_seconds)+ ///
    e(target_seconds) <= batch1_seconds+0.05

timer clear 81
timer on 81
fevc y, worker(worker) firm(firm) deletion(match) ///
    deletionid(match) targetweight(target) probeorder(observation_key) ///
    algorithm(jla) preconditioner(diagonal) memory_gib(4) probes(`probes') ///
    batch(8) seed(8675309) ///
    tolerance(1e-10) maxiter(20000) backend(mata) rng(stata) nodisplay
timer off 81
quietly timer list 81
scalar batch8_seconds = r(t81)
assert `"`c(rngstate)'"' == `"`batch1_rng'"'
assert mreldif(batch1_results,e(results)) < 2e-11
assert e(solver_max_residual) <= 1e-10
assert e(solver_schur_batches) < batch1_schur_batches
assert e(solver_precond_batches) < batch1_precond_batches
assert e(setup_seconds)+e(fit_seconds)+e(leverage_seconds)+ ///
    e(target_seconds) <= batch8_seconds+0.05

di as txt "batch(1) seconds: " %9.3f scalar(batch1_seconds)
di as txt "batch(8) seconds: " %9.3f scalar(batch8_seconds)
di as txt "observed speed ratio batch(1)/batch(8): " ///
    %9.3f scalar(batch1_seconds)/scalar(batch8_seconds)
quietly cd `"`oldpwd'"'
di as result "PASS test_perf_batch_runtime.do"
exit 0

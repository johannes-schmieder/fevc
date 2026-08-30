version 18.0
clear all
set more off
set varabbrev off

args pkgroot
if `"`pkgroot'"' == "" local pkgroot `"`c(pwd)'/vckss"'

adopath ++ `"`pkgroot'"'
do `"`pkgroot'/tests/stata/test_load.do"'

set obs 240
generate long worker = floor((_n-1)/6)
generate byte time = mod(_n-1,6)
generate long firm = mod(worker + floor(time/2),20)
generate double c1 = time - 2.5
generate double z = sin(worker/5) + cos(firm/3) + time/20
generate double noise = .25*sin((_n*17)/11) + .15*cos((_n*7)/13)
generate double y = 1 + .08*worker - .12*firm + .3*c1 + noise
generate double target_mass = 1 + mod(_n,5)/10

quietly vckss y c1, worker(worker) firm(firm) deletion(observation) ///
    algorithm(exact) backend(mata) project(z) projecteffect(firm) nodisplay
matrix exact_b = e(projection_b)
matrix exact_V = e(projection_V)

quietly vckss y c1, worker(worker) firm(firm) deletion(observation) ///
    algorithm(jla) engine(generic) backend(rust) rng(counter_v1) ///
    preconditioner(diagonal) batch(8) probes(400) tolerance(1e-12) ///
    project(z) projecteffect(firm) nodisplay

assert `"`e(status)'"' == "KSS_PROJECTION_INFERENCE"
assert `"`e(inference_method)'"' == "sparse JLA observation projection"
assert `"`e(projection_effect)'"' == "firm"
assert `"`e(projection_weight)'"' == "frequency"
assert e(projection_columns) == 2
assert rowsof(e(projection_b)) == 1 & colsof(e(projection_b)) == 2
assert rowsof(e(projection_V)) == 2 & colsof(e(projection_V)) == 2
assert rowsof(e(projection_V_naive)) == 2 & colsof(e(projection_V_naive)) == 2
assert rowsof(e(projection_results)) == 2 & colsof(e(projection_results)) == 7
assert rowsof(e(projection_diagnostics)) == 1
assert colsof(e(projection_diagnostics)) == 16
assert rowsof(e(projection_augmentation_receipt)) == 1
assert colsof(e(projection_augmentation_receipt)) == 12
assert rowsof(e(projection_solver_diagnostics)) == 2
assert e(rust_plan_rhs) == 1204
assert e(projection_result_bytes) == 80
assert e(projection_solver_max_complete) <= e(residual_acceptance_tolerance)
assert e(projection_peak_forecast_bytes) <= e(batch_memory_budget_bytes)
assert mreldif(exact_b,e(projection_b)) < 1e-11
assert mreldif(exact_V,e(projection_V)) < 1e-3
matrix rhs = e(rust_rhs_receipts)
assert rhs[rowsof(rhs)-1,1] == 6
assert rhs[rowsof(rhs),1] == 6
assert rhs[rowsof(rhs)-1,2] == 0
assert rhs[rowsof(rhs),2] == 1

quietly vckss y, worker(worker) firm(firm) deletion(observation) ///
    algorithm(exact) backend(mata) targetweight(target_mass) ///
    project(z) projecteffect(worker) projectweight(target) nodisplay
matrix exact_worker_b = e(projection_b)

quietly vckss y, worker(worker) firm(firm) deletion(observation) ///
    algorithm(jla) engine(generic) backend(rust) rng(counter_v1) ///
    preconditioner(diagonal) batch(8) probes(200) tolerance(1e-12) ///
    targetweight(target_mass) project(z) projecteffect(worker) ///
    projectweight(target) nodisplay
assert `"`e(projection_effect)'"' == "worker"
assert `"`e(projection_weight)'"' == "target"
assert mreldif(exact_worker_b,e(projection_b)) < 1e-11
assert e(projection_solver_max_complete) <= e(residual_acceptance_tolerance)

// Automatic/CMG solver routing is outside the first qualified projection
// tuple; strict Rust requests must fail before native preparation.
capture noisily vckss y, worker(worker) firm(firm) deletion(observation) ///
    algorithm(jla) engine(generic) backend(rust) rng(counter_v1)        ///
    project(z) projecteffect(worker) nodisplay
assert _rc == 498
assert `"`e(status)'"' == "WITHHELD"
assert `"`e(withholding_status)'"' == "RUST_INFERENCE_UNSUPPORTED"
vckss_rust snapshot
assert r(state) == 0

di as result "PASS test_rust_projection.do"
exit 0

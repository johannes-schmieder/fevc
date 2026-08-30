version 18.0
clear all
set more off
set varabbrev off
set processors 4

args pkgroot
if `"`pkgroot'"' == "" local pkgroot `"`c(pwd)'/vckss"'
local fixture `"`pkgroot'/qualification/inference_matlab/evidence/input.csv"'

adopath ++ `"`pkgroot'"'
do `"`pkgroot'/tests/stata/test_load.do"'
import delimited using `"`fixture'"', clear varnames(1)

quietly vckss y, worker(worker) firm(firm) deletion(observation) ///
    algorithm(exact) backend(mata) project(z1 z2) ///
    projecteffect(firm) nodisplay
matrix exact_b = e(projection_b)
matrix exact_V = e(projection_V)

timer clear 1
timer on 1
quietly vckss y, worker(worker) firm(firm) deletion(observation) ///
    algorithm(jla) engine(generic) backend(rust) rng(counter_v1) ///
    preconditioner(diagonal) batch(16) probes(2000) tolerance(1e-12) ///
    project(z1 z2) projecteffect(firm) nodisplay
timer off 1

matrix diagonal_b = e(projection_b)
matrix diagonal_V = e(projection_V)
assert mreldif(exact_b,diagonal_b) < 1e-10
assert mreldif(exact_V,diagonal_V) < 5e-4

timer clear 2
timer on 2
quietly vckss y, worker(worker) firm(firm) deletion(observation) ///
    algorithm(jla) engine(generic) backend(rust) rng(counter_v1) ///
    preconditioner(cmg) batch(16) probes(2000) tolerance(1e-12) ///
    project(z1 z2) projecteffect(firm) nodisplay
timer off 2

matrix scalable_b = e(projection_b)
matrix scalable_V = e(projection_V)
assert `"`e(preconditioner_selected)'"' == "CMG"
assert mreldif(exact_b,scalable_b) < 1e-10
assert mreldif(exact_V,scalable_V) < 5e-4
assert mreldif(diagonal_b,scalable_b) < 1e-10
assert mreldif(diagonal_V,scalable_V) < 1e-9

// Maintained lincom_KSS values from the immutable comparison bundle.
assert abs(scalable_b[1,2] - (-.142097411282)) < 2e-8
assert abs(scalable_b[1,3] - (-.138262856401)) < 2e-8
local matlab_se_z1 = .021076747813
local matlab_se_z2 = .043094579972
assert abs(sqrt(scalable_V[2,2])/`matlab_se_z1' - 1) < .005
assert abs(sqrt(scalable_V[3,3])/`matlab_se_z2' - 1) < .005

assert e(projection_solver_max_complete) <= e(residual_acceptance_tolerance)
assert e(projection_covariance_min) >= -1e-10
assert e(projection_peak_forecast_bytes) <= e(batch_memory_budget_bytes)
assert e(rust_plan_rhs) == 6004
matrix rhs = e(rust_rhs_receipts)
assert rhs[rowsof(rhs)-2,1] == 6
assert rhs[rowsof(rhs),1] == 6

timer list 1
timer list 2
di as result "PASS vckss_scalable_projection.do"
exit 0

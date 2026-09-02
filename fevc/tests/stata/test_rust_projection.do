version 18.0
clear all
set more off
set varabbrev off

args pkgroot
if `"`pkgroot'"' == "" local pkgroot `"`c(pwd)'/fevc"'

adopath ++ `"`pkgroot'"'
do `"`pkgroot'/tests/stata/test_load.do"'

set obs 240
generate long worker = floor((_n-1)/6)
generate byte time = mod(_n-1,6)
generate long firm = mod(worker + floor(time/2),20)
generate double c1 = time - 2.5
generate double z = sin(worker/5) + cos(firm/3) + time/20
generate double noise = .25*sin((_n*17)/11) + .15*cos((_n*7)/13)
// An outcome location shift leaves fitted effects and deleted residuals
// equivalent but avoids an incidental indefinite finite-sample cross-fit
// covariance in this solver-route test.
generate double y = -4 + .08*worker - .12*firm + .3*c1 + noise
generate double target_mass = 1 + mod(_n,5)/10

quietly fevc y c1, worker(worker) firm(firm) deletion(observation) ///
    algorithm(exact) backend(mata) project(z) projecteffect(firm) nodisplay
matrix exact_b = e(projection_b)
matrix exact_V = e(projection_V)

quietly fevc y c1, worker(worker) firm(firm) deletion(observation) ///
    algorithm(jla) engine(generic) backend(rust) rng(counter_v1) ///
    preconditioner(diagonal) batch(8) probes(400) tolerance(1e-12) ///
    project(z) projecteffect(firm) nodisplay
matrix diagonal_b = e(projection_b)
matrix diagonal_V = e(projection_V)

assert `"`e(status)'"' == "KSS_PROJECTION_INFERENCE"
assert `"`e(inference_method)'"' == "sparse JLA block cross-fit projection"
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

// The scalable projection route reuses the already-qualified planned generic
// CMG hierarchy.  Identical Counter-V1 probes must therefore change only the
// deterministic linear-solver path, not the estimand or covariance formula.
quietly fevc y c1, worker(worker) firm(firm) deletion(observation) ///
    algorithm(jla) engine(generic) backend(rust) rng(counter_v1) ///
    preconditioner(cmg) batch(8) probes(400) tolerance(1e-12) ///
    project(z) projecteffect(firm) nodisplay
assert `"`e(preconditioner_requested)'"' == "cmg"
assert `"`e(preconditioner_selected)'"' == "CMG"
assert `"`e(fallback_status)'"' == "NOT_NEEDED"
assert e(projection_solver_max_complete) <= e(residual_acceptance_tolerance)
assert e(complete_residual_max) <= e(residual_acceptance_tolerance)
assert e(projection_peak_forecast_bytes) <= e(batch_memory_budget_bytes)
assert mreldif(exact_b,e(projection_b)) < 1e-11
assert mreldif(diagonal_b,e(projection_b)) < 1e-11
assert mreldif(diagonal_V,e(projection_V)) < 1e-9

quietly fevc y, worker(worker) firm(firm) deletion(observation) ///
    algorithm(exact) backend(mata) targetweight(target_mass) ///
    project(z) projecteffect(worker) projectweight(target) nodisplay
matrix exact_worker_b = e(projection_b)

quietly fevc y, worker(worker) firm(firm) deletion(observation) ///
    algorithm(jla) engine(generic) backend(rust) rng(counter_v1) ///
    preconditioner(diagonal) batch(8) probes(200) tolerance(1e-12) ///
    targetweight(target_mass) project(z) projecteffect(worker) ///
    projectweight(target) nodisplay
assert `"`e(projection_effect)'"' == "worker"
assert `"`e(projection_weight)'"' == "target"
assert mreldif(exact_worker_b,e(projection_b)) < 1e-11
assert e(projection_solver_max_complete) <= e(residual_acceptance_tolerance)

// Positive integer frequency weights are literal physical copies.  Both
// frequency-mass and explicit stored-row target-mass projections must agree
// with a unit-frequency expansion of the same physical sample.
generate byte copies = 2
generate long stored_row = _n

quietly fevc y c1 [fw=copies], worker(worker) firm(firm)       ///
    deletion(observation) algorithm(exact) backend(mata)        ///
    project(z) projecteffect(firm) projectweight(frequency) nodisplay
matrix weighted_exact_frequency_b = e(projection_b)
matrix weighted_exact_frequency_V = e(projection_V)
matrix weighted_exact_frequency_naive = e(projection_V_naive)

quietly fevc y c1 [fw=copies], worker(worker) firm(firm)       ///
    deletion(observation) algorithm(exact) backend(mata)        ///
    targetweight(target_mass) project(z) projecteffect(worker)  ///
    projectweight(target) nodisplay
matrix weighted_exact_target_b = e(projection_b)
matrix weighted_exact_target_V = e(projection_V)
matrix weighted_exact_target_naive = e(projection_V_naive)

quietly fevc y c1 [fw=copies], worker(worker) firm(firm)       ///
    deletion(observation) algorithm(jla) engine(generic)        ///
    backend(rust) rng(counter_v1) preconditioner(diagonal)      ///
    batch(8) probes(600) tolerance(1e-12) project(z)            ///
    projecteffect(firm) projectweight(frequency) nodisplay
matrix weighted_rust_frequency_b = e(projection_b)
matrix weighted_rust_frequency_V = e(projection_V)
assert e(N_physical) == 480
assert e(projection_solver_max_complete) <= e(residual_acceptance_tolerance)

quietly fevc y c1 [fw=copies], worker(worker) firm(firm)       ///
    deletion(observation) algorithm(jla) engine(generic)        ///
    backend(rust) rng(counter_v1) preconditioner(cmg)           ///
    batch(8) probes(600) tolerance(1e-12) project(z)            ///
    projecteffect(firm) projectweight(frequency) nodisplay
assert `"`e(preconditioner_selected)'"' == "CMG"
assert e(N_physical) == 480
assert e(projection_solver_max_complete) <= e(residual_acceptance_tolerance)
assert mreldif(weighted_rust_frequency_b,e(projection_b)) < 2e-8
assert mreldif(weighted_rust_frequency_V,e(projection_V)) < 2e-8

quietly fevc y c1 [fw=copies], worker(worker) firm(firm)       ///
    deletion(observation) algorithm(jla) engine(generic)        ///
    backend(rust) rng(counter_v1) preconditioner(diagonal)      ///
    batch(8) probes(600) tolerance(1e-12)                       ///
    targetweight(target_mass) project(z) projecteffect(worker)  ///
    projectweight(target) nodisplay
matrix weighted_rust_target_b = e(projection_b)
matrix weighted_rust_target_V = e(projection_V)
assert e(N_physical) == 480
assert e(projection_solver_max_complete) <= e(residual_acceptance_tolerance)

preserve
    expand copies
    bysort stored_row: generate long physical_copy = _n
    sort stored_row physical_copy
    generate double target_copy = target_mass/copies

    quietly fevc y c1, worker(worker) firm(firm)                ///
        deletion(observation) algorithm(exact) backend(mata)     ///
        project(z) projecteffect(firm) projectweight(frequency) nodisplay
    assert mreldif(weighted_exact_frequency_b,e(projection_b)) < 1e-10
    assert mreldif(weighted_exact_frequency_V,e(projection_V)) < 1e-10
    assert mreldif(weighted_exact_frequency_naive,               ///
        e(projection_V_naive)) < 1e-10

    quietly fevc y c1, worker(worker) firm(firm)                ///
        deletion(observation) algorithm(exact) backend(mata)     ///
        targetweight(target_copy) project(z) projecteffect(worker) ///
        projectweight(target) nodisplay
    assert mreldif(weighted_exact_target_b,e(projection_b)) < 1e-10
    assert mreldif(weighted_exact_target_V,e(projection_V)) < 1e-10
    assert mreldif(weighted_exact_target_naive,                  ///
        e(projection_V_naive)) < 1e-10

    quietly fevc y c1, worker(worker) firm(firm)                ///
        deletion(observation) algorithm(jla) engine(generic)     ///
        backend(rust) rng(counter_v1) preconditioner(diagonal)   ///
        batch(8) probes(600) tolerance(1e-12) project(z)         ///
        projecteffect(firm) projectweight(frequency) nodisplay
    assert mreldif(weighted_rust_frequency_b,e(projection_b)) < 2e-8
    assert mreldif(weighted_rust_frequency_V,e(projection_V)) < 2e-8

    quietly fevc y c1, worker(worker) firm(firm)                ///
        deletion(observation) algorithm(jla) engine(generic)     ///
        backend(rust) rng(counter_v1) preconditioner(diagonal)   ///
        batch(8) probes(600) tolerance(1e-12)                    ///
        targetweight(target_copy) project(z) projecteffect(worker) ///
        projectweight(target) nodisplay
    assert mreldif(weighted_rust_target_b,e(projection_b)) < 2e-8
    assert mreldif(weighted_rust_target_V,e(projection_V)) < 2e-8
restore

assert mreldif(weighted_exact_frequency_b,weighted_rust_frequency_b) < 1e-10
assert mreldif(weighted_exact_frequency_V,weighted_rust_frequency_V) < .005
assert mreldif(weighted_exact_target_b,weighted_rust_target_b) < 1e-10
assert mreldif(weighted_exact_target_V,weighted_rust_target_V) < .005

local cmg_rng_before `"`c(rngstate)'"'
capture noisily fevc y c1 [fw=copies], worker(worker) firm(firm) ///
    deletion(observation) algorithm(jla) engine(generic)          ///
    backend(rust) rng(counter_v1) preconditioner(cmg)             ///
    batch(8) probes(600) tolerance(1e-12) memory_gib(.00001)     ///
    project(z) projecteffect(firm) projectweight(frequency) nodisplay
assert _rc != 0
assert inlist(`"`e(withholding_status)'"',                         ///
    "PROJECTION_MEMORY_LIMIT", "GENERIC_RESOURCE_ADMISSION_FAILED", ///
    "SOLVER_MEMORY_LIMIT", "RESOURCE_LIMIT")
assert `"`c(rngstate)'"' == `"`cmg_rng_before'"'
fevc_rust snapshot
assert r(state) == 0

local weighted_rng_before `"`c(rngstate)'"'
capture noisily fevc y c1 [fw=copies], worker(worker) firm(firm) ///
    deletion(observation) algorithm(jla) engine(generic)          ///
    backend(rust) rng(counter_v1) preconditioner(diagonal)        ///
    batch(8) probes(600) tolerance(1e-12) memory_gib(.00001)     ///
    project(z) projecteffect(firm) projectweight(frequency) nodisplay
assert _rc != 0
assert inlist(`"`e(withholding_status)'"',                         ///
    "PROJECTION_MEMORY_LIMIT", "GENERIC_RESOURCE_ADMISSION_FAILED", ///
    "SOLVER_MEMORY_LIMIT", "RESOURCE_LIMIT")
assert `"`c(rngstate)'"' == `"`weighted_rng_before'"'
fevc_rust snapshot
assert r(state) == 0

// Automatic solver routing remains outside the explicit projection tuple;
// strict Rust requests must fail before native preparation.
capture noisily fevc y, worker(worker) firm(firm) deletion(observation) ///
    algorithm(jla) engine(generic) backend(rust) rng(counter_v1)        ///
    project(z) projecteffect(worker) nodisplay
assert _rc == 498
assert `"`e(status)'"' == "WITHHELD"
assert `"`e(withholding_status)'"' == "RUST_INFERENCE_UNSUPPORTED"
fevc_rust snapshot
assert r(state) == 0

di as result "PASS test_rust_projection.do"
exit 0

version 18.0
clear all
set more off
set varabbrev off
set processors 4

args outdir pkgroot
if `"`outdir'"' == "" exit 198
if `"`pkgroot'"' == "" local pkgroot `"`c(pwd)'/vckss"'
local fixture `"`pkgroot'/qualification/inference_matlab/evidence/input.csv"'

adopath ++ `"`pkgroot'"'
do `"`pkgroot'/tests/stata/test_load.do"'
import delimited using `"`fixture'"', clear varnames(1)

generate long stored_row = _n
generate byte frequency = 1 + mod(worker + firm, 3)
generate double target_mass = 1 + mod(2*worker + firm, 4)
generate double target_per_copy = target_mass/frequency

tempfile compressed
save `compressed'

tempname handle
file open `handle' using `"`outdir'/vckss_weighted_results.csv"', write text replace
file write `handle' "route,projection,kind,row,col,value" _n

foreach projection in firm_frequency worker_target {
    use `compressed', clear
    if `"`projection'"' == "firm_frequency" {
        local projection_options "projecteffect(firm) projectweight(frequency)"
    }
    else {
        local projection_options "projecteffect(worker) projectweight(target) targetweight(target_mass)"
    }

    quietly vckss y [fw=frequency], worker(worker) firm(firm)              ///
        deletion(observation) algorithm(exact) backend(mata)              ///
        project(z1 z2) `projection_options' nodisplay
    matrix exact_b = e(projection_b)
    matrix exact_V = e(projection_V)
    matrix exact_V_naive = e(projection_V_naive)
    local physical_N = e(N_physical)

    quietly vckss y [fw=frequency], worker(worker) firm(firm)              ///
        deletion(observation) algorithm(jla) engine(generic) backend(rust) ///
        rng(counter_v1) preconditioner(diagonal) batch(16) probes(4000)    ///
        tolerance(1e-12) project(z1 z2) `projection_options' nodisplay
    matrix rust_b = e(projection_b)
    matrix rust_V = e(projection_V)
    matrix rust_V_naive = e(projection_V_naive)
    assert e(N_physical) == `physical_N'
    assert e(projection_solver_max_complete) <= e(residual_acceptance_tolerance)
    assert e(projection_covariance_min) >= -1e-10
    assert e(projection_peak_forecast_bytes) <= e(batch_memory_budget_bytes)

    expand frequency
    replace frequency = 1
    if `"`projection'"' == "worker_target" {
        local expanded_options "projecteffect(worker) projectweight(target) targetweight(target_per_copy)"
    }
    else {
        local expanded_options "projecteffect(firm) projectweight(frequency)"
    }
    quietly vckss y [fw=frequency], worker(worker) firm(firm)              ///
        deletion(observation) algorithm(exact) backend(mata)              ///
        project(z1 z2) `expanded_options' nodisplay
    matrix expanded_b = e(projection_b)
    matrix expanded_V = e(projection_V)
    matrix expanded_V_naive = e(projection_V_naive)
    assert e(N_physical) == `physical_N'

    assert mreldif(exact_b, expanded_b) < 1e-10
    assert mreldif(exact_V, expanded_V) < 1e-10
    assert mreldif(exact_V_naive, expanded_V_naive) < 1e-10
    assert mreldif(exact_b, rust_b) < 1e-10
    assert mreldif(exact_V, rust_V) < .005

    local names : colnames exact_b
    foreach route in exact expanded rust {
        forvalues j = 1/3 {
            local name : word `j' of `names'
            file write `handle' "`route',`projection',projection_b,,`name'," ///
                %24.17e (`route'_b[1,`j']) _n
            forvalues k = 1/3 {
                local name2 : word `k' of `names'
                file write `handle' "`route',`projection',projection_V,`name',`name2'," ///
                    %24.17e (`route'_V[`j',`k']) _n
                file write `handle' "`route',`projection',projection_V_naive,`name',`name2'," ///
                    %24.17e (`route'_V_naive[`j',`k']) _n
            }
        }
    }
    file write `handle' "exact,`projection',meta,,physical_N," %24.17e (`physical_N') _n
}

file close `handle'
di as result "PASS vckss_weighted_projection.do"
exit 0

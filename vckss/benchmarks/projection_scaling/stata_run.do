version 18.0
clear all
set more off
set varabbrev off
set linesize 255

args package_root input_csv output_csv phase_start phase_end role source_commit rows_arg probes_arg seed_arg
local rows = real("`rows_arg'")
local probes = real("`probes_arg'")
local seed = real("`seed_arg'")
if !inlist("`role'","exact","rust") | !inlist(`rows',6000,24000,96000) | ///
   !ustrregexm("`source_commit'","^[0-9a-f]{40}$") | `seed'<1 |          ///
   ("`role'"=="exact" & `rows'!=6000) |                                 ///
   ("`role'"=="rust" & `probes'!=ceil(log(`rows')/log(2)/.01)) {
    di as error "invalid projection-scaling Stata arguments"
    exit 198
}

adopath ++ `"`package_root'/vckss"'
confirm file `"`package_root'/vckss/vckss.ado"'
if "`role'"=="rust" {
    if strpos(c(machine_type),"Mac") confirm file `"`package_root'/vckss/vckss_rust_macos_arm64.plugin"'
    else confirm file `"`package_root'/vckss/vckss_rust_linux_x64.plugin"'
}
set processors 4
import delimited using `"`input_csv'"', clear varnames(1) asdouble bindquote(strict)
assert _N==`rows'
isid observation_key
isid worker period
assert worker==floor(worker) & firm==floor(firm)
quietly count if missing(observation_key,worker,firm,period,y,z1,z2)
assert r(N)==0
quietly bysort worker (period): assert _N==6
quietly bysort worker firm: generate byte first_match = _n==1
quietly bysort worker: egen byte firm_count = total(first_match)
assert firm_count==3
drop first_match firm_count
sort observation_key

quietly _datasignature
local data_signature `"`r(datasignature)'"'
set rng default
set seed 20260830
set sortseed 20260830
local rng_before `"`c(rngstate)'"'
local sort_rng_before `"`c(sortrngstate)'"'

tempname marker
file open `marker' using `"`phase_start'"', write text replace
file write `marker' "START `role'" _n
file close `marker'
timer clear 80
timer on 80
if "`role'"=="exact" {
    capture noisily vckss y, worker(worker) firm(firm)                 ///
        deletion(observation) algorithm(exact) backend(mata)          ///
        exact_limit(2000) project(z1 z2) projecteffect(firm)          ///
        projectweight(frequency) nodisplay
}
else {
    capture noisily vckss y, worker(worker) firm(firm)                 ///
        deletion(observation) algorithm(jla) engine(generic)          ///
        backend(rust) rng(counter_v1) preconditioner(diagonal)         ///
        batch(16) probes(`probes') seed(`seed') maxiter(20000)         ///
        memory_gib(14) wallseconds(20000) project(z1 z2)               ///
        projecteffect(firm) projectweight(frequency) nodisplay
}
local command_rc = _rc
timer off 80
tempname endmarker
file open `endmarker' using `"`phase_end'"', write text replace
file write `endmarker' "END `role' rc=`command_rc'" _n
file close `endmarker'
if `command_rc' exit `command_rc'
timer list 80
local command_seconds = r(t80)

matrix b = e(projection_b)
matrix V = e(projection_V)
matrix V_naive = e(projection_V_naive)
assert rowsof(b)==1 & colsof(b)==3 & rowsof(V)==3 & colsof(V)==3
assert e(N_stored)==`rows' & e(N_physical)==`rows' & e(N_retained)==`rows'
tempname covariance_minimum covariance_maximum
mata: st_numscalar("`covariance_minimum'",min(symeigenvalues(st_matrix("V"))))
mata: st_numscalar("`covariance_maximum'",max(symeigenvalues(st_matrix("V"))))
local covariance_min = scalar(`covariance_minimum')
local covariance_max = scalar(`covariance_maximum')
assert `covariance_min'>=-1e-8*max(1,`covariance_max')
local psd_cleanup = 0
local projection_residual = .
local residual_tolerance = .
local memory_forecast = .
local memory_limit = .
local solver_iterations = .
if "`role'"=="rust" {
    assert abs(e(projection_covariance_min)-`covariance_min')<=1e-12*max(1,abs(`covariance_min'))
    assert abs(e(projection_covariance_max)-`covariance_max')<=1e-12*max(1,abs(`covariance_max'))
    assert e(projection_psd_cleanup)<=1e-8*max(1,e(projection_covariance_max))
    assert "`e(backend_selected)'"=="rust" & "`e(algorithm)'"=="jla"
    assert "`e(engine_selected)'"=="generic"
    assert "`e(preconditioner_selected)'"=="DIAGONAL"
    assert "`e(rng_selected)'"=="counter_v1"
    assert e(complete_residual_max)<=e(residual_acceptance_tolerance)
    assert e(projection_solver_max_complete)<=e(residual_acceptance_tolerance)
    assert e(projection_peak_forecast_bytes)<=e(batch_memory_budget_bytes)
    local projection_residual = e(projection_solver_max_complete)
    local residual_tolerance = e(residual_acceptance_tolerance)
    local memory_forecast = e(projection_peak_forecast_bytes)
    local memory_limit = e(batch_memory_budget_bytes)
    local solver_iterations = e(projection_solver_iterations)
    local psd_cleanup = e(projection_psd_cleanup)
}

quietly _datasignature
assert `"`r(datasignature)'"'==`"`data_signature'"'
assert `"`c(rngstate)'"'==`"`rng_before'"'
assert `"`c(sortrngstate)'"'==`"`sort_rng_before'"'

clear
set obs 1
generate str40 schema = "VCKSS-PROJECTION-SCALING-STATA-V1"
generate str8 role = "`role'"
generate str4 status = "PASS"
generate str40 source_commit = "`source_commit'"
generate long rows = `rows'
generate long probes = `probes'
generate long seed = `seed'
generate double command_seconds = `command_seconds'
generate double b_cons = b[1,1]
generate double b_z1 = b[1,2]
generate double b_z2 = b[1,3]
forvalues left = 1/3 {
    forvalues right = 1/3 {
        generate double V_`left'_`right' = V[`left',`right']
        generate double V_naive_`left'_`right' = V_naive[`left',`right']
    }
}
generate double covariance_min = `covariance_min'
generate double covariance_max = `covariance_max'
generate double psd_cleanup = `psd_cleanup'
generate double projection_complete_residual = `projection_residual'
generate double residual_tolerance = `residual_tolerance'
generate double memory_forecast_bytes = `memory_forecast'
generate double memory_limit_bytes = `memory_limit'
generate double projection_solver_iterations = `solver_iterations'
export delimited using `"`output_csv'"', replace
di as result "VCKSS PROJECTION SCALING STATA PASS `role' rows=`rows'"
exit 0
